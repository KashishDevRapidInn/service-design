use helpers::auth_jwt::auth::create_jwt;
use lib_config::db::db::PgPool;
use errors::{AuthError, CustomError, DbError};
use crate::schema::admins::dsl::*;
use crate::session_state::TypedSession;
use crate::routes::admin::validate_user::validate_credentials;
use helpers::validations::validations::{UserEmail, UserName, CreateUserBody, LoginUserBody, generate_random_salt};
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
 * @route   POST /register
 * @access  Public
 */
#[instrument(name = "Register a new admin", skip(req_admin, pool), fields(username = %req_admin.username, email = %req_admin.email))]
pub async fn register_admin(
    pool: web::Data<PgPool>,
    req_admin: web::Json<CreateUserBody>,
) -> Result<HttpResponse, CustomError> {
    let pool = pool.clone();
    let admin_data = req_admin.into_inner();
    let admin_password = admin_data.password.clone();
    let (validated_name, validated_email) = admin_data
        .validate()
        .map_err(|err| CustomError::ValidationError(err.to_string()))?;
    let admin_id = Uuid::new_v4();
    let mut conn = pool
        .get()
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::ConnectionError(err.to_string())))?;
    let argon2 = Argon2::default();

    let salt = generate_random_salt();
    let password_hashed = argon2
        .hash_password(admin_password.as_bytes(), &salt)
        .map_err(|err| CustomError::HashingError(err.to_string()))?;

    let result = diesel::insert_into(admins)
        .values((
            id.eq(admin_id),
            username.eq(validated_name.as_ref()),
            password_hash.eq(password_hashed.to_string()),
            email.eq(validated_email.as_ref()),
        ))
        .execute(&mut conn)
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::QueryBuilderError(err.to_string())))?;
    if result == 0 {
        return Err(CustomError::DatabaseError(DbError::InsertionError(
            "Failed data insertion in db".to_string(),
        )));
    }
    Ok(HttpResponse::Ok().body("Admin created successfully".to_string()))
}

/******************************************/
// Login Route
/******************************************/
/**
 * @route   POST /login
 * @access  Public
 */
#[instrument(name = "Login a admin", skip(req_login, pool, redis_service), fields(username = %req_login.email))]

pub async fn login_admin(
    pool: web::Data<PgPool>,
    req_login: web::Json<LoginUserBody>,
    redis_service: web::Data<RedisService>,
) -> Result<HttpResponse, CustomError> {
    let admin_id = validate_credentials(&pool, &req_login.into_inner()).await;

    match admin_id {
        Ok(id_admin) => {
            let (token, sid) = create_jwt(&id_admin.to_string()).map_err(|err| {
                CustomError::AuthenticationError(AuthError::JwtAuthenticationError(err.to_string()))
            })?;
            let _= redis_service.set_session(&sid, &id_admin.to_string()).await;
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
// Logout admin Route
/******************************************/
/**
 * @route   POST /protected/logout
 * @access  JWT Protected
 */
#[instrument(name = "Logout a admin", skip(session))]
pub async fn logout_admin(session: TypedSession) -> HttpResponse {
    session.log_out();
    HttpResponse::Ok().body("Logout successfull")
}


// #[instrument(name = "Get user", skip(pool))]
// pub async fn load_by_id(
//     pool: web::Data<PgPool>,
//     admin_id: Uuid
// ) -> Result<HttpResponse, CustomError> {
    
//     let mut conn = pool
//         .get()
//         .await
//         .map_err(|err| CustomError::DatabaseError(DbError::ConnectionError(err.to_string())))?;


//     let user: (String, String) = admins
//         .filter(id.eq(admin_id))
//         .select((username, email))
//         .first(&mut conn)
//         .await
//         .map_err(|err| CustomError::DatabaseError(DbError::QueryBuilderError(err.to_string())))?;
//     Ok(HttpResponse::Ok().json(user))
// }
