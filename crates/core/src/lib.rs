pub mod auth;
pub mod crypto;
pub mod extraction;
pub mod graphql;
pub mod import_export;
pub mod models;
pub mod pipeline;
pub mod proxy;
pub mod scripting;
pub mod storage;
pub mod websocket;

use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::AtomicU32;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Storage error: {0}")]
    Storage(#[from] storage::StorageError),
    #[error("Auth error: {0}")]
    Auth(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for HttpMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::Get),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            "PATCH" => Ok(HttpMethod::Patch),
            "DELETE" => Ok(HttpMethod::Delete),
            "HEAD" => Ok(HttpMethod::Head),
            "OPTIONS" => Ok(HttpMethod::Options),
            _ => Err(format!("Unknown HTTP method: {s}")),
        }
    }
}

/// Shared reqwest client with sensible timeouts.
static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("failed to build HTTP client")
});

/// Counter for history pruning throttle (runs every 100th request).
static PRUNE_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub duration_ms: u64,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
}

/// Resolve a dynamic variable whose name starts with `$`.
/// Returns `Some(value)` if the name is a known dynamic variable, `None` otherwise.
pub fn resolve_dynamic_variable(name: &str) -> Option<String> {
    match name {
        "$timestamp" => Some(chrono::Utc::now().timestamp().to_string()),
        "$isoTimestamp" => Some(chrono::Utc::now().to_rfc3339()),
        "$randomUUID" => Some(uuid::Uuid::new_v4().to_string()),
        "$randomInt" => {
            use rand::Rng;
            Some(rand::rng().random_range(0..10000).to_string())
        }
        "$randomEmail" => {
            use rand::Rng;
            let n: u32 = rand::rng().random_range(1000..99999);
            Some(format!("user{n}@example.com"))
        }
        "$randomName" => {
            const NAMES: &[&str] = &[
                "Alice", "Bob", "Charlie", "Diana", "Eve", "Frank", "Grace",
                "Henry", "Iris", "Jack", "Karen", "Leo", "Mia", "Noah",
                "Olivia", "Paul", "Quinn", "Rose", "Sam", "Tina",
            ];
            use rand::Rng;
            let idx = rand::rng().random_range(0..NAMES.len());
            Some(NAMES[idx].to_string())
        }
        _ => None,
    }
}

/// Return a list of (name, description) pairs for all supported dynamic variables.
/// Used by the frontend for autocomplete.
pub fn dynamic_variable_list() -> Vec<(&'static str, &'static str)> {
    vec![
        ("$timestamp", "Current Unix timestamp in seconds"),
        ("$isoTimestamp", "Current ISO 8601 timestamp"),
        ("$randomUUID", "Random UUID v4"),
        ("$randomInt", "Random integer between 0 and 9999"),
        ("$randomEmail", "Random email address"),
        ("$randomName", "Random first name"),
    ]
}

/// Interpolate `{{variable}}` placeholders in a string using the given variable map.
/// Variables starting with `$` are resolved as dynamic variables first.
/// Returns the interpolated string and a list of unresolved variable names.
pub fn interpolate(
    input: &str,
    variables: &HashMap<String, String>,
) -> (String, Vec<String>) {
    let mut result = String::with_capacity(input.len());
    let mut unresolved = Vec::new();
    let mut rest = input;

    while let Some(start) = rest.find("{{") {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find("}}") {
            let key = after_open[..end].trim();
            if key.starts_with('$') {
                if let Some(value) = resolve_dynamic_variable(key) {
                    result.push_str(&value);
                } else if let Some(value) = variables.get(key) {
                    result.push_str(value);
                } else {
                    unresolved.push(key.to_string());
                    result.push_str(&rest[start..start + 2 + end + 2]);
                }
            } else if let Some(value) = variables.get(key) {
                result.push_str(value);
            } else {
                unresolved.push(key.to_string());
                result.push_str(&rest[start..start + 2 + end + 2]);
            }
            rest = &after_open[end + 2..];
        } else {
            result.push_str(&rest[start..]);
            rest = "";
            break;
        }
    }
    result.push_str(rest);

    (result, unresolved)
}

/// Apply variable interpolation to an entire ApiRequest.
/// Returns the interpolated request and all unresolved variable names.
pub fn interpolate_request(
    request: &ApiRequest,
    variables: &HashMap<String, String>,
) -> (ApiRequest, Vec<String>) {
    let mut all_unresolved = Vec::new();

    let (url, u) = interpolate(&request.url, variables);
    all_unresolved.extend(u);

    let mut headers = HashMap::new();
    for (k, v) in &request.headers {
        let (ik, u) = interpolate(k, variables);
        all_unresolved.extend(u);
        let (iv, u) = interpolate(v, variables);
        all_unresolved.extend(u);
        headers.insert(ik, iv);
    }

    let body = request.body.as_ref().map(|b| {
        let (ib, u) = interpolate(b, variables);
        all_unresolved.extend(u);
        ib
    });

    let interpolated = ApiRequest {
        method: request.method.clone(),
        url,
        headers,
        body,
    };

    all_unresolved.sort();
    all_unresolved.dedup();
    (interpolated, all_unresolved)
}

/// Interpolate all {{variable}} placeholders in an AuthConfig.
pub fn interpolate_auth(
    auth: &models::AuthConfig,
    variables: &HashMap<String, String>,
) -> (models::AuthConfig, Vec<String>) {
    let mut unresolved = Vec::new();
    let mut interp = |s: &str| -> String {
        let (result, u) = interpolate(s, variables);
        unresolved.extend(u);
        result
    };
    let new_auth = match auth {
        models::AuthConfig::None => models::AuthConfig::None,
        models::AuthConfig::Basic { username, password } => models::AuthConfig::Basic {
            username: interp(username),
            password: interp(password),
        },
        models::AuthConfig::Bearer { token } => models::AuthConfig::Bearer {
            token: interp(token),
        },
        models::AuthConfig::ApiKey { key, value, add_to } => models::AuthConfig::ApiKey {
            key: interp(key),
            value: interp(value),
            add_to: add_to.clone(),
        },
        models::AuthConfig::OAuth2ClientCredentials {
            token_url,
            client_id,
            client_secret,
            scope,
        } => models::AuthConfig::OAuth2ClientCredentials {
            token_url: interp(token_url),
            client_id: interp(client_id),
            client_secret: interp(client_secret),
            scope: scope.as_ref().map(|s| interp(s)),
        },
    };
    unresolved.sort();
    unresolved.dedup();
    (new_auth, unresolved)
}

/// Apply auth config to request headers/URL. Returns updated headers and optional query params to append.
pub async fn apply_auth(
    auth: &models::AuthConfig,
    headers: &mut HashMap<String, String>,
) -> Result<Vec<(String, String)>, ApiError> {
    use base64::Engine;
    let mut extra_params = Vec::new();
    match auth {
        models::AuthConfig::None => {}
        models::AuthConfig::Basic { username, password } => {
            let credentials = base64::engine::general_purpose::STANDARD
                .encode(format!("{}:{}", username, password));
            headers.insert("Authorization".into(), format!("Basic {}", credentials));
        }
        models::AuthConfig::Bearer { token } => {
            headers.insert("Authorization".into(), format!("Bearer {}", token));
        }
        models::AuthConfig::ApiKey { key, value, add_to } => match add_to {
            models::ApiKeyLocation::Header => {
                headers.insert(key.clone(), value.clone());
            }
            models::ApiKeyLocation::QueryParam => {
                extra_params.push((key.clone(), value.clone()));
            }
        },
        models::AuthConfig::OAuth2ClientCredentials {
            token_url,
            client_id,
            client_secret,
            scope,
        } => {
            let token = fetch_oauth2_token(token_url, client_id, client_secret, scope.as_deref())
                .await?;
            headers.insert("Authorization".into(), format!("Bearer {}", token));
        }
    }
    Ok(extra_params)
}

async fn fetch_oauth2_token(
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    scope: Option<&str>,
) -> Result<String, ApiError> {
    let client = &*HTTP_CLIENT;
    let mut form = vec![
        ("grant_type", "client_credentials"),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ];
    let scope_owned;
    if let Some(s) = scope {
        scope_owned = s.to_string();
        form.push(("scope", &scope_owned));
    }
    let resp = client.post(token_url).form(&form).send().await?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ApiError::Auth(format!("OAuth2 token request failed: {}", body)));
    }
    let json: serde_json::Value = resp.json().await?;
    json["access_token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| ApiError::Auth("OAuth2 response missing access_token".into()))
}

pub fn mask_auth_header(value: &str) -> String {
    if value.len() <= 8 {
        return "****".into();
    }
    let visible = &value[..4];
    format!("{}****", visible)
}

/// Server-side orchestrator: resolves variables, applies auth, sends request, records history.
pub async fn send_full_request(
    payload: models::SendRequestPayload,
    storage: &dyn storage::StorageBackend,
    encryption_key: &[u8; 32],
    user_id: Option<&str>,
) -> Result<models::SendRequestResponse, ApiError> {
    let mut pipeline = pipeline::RequestPipeline::new(storage, encryption_key);
    if let Some(uid) = user_id {
        pipeline = pipeline.with_user_id(Some(uid.to_string()));
    }
    pipeline.execute(payload).await
}

pub async fn send_request(request: ApiRequest) -> Result<ApiResponse, ApiError> {
    let client = &*HTTP_CLIENT;

    let mut builder = match request.method {
        HttpMethod::Get => client.get(&request.url),
        HttpMethod::Post => client.post(&request.url),
        HttpMethod::Put => client.put(&request.url),
        HttpMethod::Patch => client.patch(&request.url),
        HttpMethod::Delete => client.delete(&request.url),
        HttpMethod::Head => client.head(&request.url),
        HttpMethod::Options => client.request(reqwest::Method::OPTIONS, &request.url),
    };

    for (key, value) in &request.headers {
        builder = builder.header(key, value);
    }

    if let Some(body) = request.body {
        builder = builder.body(body);
    }

    let start = std::time::Instant::now();
    let response = builder.send().await?;
    let duration_ms = start.elapsed().as_millis() as u64;

    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let body = response.text().await?;
    let size_bytes = body.len() as u64;

    Ok(ApiResponse {
        status,
        headers,
        body,
        duration_ms,
        content_type,
        size_bytes: Some(size_bytes),
    })
}

/// Load variables (globals + environment-specific) into a HashMap.
pub async fn load_variables(
    storage: &dyn storage::StorageBackend,
    environment_id: Option<&str>,
    user_id: Option<&str>,
) -> Result<HashMap<String, String>, storage::StorageError> {
    let mut var_map = HashMap::new();
    let globals = storage.list_variables(None, user_id).await?;
    for v in globals {
        var_map.insert(v.key, v.value);
    }
    if let Some(env_id) = environment_id {
        let env_vars = storage.list_variables(Some(env_id), user_id).await?;
        for v in env_vars {
            var_map.insert(v.key, v.value);
        }
    }
    Ok(var_map)
}

/// A folder with its requests and sub-folders, structured for export.
pub type FolderTree = (models::Folder, Vec<models::SavedRequest>, Vec<(models::Folder, Vec<models::SavedRequest>)>);

/// Load a collection with its full folder/request tree.
pub async fn load_collection_tree(
    storage: &dyn storage::StorageBackend,
    collection_id: &str,
    user_id: Option<&str>,
) -> Result<(models::Collection, Vec<FolderTree>, Vec<models::SavedRequest>), storage::StorageError> {
    let collection = storage.get_collection(collection_id, user_id).await?;
    let root_requests = storage.list_requests(collection_id, None, user_id).await?;
    let top_folders = storage.list_folders(collection_id, None, user_id).await?;

    let mut folders = Vec::new();
    for folder in top_folders {
        let folder_requests = storage
            .list_requests(collection_id, Some(&folder.id), user_id)
            .await?;
        let sub_folders = storage
            .list_folders(collection_id, Some(&folder.id), user_id)
            .await?;
        let mut sub_folder_entries = Vec::new();
        for sf in sub_folders {
            let sf_requests = storage
                .list_requests(collection_id, Some(&sf.id), user_id)
                .await?;
            sub_folder_entries.push((sf, sf_requests));
        }
        folders.push((folder, folder_requests, sub_folder_entries));
    }

    Ok((collection, folders, root_requests))
}

/// Startup migration: ensure all existing users have a personal workspace.
pub async fn ensure_personal_workspaces(storage: &dyn storage::StorageBackend) -> Result<(), storage::StorageError> {
    let users = storage.list_all_users().await?;
    for user in &users {
        match storage.get_personal_workspace(&user.id).await {
            Ok(_) => {} // already has one
            Err(storage::StorageError::NotFound(_)) => {
                let ws = storage
                    .create_workspace("Personal", None, &user.id, true)
                    .await?;
                storage
                    .add_workspace_member(&ws.id, &user.id, "owner")
                    .await?;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_basic() {
        let mut vars = HashMap::new();
        vars.insert("host".to_string(), "example.com".to_string());
        vars.insert("port".to_string(), "8080".to_string());

        let (result, unresolved) = interpolate("https://{{host}}:{{port}}/api", &vars);
        assert_eq!(result, "https://example.com:8080/api");
        assert!(unresolved.is_empty());
    }

    #[test]
    fn test_interpolate_unresolved() {
        let vars = HashMap::new();
        let (result, unresolved) = interpolate("{{host}}/{{path}}", &vars);
        assert_eq!(result, "{{host}}/{{path}}");
        assert_eq!(unresolved, vec!["host", "path"]);
    }

    #[test]
    fn test_interpolate_partial() {
        let mut vars = HashMap::new();
        vars.insert("host".to_string(), "example.com".to_string());

        let (result, unresolved) = interpolate("https://{{host}}/{{path}}", &vars);
        assert_eq!(result, "https://example.com/{{path}}");
        assert_eq!(unresolved, vec!["path"]);
    }

    #[test]
    fn test_interpolate_no_placeholders() {
        let vars = HashMap::new();
        let (result, unresolved) = interpolate("plain text", &vars);
        assert_eq!(result, "plain text");
        assert!(unresolved.is_empty());
    }

    #[test]
    fn test_interpolate_unclosed() {
        let vars = HashMap::new();
        let (result, unresolved) = interpolate("text {{unclosed", &vars);
        assert_eq!(result, "text {{unclosed");
        assert!(unresolved.is_empty());
    }

    #[test]
    fn test_http_method_display_fromstr_roundtrip() {
        let methods = vec![
            HttpMethod::Get,
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Patch,
            HttpMethod::Delete,
            HttpMethod::Head,
            HttpMethod::Options,
        ];
        for method in methods {
            let s = method.to_string();
            let parsed: HttpMethod = s.parse().unwrap();
            assert_eq!(parsed.to_string(), method.to_string());
        }
    }

    #[test]
    fn test_http_method_case_insensitive() {
        assert_eq!("get".parse::<HttpMethod>().unwrap().to_string(), "GET");
        assert_eq!("Post".parse::<HttpMethod>().unwrap().to_string(), "POST");
        assert!("INVALID".parse::<HttpMethod>().is_err());
    }

    #[test]
    fn test_encryption_roundtrip() {
        let key = [42u8; 32];
        let plaintext = "secret data";
        let encrypted = crypto::encrypt(plaintext, &key).unwrap();
        let decrypted = crypto::decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encryption_empty_string() {
        let key = [42u8; 32];
        let encrypted = crypto::encrypt("", &key).unwrap();
        let decrypted = crypto::decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, "");
    }

    #[test]
    fn test_mask_auth_header() {
        assert_eq!(mask_auth_header("short"), "****");
        assert_eq!(mask_auth_header("Bearer abc123def456"), "Bear****");
    }

    #[test]
    fn test_request_type_display_fromstr() {
        use models::RequestType;
        assert_eq!(RequestType::Http.to_string(), "http");
        assert_eq!(RequestType::Websocket.to_string(), "websocket");
        assert_eq!(RequestType::Graphql.to_string(), "graphql");
        assert_eq!("http".parse::<RequestType>().unwrap().to_string(), "http");
        assert_eq!("websocket".parse::<RequestType>().unwrap().to_string(), "websocket");
        assert_eq!("ws".parse::<RequestType>().unwrap().to_string(), "websocket");
        assert_eq!("graphql".parse::<RequestType>().unwrap().to_string(), "graphql");
        assert_eq!("gql".parse::<RequestType>().unwrap().to_string(), "graphql");
        assert!("invalid".parse::<RequestType>().is_err());
    }

    #[test]
    fn test_dynamic_variable_timestamp() {
        let val = resolve_dynamic_variable("$timestamp").unwrap();
        let ts: i64 = val.parse().expect("should be a valid integer");
        assert!(ts > 1_700_000_000); // after 2023
    }

    #[test]
    fn test_dynamic_variable_iso_timestamp() {
        let val = resolve_dynamic_variable("$isoTimestamp").unwrap();
        assert!(val.contains('T'));
    }

    #[test]
    fn test_dynamic_variable_uuid() {
        let val = resolve_dynamic_variable("$randomUUID").unwrap();
        assert_eq!(val.len(), 36); // UUID v4 format
        assert!(val.contains('-'));
    }

    #[test]
    fn test_dynamic_variable_random_int() {
        let val = resolve_dynamic_variable("$randomInt").unwrap();
        let n: i32 = val.parse().expect("should be a valid integer");
        assert!((0..10000).contains(&n));
    }

    #[test]
    fn test_dynamic_variable_random_email() {
        let val = resolve_dynamic_variable("$randomEmail").unwrap();
        assert!(val.contains('@'));
        assert!(val.ends_with("@example.com"));
    }

    #[test]
    fn test_dynamic_variable_random_name() {
        let val = resolve_dynamic_variable("$randomName").unwrap();
        assert!(!val.is_empty());
    }

    #[test]
    fn test_dynamic_variable_unknown() {
        assert!(resolve_dynamic_variable("$unknown").is_none());
        assert!(resolve_dynamic_variable("notDynamic").is_none());
    }

    #[test]
    fn test_dynamic_variable_list() {
        let list = dynamic_variable_list();
        assert!(!list.is_empty());
        assert!(list.iter().any(|(name, _)| *name == "$timestamp"));
        assert!(list.iter().any(|(name, _)| *name == "$randomUUID"));
    }

    #[test]
    fn test_interpolate_dynamic_variable() {
        let vars = HashMap::new();
        let (result, unresolved) = interpolate("ts={{$timestamp}}", &vars);
        assert!(!result.contains("{{"));
        assert!(unresolved.is_empty());
    }

    #[test]
    fn test_interpolate_dynamic_variable_not_marked_unresolved() {
        let vars = HashMap::new();
        let (_, unresolved) = interpolate("{{$randomUUID}}", &vars);
        assert!(unresolved.is_empty());
    }

    #[test]
    fn test_interpolate_mixed_dynamic_and_user_vars() {
        let mut vars = HashMap::new();
        vars.insert("host".to_string(), "example.com".to_string());
        let (result, unresolved) = interpolate("https://{{host}}/{{$randomUUID}}", &vars);
        assert!(result.starts_with("https://example.com/"));
        assert!(!result.contains("{{"));
        assert!(unresolved.is_empty());
    }
}
