//! Shared row-mapping functions for SQLite and PostgreSQL storage backends.
//!
//! Both backends store data identically; only SQL placeholder syntax and minor
//! type coercions differ. This module extracts all row-to-model mapping so that
//! `sqlite.rs` and `postgres.rs` only contain the queries themselves.

use sqlx::Row;

use crate::HttpMethod;
use crate::crypto::{decrypt_auth_secrets, encrypt_auth_secrets};
use crate::models::*;

// ── Auth helpers ────────────────────────────────────────────────────────

pub fn parse_auth(json_str: &str, encryption_key: &[u8; 32]) -> AuthConfig {
    let auth: AuthConfig = serde_json::from_str(json_str).unwrap_or_default();
    decrypt_auth_secrets(&auth, encryption_key)
}

pub fn serialize_auth(auth: &AuthConfig, encryption_key: &[u8; 32]) -> String {
    match encrypt_auth_secrets(auth, encryption_key) {
        Ok(encrypted) => {
            serde_json::to_string(&encrypted).unwrap_or_else(|_| r#"{"type":"none"}"#.into())
        }
        Err(_) => r#"{"type":"none"}"#.into(),
    }
}

// ── SQL dialect helper ────────────────────────────────────────────────

/// Rewrite Postgres-style `$1, $2, …` placeholders to SQLite-style `?`.
/// When `is_postgres` is `true` the input is returned unchanged.
pub fn sql(template: &str, is_postgres: bool) -> String {
    if is_postgres {
        return template.to_string();
    }
    let mut result = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
            result.push('?');
            i += 1; // skip '$'
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

// ── Row → Model mappers ────────────────────────────────────────────────

pub fn row_to_collection<R: Row>(row: &R, encryption_key: &[u8; 32]) -> Collection
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    let auth_str: String = row.get("auth");
    Collection {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        auth: parse_auth(&auth_str, encryption_key),
        pre_request_script: row.get("pre_request_script"),
        test_script: row.get("test_script"),
        workspace_id: row.get("workspace_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub fn row_to_folder<R: Row>(row: &R) -> Folder
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> i32: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    Folder {
        id: row.get("id"),
        collection_id: row.get("collection_id"),
        parent_folder_id: row.get("parent_folder_id"),
        name: row.get("name"),
        sort_order: row.get("sort_order"),
    }
}

pub fn row_to_saved_request<R: Row>(row: &R, encryption_key: &[u8; 32]) -> SavedRequest
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> i32: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    let headers_str: String = row.get("headers");
    let params_str: String = row.get("params");
    let method_str: String = row.get("method");
    let auth_str: String = row.get("auth");
    let request_type_str: String = row.get("request_type");
    SavedRequest {
        id: row.get("id"),
        collection_id: row.get("collection_id"),
        folder_id: row.get("folder_id"),
        name: row.get("name"),
        method: method_str.parse().unwrap_or(HttpMethod::Get),
        url: row.get("url"),
        headers: serde_json::from_str(&headers_str).unwrap_or_default(),
        body: row.get("body"),
        params: serde_json::from_str(&params_str).unwrap_or_default(),
        auth: parse_auth(&auth_str, encryption_key),
        pre_request_script: row.get("pre_request_script"),
        test_script: row.get("test_script"),
        request_type: request_type_str.parse().unwrap_or_default(),
        graphql_query: row.get("graphql_query"),
        graphql_variables: row.get("graphql_variables"),
        graphql_operation_name: row.get("graphql_operation_name"),
        extraction_rules: row.get("extraction_rules"),
        sort_order: row.get("sort_order"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub fn row_to_environment<R: Row>(row: &R) -> Environment
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    Environment {
        id: row.get("id"),
        name: row.get("name"),
        workspace_id: row.get("workspace_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub fn row_to_workspace<R: Row>(row: &R) -> Workspace
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> bool: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    Workspace {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        owner_id: row.get("owner_id"),
        is_personal: row.get("is_personal"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub fn row_to_workspace_member<R: Row>(row: &R) -> WorkspaceMember
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    let role_str: String = row.get("role");
    WorkspaceMember {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        user_id: row.get("user_id"),
        role: role_str.parse().unwrap_or(WorkspaceRole::Viewer),
        created_at: row.get("created_at"),
    }
}

pub fn row_to_workspace_invite<R: Row>(row: &R) -> WorkspaceInvite
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    let role_str: String = row.get("role");
    WorkspaceInvite {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        email: row.get("email"),
        role: role_str.parse().unwrap_or(WorkspaceRole::Viewer),
        token: row.get("token"),
        expires_at: row.get("expires_at"),
        created_at: row.get("created_at"),
    }
}

pub fn row_to_ws_session<R: Row>(row: &R) -> WsSession
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<i64>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> i64: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    let duration: Option<i64> = row.get("duration_ms");
    let count: i64 = row.get("message_count");
    WsSession {
        id: row.get("id"),
        url: row.get("url"),
        connected_at: row.get("connected_at"),
        disconnected_at: row.get("disconnected_at"),
        duration_ms: duration.map(|v| v as u64),
        message_count: count as u64,
    }
}

pub fn row_to_history_entry<R: Row>(row: &R) -> HistoryEntry
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> i32: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> i64: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    let method_str: String = row.get("method");
    let status: i32 = row.get("response_status");
    let duration: i64 = row.get("duration_ms");
    HistoryEntry {
        id: row.get("id"),
        method: method_str.parse().unwrap_or(HttpMethod::Get),
        url: row.get("url"),
        request_headers: row.get("request_headers"),
        request_body: row.get("request_body"),
        response_status: status as u16,
        response_headers: row.get("response_headers"),
        response_body: row.get("response_body"),
        duration_ms: duration as u64,
        timestamp: row.get("timestamp"),
    }
}

pub fn row_to_user<R: Row>(row: &R) -> crate::models::User
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    crate::models::User {
        id: row.get("id"),
        email: row.get("email"),
        display_name: row.get("display_name"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub fn row_to_user_with_hash<R: Row>(row: &R) -> crate::models::UserWithHash
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    crate::models::UserWithHash {
        id: row.get("id"),
        email: row.get("email"),
        password_hash: row.get("password_hash"),
        display_name: row.get("display_name"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub fn row_to_refresh_token<R: Row>(row: &R) -> crate::models::RefreshToken
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    crate::models::RefreshToken {
        id: row.get("id"),
        user_id: row.get("user_id"),
        token_hash: row.get("token_hash"),
        expires_at: row.get("expires_at"),
        created_at: row.get("created_at"),
    }
}

pub fn row_to_connection<R: Row>(row: &R, encryption_key: &[u8; 32]) -> crate::models::ServerConnection
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    let access_token_enc: Option<String> = row.get("access_token");
    let refresh_token_enc: Option<String> = row.get("refresh_token");

    let access_token = access_token_enc
        .as_deref()
        .and_then(|e| crate::crypto::decrypt(e, encryption_key).ok());
    let refresh_token = refresh_token_enc
        .as_deref()
        .and_then(|e| crate::crypto::decrypt(e, encryption_key).ok());

    crate::models::ServerConnection {
        id: row.get("id"),
        server_url: row.get("server_url"),
        display_name: row.get("display_name"),
        access_token,
        refresh_token,
        user_email: row.get("user_email"),
        user_display_name: row.get("user_display_name"),
        user_server_id: row.get("user_server_id"),
        server_mode: row.get("server_mode"),
        status: row.get("status"),
        created_at: row.get("created_at"),
        last_used_at: row.get("last_used_at"),
    }
}

pub fn row_to_variable<R: Row>(row: &R) -> Variable
where
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> bool: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    Variable {
        id: row.get("id"),
        environment_id: row.get("environment_id"),
        key: row.get("key"),
        value: row.get("value"),
        is_secret: row.get("is_secret"),
    }
}
