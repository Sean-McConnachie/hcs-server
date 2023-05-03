use std::fs;

use hcs_lib::{data, server_database};

pub async fn handle_directory_delete(
    db_pool: &sqlx::PgPool,
    file_handler_config: &server_database::ServerFileHandlerConfig,
    directory_delete: data::DirectoryDelete,
) -> Result<(), Box<dyn std::error::Error>> {
    let delete_path = file_handler_config
        .storage_directory()
        .join(directory_delete.path());

    if delete_path.exists() {
        fs::remove_dir_all(delete_path)?;
    } else {
        log::error!(
            "Directory to delete does not exist: `{}`. Inserting change regardless.",
            directory_delete.path()
        );
    }

    let change_event = data::ChangeEvent::Directory(data::DirectoryEvent::Delete(directory_delete));
    server_database::insert_change(change_event, db_pool).await?;

    Ok(())
}
