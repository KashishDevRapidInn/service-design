use anyhow::Context;
use lib_config::db::db::PgPool;
use errors::{AuthError, CustomError};
use helpers::validations::validations::LoginUserBody;
use crate::{db_error::DbError, schema::admins::dsl::*};
use utils::telemetry::spawn_blocking_with_tracing;
use argon2::{self, Argon2, PasswordHash, PasswordVerifier};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tracing::instrument;
use uuid::Uuid;

#[instrument(name = "Get stored credentials", skip(user_email, pool), fields(username = %user_email))]
async fn get_stored_credentials(
    user_email: &str,
    pool: &PgPool,
) -> Result<(Uuid, String), CustomError> {
    let mut conn = pool
        .get()
        .await
        .context("Failed to get connection from pool")?;

    let row: Option<Vec<(String, Uuid)>> = admins
        .filter(email.eq(user_email))
        .select((password_hash, id))
        .load::<(String, Uuid)>(&mut conn)
        .await
        .optional()
        .map_err(DbError)?;

    let (id_user, expected_hash_password) = match row {
        Some(vec) => {
            if let Some((hash_password, id_user)) = vec.into_iter().next() {
                (id_user, hash_password)
            } else {
                return Err(CustomError::AuthenticationError(
                    AuthError::InvalidCredentials(anyhow::anyhow!(format!("Did not receive entry for email: {}", user_email))),
                ));
            }
        }
        None => {
            return Err(CustomError::AuthenticationError(
                AuthError::InvalidCredentials(anyhow::anyhow!(format!("Did not receive entry for email: {}", user_email))),
            ));
        }
    };
    Ok((id_user, expected_hash_password))
}

#[instrument(name = "Verify password", skip(expected_hash, candidate))]
fn verify_password(expected_hash: &str, candidate: String) -> bool {
    let argon2 = Argon2::default();
    let password_hashed = PasswordHash::new(expected_hash).expect("Failed to parse password hash");

    argon2
        .verify_password(candidate.as_bytes(), &password_hashed)
        .is_ok()
}
#[instrument(name = "Validate credentials", skip(req_login, pool), fields(email = %req_login.email))]
pub async fn validate_credentials(
    pool: &PgPool,
    req_login: &LoginUserBody,
) -> Result<Uuid, CustomError> {
    let (user_id, stored_password_hash) = get_stored_credentials(&req_login.email, pool)
        .await?;

    let entered_pasword = req_login.password.to_owned();
    let is_valid = spawn_blocking_with_tracing(move || {
        verify_password(&stored_password_hash, entered_pasword)
    })
    .await
    .context("Failed to hash inside spawn")?;
    if is_valid {
        return Ok(user_id);
    } else {
        return Err(CustomError::AuthenticationError(
            AuthError::InvalidCredentials(anyhow::anyhow!("Password invalid")),
        ));
    }
}
