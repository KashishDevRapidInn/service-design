use kafka::{channel::KafkaMessage, setup::setup_kafka_sender};
use lib_config::{config::configuration::Settings, db::db::PgPool};
// use crate::middleware::jwt_auth_middleware;
use crate::routes::{
    health_check::{health_check, set_session, get_session},
    user::crud::{register_user,login_user,logout_user,view_user}
};
use actix_session::storage::RedisSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::{dev::Server, web, App, HttpServer};
use actix_web_lab::middleware::from_fn;
use middleware::jwt::jwt_auth_middleware;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use lib_config::session::redis::RedisService;

use flume::Sender;
/******************************************/
// Initializing Redis connection
/******************************************/
pub async fn init_redis(redis_uri: String) -> Result<RedisSessionStore, std::io::Error> {
    // let redis_uri = env::var("REDIS_URI").expect("Failed to get redis uri");
    RedisSessionStore::new(redis_uri).await.map_err(|e| {
        eprintln!("Failed to create Redis session store: {:?}", e);
        std::io::Error::new(std::io::ErrorKind::Other, "Redis connection failed")
    })
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
        let listener = if config.service.user_service_port == 0 {
            TcpListener::bind("127.0.0.1:0")?
        } else {
            let address = format!("127.0.0.1:{}", config.service.user_service_port);
            TcpListener::bind(&address)?
        };

        let actual_port = listener.local_addr()?.port();

        let tx = setup_kafka_sender(&config.kafka.user_url, &config.kafka.user_topics).await;
        let server = run_server(listener, pool.clone(), config.redis.uri.clone(), tx).await?;

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
    kafka_sender: Sender<KafkaMessage<String>>
) -> Result<Server, std::io::Error> {
    let redis_store = init_redis(redis_uri.clone()).await?;
    let secret_key = generate_secret_key();

    let redis_service = RedisService::new(redis_uri).await;

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .app_data(web::Data::new(redis_service.clone()))
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(kafka_sender.clone()))
            .route("/health_check", web::get().to(health_check))
            .route("/set_session", web::get().to(set_session))
            .route("/get_session", web::get().to(get_session))
            .service(
                web::scope("/api/v1")
                .service(
                    web::scope("/users")
                                .route("/register", web::post().to(register_user))
                                .route("/login", web::post().to(login_user))
                                .route("/logout", web::post().to(logout_user))
                )
                .service(
                    web::scope("/user/protected")
                    .wrap(from_fn(jwt_auth_middleware))
                    .route("/view_user", web::get().to(view_user))
                )
            )       
    })
    .listen(listener)?
    .run();
    Ok(server)
}
