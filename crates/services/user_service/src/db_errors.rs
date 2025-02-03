use std::ops::Deref;
use actix_web::http::StatusCode;
use diesel::result::Error as DieselError;
use errors::CustomError;
use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct DbError(#[from] pub DieselError);

impl Deref for DbError {
    type Target = DieselError;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<DbError> for CustomError{
    fn from(value: DbError) -> Self {
        diesel_db_response_error(&*value)
    }
}

fn diesel_db_response_error(err: &DieselError) -> CustomError {
    match err {
        DieselError::DatabaseError(kind, info) => {
            let msg = match kind {
                diesel::result::DatabaseErrorKind::UniqueViolation => {
                    format!("Unique constraint violation: {}", info.message())
                }
                diesel::result::DatabaseErrorKind::ForeignKeyViolation => {
                    format!("Foreign key constraint violation: {}", info.message())
                }
                diesel::result::DatabaseErrorKind::NotNullViolation => {
                    format!("Not null constraint violation: {}", info.message())
                }
                diesel::result::DatabaseErrorKind::CheckViolation => {
                    format!("Check constraint violation: {}", info.message())
                }
                _ => format!("Database error: {}", info.message()),
            };
            
            CustomError::DatabaseError {
                msg,
                resp: "Database error occurred".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            }
        }
        DieselError::QueryBuilderError(err) => {
            CustomError::DatabaseError {
                msg: format!("Query builder error: {}", err),
                resp: "Error constructing query".to_string(),
                status_code: StatusCode::BAD_REQUEST,
            }
        }
        DieselError::NotFound => {
            CustomError::DatabaseError {
                msg: "Record not found".to_string(),
                resp: "Record not found in the database".to_string(),
                status_code: StatusCode::NOT_FOUND,
            }
        }
        _ => {
            CustomError::DatabaseError {
                msg: format!("Other database error: {}", err),
                resp: "An unexpected error occurred with the database".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            }
        }
    }
}
