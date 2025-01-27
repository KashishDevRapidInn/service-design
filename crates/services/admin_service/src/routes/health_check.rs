use actix_web::HttpResponse;
use crate::session_state::TypedSession;
use uuid::Uuid;

/******************************************/
// Health check route
/******************************************/
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json("admin application working")
}

/******************************************/
// Session check routes
/******************************************/
pub async fn set_session(typed_session: TypedSession) -> HttpResponse {
    let user_id = Uuid::new_v4();
    typed_session.insert_user_id(user_id).unwrap();
    HttpResponse::Ok().json("User ID inserted into session")
}

pub async fn get_session(typed_session: TypedSession) -> HttpResponse {
    match typed_session.get_user_id() {
        Ok(Some(user_id)) => HttpResponse::Ok().json(format!("User ID: {}", user_id)),
        Ok(None) => HttpResponse::BadRequest().json("No user ID in session"),
        Err(_) => HttpResponse::InternalServerError().json("Error accessing session"),
    }
}