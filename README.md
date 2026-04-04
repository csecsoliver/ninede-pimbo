# ninede-pimbo (9-e.cc personal inventory management by olio)

## endpoints
Routes starting with api are the json endpoints, ones not are html or plain text endpoints
1. /api/item (post)  
An endpoint to 
2. /api/item/{id} (get)  
3. /api/items (get)  
4. /#{id} (get)  
5. /login (get)  
6. /login (post)  
6. /dash (get)  
8. /search (get)  
7. / (get)  

## DB tables
### users (placeholder table)
1. id (SERIAL, PRIMARY KEY)
2. email (TEXT)
3. passhash (TEXT)
### accesskeys (this is the current way to authenticate)
1. id (SERIAL, PRIMARY KEY)
2. user_id (INTEGER NOT NULL)
2. key (TEXT NOT NULL)
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