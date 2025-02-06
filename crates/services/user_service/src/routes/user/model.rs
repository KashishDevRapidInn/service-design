use chrono::{naive, NaiveDateTime};
use diesel::{Queryable, Selectable};
use kafka::setup::KafkaTopic;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use helpers::validations::validations::UpdateUserBody;
use diesel_derive_enum::DbEnum;
use diesel::prelude::*;

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

#[derive(Serialize)]
pub struct UserMessage{
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: Option<NaiveDateTime>,
    pub modified_at: Option<NaiveDateTime>,
}

impl From<User> for UserMessage {
     fn from(value: User) -> Self {
        Self {
            id: value.id,
            username: value.username,
            email: value.email,
            created_at: value.created_at,
            modified_at: value.modified_at
        }
    }
}
#[derive(Serialize)]
pub enum KafkaUserMessage{
    Create (UserMessage),
    Update {
        id: uuid::Uuid,
        changes: UpdateUserBody
    }
}

impl<'a> KafkaTopic for KafkaUserMessage{
    fn topic_name(&self) -> String {
        "user_events".into()
    }
}

// #[derive(Debug, DbEnum, serde::Serialize, serde::Deserialize)]
// #[ExistingTypePath = "crate::schema::sql_types::VerificationStatus"]
// pub enum EmailVerificationStatus {
//     Pending,
//     Verified,
//     Expired,
// }
#[derive(Queryable, Deserialize, Serialize, Debug, Selectable)]
#[diesel(table_name = crate::schema::email_verifications)]
pub struct EmailVerification {
    pub token: String,
    pub user_id: uuid::Uuid,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub status: String,
}
