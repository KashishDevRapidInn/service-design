-- Your SQL goes here
ALTER TABLE rate_game RENAME CONSTRAINT rate_game_rating_check TO rating_range;
