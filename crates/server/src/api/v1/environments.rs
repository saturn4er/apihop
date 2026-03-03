use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;

use apihop_core::models::{Environment, Variable};
use crate::auth::OptionalAuthUser;
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/environments", get(list_environments).post(create_environment))
        .route(
            "/environments/{id}",
            get(get_environment).put(update_environment).delete(delete_environment),
        )
        .route(
            "/environments/{id}/variables",
            get(list_env_variables),
        )
        .route("/variables", get(list_global_variables).post(set_variable))
        .route("/variables/{id}", axum::routing::delete(delete_variable))
}

async fn list_environments(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
) -> Result<Json<Vec<Environment>>, AppError> {
    let envs = state.storage().list_environments(auth_user.user_id()).await?;
    Ok(Json(envs))
}

async fn get_environment(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
) -> Result<Json<Environment>, AppError> {
    let env = state.storage().get_environment(&id, auth_user.user_id()).await?;
    Ok(Json(env))
}

#[derive(Deserialize)]
struct CreateEnvironmentBody {
    name: String,
    workspace_id: Option<String>,
}

async fn create_environment(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Json(body): Json<CreateEnvironmentBody>,
) -> Result<(StatusCode, Json<Environment>), AppError> {
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

    let env = storage.create_environment(&body.name, auth_user.user_id(), body.workspace_id.as_deref()).await?;
    Ok((StatusCode::CREATED, Json(env)))
}

async fn update_environment(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
    Json(body): Json<CreateEnvironmentBody>,
) -> Result<Json<Environment>, AppError> {
    let env = state.storage().update_environment(&id, &body.name, auth_user.user_id()).await?;
    Ok(Json(env))
}

async fn delete_environment(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.storage().delete_environment(&id, auth_user.user_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_env_variables(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
) -> Result<Json<Vec<Variable>>, AppError> {
    let vars = state.storage().list_variables(Some(&id), auth_user.user_id()).await?;
    Ok(Json(vars))
}

async fn list_global_variables(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
) -> Result<Json<Vec<Variable>>, AppError> {
    let vars = state.storage().list_variables(None, auth_user.user_id()).await?;
    Ok(Json(vars))
}

async fn set_variable(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Json(body): Json<Variable>,
) -> Result<(StatusCode, Json<Variable>), AppError> {
    let var = state.storage().set_variable(&body, auth_user.user_id()).await?;
    Ok((StatusCode::CREATED, Json(var)))
}

async fn delete_variable(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.storage().delete_variable(&id, auth_user.user_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}
