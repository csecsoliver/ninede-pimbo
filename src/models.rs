use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Serialize, sqlx::FromRow)]
pub struct ItemInfo {
    pub id: i32,
    pub user_id: Option<i32>,
    pub name: String,
    pub tags: String,
    pub description: String,
    pub location: String,
    pub last_seen: chrono::DateTime<Utc>,
    pub searching: bool,
}

#[derive(Serialize, sqlx::FromRow, Default)]
pub struct User {
    pub id: i32,
    pub email: Option<String>,
    pub passhash: Option<String>,
}

/// Body of `POST /api/items`, and of the `POST /items` html form.
#[derive(Deserialize)]
pub struct CreateItemRequest {
    pub name: String,
    pub tags: String,
    pub desc: String,
    pub loc: String,
}

/// Body of `PATCH /api/items/{id}`, and of the `POST /items/{id}` html form.
/// Every field is optional: absent means "leave as it is".
#[derive(Deserialize, Default)]
pub struct ModifyItemRequest {
    pub name: Option<String>,
    pub tags: Option<String>,
    pub desc: Option<String>,
    pub loc: Option<String>,
    pub searching: Option<bool>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

impl ItemInfo {
    /// `2026-08-20 14:31 UTC`, for display in html.
    pub fn seen_at(&self) -> String {
        self.last_seen.format("%Y-%m-%d %H:%M UTC").to_string()
    }
}
