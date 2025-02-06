ALTER TABLE email_verifications
    DROP COLUMN status;

CREATE TYPE status_enum AS ENUM ('pending', 'verified');

ALTER TABLE email_verifications
    ADD COLUMN status status_enum DEFAULT 'pending';
