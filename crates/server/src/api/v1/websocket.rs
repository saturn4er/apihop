use std::collections::HashMap;
use std::convert::Infallible;
use std::time::Duration;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::sse::{Event, Sse},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use apihop_core::models::WsSession;
use apihop_core::websocket::{WsMessage, WsStatus};
use crate::auth::OptionalAuthUser;
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ws/connect", post(connect))
        .route("/ws/{id}/send", post(send))
        .route("/ws/{id}/disconnect", post(disconnect))
        .route("/ws/{id}/status", get(status))
        .route("/ws/{id}/messages", get(messages))
        .route("/ws/sessions", get(list_sessions))
        .route("/ws/sessions/{id}", axum::routing::delete(delete_session))
}

#[derive(Deserialize)]
struct ConnectRequest {
    url: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    auth: apihop_core::models::AuthConfig,
    environment_id: Option<String>,
}

#[derive(Serialize)]
struct ConnectResponse {
    connection_id: String,
    unresolved_variables: Vec<String>,
}

async fn connect(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Json(req): Json<ConnectRequest>,
) -> Result<Json<ConnectResponse>, AppError> {
    // 1. Load variables (globals + environment-specific)
    let var_map = apihop_core::load_variables(state.storage(), req.environment_id.as_deref(), auth_user.user_id()).await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 2. Interpolate URL and headers
    let mut all_unresolved = Vec::new();
    let (url, u) = apihop_core::interpolate(&req.url, &var_map);
    all_unresolved.extend(u);

    let mut headers = HashMap::new();
    for (k, v) in &req.headers {
        let (ik, u1) = apihop_core::interpolate(k, &var_map);
        all_unresolved.extend(u1);
        let (iv, u2) = apihop_core::interpolate(v, &var_map);
        all_unresolved.extend(u2);
        headers.insert(ik, iv);
    }

    // 3. Interpolate and apply auth
    let (resolved_auth, auth_unresolved) = apihop_core::interpolate_auth(&req.auth, &var_map);
    all_unresolved.extend(auth_unresolved);
    let _ = apihop_core::apply_auth(&resolved_auth, &mut headers).await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    all_unresolved.sort();
    all_unresolved.dedup();

    // 4. Connect
    let connection_id = state
        .ws_manager()
        .connect(&url, headers)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(ConnectResponse { connection_id, unresolved_variables: all_unresolved }))
}

#[derive(Deserialize)]
struct SendRequest {
    payload: String,
    #[serde(default)]
    is_binary: bool,
    environment_id: Option<String>,
}

async fn send(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<String>,
    Json(req): Json<SendRequest>,
) -> Result<Json<WsMessage>, AppError> {
    // Interpolate message payload with variables
    let payload = if !req.is_binary {
        let var_map = apihop_core::load_variables(state.storage(), req.environment_id.as_deref(), auth_user.user_id()).await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let (interpolated, _) = apihop_core::interpolate(&req.payload, &var_map);
        interpolated
    } else {
        req.payload
    };

    let msg = state
        .ws_manager()
        .send(&id, &payload, req.is_binary)
        .await
        .map_err(|e| match e {
            apihop_core::websocket::WsError::NotFound(_) => AppError::NotFound(e.to_string()),
            _ => AppError::BadRequest(e.to_string()),
        })?;

    Ok(Json(msg))
}

async fn disconnect(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let summary = state
        .ws_manager()
        .disconnect(&id)
        .await
        .map_err(|e| match e {
            apihop_core::websocket::WsError::NotFound(_) => AppError::NotFound(e.to_string()),
            _ => AppError::Internal(e.to_string()),
        })?;

    // Persist session to storage
    let now = chrono::Utc::now().to_rfc3339();
    let connected_at = chrono::Utc::now()
        - chrono::Duration::milliseconds(summary.duration_ms as i64);
    let session = WsSession {
        id: uuid::Uuid::new_v4().to_string(),
        url: summary.url,
        connected_at: connected_at.to_rfc3339(),
        disconnected_at: Some(now),
        duration_ms: Some(summary.duration_ms),
        message_count: summary.message_count,
    };

    if let Err(e) = state.storage().create_ws_session(&session).await {
        eprintln!("Warning: failed to persist WS session: {e}");
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<WsStatus>, AppError> {
    let status = state
        .ws_manager()
        .status(&id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;

    Ok(Json(status))
}

async fn messages(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, AppError> {
    let sender = state
        .ws_manager()
        .get_sender(&id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;

    let rx = sender.subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|result| match result {
            Ok(msg) => {
                let json = serde_json::to_string(&msg).unwrap_or_default();
                Some(Ok(Event::default().data(json)))
            }
            Err(_) => None,
        });

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    ))
}

#[derive(Deserialize)]
struct SessionsQuery {
    #[serde(default = "default_limit")]
    limit: u32,
    #[serde(default)]
    offset: u32,
}

fn default_limit() -> u32 {
    50
}

async fn list_sessions(
    State(state): State<AppState>,
    Query(query): Query<SessionsQuery>,
) -> Result<Json<Vec<WsSession>>, AppError> {
    let sessions = state
        .storage()
        .list_ws_sessions(query.limit, query.offset)
        .await?;
    Ok(Json(sessions))
}

async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.storage().delete_ws_session(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
