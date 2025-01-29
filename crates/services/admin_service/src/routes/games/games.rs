use anyhow::Context;
use flume::Sender;
use kafka::channel::{push_to_broker, KafkaMessage};
use helpers::auth_jwt::auth::Claims;
use lib_config::db::db::PgPool;
use errors::{AuthError, CustomError, DbError};
use actix_web::{web, HttpResponse};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use crate::routes::games::models::{Game, CreateGameBody, UpdateGameBody};
use uuid::Uuid;
use tracing::instrument;
use crate::schema::games::dsl::*;
use lib_config::session::redis::RedisService;

use super::models::KafkaGameMessage;

pub async fn create_game(
    pool: web::Data<PgPool>,
    req_game: web::Json<CreateGameBody>, 
    kafka_producer: web::Data<Sender<KafkaMessage<String>>>,
    admin: web::ReqData<Claims>,
) -> Result<HttpResponse, CustomError> {
    let pool = pool.clone();
    let game_data = req_game.into_inner();
    
    let game_slug = game_data.name.to_lowercase().replace(" ", "-");
    let game_id = Uuid::new_v4();
    let game_slug = format!("srv-{}-{}", game_slug, game_id);

    let mut conn = pool
        .get()
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::ConnectionError(err.to_string())))?;
    let admin_id= admin.into_inner().sid;
    let admin_id= Uuid::parse_str(&admin_id).unwrap();
    let new_game = Game {
        slug: game_slug,
        name: game_data.name,
        title: game_data.title,
        description: game_data.description,
        created_at: Some(chrono::Utc::now().naive_utc()),
        created_by_uid: Some(admin_id),
        is_admin: Some(true),
        genre: game_data.genre,
    };

    let result = diesel::insert_into(games)
        .values(&new_game)
        .execute(&mut conn)
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::QueryBuilderError(err.to_string())))?;

    if result == 0 {
        return Err(CustomError::DatabaseError(DbError::InsertionError(
            "Failed to insert game data".to_string(),
        )));
    }

    let message = KafkaGameMessage::Create(&new_game);
    let _ = push_to_broker(&kafka_producer, &message)
        .await
        .context("Failed to send message to broker");

    Ok(HttpResponse::Created().json(new_game))
}

pub async fn get_game(
    pool: web::Data<PgPool>,
    game_slug: web::Path<String>
) -> Result<HttpResponse, CustomError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::ConnectionError(err.to_string())))?;

    let game_slug = game_slug.into_inner();
    
    let mut conn = pool
        .get()
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::ConnectionError(err.to_string())))?;

    let game: Game = games
        .filter(slug.eq(game_slug))
        .first(&mut conn)
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::QueryBuilderError(err.to_string())))?;

    Ok(HttpResponse::Ok().json(game))
}

pub async fn update_game(
    pool: web::Data<PgPool>,
    game_slug: web::Path<String>,
    game_data: web::Json<UpdateGameBody>,
    kafka_producer: web::Data<Sender<KafkaMessage<String>>>
) -> Result<HttpResponse, CustomError> {
    let game_slug = game_slug.into_inner();
    let updated_data = game_data.into_inner();

    let mut conn = pool
        .get()
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::ConnectionError(err.to_string())))?;

    let mut game: Game = games
        .filter(slug.eq(&game_slug))
        .first(&mut conn)
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::QueryBuilderError(err.to_string())))?;


    if let Some(new_title) = updated_data.title.clone() {
        game.title = Some(new_title);
    }
    if let Some(new_description) = updated_data.description.clone() {
        game.description = Some(new_description);
    }
    if let Some(new_genre) = updated_data.genre.clone() {
        game.genre = Some(new_genre);
    }

    diesel::update(games
        .filter(slug.eq(&game_slug)))
        .set(&game)
        .execute(&mut conn)
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::QueryBuilderError(err.to_string())))?;
    
    let message = KafkaGameMessage::Update { slug: &game_slug, changes: &updated_data };
    let _ = push_to_broker(&kafka_producer, &message)
        .await
        .context("Failed to push update game data to broker")?;

    Ok(HttpResponse::Ok().json(game))
}

pub async fn delete_game(
    game_slug: web::Path<String>,
    pool: web::Data<PgPool>,
    kafka_producer: web::Data<Sender<KafkaMessage<String>>>
) -> Result<HttpResponse, CustomError> {
    let game_slug = game_slug.into_inner();

    let mut conn = pool
        .get()
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::ConnectionError(err.to_string())))?;

    
    let rows_deleted = diesel::delete(games.filter(slug.eq(&game_slug)))
        .execute(&mut conn)
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::QueryBuilderError(err.to_string())))?;

    if rows_deleted == 0 {
        return Err(CustomError::DatabaseError(DbError::Other(
            "No game found with the given slug".to_string(),
        )));
    }
    let message = KafkaGameMessage::Delete(&game_slug);
    let _ = push_to_broker(&kafka_producer, &message)
        .await
        .context("Failed to push delete game data to broker")?;
    Ok(HttpResponse::Ok().json("Game deleted successfully"))
}
