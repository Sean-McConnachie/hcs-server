use std::{fs, io::Write};

use hcs_lib::{data, protocol, server_database};

pub async fn handle_file_create(
    tcp_connection: &mut Box<protocol::TcpConnection>,
    db_pool: &sqlx::PgPool,
    file_handler_config: &server_database::ServerFileHandlerConfig,
    file_create: data::FileCreate,
) -> Result<(), Box<dyn std::error::Error>> {
    let packets = protocol::calculate_num_packets(file_create.size());
    let mut file = fs::File::create(
        file_handler_config
            .storage_directory()
            .join(file_create.path()),
    )?;
    for _ in 0..packets {
        let buffer = tcp_connection.read_next_chunk()?;
        file.write(buffer)?;
    }

    let change_event = data::ChangeEvent::File(data::FileEvent::Create(file_create));
    server_database::insert_change(change_event, db_pool).await?;
    Ok(())
}
