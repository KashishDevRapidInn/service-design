// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "status_enum"))]
    pub struct StatusEnum;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::StatusEnum;

    email_verifications (user_id) {
        token -> Varchar,
        user_id -> Uuid,
        created_at -> Timestamp,
        expires_at -> Timestamp,
        status -> StatusEnum,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Varchar,
        password_hash -> Varchar,
        email -> Varchar,
        created_at -> Nullable<Timestamp>,
        modified_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(email_verifications -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    email_verifications,
    users,
);
