-- Your SQL goes here
ALTER TABLE users RENAME CONSTRAINT users_username_key TO unique_username;
ALTER TABLE users RENAME CONSTRAINT users_email_key TO unique_email;
