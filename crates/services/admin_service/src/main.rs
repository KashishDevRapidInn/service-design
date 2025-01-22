use lib_config::config::configuration;
use lib_config::db::db::establish_connection;
use admin_service::startup::Application;
use utils::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("admin_srv".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let config = configuration::Settings::new().expect("Failed to load configurations");
    let pool = establish_connection(&config.databases.admin_db_url).await;
    let port = config.service.admin_service_port;

    let application = Application::build(port, pool, config.redis.uri).await?;
    application.run_until_stopped().await?;
    Ok(())
}
