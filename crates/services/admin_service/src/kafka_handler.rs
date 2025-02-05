use chrono::NaiveDateTime;
use diesel::{prelude::Insertable, ExpressionMethods};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use flume::Receiver;
use kafka::models::{UserEventType, UserEventsMessage};
use lib_config::db::db::PgPool;
use rdkafka::message::OwnedMessage;
use futures::StreamExt;
use rdkafka::Message;
use serde::Deserialize;
use serde_json::json;
use tracing::instrument;
use uuid::Uuid;

#[derive(Deserialize, Insertable, Debug)]
#[diesel(table_name = crate::schema::users)]
pub struct ReceivedUser {
    pub id: uuid::Uuid,
    pub username: String,
    pub email: String,
    pub created_at: Option<NaiveDateTime>
}

pub async fn process_kafka_message(
    kafka_receiver: Receiver<OwnedMessage>,
    pool: PgPool 
) {
    kafka_receiver.stream()
        .for_each_concurrent(Some(10), |msg| {
            let pool = pool.clone();
            async move {
                if msg.topic() == "user_events" {
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
                                UserEventType::Login { time } => {
                                    use crate::schema::user_events;

                                    let mut conn = pool.get().await.unwrap();
                                    let res = diesel::insert_into(user_events::table)
                                        .values((
                                            user_events::id.eq(Uuid::new_v4()),
                                            user_events::user_id.eq(message.user_id),
                                            user_events::event_type.eq(DbUserEventType::Login),
                                            user_events::data.eq(json!({
                                                "time": time
                                            }))
                                        ))
                                        .execute(&mut conn)
                                        .await;

                                    if let Err(e) = res {
                                        tracing::error!("Failed to insert user login to user_events table: {:?}", e);
                                    } else {
                                        tracing::info!("Inserted user login to user_events table")
                                    }
                                    
                                },
                                UserEventType::Logout { time } => {
                                    use crate::schema::user_events;

                                    let mut conn = pool.get().await.unwrap();
                                    let res = diesel::insert_into(user_events::table)
                                        .values((
                                            user_events::id.eq(Uuid::new_v4()),
                                            user_events::user_id.eq(message.user_id),
                                            user_events::event_type.eq(DbUserEventType::Logout),
                                            user_events::data.eq(json!({
                                                "time": time
                                            }))
                                        ))
                                        .execute(&mut conn)
                                        .await;

                                    if let Err(e) = res {
                                        tracing::error!("Failed to insert user logout to user_events table: {:?}", e);
                                    } else {
                                        tracing::info!("Inserted user logout to user_events table")
                                    }
                                },
                                UserEventType::Rate { rating, game_slug, time } => {
                                    use crate::schema::user_events;

                                    let mut conn = pool.get().await.unwrap();
                                    let res = diesel::insert_into(user_events::table)
                                        .values((
                                            user_events::id.eq(Uuid::new_v4()),
                                            user_events::user_id.eq(message.user_id),
                                            user_events::event_type.eq(DbUserEventType::Rate),
                                            user_events::data.eq(json!({
                                                "rating": rating,
                                                "game_slug": game_slug,
                                                "time": time
                                            }))
                                        ))
                                        .execute(&mut conn)
                                        .await;

                                    if let Err(e) = res {
                                        tracing::error!("Failed to insert user rating to user_events table: {:?}", e);
                                    } else {
                                        tracing::info!("Inserted user rating to user_events table")
                                    }
                                },
                                UserEventType::Register { username, email, created_at } => {
                                    let user = ReceivedUser{
                                        id: message.user_id,
                                        username,
                                        email,
                                        created_at
                                    };
                                    let mut conn = pool.get().await.unwrap();
                                    add_user_to_db(user, &mut conn).await;
                                }
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

#[instrument("Adding user to db", skip(conn))]
async fn add_user_to_db(user: ReceivedUser, conn: &mut AsyncPgConnection) {
    use crate::schema::users;

    let res = diesel::insert_into(users::table)
        .values(&user)
        .execute(conn)
        .await;

    match res {
        Ok(_) => tracing::info!("Added user: {:?} to db", user),
        Err(e) => tracing::error!(
            "Failed to add user: {:?}",
            e
        )
    };
}

#[derive(diesel_derive_enum::DbEnum, Debug)]
#[ExistingTypePath = "crate::schema::sql_types::UserEventType"]
#[DbValueStyle = "verbatim"]
enum DbUserEventType{
    Register,
    Login,
    Logout,
    Rate,
    Update
} 
