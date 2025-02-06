-- This file should undo anything in `up.sql`
ALTER TABLE email_verifications
    DROP COLUMN status;

DROP TYPE status_enum;

ALTER TABLE email_verifications
    ADD COLUMN status VARCHAR(255) NOT NULL DEFAULT 'pending';
