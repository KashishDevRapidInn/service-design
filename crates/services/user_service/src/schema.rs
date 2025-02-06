// @generated automatically by Diesel CLI.

diesel::table! {
    email_verifications (user_id) {
        token -> Varchar,
        user_id -> Uuid,
        created_at -> Timestamp,
        expires_at -> Timestamp,
        #[max_length = 255]
        status -> Varchar,
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
