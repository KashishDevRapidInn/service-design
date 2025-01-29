use elasticsearch::{Elasticsearch, IndexParts, SearchParts, UpdateParts, DeleteParts};
use elasticsearch::http::transport::Transport;
use serde_json::json;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use crate::kafka_handler::ReceivedGame;

#[derive(Serialize, Deserialize, Debug)]
pub struct ElasticsearchGame {
    pub slug: String,
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub genre: Option<String>,
    // pub rating: Option<Vec<i32>>,
    pub average_rating: Option<f32>,
    pub rating_count: Option<i32>
}

impl ElasticsearchGame {
    pub fn new(game: &ReceivedGame) -> Self {
        ElasticsearchGame {
            slug: game.slug.clone(),
            name: game.name.clone(),
            title: game.title.clone(),
            description: game.description.clone(),
            genre: game.genre.clone(),
            // rating:None,
            average_rating: None,
            rating_count: None
        }
    }

    pub async fn index_game(elastic_client: &Elasticsearch, game: &ElasticsearchGame) -> Result<(), Box<dyn std::error::Error>> {
        let response = elastic_client
            .index(IndexParts::IndexId("rate", &game.slug))
            .body(json!({
                "slug": game.slug,
                "name": game.name,
                "title": game.title,
                "description": game.description,
                "genre": game.genre,
                "average_rating": None::<f32>,
                "rating_count": None::<i32>
            }))
            .send()
            .await?;

        if response.status_code().is_success() {
            tracing::info!("Indexed game: {} into Elasticsearch", game.slug);
        } else {
            tracing::error!("Failed to index game into Elasticsearch: {}", game.slug);
        }

        Ok(())
    }
    pub async fn update_game(elastic_client: &Elasticsearch, game: &ElasticsearchGame, avg_rating: Option<f32>, count: Option<i32>) -> Result<(), Box<dyn std::error::Error>> {
        let response = elastic_client
            .update(UpdateParts::IndexId("rate", &game.slug))
            .body(json!({
                "doc": {
                    "name": game.name,
                    "title": game.title,
                    "description": game.description,
                    "genre": game.genre,
                    "average_rating": avg_rating,
                    "rating_count":count
                }
            }))
            .send()
            .await?;
    
        if response.status_code().is_success() {
            tracing::info!("Updated game: {} in Elasticsearch", game.slug);
        } else {
            tracing::error!("Failed to update game in Elasticsearch: {}", game.slug);
        }
    
        Ok(())
    }
    pub async fn delete_game(elastic_client: &Elasticsearch, slug: &str) -> Result<(), Box<dyn std::error::Error>> {
        let response = elastic_client
            .delete(DeleteParts::IndexId("rate", slug))
            .send()
            .await?;

        if response.status_code().is_success() {
            tracing::info!("Deleted game: {} from Elasticsearch", slug);
        } else {
            tracing::error!("Failed to delete game from Elasticsearch: {}", slug);
        }

        Ok(())
    }
}
