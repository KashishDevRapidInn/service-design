use chrono::NaiveDateTime;
use diesel::{Queryable, Selectable};
use kafka::setup::KafkaTopic;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Queryable, Deserialize, Serialize, Debug)]
pub struct Admin {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub email: String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Queryable, Deserialize, Serialize, Debug, Selectable)]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: Option<NaiveDateTime>
}

#[derive(Deserialize)]
pub struct Paginate {
    pub page: i64,
    pub limit: i64
}

#[derive(Serialize, Debug)]
pub struct DeleteUserMessage {
    pub id: uuid::Uuid
}

impl KafkaTopic for DeleteUserMessage {
    fn topic_name(&self) -> String {
        "admin_events".into()
    }
}

