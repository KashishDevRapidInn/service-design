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
use crate::elasticsearch::ElasticsearchGame;
use elasticsearch::{Elasticsearch, SearchParts};

use super::model::Paginate;

/******************************************/
// Rate game Route
/******************************************/
/**
 * @route   POST /rate
 * @access  Protected
 */
#[instrument(name = "Rate game", skip(pool, rate_game_req, req, elastic_client))]
pub async fn rate(
    pool: web::Data<PgPool>,
    rate_game_req: web::Json<RateGameRequest>, 
    req: web::ReqData<Claims>,
    elastic_client: web::Data<Elasticsearch>,
) -> Result<HttpResponse, CustomError>{
    let pool_clone= pool.clone();
    let pool_ref= pool_clone.as_ref();
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
    let new_rate_game = RateGame {
        id: uuid::Uuid::new_v4(),
        game_slug: slug.clone(),
        user_id: id_user,
        rating: game_rating.clone(),
        review: game_review,
        created_at: Utc::now().naive_utc(),
    };

    let result= diesel::insert_into(rate_game)
        .values(&new_rate_game)
        .execute(&mut conn)
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::InsertionError(err.to_string())))?;

    let new_game= get_game_by_slug(&slug, pool_ref).await.map_err(|err|CustomError::DatabaseError(DbError::Other(err.to_string())))?;
    let es_game = ElasticsearchGame::new(&new_game);
    let current_game = elastic_client
        .get(elasticsearch::GetParts::IndexId("rate", &es_game.slug))
        .send()
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::Other(err.to_string())))?;

        let json_response = current_game.json::<serde_json::Value>().await.ok();
        
        let (current_avg_rating, current_rating_count) = if let Some(hit) = json_response {
            let source = hit.get("_source");
            if let Some(res) = source {
                tracing::info!("Elasticsearch response source: {}", res);
                let avg_rating = res.get("average_rating")
                                    .and_then(|r| r.as_f64())
                                    .unwrap_or(0.0) as f32;
                let rating_count = res.get("rating_count")
                                      .and_then(|r| r.as_i64())
                                      .unwrap_or(0) as i32;
                tracing::info!("Found rating: avg_rating = {}, rating_count = {}", avg_rating, rating_count);
                (avg_rating, rating_count)
            } else {
                tracing::error!("NO source in elasticsearch response");
                (0.0, 0)
            }
        } else {
            tracing::error!("No data found in the Elasticsearch response.");
            (0.0, 0)
        };
    let new_rating_count = current_rating_count + 1;
    let new_average_rating = ((current_avg_rating * current_rating_count as f32) + game_rating as f32) / new_rating_count as f32;
    
    ElasticsearchGame::update_game(&elastic_client, &es_game, Some(new_average_rating), Some(new_rating_count)).await.unwrap();

    Ok(HttpResponse::Ok().json("Rating successfully added."))
}

#[instrument(name = "Get game list", skip_all)]
pub async fn get_game(
    paginate: web::Query<Paginate>,
    elastic_client: web::Data<Elasticsearch>
) -> Result<HttpResponse, CustomError> {
    let paginate = paginate.into_inner();
    let from = (paginate.page - 1) * paginate.limit;

    let response = elastic_client
        .search(SearchParts::Index(&vec![&String::from("rate")[..]]))
        .body(json!({
            "sort": [
                {
                    "average_rating": {
                        "order": "desc"
                    }
                }
            ],
            "from": from,
            "size": paginate.limit
        }))
        .send()
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::Other(err.to_string())))?;

    let response_json = response.json::<serde_json::Value>()
        .await
        .map_err(|err| CustomError::DatabaseError(DbError::Other(err.to_string())))?;

    let hits = response_json.get("hits")
        .ok_or(CustomError::DatabaseError(DbError::Other("Did not find outer hits".into())))?
        .get("hits")
        .ok_or(CustomError::DatabaseError(DbError::Other("Did not find inner hits".into())))?;

    let ret = match hits {
        serde_json::Value::Array(res_vec) => {
            let temp: Vec<ElasticsearchGame> = res_vec.into_iter()
                .map(|res| {
                    let temp = res.get("_source")
                        .ok_or(
                            CustomError::DatabaseError(DbError::Other(
                                    "Unexpected result structure: Did not find _source".into()
                            )
                        ));

                    temp
                })
                .map(|source| {
                    let temp = source.map(|doc| {
                        let game: Result<ElasticsearchGame, serde_json::Error> = serde_json::from_value(doc.clone());
                        game.map_err(|_| {
                            CustomError::DatabaseError(DbError::Other("Failed to deserialize to ElasticsearchGame".into()))
                        })
                    })?;

                    temp
                })
                .collect::<Result<Vec<_>, _>>()?;

            Ok(temp)
        }

        _ => {
            Err(CustomError::DatabaseError(DbError::Other("Unexpected data structure".into())))
        }
    }?;

    if ret.len() == 0 {
        tracing::info!("No more games");
        return Ok(HttpResponse::Ok().body("No more games"))
    }

    Ok(HttpResponse::Ok().json(ret))
}

#[instrument(name = "Get game by slug", skip(pool, slug_game))]
pub async fn get_game_by_slug(
    slug_game: &str,
    pool: &PgPool,
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
