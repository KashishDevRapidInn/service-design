// @generated automatically by Diesel CLI.

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
    users (id) {
        id -> Uuid,
        username -> Varchar,
        email -> Varchar,
        created_at -> Nullable<Timestamp>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    admins,
    games,
    users,
);
