use axum::{
    Error, Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, patch, post},
};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{
    Database, PgPool,
    postgres::{self, PgPoolOptions},
};
use std::{collections::linked_list, fmt::format, os::linux::raw::stat, sync::Arc};

struct AppState {
    db: PgPool,
    api_key: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct ItemInfo {
    id: i32,
    user_id: Option<i32>,
    name: String,
    tags: String,
    description: String,
    location: String,
    last_seen: chrono::DateTime<Local>,
    searching: bool,
}

#[derive(Deserialize)]
struct CreateItemRequest {
    name: String,
    tags: String,
    desc: String,
    loc: String,
}
#[derive(Deserialize)]
struct ModifyItemRequest {
    name: Option<String>,
    tags: Option<String>,
    desc: Option<String>,
    loc: Option<String>,
    searching: Option<bool>,
}

#[derive(Serialize)]
struct ApiResponse {
    message: String,
}
#[derive(Serialize, sqlx::FromRow, Default)]
struct User {
    id: i32,
    email: Option<String>,
    passhash: Option<String>,
}
#[derive(Serialize, sqlx::FromRow)]
struct Accesskey {
    id: i32,
    user_id: i32,
    keytext: String,
    expiry: Option<chrono::DateTime<Local>>,
}

fn check_auth(headers: &HeaderMap, api_key: &str) -> Result<(), (StatusCode, Json<ApiResponse>)> {
    let provided = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided != api_key {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse {
                message: "invalid or missing API key".to_string(),
            }),
        ));
    }

    Ok(())
}

async fn log_action(db: &PgPool, action: &str, container_name: &str) {
    let _ = sqlx::query("INSERT INTO audit_log (action, container_name) VALUES (?, ?)")
        .bind(action)
        .bind(container_name)
        .execute(db)
        .await;
}
async fn create_item(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateItemRequest>,
) {
    let _ = sqlx::query!(r#"
         INSERT INTO items (user_id, name, tags, description, location, last_seen, searching) VALUES ($1, $2, $3, $4, $5, $6, $7);
         "#, 1, body.name, body.tags, body.desc, body.loc, chrono::offset::Local::now(), false).execute(&state.db).await;
}
async fn get_item_info(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i32>,
) -> Result<Json<ItemInfo>, StatusCode> {
    let result = sqlx::query_as!(ItemInfo, r#"
        SELECT id, user_id, name, tags, description, location, last_seen, searching FROM items WHERE user_id = 1 AND id = $1
        "#, id).fetch_optional(&state.db).await;
    match result {
        Ok(Some(r)) => Ok(Json(r)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
async fn edit_item(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i32>,
    Json(body): Json<ModifyItemRequest>,
) -> Result<(), StatusCode> {
    let result = sqlx::query!(
        r#"
        UPDATE items
        SET
            name = COALESCE($1, name),
            description = COALESCE($2, description),
            tags = COALESCE($3, tags),
            location = COALESCE($4, location),
            searching = COALESCE($5, searching)
        WHERE user_id = 1 AND id = $6
        RETURNING id;
        "#,
        body.name,
        body.desc,
        body.tags,
        body.loc,
        body.searching,
        id
    )
    .fetch_optional(&state.db)
    .await;
    match result {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
async fn delete_item(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i32>,
) -> Result<(), StatusCode> {
    let result = sqlx::query!(
        r#"
        DELETE FROM items WHERE id = $1
        "#,
        id
    )
    .execute(&state.db)
    .await;
    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
async fn list_items(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ItemInfo>>, StatusCode> {
    let result = sqlx::query_as!(
        ItemInfo,
        r#"
        SELECT id, user_id, name, tags, description, location, last_seen, searching FROM items
        "#
    )
    .fetch_all(&state.db)
    .await;
    match result {
        Ok(r) => Ok(Json(r)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
async fn scanned_item(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i32>,
) -> Result<String, StatusCode> {
    let result = sqlx::query_as!(ItemInfo, r#"
        SELECT id, user_id, name, tags, description, location, last_seen, searching FROM items WHERE id = $1
        "#, id).fetch_optional(&state.db).await;
    match result {
        Ok(Some(r)) => {
            let user = sqlx::query_as!(
                User,
                "SELECT id, email, passhash FROM users WHERE ID = $1",
                r.user_id
            )
            .fetch_optional(&state.db)
            .await;
            Ok(match user.ok().flatten() {
                Some(u) => {
                    format!(
                        r#"
                        This item belongs to {} <br>
                        Name: {} <br>
                        Its location is supposed to be: {} <br>
                        Is the owner searching for it: {}
                        "#,
                        match u.email {
                            Some(e) => e,
                            None => u.id.to_string(),
                        },
                        r.name,
                        r.location,
                        r.searching,
                    )
                }
                None => {
                    format!(
                        r#"
                        This item got orphaned <br>
                        Name: {} <br>
                        Its location is supposed to be: {} <br>
                        Is the owner (well, we don't know who it is) searching for it: {}
                        "#,
                        r.name,
                        r.location,
                        r.searching,
                    )
                }
            })
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
async fn health() -> Json<ApiResponse> {
    Json(ApiResponse {
        message: "ok".to_string(),
    })
}

#[tokio::main]
async fn main() {
    let api_key = std::env::var("API_KEY").unwrap_or(("asd").to_string());
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect(&db_url)
        .await
        .expect(&format!("Failed to connect to db at: {}", db_url));

    let _ = sqlx::migrate!().run(&db).await;

    let state = Arc::new(AppState { db, api_key });

    let app = Router::new()
        .route("/api/items", post(create_item))
        .route("/api/items/{id}", get(get_item_info))
        .route("/api/items/{id}", patch(edit_item))
        .route("/api/items/{id}", delete(delete_item))
        .route("/api/items", get(list_items))
        .route("/#{id}", get(scanned_item))
        // .route("/#{id}/seen", post(mark_item_seen))
        // .route("/search", get(search_item))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind");

    println!("listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.expect("server error");
}
