-- Your SQL goes here
CREATE TABLE users (
    id uuid PRIMARY KEY NOT NULL,
    username VARCHAR UNIQUE NOT NULL,
    email VARCHAR UNIQUE NOT NULL,
    created_at TIMESTAMP
);
