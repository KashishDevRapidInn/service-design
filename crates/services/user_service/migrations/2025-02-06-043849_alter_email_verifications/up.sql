-- Your SQL goes here
CREATE TYPE verification_status AS ENUM ('PENDING', 'VERIFIED', 'EXPIRED');

ALTER TABLE email_verifications
    ADD COLUMN status verification_status NOT NULL DEFAULT 'PENDING';