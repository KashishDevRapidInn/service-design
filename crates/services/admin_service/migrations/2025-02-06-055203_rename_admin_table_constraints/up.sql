-- Your SQL goes here
ALTER TABLE admins RENAME CONSTRAINT admins_username_key TO unique_username;
ALTER TABLE admins RENAME CONSTRAINT admins_email_key TO unique_email;
