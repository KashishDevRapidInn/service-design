use chrono::{NaiveDateTime, Utc};
use helpers::auth_jwt::auth::{create_jwt, Claims, Role};
use flume::Sender;
use kafka::channel::{push_to_broker, KafkaMessage};
use kafka::models::{UserEventType, UserEventsMessage};
use lib_config::db::db::PgPool;
use errors::{AuthError, CustomError};
use crate::db_errors;
use crate::schema::users::dsl::*;
use crate::routes::user::validate_user::validate_credentials;
use helpers::validations::validations::{CreateUserBody, LoginUserBody, generate_random_salt};
use actix_web::{web, HttpResponse, HttpRequest};
use argon2::{self, Argon2, PasswordHasher};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use anyhow::Error;  
use serde_json::json;
use tracing::instrument;
use uuid::Uuid;
use lib_config::session::redis::RedisService;
use actix_web::HttpMessage; //for .extensions()
use anyhow::Context;
use super::response::UserResponse;
use super::model::User;


/******************************************/
// Registering user Route
/******************************************/
/**
 * @route   POST /register
 * @access  Public
 */
#[instrument(name = "Register a new user", skip(req_user, pool), fields(username = %req_user.username, email = %req_user.email))]
pub async fn register_user(
    pool: web::Data<PgPool>,
    req_user: web::Json<CreateUserBody>,
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

    let result: UserEventsMessage = diesel::insert_into(users)
        .values((
            id.eq(user_id),
            username.eq(validated_name.as_ref()),
            password_hash.eq(password_hashed.to_string()),
            email.eq(validated_email.as_ref()),
        ))
        .returning(User::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(|err| db_errors::DbError(err))? 
        .into();

    let _ = push_to_broker(&kafka_producer, &result).await;

    Ok(HttpResponse::Ok().json("User created successfully".to_string()))
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
    redis_service: web::Data<RedisService>,
    kafka_producer: web::Data<Sender<KafkaMessage<String>>>
) -> Result<HttpResponse, CustomError> {
    let user_id = validate_credentials(&pool, &req_login.into_inner()).await?;

    let (token, sid) = create_jwt(&user_id.to_string(), Role::User)?;
    
    let _= redis_service.set_session(&sid, &user_id.to_string()).await?;

    let message = UserEventsMessage{
        user_id,
        event_type: UserEventType::Login { time: Utc::now().naive_utc() }
    };
    let _ = push_to_broker(&kafka_producer, &message)
        .await
        .context("Failed to send message to broker");

    Ok(HttpResponse::Ok().json(json!({"token": token})))

}

/******************************************/
// Logout user Route
/******************************************/
/**
 * @route   POST /user/protected/logout
 * @access  JWT Protected
 */
#[instrument(name = "Logout a user", skip(session, req))]
pub async fn logout_user(
    session: web::Data<RedisService>,
    req: web::ReqData<Claims>,
    kafka_producer: web::Data<Sender<KafkaMessage<String>>>
) -> Result<HttpResponse, CustomError> {
    let session_id= req.into_inner().sid;
    let user_id = session.get_user_from_session(&session_id).await?;

    let uid = Uuid::parse_str(&user_id).map_err(|_| {
        CustomError::AuthenticationError(AuthError::InvalidSession(
            anyhow::anyhow!("Invalid session ID".to_string()),
        ))
    })?;

    let message = UserEventsMessage{
        user_id: uid,
        event_type: UserEventType::Logout { time: Utc::now().naive_utc() }
    };

    let _ = push_to_broker(&kafka_producer, &message)
        .await
        .context("Failed to send message to broker");

    match session.delete_session(&session_id).await {
        Ok(_) => {
            Ok(HttpResponse::Ok().json("Logout successful"))
        }
        Err(err) => {
            eprintln!("Failed to delete session: {:?}", err);
            Err(CustomError::UnexpectedError(anyhow::anyhow!("Failed to log out").into())) 
        }
    }
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
    let session_id= req.into_inner().sid;
    let user_id_str = redis_service.get_user_from_session(&session_id).await?;

    // Parse the session ID string into a UUID
    let user_id = Uuid::parse_str(&user_id_str).map_err(|_| {
        CustomError::AuthenticationError(AuthError::InvalidSession(
            anyhow::anyhow!("Invalid session ID".to_string()),
        ))
    })?;

    let mut conn = pool
        .get()
        .await
        .context("Failed to fetch connection from pool")?;

    let user: UserResponse = users
        .filter(id.eq(user_id))
        .select(UserResponse::as_select())
        .first::<UserResponse>(&mut conn)
        .await
        .map_err(|err| db_errors::DbError(err))? 
        .into();
    Ok(HttpResponse::Ok().json(user))
}
