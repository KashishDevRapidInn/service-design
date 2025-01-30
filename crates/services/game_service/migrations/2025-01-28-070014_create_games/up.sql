-- Your SQL goes here
-- Your SQL goes here
CREATE TABLE rate_game (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    game_slug VARCHAR NOT NULL,
    user_id uuid NOT NULL,
    rating INT CHECK (rating >= 1 AND rating <= 5), 
    review TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    
    CONSTRAINT fk_game FOREIGN KEY (game_slug) REFERENCES games(slug) ON DELETE CASCADE,
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);