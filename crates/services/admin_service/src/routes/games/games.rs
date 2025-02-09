use anyhow::Context;
use flume::Sender;
use kafka::channel::{push_to_broker, KafkaMessage};
use helpers::auth_jwt::auth::Claims;
use lib_config::db::db::PgPool;
use errors::{AuthError, CustomError};
use actix_web::{http::StatusCode, web, HttpResponse};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use crate::{db_error::DbError, routes::games::models::{CreateGameBody, Game, UpdateGameBody}};
use uuid::Uuid;
use tracing::instrument;
use crate::schema::games::dsl::*;
use lib_config::session::redis::RedisService;

use super::models::KafkaGameMessage;

/******************************************/
// Create New Game Route
/******************************************/
/**
 * @route   POST /api/v1/auth/games/new
 * @access  Private
 */
#[instrument(name = "Create a new game", skip(pool, kafka_producer, admin))]
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
        .context("Failed to get connection from pool")?;
    let admin_id= admin.into_inner().sid;
    let admin_id= Uuid::parse_str(&admin_id).map_err(|err| {
        CustomError::ValidationError(format!("Invalid admin ID format: {}", err))
    })?;
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
        .map_err(DbError)?;

    if result == 0 {
        return Err(CustomError::DatabaseError{
            msg: "Failed to insert user to table admins".into(),
            resp: "Failed to create admin".into(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR
        });
    }

    let message = KafkaGameMessage::Create(&new_game);
    let _ = push_to_broker(&kafka_producer, &message)
        .await
        .context("Failed to send message to broker");

    Ok(HttpResponse::Created().json(new_game))
}
/******************************************/
// Get Game Route
/******************************************/
/**
 * @route   GET /api/v1/auth/games/get/{slug}
 * @access  Private
 */
#[instrument(name = "Get game", skip(pool))]
pub async fn get_game(
    pool: web::Data<PgPool>,
    game_slug: web::Path<String>
) -> Result<HttpResponse, CustomError> {
    let mut conn = pool
        .get()
        .await
        .context("Failed to get connection from pool")?;

    let game_slug = game_slug.into_inner();

    let game: Game = games
        .filter(slug.eq(game_slug))
        .first(&mut conn)
        .await
        .map_err(DbError)?;

    Ok(HttpResponse::Ok().json(game))
}
/******************************************/
// Update Game Route
/******************************************/
/**
 * @route   PATCH /api/v1/auth/games/{slug}
 * @access  Private
 */
#[instrument(name = "Update game", skip(pool, kafka_producer))]
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
        .context("Failed to get connection from pool")?;

    let mut game: Game = games
        .filter(slug.eq(&game_slug))
        .first(&mut conn)
        .await
        .map_err(DbError)?;


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
        .map_err(DbError)?;
    
    let message = KafkaGameMessage::Update { slug: &game_slug, changes: &updated_data };
    let _ = push_to_broker(&kafka_producer, &message)
        .await
        .context("Failed to push update game data to broker")?;

    Ok(HttpResponse::Ok().json(game))
}
/******************************************/
// Delete Game Route
/******************************************/
/**
 * @route   POST /api/v1/auth/games/remove/{slug}
 * @access  Private
 */
#[instrument(name = "Create a new game", skip(pool, kafka_producer))]
pub async fn delete_game(
    game_slug: web::Path<String>,
    pool: web::Data<PgPool>,
    kafka_producer: web::Data<Sender<KafkaMessage<String>>>
) -> Result<HttpResponse, CustomError> {
    let game_slug = game_slug.into_inner();

    let mut conn = pool
        .get()
        .await
        .context("Failed to get connection from pool")?;

    
    let rows_deleted = diesel::delete(games.filter(slug.eq(&game_slug)))
        .execute(&mut conn)
        .await
        .map_err(DbError)?;

    if rows_deleted == 0 {
        return Err(CustomError::DatabaseError{
            msg: "rows_deleted == 0".into(),
            resp: "Did not find game for deletion".into(),
            status_code: StatusCode::NOT_FOUND
        });
    }
    let message = KafkaGameMessage::Delete(&game_slug);
    let _ = push_to_broker(&kafka_producer, &message)
        .await
        .context("Failed to push delete game data to broker")?;
    Ok(HttpResponse::Ok().json("Game deleted successfully"))
}
