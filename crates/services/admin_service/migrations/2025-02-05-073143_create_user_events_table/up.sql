-- Your SQL goes here
CREATE TYPE user_event_type AS ENUM ('Register', 'Login', 'Logout', 'Rate', 'Update');

CREATE TABLE user_events (
    id uuid PRIMARY KEY,
    event_type user_event_type NOT NULL,
    data jsonb NOT NULL
);
