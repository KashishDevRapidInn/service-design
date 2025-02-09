// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "user_event_type"))]
    pub struct UserEventType;
}

diesel::table! {
    admins (id) {
        id -> Uuid,
        username -> Varchar,
        password_hash -> Varchar,
        email -> Varchar,
    }
}

diesel::table! {
    games (slug) {
        #[max_length = 512]
        slug -> Varchar,
        #[max_length = 512]
        name -> Varchar,
        #[max_length = 255]
        title -> Nullable<Varchar>,
        description -> Nullable<Text>,
        created_at -> Nullable<Timestamp>,
        created_by_uid -> Nullable<Uuid>,
        is_admin -> Nullable<Bool>,
        #[max_length = 255]
        genre -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::UserEventType;

    user_events (id) {
        id -> Uuid,
        event_type -> UserEventType,
        data -> Jsonb,
        user_id -> Uuid,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Varchar,
        email -> Varchar,
        created_at -> Nullable<Timestamp>,
        modified_at -> Nullable<Timestamp>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    admins,
    games,
    user_events,
    users,
);
