use std::net;

use hcs_lib::{config, server_database};

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ServerConfig {
    #[serde(deserialize_with = "config::parse_log_filter")]
    log_level: log::LevelFilter,

    db_config: server_database::DbConfig,
    tcp_config: TcpConfig,
    file_handler_config: server_database::ServerFileHandlerConfig,
}

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
pub struct TcpConfig {
    addr: net::SocketAddr,
}

impl ServerConfig {
    pub fn log_level(&self) -> log::LevelFilter {
        self.log_level
    }

    pub fn db_config(&self) -> &server_database::DbConfig {
        &self.db_config
    }

    pub fn tcp_config(&self) -> &TcpConfig {
        &self.tcp_config
    }

    pub fn file_handler_config(&self) -> &server_database::ServerFileHandlerConfig {
        &self.file_handler_config
    }
}

impl TcpConfig {
    pub fn addr(&self) -> &net::SocketAddr {
        &self.addr
    }
}
