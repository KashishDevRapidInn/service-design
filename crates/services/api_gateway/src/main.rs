use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web::middleware::Logger;
use reqwest::Client;
use std::sync::Arc;
use lib_config::config::configuration;
use utils::telemetry::{get_subscriber, init_subscriber};
use serde_json::Value;
use reqwest::{header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE}};

use lib_config::session::redis::RedisService;
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("api_gateway".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    
    let config = configuration::Settings::new().expect("Failed to load configurations");

    let client = Arc::new(Client::new());
    let redis_service = RedisService::new(config.redis.uri).await;
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone())) 
            .wrap(Logger::default())
            .app_data(web::Data::new(redis_service.clone()))
            .route("/{service}/{endpoint:.*}", web::to(forward_requests))
    })
    .bind(format!("127.0.0.1:{}", config.service.gateway_service_port))?
    .run()
    .await
}
async fn forward_requests(
    path: web::Path<(String, String)>, 
    client: web::Data<Arc<Client>>,
    req: HttpRequest,
    body: Option<web::Json<Value>>,
) -> HttpResponse {
    let (service, endpoint) = path.into_inner();
    
    let config = configuration::Settings::new().expect("Failed to load configurations");

    let base_url = match service.as_str() {
        "user" => format!("http://127.0.0.1:{}/{}", config.service.user_service_port, endpoint),
        "admin" => format!("http://127.0.0.1:{}/{}", config.service.admin_service_port, endpoint),
        "game" => format!("http://127.0.0.1:{}/{}", config.service.game_service_port, endpoint),
        _ => return HttpResponse::BadRequest().body("Invalid service name"),
    };

    let auth_header = req.headers().get(AUTHORIZATION).cloned();
    let mut headers = HeaderMap::new();
    if let Some(auth_header_value) = auth_header {
        headers.insert(AUTHORIZATION, auth_header_value);
    }

    let method = req.method().clone();
    
    let response = match method {
        actix_web::http::Method::GET => {
            client
                .get(&base_url)
                .headers(headers)
                .send()
                .await
                .expect("Failed to send GET request")
        },
        actix_web::http::Method::POST => {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            let json_body = match body {
                Some(b) => serde_json::to_string(&b).expect("Failed to serialize body to JSON"),
                None => return HttpResponse::BadRequest().body("Expected body for POST request"),
            };
            client
                .post(&base_url)
                .headers(headers)
                .body(json_body)
                .send()
                .await
                .expect("Failed to send POST request")
        },
        actix_web::http::Method::DELETE => {
            client
                .delete(&base_url)
                .headers(headers)
                .send()
                .await
                .expect("Failed to send DELETE request")
        },
        actix_web::http::Method::PATCH => {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            let json_body = match body {
                Some(b) => serde_json::to_string(&b).expect("Failed to serialize body to JSON"),
                None => return HttpResponse::BadRequest().body("Expected body for PATCH request"),
            };
            client
                .patch(&base_url)
                .headers(headers)
                .body(json_body)
                .send()
                .await
                .expect("Failed to send PATCH request")
        },
        _ => return HttpResponse::MethodNotAllowed().body("Method not allowed"),
    };

    let body = response.text().await.expect("Failed to read response");
    HttpResponse::Ok().body(body)
}