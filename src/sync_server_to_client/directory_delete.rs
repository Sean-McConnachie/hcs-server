use crate::{errors, extra_data, serve::transmission_type_to_bytes};
use hcs_lib::{data, protocol};

pub fn handle_directory_delete(
    tcp_connection: &mut Box<protocol::TcpConnection>,
    directory_delete: data::DirectoryDelete,
) -> Result<(), Box<dyn std::error::Error>> {
    let change_event = data::ChangeEvent::Directory(data::DirectoryEvent::Delete(directory_delete));
    let transmission =
        data::Transmission::<errors::ServerTcpError, extra_data::ExtraData>::ChangeEvent(
            change_event.into(),
        );
    let bytes = transmission_type_to_bytes(transmission)?;
    tcp_connection.write(&*bytes)?;
    Ok(())
}
