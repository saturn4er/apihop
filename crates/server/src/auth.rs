use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub email: String,
}

impl AuthUser {
    pub fn user_id(&self) -> Option<&str> {
        Some(&self.user_id)
    }
}

#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<AuthUser>);

impl OptionalAuthUser {
    pub fn user_id(&self) -> Option<&str> {
        self.0.as_ref().map(|u| u.user_id.as_str())
    }
}

#[derive(Debug, Serialize)]
struct AuthErrorResponse {
    error: String,
}

impl IntoResponse for AuthErrorResponse {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, axum::Json(self)).into_response()
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jwt_secret = state.config().jwt_secret.as_deref().ok_or_else(|| {
            (StatusCode::INTERNAL_SERVER_ERROR, "Auth not configured").into_response()
        })?;

        let token = extract_bearer_token(parts).ok_or_else(|| {
            AuthErrorResponse {
                error: "Missing authorization token".into(),
            }
            .into_response()
        })?;

        let claims = apihop_core::auth::validate_access_token(&token, jwt_secret).map_err(|e| {
            AuthErrorResponse {
                error: e.to_string(),
            }
            .into_response()
        })?;

        Ok(AuthUser {
            user_id: claims.sub,
            email: claims.email,
        })
    }
}

impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        use apihop_core::models::DeploymentMode;

        match state.config().mode {
            DeploymentMode::Personal => Ok(OptionalAuthUser(None)),
            DeploymentMode::Organization => {
                let user = AuthUser::from_request_parts(parts, state).await?;
                Ok(OptionalAuthUser(Some(user)))
            }
        }
    }
}

fn extract_bearer_token(parts: &Parts) -> Option<String> {
    let auth_header = parts.headers.get("authorization")?.to_str().ok()?;
    let token = auth_header
        .strip_prefix("Bearer ")
        .or_else(|| auth_header.strip_prefix("bearer "))?;
    Some(token.to_string())
}
