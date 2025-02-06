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

fn diesel_db_response_error(err: &DieselError) -> CustomError{
    // returns CustomError::DatabaseError
    let (resp, status_code) = match err {
        DieselError::DatabaseError(ref kind, ref info) => {
            match kind {
                diesel::result::DatabaseErrorKind::UniqueViolation => {
                    match info.constraint_name() {
                        Some("unique_username") => ("Username has already been taken", StatusCode::BAD_REQUEST),
                        Some("unique_email") => ("Account already associated with this email", StatusCode::BAD_REQUEST),
                        _ => ("internal server error", StatusCode::INTERNAL_SERVER_ERROR)
                    }
                },
                _ => ("internal server error", StatusCode::INTERNAL_SERVER_ERROR)
            }
        },

        DieselError::NotFound => ("Not Found", StatusCode::NOT_FOUND),
        _ => ("internal server error", StatusCode::INTERNAL_SERVER_ERROR)
    };

    CustomError::DatabaseError {
        msg: format!("Database error occured: {:?}", err),
        resp: resp.into(),
        status_code
    }
}
