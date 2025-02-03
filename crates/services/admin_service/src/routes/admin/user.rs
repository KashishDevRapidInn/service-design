use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel::prelude::OptionalExtension;
use diesel_async::RunQueryDsl;
use errors::{AuthError, CustomError};
use flume::Sender;
use kafka::channel::{push_to_broker, KafkaMessage};
use lib_config::db::db::PgPool;
use serde_json::json;
use uuid::Uuid;

use crate::db_error::DbError;
use crate::routes::admin::model::{DeleteUserMessage, User};

use super::model::Paginate;
use tracing::instrument;

/******************************************/
// Get user Route
/******************************************/
/**
 * @route   GET /ap1/v1/auth/users/{user_id}
 * @access  Privare
 */
#[instrument(name = "Get user by id", skip(user_id, pool))]
pub async fn get_user_by_id(
    pool: web::Data<PgPool>,
    user_id: web::Path<Uuid>
) -> Result<HttpResponse, CustomError> {
    use crate::schema::users;

    let user_id = user_id.into_inner();

    let mut conn = pool.get()
        .await
        .context("Failed to get connection from pool")?;

    let res = users::table
        .select(User::as_select())
        .filter(users::id.eq(&user_id))
        .get_result(&mut conn)
        .await
        .optional()
        .map_err(DbError)?
        .ok_or(CustomError::DatabaseError {
            msg: "User not found by user id".into(),
            resp: "User not found".into(),
            status_code: StatusCode::NOT_FOUND
        })?;

    Ok(HttpResponse::Ok().json(res))
}
/******************************************/
// Get user by ID Route
/******************************************/
/**
 * @route   GET /ap1/v1/auth/users/
 * @access  Private
 */
#[instrument(name = "Get users", skip(query, pool))]
pub async fn get_users(
    pool: web::Data<PgPool>,
    query: web::Query<Paginate>
) -> Result<HttpResponse, CustomError> {
    use crate::schema::users;

    let query = query.into_inner();
    let offset =  (query.page - 1) * query.limit;

    let mut conn = pool.get()
        .await
        .context("Failed to get connection from pool")?;

    let user_ids = users::table
        .select(users::id)
        .offset(offset)
        .limit(query.limit)
        .get_results::<Uuid>(&mut conn)
        .await
        .map_err(DbError)?;

    if user_ids.len() == 0 {
        return Ok(HttpResponse::NotFound().json(json!({ "page": query.page, "limit": query.limit, "message": "No more results" })))
    }

    Ok(HttpResponse::Ok().json(json!({ "page": query.page, "limit": query.limit, "result": user_ids })))
}
/******************************************/
// Dekete user by ID Route
/******************************************/
/**
 * @route   DELETE /ap1/v1/auth/users/{user_id}
 * @access  Private
 */
#[instrument(name = "Delete user", skip(user_id, pool, kafka_producer))]
pub async fn delete_user(
    pool: web::Data<PgPool>,
    user_id: web::Path<Uuid>,
    kafka_producer: web::Data<Sender<KafkaMessage<String>>>
) -> Result<HttpResponse, CustomError> {
    use crate::schema::users;

    let user_id = user_id.into_inner();

    let mut conn = pool.get()
        .await
        .context("Failed to get connection from pool")?;

    let res = diesel::delete(users::table)
        .filter(users::id.eq(user_id))
        .execute(&mut conn)
        .await
        .map_err(DbError)?;


    if res == 0 {
        Err(CustomError::DatabaseError {
            msg: "Returned usize == 0".into(),
            resp: "Did not find user".into(),
            status_code: StatusCode::NOT_FOUND
        })
    } else {
        let message = DeleteUserMessage {
            id: user_id
        };

        let _ = push_to_broker(&kafka_producer, &message).await
            .map_err(|_| tracing::error!("Failed to send message to broker: {:?}", message));

        Ok(HttpResponse::Ok().json(format!("Deleted user: {}", user_id)))
    }
}
