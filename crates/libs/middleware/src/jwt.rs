use helpers::auth_jwt::auth::verify_jwt;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::error::ErrorUnauthorized;
use actix_web::{Error, HttpMessage};
use actix_web_lab::middleware::Next;

pub async fn jwt_auth_middleware(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let token = req.headers().get("Authorization");
    if token.is_none() {
        return Err(ErrorUnauthorized("Missing token"));
    }
    let token = token.unwrap().to_str().unwrap_or("").replace("Bearer ", "");
    if token.is_empty() {
        return Err(ErrorUnauthorized("Invalid token"));
    }
    match verify_jwt(&token) {
        Ok(claims) => {
            req.extensions_mut().insert(claims);
            next.call(req).await
        }
        Err(_) => return Err(ErrorUnauthorized("Invalid token")),
    }
}


