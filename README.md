# ninede-pimbo (9-e.cc personal inventory management by olio)
## Partially complete docs at [https://q238dk0jb7.apidog.io](https://q238dk0jb7.apidog.io)
## Prod deployment at [https://i.9-e.cc](https://i.9-e.cc)
## endpoints
Routes starting with api are the json endpoints, ones not are html, plain text or form endpoints
1. /api/items (post) (auth)  
Create an item
2. /api/items/{id} (get) (auth)
Get info about an item
3. /api/items/{id} (patch) (auth)
modify an item
4. /api/items/{id} (delete) (auth)
3. /api/items (get) (auth)  
4. /#{id} (get)  
5. /#{id}/seen (post)
5. /login (get) (not implemented)  
6. /login (post) (not implemented)  
6. /dash (get) (not implemented)  
8. /search?q={query} (get) (auth)  
7. / (get)  

## DB tables
### users (placeholder table)
1. id (SERIAL, PRIMARY KEY)
2. email (TEXT)
3. passhash (TEXT)
### accesskeys (this is the current way to authenticate)
1. id (SERIAL, PRIMARY KEY)
2. user_id (INTEGER NOT NULL)
2. keytext (TEXT NOT NULL)
3. expiry (TIMESTAMPZ)
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
