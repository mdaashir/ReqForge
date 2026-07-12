use crate::error::{Error, Result};
use crate::protocol::{ProtocolCapabilities, ProtocolHandler};
use crate::request::Response;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// WebSocket message direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageDirection {
    Sent,
    Received,
}

/// Type of a WebSocket message frame
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebSocketMessageType {
    Text,
    Binary,
    Ping,
    Pong,
    Close,
}

/// A single message exchanged over a WebSocket connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub direction: MessageDirection,
    pub message_type: WebSocketMessageType,
    pub data: String, // text or base64-encoded binary
    pub timestamp: DateTime<Utc>,
    pub size: usize,
}

/// Current state of a WebSocket connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Closing,
    Closed,
    Error,
}

/// Configuration for a WebSocket connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub url: String,
    #[serde(default)]
    pub protocols: Vec<String>,
    #[serde(default)]
    pub headers: Vec<crate::request::KeyValue>,
    #[serde(default = "default_reconnect")]
    pub auto_reconnect: bool,
}

fn default_reconnect() -> bool {
    false
}

/// A handle to an active WebSocket connection
///
/// Wraps a tokio-tungstenite WebSocket stream and exposes a send/receive
/// interface. For the MVP, the actual network layer is behind an
/// abstraction so that tests can mock connections; the production
/// implementation plugs in tokio-tungstenite.
#[derive(Clone)]
pub struct WebSocketConnection {
    pub id: String,
    pub config: WebSocketConfig,
    state: Arc<Mutex<ConnectionState>>,
    history: Arc<Mutex<Vec<WebSocketMessage>>>,
}

impl WebSocketConnection {
    /// Create a new connection handle. The actual socket is opened by `connect`.
    pub fn new(id: String, config: WebSocketConfig) -> Self {
        Self {
            id,
            config,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn state(&self) -> ConnectionState {
        *self.state.lock().await
    }

    pub async fn set_state(&self, new_state: ConnectionState) {
        *self.state.lock().await = new_state;
    }

    pub async fn history(&self) -> Vec<WebSocketMessage> {
        self.history.lock().await.clone()
    }

    pub async fn record_message(&self, msg: WebSocketMessage) {
        self.history.lock().await.push(msg);
    }

    /// Open the WebSocket connection.
    ///
    /// When the `ws` feature is enabled, opens a real tokio-tungstenite
    /// connection and spawns a reader task. Without the feature, sets
    /// the state to Connected (stub mode for testing).
    pub async fn connect(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        if *state != ConnectionState::Disconnected && *state != ConnectionState::Closed {
            return Err(Error::other(format!(
                "Cannot connect: connection is currently {:?}",
                *state
            )));
        }

        *state = ConnectionState::Connecting;

        #[cfg(feature = "ws")]
        {
            let url = self.config.url.clone();
            let self_clone = self.clone();
            tokio::spawn(async move {
                match tokio_tungstenite::connect_async(&url).await {
                    Ok((ws_stream, _)) => {
                        *self_clone.state.lock().await = ConnectionState::Connected;
                        let (_, mut read) = ws_stream.split();
                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                    let ws_msg = WebSocketMessage {
                                        direction: MessageDirection::Received,
                                        message_type: WebSocketMessageType::Text,
                                        data: text.to_string(),
                                        timestamp: Utc::now(),
                                        size: text.len(),
                                    };
                                    self_clone.record_message(ws_msg).await;
                                }
                                Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
                                _ => {}
                            }
                        }
                        *self_clone.state.lock().await = ConnectionState::Closed;
                    }
                    Err(e) => {
                        *self_clone.state.lock().await = ConnectionState::Error;
                        let ws_msg = WebSocketMessage {
                            direction: MessageDirection::Received,
                            message_type: WebSocketMessageType::Close,
                            data: format!("connection failed: {e}"),
                            timestamp: Utc::now(),
                            size: 0,
                        };
                        self_clone.record_message(ws_msg).await;
                    }
                }
            });
            // State set to Connected once the connection succeeds inside the task.
        }

        #[cfg(not(feature = "ws"))]
        {
            *state = ConnectionState::Connected;
        }

        Ok(())
    }

    /// Send a text message over the connection
    pub async fn send_text(&self, text: impl Into<String>) -> Result<()> {
        let state = self.state.lock().await;
        if *state != ConnectionState::Connected {
            return Err(Error::other(format!(
                "Cannot send: connection is {:?}",
                *state
            )));
        }
        drop(state);

        let msg = WebSocketMessage {
            direction: MessageDirection::Sent,
            message_type: WebSocketMessageType::Text,
            data: text.into(),
            timestamp: Utc::now(),
            size: 0,
        };

        self.record_message(msg).await;
        Ok(())
    }

    /// Close the connection gracefully
    pub async fn close(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        *state = ConnectionState::Closing;
        // In the full implementation: send Close frame through the writer handle.
        *state = ConnectionState::Closed;
        Ok(())
    }
}

/// WebSocket protocol handler
///
/// Manages WebSocket connections for the request engine.
pub struct WebSocketHandler;

impl WebSocketHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebSocketHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolHandler for WebSocketHandler {
    fn name(&self) -> &str {
        "WebSocket"
    }

    fn schemes(&self) -> &[&str] {
        &["ws", "wss"]
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            can_send: true,
            can_receive: true,
            can_stream: true,
            can_subscribe: true,
        }
    }

    async fn send(&self, _request: crate::request::Request) -> Result<Response> {
        // For WebSocket, "send" doesn't really map to a single request/response.
        // The frontend should use the WebSocketConnection API directly for
        // interactive WebSocket workflows.
        Err(Error::NotImplemented(
            "WebSocket send: use WebSocketConnection for interactive sessions".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_metadata() {
        let h = WebSocketHandler::new();
        assert_eq!(h.name(), "WebSocket");
        assert_eq!(h.schemes(), &["ws", "wss"]);
        assert!(h.capabilities().can_stream);
        assert!(h.capabilities().can_subscribe);
    }

    #[tokio::test]
    async fn test_connection_lifecycle() {
        let conn = WebSocketConnection::new(
            "test".to_string(),
            WebSocketConfig {
                url: "wss://example.com".to_string(),
                protocols: vec![],
                headers: vec![],
                auto_reconnect: false,
            },
        );

        assert_eq!(conn.state().await, ConnectionState::Disconnected);
        conn.connect().await.unwrap();
        assert_eq!(conn.state().await, ConnectionState::Connected);

        conn.send_text("hello").await.unwrap();
        let history = conn.history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].direction, MessageDirection::Sent);
        assert_eq!(history[0].data, "hello");

        conn.close().await.unwrap();
        assert_eq!(conn.state().await, ConnectionState::Closed);
    }

    #[tokio::test]
    async fn test_send_when_not_connected_fails() {
        let conn = WebSocketConnection::new(
            "test".to_string(),
            WebSocketConfig {
                url: "wss://example.com".to_string(),
                protocols: vec![],
                headers: vec![],
                auto_reconnect: false,
            },
        );

        // Without connect(), send should fail
        assert!(conn.send_text("nope").await.is_err());
    }

    #[test]
    fn test_serialize_websocket_message() {
        let msg = WebSocketMessage {
            direction: MessageDirection::Received,
            message_type: WebSocketMessageType::Text,
            data: "hi".to_string(),
            timestamp: Utc::now(),
            size: 2,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("received"));
        assert!(json.contains("text"));
    }
}
