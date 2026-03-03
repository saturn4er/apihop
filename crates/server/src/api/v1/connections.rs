use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::Deserialize;

use apihop_core::models::{DeploymentMode, ServerConnection};
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/connections", get(list).post(create))
        .route("/connections/{id}", axum::routing::delete(remove))
        .route("/connections/{id}/login", post(connection_login))
        .route("/connections/{id}/refresh", post(connection_refresh))
        .route("/connections/{id}/proxy", post(connection_proxy))
}

fn require_personal_mode(state: &AppState) -> Result<(), AppError> {
    if matches!(state.config().mode, DeploymentMode::Organization) {
        return Err(AppError::NotFound(
            "Connections are only available in personal mode".into(),
        ));
    }
    Ok(())
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<ServerConnection>>, AppError> {
    require_personal_mode(&state)?;
    let connections = state.storage().list_connections().await?;
    Ok(Json(connections))
}

#[derive(Deserialize)]
struct CreateConnectionBody {
    server_url: String,
    display_name: String,
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateConnectionBody>,
) -> Result<(StatusCode, Json<ServerConnection>), AppError> {
    require_personal_mode(&state)?;

    let connection = ServerConnection {
        id: String::new(),
        server_url: body.server_url,
        display_name: body.display_name,
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

    let created = state.storage().create_connection(&connection).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

async fn remove(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    require_personal_mode(&state)?;
    state.storage().delete_connection(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct LoginBody {
    email: String,
    password: String,
}

async fn connection_login(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<LoginBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_personal_mode(&state)?;

    let conn = state.storage().get_connection(&id).await?;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/api/v1/auth/login", conn.server_url))
        .json(&serde_json::json!({ "email": body.email, "password": body.password }))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to connect: {e}")))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("Login failed: {text}")));
    }

    let tokens: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Invalid response: {e}")))?;

    state
        .storage()
        .update_connection_tokens(
            &id,
            tokens["access_token"].as_str(),
            tokens["refresh_token"].as_str(),
        )
        .await?;

    state
        .storage()
        .update_connection_status(&id, "connected")
        .await?;

    Ok(Json(tokens))
}

async fn connection_refresh(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_personal_mode(&state)?;

    let conn = state.storage().get_connection(&id).await?;
    let (_, refresh_token) = state.storage().get_connection_tokens(&id).await?;

    let refresh_token = refresh_token
        .ok_or_else(|| AppError::BadRequest("No refresh token available".into()))?;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/api/v1/auth/refresh", conn.server_url))
        .json(&serde_json::json!({ "refresh_token": refresh_token }))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to connect: {e}")))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("Token refresh failed: {text}")));
    }

    let tokens: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Invalid response: {e}")))?;

    state
        .storage()
        .update_connection_tokens(
            &id,
            tokens["access_token"].as_str(),
            tokens["refresh_token"].as_str(),
        )
        .await?;

    state
        .storage()
        .update_connection_status(&id, "connected")
        .await?;

    Ok(Json(tokens))
}

#[derive(Deserialize)]
struct ProxyBody {
    method: String,
    path: String,
    body: Option<serde_json::Value>,
}

async fn connection_proxy(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ProxyBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_personal_mode(&state)?;

    let (access_token, _) = state.storage().get_connection_tokens(&id).await?;
    let conn = state.storage().get_connection(&id).await?;

    let client = apihop_core::proxy::RemoteClient::new(conn.server_url, access_token);

    let result: serde_json::Value = match body.method.to_uppercase().as_str() {
        "GET" => client
            .get(&body.path)
            .await
            .map_err(|e| AppError::BadGateway(e.to_string()))?,
        "POST" => {
            let payload = body.body.unwrap_or(serde_json::Value::Null);
            client
                .post(&body.path, &payload)
                .await
                .map_err(|e| AppError::BadGateway(e.to_string()))?
        }
        "PUT" => {
            let payload = body.body.unwrap_or(serde_json::Value::Null);
            client
                .put(&body.path, &payload)
                .await
                .map_err(|e| AppError::BadGateway(e.to_string()))?
        }
        "DELETE" => {
            client
                .delete(&body.path)
                .await
                .map_err(|e| AppError::BadGateway(e.to_string()))?;
            serde_json::Value::Null
        }
        _ => return Err(AppError::BadRequest(format!("Unsupported method: {}", body.method))),
    };

    Ok(Json(result))
}
