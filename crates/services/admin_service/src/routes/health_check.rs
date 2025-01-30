use actix_web::HttpResponse;
use uuid::Uuid;

/******************************************/
// Health check route
/******************************************/
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json("admin application working")
}

