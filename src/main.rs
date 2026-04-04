use axum::{
    Json, Router,
    extract::{State, Path},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::{self, PgPoolOptions}};
use std::sync::Arc;

struct AppState {
    docker: Docker,
    db: PgPool,
    api_key: String,
}

#[derive(Serialize)]
struct ContainerInfo {
    id: String,
    name: String,
    image: String,
    state: String,
}

#[derive(Deserialize)]
struct CreateContainerRequest {
    name: String,
    image: String,
}

#[derive(Serialize)]
struct ApiResponse {
    message: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct AuditLog {
    id: i64,
    action: String,
    container_name: String,
    timestamp: String,
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

async fn log_action(db: &SqlitePool, action: &str, container_name: &str) {
    let _ = sqlx::query("INSERT INTO audit_log (action, container_name) VALUES (?, ?)")
        .bind(action)
        .bind(container_name)
        .execute(db)
        .await;
}

async fn health() -> Json<ApiResponse> {
    Json(ApiResponse {
        message: "ok".to_string(),
    })
}

#[tokio::main]
async fn main() {
    let api_key = std::env::var("API_KEY").expect("API_KEY must be set");
    let db_url = "postgres://pg.local.olio.ovh";
    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect(db_url)
        .await
        .expect(&format!("Failed to connect to db at: {}", db_url));

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            action TEXT NOT NULL,
            container_name TEXT NOT NULL,
            timestamp TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(&db)
    .await
    .expect("failed to create table");


    let state = Arc::new(AppState {
        docker,
        db,
        api_key,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/containers", get(list_containers))
        .route("/containers", post(create_container))
        .route("/containers/{name}/stop", post(stop_container))
        .route("/containers/{name}", delete(remove_container))
        .route("/logs", get(get_logs))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind");

    println!("listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.expect("server error");
}