use actix_web::{HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CustomError {
    #[error("Database Error: {0}")]
    DatabaseError(#[from] DbError),

    #[error("Blocking Error: {0}")]
    BlockingError(String),

    #[error("Hashing Error: {0}")]
    HashingError(String),

    #[error("Validation Error: {0}")]
    ValidationError(String),

    #[error("Authentication Error: {0}")]
    AuthenticationError(#[from] AuthError),
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
            CustomError::BlockingError(_) => {
                HttpResponse::InternalServerError().body(self.to_string())
            }
            CustomError::ValidationError(_) => HttpResponse::BadRequest().body(self.to_string()),
            CustomError::HashingError(_) => {
                HttpResponse::InternalServerError().body(self.to_string())
            }
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
        }
    }
}
