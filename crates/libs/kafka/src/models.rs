use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::setup::KafkaTopic;

#[derive(Serialize, Deserialize)]
pub struct UserEventsMessage {
    pub user_id: uuid::Uuid,
    pub event_type: UserEventType
}

#[derive(Serialize, Deserialize)]
pub enum UserEventType{
    Register{
        username: String,
        email: String,
        created_at: Option<NaiveDateTime>
    },

    Login{
        time: NaiveDateTime
    },

    Logout{
        time: NaiveDateTime
    },

    Rate{
        rating: i32,
        game_slug: String,
        time: Option<NaiveDateTime>
    }
}

impl KafkaTopic for UserEventsMessage {
    fn topic_name(&self) -> String {
        return "user_events".to_string()
    }
}
