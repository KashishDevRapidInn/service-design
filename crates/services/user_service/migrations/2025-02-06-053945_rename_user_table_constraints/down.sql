-- This file should undo anything in `up.sql`
ALTER TABLE users RENAME CONSTRAINT unique_username TO users_username_key;
ALTER TABLE users RENAME CONSTRAINT unique_email TO users_email_key;
