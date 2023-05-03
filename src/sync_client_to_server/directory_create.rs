use std::fs;

use hcs_lib::{data, server_database};

pub async fn handle_directory_create(
    db_pool: &sqlx::PgPool,
    file_handler_config: &server_database::ServerFileHandlerConfig,
    directory_create: data::DirectoryCreate,
) -> Result<(), Box<dyn std::error::Error>> {
    let create_path = file_handler_config
        .storage_directory()
        .join(directory_create.path());

    if !create_path.exists() {
        fs::create_dir_all(create_path)?;
    } else {
        log::error!(
            "Directory to create already exists: `{}`. Inserting change regardless.",
            directory_create.path()
        );
    }

    let change_event = data::ChangeEvent::Directory(data::DirectoryEvent::Create(directory_create));
    server_database::insert_change(change_event, db_pool).await?;

    Ok(())
}
