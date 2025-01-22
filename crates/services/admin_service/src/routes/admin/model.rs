use chrono::NaiveDateTime;
use diesel::Queryable;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::schema::admins;

#[derive(Queryable, Deserialize, Serialize, Debug)]
#[diesel(table_name = admins)]
pub struct Admin {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub email: String,
    pub created_at: Option<NaiveDateTime>,
}