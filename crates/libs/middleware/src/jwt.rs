use helpers::auth_jwt::auth::{verify_jwt, Role};
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::error::ErrorUnauthorized;
use actix_web::{Error, HttpMessage};
use actix_web_lab::middleware::Next;

pub async fn jwt_auth_middleware<T: RoleRestrictor>(
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
            if claims.role != T::role_allowed() {
                return Err(ErrorUnauthorized("Invalid role"));
            }
            req.extensions_mut().insert(claims);
            next.call(req).await
        }
        Err(_) => return Err(ErrorUnauthorized("Invalid token")),
    }
}

pub trait RoleRestrictor {
    fn role_allowed() -> Role;
}
