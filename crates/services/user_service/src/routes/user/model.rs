
use chrono::{naive, NaiveDateTime};
use diesel::{Queryable, Selectable};
use kafka::models::{UserEventsMessage, UserEventType};
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
#[derive(Debug, DbEnum, serde::Serialize, serde::Deserialize, PartialEq)]
#[ExistingTypePath = "crate::schema::sql_types::StatusEnum"]
pub enum StatusEnum {
    Pending,
    Verified,
    Expired,
}
#[derive(Queryable, Deserialize, Serialize, Debug, Selectable)]
#[diesel(table_name = crate::schema::email_verifications)]
pub struct EmailVerification {
    pub token: String,
    pub user_id: uuid::Uuid,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub status: StatusEnum,
}

#[derive(Deserialize)]
pub struct MailQuery{
    pub token: String
}