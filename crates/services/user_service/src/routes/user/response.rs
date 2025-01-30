use chrono::NaiveDateTime;
use diesel::{Queryable, Selectable};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Queryable, Deserialize, Serialize, Debug, Selectable)]
#[diesel(table_name = crate::schema::users)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: Option<NaiveDateTime>,
}
