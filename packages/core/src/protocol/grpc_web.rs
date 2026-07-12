//! gRPC-Web JSON fallback (used when `grpc-tonic` feature is disabled).
//!
//! Implements unary gRPC-Web calls over HTTP/1.1 by wrapping the HTTP handler.
//! No streaming support.

use crate::error::{Error, Result};
use crate::protocol::{ProtocolCapabilities, ProtocolHandler};
use crate::protocol::http::HttpHandler;
use crate::request::{Body, BodyMode, KeyValue, Request, Response, ResponseBody, ResponseSize, ResponseTiming};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{Instant, SystemTime};

pub struct Backend;

impl Backend {
    pub fn new() -> Self {
        Self
    }

    pub fn name(&self) -> &str {
        "grpc-web"
    }

    pub fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            can_send: true,
            can_receive: true,
            can_stream: false,
            can_subscribe: false,
        }
    }

    /// Invoke a unary gRPC call via gRPC-Web JSON.
    pub async fn invoke_unary(&self, mut request: Request) -> Result<Response> {
        // Identify the method from headers or URL path
        let method_path = request
            .headers
            .iter()
            .find(|h| h.key.eq_ignore_ascii_case("grpc-method"))
            .or_else(|| request.headers.iter().find(|h| h.key.eq_ignore_ascii_case("grpc-service")))
            .map(|h| h.value.clone())
            .unwrap_or_else(|| {
                // Derive from the URL path (strip trailing portion)
                request.url.trim_end_matches('/').to_string()
            });

        // Set gRPC-Web content type
        request.body.content_type = Some("application/grpc-web+json".into());

        let http_handler = HttpHandler::new();
        let response = http_handler.send(request).await?;

        // Parse gRPC-Web envelope out of the body:
        // gRPC-Web frames: 1 byte flag + 4 bytes length + data
        let raw = response.body.content;
        if raw.len() < 5 {
            return Ok(Response {
                status: 0,
                status_text: String::new(),
                headers: response.headers,
                body: ResponseBody::default(),
                cookies: Vec::new(),
                timing: response.timing,
                size: response.size,
                url: response.url,
                protocol: "grpc-web".to_string(),
            });
        }

        let _flag = raw[0]; // 0 = data, 1 = trailers
        let msg_len = u32::from_be_bytes([raw[1], raw[2], raw[3], raw[4]]) as usize;
        let json_body = if msg_len > 0 && 5 + msg_len <= raw.len() {
            &raw[5..5 + msg_len]
        } else {
            &raw[5..]
        };

        Ok(Response {
            status: response.status,
            status_text: response.status_text,
            headers: response.headers,
            body: ResponseBody {
                content: json_body.to_vec(),
                content_type: Some("application/json".into()),
                is_text: true,
            },
            cookies: response.cookies,
            timing: response.timing,
            size: response.size,
            url: response.url,
            protocol: "grpc-web".to_string(),
        })
    }
}

#[async_trait]
impl ProtocolHandler for Backend {
    fn name(&self) -> &str {
        self.name()
    }

    fn schemes(&self) -> &[&str] {
        &["http", "https"]
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        self.capabilities()
    }

    async fn send(&self, request: Request) -> Result<Response> {
        self.invoke_unary(request).await
    }
}
