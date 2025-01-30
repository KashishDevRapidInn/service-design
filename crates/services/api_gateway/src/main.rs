use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web::middleware::Logger;
use errors::CustomError;
use reqwest::Client;
use std::sync::Arc;
use lib_config::config::configuration;
use utils::telemetry::{get_subscriber, init_subscriber};
use serde_json::{Value, to_string_pretty};
use reqwest::{header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE}};
use serde::{Serialize, Deserialize};
use actix_web::http::StatusCode;

use lib_config::session::redis::RedisService;
use bytes::Bytes;
use anyhow;
use tracing::instrument;
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
    .bind(format!("{}:{}", config.domain.gateway_service_domain, config.service.gateway_service_port))?
    .run()
    .await
}
#[instrument("forward requests", skip(client, req, body))]
pub(crate) async fn forward_requests(
    path: web::Path<(String, String)>, 
    client: web::Data<Arc<Client>>,
    req: HttpRequest,
    body: Option<web::Json<Value>>,
) -> Result<HttpResponse, CustomError> {
    let (service, endpoint) = path.into_inner();
    
    let config = match configuration::Settings::new() {
        Ok(conf) => conf,
        Err(_) => {
            let err_msg = "Failed to load configurations".to_string();
            return Err(CustomError::UnexpectedError(anyhow::Error::msg(err_msg)));
        }
    };

    let base_url = match service.as_str() {
        "user" => format!("http://{}:{}/{}", config.domain.user_service_domain, config.service.user_service_port, endpoint),
        "admin" => format!("http://{}:{}/{}", config.domain.admin_service_domain, config.service.admin_service_port, endpoint),
        "game" => format!("http://{}:{}/{}", config.domain.game_service_domain, config.service.game_service_port, endpoint),
        _ => {
            let err_msg = "Invalid service name".to_string();
            return Err(CustomError::ValidationError(err_msg));
        }
    };

    let query_string = req.query_string();
    let mut full_url = base_url;
    if !query_string.is_empty() {
        full_url.push_str("?");
        full_url.push_str(query_string);
    }

    let auth_header = req.headers().get("Authorization").cloned();
    let mut headers = HeaderMap::new();
    if let Some(auth_header_value) = auth_header {
        headers.insert("Authorization", auth_header_value);
    }
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    let mut request_builder = client.request(req.method().clone(), full_url)
        .headers(headers);

    let request_builder = request_builder.body(match body {
        Some(b) => serde_json::to_string(&b).unwrap_or_default(),
        None => "".to_string(),
    });

    let response = request_builder.send().await;

    match response {
        Ok(mut resp) => {
            let status = resp.status().clone();
            let headers = resp.headers().clone();
            let body = resp.bytes().await.unwrap_or_else(|_| Bytes::from("Failed to read response body"));

            let mut http_response = HttpResponse::build(status);
            for (header_name, header_value) in headers.iter() {
                http_response.insert_header((
                    header_name.as_str(),
                    header_value.to_str().unwrap_or_default().to_string(),
                ));
            }
            Ok(http_response.body(body))
        },
        Err(err) => {
            let err_msg = format!("Error forwarding request: {:#?}", err);
            Err(CustomError::UnexpectedError(anyhow::Error::msg(err_msg)))
        }
    }
}