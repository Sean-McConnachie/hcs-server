use hcs_lib::{logger, server_database};
use hcs_server::{config, serve};

#[tokio::main]
async fn main() {
    let config: config::ServerConfig =
        hcs_lib::config::read_config("Config.toml").expect("Failed to read `Config.toml`");

    logger::init_logger(config.log_level());

    let db_pool = server_database::connect_db(&config.db_config())
        .await
        .expect("Failed to connect to database");

    server_database::initialize_db(&db_pool).await.unwrap();

    serve::tcp_handler(db_pool, config).await;
}
