use axum::{Json, Router, routing::get};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
struct ServerInfo {
    name: String,
    version: String,
    mode: String,
    registration_enabled: bool,
}

async fn get_server_info(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<ServerInfo> {
    let config = state.config();
    Json(ServerInfo {
        name: config.server_name.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        mode: config.mode.to_string(),
        registration_enabled: config.registration_enabled,
    })
}

pub fn router() -> Router<AppState> {
    Router::new().route("/server/info", get(get_server_info))
}
