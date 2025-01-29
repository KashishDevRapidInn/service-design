use std::{error::Error, fmt::Debug};

use actix_web::{HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Error)]
pub enum CustomError {
    #[error("Database Error: {0}")]
    DatabaseError(#[from] DbError),

    #[error("Validation Error: {0}")]
    ValidationError(String),

    #[error("Authentication Error: {0}")]
    AuthenticationError(#[from] AuthError),

    #[error("Unexpected Error")]
    UnexpectedError(#[from] anyhow::Error)
}

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Connection Error: {0}")]
    ConnectionError(String),

    #[error("Query Error: {0}")]
    QueryBuilderError(String),

    #[error("Insertion Error: {0}")]
    InsertionError(String),

    #[error("Updation Error: {0}")]
    UpdationError(String),

    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Other Database Error: {0}")]
    Other(String),
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Session Authentication Error: {0}")]
    SessionAuthenticationError(String),

    #[error("JWT Authentication Error: {0}")]
    JwtAuthenticationError(String),

    #[error("Other Authentication Error: {0}")]
    OtherAuthenticationError(String),
}
impl ResponseError for CustomError {
    fn error_response(&self) -> HttpResponse {
        match self {
            CustomError::ValidationError(_) => HttpResponse::BadRequest().body(self.to_string()),
            CustomError::DatabaseError(err) => match err {
                DbError::ConnectionError(_) => {
                    HttpResponse::InternalServerError().body(self.to_string())
                }
                DbError::QueryBuilderError(_) => {
                    HttpResponse::InternalServerError().body(self.to_string())
                }
                DbError::InsertionError(_) => {
                    HttpResponse::InternalServerError().body(self.to_string())
                }
                DbError::UpdationError(_) => {
                    HttpResponse::InternalServerError().body(self.to_string())
                }
                DbError::NotFound(_) => {
                    HttpResponse::BadRequest().body(self.to_string()) 
                }
                DbError::Other(_) => HttpResponse::InternalServerError().body(self.to_string()),
            },
            CustomError::AuthenticationError(err) => match err {
                AuthError::SessionAuthenticationError(_) => {
                    HttpResponse::Unauthorized().body(self.to_string())
                }
                AuthError::JwtAuthenticationError(_) => {
                    HttpResponse::Unauthorized().body(self.to_string())
                }
                AuthError::OtherAuthenticationError(_) => {
                    HttpResponse::Unauthorized().body(self.to_string())
                }
            },
            CustomError::UnexpectedError(err) => HttpResponse::InternalServerError().body(self.to_string())
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
            write!(f, "Caused by: \n\t{:?}", next);
        },
        None => {}
    };

    Ok(())
}
