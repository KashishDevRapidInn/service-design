use chrono::NaiveDateTime;
use diesel::prelude::{Insertable};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use flume::Receiver;
use lib_config::db::db::PgPool;
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use diesel::prelude::*;
#[derive(Deserialize, Insertable, Debug, Serialize, Clone)]
#[diesel(table_name = crate::schema::users)]
pub struct UserMessage {
    pub id: uuid::Uuid,
    pub username: String,
    pub email: String,
    pub created_at: Option<NaiveDateTime>,
    pub modified_at: Option<NaiveDateTime>,
}

#[derive(Serialize, Deserialize)]
pub enum KafkaUserMessage{
    Create(UserMessage),
    Update {
        id: uuid::Uuid,
        changes: UpdateUserBody,
    },
}
#[derive(Deserialize, Serialize, Clone)]
pub struct UpdateUserBody {
    pub username: String,
    pub email: String,
}

pub async fn process_kafka_message(
    kafka_receiver: Receiver<OwnedMessage>,
    pool: PgPool,
) {
    kafka_receiver.stream().for_each_concurrent(Some(10), |msg| {
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
                        return;
                    }
                };

                let user_message_result: Result<KafkaUserMessage, _> =
                    serde_json::from_slice(payload);

                match user_message_result {
                    Ok(KafkaUserMessage::Create(user)) => {
                        let mut conn = pool.get().await.unwrap();
                        add_user_to_db(user, &mut conn).await;
                    }
                    Ok(KafkaUserMessage::Update { id, changes }) => {
                        let mut conn = pool.get().await.unwrap();
                        update_user_in_db(id, changes, &mut conn).await;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to deserialize message to KafkaUserMessage | Error: {:?} \
                             Topic: {}, Partition: {}, Offset: {}",
                            e,
                            msg.topic(),
                            msg.partition(),
                            msg.offset()
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
async fn add_user_to_db(user: UserMessage, conn: &mut AsyncPgConnection) {
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

#[instrument("Updating user in db", skip(conn, changes))]
async fn update_user_in_db(
    user_id: uuid::Uuid,
    changes: UpdateUserBody,
    conn: &mut AsyncPgConnection,
) {
    use crate::schema::users::{self, dsl::*};

    let res = diesel::update(users.filter(id.eq(user_id)))
        .set((
            username.eq(&changes.username),
            email.eq(&changes.email),
            modified_at.eq(chrono::Utc::now().naive_utc()),
        ))
        .execute(conn)
        .await;

    match res {
        Ok(rows_updated) => {
            if rows_updated > 0 {
                tracing::info!("User with ID {} updated successfully", user_id);
            } else {
                tracing::warn!("No user found with ID {} to update", user_id);
            }
        }
        Err(e) => tracing::error!("Failed to update user: {:?}", e),
    };
}
