//! The json api. Same data as the html pages, authenticated with the
//! `x-api-key` header (a cookie works too, so the api is browsable while
//! logged in).

use crate::{
    AppState,
    auth::Auth,
    db,
    models::{CreateItemRequest, ItemInfo, ModifyItemRequest, SearchQuery},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use std::sync::Arc;

/// `POST /api/items`
pub async fn create_item(
    State(state): State<Arc<AppState>>,
    who: Auth,
    Json(body): Json<CreateItemRequest>,
) -> Result<Json<i32>, StatusCode> {
    if body.name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    db::create_item(
        &state.db,
        who.user_id,
        &body.name,
        &body.tags,
        &body.desc,
        &body.loc,
    )
    .await
    .map(Json)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// `GET /api/items/{id}`
pub async fn get_item_info(
    State(state): State<Arc<AppState>>,
    who: Auth,
    Path(id): Path<i32>,
) -> Result<Json<ItemInfo>, StatusCode> {
    match db::item_of(&state.db, who.user_id, id).await {
        Ok(Some(item)) => Ok(Json(item)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// `PATCH /api/items/{id}`
pub async fn edit_item(
    State(state): State<Arc<AppState>>,
    who: Auth,
    Path(id): Path<i32>,
    Json(body): Json<ModifyItemRequest>,
) -> Result<Json<i32>, StatusCode> {
    if body.name.as_ref().is_some_and(|n| n.trim().is_empty()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    match db::update_item(
        &state.db,
        who.user_id,
        id,
        body.name,
        body.tags,
        body.desc,
        body.loc,
        body.searching,
    )
    .await
    {
        Ok(Some(id)) => Ok(Json(id)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// `DELETE /api/items/{id}`
pub async fn delete_item(
    State(state): State<Arc<AppState>>,
    who: Auth,
    Path(id): Path<i32>,
) -> Result<Json<i32>, StatusCode> {
    match db::delete_item(&state.db, who.user_id, id).await {
        Ok(Some(id)) => Ok(Json(id)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// `GET /api/items`
pub async fn list_items(
    State(state): State<Arc<AppState>>,
    who: Auth,
) -> Result<Json<Vec<ItemInfo>>, StatusCode> {
    db::list_items(&state.db, who.user_id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// `GET /api/search?q=`
pub async fn search_items(
    State(state): State<Arc<AppState>>,
    who: Auth,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<ItemInfo>>, StatusCode> {
    let q = query.q.unwrap_or_default();
    db::search_items(&state.db, who.user_id, q.trim())
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// `GET /api/health` &mdash; used by the container healthcheck.
pub async fn health(State(state): State<Arc<AppState>>) -> Result<Json<&'static str>, StatusCode> {
    match sqlx::query_scalar!("SELECT 1").fetch_one(&state.db).await {
        Ok(_) => Ok(Json("ok")),
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}
