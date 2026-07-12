//! gRPC handler supporting both:
//!
//! - **Full gRPC** (feature `grpc-tonic`) — `tonic`-based transport over HTTP/2
//!   with unary, server-streaming, client-streaming, bidirectional streaming,
//!   and server reflection support.
//! - **gRPC-Web JSON** (feature off) — gRPC-Web spec over HTTP/1.1, unary only.

use crate::error::{Error, Result};
use crate::request::{Request, Response};
use crate::protocol::{ProtocolCapabilities, ProtocolHandler};
use async_trait::async_trait;

// ── Feature-gated tonic imports ──────────────────────────

#[cfg(feature = "grpc-tonic")]
#[path = "tonic_impl.rs"]
mod tonic_impl;

#[cfg(feature = "grpc-tonic")]
use tonic_impl as backend;

// ── gRPC-Web fallback ────────────────────────────────────

#[cfg(not(feature = "grpc-tonic"))]
#[path = "grpc_web.rs"]
mod grpc_web;
#[cfg(not(feature = "grpc-tonic"))]
use grpc_web as backend;

// ── Public API ───────────────────────────────────────────

/// gRPC method descriptor returned by the proto parser.
#[derive(Debug, Clone)]
pub struct GrpcMethodDescriptor {
    pub service_name: String,
    pub method_name: String,
    pub request_type: String,
    pub response_type: String,
    pub client_streaming: bool,
    pub server_streaming: bool,
}

impl GrpcMethodDescriptor {
    pub fn fully_qualified(&self) -> String {
        format!("{}/{}", self.service_name, self.method_name)
    }
}

/// Result of a gRPC stream operation.
#[derive(Debug, Clone)]
pub struct GrpcStreamMessage {
    pub data: Vec<u8>,
    pub is_text: bool,
}

/// gRPC handler that dispatches to either tonic or gRPC-Web backend.
pub struct GrpcHandler {
    backend: backend::Backend,
}

impl GrpcHandler {
    pub fn new() -> Self {
        Self {
            backend: backend::Backend::new(),
        }
    }

    /// Invoke a unary gRPC call.
    pub async fn invoke_unary(&self, request: Request) -> Result<Response> {
        self.backend.invoke_unary(request).await
    }

    /// Perform a server-streaming gRPC call. Returns a channel of messages.
    #[cfg(feature = "grpc-tonic")]
    pub async fn invoke_server_streaming(
        &self,
        request: Request,
    ) -> Result<tokio::sync::mpsc::Receiver<std::result::Result<GrpcStreamMessage, Error>>> {
        self.backend.invoke_server_streaming(request).await
    }

    /// Perform a bidirectional streaming gRPC call.
    #[cfg(feature = "grpc-tonic")]
    pub async fn invoke_bidi_stream(
        &self,
        request: Request,
    ) -> Result<(
        tokio::sync::mpsc::Sender<Vec<u8>>,
        tokio::sync::mpsc::Receiver<std::result::Result<GrpcStreamMessage, Error>>,
    )> {
        self.backend.invoke_bidi_stream(request).await
    }

    /// Resolve a method via server reflection. Returns the method descriptor.
    #[cfg(feature = "grpc-tonic")]
    pub async fn resolve_method(
        &self,
        target: &str,
        service: &str,
        method: &str,
    ) -> Result<GrpcMethodDescriptor> {
        self.backend.resolve_method(target, service, method).await
    }

    /// Re-parse a .proto file (available in both modes).
    pub fn parse_proto(&self, input: &str) -> Result<crate::protocol::proto::ProtoFile> {
        crate::protocol::proto::parse_proto(input).map_err(Error::other)
    }
}

impl Default for GrpcHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolHandler for GrpcHandler {
    fn name(&self) -> &str {
        self.backend.name()
    }

    fn schemes(&self) -> &[&str] {
        &["http", "https"]
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        self.backend.capabilities()
    }

    async fn send(&self, request: Request) -> Result<Response> {
        self.backend.invoke_unary(request).await
    }
}

/// Parse a `.proto` file (top-level convenience).
pub fn parse_proto_file(input: &str) -> Result<crate::protocol::proto::ProtoFile> {
    crate::protocol::proto::parse_proto(input).map_err(Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_reports_capabilities() {
        let h = GrpcHandler::new();
        #[cfg(feature = "grpc-tonic")]
        assert!(h.name().contains("tonic"));
        #[cfg(not(feature = "grpc-tonic"))]
        assert_eq!(h.name(), "grpc-web");
        let caps = h.capabilities();
        assert!(caps.can_send);
    }

    #[test]
    fn test_send_returns_result() {
        let h = GrpcHandler::new();
        let req = Request::new(crate::request::HttpMethod::Post, "https://example.com/svc");
        let body = crate::request::Body::default();
        let req = Request { body, ..req };
        let rt = tokio::runtime::Runtime::new().unwrap();
        // Should not panic — may error on network or succeed depending on backend
        let _ = rt.block_on(h.send(req));
    }

    #[test]
    fn test_parse_proto() {
        let proto = r#"
            syntax = "proto3";
            package helloworld;
            service Greeter {
                rpc SayHello (HelloRequest) returns (HelloReply);
            }
            message HelloRequest { string name = 1; }
            message HelloReply { string message = 1; }
        "#;
        let f = parse_proto_file(proto).unwrap();
        assert_eq!(f.package, "helloworld");
    }
}
