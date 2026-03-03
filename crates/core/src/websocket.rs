use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock, broadcast};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Error)]
pub enum WsError {
    #[error("Connection not found: {0}")]
    NotFound(String),
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Send failed: {0}")]
    SendFailed(String),
    #[error("Already disconnected")]
    AlreadyDisconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WsStatus {
    Connecting,
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WsDirection {
    Sent,
    Received,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    pub id: String,
    pub direction: WsDirection,
    pub payload: String,
    pub is_binary: bool,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsSessionSummary {
    pub url: String,
    pub duration_ms: u64,
    pub message_count: u64,
}

type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;

struct WsConn {
    status: Arc<RwLock<WsStatus>>,
    url: String,
    connected_at_ms: u64,
    message_count: Arc<AtomicU64>,
    tx: broadcast::Sender<WsMessage>,
    sink: Arc<Mutex<WsSink>>,
    cancel_token: CancellationToken,
    recv_handle: JoinHandle<()>,
}

pub struct WebSocketManager {
    connections: Arc<RwLock<HashMap<String, WsConn>>>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn connect(
        &self,
        url: &str,
        headers: HashMap<String, String>,
    ) -> Result<String, WsError> {
        let id = uuid::Uuid::new_v4().to_string();

        // Build WebSocket request with proper handshake headers, then add custom headers
        let mut request = url
            .into_client_request()
            .map_err(|e| WsError::ConnectionFailed(e.to_string()))?;
        for (k, v) in &headers {
            request.headers_mut().insert(
                http::header::HeaderName::from_bytes(k.as_bytes())
                    .map_err(|e| WsError::ConnectionFailed(e.to_string()))?,
                http::header::HeaderValue::from_str(v)
                    .map_err(|e| WsError::ConnectionFailed(e.to_string()))?,
            );
        }

        let (ws_stream, _) = tokio_tungstenite::connect_async(request)
            .await
            .map_err(|e| WsError::ConnectionFailed(e.to_string()))?;

        let (sink, mut stream) = ws_stream.split();

        let (tx, _) = broadcast::channel::<WsMessage>(256);
        let message_count = Arc::new(AtomicU64::new(0));
        let status = Arc::new(RwLock::new(WsStatus::Connected));
        let cancel_token = CancellationToken::new();

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Spawn receive loop with cancellation support
        let recv_tx = tx.clone();
        let recv_count = message_count.clone();
        let recv_status = status.clone();
        let recv_token = cancel_token.clone();
        let conn_id = id.clone();
        let connections = self.connections.clone();

        let recv_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = recv_token.cancelled() => break,
                    msg_opt = stream.next() => {
                        match msg_opt {
                            Some(Ok(msg)) => {
                                let (payload, is_binary) = match &msg {
                                    Message::Text(t) => (t.to_string(), false),
                                    Message::Binary(b) => {
                                        (base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b), true)
                                    }
                                    Message::Close(_) => break,
                                    Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
                                };

                                recv_count.fetch_add(1, Ordering::Relaxed);
                                let ws_msg = WsMessage {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    direction: WsDirection::Received,
                                    payload,
                                    is_binary,
                                    timestamp_ms: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as u64,
                                };
                                let _ = recv_tx.send(ws_msg);
                            }
                            Some(Err(_)) | None => break,
                        }
                    }
                }
            }
            // Mark as disconnected
            *recv_status.write().await = WsStatus::Disconnected;
            // Send a final disconnect notification
            let _ = recv_tx.send(WsMessage {
                id: uuid::Uuid::new_v4().to_string(),
                direction: WsDirection::Received,
                payload: "__ws_disconnected__".to_string(),
                is_binary: false,
                timestamp_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            });
            // Remove from connections map
            connections.write().await.remove(&conn_id);
        });

        let conn = WsConn {
            status: status.clone(),
            url: url.to_string(),
            connected_at_ms: now_ms,
            message_count: message_count.clone(),
            tx: tx.clone(),
            sink: Arc::new(Mutex::new(sink)),
            cancel_token,
            recv_handle,
        };

        self.connections.write().await.insert(id.clone(), conn);

        Ok(id)
    }

    pub async fn send(
        &self,
        id: &str,
        payload: &str,
        is_binary: bool,
    ) -> Result<WsMessage, WsError> {
        let connections = self.connections.read().await;
        let conn = connections
            .get(id)
            .ok_or_else(|| WsError::NotFound(id.to_string()))?;

        let status = conn.status.read().await;
        if *status == WsStatus::Disconnected {
            return Err(WsError::AlreadyDisconnected);
        }
        drop(status);

        let msg = if is_binary {
            let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, payload)
                .map_err(|e| WsError::SendFailed(e.to_string()))?;
            Message::Binary(bytes.into())
        } else {
            Message::Text(payload.to_string().into())
        };

        conn.sink
            .lock()
            .await
            .send(msg)
            .await
            .map_err(|e| WsError::SendFailed(e.to_string()))?;

        conn.message_count.fetch_add(1, Ordering::Relaxed);

        let ws_msg = WsMessage {
            id: uuid::Uuid::new_v4().to_string(),
            direction: WsDirection::Sent,
            payload: payload.to_string(),
            is_binary,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };

        Ok(ws_msg)
    }

    pub async fn disconnect(&self, id: &str) -> Result<WsSessionSummary, WsError> {
        let mut connections = self.connections.write().await;
        let conn = connections
            .remove(id)
            .ok_or_else(|| WsError::NotFound(id.to_string()))?;

        // Cancel the receive loop first, then close the sink
        conn.cancel_token.cancel();
        let _ = conn.sink.lock().await.close().await;

        // Wait for the receive loop to finish
        let _ = conn.recv_handle.await;

        *conn.status.write().await = WsStatus::Disconnected;

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(WsSessionSummary {
            url: conn.url,
            duration_ms: now_ms.saturating_sub(conn.connected_at_ms),
            message_count: conn.message_count.load(Ordering::Relaxed),
        })
    }

    pub async fn get_sender(
        &self,
        id: &str,
    ) -> Result<broadcast::Sender<WsMessage>, WsError> {
        let connections = self.connections.read().await;
        let conn = connections
            .get(id)
            .ok_or_else(|| WsError::NotFound(id.to_string()))?;
        Ok(conn.tx.clone())
    }

    pub async fn status(&self, id: &str) -> Result<WsStatus, WsError> {
        let connections = self.connections.read().await;
        let conn = connections
            .get(id)
            .ok_or_else(|| WsError::NotFound(id.to_string()))?;
        Ok(conn.status.read().await.clone())
    }
}
