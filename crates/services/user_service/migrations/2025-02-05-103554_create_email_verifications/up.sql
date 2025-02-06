-- Your SQL goes here
CREATE TABLE email_verifications (
    token VARCHAR NOT NULL UNIQUE,
    user_id UUID NOT NULL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL
);

ALTER TABLE email_verifications
    ADD CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id);
