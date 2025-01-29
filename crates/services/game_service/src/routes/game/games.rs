use helpers::auth_jwt::auth::Claims;
use lib_config::db::db::PgPool;
use errors::{AuthError, CustomError, DbError};
use crate::kafka_handler::ReceivedGame;
use crate::schema::users::dsl::*;
use crate::schema::rate_game::dsl::*;
use crate::routes::game::model::{Game, RateGame, RateGameRequest};
use actix_web::{web, HttpResponse, HttpRequest};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;
use lib_config::session::redis::RedisService;
use serde_json::json;
use tracing::instrument;
use chrono::Utc;

/******************************************/
// Rate game Route
/******************************************/
/**
 * @route   POST /rate
 * @access  Protected
 */
#[instrument(name = "Rate game", skip(pool, rate_game_req, req, redis_service))]
pub async fn rate(
    pool: web::Data<PgPool>,
    rate_game_req: web::Json<RateGameRequest>, 
    req: web::ReqData<Claims>,
    redis_service: web::Data<RedisService>
) -> Result<HttpResponse, CustomError>{
    let mut conn = pool
        .get()
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::ConnectionError(err.to_string())))?;

    let rate_game_req = rate_game_req.into_inner();
    let slug = rate_game_req.game_slug;
    let id_user = req.into_inner().sub;
    let game_rating = rate_game_req.rating;
    let game_review: Option<String> = rate_game_req.review;

    if rate_game_req.rating < 1 || rate_game_req.rating > 5 {
        return Err(CustomError::ValidationError("Rating must be between 1 and 5.".to_string()));
    }
    let id_user = Uuid::parse_str(&id_user).map_err(|_| {
        CustomError::AuthenticationError(AuthError::SessionAuthenticationError(
            "Invalid user ID".to_string(),
        ))
    })?;
    // Create a new RateGame instance
    let new_rate_game = RateGame {
        id: uuid::Uuid::new_v4(),
        game_slug: slug,
        user_id: id_user,
        rating: game_rating,
        review: game_review,
        created_at: Utc::now().naive_utc(),
    };

    let result= diesel::insert_into(rate_game)
        .values(&new_rate_game)
        .execute(&mut conn)
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::InsertionError(err.to_string())))?;


    Ok(HttpResponse::Ok().json("Rating successfully added."))
}

pub async fn get_game_by_slug(
    slug_game: &str,
    pool: PgPool,
) -> Result<ReceivedGame, CustomError> {
   let mut conn = pool
   .get()
   .await
   .map_err(|err| CustomError::DatabaseError(DbError::ConnectionError(err.to_string())))?;

    use crate::schema::games;
    use crate::schema::games::dsl::*;
    let response= games.filter(games::slug.eq(slug_game))
    .select((ReceivedGame::as_select()))
    .get_result::<ReceivedGame>(&mut conn)
    .await
    .map_err(|err| CustomError::DatabaseError(DbError::InsertionError(err.to_string())))?;
    Ok((response))
}