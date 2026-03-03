mod v1;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let api_v1 = v1::router();

    let cors = if cfg!(debug_assertions) {
        CorsLayer::permissive()
    } else {
        // In release mode, only allow same-origin requests (no CORS headers needed
        // since the server serves the frontend from the same origin).
        CorsLayer::new()
    };

    Router::new()
        .nest("/api/v1", api_v1)
        .fallback_service(ServeDir::new("ui/dist"))
        .layer(cors)
        .with_state(state)
}
