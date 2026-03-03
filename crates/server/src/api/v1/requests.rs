use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;

use apihop_core::models::SavedRequest;
use crate::auth::OptionalAuthUser;
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/requests", get(list).post(create))
        .route(
            "/requests/{id}",
            get(get_one).put(update).delete(remove),
        )
}

#[derive(Deserialize)]
struct RequestQuery {
    collection_id: String,
    folder_id: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Query(query): Query<RequestQuery>,
) -> Result<Json<Vec<SavedRequest>>, AppError> {
    let requests = state
        .storage()
        .list_requests(&query.collection_id, query.folder_id.as_deref(), auth_user.user_id())
        .await?;
    Ok(Json(requests))
}

async fn get_one(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
) -> Result<Json<SavedRequest>, AppError> {
    let request = state.storage().get_request(&id, auth_user.user_id()).await?;
    Ok(Json(request))
}

async fn create(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Json(req): Json<SavedRequest>,
) -> Result<(StatusCode, Json<SavedRequest>), AppError> {
    let request = state.storage().create_request(&req, auth_user.user_id()).await?;
    Ok((StatusCode::CREATED, Json(request)))
}

async fn update(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
    Json(mut req): Json<SavedRequest>,
) -> Result<Json<SavedRequest>, AppError> {
    req.id = id;
    let request = state.storage().update_request(&req, auth_user.user_id()).await?;
    Ok(Json(request))
}

async fn remove(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.storage().delete_request(&id, auth_user.user_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}
