//! Every sql statement the app runs lives here, so the http layers (json api
//! and html pages) can share one set of queries.

use crate::models::{ItemInfo, ModifyItemRequest, User};
use sqlx::PgPool;

/// Resolve an access key to the user it belongs to, ignoring expired keys.
pub async fn user_for_key(db: &PgPool, keytext: &str) -> Result<Option<i32>, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_id FROM accesskeys
        WHERE keytext = $1 AND (expiry IS NULL OR expiry > now())
        "#,
        keytext
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| r.user_id))
}

pub async fn user(db: &PgPool, id: i32) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT id, email, passhash FROM users WHERE id = $1",
        id
    )
    .fetch_optional(db)
    .await
}

pub async fn list_items(db: &PgPool, user_id: i32) -> Result<Vec<ItemInfo>, sqlx::Error> {
    sqlx::query_as!(
        ItemInfo,
        r#"
        SELECT id, user_id, name, tags, description, location, last_seen, searching
        FROM items WHERE user_id = $1 ORDER BY searching DESC, name ASC
        "#,
        user_id
    )
    .fetch_all(db)
    .await
}

pub async fn search_items(
    db: &PgPool,
    user_id: i32,
    query: &str,
) -> Result<Vec<ItemInfo>, sqlx::Error> {
    // The wildcards are ours; anything the user typed is matched literally.
    let pattern = format!(
        "%{}%",
        query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    sqlx::query_as!(
        ItemInfo,
        r#"
        SELECT id, user_id, name, tags, description, location, last_seen, searching
        FROM items
        WHERE user_id = $1
          AND (name ILIKE $2 OR tags ILIKE $2 OR description ILIKE $2 OR location ILIKE $2)
        ORDER BY searching DESC, name ASC
        "#,
        user_id,
        pattern
    )
    .fetch_all(db)
    .await
}

/// An item, but only if it belongs to `user_id`.
pub async fn item_of(db: &PgPool, user_id: i32, id: i32) -> Result<Option<ItemInfo>, sqlx::Error> {
    sqlx::query_as!(
        ItemInfo,
        r#"
        SELECT id, user_id, name, tags, description, location, last_seen, searching
        FROM items WHERE user_id = $1 AND id = $2
        "#,
        user_id,
        id
    )
    .fetch_optional(db)
    .await
}

/// An item regardless of owner, for the public "someone scanned it" page.
pub async fn item(db: &PgPool, id: i32) -> Result<Option<ItemInfo>, sqlx::Error> {
    sqlx::query_as!(
        ItemInfo,
        r#"
        SELECT id, user_id, name, tags, description, location, last_seen, searching
        FROM items WHERE id = $1
        "#,
        id
    )
    .fetch_optional(db)
    .await
}

pub async fn create_item(
    db: &PgPool,
    user_id: i32,
    name: &str,
    tags: &str,
    description: &str,
    location: &str,
) -> Result<i32, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        INSERT INTO items (user_id, name, tags, description, location, last_seen, searching)
        VALUES ($1, $2, $3, $4, $5, now(), false)
        RETURNING id
        "#,
        user_id,
        name,
        tags,
        description,
        location
    )
    .fetch_one(db)
    .await?;
    Ok(row.id)
}

/// `None` fields of `changes` are left untouched. Returns `None` when the item
/// does not exist or belongs to somebody else.
pub async fn update_item(
    db: &PgPool,
    user_id: i32,
    id: i32,
    changes: ModifyItemRequest,
) -> Result<Option<i32>, sqlx::Error> {
    let ModifyItemRequest {
        name,
        tags,
        desc: description,
        loc: location,
        searching,
    } = changes;
    let row = sqlx::query!(
        r#"
        UPDATE items
        SET
            name = COALESCE($1, name),
            description = COALESCE($2, description),
            tags = COALESCE($3, tags),
            location = COALESCE($4, location),
            searching = COALESCE($5, searching)
        WHERE user_id = $6 AND id = $7
        RETURNING id
        "#,
        name,
        description,
        tags,
        location,
        searching,
        user_id,
        id
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| r.id))
}

pub async fn delete_item(db: &PgPool, user_id: i32, id: i32) -> Result<Option<i32>, sqlx::Error> {
    let row = sqlx::query!(
        "DELETE FROM items WHERE user_id = $1 AND id = $2 RETURNING id",
        user_id,
        id
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| r.id))
}

/// Bump `last_seen` to now. Not scoped to a user: whoever finds the item and
/// opens its public page may report it, which is the point of the qr codes.
pub async fn mark_seen(db: &PgPool, id: i32) -> Result<Option<i32>, sqlx::Error> {
    let row = sqlx::query!(
        "UPDATE items SET last_seen = now() WHERE id = $1 RETURNING id",
        id
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| r.id))
}

/// Give user 1 a known access key on boot, so a fresh deployment is usable.
pub async fn bootstrap_key(db: &PgPool, keytext: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO accesskeys (user_id, keytext, expiry)
        SELECT 1, $1, NULL
        WHERE EXISTS (SELECT 1 FROM users WHERE id = 1)
          AND NOT EXISTS (SELECT 1 FROM accesskeys WHERE keytext = $1)
        "#,
        keytext
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Look an account up by email, case insensitively.
pub async fn user_by_email(db: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT id, email, passhash FROM users WHERE lower(email) = lower($1)",
        email
    )
    .fetch_optional(db)
    .await
}

/// Create an account. `Ok(None)` means the email is already taken.
pub async fn create_user(
    db: &PgPool,
    email: &str,
    passhash: &str,
) -> Result<Option<i32>, sqlx::Error> {
    let created = sqlx::query!(
        "INSERT INTO users (email, passhash) VALUES ($1, $2) RETURNING id",
        email,
        passhash
    )
    .fetch_one(db)
    .await;
    match created {
        Ok(row) => Ok(Some(row.id)),
        // Somebody signed up with the same address in between our check and
        // this insert; the unique index caught it.
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Ok(None),
        Err(e) => Err(e),
    }
}

/// Mint an access key for a browser session.
pub async fn create_session_key(
    db: &PgPool,
    user_id: i32,
    keytext: &str,
    days: i64,
) -> Result<(), sqlx::Error> {
    let expiry = chrono::Utc::now() + chrono::Duration::days(days);
    sqlx::query!(
        "INSERT INTO accesskeys (user_id, keytext, expiry, session) VALUES ($1, $2, $3, true)",
        user_id,
        keytext,
        expiry
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Drop a session key on logout, so the cookie is dead even if it was copied.
/// Keys you made by hand for an api client are left alone.
pub async fn revoke_key(db: &PgPool, keytext: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM accesskeys WHERE keytext = $1 AND session",
        keytext
    )
    .execute(db)
    .await?;
    Ok(())
}
