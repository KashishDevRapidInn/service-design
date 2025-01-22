use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web::middleware::Logger;
use reqwest::Client;
use std::sync::Arc;
use lib_config::config::configuration;
use utils::telemetry::{get_subscriber, init_subscriber};
use serde_json::Value;
use reqwest::{header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE}};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("api_gateway".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    
    let config = configuration::Settings::new().expect("Failed to load configurations");

    let client = Arc::new(Client::new());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone())) 
            .wrap(Logger::default())
            .route("/user/{endpoint:.*}", web::get().to(forward_user_get_requests)) 
            .route("/user/{endpoint:.*}", web::post().to(forward_user_post_requests)) 
            .route("/admin/{endpoint:.*}", web::to(forward_admin_requests))
            .route("/game/{endpoint:.*}", web::to(forward_game_requests))
    })
    .bind(format!("127.0.0.1:{}", config.service.gateway_service_port))?
    .run()
    .await
}

async fn forward_user_get_requests(
    path: web::Path<String>, 
    client: web::Data<Arc<Client>>,
    req:  HttpRequest
) -> HttpResponse {
    let path = path.into_inner();
    
    let config = configuration::Settings::new().expect("Failed to load configurations");
    let auth_header = req.headers().get(AUTHORIZATION).cloned();

    let mut headers = HeaderMap::new();
    if let Some(auth_header_value) = auth_header {
        headers.insert(AUTHORIZATION, auth_header_value); 
    }
    let response = client
        .get(format!("http://127.0.0.1:{}/{}", config.service.user_service_port, path))
        .headers(headers)
        .send()
        .await
        .expect("Failed to send request to user service");

    let body = response.text().await.expect("Failed to read response");
    HttpResponse::Ok().body(body)
}

// Handle POST requests to user service
async fn forward_user_post_requests(path: web::Path<String>, body: web::Json<Value>, client: web::Data<Arc<Client>>) -> HttpResponse {
    let path = path.into_inner();

    let config = configuration::Settings::new().expect("Failed to load configurations");
    let response = client
        .post(format!("http://127.0.0.1:{}/{}", config.service.user_service_port, path))
        .json(&body)  
        .send()
        .await
        .expect("Failed to send request to user service");

    let body = response.text().await.expect("Failed to read response");
    HttpResponse::Ok().body(body)
}

async fn forward_admin_requests(path: web::Path<String>, client: web::Data<Arc<Client>>) -> HttpResponse {
    let path = path.into_inner();

    let config = configuration::Settings::new().expect("Failed to load configurations");
    let response = client.get(format!("http://127.0.0.1:{}/{}", config.service.admin_service_port, path))
        .send()
        .await
        .expect("Failed to send request to admin service");

    let body = response.text().await.expect("Failed to read response");
    HttpResponse::Ok().body(body)
}

async fn forward_game_requests(path: web::Path<String>, client: web::Data<Arc<Client>>) -> HttpResponse {
    let path = path.into_inner();
    let config = configuration::Settings::new().expect("Failed to load configurations");
    let response = client.get(format!("http://127.0.0.1:{}/{}", config.service.game_service_port, path))
        .send()
        .await
        .expect("Failed to send request to game service");

    let body = response.text().await.expect("Failed to read response");
    HttpResponse::Ok().body(body)
}
