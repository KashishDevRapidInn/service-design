use flume::Sender;
use kafka::{channel::KafkaMessage, setup::{setup_kafka_receiver, setup_kafka_sender}};
use lib_config::{config::configuration::Settings, db::db::PgPool};
// use crate::middleware::jwt_auth_middleware;
use crate::routes::{
    admin::{crud::{login_admin, logout_admin, register_admin}, user::{delete_user, get_users, get_user_by_id}}, games::games::{create_game, delete_game, get_game, update_game}, health_check::health_check,
};

use crate::kafka_handler::process_kafka_message;
use actix_web::{dev::Server, web, App, HttpServer};
use actix_web_lab::middleware::from_fn;
use middleware::jwt::jwt_auth_middleware;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;
use lib_config::session::redis::RedisService;

/**************************************************************/
// Application State to reuse the same code in main and tests
/***************************************************************/
pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(pool: PgPool, config: &Settings) -> Result<Self, std::io::Error> {
        let listener = if config.service.admin_service_port== 0 {
            TcpListener::bind("127.0.0.1:0")?
        } else {
            let address = format!("127.0.0.1:{}", config.service.admin_service_port);
            TcpListener::bind(&address)?
        };

        let actual_port = listener.local_addr()?.port();

        let consumer_group = "admin_group".to_string();
        let tx = setup_kafka_sender(&config.kafka.admin_url, &config.kafka.admin_topics).await;
        let rx = setup_kafka_receiver(&config.kafka.admin_url, &config.kafka.admin_subscribe_topics, &consumer_group).await;

        let pool_clone = pool.clone();
        tokio::spawn(async move {
            process_kafka_message(rx, pool_clone).await;
        });

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
    let redis_service = RedisService::new(redis_uri).await;
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(kafka_sender.clone()))
            .app_data(web::Data::new(redis_service.clone()))
            .route("/health_check", web::get().to(health_check))
            .service(
                web::scope("/api/v1")
                .service(
                    web::scope("/admins")
                                .route("/register", web::post().to(register_admin))
                                .route("/login", web::post().to(login_admin))
                )
                .service(
                    web::scope("/protected")
                    .wrap(from_fn(jwt_auth_middleware))
                    .route("/logout", web::get().to(logout_admin))
                )
                .service(
                    web::scope("/auth/games")
                    .wrap(from_fn(jwt_auth_middleware))
                    .route("/new", web::post().to(create_game))
                    .route("/get/{slug}", web::get().to(get_game))
                    .route("/update/{slug}", web::patch().to(update_game))
                    .route("/remove/{slug}", web::delete().to(delete_game))
                )
                .service(
                    web::scope("/auth/users")
                    .wrap(from_fn(jwt_auth_middleware))
                    .route("/", web::get().to(get_users))
                    .route("/{user_id}", web::get().to(get_user_by_id))
                    .route("/{user_id}", web::delete().to(delete_user))
                )
            )       
    })
    .listen(listener)?
    .run();
    Ok(server)
}
