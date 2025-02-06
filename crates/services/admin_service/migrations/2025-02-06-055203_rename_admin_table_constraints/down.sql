-- This file should undo anything in `up.sql`
ALTER TABLE admins RENAME CONSTRAINT unique_username TO admins_username_key;
ALTER TABLE admins RENAME CONSTRAINT unique_email TO admins_email_key;
