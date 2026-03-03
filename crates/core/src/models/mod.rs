use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ── Deployment Mode ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentMode {
    Personal,
    Organization,
}

impl std::fmt::Display for DeploymentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeploymentMode::Personal => f.write_str("personal"),
            DeploymentMode::Organization => f.write_str("organization"),
        }
    }
}

impl std::str::FromStr for DeploymentMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "personal" => Ok(DeploymentMode::Personal),
            "organization" | "org" => Ok(DeploymentMode::Organization),
            _ => Err(format!("Unknown deployment mode: {s}")),
        }
    }
}

// ── User Model ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct UserWithHash {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl UserWithHash {
    pub fn to_user(&self) -> User {
        User {
            id: self.id.clone(),
            email: self.email.clone(),
            display_name: self.display_name.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub expires_at: String,
    pub created_at: String,
}

// ── Workspace Models ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRole {
    Owner,
    Editor,
    Viewer,
}

impl std::fmt::Display for WorkspaceRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceRole::Owner => f.write_str("owner"),
            WorkspaceRole::Editor => f.write_str("editor"),
            WorkspaceRole::Viewer => f.write_str("viewer"),
        }
    }
}

impl std::str::FromStr for WorkspaceRole {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "owner" => Ok(WorkspaceRole::Owner),
            "editor" => Ok(WorkspaceRole::Editor),
            "viewer" => Ok(WorkspaceRole::Viewer),
            _ => Err(format!("Unknown workspace role: {s}")),
        }
    }
}

impl WorkspaceRole {
    pub fn rank(&self) -> u8 {
        match self {
            WorkspaceRole::Viewer => 1,
            WorkspaceRole::Editor => 2,
            WorkspaceRole::Owner => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: String,
    pub is_personal: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMember {
    pub id: String,
    pub workspace_id: String,
    pub user_id: String,
    pub role: WorkspaceRole,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInvite {
    pub id: String,
    pub workspace_id: String,
    pub email: String,
    pub role: WorkspaceRole,
    pub token: String,
    pub expires_at: String,
    pub created_at: String,
}

// ── Server Connection Model ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConnection {
    pub id: String,
    pub server_url: String,
    pub display_name: String,
    #[serde(skip_serializing)]
    pub access_token: Option<String>,
    #[serde(skip_serializing)]
    pub refresh_token: Option<String>,
    pub user_email: Option<String>,
    pub user_display_name: Option<String>,
    pub user_server_id: Option<String>,
    pub server_mode: String,
    pub status: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    #[default]
    None,
    Basic {
        username: String,
        password: String,
    },
    Bearer {
        token: String,
    },
    ApiKey {
        key: String,
        value: String,
        add_to: ApiKeyLocation,
    },
    OAuth2ClientCredentials {
        token_url: String,
        client_id: String,
        client_secret: String,
        scope: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyLocation {
    #[default]
    Header,
    QueryParam,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RequestType {
    #[default]
    Http,
    Websocket,
    Graphql,
}

impl std::fmt::Display for RequestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestType::Http => f.write_str("http"),
            RequestType::Websocket => f.write_str("websocket"),
            RequestType::Graphql => f.write_str("graphql"),
        }
    }
}

impl std::str::FromStr for RequestType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "http" => Ok(RequestType::Http),
            "websocket" | "ws" => Ok(RequestType::Websocket),
            "graphql" | "gql" => Ok(RequestType::Graphql),
            _ => Err(format!("Unknown request type: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub pre_request_script: Option<String>,
    #[serde(default)]
    pub test_script: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub collection_id: String,
    pub parent_folder_id: Option<String>,
    pub name: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedRequest {
    pub id: String,
    pub collection_id: String,
    pub folder_id: Option<String>,
    pub name: String,
    pub method: super::HttpMethod,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    #[serde(default)]
    pub params: Vec<KeyValueParam>,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub pre_request_script: Option<String>,
    #[serde(default)]
    pub test_script: Option<String>,
    #[serde(default)]
    pub request_type: RequestType,
    #[serde(default)]
    pub graphql_query: Option<String>,
    #[serde(default)]
    pub graphql_variables: Option<String>,
    #[serde(default)]
    pub graphql_operation_name: Option<String>,
    #[serde(default)]
    pub extraction_rules: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExtractionSource {
    JsonPath { path: String },
    Header { name: String },
    Status,
    ResponseBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionRule {
    pub source: ExtractionSource,
    pub target_variable: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedVariable {
    pub variable_name: String,
    pub value: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValueParam {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub workspace_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub id: String,
    pub environment_id: Option<String>,
    pub key: String,
    pub value: String,
    pub is_secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub method: super::HttpMethod,
    pub url: String,
    pub request_headers: String,
    pub request_body: Option<String>,
    pub response_status: u16,
    pub response_headers: String,
    pub response_body: String,
    pub duration_ms: u64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsSession {
    pub id: String,
    pub url: String,
    pub connected_at: String,
    pub disconnected_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub message_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendRequestPayload {
    pub method: super::HttpMethod,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    #[serde(default)]
    pub params: Vec<KeyValueParam>,
    #[serde(default)]
    pub auth: AuthConfig,
    pub environment_id: Option<String>,
    #[serde(default)]
    pub pre_request_script: Option<String>,
    #[serde(default)]
    pub test_script: Option<String>,
    #[serde(default)]
    pub collection_id: Option<String>,
    #[serde(default)]
    pub request_type: RequestType,
    #[serde(default)]
    pub graphql_query: Option<String>,
    #[serde(default)]
    pub graphql_variables: Option<String>,
    #[serde(default)]
    pub graphql_operation_name: Option<String>,
    #[serde(default)]
    pub extraction_rules: Vec<ExtractionRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendRequestResponse {
    pub response: super::ApiResponse,
    pub unresolved_variables: Vec<String>,
    pub history_id: String,
    #[serde(default)]
    pub script_result: Option<ScriptExecutionResult>,
    #[serde(default)]
    pub extracted_variables: Vec<ExtractedVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleEntry {
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptExecutionResult {
    pub pre_request_console: Vec<ConsoleEntry>,
    pub pre_request_error: Option<String>,
    pub test_results: Vec<TestResult>,
    pub test_console: Vec<ConsoleEntry>,
    pub test_error: Option<String>,
}
