use chrono::NaiveDateTime;
use diesel::prelude::Insertable;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use errors::CustomError;
use flume::Receiver;
use kafka::models::{UserEventType, UserEventsMessage};
use lib_config::db::db::PgPool;
use rdkafka::message::OwnedMessage;
use futures::StreamExt;
use rdkafka::Message;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use diesel::prelude::*;
use uuid::Uuid;
use elasticsearch::Elasticsearch;
use crate::elasticsearch::ElasticsearchGame;
use crate::routes::game::games::get_game_by_slug;

#[derive(Deserialize, Insertable, Debug, Serialize, Clone, Queryable, AsChangeset, Selectable)]
#[diesel(table_name = crate::schema::games)]
pub struct ReceivedGame {
    pub slug: String,
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub created_by_uid: Option<Uuid>,
    pub is_admin: Option<bool>,
    pub genre: Option<String>,
}

#[derive(Deserialize, Debug, Serialize)]
pub enum KafkaGameMessage{
    Create(ReceivedGame),
    Update {
        slug: String,
        changes: UpdateGameBody,
    },
    Delete(String),
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct UpdateGameBody {
    pub title: Option<String>,
    pub description: Option<String>,
    pub genre: Option<String>,
}

#[derive(Deserialize, Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::users)]
struct ReceivedUser {
    pub id: uuid::Uuid,
    pub username: String,
    pub email: String,
    pub created_at: Option<NaiveDateTime>
}

pub async fn process_kafka_game_message(
    kafka_receiver: Receiver<OwnedMessage>,
    pool: PgPool,
    elastic_client: Elasticsearch,
) {
    kafka_receiver
        .stream()
        .for_each_concurrent(Some(10), |msg| {
            let pool = pool.clone();
            let value = elastic_client.clone();
            async move {
                if msg.topic() == "game_events" {
                    let payload = match msg.payload() {
                        Some(p) => p,
                        None => {
                            tracing::error!(
                                "No payload found in message. Topic: {}, Partition: {}, Offset: {}",
                                msg.topic(),
                                msg.partition(),
                                msg.offset()
                            );
                            return;
                        }
                    };

                    let game_msg = serde_json::from_slice::<KafkaGameMessage>(&payload);

                    match game_msg {
                        Ok(msg) => {
                            match msg {
                                KafkaGameMessage::Create(game) => {
                                    let mut conn = pool.get().await.unwrap();
                                    add_game_to_db(game.clone(), &mut conn).await;

                                    let es_game = ElasticsearchGame::new(&game);
                                    ElasticsearchGame::index_game(&value, &es_game).await.unwrap();
                                }
                                KafkaGameMessage::Update { slug, changes } => {
                                    let mut conn = pool.get().await.unwrap();
                                    update_game_in_db(slug.clone(), changes.clone(), &mut conn).await;
                                    let full_game = get_game_by_slug(&slug.clone(), &pool).await.unwrap(); 

                                    let es_game = ReceivedGame {
                                        slug: full_game.slug, 
                                        name: full_game.name, 
                                        title: changes.title.clone(), 
                                        description: changes.description.clone(), 
                                        genre: changes.genre.clone(),
                                        created_at: full_game.created_at,
                                        created_by_uid: full_game.created_by_uid,
                                        is_admin: full_game.is_admin,
                                    };
                                    let es_game = ElasticsearchGame::new(&es_game);
                                    ElasticsearchGame::update_game(&value, &es_game, None::<f32>, None::<i32>).await.unwrap();
                                }
                                KafkaGameMessage::Delete(slug) => {
                                    let mut conn = pool.get().await.unwrap();
                                    delete_game_from_db(slug.clone(), &mut conn).await;

                                    ElasticsearchGame::delete_game(&value, &slug).await.unwrap();
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to deserialize message to game
                                Topic: {}, Partition: {}, Offset: {} | Error: {:?}",
                                msg.topic(),
                                msg.partition(),
                                msg.offset(),
                                e
                            );
                        }
                    }
                } else if msg.topic() == "user_events" {
                    let payload = match msg.payload() {
                        Some(p) => p,
                        None => {
                            tracing::error!(
                                "No payload found in message. Topic: {}, Partition: {}, Offset: {}",
                                msg.topic(),
                                msg.partition(),
                                msg.offset()
                            );
                            return
                        }
                    };
                    let message_result = serde_json::from_slice::<UserEventsMessage>(payload);
                    
                    match message_result {
                        Ok(message) => {
                            match message.event_type {
                                UserEventType::Register { username, email, created_at } => {
                                    let user = ReceivedUser{
                                        id: message.user_id,
                                        username,
                                        email,
                                        created_at
                                    };
                                    let mut conn = pool.get().await.unwrap();
                                    add_user_to_db(user, &mut conn).await;
                                },

                                _ => {}
                            }
                        },

                        Err(e) => {
                            tracing::error!(
                                "Failed to deserialize message to user
                                Topic: {}, Partition: {}, Offset: {} | Error: {:?}",
                                msg.topic(),
                                msg.partition(),
                                msg.offset(),
                                e
                            );
                        }
                    }

                } else {
                    tracing::error!("Handler for topic {} not found", msg.topic());
                }
            }
        })
        .await;
}


#[instrument("Adding game to db", skip(conn))]
async fn add_game_to_db(game: ReceivedGame, conn: &mut AsyncPgConnection) {
    use crate::schema::games;

    let res = diesel::insert_into(games::table)
        .values(game.clone())
        .execute(conn)
        .await;

    match res {
        Ok(_) => tracing::info!("Added game: {:?} to db", game),
        Err(e) => tracing::error!("Failed to add game: {:?}", e),
    };
}

#[instrument("Updating game in db", skip(conn))]
async fn update_game_in_db(
    slug: String,
    changes: UpdateGameBody,
    conn: &mut AsyncPgConnection,
) {
    use crate::schema::games;

    let game: Option<ReceivedGame> = games::table
        .filter(games::slug.eq(slug.clone()))
        .first(conn)
        .await
        .ok();

    match game {
        Some(mut game) => {
            if let Some(new_title) = &changes.title {
                game.title = Some(new_title.clone());
            }
            if let Some(new_description) = &changes.description {
                game.description = Some(new_description.clone());
            }
            if let Some(new_genre) = &changes.genre {
                game.genre = Some(new_genre.clone());
            }

            let res = diesel::update(games::table.filter(games::slug.eq(slug)))
                .set(&game)
                .execute(conn)
                .await;

            match res {
                Ok(_) => tracing::info!("Updated game: {:?} in db", game),
                Err(e) => tracing::error!("Failed to update game: {:?}", e),
            }
        }
        None => tracing::error!("Game not found for update: {}", slug),
    }
}

#[instrument("Deleting game from db", skip(conn))]  
async fn delete_game_from_db(slug: String, conn: &mut AsyncPgConnection) {
    use crate::schema::games;

    let res = diesel::delete(games::table.filter(games::slug.eq(slug.clone())))
        .execute(conn)
        .await;

    match res {
        Ok(_) => tracing::info!("Deleted game with slug: {} from db", slug),
        Err(e) => tracing::error!("Failed to delete game: {:?}", e),
    }
}

#[instrument("Adding user to db", skip(conn))]
async fn add_user_to_db(user: ReceivedUser, conn: &mut AsyncPgConnection) {
    use crate::schema::users;

    let res = diesel::insert_into(users::table)
        .values(user.clone())
        .execute(conn)
        .await;

    match res {
        Ok(_) => tracing::info!("Added user: {:?} to db", user),
        Err(e) => tracing::error!("Failed to add user: {:?}", e),
    };
}

