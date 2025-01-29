// @generated automatically by Diesel CLI.

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
    rate_game (id) {
        id -> Uuid,
        game_slug -> Varchar,
        user_id -> Uuid,
        rating -> Int4,
        review -> Nullable<Text>,
        created_at -> Timestamp,
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

diesel::joinable!(rate_game -> games (game_slug));
diesel::joinable!(rate_game -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    games,
    rate_game,
    users,
);
