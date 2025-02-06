-- This file should undo anything in `up.sql`
ALTER TABLE email_verifications  
    ALTER COLUMN status DROP NOT NULL,  
    ALTER COLUMN status SET DATA TYPE VARCHAR;
