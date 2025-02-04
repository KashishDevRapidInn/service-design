use helpers::auth_jwt::auth::Claims;
use lib_config::db::db::PgPool;
use errors::{AuthError, CustomError};
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
use anyhow::Context;
use super::model::Paginate;
use  crate::db_error;
/******************************************/
// Rate game Route
/******************************************/
/**
 * @route   POST /rate
 * @access  Protected
 */
#[instrument(name = "Rate game", skip(pool, rate_game_req, req, elastic_client, redis_service))]
pub async fn rate(
    pool: web::Data<PgPool>,
    rate_game_req: web::Json<RateGameRequest>, 
    req: web::ReqData<Claims>,
    elastic_client: web::Data<Elasticsearch>,
    redis_service: web::Data<RedisService>
) -> Result<HttpResponse, CustomError>{
    let req = req.into_inner();
    let session_id= req.sid;

    let _ = redis_service.get_user_from_session(&session_id).await?;

    let pool_clone= pool.clone();
    let pool_ref= pool_clone.as_ref();
    let mut conn = pool
        .get()
        .await
        .context("Failed to fetch connection from pool")?;

    let rate_game_req = rate_game_req.into_inner();
    let slug = rate_game_req.game_slug;
    let id_user = req.sub;
    let game_rating = rate_game_req.rating;
    let game_review: Option<String> = rate_game_req.review;

    if rate_game_req.rating < 1 || rate_game_req.rating > 5 {
        return Err(CustomError::ValidationError("Rating must be between 1 and 5.".to_string()));
    }
    let id_user = Uuid::parse_str(&id_user).map_err(|_| {
        CustomError::AuthenticationError(AuthError::InvalidSession(
            anyhow::anyhow!("Invalid session ID".to_string()),
        ))
    })?;
    tracing::info!("User id: {}", id_user);

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
        .map_err(|err| db_error::DbError(err))?;

    let new_game= get_game_by_slug(&slug, pool_ref)
    .await?;

    let es_game = ElasticsearchGame::new(&new_game);
    let current_game = elastic_client
        .get(elasticsearch::GetParts::IndexId("rate", &es_game.slug))
        .send()
        .await
        .map_err(|err| 
            CustomError::UnexpectedError(anyhow::anyhow!("Failed to retrieve game data").into())
        )?;

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
    elastic_client: web::Data<Elasticsearch>,
    req: web::ReqData<Claims>,
    redis_service: web::Data<RedisService>
) -> Result<HttpResponse, CustomError> {
    let session_id= req.into_inner().sid;
    let _ = redis_service.get_user_from_session(&session_id).await?;

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
        .map_err(|err| {
            tracing::error!("Failed to fetch game data Elasticsearch response: {:?}", err);
            CustomError::UnexpectedError(anyhow::anyhow!("Failed to get response from elasticsearch").into())
        })?;

    let response_json = response.json::<serde_json::Value>()
        .await
        .map_err(|err| {
            tracing::error!("Failed to parse Elasticsearch response: {:?}", err);
            CustomError::UnexpectedError(anyhow::anyhow!("Error parsing response").into())
        })?;

    tracing::info!("{}", response_json.to_string());

    let hits = response_json.get("hits")
        .ok_or(CustomError::UnexpectedError(anyhow::anyhow!("Failed to fetch outer hits field").into()))?
        .get("hits")
        .ok_or(CustomError::UnexpectedError(anyhow::anyhow!("Failed to fetch inner hits field").into()))?;

    let ret = match hits {
        serde_json::Value::Array(res_vec) => {
            let temp: Vec<ElasticsearchGame> = res_vec.into_iter()
                .map(|res| {
                    let temp = res.get("_source")
                        .ok_or(
                            CustomError::UnexpectedError(anyhow::anyhow!(
                                "Failed to fetch \"_source\" field"
                            ).into()
                        ));

                    temp
                })
                .map(|source| {
                    let temp = source.map(|doc| {
                        let game: Result<ElasticsearchGame, serde_json::Error> = serde_json::from_value(doc.clone());
                        game.map_err(|_| {
                            CustomError::UnexpectedError(anyhow::anyhow!("Failed to deserialize \"_source\"").into())
                        })
                    })?;

                    temp
                })
                .collect::<Result<Vec<_>, _>>()?;

            Ok(temp)
        }

        _ => {
            Err(CustomError::UnexpectedError(anyhow::anyhow!("Failed to fetch game data").into()))
        }
    }?;

    if ret.len() == 0 {
        tracing::info!("No more games");
        return Ok(HttpResponse::Ok().json("No more games"))
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
   .context("Failed to fetch connection from pool")?;

    use crate::schema::games;
    use crate::schema::games::dsl::*;
    let response= games.filter(games::slug.eq(slug_game))
    .select((ReceivedGame::as_select()))
    .get_result::<ReceivedGame>(&mut conn)
    .await.map_err(|err| db_error::DbError(err))? 
    .into();
    Ok((response))
}
