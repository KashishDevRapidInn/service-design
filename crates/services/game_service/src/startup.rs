use kafka::{channel::KafkaMessage, setup::{setup_kafka_sender, setup_kafka_receiver}};
use lib_config::{config::configuration::Settings, db::db::PgPool};
use crate::routes::health_check::health_check;
use actix_web::cookie::Key;
use actix_web::{dev::Server, web, App, HttpServer};
use actix_web_lab::middleware::from_fn;
use middleware::jwt::jwt_auth_middleware;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use lib_config::session::redis::RedisService;
use crate::kafka_handler::process_kafka_game_message;
use flume::Sender;
use crate::routes::game::games::rate;
use elasticsearch::{Elasticsearch};
use elasticsearch::http::transport::Transport;
use std::error::Error;
use reqwest::ClientBuilder;

/******************************************/
// Initializing Elastic client
/******************************************/
pub fn init_elasticsearch() -> Result<Elasticsearch, Box<dyn Error>> {
    // let client = ClientBuilder::new()
    // .danger_accept_invalid_certs(true)
    // .build()?;
    let transport = Transport::single_node("http://127.0.0.1:9200")?;

    let es_client = Elasticsearch::new(transport);
    Ok(es_client)
}


pub fn generate_secret_key() -> Key {
    Key::generate()
}
/**************************************************************/
// Application State re reuse the same code in main and tests
/***************************************************************/
pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(pool: PgPool, config: &Settings) -> Result<Self, std::io::Error> {
        let listener = if config.service.game_service_port == 0 {
            TcpListener::bind("127.0.0.1:0")?
        } else {
            let address = format!("127.0.0.1:{}", config.service.game_service_port);
            TcpListener::bind(&address)?
        };

        let actual_port = listener.local_addr()?.port();

        let tx = setup_kafka_sender(&config.kafka.game_url, &config.kafka.game_topics).await;
        let rx = setup_kafka_receiver(&config.kafka.game_url, &config.kafka.game_subscribe_topics, &config.kafka.game_consumer_group).await;

        let elastic_client = init_elasticsearch().map_err(|e| {
            eprintln!("Failed to create elastic search client: {:?}", e);
            std::io::Error::new(std::io::ErrorKind::Other, "Redis connection failed")
        })?;

        let pool_clone = pool.clone();
        let es_clone= elastic_client.clone();
        tokio::spawn(async move {
            process_kafka_game_message(rx, pool_clone, es_clone).await;
        });

        let server = run_server(listener, pool.clone(), config.redis.uri.clone(), tx, elastic_client).await?;

        Ok(Self {
            port: actual_port,
            server,
        })
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

/******************************************/
// Running Server
/******************************************/
pub async fn run_server(
    listener: TcpListener,
    pool: PgPool,
    redis_uri: String,
    kafka_sender: Sender<KafkaMessage<String>>,
    es_client: Elasticsearch
) -> Result<Server, std::io::Error> {

    let redis_service = RedisService::new(redis_uri).await;

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(web::Data::new(redis_service.clone()))
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(kafka_sender.clone()))
            .app_data(web::Data::new(es_client.clone()))
            .route("/health_check", web::get().to(health_check))
            .service(
                web::scope("/api/v1")
                    .wrap(from_fn(jwt_auth_middleware))
                    .route("/rate", web::post().to(rate))
            )       
    })
    .listen(listener)?
    .run();
    Ok(server)
}
