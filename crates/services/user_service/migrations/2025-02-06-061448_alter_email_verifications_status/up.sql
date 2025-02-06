-- Your SQL goes here
ALTER TABLE email_verifications
    ALTER COLUMN status SET DATA TYPE VARCHAR(255),
    ALTER COLUMN status SET NOT NULL;
