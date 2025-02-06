-- This file should undo anything in `up.sql`
ALTER TABLE email_verifications DROP COLUMN status;
DROP TYPE verification_status;
