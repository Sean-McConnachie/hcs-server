use hcs_lib::{data, protocol};

use crate::{errors, extra_data, serve::transmission_type_to_bytes};

pub fn handle_file_delete(
    tcp_connection: &mut Box<protocol::TcpConnection>,
    file_delete: data::FileDelete,
) -> Result<(), Box<dyn std::error::Error>> {
    // Send change event to client
    let change_event = data::ChangeEvent::File(data::FileEvent::Delete(file_delete));
    let transmission =
        data::Transmission::<errors::ServerTcpError, extra_data::ExtraData>::ChangeEvent(
            change_event.into(),
        );
    let bytes = transmission_type_to_bytes(transmission)?;
    tcp_connection.write(&*bytes)?;

    Ok(())
}
