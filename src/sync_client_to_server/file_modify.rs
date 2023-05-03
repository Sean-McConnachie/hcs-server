use std::{fs, io::Write};

use hcs_lib::{data, protocol, server_database};

pub async fn handle_file_modify(
    tcp_connection: &mut Box<protocol::TcpConnection>,
    db_pool: &sqlx::PgPool,
    file_handler_config: &server_database::ServerFileHandlerConfig,
    file_modify: data::FileModify,
) -> Result<(), Box<dyn std::error::Error>> {
    let packets = protocol::calculate_num_packets(file_modify.size());
    let mut file = fs::File::create(
        file_handler_config
            .storage_directory()
            .join(file_modify.path()),
    )?;
    for _ in 0..packets {
        let buffer = tcp_connection.read_next_chunk()?;
        file.write(buffer)?;
    }

    let change_event = data::ChangeEvent::File(data::FileEvent::Modify(file_modify));
    server_database::insert_change(change_event, db_pool).await?;
    Ok(())
}
