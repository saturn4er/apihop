use std::sync::Arc;

use apihop_core::storage::StorageBackend;
use apihop_core::websocket::WebSocketManager;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    storage: Arc<dyn StorageBackend>,
    encryption_key: [u8; 32],
    ws_manager: Arc<WebSocketManager>,
    config: Arc<Config>,
}

impl AppState {
    pub fn new(
        storage: Arc<dyn StorageBackend>,
        encryption_key: [u8; 32],
        ws_manager: Arc<WebSocketManager>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            storage,
            encryption_key,
            ws_manager,
            config,
        }
    }

    pub fn storage(&self) -> &dyn StorageBackend {
        &*self.storage
    }

    pub fn encryption_key(&self) -> &[u8; 32] {
        &self.encryption_key
    }

    pub fn ws_manager(&self) -> &Arc<WebSocketManager> {
        &self.ws_manager
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
