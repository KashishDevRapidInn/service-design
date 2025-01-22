// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Varchar,
        password_hash -> Varchar,
        email -> Varchar,
        created_at -> Nullable<Timestamp>,
    }
}
