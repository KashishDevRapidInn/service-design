use serde::{Serialize, Deserialize};
use uuid::Uuid;
use diesel::{Queryable, Insertable};
use crate::schema::games;
use diesel::prelude::*;
use chrono;

use kafka::setup::KafkaTopic;

#[derive(Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = games)]
pub struct Game {
    pub slug: String,
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub created_by_uid: Option<Uuid>,
    pub is_admin: Option<bool>,
    pub genre: Option<String>,
}

#[derive(Serialize, Deserialize, Insertable, Debug, Queryable)]
#[diesel(table_name = games)]
pub struct CreateGameBody {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub genre: Option<String>,
    pub created_by_uid: Option<Uuid>,
    pub is_admin: Option<bool>,
}
#[derive(Serialize, Deserialize, Insertable, Debug, Queryable)]
#[diesel(table_name = games)]
pub struct UpdateGameBody {
    pub title: Option<String>,
    pub description: Option<String>,
    pub genre: Option<String>,
}

#[derive(Serialize)]
pub enum KafkaGameMessage<'a> {
    Create (&'a Game),
    Update {
        slug: &'a String,
        changes: &'a UpdateGameBody
    },
    Delete (&'a String)
}

impl<'a> KafkaTopic for KafkaGameMessage<'a> {
    fn topic_name(&self) -> String {
        "game_events".into()
    }
}
