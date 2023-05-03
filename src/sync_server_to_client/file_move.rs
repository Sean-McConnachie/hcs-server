use hcs_lib::{data, protocol};

use crate::{errors, extra_data, serve::transmission_type_to_bytes};

pub fn handle_file_move(
    tcp_connection: &mut Box<protocol::TcpConnection>,
    file_move: data::FileMove,
) -> Result<(), Box<dyn std::error::Error>> {
    // Send change event to client
    let change_event = data::ChangeEvent::File(data::FileEvent::Move(file_move));
    let transmission =
        data::Transmission::<errors::ServerTcpError, extra_data::ExtraData>::ChangeEvent(
            change_event.into(),
        );
    let bytes = transmission_type_to_bytes(transmission)?;
    tcp_connection.write(&*bytes)?;

    Ok(())
}
