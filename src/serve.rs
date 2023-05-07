use std::collections::LinkedList;
use std::net as s_net;

use hcs_lib::protocol::server::HCSProtocol;
use hcs_lib::{data, protocol, server_database};

use crate::{config, errors, extra_data, sync_client_to_server, sync_server_to_client};

static SLEEP_TIME: u64 = 5;

pub async fn tcp_handler(db_pool: sqlx::PgPool, config: config::ServerConfig) {
    let listener = s_net::TcpListener::bind(config.tcp_config().addr())
        .expect("Failed to bind to TCP address");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let db_pool = db_pool.clone();
                let file_handler_config = config.file_handler_config().clone();
                tokio::spawn(async move {
                    let mut tcp_hcs_handler =
                        TcpHCSHandler::new(stream, db_pool.clone(), file_handler_config.clone());

                    let transmission_result = tcp_hcs_handler.start_transmission().await;

                    match transmission_result {
                        Ok(_) => log::info!("Transmission successful"),
                        Err(e) => log::error!("Transmission failed: {}", e),
                    }
                });
            }
            Err(e) => log::error!("Error accepting client: {}", e),
        }
    }
}

fn bytes_to_transmission_type(
    bytes: &[u8],
) -> Result<
    data::Transmission<errors::ServerTcpError, extra_data::ExtraData>,
    Box<dyn std::error::Error>,
> {
    let return_type: data::Transmission<errors::ServerTcpError, extra_data::ExtraData> =
        bincode::deserialize(bytes)?;
    Ok(return_type)
}

pub fn transmission_type_to_bytes(
    transmission: data::Transmission<errors::ServerTcpError, extra_data::ExtraData>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let bytes = bincode::serialize(&transmission)?;
    Ok(bytes)
}

struct TcpHCSHandler {
    tcp_connection: Box<protocol::TcpConnection>,
    db_pool: sqlx::PgPool,
    file_handler_config: server_database::ServerFileHandlerConfig,
}

impl TcpHCSHandler {
    fn new(
        tcp_stream: s_net::TcpStream,
        db_pool: sqlx::PgPool,
        file_handler_config: server_database::ServerFileHandlerConfig,
    ) -> Self {
        let tcp_connection = protocol::TcpConnection::new(tcp_stream);
        Self {
            tcp_connection,
            db_pool,
            file_handler_config,
        }
    }

    async fn start_transmission(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting transmission");
        log::debug!("Waiting for greeting");
        // Receive greeting from client
        let bytes = self.tcp_connection.read_next_chunk()?;
        let transmission = bytes_to_transmission_type(&bytes)?;
        let greeting: data::Greeting = transmission.try_into().map_err(|_| {
            "Failed to convert transmission to greeting. Expected greeting transmission."
        })?;
        let response = self.greet(greeting).await;

        log::debug!("Sending response");
        // Send response to client (either proceed or error)
        self.tcp_connection
            .write(&transmission_type_to_bytes(response)?)?;

        log::debug!("Starting payload loop");
        loop {
            let bytes = self.tcp_connection.read_next_chunk()?;
            let transmission = bytes_to_transmission_type(&bytes)?;
            let end_transmission = self.receive_payload(transmission).await;

            if let Ok(_end_transmission) = end_transmission {
                break;
            } else {
                dbg!(&end_transmission);
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl HCSProtocol<data::Transmission<errors::ServerTcpError, extra_data::ExtraData>>
    for TcpHCSHandler
{
    async fn greet(
        &mut self,
        _payload: data::Greeting,
    ) -> data::Transmission<errors::ServerTcpError, extra_data::ExtraData> {
        // TODO: If client version (payload.version()) and server_version are out of sync, send error to upgrade/downgrade client.

        let response = data::Transmission::Proceed;
        response
    }

    async fn receive_payload(
        &mut self,
        payload: data::Transmission<errors::ServerTcpError, extra_data::ExtraData>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match payload {
            data::Transmission::SyncClientToServer(sync_client_to_server) => {
                log::debug!("Handling sync client to server");
                handle_sync_client_to_server(
                    &mut self.tcp_connection,
                    &self.db_pool,
                    &self.file_handler_config,
                    sync_client_to_server,
                )
                .await?;
            }
            data::Transmission::SyncServerToClient(sync_server_to_client) => {
                log::debug!("Handling sync server to client");
                handle_sync_server_to_client(
                    &mut self.tcp_connection,
                    &self.db_pool,
                    &self.file_handler_config,
                    sync_server_to_client,
                )
                .await?;
            }
            data::Transmission::ServerVersion(_) => {
                handle_server_version(&mut self.tcp_connection, &self.db_pool).await?;
            }
            data::Transmission::EndConnection => return Ok(true),
            data::Transmission::Other(_) => unimplemented!("Other transmission type"),
            _ => {
                panic!("Unexpected transmission type");
            }
        }

        return Ok(false);
    }

    // async fn send_payload(
    //     &mut self,
    // ) -> data::Transmission<errors::ServerTcpError, extra_data::ExtraData> {
    //     // self.tcp_connection.send_payload(payload).await?;
    //     // Ok(())
    //     unimplemented!()
    // }
}

async fn handle_server_to_client_change_event(
    tcp_connection: &mut Box<protocol::TcpConnection>,
    file_handler_config: &server_database::ServerFileHandlerConfig,
    change_event: data::ChangeEvent,
) -> Result<(), Box<dyn std::error::Error>> {
    match change_event {
        data::ChangeEvent::File(file_event) => match file_event {
            data::FileEvent::Create(file_create) => {
                sync_server_to_client::handle_file_create(
                    tcp_connection,
                    file_handler_config,
                    file_create,
                )?;
            }
            data::FileEvent::Delete(file_delete) => {
                sync_server_to_client::handle_file_delete(tcp_connection, file_delete)?;
            }
            data::FileEvent::Modify(file_modify) => {
                sync_server_to_client::handle_file_modify(
                    tcp_connection,
                    file_handler_config,
                    file_modify,
                )?;
            }
            data::FileEvent::Move(file_move) => {
                sync_server_to_client::handle_file_move(tcp_connection, file_move)?;
            }
            data::FileEvent::UndoDelete(_file_undo_delete) => {
                unimplemented!("Undo delete file")
            }
        },
        data::ChangeEvent::Directory(directory_event) => match directory_event {
            data::DirectoryEvent::Create(directory_create) => {
                sync_server_to_client::handle_directory_create(tcp_connection, directory_create)?;
            }
            data::DirectoryEvent::Delete(directory_delete) => {
                sync_server_to_client::handle_directory_delete(tcp_connection, directory_delete)?;
            }
            data::DirectoryEvent::Move(directory_move) => {
                sync_server_to_client::handle_directory_move(tcp_connection, directory_move)?;
            }
            data::DirectoryEvent::UndoDelete(_directory_undo_delete) => {
                unimplemented!("Undo delete directory")
            }
        },
        _ => unimplemented!(),
    }
    Ok(())
}

async fn handle_sync_server_to_client(
    tcp_connection: &mut Box<protocol::TcpConnection>,
    db_pool: &sqlx::PgPool,
    file_handler_config: &server_database::ServerFileHandlerConfig,
    sync_server_to_client: data::SyncServerToClient,
) -> Result<(), Box<dyn std::error::Error>> {
    let server_version = server_database::get_server_version(db_pool).await?;
    let client_version = sync_server_to_client.client_version();

    let changes = server_database::get_changes(client_version, server_version, db_pool).await?;
    let changes = changes
        .into_iter()
        .map(|change| change.into())
        .collect::<LinkedList<_>>();
    let optimized_changes = data::optimize_changes(changes);

    let change_len = optimized_changes.len();

    if optimized_changes.len() == 0 {
        log::error!("Sending new server version {}", server_version);
        let sv = data::ServerVersion::new(server_version);
        let transmission =
            data::Transmission::<errors::ServerTcpError, extra_data::ExtraData>::ServerVersion(sv);
        let bytes = transmission_type_to_bytes(transmission)?;
        tcp_connection.write(&bytes)?;
    }

    for (i, change_event) in optimized_changes.into_iter().enumerate() {
        log::info!("Sending change event {}/{}", i + 1, change_len);
        std::thread::sleep(std::time::Duration::from_millis(SLEEP_TIME));
        match handle_server_to_client_change_event(
            tcp_connection,
            file_handler_config,
            change_event.1,
        )
        .await
        {
            Ok(_) => {}
            Err(err) => {
                log::error!("Error handling server to client change event: {}", err);
                let skip_current = data::Transmission::<
                    errors::ServerTcpError,
                    extra_data::ExtraData,
                >::SkipCurrent;
                let bytes = transmission_type_to_bytes(skip_current)?;
                tcp_connection.write(&bytes)?;
            }
        };
        std::thread::sleep(std::time::Duration::from_millis(SLEEP_TIME));
        {
            // send new server version to client
            let sv = if i == change_len - 1 {
                data::ServerVersion::new(server_version)
            } else {
                data::ServerVersion::new(change_event.0)
            };
            let transmission =
                data::Transmission::<errors::ServerTcpError, extra_data::ExtraData>::ServerVersion(
                    sv,
                );
            let bytes = transmission_type_to_bytes(transmission)?;
            tcp_connection.write(&bytes)?;
        }
    }

    {
        std::thread::sleep(std::time::Duration::from_millis(SLEEP_TIME));
        // send transaction complete
        let transmission =
            data::Transmission::<errors::ServerTcpError, extra_data::ExtraData>::TransactionComplete;
        let bytes = transmission_type_to_bytes(transmission)?;
        tcp_connection.write(&bytes)?;
    }

    Ok(())
}

async fn handle_sync_client_to_server(
    tcp_connection: &mut Box<protocol::TcpConnection>,
    db_pool: &sqlx::PgPool,
    file_handler_config: &server_database::ServerFileHandlerConfig,
    sync_client_to_server: data::SyncClientToServer,
) -> Result<(), Box<dyn std::error::Error>> {
    {
        log::debug!("Handling sync client to server. Checking if client is in sync with server.");
        // Check if client is in sync with the server. If no, sync_server_to_client first
        let server_version = server_database::get_server_version(db_pool).await?;
        if server_version != sync_client_to_server.client_version() {
            log::debug!("Client is not in sync with server. Sending sync_server_to_client first.");
            let transmission =
                data::Transmission::<errors::ServerTcpError, extra_data::ExtraData>::ServerVersion(
                    data::ServerVersion::new(server_version),
                );
            let bytes = transmission_type_to_bytes(transmission)?;
            tcp_connection.write(&*bytes)?;
            return Ok(());
        } else {
            log::debug!("Client is in sync with server.");
            // Proceed
            let transmission =
                data::Transmission::<errors::ServerTcpError, extra_data::ExtraData>::Proceed;
            let bytes = transmission_type_to_bytes(transmission)?;
            tcp_connection.write(&*bytes)?;
        }
    }

    {
        // Iterate num_changes and receive changes
        for change_num in 0..sync_client_to_server.number_of_changes() {
            log::info!(
                "Change number: {} of {}",
                change_num + 1,
                sync_client_to_server.number_of_changes()
            );
            let bytes = tcp_connection.read_next_chunk()?;
            let transmission = bytes_to_transmission_type(bytes)?;
            match transmission {
                data::Transmission::ChangeEvent(change_event) => match change_event {
                    data::ChangeEvent::File(file_event) => match file_event {
                        data::FileEvent::Create(file_create) => {
                            sync_client_to_server::handle_file_create(
                                tcp_connection,
                                db_pool,
                                file_handler_config,
                                file_create,
                            )
                            .await?;
                        }
                        data::FileEvent::Delete(file_delete) => {
                            sync_client_to_server::handle_file_delete(
                                db_pool,
                                file_handler_config,
                                file_delete,
                            )
                            .await?;
                        }
                        data::FileEvent::Modify(file_modify) => {
                            sync_client_to_server::handle_file_modify(
                                tcp_connection,
                                db_pool,
                                file_handler_config,
                                file_modify,
                            )
                            .await?;
                        }
                        data::FileEvent::Move(file_move) => {
                            sync_client_to_server::handle_file_move(
                                db_pool,
                                file_handler_config,
                                file_move,
                            )
                            .await?;
                        }
                        data::FileEvent::UndoDelete(_file_undo_delete) => {
                            unimplemented!("Undo delete file")
                        }
                    },
                    data::ChangeEvent::Directory(directory_event) => match directory_event {
                        data::DirectoryEvent::Create(directory_create) => {
                            sync_client_to_server::handle_directory_create(
                                db_pool,
                                file_handler_config,
                                directory_create,
                            )
                            .await?;
                        }
                        data::DirectoryEvent::Delete(directory_delete) => {
                            sync_client_to_server::handle_directory_delete(
                                db_pool,
                                file_handler_config,
                                directory_delete,
                            )
                            .await?;
                        }
                        data::DirectoryEvent::Move(directory_move) => {
                            sync_client_to_server::handle_directory_move(
                                db_pool,
                                file_handler_config,
                                directory_move,
                            )
                            .await?;
                        }
                        data::DirectoryEvent::UndoDelete(_directory_undo_delete) => {
                            unimplemented!("Undo delete directory")
                        }
                    },
                    _ => unimplemented!(),
                },
                _ => unimplemented!(),
            }
            {
                log::debug!("Sending new server version to client.");
                let server_version = server_database::get_server_version(db_pool).await?;
                let transmission = data::Transmission::<
                    errors::ServerTcpError,
                    extra_data::ExtraData,
                >::ServerVersion(data::ServerVersion::new(
                    server_version,
                ));
                let bytes = transmission_type_to_bytes(transmission)?;
                tcp_connection.write(&*bytes)?;
            }
        }
    }

    Ok(())
}

async fn handle_server_version(
    tcp_connection: &mut Box<protocol::TcpConnection>,
    db_pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let server_version = server_database::get_server_version(db_pool).await?;

    let transmission =
        data::Transmission::<errors::ServerTcpError, extra_data::ExtraData>::ServerVersion(
            data::ServerVersion::new(server_version),
        );

    let bytes = transmission_type_to_bytes(transmission)?;
    tcp_connection.write(&*bytes)?;

    Ok(())
}
