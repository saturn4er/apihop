use std::collections::HashMap;

use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;

use apihop_core::graphql::GraphQLSchema;
use crate::auth::OptionalAuthUser;
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/graphql/introspect", post(introspect))
}

#[derive(Deserialize)]
struct IntrospectRequest {
    url: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    auth: Option<apihop_core::models::AuthConfig>,
    environment_id: Option<String>,
}

async fn introspect(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Json(payload): Json<IntrospectRequest>,
) -> Result<Json<GraphQLSchema>, AppError> {
    let var_map = apihop_core::load_variables(
        state.storage(),
        payload.environment_id.as_deref(),
        auth_user.user_id(),
    )
    .await?;

    // Interpolate URL
    let (resolved_url, _) = apihop_core::interpolate(&payload.url, &var_map);

    // Interpolate headers
    let mut resolved_headers = HashMap::new();
    for (k, v) in &payload.headers {
        let (ik, _) = apihop_core::interpolate(k, &var_map);
        let (iv, _) = apihop_core::interpolate(v, &var_map);
        resolved_headers.insert(ik, iv);
    }

    // Apply auth
    if let Some(ref auth_config) = payload.auth {
        let (resolved_auth, _) = apihop_core::interpolate_auth(auth_config, &var_map);
        let _ = apihop_core::apply_auth(&resolved_auth, &mut resolved_headers).await?;
    }

    let schema = apihop_core::graphql::introspect(&resolved_url, &resolved_headers).await?;
    Ok(Json(schema))
}
