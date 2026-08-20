# ninede-pimbo (9-e.cc personal inventory management by olio)
## Partially complete docs at [https://q238dk0jb7.apidog.io](https://q238dk0jb7.apidog.io)
## Prod deployment at [https://i.9-e.cc](https://i.9-e.cc)

Keep track of where your things live, and of which ones you are currently
looking for. Put a label with an item's public address on the item itself, and
whoever finds it can tell you they saw it.

The web interface is html and nothing else: no javascript, no stylesheet. Every
page works with plain links and forms, so it renders in anything that speaks
http.

## Running it

`docker compose up -d --build`, or `podman-compose up -d --build`. The app
creates its own tables on startup.

Copy `.env.example` to `.env` first and set `BOOTSTRAP_KEY` to the access key
you want to log in with; the app registers it for user 1 on startup.

Without compose, the app needs `DATABASE_URL` and nothing else:

```
DATABASE_URL=postgres://user:pass@host/db cargo run
```

| variable | default | meaning |
| --- | --- | --- |
| `DATABASE_URL` | required | postgres connection string |
| `BIND` | `0.0.0.0:3000` | address to listen on |
| `COOKIE_SECURE` | `true` | mark the session cookie `Secure`; set `false` to log in over plain http |
| `BOOTSTRAP_KEY` | unset | if set, becomes an access key for user 1 |
| `SIGNUP_OPEN` | `true` | whether strangers can create accounts at `/signup` |

The sqlx macros are checked against a real database at compile time. Building
without one works from the committed `.sqlx/` data (`SQLX_OFFLINE=true`, which
the Dockerfile sets); after changing a query, regenerate it with
`cargo sqlx prepare` against a live database.

## Authentication
Sign up at `/signup` with an email and a password, which is stored as an argon2id
hash in `users.passhash`. Emails are unique, case insensitively, and are held
lowercased.

Whichever way you log in, a row in `accesskeys` is what actually authenticates a
request, presented either as the `x-api-key` header (api clients) or in the
`pimbo_key` cookie (browsers). Expired keys (`expiry` in the past) are rejected;
`expiry = NULL` never expires.

- Logging in with an email mints a fresh random key, marked `session = true`,
  good for 30 days. `/logout` deletes it, so the cookie is dead even if it was
  copied somewhere.
- Logging in with an access key puts that key in the cookie as it is. `/logout`
  leaves keys you created by hand alone, so signing out of a browser never
  breaks an api client.

To make a key for an api client, insert one:

```sql
INSERT INTO accesskeys (user_id, keytext, expiry) VALUES (1, 'a long random string', NULL);
```

The migration seeds the key `placeholder` for user 1. Delete it on any
deployment you care about:

```sql
DELETE FROM accesskeys WHERE keytext = 'placeholder';
```

Set `SIGNUP_OPEN=false` to stop anybody else making an account on your
deployment. Existing accounts keep working.

## endpoints
Routes starting with api are the json endpoints, ones not are html pages and
form targets. Browsers only send GET and POST, so the html side does its writes
with POST and answers with a redirect.

### json api (auth)
1. `/api/items` (post) — create an item, returns its id
2. `/api/items` (get) — list your items
3. `/api/items/{id}` (get) — one item
4. `/api/items/{id}` (patch) — modify an item, fields left out are unchanged
5. `/api/items/{id}` (delete) — delete an item
6. `/api/search?q={query}` (get) — search name, tags, description and location
7. `/api/health` (get) — no auth, used by the container healthcheck

### html
1. `/` (get) — landing page, or your dashboard when logged in
2. `/signup` (get, post) — create an account with an email and a password
3. `/login` (get, post) — email and password, or an access key
4. `/logout` (post) — ends the session
5. `/dash` (get) (auth) — everything you own
6. `/search?q={query}` (get) (auth)
7. `/items/new` (get) (auth) — the add-an-item form
8. `/items` (post) (auth) — create
9. `/items/{id}` (get, post) (auth) — view, and save edits
10. `/items/{id}/seen` (post) (auth) — record that you saw it just now
11. `/items/{id}/searching` (post) (auth) — start or stop looking for it
12. `/items/{id}/delete` (post) (auth)
13. `/_{id}` (get) — the public page for an item, for whoever finds it
14. `/_{id}/seen` (post) — a finder reporting the item is still around

Items are scoped to the account that owns them: an item that is not yours is a
404, whichever route you ask through.

## DB tables
### users (accounts)
1. id (SERIAL, PRIMARY KEY)
2. email (TEXT)
3. passhash (TEXT, argon2id)
UNIQUE INDEX users_email_lower_key ON (lower(email))
### accesskeys (what actually authenticates a request)
1. id (SERIAL, PRIMARY KEY)
2. user_id (INTEGER NOT NULL)
2. keytext (TEXT NOT NULL)
3. expiry (TIMESTAMPZ)
4. session (BOOL NOT NULL DEFAULT false) — minted by a login, revoked by logout
CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
### items (actual stuff)
1. id (SERIAL, PRIMARY KEY)
2. user_id (INTEGER)
2. name (TEXT)
3. tags (TEXT)
4. desc (TEXT)
5. loc (TEXT)
6. last_seen (TIMESTAMPZ)
7. searching (BOOL)
CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL

## Source layout
| file | what is in it |
| --- | --- |
| `src/main.rs` | startup, configuration, the route table |
| `src/db.rs` | every sql statement |
| `src/auth.rs` | access-key extractors and the session cookie |
| `src/html.rs` | page shell, escaping, shared markup |
| `src/web.rs` | the html pages and form handlers |
| `src/api.rs` | the json handlers |
| `src/models.rs` | the shared structs |
