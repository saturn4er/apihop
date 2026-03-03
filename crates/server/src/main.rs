mod api;
mod auth;
mod config;
mod error;
mod state;

use std::sync::Arc;

use apihop_core::storage::StorageBackend;
use apihop_core::websocket::WebSocketManager;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "apihop_server=info,apihop_core=info".into()),
        )
        .init();

    let config = config::Config::from_env();

    let storage: Arc<dyn StorageBackend> =
        if config.database_url.starts_with("postgres://")
            || config.database_url.starts_with("postgresql://")
        {
            let backend = apihop_core::storage::postgres::PostgresBackend::new(
                &config.database_url,
                config.encryption_key,
            )
            .await
            .expect("Failed to connect to PostgreSQL");
            Arc::new(backend)
        } else {
            let backend = apihop_core::storage::sqlite::SqliteBackend::new(
                &config.database_url,
                config.encryption_key,
            )
            .await
            .expect("Failed to open SQLite database");
            Arc::new(backend)
        };

    // Cleanup orphaned WS sessions from previous runs
    if let Err(e) = storage.cleanup_orphaned_ws_sessions().await {
        tracing::warn!("Failed to cleanup orphaned WS sessions: {e}");
    }

    // Ensure all existing users have personal workspaces
    if let Err(e) = apihop_core::ensure_personal_workspaces(&*storage).await {
        tracing::warn!("Failed to ensure personal workspaces: {e}");
    }

    let ws_manager = Arc::new(WebSocketManager::new());
    let config = Arc::new(config);
    let state = state::AppState::new(storage, config.encryption_key, ws_manager, config.clone());
    let app = api::router(state);

    tracing::info!(
        "apihop server listening on http://{}:{}",
        config.host,
        config.port
    );
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.host, config.port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
