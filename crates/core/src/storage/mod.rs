pub mod common;
pub mod migrator;
#[macro_use]
mod impl_backend;
pub mod sqlite;
#[cfg(feature = "postgres")]
pub mod postgres;

use std::future::Future;
use std::pin::Pin;

use thiserror::Error;

use crate::models::*;

/// Generate a new UUID v4 string.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Current UTC time as ISO 8601 / RFC 3339 string.
pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("Not found: {0}")]
    NotFound(String),
}

impl From<sqlx::Error> for StorageError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => StorageError::NotFound("Record not found".into()),
            other => StorageError::Database(Box::new(other)),
        }
    }
}

// We use `BoxFuture` instead of `async fn` in the trait because `StorageBackend` is used
// as `dyn StorageBackend` (trait object) in both server and desktop AppState. Native
// `async fn` in traits is not object-safe, so `BoxFuture` is required for dynamic dispatch.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait StorageBackend: Send + Sync {
    // Collections
    fn list_collections(&self, user_id: Option<&str>) -> BoxFuture<'_, Result<Vec<Collection>, StorageError>>;
    fn get_collection(&self, id: &str, user_id: Option<&str>) -> BoxFuture<'_, Result<Collection, StorageError>>;
    fn create_collection(
        &self,
        name: &str,
        description: Option<&str>,
        auth: Option<&crate::models::AuthConfig>,
        pre_request_script: Option<&str>,
        test_script: Option<&str>,
        user_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> BoxFuture<'_, Result<Collection, StorageError>>;
    fn update_collection(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        auth: Option<&crate::models::AuthConfig>,
        pre_request_script: Option<&str>,
        test_script: Option<&str>,
        user_id: Option<&str>,
    ) -> BoxFuture<'_, Result<Collection, StorageError>>;
    fn delete_collection(&self, id: &str, user_id: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>>;

    // Folders
    fn list_folders(
        &self,
        collection_id: &str,
        parent_folder_id: Option<&str>,
        user_id: Option<&str>,
    ) -> BoxFuture<'_, Result<Vec<Folder>, StorageError>>;
    fn create_folder(
        &self,
        collection_id: &str,
        parent_folder_id: Option<&str>,
        name: &str,
        user_id: Option<&str>,
    ) -> BoxFuture<'_, Result<Folder, StorageError>>;
    fn update_folder(&self, id: &str, name: &str, user_id: Option<&str>) -> BoxFuture<'_, Result<Folder, StorageError>>;
    fn delete_folder(&self, id: &str, user_id: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>>;

    // Saved Requests
    fn list_requests(
        &self,
        collection_id: &str,
        folder_id: Option<&str>,
        user_id: Option<&str>,
    ) -> BoxFuture<'_, Result<Vec<SavedRequest>, StorageError>>;
    fn get_request(&self, id: &str, user_id: Option<&str>) -> BoxFuture<'_, Result<SavedRequest, StorageError>>;
    fn create_request(
        &self,
        req: &SavedRequest,
        user_id: Option<&str>,
    ) -> BoxFuture<'_, Result<SavedRequest, StorageError>>;
    fn update_request(
        &self,
        req: &SavedRequest,
        user_id: Option<&str>,
    ) -> BoxFuture<'_, Result<SavedRequest, StorageError>>;
    fn delete_request(&self, id: &str, user_id: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>>;

    // History
    fn list_history(
        &self,
        limit: u32,
        offset: u32,
        user_id: Option<&str>,
    ) -> BoxFuture<'_, Result<Vec<HistoryEntry>, StorageError>>;
    fn create_history_entry(
        &self,
        entry: &HistoryEntry,
        user_id: Option<&str>,
    ) -> BoxFuture<'_, Result<HistoryEntry, StorageError>>;
    fn delete_history_entry(&self, id: &str, user_id: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>>;
    fn clear_history(&self, user_id: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>>;
    fn prune_history(
        &self,
        max_entries: u32,
        max_age_days: u32,
    ) -> BoxFuture<'_, Result<u64, StorageError>>;

    // Environments
    fn list_environments(&self, user_id: Option<&str>) -> BoxFuture<'_, Result<Vec<Environment>, StorageError>>;
    fn get_environment(&self, id: &str, user_id: Option<&str>) -> BoxFuture<'_, Result<Environment, StorageError>>;
    fn create_environment(&self, name: &str, user_id: Option<&str>, workspace_id: Option<&str>) -> BoxFuture<'_, Result<Environment, StorageError>>;
    fn update_environment(
        &self,
        id: &str,
        name: &str,
        user_id: Option<&str>,
    ) -> BoxFuture<'_, Result<Environment, StorageError>>;
    fn delete_environment(&self, id: &str, user_id: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>>;

    // Variables
    fn list_variables(
        &self,
        environment_id: Option<&str>,
        user_id: Option<&str>,
    ) -> BoxFuture<'_, Result<Vec<Variable>, StorageError>>;
    fn set_variable(
        &self,
        variable: &Variable,
        user_id: Option<&str>,
    ) -> BoxFuture<'_, Result<Variable, StorageError>>;
    fn delete_variable(&self, id: &str, user_id: Option<&str>) -> BoxFuture<'_, Result<(), StorageError>>;

    // Users & Auth
    fn create_user(
        &self,
        email: &str,
        password_hash: &str,
        display_name: Option<&str>,
    ) -> BoxFuture<'_, Result<crate::models::User, StorageError>>;
    fn get_user_by_email(
        &self,
        email: &str,
    ) -> BoxFuture<'_, Result<crate::models::UserWithHash, StorageError>>;
    fn get_user_by_id(
        &self,
        id: &str,
    ) -> BoxFuture<'_, Result<crate::models::User, StorageError>>;
    fn list_all_users(&self) -> BoxFuture<'_, Result<Vec<crate::models::User>, StorageError>>;
    fn store_refresh_token(
        &self,
        user_id: &str,
        token_hash: &str,
        expires_at: &str,
    ) -> BoxFuture<'_, Result<(), StorageError>>;
    fn get_refresh_token(
        &self,
        token_hash: &str,
    ) -> BoxFuture<'_, Result<crate::models::RefreshToken, StorageError>>;
    fn delete_refresh_token(
        &self,
        token_hash: &str,
    ) -> BoxFuture<'_, Result<(), StorageError>>;
    fn delete_user_refresh_tokens(
        &self,
        user_id: &str,
    ) -> BoxFuture<'_, Result<(), StorageError>>;

    // Server Connections
    fn list_connections(&self) -> BoxFuture<'_, Result<Vec<crate::models::ServerConnection>, StorageError>>;
    fn get_connection(&self, id: &str) -> BoxFuture<'_, Result<crate::models::ServerConnection, StorageError>>;
    fn create_connection(
        &self,
        connection: &crate::models::ServerConnection,
    ) -> BoxFuture<'_, Result<crate::models::ServerConnection, StorageError>>;
    fn update_connection_tokens(
        &self,
        id: &str,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
    ) -> BoxFuture<'_, Result<(), StorageError>>;
    fn update_connection_status(
        &self,
        id: &str,
        status: &str,
    ) -> BoxFuture<'_, Result<(), StorageError>>;
    fn delete_connection(&self, id: &str) -> BoxFuture<'_, Result<(), StorageError>>;
    fn get_connection_tokens(
        &self,
        id: &str,
    ) -> BoxFuture<'_, Result<(Option<String>, Option<String>), StorageError>>;

    // WS Sessions
    fn create_ws_session(
        &self,
        session: &WsSession,
    ) -> BoxFuture<'_, Result<WsSession, StorageError>>;
    fn list_ws_sessions(
        &self,
        limit: u32,
        offset: u32,
    ) -> BoxFuture<'_, Result<Vec<WsSession>, StorageError>>;
    fn delete_ws_session(&self, id: &str) -> BoxFuture<'_, Result<(), StorageError>>;
    fn cleanup_orphaned_ws_sessions(&self) -> BoxFuture<'_, Result<u64, StorageError>>;

    // Workspaces
    fn create_workspace(&self, name: &str, description: Option<&str>, owner_id: &str, is_personal: bool) -> BoxFuture<'_, Result<Workspace, StorageError>>;
    fn get_workspace(&self, id: &str) -> BoxFuture<'_, Result<Workspace, StorageError>>;
    fn update_workspace(&self, id: &str, name: &str, description: Option<&str>) -> BoxFuture<'_, Result<Workspace, StorageError>>;
    fn delete_workspace(&self, id: &str) -> BoxFuture<'_, Result<(), StorageError>>;
    fn list_user_workspaces(&self, user_id: &str) -> BoxFuture<'_, Result<Vec<Workspace>, StorageError>>;
    fn get_personal_workspace(&self, user_id: &str) -> BoxFuture<'_, Result<Workspace, StorageError>>;

    // Workspace Members
    fn add_workspace_member(&self, workspace_id: &str, user_id: &str, role: &str) -> BoxFuture<'_, Result<WorkspaceMember, StorageError>>;
    fn remove_workspace_member(&self, workspace_id: &str, user_id: &str) -> BoxFuture<'_, Result<(), StorageError>>;
    fn update_workspace_member_role(&self, workspace_id: &str, user_id: &str, role: &str) -> BoxFuture<'_, Result<WorkspaceMember, StorageError>>;
    fn list_workspace_members(&self, workspace_id: &str) -> BoxFuture<'_, Result<Vec<WorkspaceMember>, StorageError>>;
    fn get_workspace_member(&self, workspace_id: &str, user_id: &str) -> BoxFuture<'_, Result<WorkspaceMember, StorageError>>;

    // Workspace Invites
    fn create_workspace_invite(&self, workspace_id: &str, email: &str, role: &str, token: &str, expires_at: &str) -> BoxFuture<'_, Result<WorkspaceInvite, StorageError>>;
    fn get_workspace_invite_by_token(&self, token: &str) -> BoxFuture<'_, Result<WorkspaceInvite, StorageError>>;
    fn delete_workspace_invite(&self, id: &str) -> BoxFuture<'_, Result<(), StorageError>>;
    fn list_workspace_invites(&self, workspace_id: &str) -> BoxFuture<'_, Result<Vec<WorkspaceInvite>, StorageError>>;

    // Workspace-scoped queries
    fn list_workspace_collections(&self, workspace_id: &str) -> BoxFuture<'_, Result<Vec<Collection>, StorageError>>;
    fn list_workspace_environments(&self, workspace_id: &str) -> BoxFuture<'_, Result<Vec<Environment>, StorageError>>;
}
