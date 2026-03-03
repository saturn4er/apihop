use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use apihop_core::auth::{self, AuthError, AuthTokens};
use apihop_core::models::DeploymentMode;

use crate::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
}

// ── Error mapping ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

impl From<AuthError> for AuthApiError {
    fn from(err: AuthError) -> Self {
        AuthApiError(err)
    }
}

struct AuthApiError(AuthError);

impl IntoResponse for AuthApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, self.0.to_string()),
            AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, self.0.to_string()),
            AuthError::InvalidToken(_) => (StatusCode::UNAUTHORIZED, self.0.to_string()),
            AuthError::RegistrationDisabled => (StatusCode::FORBIDDEN, self.0.to_string()),
            AuthError::EmailAlreadyExists => (StatusCode::CONFLICT, self.0.to_string()),
            AuthError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()),
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

impl From<apihop_core::storage::StorageError> for AuthApiError {
    fn from(err: apihop_core::storage::StorageError) -> Self {
        AuthApiError(AuthError::Internal(err.to_string()))
    }
}

// ── Request/Response types ──────────────────────────────────────

#[derive(Deserialize)]
struct RegisterBody {
    email: String,
    password: String,
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct LoginBody {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct RefreshBody {
    refresh_token: String,
}

#[derive(Deserialize)]
struct LogoutBody {
    refresh_token: String,
}

#[derive(Serialize)]
struct MeResponse {
    id: String,
    email: String,
    display_name: Option<String>,
    created_at: String,
}

// ── Handlers ────────────────────────────────────────────────────

async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> Result<(StatusCode, Json<AuthTokens>), AuthApiError> {
    let config = state.config();

    if !matches!(config.mode, DeploymentMode::Organization) {
        return Err(AuthError::Internal("Auth not available in personal mode".into()).into());
    }

    if !config.registration_enabled {
        return Err(AuthError::RegistrationDisabled.into());
    }

    let jwt_secret = config
        .jwt_secret
        .as_deref()
        .ok_or_else(|| AuthError::Internal("JWT secret not configured".into()))?;

    let password_hash = auth::hash_password(&body.password)?;

    let storage = state.storage();
    let user = storage
        .create_user(&body.email, &password_hash, body.display_name.as_deref())
        .await
        .map_err(|e| match e {
            apihop_core::storage::StorageError::Database(ref msg) => {
                let msg_str = msg.to_string();
                if msg_str.contains("UNIQUE") || msg_str.contains("unique") || msg_str.contains("duplicate") {
                    AuthError::EmailAlreadyExists
                } else {
                    AuthError::Internal(e.to_string())
                }
            }
            _ => AuthError::Internal(e.to_string()),
        })?;

    // Auto-create personal workspace
    let ws = storage
        .create_workspace("Personal", None, &user.id, true)
        .await
        .map_err(|e| AuthError::Internal(format!("Failed to create personal workspace: {e}")))?;
    storage
        .add_workspace_member(&ws.id, &user.id, "owner")
        .await
        .map_err(|e| AuthError::Internal(format!("Failed to add workspace member: {e}")))?;

    let access_token =
        auth::create_access_token(&user.id, &user.email, jwt_secret, config.session_duration_secs)?;
    let raw_refresh = auth::generate_refresh_token();
    let refresh_hash = auth::hash_refresh_token(&raw_refresh);
    let expires_at = (chrono::Utc::now()
        + chrono::Duration::seconds(config.refresh_duration_secs as i64))
    .to_rfc3339();
    storage
        .store_refresh_token(&user.id, &refresh_hash, &expires_at)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(AuthTokens {
            access_token,
            refresh_token: raw_refresh,
            expires_in: config.session_duration_secs,
            user,
        }),
    ))
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> Result<Json<AuthTokens>, AuthApiError> {
    let config = state.config();

    if !matches!(config.mode, DeploymentMode::Organization) {
        return Err(AuthError::Internal("Auth not available in personal mode".into()).into());
    }

    let jwt_secret = config
        .jwt_secret
        .as_deref()
        .ok_or_else(|| AuthError::Internal("JWT secret not configured".into()))?;

    let storage = state.storage();
    let user_with_hash = storage
        .get_user_by_email(&body.email)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;

    if !auth::verify_password(&body.password, &user_with_hash.password_hash) {
        return Err(AuthError::InvalidCredentials.into());
    }

    let user = user_with_hash.to_user();

    let access_token =
        auth::create_access_token(&user.id, &user.email, jwt_secret, config.session_duration_secs)?;
    let raw_refresh = auth::generate_refresh_token();
    let refresh_hash = auth::hash_refresh_token(&raw_refresh);
    let expires_at = (chrono::Utc::now()
        + chrono::Duration::seconds(config.refresh_duration_secs as i64))
    .to_rfc3339();
    storage
        .store_refresh_token(&user.id, &refresh_hash, &expires_at)
        .await?;

    Ok(Json(AuthTokens {
        access_token,
        refresh_token: raw_refresh,
        expires_in: config.session_duration_secs,
        user,
    }))
}

async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshBody>,
) -> Result<Json<AuthTokens>, AuthApiError> {
    let config = state.config();
    let jwt_secret = config
        .jwt_secret
        .as_deref()
        .ok_or_else(|| AuthError::Internal("JWT secret not configured".into()))?;

    let storage = state.storage();
    let token_hash = auth::hash_refresh_token(&body.refresh_token);

    let stored_token = storage
        .get_refresh_token(&token_hash)
        .await
        .map_err(|_| AuthError::InvalidToken("Refresh token not found".into()))?;

    // Check expiration
    let expires_at = chrono::DateTime::parse_from_rfc3339(&stored_token.expires_at)
        .map_err(|e| AuthError::Internal(format!("Invalid expiration date: {e}")))?;
    if expires_at < chrono::Utc::now() {
        // Clean up expired token
        let _ = storage.delete_refresh_token(&token_hash).await;
        return Err(AuthError::TokenExpired.into());
    }

    // Delete old refresh token
    storage.delete_refresh_token(&token_hash).await?;

    // Get user
    let user = storage
        .get_user_by_id(&stored_token.user_id)
        .await
        .map_err(|_| AuthError::Internal("User not found".into()))?;

    // Issue new tokens
    let access_token =
        auth::create_access_token(&user.id, &user.email, jwt_secret, config.session_duration_secs)?;
    let raw_refresh = auth::generate_refresh_token();
    let refresh_hash = auth::hash_refresh_token(&raw_refresh);
    let expires_at = (chrono::Utc::now()
        + chrono::Duration::seconds(config.refresh_duration_secs as i64))
    .to_rfc3339();
    storage
        .store_refresh_token(&user.id, &refresh_hash, &expires_at)
        .await?;

    Ok(Json(AuthTokens {
        access_token,
        refresh_token: raw_refresh,
        expires_in: config.session_duration_secs,
        user,
    }))
}

async fn logout(
    State(state): State<AppState>,
    Json(body): Json<LogoutBody>,
) -> Result<StatusCode, AuthApiError> {
    let token_hash = auth::hash_refresh_token(&body.refresh_token);
    let _ = state.storage().delete_refresh_token(&token_hash).await;
    Ok(StatusCode::NO_CONTENT)
}

async fn me(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<MeResponse>, AuthApiError> {
    let user = state
        .storage()
        .get_user_by_id(&auth_user.user_id)
        .await
        .map_err(|_| AuthError::Internal("User not found".into()))?;

    Ok(Json(MeResponse {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        created_at: user.created_at,
    }))
}
