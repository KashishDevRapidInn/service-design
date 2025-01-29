use actix_web::{web, HttpResponse};
use anyhow::Context;
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel::prelude::OptionalExtension;
use diesel_async::RunQueryDsl;
use errors::{CustomError, DbError};
use flume::Sender;
use kafka::channel::{push_to_broker, KafkaMessage};
use lib_config::db::db::PgPool;
use serde_json::json;
use uuid::Uuid;

use crate::routes::admin::model::{DeleteUserMessage, User};

use super::model::Paginate;

pub async fn get_user(
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
        .map_err(|_| {
            DbError::Other(
                "Failed to get user from database".to_string()
            )
        })?;

    match res {
        Some(user) => Ok(HttpResponse::Ok().json(user)),
        None => Err(DbError::NotFound(user_id.to_string()).into())
    }
}

pub async fn get_user_ids(
    pool: web::Data<PgPool>,
    query: web::Query<Paginate>
) -> Result<HttpResponse, CustomError> {
    use crate::schema::users;

    let query = query.into_inner();
    let offset =  (query.page - 1) * query.limit;

    let mut conn = pool.get()
        .await
        .map_err(|_| {
            DbError::ConnectionError(
                "Failed to get connection from pool".to_string()
            )
        })?;

    let user_ids = users::table
        .select(users::id)
        .offset(offset)
        .limit(query.limit)
        .get_results::<Uuid>(&mut conn)
        .await
        .optional()
        .map_err(|_| {
            DbError::Other(
                "Failed to get user ids from database".to_string()
            )
        })?;

    match user_ids {
        Some(uids) => Ok(HttpResponse::Ok().json(json!({ "page": query.page, "limit": query.limit, "result": uids }))),
        None => Err(DbError::NotFound("No users found".to_string()).into())
    }
}

pub async fn delete_user(
    pool: web::Data<PgPool>,
    user_id: web::Path<Uuid>,
    kafka_producer: web::Data<Sender<KafkaMessage<String>>>
) -> Result<HttpResponse, CustomError> {
    use crate::schema::users;

    let user_id = user_id.into_inner();

    let mut conn = pool.get()
        .await
        .map_err(|_| {
            DbError::ConnectionError(
                "Failed to get connection from pool".to_string()
            )
        })?;

    let res = diesel::delete(users::table)
        .filter(users::id.eq(user_id))
        .execute(&mut conn)
        .await
        .map_err(|e| {
            DbError::Other(e.to_string())
        })?;


    if res == 0 {
        Err(DbError::NotFound(format!("Didn't find user: {}", user_id).to_string()).into())
    } else {
        let message = DeleteUserMessage {
            id: user_id
        };

        let _ = push_to_broker(&kafka_producer, &message).await
            .map_err(|_| tracing::error!("Failed to send message to broker: {:?}", message));

        Ok(HttpResponse::Ok().body(format!("Deleted user: {}", user_id)))
    }
}
