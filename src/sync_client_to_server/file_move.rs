use std::fs;

use hcs_lib::{data, server_database};

pub async fn handle_file_move(
    db_pool: &sqlx::PgPool,
    file_handler_config: &server_database::ServerFileHandlerConfig,
    file_move: data::FileMove,
) -> Result<(), Box<dyn std::error::Error>> {
    let old_path = file_handler_config
        .storage_directory()
        .join(file_move.from_path());
    let new_path = file_handler_config
        .storage_directory()
        .join(file_move.to_path());

    if old_path.exists() {
        fs::rename(old_path, new_path)?;
    } else {
        log::error!(
            "File to move does not exist: `{}`. Inserting change regardless.",
            file_move.to_path()
        );
    }

    let change_event = data::ChangeEvent::File(data::FileEvent::Move(file_move));
    server_database::insert_change(change_event, db_pool).await?;

    Ok(())
}
