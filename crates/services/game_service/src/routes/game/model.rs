use diesel::prelude::*;
use diesel::Queryable;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::NaiveDateTime;

#[derive(Queryable, Deserialize, Serialize, Debug)]
pub struct Game {
    pub slug: String,
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub created_by_uid: Option<Uuid>,
    pub is_admin: Option<bool>,
    pub genre: Option<String>,
}

#[derive(Queryable, Deserialize, Serialize, Debug, Insertable)]

#[diesel(table_name = crate::schema::rate_game)]
pub struct RateGame {
    pub id: Uuid,
    pub game_slug: String,
    pub user_id: Uuid,
    pub rating: i32,
    pub review: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Queryable, Deserialize, Serialize, Debug)]

#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Deserialize, Queryable)]

#[diesel(table_name = crate::schema::rate_game)]
pub struct RateGameRequest {
    pub game_slug: String,
    pub rating: i32,
    pub review: Option<String>,
}