use chrono::NaiveDateTime;
use diesel::{Queryable, Selectable};
use kafka::models::{UserEventsMessage, UserEventType};
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
    pub modified_at: Option<NaiveDateTime>,
}

impl From<User> for UserEventsMessage {
     fn from(value: User) -> Self {
        Self {
            user_id: value.id,
            event_type: UserEventType::Register {
                username: value.username,
                email: value.email,
                created_at: value.created_at
            }
        }
    }
}
