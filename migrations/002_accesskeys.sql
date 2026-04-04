CREATE TABLE accesskeys (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    keytext TEXT NOT NULL,
    expiry TIMESTAMPTZ,
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
INSERT INTO accesskeys (user_id, keytext, expiry) VALUES (1, 'placeholder', NULL);