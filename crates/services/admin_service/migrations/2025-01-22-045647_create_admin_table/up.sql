-- Your SQL goes here
CREATE TABLE admins (
    id uuid PRIMARY KEY NOT NULL,
    username VARCHAR UNIQUE NOT NULL,
    password_hash VARCHAR NOT NULL
);
