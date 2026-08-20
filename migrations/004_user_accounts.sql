-- One account per email address, case insensitively. Rows with no email (the
-- placeholder user) are unaffected.
CREATE UNIQUE INDEX IF NOT EXISTS users_email_lower_key ON users (lower(email));

-- Keys minted when somebody logs in with their email, as opposed to keys you
-- created by hand for an api client. Logging out only revokes the former.
ALTER TABLE accesskeys ADD COLUMN IF NOT EXISTS session BOOLEAN NOT NULL DEFAULT false;
