use chrono::NaiveDateTime;
use diesel::prelude::Insertable;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use flume::Receiver;
use kafka::models::{UserEventType, UserEventsMessage};
use lib_config::db::db::PgPool;
use rdkafka::message::OwnedMessage;
use futures::StreamExt;
use rdkafka::Message;
use serde::Deserialize;
use tracing::instrument;

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
                                UserEventType::Login { time } => todo!(),
                                UserEventType::Logout { time } => todo!(),
                                UserEventType::Rate { rating, game_slug, time } => todo!(),
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
