use helpers::auth_jwt::auth::{create_jwt, Claims};
use flume::Sender;
use kafka::channel::{push_to_broker, KafkaMessage};
use lib_config::db::db::PgPool;
use errors::{AuthError, CustomError, DbError};
use crate::schema::users::dsl::*;
use crate::session_state::TypedSession;
use crate::routes::user::validate_user::validate_credentials;
use helpers::validations::validations::{CreateUserBody, LoginUserBody, generate_random_salt};
use actix_web::{web, HttpResponse, HttpRequest};
use argon2::{self, Argon2, PasswordHasher};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use serde_json::json;
use tracing::instrument;
use uuid::Uuid;
use actix_web::cookie::{Cookie, CookieJar};
use actix_web::cookie::time::Duration;
use lib_config::session::redis::RedisService;
use actix_web::HttpMessage; //for .extensions()
use anyhow::Context;

use super::model::{RegisterUserMessage, User};


/******************************************/
// Registering user Route
/******************************************/
/**
 * @route   POST /register
 * @access  Public
 */
#[instrument(name = "Register a new user", skip(req_user, pool, session), fields(username = %req_user.username, email = %req_user.email))]
pub async fn register_user(
    pool: web::Data<PgPool>,
    req_user: web::Json<CreateUserBody>,
    session: TypedSession,
    kafka_producer: web::Data<Sender<KafkaMessage<String>>>
) -> Result<HttpResponse, CustomError> {
    let pool = pool.clone();
    let user_data = req_user.into_inner();
    let user_password = user_data.password.clone();
    let (validated_name, validated_email) = user_data
        .validate()
        .map_err(|err| CustomError::ValidationError(err.to_string()))?;
    let user_id = Uuid::new_v4();
    let mut conn = pool
        .get()
        .await
        .context("Failed to fetch connection from pool")?;
    let argon2 = Argon2::default();

    let salt = generate_random_salt();
    let password_hashed = argon2
        .hash_password(user_password.as_bytes(), &salt)
        .context("Failed to hash password")?;

    let result: RegisterUserMessage = diesel::insert_into(users)
        .values((
            id.eq(user_id),
            username.eq(validated_name.as_ref()),
            password_hash.eq(password_hashed.to_string()),
            email.eq(validated_email.as_ref()),
        ))
        .returning(User::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::InsertionError(err.to_string())))?
        .into();

    let _ = push_to_broker(&kafka_producer, &result).await;

    let _ = session.insert_user_id(user_id);
    Ok(HttpResponse::Ok().body("User created successfully".to_string()))
}

/******************************************/
// Login Route
/******************************************/
/**
 * @route   POST /login
 * @access  Public
 */
#[instrument(name = "Login a customer", skip(req_login, pool, redis_service), fields(username = %req_login.email))]

pub async fn login_user(
    pool: web::Data<PgPool>,
    req_login: web::Json<LoginUserBody>,
    redis_service: web::Data<RedisService>
) -> Result<HttpResponse, CustomError> {
    let user_id = validate_credentials(&pool, &req_login.into_inner()).await;

    match user_id {
        Ok(id_user) => {

            let (token, sid) = create_jwt(&id_user.to_string()).map_err(|err| {
                CustomError::AuthenticationError(AuthError::JwtAuthenticationError(err.to_string()))
            })?;
           
            let _= redis_service.set_session(&sid, &id_user.to_string()).await;

            Ok(HttpResponse::Ok().json(json!({"token": token})))
        }
        Err(err) => {
            return Err(CustomError::AuthenticationError(
                AuthError::OtherAuthenticationError(err.to_string()),
            ));
        }
    }
}

/******************************************/
// Logout user Route
/******************************************/
/**
 * @route   POST /protected/logout
 * @access  JWT Protected
 */
#[instrument(name = "Logout a user", skip(session))]
pub async fn logout_user(session: TypedSession) -> HttpResponse {
    session.log_out();
    HttpResponse::Ok().body("Logout successfull")
}

/******************************************/
// View user Info Route
/******************************************/
/**
 * @route   Get /protected/view
 * @access  JWT Protected
 */
#[instrument(name = "Get user", skip(pool, req, redis_service))]
pub async fn view_user(
    pool: web::Data<PgPool>,
    req: web::ReqData<Claims>,
    redis_service: web::Data<RedisService>
) -> Result<HttpResponse, CustomError> {
    // let session_id: String = req
    //     .extensions()
    //     .get::<String>()
    //     .ok_or_else(|| CustomError::AuthenticationError(AuthError::SessionAuthenticationError("Session not found".to_string())))?
    //     .clone();
    let session_id= req.into_inner().sid;

    let user_id_str = redis_service.get_session(session_id).await.map_err(|_| {
        CustomError::AuthenticationError(AuthError::SessionAuthenticationError(
            "User not found".to_string(),
        ))
    })?;
    let user_id_str = match user_id_str {
        Some(id_user) => id_user,
        None => {
            return Err(CustomError::AuthenticationError(
                AuthError::SessionAuthenticationError("User not found".to_string()),
            ));
        }
    };
    // Parse the session ID string into a UUID
    let user_id = Uuid::parse_str(&user_id_str).map_err(|_| {
        CustomError::AuthenticationError(AuthError::SessionAuthenticationError(
            "Invalid session ID".to_string(),
        ))
    })?;

    let mut conn = pool
        .get()
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::ConnectionError(err.to_string())))?;
    // if user_id.is_none() {
    //     return Err(CustomError::AuthenticationError(
    //         AuthError::SessionAuthenticationError("User not found".to_string()),
    //     ));
    // }
    // let user_id = user_id.unwrap();

    let user: (String, String) = users
        .filter(id.eq(user_id))
        .select((username, email))
        .first(&mut conn)
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::QueryBuilderError(err.to_string())))?;
    Ok(HttpResponse::Ok().json(user))
}
