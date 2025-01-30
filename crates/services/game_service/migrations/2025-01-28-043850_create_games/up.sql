-- Your SQL goes here
CREATE TABLE games(
    slug VARCHAR(512) NOT NULL PRIMARY KEY,
    name VARCHAR(512) NOT NULL,
    title VARCHAR(255),
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    created_by_uid UUID,
    is_admin Boolean,
    genre VARCHAR(255)
);