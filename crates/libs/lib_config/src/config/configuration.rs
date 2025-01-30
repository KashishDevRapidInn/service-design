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
pub struct DomainSettings {
    pub user_service_domain: String,
    pub admin_service_domain: String,
    pub game_service_domain: String,
    pub gateway_service_domain: String,
}

#[derive(Debug, Deserialize)]
pub struct KafkaSettings {
    pub user_url: String,
    pub admin_url: String,
    pub game_url: String,
    pub user_topics: Vec<String>,
    pub admin_topics: Vec<String>,
    pub game_topics: Vec<String>,
    pub user_consumer_group: String,
    pub admin_subscribe_topics: Vec<String>,
    pub game_subscribe_topics: Vec<String>,
    pub game_consumer_group: String,
    pub user_subscribe_topics: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub service: ServiceSettings,
    pub databases: DatabaseSettings,
    pub redis: RedisSettings,
    pub jwt: JwtSettings,
    pub kafka: KafkaSettings,
    pub domain: DomainSettings,
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

