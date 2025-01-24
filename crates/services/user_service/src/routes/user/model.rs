use chrono::NaiveDateTime;
use diesel::Queryable;
use kafka::setup::KafkaTopic;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::schema::users;

#[derive(Queryable, Deserialize, Serialize, Debug)]
#[diesel(table_name = users)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub email: String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Serialize)]
pub struct RegisterUserMessage{
    pub email: String,
    pub name: String
}

impl KafkaTopic for RegisterUserMessage {
    fn topic_name(&self) -> String {
        return "user_events".to_string()
    }
}
