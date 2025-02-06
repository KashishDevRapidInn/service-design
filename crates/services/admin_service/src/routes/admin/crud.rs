use actix_web::http::StatusCode;
use helpers::auth_jwt::auth::{create_jwt, Claims};
use anyhow::Context;
use helpers::auth_jwt::auth::Role;
use lib_config::db::db::PgPool;
use errors::{AuthError, CustomError};
use crate::{db_error::DbError, schema::admins::dsl::*};
use crate::routes::admin::validate_user::validate_credentials;
use helpers::validations::validations::{check_password_strength, generate_random_salt, CreateUserBody, LoginUserBody, UserEmail, UserName};
use actix_web::{web, HttpResponse};
use argon2::{self, password_hash::SaltString, Argon2, PasswordHasher};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use rand::Rng;
use serde::Deserialize;
use serde_json::json;
use tracing::instrument;
use uuid::Uuid;
use lib_config::session::redis::RedisService;

/******************************************/
// Registering admin Route
/******************************************/
/**
 * @route   POST /ap1/v1/register
 * @access  Public
 */
#[instrument(name = "Register a new admin", skip(req_admin, pool, redis_service), fields(username = %req_admin.username, email = %req_admin.email))]
pub async fn register_admin(
    pool: web::Data<PgPool>,
    req_admin: web::Json<CreateUserBody>,
    redis_service: web::Data<RedisService>
) -> Result<HttpResponse, CustomError> {
    let pool = pool.clone();
    let admin_data = req_admin.into_inner();
    let admin_password = admin_data.password.clone();
    let (validated_name, validated_email) = admin_data
        .validate()
        .map_err(|err| CustomError::ValidationError(err.to_string()))?;
    let _= check_password_strength(&admin_password)?;
    let admin_id = Uuid::new_v4();
    let mut conn = pool
        .get()
        .await
        .context("Failed to fetch connection from pool")?;
    let argon2 = Argon2::default();

    let salt = generate_random_salt();
    let password_hashed = argon2
        .hash_password(admin_password.as_bytes(), &salt)
        .context("Failed to hash password")?;

    let result = diesel::insert_into(admins)
        .values((
            id.eq(admin_id),
            username.eq(validated_name.as_ref()),
            password_hash.eq(password_hashed.to_string()),
            email.eq(validated_email.as_ref()),
        ))
        .execute(&mut conn)
        .await
        .map_err(DbError)?;

    if result == 0 {
        return Err(CustomError::DatabaseError{
            msg: "Failed to insert user to table admins".into(),
            resp: "Failed to create admin".into(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR
        });
    }
    let (token, sid) = create_jwt(&admin_id.to_string(), Role::Admin)?;
    let _= redis_service.set_session(&sid, &admin_id.to_string(), true).await?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "message":"Admin created successfully",
        "token": token
    })))   
}

/******************************************/
// Login Route
/******************************************/
/**
 * @route   POST /ap1/v1/login
 * @access  Public
 */
#[instrument(name = "Login a admin", skip(req_login, pool, redis_service), fields(username = %req_login.email))]

pub async fn login_admin(
    pool: web::Data<PgPool>,
    req_login: web::Json<LoginUserBody>,
    redis_service: web::Data<RedisService>,
) -> Result<HttpResponse, CustomError> {
    let id_admin = validate_credentials(&pool, &req_login.into_inner()).await?;

    let (token, sid) = create_jwt(&id_admin.to_string(), Role::Admin)?;
    redis_service.set_session(&sid, &id_admin.to_string(), true).await?;
    Ok(HttpResponse::Ok().json(json!({"token": token})))
}

/******************************************/
// Logout admin Route
/******************************************/
/**
 * @route   Get /ap1/v1/protected/logout
 * @access  JWT Protected
 */
#[instrument(name = "Logout a admin", skip(session))]
pub async fn logout_admin(
    session: web::Data<RedisService>,
    req: web::ReqData<Claims>,
) -> Result<HttpResponse, CustomError> {
    let session_id= req.into_inner().sid;
    session.delete_session(&session_id).await?; 
    Ok(HttpResponse::Ok().json("Logout successful"))
}
