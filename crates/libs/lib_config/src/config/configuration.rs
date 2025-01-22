use config::{Config, ConfigError};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DatabaseSettings {
    pub user_db_url: String,
    pub admin_db_url: String,
    pub game_db_url: String,
}

#[derive(Debug, Deserialize)]
pub struct RedisSettings {
    pub uri: String,
}

#[derive(Debug, Deserialize)]
pub struct JwtSettings {
    pub secret: String,
}

#[derive(Debug, Deserialize)]
pub struct ServiceSettings {
    pub user_service_port: u16,
    pub admin_service_port: u16,
    pub game_service_port: u16,
    pub gateway_service_port: u16,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub service: ServiceSettings,
    pub databases: DatabaseSettings,
    pub redis: RedisSettings,
    pub jwt: JwtSettings,
}

// impl Settings {
//     pub fn new() -> Result<Self, ConfigError> {
//         let mut s = Config::default();
//         s.merge(config::File::with_name("config.toml"))?;
//         s.try_into()
//     }
// }
impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::default();
        s.merge(config::File::with_name("config"))?;
        s.try_into()
    }
}

