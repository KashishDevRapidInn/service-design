-- This file should undo anything in `up.sql`
ALTER TABLE rate_game  
ALTER COLUMN rating DROP NOT NULL;
