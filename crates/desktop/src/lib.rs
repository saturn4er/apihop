use std::collections::HashMap;
use std::sync::Arc;

use rand::Fill as _;
use serde::Serialize;
use tauri::{Emitter, Manager};

use apihop_core::graphql::GraphQLSchema;
use apihop_core::models::*;
use apihop_core::storage::sqlite::SqliteBackend;
use apihop_core::storage::{StorageBackend, StorageError};
use apihop_core::websocket::{WebSocketManager, WsMessage, WsStatus};
use apihop_core::{ApiRequest, ApiResponse, send_request};

struct AppStorage(Arc<dyn StorageBackend>);
struct AppEncryptionKey([u8; 32]);
struct WsManager(Arc<WebSocketManager>);

// ── Encryption key management ────────────────────────────────────────

fn get_or_create_key_from_keychain() -> Result<[u8; 32], String> {
    let entry = keyring::Entry::new("apihop", "encryption_key")
        .map_err(|e| format!("keyring entry error: {e}"))?;

    match entry.get_password() {
        Ok(hex_str) => {
            let bytes = hex::decode(&hex_str)
                .map_err(|e| format!("invalid hex in keychain: {e}"))?;
            if bytes.len() != 32 {
                return Err(format!("keychain key has wrong length: {}", bytes.len()));
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            Ok(key)
        }
        Err(keyring::Error::NoEntry) => {
            let mut key = [0u8; 32];
            key.fill(&mut rand::rng());
            entry
                .set_password(&hex::encode(key))
                .map_err(|e| format!("failed to store key in keychain: {e}"))?;
            Ok(key)
        }
        Err(e) => Err(format!("keychain read error: {e}")),
    }
}

fn get_or_create_key_from_file(data_dir: &std::path::Path) -> [u8; 32] {
    let key_path = data_dir.join("secret.key");
    if key_path.exists() {
        let bytes = std::fs::read(&key_path).expect("Failed to read secret.key");
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        key
    } else {
        let mut key = [0u8; 32];
        key.fill(&mut rand::rng());
        std::fs::write(&key_path, &key).expect("Failed to write secret.key");
        key
    }
}

// ── Error type for Tauri commands ──────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum CommandError {
    NotFound(String),
    Storage(String),
    Network(String),
    Validation(String),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::NotFound(msg) => write!(f, "Not found: {msg}"),
            CommandError::Storage(msg) => write!(f, "Storage error: {msg}"),
            CommandError::Network(msg) => write!(f, "Network error: {msg}"),
            CommandError::Validation(msg) => write!(f, "Validation error: {msg}"),
        }
    }
}

impl From<StorageError> for CommandError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::NotFound(msg) => CommandError::NotFound(msg),
            StorageError::Database(source) => CommandError::Storage(source.to_string()),
        }
    }
}

impl From<apihop_core::ApiError> for CommandError {
    fn from(e: apihop_core::ApiError) -> Self {
        match e {
            apihop_core::ApiError::Request(re) => CommandError::Network(re.to_string()),
            apihop_core::ApiError::Storage(se) => se.into(),
            apihop_core::ApiError::Auth(msg) => CommandError::Network(msg),
        }
    }
}

impl From<String> for CommandError {
    fn from(s: String) -> Self {
        CommandError::Validation(s)
    }
}

#[tauri::command]
async fn send_api_request(request: ApiRequest) -> Result<ApiResponse, CommandError> {
    send_request(request).await.map_err(|e| CommandError::Network(e.to_string()))
}

#[tauri::command]
async fn send_full_request(
    storage: tauri::State<'_, AppStorage>,
    key: tauri::State<'_, AppEncryptionKey>,
    payload: SendRequestPayload,
) -> Result<SendRequestResponse, CommandError> {
    Ok(apihop_core::send_full_request(payload, &*storage.0, &key.0, None).await?)
}

// --- Collections ---

#[tauri::command]
async fn list_collections(
    storage: tauri::State<'_, AppStorage>,
) -> Result<Vec<Collection>, CommandError> {
    Ok(storage.0.list_collections(None).await?)
}

#[tauri::command]
async fn get_collection(
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<Collection, CommandError> {
    Ok(storage.0.get_collection(&id, None).await?)
}

#[tauri::command]
async fn create_collection(
    storage: tauri::State<'_, AppStorage>,
    name: String,
    description: Option<String>,
    auth: Option<AuthConfig>,
    pre_request_script: Option<String>,
    test_script: Option<String>,
    workspace_id: Option<String>,
) -> Result<Collection, CommandError> {
    Ok(storage
        .0
        .create_collection(
            &name,
            description.as_deref(),
            auth.as_ref(),
            pre_request_script.as_deref(),
            test_script.as_deref(),
            None,
            workspace_id.as_deref(),
        )
        .await?)
}

#[tauri::command]
async fn update_collection(
    storage: tauri::State<'_, AppStorage>,
    id: String,
    name: String,
    description: Option<String>,
    auth: Option<AuthConfig>,
    pre_request_script: Option<String>,
    test_script: Option<String>,
) -> Result<Collection, CommandError> {
    Ok(storage
        .0
        .update_collection(
            &id,
            &name,
            description.as_deref(),
            auth.as_ref(),
            pre_request_script.as_deref(),
            test_script.as_deref(),
            None,
        )
        .await?)
}

#[tauri::command]
async fn delete_collection(
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<(), CommandError> {
    Ok(storage.0.delete_collection(&id, None).await?)
}

// --- Folders ---

#[tauri::command]
async fn list_folders(
    storage: tauri::State<'_, AppStorage>,
    collection_id: String,
    parent_folder_id: Option<String>,
) -> Result<Vec<Folder>, CommandError> {
    Ok(storage
        .0
        .list_folders(&collection_id, parent_folder_id.as_deref(), None)
        .await?)
}

#[tauri::command]
async fn create_folder(
    storage: tauri::State<'_, AppStorage>,
    collection_id: String,
    parent_folder_id: Option<String>,
    name: String,
) -> Result<Folder, CommandError> {
    Ok(storage
        .0
        .create_folder(&collection_id, parent_folder_id.as_deref(), &name, None)
        .await?)
}

#[tauri::command]
async fn update_folder(
    storage: tauri::State<'_, AppStorage>,
    id: String,
    name: String,
) -> Result<Folder, CommandError> {
    Ok(storage.0.update_folder(&id, &name, None).await?)
}

#[tauri::command]
async fn delete_folder(
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<(), CommandError> {
    Ok(storage.0.delete_folder(&id, None).await?)
}

// --- Requests ---

#[tauri::command]
async fn list_requests(
    storage: tauri::State<'_, AppStorage>,
    collection_id: String,
    folder_id: Option<String>,
) -> Result<Vec<SavedRequest>, CommandError> {
    Ok(storage
        .0
        .list_requests(&collection_id, folder_id.as_deref(), None)
        .await?)
}

#[tauri::command]
async fn get_request(
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<SavedRequest, CommandError> {
    Ok(storage.0.get_request(&id, None).await?)
}

#[tauri::command]
async fn create_request(
    storage: tauri::State<'_, AppStorage>,
    req: SavedRequest,
) -> Result<SavedRequest, CommandError> {
    Ok(storage.0.create_request(&req, None).await?)
}

#[tauri::command]
async fn update_request(
    storage: tauri::State<'_, AppStorage>,
    req: SavedRequest,
) -> Result<SavedRequest, CommandError> {
    Ok(storage.0.update_request(&req, None).await?)
}

#[tauri::command]
async fn delete_request(
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<(), CommandError> {
    Ok(storage.0.delete_request(&id, None).await?)
}

// --- History ---

#[tauri::command]
async fn list_history(
    storage: tauri::State<'_, AppStorage>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<HistoryEntry>, CommandError> {
    Ok(storage
        .0
        .list_history(limit.unwrap_or(50), offset.unwrap_or(0), None)
        .await?)
}

#[tauri::command]
async fn create_history_entry(
    storage: tauri::State<'_, AppStorage>,
    entry: HistoryEntry,
) -> Result<HistoryEntry, CommandError> {
    Ok(storage.0.create_history_entry(&entry, None).await?)
}

#[tauri::command]
async fn delete_history_entry(
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<(), CommandError> {
    Ok(storage.0.delete_history_entry(&id, None).await?)
}

// --- Environments ---

#[tauri::command]
async fn list_environments(
    storage: tauri::State<'_, AppStorage>,
) -> Result<Vec<Environment>, CommandError> {
    Ok(storage.0.list_environments(None).await?)
}

#[tauri::command]
async fn get_environment(
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<Environment, CommandError> {
    Ok(storage.0.get_environment(&id, None).await?)
}

#[tauri::command]
async fn create_environment(
    storage: tauri::State<'_, AppStorage>,
    name: String,
    workspace_id: Option<String>,
) -> Result<Environment, CommandError> {
    Ok(storage.0.create_environment(&name, None, workspace_id.as_deref()).await?)
}

#[tauri::command]
async fn update_environment(
    storage: tauri::State<'_, AppStorage>,
    id: String,
    name: String,
) -> Result<Environment, CommandError> {
    Ok(storage.0.update_environment(&id, &name, None).await?)
}

#[tauri::command]
async fn delete_environment(
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<(), CommandError> {
    Ok(storage.0.delete_environment(&id, None).await?)
}

// --- Variables ---

#[tauri::command]
async fn list_variables(
    storage: tauri::State<'_, AppStorage>,
    environment_id: Option<String>,
) -> Result<Vec<Variable>, CommandError> {
    Ok(storage
        .0
        .list_variables(environment_id.as_deref(), None)
        .await?)
}

#[tauri::command]
async fn set_variable(
    storage: tauri::State<'_, AppStorage>,
    variable: Variable,
) -> Result<Variable, CommandError> {
    Ok(storage.0.set_variable(&variable, None).await?)
}

#[tauri::command]
async fn delete_variable(
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<(), CommandError> {
    Ok(storage.0.delete_variable(&id, None).await?)
}

#[tauri::command]
async fn clear_history(
    storage: tauri::State<'_, AppStorage>,
) -> Result<(), CommandError> {
    Ok(storage.0.clear_history(None).await?)
}

// --- Import / Export ---

#[tauri::command]
async fn import_postman_collection(
    storage: tauri::State<'_, AppStorage>,
    data: String,
) -> Result<Collection, CommandError> {
    let result = apihop_core::import_export::import_postman(&data)?;
    Ok(apihop_core::import_export::persist_import(result, &*storage.0, None).await?)
}

#[tauri::command]
async fn import_openapi_spec(
    storage: tauri::State<'_, AppStorage>,
    data: String,
) -> Result<Collection, CommandError> {
    let result = apihop_core::import_export::import_openapi(&data)?;
    Ok(apihop_core::import_export::persist_import(result, &*storage.0, None).await?)
}

#[tauri::command]
async fn import_curl_command(data: String) -> Result<apihop_core::import_export::CurlImportResponse, CommandError> {
    let req = apihop_core::import_export::import_curl(&data)?;
    Ok(req.into())
}

#[tauri::command]
async fn export_collection_apihop(
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<serde_json::Value, CommandError> {
    let (collection, folders, root_requests) =
        apihop_core::load_collection_tree(&*storage.0, &id, None).await?;
    Ok(apihop_core::import_export::export_apihop(
        &collection,
        &folders,
        &root_requests,
    ))
}

#[tauri::command]
async fn export_collection_postman(
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<serde_json::Value, CommandError> {
    let (collection, folders, root_requests) =
        apihop_core::load_collection_tree(&*storage.0, &id, None).await?;
    Ok(apihop_core::import_export::export_postman(
        &collection,
        &folders,
        &root_requests,
    ))
}

#[tauri::command]
async fn export_request_curl(
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<String, CommandError> {
    let req = storage.0.get_request(&id, None).await?;
    Ok(apihop_core::import_export::export_curl(&req))
}

// --- WebSocket ---

#[derive(Debug, Clone, Serialize)]
pub struct WsConnectResult {
    pub connection_id: String,
    pub unresolved_variables: Vec<String>,
}

#[tauri::command]
async fn ws_connect(
    ws: tauri::State<'_, WsManager>,
    storage: tauri::State<'_, AppStorage>,
    url: String,
    headers: HashMap<String, String>,
    auth: Option<AuthConfig>,
    environment_id: Option<String>,
) -> Result<WsConnectResult, CommandError> {
    // 1. Load variables
    let var_map = apihop_core::load_variables(&*storage.0, environment_id.as_deref(), None)
        .await?;

    // 2. Interpolate URL and headers
    let mut all_unresolved = Vec::new();
    let (resolved_url, u) = apihop_core::interpolate(&url, &var_map);
    all_unresolved.extend(u);

    let mut resolved_headers = HashMap::new();
    for (k, v) in &headers {
        let (ik, u1) = apihop_core::interpolate(k, &var_map);
        all_unresolved.extend(u1);
        let (iv, u2) = apihop_core::interpolate(v, &var_map);
        all_unresolved.extend(u2);
        resolved_headers.insert(ik, iv);
    }

    // 3. Apply auth
    let auth_config = auth.unwrap_or_default();
    let (resolved_auth, auth_unresolved) = apihop_core::interpolate_auth(&auth_config, &var_map);
    all_unresolved.extend(auth_unresolved);
    let _ = apihop_core::apply_auth(&resolved_auth, &mut resolved_headers)
        .await
        .map_err(|e| CommandError::Network(e.to_string()))?;

    all_unresolved.sort();
    all_unresolved.dedup();

    // 4. Connect
    let connection_id = ws.0.connect(&resolved_url, resolved_headers)
        .await
        .map_err(|e| CommandError::Network(e.to_string()))?;

    Ok(WsConnectResult { connection_id, unresolved_variables: all_unresolved })
}

#[tauri::command]
async fn ws_send(
    ws: tauri::State<'_, WsManager>,
    storage: tauri::State<'_, AppStorage>,
    id: String,
    payload: String,
    is_binary: bool,
    environment_id: Option<String>,
) -> Result<WsMessage, CommandError> {
    // Interpolate text payloads with variables
    let resolved_payload = if !is_binary {
        let var_map = apihop_core::load_variables(&*storage.0, environment_id.as_deref(), None)
            .await?;
        let (interpolated, _) = apihop_core::interpolate(&payload, &var_map);
        interpolated
    } else {
        payload
    };

    ws.0.send(&id, &resolved_payload, is_binary)
        .await
        .map_err(|e| CommandError::Network(e.to_string()))
}

#[tauri::command]
async fn ws_disconnect(
    ws: tauri::State<'_, WsManager>,
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<(), CommandError> {
    let summary = ws.0.disconnect(&id).await.map_err(|e| CommandError::Network(e.to_string()))?;

    let now = chrono::Utc::now();
    let connected_at = now - chrono::Duration::milliseconds(summary.duration_ms as i64);
    let session = WsSession {
        id: uuid::Uuid::new_v4().to_string(),
        url: summary.url,
        connected_at: connected_at.to_rfc3339(),
        disconnected_at: Some(now.to_rfc3339()),
        duration_ms: Some(summary.duration_ms),
        message_count: summary.message_count,
    };

    let _ = storage.0.create_ws_session(&session).await;
    Ok(())
}

#[tauri::command]
async fn ws_status(
    ws: tauri::State<'_, WsManager>,
    id: String,
) -> Result<WsStatus, CommandError> {
    ws.0.status(&id).await.map_err(|e| CommandError::Network(e.to_string()))
}

#[tauri::command]
async fn ws_subscribe(
    app: tauri::AppHandle,
    ws: tauri::State<'_, WsManager>,
    id: String,
) -> Result<(), CommandError> {
    let sender = ws.0.get_sender(&id).await.map_err(|e| CommandError::Network(e.to_string()))?;
    let mut rx = sender.subscribe();
    let event_name = format!("ws-message-{id}");

    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let _ = app.emit(&event_name, &msg);
        }
    });

    Ok(())
}

#[tauri::command]
async fn list_ws_sessions(
    storage: tauri::State<'_, AppStorage>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<WsSession>, CommandError> {
    Ok(storage
        .0
        .list_ws_sessions(limit.unwrap_or(50), offset.unwrap_or(0))
        .await?)
}

#[tauri::command]
async fn delete_ws_session(
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<(), CommandError> {
    Ok(storage.0.delete_ws_session(&id).await?)
}

// --- Connections ---

#[tauri::command]
async fn list_connections(
    storage: tauri::State<'_, AppStorage>,
) -> Result<Vec<ServerConnection>, CommandError> {
    Ok(storage.0.list_connections().await?)
}

#[tauri::command]
async fn add_connection(
    storage: tauri::State<'_, AppStorage>,
    server_url: String,
    display_name: String,
) -> Result<ServerConnection, CommandError> {
    let connection = ServerConnection {
        id: String::new(),
        server_url,
        display_name,
        access_token: None,
        refresh_token: None,
        user_email: None,
        user_display_name: None,
        user_server_id: None,
        server_mode: String::new(),
        status: "disconnected".to_string(),
        created_at: String::new(),
        last_used_at: None,
    };
    Ok(storage.0.create_connection(&connection).await?)
}

#[tauri::command]
async fn remove_connection(
    storage: tauri::State<'_, AppStorage>,
    id: String,
) -> Result<(), CommandError> {
    Ok(storage.0.delete_connection(&id).await?)
}

#[tauri::command]
async fn test_connection(
    server_url: String,
) -> Result<serde_json::Value, CommandError> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/v1/server/info", server_url))
        .send()
        .await
        .map_err(|e| CommandError::Network(format!("Failed to connect: {e}")))?;
    if !resp.status().is_success() {
        return Err(CommandError::Network("Server returned error".into()));
    }
    let info: serde_json::Value = resp.json().await
        .map_err(|e| CommandError::Network(format!("Invalid response: {e}")))?;
    Ok(info)
}

#[tauri::command]
async fn connection_login(
    storage: tauri::State<'_, AppStorage>,
    id: String,
    email: String,
    password: String,
) -> Result<serde_json::Value, CommandError> {
    let conn = storage.0.get_connection(&id).await?;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/api/v1/auth/login", conn.server_url))
        .json(&serde_json::json!({ "email": email, "password": password }))
        .send()
        .await
        .map_err(|e| CommandError::Network(format!("Failed to connect: {e}")))?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(CommandError::Network(format!("Login failed: {text}")));
    }
    let tokens: serde_json::Value = resp.json().await
        .map_err(|e| CommandError::Network(format!("Invalid response: {e}")))?;
    storage.0.update_connection_tokens(
        &id,
        tokens["access_token"].as_str(),
        tokens["refresh_token"].as_str(),
    ).await?;
    storage.0.update_connection_status(&id, "connected").await?;
    Ok(tokens)
}

// --- Connection Proxy ---

#[tauri::command]
async fn connection_proxy(
    storage: tauri::State<'_, AppStorage>,
    connection_id: String,
    method: String,
    path: String,
    body: Option<serde_json::Value>,
) -> Result<serde_json::Value, CommandError> {
    let conn = storage.0.get_connection(&connection_id).await?;
    let (access_token, _) = storage.0.get_connection_tokens(&connection_id).await?;

    let client = apihop_core::proxy::RemoteClient::new(
        conn.server_url,
        access_token,
    );

    let result: serde_json::Value = match method.to_uppercase().as_str() {
        "GET" => client.get(&path).await
            .map_err(|e| CommandError::Network(e.to_string()))?,
        "POST" => {
            let payload = body.unwrap_or(serde_json::Value::Null);
            client.post(&path, &payload).await
                .map_err(|e| CommandError::Network(e.to_string()))?
        }
        "PUT" => {
            let payload = body.unwrap_or(serde_json::Value::Null);
            client.put(&path, &payload).await
                .map_err(|e| CommandError::Network(e.to_string()))?
        }
        "DELETE" => {
            client.delete(&path).await
                .map_err(|e| CommandError::Network(e.to_string()))?;
            serde_json::Value::Null
        }
        _ => return Err(CommandError::Validation(format!("Unsupported method: {method}"))),
    };

    Ok(result)
}

// --- GraphQL ---

#[tauri::command]
async fn graphql_introspect(
    storage: tauri::State<'_, AppStorage>,
    url: String,
    headers: HashMap<String, String>,
    auth: Option<AuthConfig>,
    environment_id: Option<String>,
) -> Result<GraphQLSchema, CommandError> {
    let var_map = apihop_core::load_variables(&*storage.0, environment_id.as_deref(), None).await?;

    let (resolved_url, _) = apihop_core::interpolate(&url, &var_map);

    let mut resolved_headers = HashMap::new();
    for (k, v) in &headers {
        let (ik, _) = apihop_core::interpolate(k, &var_map);
        let (iv, _) = apihop_core::interpolate(v, &var_map);
        resolved_headers.insert(ik, iv);
    }

    if let Some(ref auth_config) = auth {
        let (resolved_auth, _) = apihop_core::interpolate_auth(auth_config, &var_map);
        let _ = apihop_core::apply_auth(&resolved_auth, &mut resolved_headers)
            .await
            .map_err(|e| CommandError::Network(e.to_string()))?;
    }

    let schema = apihop_core::graphql::introspect(&resolved_url, &resolved_headers)
        .await
        .map_err(|e| CommandError::Network(e.to_string()))?;
    Ok(schema)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "apihop_desktop=info,apihop_core=info".into()),
        )
        .init();

    tauri::Builder::default()
        .setup(|app| {
            #[cfg(debug_assertions)]
            if let Some(window) = app.get_webview_window("main") {
                window.open_devtools();
            }

            let data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            std::fs::create_dir_all(&data_dir).expect("Failed to create data dir");
            let db_path = data_dir.join("apihop.db");

            let encryption_key = match get_or_create_key_from_keychain() {
                Ok(key) => key,
                Err(e) => {
                    tracing::warn!("Keychain unavailable ({e}), falling back to file-based key");
                    get_or_create_key_from_file(&data_dir)
                }
            };

            let storage = tauri::async_runtime::block_on(async {
                SqliteBackend::new(db_path.to_str().unwrap(), encryption_key)
                    .await
                    .expect("Failed to open SQLite database")
            });

            app.manage(AppStorage(Arc::new(storage)));
            app.manage(AppEncryptionKey(encryption_key));
            app.manage(WsManager(Arc::new(WebSocketManager::new())));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            send_api_request,
            send_full_request,
            list_collections,
            get_collection,
            create_collection,
            update_collection,
            delete_collection,
            list_folders,
            create_folder,
            update_folder,
            delete_folder,
            list_requests,
            get_request,
            create_request,
            update_request,
            delete_request,
            list_history,
            create_history_entry,
            delete_history_entry,
            clear_history,
            list_environments,
            get_environment,
            create_environment,
            update_environment,
            delete_environment,
            list_variables,
            set_variable,
            delete_variable,
            import_postman_collection,
            import_openapi_spec,
            import_curl_command,
            export_collection_apihop,
            export_collection_postman,
            export_request_curl,
            ws_connect,
            ws_send,
            ws_disconnect,
            ws_status,
            ws_subscribe,
            list_ws_sessions,
            delete_ws_session,
            graphql_introspect,
            list_connections,
            add_connection,
            remove_connection,
            test_connection,
            connection_login,
            connection_proxy,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
