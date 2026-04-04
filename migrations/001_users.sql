CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email TEXT,
    passhash TEXT
);
INSERT INTO users (email, passhash) VALUES ('a@a.a', '');