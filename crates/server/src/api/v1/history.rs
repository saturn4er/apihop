use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get},
};
use serde::Deserialize;

use apihop_core::models::HistoryEntry;
use crate::auth::OptionalAuthUser;
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/history", get(list).post(create).delete(clear))
        .route("/history/{id}", delete(remove))
}

#[derive(Deserialize)]
struct HistoryQuery {
    #[serde(default = "default_limit")]
    limit: u32,
    #[serde(default)]
    offset: u32,
}

fn default_limit() -> u32 {
    50
}

async fn list(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<HistoryEntry>>, AppError> {
    let entries = state
        .storage()
        .list_history(query.limit, query.offset, auth_user.user_id())
        .await?;
    Ok(Json(entries))
}

async fn create(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Json(entry): Json<HistoryEntry>,
) -> Result<(StatusCode, Json<HistoryEntry>), AppError> {
    let entry = state.storage().create_history_entry(&entry, auth_user.user_id()).await?;
    Ok((StatusCode::CREATED, Json(entry)))
}

async fn remove(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.storage().delete_history_entry(&id, auth_user.user_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn clear(State(state): State<AppState>, auth_user: OptionalAuthUser) -> Result<StatusCode, AppError> {
    state.storage().clear_history(auth_user.user_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}
