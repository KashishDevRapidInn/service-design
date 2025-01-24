use chrono::NaiveDateTime;
use diesel::{Queryable, Selectable};
use kafka::setup::KafkaTopic;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Queryable, Deserialize, Serialize, Debug, Selectable)]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub email: String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Serialize)]
pub struct RegisterUserMessage{
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: Option<NaiveDateTime>
}

impl KafkaTopic for RegisterUserMessage {
    fn topic_name(&self) -> String {
        return "user_events".to_string()
    }
}

impl From<User> for RegisterUserMessage {
     fn from(value: User) -> Self {
        Self {
            id: value.id,
            username: value.username,
            email: value.email,
            created_at: value.created_at
        }
    }
}
