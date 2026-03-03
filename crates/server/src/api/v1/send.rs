use axum::{Json, Router, extract::State, routing::post};

use apihop_core::models::SendRequestPayload;
use apihop_core::models::SendRequestResponse;
use crate::auth::OptionalAuthUser;
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/send", post(handle_send_request))
}

async fn handle_send_request(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Json(payload): Json<SendRequestPayload>,
) -> Result<Json<SendRequestResponse>, AppError> {
    apihop_core::send_full_request(payload, state.storage(), state.encryption_key(), auth_user.user_id())
        .await
        .map(Json)
        .map_err(AppError::from)
}
