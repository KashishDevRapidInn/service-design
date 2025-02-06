-- This file should undo anything in `up.sql`

ALTER TABLE email_verifications
DROP COLUMN status;

ALTER TABLE email_verifications
ADD COLUMN status verification_status NOT NULL DEFAULT 'PENDING';
