//! Protocol handlers
//!
//! This module defines the protocol abstraction and provides implementations
//! for different protocols (HTTP, GraphQL, WebSocket, gRPC, etc.).

pub mod graphql;
pub mod grpc;
pub mod http;
pub mod http3;
#[cfg(feature = "kafka")]
pub mod kafka;
#[cfg(feature = "mqtt")]
pub mod mqtt;
pub mod proto;
pub mod soap;
pub mod sse;
pub mod tcp_udp;
pub mod websocket;

use crate::error::Result;
use crate::request::{Request, Response};
use async_trait::async_trait;

/// Capabilities a protocol handler can have
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolCapabilities {
    pub can_send: bool,
    pub can_receive: bool,
    pub can_stream: bool,
    pub can_subscribe: bool,
}

impl Default for ProtocolCapabilities {
    fn default() -> Self {
        Self {
            can_send: true,
            can_receive: true,
            can_stream: false,
            can_subscribe: false,
        }
    }
}

/// Protocol handler trait
///
/// Implemented by all protocol handlers (HTTP, GraphQL, WebSocket, etc.)
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    /// Name of the protocol
    fn name(&self) -> &str;

    /// URL schemes this handler supports (e.g., "http", "https", "ws", "wss")
    fn schemes(&self) -> &[&str];

    /// Capabilities of this protocol handler
    fn capabilities(&self) -> ProtocolCapabilities;

    /// Send a request and return the response
    async fn send(&self, request: Request) -> Result<Response>;

    /// Send a streaming request
    async fn stream(&self, _request: Request) -> Result<()> {
        Err(crate::Error::NotImplemented(format!(
            "Streaming not supported by {}",
            self.name()
        )))
    }
}
