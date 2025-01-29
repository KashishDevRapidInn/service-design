use elasticsearch::{Elasticsearch, IndexParts, SearchParts};
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
}

impl ElasticsearchGame {
    pub fn new(game: &ReceivedGame) -> Self {
        ElasticsearchGame {
            slug: game.slug.clone(),
            name: game.name.clone(),
            title: game.title.clone(),
            description: game.description.clone(),
            genre: game.genre.clone(),
        }
    }

    pub async fn index_game(elastic_client: &Elasticsearch, game: &ElasticsearchGame) -> Result<(), Box<dyn std::error::Error>> {
        let response = elastic_client
            .index(IndexParts::Index("games"))
            .body(json!({
                "slug": game.slug,
                "name": game.name,
                "title": game.title,
                "description": game.description,
                "genre": game.genre,
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

    // pub async fn delete_game(elastic_client: &Elasticsearch, slug: &str) -> Result<(), Box<dyn std::error::Error>> {
    //     let response = elastic_client
    //         .delete(IndexParts::IndexId("games", slug))
    //         .send()
    //         .await?;

    //     if response.status_code().is_success() {
    //         tracing::info!("Deleted game: {} from Elasticsearch", slug);
    //     } else {
    //         tracing::error!("Failed to delete game from Elasticsearch: {}", slug);
    //     }

    //     Ok(())
    // }
}
