use helpers::auth_jwt::auth::{create_jwt, Claims, Role};
use flume::Sender;
use kafka::channel::{push_to_broker, KafkaMessage};
use lib_config::db::db::PgPool;
use errors::{AuthError, CustomError};
use serde::Deserialize;
use crate::db_errors;
use crate::schema::users::dsl::*;
use crate::routes::user::validate_user::validate_credentials;
use helpers::validations::validations::{CreateUserBody, LoginUserBody, generate_random_salt, UpdateUserBody, check_password_strength};
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
use super::model::{UserMessage, User, KafkaUserMessage, EmailVerification};
use lib_config::send_mail::send::send_email;
use helpers::validations::mail_token::generate_token;
use chrono::{Utc, Duration};
use crate::schema::email_verifications::dsl as email_verification_dsl;

/******************************************/
// Registering user Route
/******************************************/
/**
 * @route   POST /register
 * @access  Public
 */
#[instrument(name = "Register a new user", skip(req_user, pool, redis_service), fields(username = %req_user.username, email = %req_user.email))]
pub async fn register_user(
    pool: web::Data<PgPool>,
    req_user: web::Json<CreateUserBody>,
    kafka_producer: web::Data<Sender<KafkaMessage<String>>>,
    redis_service: web::Data<RedisService>
) -> Result<HttpResponse, CustomError> {
    let pool = pool.clone();
    let user_data = req_user.into_inner();
    let user_password = user_data.password.clone();
    let (validated_name, validated_email) = user_data
        .validate()
        .map_err(|err| CustomError::ValidationError(err.to_string()))?;
    let _= check_password_strength(&user_password)?;

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

    let result: UserMessage = diesel::insert_into(users)
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
    let kafka_message = KafkaUserMessage::Create(result);
    let _ = push_to_broker(&kafka_producer, &kafka_message).await;
    
    let (token, sid) = create_jwt(&user_id.to_string(), Role::User)?;
    let _= redis_service.set_session(&sid, &user_id.to_string(), false).await?;

    let mail_token = generate_token();
    let expires_at = Utc::now() + Duration::hours(24); // 24 hours

    diesel::insert_into(email_verification_dsl::email_verifications)
        .values((
            email_verification_dsl::token.eq(&mail_token),
            email_verification_dsl::user_id.eq(&user_id),
            email_verification_dsl::expires_at.eq(&expires_at.naive_utc()),
            email_verification_dsl::status.eq("pending")
        ))
        .execute(&mut conn)
        .await
        .map_err(|err| db_errors::DbError(err))?;
    
    send_email(
            validated_email.as_ref(),
            mail_token
        ).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message":"User created successfully",
        "token": token
    })))    
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
    let user_id = validate_credentials(&pool, &req_login.into_inner()).await?;

    let (token, sid) = create_jwt(&user_id.to_string(), Role::User)?;
    
    let _= redis_service.set_session(&sid, &user_id.to_string(), false).await?;

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
) -> Result<HttpResponse, CustomError> {
    let session_id= req.into_inner().sid;
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


/******************************************/
// Update user Route
/******************************************/
/**
 * @route   PUT /user/protected/update
 * @access  JWT Protected
 */
#[instrument(name = "Update user", skip(req_update, pool, redis_service), fields(username = %req_update.username))]
pub async fn update_user(
    pool: web::Data<PgPool>,
    req_update: web::Json<UpdateUserBody>,
    req: web::ReqData<Claims>, 
    redis_service: web::Data<RedisService>,
    kafka_producer: web::Data<Sender<KafkaMessage<String>>>
) -> Result<HttpResponse, CustomError> {
    let session_id= req.into_inner().sid;
    let user_id_str = redis_service.get_user_from_session(&session_id).await?;

    let user_id = Uuid::parse_str(&user_id_str).map_err(|_| {
        CustomError::AuthenticationError(AuthError::InvalidSession(
            anyhow::anyhow!("Invalid session ID".to_string()),
        ))
    })?;
    let pool = pool.clone();
    let updated_data: UpdateUserBody = req_update.into_inner();
    let (validated_name, validated_email) = updated_data
        .validate()
        .map_err(|err| CustomError::ValidationError(err.to_string()))?;

    let kafka_message= UpdateUserBody{
        username: validated_name.as_ref().to_string(),
        email: validated_email.as_ref().to_string()
    };
    let mut conn = pool
        .get()
        .await
        .context("Failed to fetch connection from pool")?;
    tracing::info!("{:?}", user_id);
    let result= diesel::update(users
        .filter(id.eq(user_id)))
        .set((
            username.eq(validated_name.as_ref()),
            email.eq(validated_email.as_ref()),
            modified_at.eq(chrono::Utc::now().naive_utc()), 
        ))
        .execute(&mut conn)
        .await
        .map_err(|err| db_errors::DbError(err))?;
    if(result==0){
        tracing::info!("couldn't update {:?}", user_id);
        return Err(CustomError::UnexpectedError(anyhow::anyhow!("No user found to update").into()));

    }
    let message = KafkaUserMessage::Update { id: user_id, changes: kafka_message };

    let _ = push_to_broker(&kafka_producer, &message).await;
    Ok(HttpResponse::Ok().json(json!({"message": "User updated successfully"})))
}
#[derive(Deserialize)]
pub struct MailQuery{
    token: String
}
#[instrument(name = "Verify user email", skip(pool, query))]
pub async fn verify_email(
    pool: web::Data<PgPool>,
    query: web::Query<MailQuery>
) -> Result<HttpResponse, CustomError> {
    let query: MailQuery = query.into_inner();
    let token = query.token;

    let mut conn = pool
        .get()
        .await
        .context("Failed to fetch connection from pool")?;

    tracing::info!("token: {}", token);
    let verification_record: Option<EmailVerification>= email_verification_dsl::email_verifications
        .filter(email_verification_dsl::token.eq(token.clone()))
        .filter(email_verification_dsl::expires_at.gt(Utc::now().naive_utc()))
        .filter(email_verification_dsl::status.eq("pending"))
        .select(EmailVerification::as_select())
        .first::<EmailVerification>(&mut conn)
        .await
        .optional()
        .map_err(|err| db_errors::DbError(err))?;

    if verification_record.is_none() {
        return Err(CustomError::UnexpectedError(anyhow::anyhow!("Invalid or expired token".to_string())));
    }

    let _ = diesel::update(email_verification_dsl::email_verifications)
        .filter(email_verification_dsl::token.eq(token))
        .set(email_verification_dsl::status.eq("verified"))
        .execute(&mut conn)
        .await
        .map_err(|err| db_errors::DbError(err))?;

    // let user_id = verification_record.unwrap().user_id;

    // let _ = diesel::update(users)
    //     .filter(users::id.eq(user_id))
    //     .set(users::email_verified.eq(true))
    //     .execute(&mut conn)
    //     .await
    //     .map_err(|err| db_errors::DbError(err))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Email successfully verified"
    })))
}

