use diesel::ExpressionMethods;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use flume::Receiver;
use futures::StreamExt;
use lib_config::db::db::PgPool;
use rdkafka::{message::OwnedMessage, Message};
use serde::Deserialize;
use tracing::instrument;

#[derive(Deserialize, Debug)]
struct DeleteUser {
    id: uuid::Uuid
}

pub async fn process_kafka_message(
    rx: Receiver<OwnedMessage>,
    pool: PgPool
){
    rx.stream()
        .for_each_concurrent(10, |msg| {
            let pool_clone = pool.clone();
            async move {
                if msg.topic() == "admin_events" {
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

                    let user = serde_json::from_slice::<DeleteUser>(payload);

                    match user {
                        Ok(user) => {
                            let mut conn = pool_clone.get().await.unwrap();
                            delete_user_from_db(user, &mut conn).await
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
                }
            }
        }).await;
}

#[instrument("Deleting user from db", skip(conn))]
async fn delete_user_from_db(user: DeleteUser, conn: &mut AsyncPgConnection) {
    use crate::schema::users;

    let res = diesel::delete(users::table)
        .filter(users::id.eq(&user.id))
        .execute(conn)
        .await;

    match res {
        Ok(_) => tracing::info!("Deleted user: {:?} from db", user),
        Err(e) => tracing::error!(
            "Failed to delete user: {:?}",
            e
        )
    };
}
