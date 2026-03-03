use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;

use apihop_core::models::{AuthConfig, Collection};
use crate::auth::OptionalAuthUser;
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/collections", get(list).post(create))
        .route("/collections/{id}", get(get_one).put(update).delete(remove))
}

async fn list(State(state): State<AppState>, auth_user: OptionalAuthUser) -> Result<Json<Vec<Collection>>, AppError> {
    let collections = state.storage().list_collections(auth_user.user_id()).await?;
    Ok(Json(collections))
}

async fn get_one(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
) -> Result<Json<Collection>, AppError> {
    let collection = state.storage().get_collection(&id, auth_user.user_id()).await?;
    Ok(Json(collection))
}

#[derive(Deserialize)]
struct CreateCollectionBody {
    name: String,
    description: Option<String>,
    auth: Option<AuthConfig>,
    pre_request_script: Option<String>,
    test_script: Option<String>,
    workspace_id: Option<String>,
}

async fn create(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Json(body): Json<CreateCollectionBody>,
) -> Result<(StatusCode, Json<Collection>), AppError> {
    let storage = state.storage();

    // Verify editor role when workspace_id is provided
    if let Some(ref ws_id) = body.workspace_id {
        if let Some(uid) = auth_user.user_id() {
            let member = storage
                .get_workspace_member(ws_id, uid)
                .await
                .map_err(|_| AppError::Forbidden("Not a member of this workspace".into()))?;
            if member.role.rank() < apihop_core::models::WorkspaceRole::Editor.rank() {
                return Err(AppError::Forbidden("Requires editor role or higher".into()));
            }
        }
    }

    let collection = storage
        .create_collection(
            &body.name,
            body.description.as_deref(),
            body.auth.as_ref(),
            body.pre_request_script.as_deref(),
            body.test_script.as_deref(),
            auth_user.user_id(),
            body.workspace_id.as_deref(),
        )
        .await?;
    Ok((StatusCode::CREATED, Json(collection)))
}

async fn update(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
    Json(body): Json<CreateCollectionBody>,
) -> Result<Json<Collection>, AppError> {
    let collection = state
        .storage()
        .update_collection(
            &id,
            &body.name,
            body.description.as_deref(),
            body.auth.as_ref(),
            body.pre_request_script.as_deref(),
            body.test_script.as_deref(),
            auth_user.user_id(),
        )
        .await?;
    Ok(Json(collection))
}

async fn remove(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.storage().delete_collection(&id, auth_user.user_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}
