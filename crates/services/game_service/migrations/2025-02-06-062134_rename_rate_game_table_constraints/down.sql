-- This file should undo anything in `up.sql`
ALTER TABLE rate_game RENAME CONSTRAINT rating_range TO rate_game_rating_check ;
