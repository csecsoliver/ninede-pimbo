mod api;
mod auth;
mod db;
mod html;
mod models;
mod web;

use axum::{
    Router,
    routing::{delete, get, patch, post},
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;

pub struct AppState {
    pub db: PgPool,
    /// Whether the session cookie is marked `Secure`. On by default; turn it
    /// off with `COOKIE_SECURE=false` when serving over plain http locally.
    pub cookie_secure: bool,
    /// Whether strangers can create accounts. On by default; turn it off with
    /// `SIGNUP_OPEN=false` to keep a deployment to yourself.
    pub signup_open: bool,
}

fn env_flag(name: &str, default: bool) -> bool {
    match std::env::var(name) {
        Ok(value) => !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "no" | "off" | ""
        ),
        Err(_) => default,
    }
}

#[tokio::main]
async fn main() {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let bind = std::env::var("BIND").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect(&db_url)
        .await
        .unwrap_or_else(|e| panic!("failed to connect to the database: {e}"));

    sqlx::migrate!()
        .run(&db)
        .await
        .expect("failed to run migrations");

    // Handy for a fresh deployment: hand user 1 a key you already know.
    if let Ok(key) = std::env::var("BOOTSTRAP_KEY") {
        let key = key.trim();
        if auth::plausible_key(key) {
            db::bootstrap_key(&db, key)
                .await
                .expect("failed to store BOOTSTRAP_KEY");
            println!("BOOTSTRAP_KEY is available as an access key for user 1");
        } else {
            eprintln!("BOOTSTRAP_KEY is not a usable key, ignoring it");
        }
    }

    let state = Arc::new(AppState {
        db,
        cookie_secure: env_flag("COOKIE_SECURE", true),
        signup_open: env_flag("SIGNUP_OPEN", true),
    });

    let app = Router::new()
        // json api
        .route("/api/items", post(api::create_item).get(api::list_items))
        .route("/api/items/{id}", get(api::get_item_info))
        .route("/api/items/{id}", patch(api::edit_item))
        .route("/api/items/{id}", delete(api::delete_item))
        .route("/api/search", get(api::search_items))
        .route("/api/health", get(api::health))
        // html pages
        .route("/", get(web::index))
        .route("/login", get(web::login_page).post(web::login_submit))
        .route("/signup", get(web::signup_page).post(web::signup_submit))
        .route("/logout", post(web::logout))
        .route("/dash", get(web::dash))
        .route("/search", get(web::search))
        .route("/items", post(web::create_item))
        .route("/items/new", get(web::new_item_page))
        .route("/items/{id}", get(web::item_page).post(web::update_item))
        .route("/items/{id}/seen", post(web::mark_seen))
        .route("/items/{id}/searching", post(web::set_searching))
        .route("/items/{id}/delete", post(web::delete_item))
        // what a stranger who finds one of your things sees
        .route("/_{id}", get(web::public_item))
        .route("/_{id}/seen", post(web::public_mark_seen))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {bind}: {e}"));

    println!("listening on http://{bind}");
    axum::serve(listener, app).await.expect("server error");
}
