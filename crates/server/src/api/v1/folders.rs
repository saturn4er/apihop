use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;

use apihop_core::models::Folder;
use crate::auth::OptionalAuthUser;
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/collections/{id}/folders", get(list).post(create))
        .route(
            "/collections/{collection_id}/folders/{folder_id}",
            axum::routing::put(update).delete(remove),
        )
}

#[derive(Deserialize)]
struct FolderQuery {
    parent_folder_id: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(collection_id): Path<String>,
    Query(query): Query<FolderQuery>,
) -> Result<Json<Vec<Folder>>, AppError> {
    let folders = state
        .storage()
        .list_folders(&collection_id, query.parent_folder_id.as_deref(), auth_user.user_id())
        .await?;
    Ok(Json(folders))
}

#[derive(Deserialize)]
struct CreateFolderBody {
    name: String,
    parent_folder_id: Option<String>,
}

async fn create(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(collection_id): Path<String>,
    Json(body): Json<CreateFolderBody>,
) -> Result<(StatusCode, Json<Folder>), AppError> {
    let folder = state
        .storage()
        .create_folder(
            &collection_id,
            body.parent_folder_id.as_deref(),
            &body.name,
            auth_user.user_id(),
        )
        .await?;
    Ok((StatusCode::CREATED, Json(folder)))
}

#[derive(Deserialize)]
struct UpdateFolderBody {
    name: String,
}

async fn update(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path((_collection_id, folder_id)): Path<(String, String)>,
    Json(body): Json<UpdateFolderBody>,
) -> Result<Json<Folder>, AppError> {
    let folder = state.storage().update_folder(&folder_id, &body.name, auth_user.user_id()).await?;
    Ok(Json(folder))
}

async fn remove(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path((_collection_id, folder_id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    state.storage().delete_folder(&folder_id, auth_user.user_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}
