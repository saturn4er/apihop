use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::Deserialize;

use apihop_core::models::{
    Collection, Environment, Workspace, WorkspaceInvite, WorkspaceMember, WorkspaceRole,
};
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/workspaces", get(list_workspaces).post(create_workspace))
        .route(
            "/workspaces/{id}",
            get(get_workspace).put(update_workspace).delete(delete_workspace),
        )
        .route("/workspaces/{id}/collections", get(list_workspace_collections))
        .route("/workspaces/{id}/environments", get(list_workspace_environments))
        .route("/workspaces/{id}/members", get(list_members).post(add_member))
        .route(
            "/workspaces/{id}/members/{user_id}",
            axum::routing::put(update_member_role).delete(remove_member),
        )
        .route("/workspaces/{id}/leave", post(leave_workspace))
        .route("/workspaces/{id}/invites", get(list_invites).post(create_invite))
        .route(
            "/workspaces/{id}/invites/{invite_id}",
            axum::routing::delete(revoke_invite),
        )
        .route("/workspaces/accept-invite", post(accept_invite))
}

// ── Authorization helper ────────────────────────────────────────

async fn require_workspace_role(
    storage: &dyn apihop_core::storage::StorageBackend,
    workspace_id: &str,
    user_id: &str,
    min_role: WorkspaceRole,
) -> Result<WorkspaceMember, AppError> {
    let member = storage
        .get_workspace_member(workspace_id, user_id)
        .await
        .map_err(|_| AppError::Forbidden("Not a member of this workspace".into()))?;
    if member.role.rank() < min_role.rank() {
        return Err(AppError::Forbidden(format!(
            "Requires {} role or higher",
            min_role
        )));
    }
    Ok(member)
}

// ── Workspace CRUD ──────────────────────────────────────────────

async fn list_workspaces(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Workspace>>, AppError> {
    let workspaces = state.storage().list_user_workspaces(&auth.user_id).await?;
    Ok(Json(workspaces))
}

#[derive(Deserialize)]
struct CreateWorkspaceBody {
    name: String,
    description: Option<String>,
}

async fn create_workspace(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateWorkspaceBody>,
) -> Result<(StatusCode, Json<Workspace>), AppError> {
    let storage = state.storage();
    let ws = storage
        .create_workspace(&body.name, body.description.as_deref(), &auth.user_id, false)
        .await?;
    storage
        .add_workspace_member(&ws.id, &auth.user_id, "owner")
        .await?;
    Ok((StatusCode::CREATED, Json(ws)))
}

async fn get_workspace(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Workspace>, AppError> {
    let storage = state.storage();
    require_workspace_role(storage, &id, &auth.user_id, WorkspaceRole::Viewer).await?;
    let ws = storage.get_workspace(&id).await?;
    Ok(Json(ws))
}

#[derive(Deserialize)]
struct UpdateWorkspaceBody {
    name: String,
    description: Option<String>,
}

async fn update_workspace(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateWorkspaceBody>,
) -> Result<Json<Workspace>, AppError> {
    let storage = state.storage();
    require_workspace_role(storage, &id, &auth.user_id, WorkspaceRole::Owner).await?;
    let ws = storage
        .update_workspace(&id, &body.name, body.description.as_deref())
        .await?;
    Ok(Json(ws))
}

async fn delete_workspace(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let storage = state.storage();
    require_workspace_role(storage, &id, &auth.user_id, WorkspaceRole::Owner).await?;
    let ws = storage.get_workspace(&id).await?;
    if ws.is_personal {
        return Err(AppError::BadRequest("Cannot delete personal workspace".into()));
    }
    storage.delete_workspace(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Workspace-scoped resources ──────────────────────────────────

async fn list_workspace_collections(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Vec<Collection>>, AppError> {
    let storage = state.storage();
    require_workspace_role(storage, &id, &auth.user_id, WorkspaceRole::Viewer).await?;
    let collections = storage.list_workspace_collections(&id).await?;
    Ok(Json(collections))
}

async fn list_workspace_environments(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Vec<Environment>>, AppError> {
    let storage = state.storage();
    require_workspace_role(storage, &id, &auth.user_id, WorkspaceRole::Viewer).await?;
    let environments = storage.list_workspace_environments(&id).await?;
    Ok(Json(environments))
}

// ── Members ─────────────────────────────────────────────────────

async fn list_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Vec<WorkspaceMember>>, AppError> {
    let storage = state.storage();
    require_workspace_role(storage, &id, &auth.user_id, WorkspaceRole::Viewer).await?;
    let members = storage.list_workspace_members(&id).await?;
    Ok(Json(members))
}

#[derive(Deserialize)]
struct AddMemberBody {
    user_id: String,
    role: String,
}

async fn add_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<AddMemberBody>,
) -> Result<(StatusCode, Json<WorkspaceMember>), AppError> {
    let storage = state.storage();
    require_workspace_role(storage, &id, &auth.user_id, WorkspaceRole::Owner).await?;
    let member = storage
        .add_workspace_member(&id, &body.user_id, &body.role)
        .await?;
    Ok((StatusCode::CREATED, Json(member)))
}

#[derive(Deserialize)]
struct UpdateMemberRoleBody {
    role: String,
}

async fn update_member_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, member_user_id)): Path<(String, String)>,
    Json(body): Json<UpdateMemberRoleBody>,
) -> Result<Json<WorkspaceMember>, AppError> {
    let storage = state.storage();
    require_workspace_role(storage, &id, &auth.user_id, WorkspaceRole::Owner).await?;
    let member = storage
        .update_workspace_member_role(&id, &member_user_id, &body.role)
        .await?;
    Ok(Json(member))
}

async fn remove_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, member_user_id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    let storage = state.storage();
    require_workspace_role(storage, &id, &auth.user_id, WorkspaceRole::Owner).await?;
    storage.remove_workspace_member(&id, &member_user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn leave_workspace(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let storage = state.storage();
    let ws = storage.get_workspace(&id).await?;
    if ws.is_personal {
        return Err(AppError::BadRequest("Cannot leave personal workspace".into()));
    }
    storage.remove_workspace_member(&id, &auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Invites ─────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateInviteBody {
    email: String,
    role: String,
}

async fn create_invite(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<CreateInviteBody>,
) -> Result<(StatusCode, Json<WorkspaceInvite>), AppError> {
    let storage = state.storage();
    require_workspace_role(storage, &id, &auth.user_id, WorkspaceRole::Owner).await?;
    let token = uuid::Uuid::new_v4().to_string();
    let expires_at =
        (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339();
    let invite = storage
        .create_workspace_invite(&id, &body.email, &body.role, &token, &expires_at)
        .await?;
    Ok((StatusCode::CREATED, Json(invite)))
}

async fn list_invites(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Vec<WorkspaceInvite>>, AppError> {
    let storage = state.storage();
    require_workspace_role(storage, &id, &auth.user_id, WorkspaceRole::Owner).await?;
    let invites = storage.list_workspace_invites(&id).await?;
    Ok(Json(invites))
}

async fn revoke_invite(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, invite_id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    let storage = state.storage();
    require_workspace_role(storage, &id, &auth.user_id, WorkspaceRole::Owner).await?;
    storage.delete_workspace_invite(&invite_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct AcceptInviteBody {
    token: String,
}

async fn accept_invite(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<AcceptInviteBody>,
) -> Result<Json<Workspace>, AppError> {
    let storage = state.storage();
    let invite = storage
        .get_workspace_invite_by_token(&body.token)
        .await
        .map_err(|_| AppError::NotFound("Invalid or expired invite token".into()))?;
    storage
        .add_workspace_member(&invite.workspace_id, &auth.user_id, &invite.role.to_string())
        .await?;
    storage.delete_workspace_invite(&invite.id).await?;
    let ws = storage.get_workspace(&invite.workspace_id).await?;
    Ok(Json(ws))
}
