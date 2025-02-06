use std::fmt::Debug;
use std::error::Error;
use actix_web::http::StatusCode;
use actix_web::HttpResponseBuilder;
use actix_web::{HttpResponse, ResponseError};
use serde_json::json;
use thiserror::Error;

#[derive(Error)]
pub enum CustomError {
    #[error("Database Error: {msg}")]
    DatabaseError {
        msg: String,
        resp: String,
        status_code: StatusCode
    },

    #[error("Validation Error: {0}")]
    ValidationError(String),

    #[error("Authentication Error: {0}")]
    AuthenticationError(#[from] AuthError),

    #[error("Unexpected Error Occured")]
    UnexpectedError(#[from] anyhow::Error)
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[from] anyhow::Error),
    #[error("Invalid session")]
    InvalidSession(#[source] anyhow::Error)
}

impl ResponseError for CustomError {
    fn error_response(&self) -> HttpResponse {
        match self {
            CustomError::ValidationError(_) => HttpResponse::BadRequest().json(json!({
                "status": "Failure",
                "message": self.to_string()
            })),
            CustomError::DatabaseError{status_code, resp, ..} => HttpResponseBuilder::new(*status_code).json(json!({
                "status": "Failure",
                "message": resp
            })),
            CustomError::AuthenticationError(_) => HttpResponse::Unauthorized().json(json!({
                "status": "Failure",
                "message": self.to_string()
            })),
            CustomError::UnexpectedError(_) => HttpResponse::InternalServerError().json(json!({
                "status": "Failure",
                "message": self.to_string()
            }))
        }
    }
}

impl Debug for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain(self, f)
    }
}

fn error_chain(
    source: &impl Error,
    f: &mut std::fmt::Formatter
) -> std::fmt::Result {
    writeln!(f, "{}", source)?;

    match source.source() {
        Some(next) => {
            write!(f, "Caused by: \n\t{:?}", next)?;
        },
        None => {}
    };

    Ok(())
}
