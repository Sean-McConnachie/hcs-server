use std::fs;

use std::io::Read;

use crate::{errors, extra_data, serve::transmission_type_to_bytes};
use hcs_lib::{data, protocol, server_database};

pub fn handle_file_create(
    tcp_connection: &mut Box<protocol::TcpConnection>,
    file_handler_config: &server_database::ServerFileHandlerConfig,
    mut file_create: data::FileCreate,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = file_handler_config
        .storage_directory()
        .join(file_create.path());
    let file_size = fs::metadata(&file_path)?.len();

    file_create.set_size(file_size);
    {
        // Send change event to client
        let change_event = data::ChangeEvent::File(data::FileEvent::Create(file_create));
        let transmission =
            data::Transmission::<errors::ServerTcpError, extra_data::ExtraData>::ChangeEvent(
                change_event.into(),
            );
        let bytes = transmission_type_to_bytes(transmission)?;
        tcp_connection.write(&*bytes)?;
    }

    {
        // Read the file buffer by buffer, write into tcp stream.
        let packets = protocol::calculate_num_packets(file_size);
        let mut file = fs::File::open(&file_path)?;
        let mut buffer = vec![0; protocol::BUFFER_SIZE];
        for _ in 0..packets {
            let bytes_read = file.read(&mut buffer)?;
            tcp_connection.write(&buffer[..bytes_read])?;
        }
    }
    Ok(())
}
