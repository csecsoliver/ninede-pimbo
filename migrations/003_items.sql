CREATE TABLE items (
    id SERIAL PRIMARY KEY,
    user_id INTEGER,
    name TEXT NOT NULL,
    tags TEXT NOT NULL,
    description TEXT NOT NULL,
    location TEXT NOT NULL,
    last_seen TIMESTAMPTZ NOT NULL,
    searching BOOL NOT NULL,
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
)