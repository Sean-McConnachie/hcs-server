use std::fs;

use hcs_lib::{data, server_database};

pub async fn handle_file_delete(
    db_pool: &sqlx::PgPool,
    file_handler_config: &server_database::ServerFileHandlerConfig,
    file_delete: data::FileDelete,
) -> Result<(), Box<dyn std::error::Error>> {
    let delete_path = file_handler_config
        .storage_directory()
        .join(file_delete.path());

    if delete_path.exists() {
        fs::remove_file(delete_path)?;
    } else {
        log::error!(
            "File to delete does not exist: `{}`. Inserting change regardless.",
            file_delete.path()
        );
    }

    let change_event = data::ChangeEvent::File(data::FileEvent::Delete(file_delete));
    server_database::insert_change(change_event, db_pool).await?;

    Ok(())
}
