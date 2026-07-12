//! HTTP/3 (QUIC) protocol handler.
//!
//! Feature-gated behind `http3`. Requires nightly Rust for `h3-quinn`.
//! Uses the `reqwest::http3` feature flag which pulls in `h3`, `h3-quinn`,
//! and `quinn`.
//!
//! When enabled, this handler emits a `H3` version string in responses
//! and uses reqwest's HTTP/3 transport. When disabled, it falls back to
//! the standard HTTP handler.
//!
//! To enable: `cargo build --features http3`
//! Requires: nightly Rust toolchain, cmake, openssl-dev

#![cfg_attr(not(feature = "http3"), allow(dead_code, unused_imports))]

use crate::error::{Error, Result};
use crate::protocol::{ProtocolCapabilities, ProtocolHandler};
use crate::request::Request;
use async_trait::async_trait;

pub struct Http3Handler;

impl Http3Handler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Http3Handler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolHandler for Http3Handler {
    fn name(&self) -> &str {
        "HTTP/3"
    }

    fn schemes(&self) -> &[&str] {
        &["http", "https", "h3"]
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            can_send: true,
            can_receive: true,
            can_stream: false,
            can_subscribe: false,
        }
    }

    async fn send(&self, request: Request) -> Result<crate::request::Response> {
        #[cfg(feature = "http3")]
        {
            self.send_inner(request).await
        }
        #[cfg(not(feature = "http3"))]
        {
            tracing::warn!("HTTP/3 support not enabled; falling back to HTTP/1.1");
            use crate::protocol::http::HttpHandler;
            HttpHandler::new().send(request).await
        }
    }
}

#[cfg(feature = "http3")]
impl Http3Handler {
    async fn send_inner(&self, request: Request) -> Result<crate::request::Response> {
        // Uses reqwest's native HTTP/3 support.
        let client = reqwest::ClientBuilder::new()
            .http3_prior_knowledge()
            .build()
            .map_err(|e| Error::other(format!("HTTP/3 client: {e}")))?;

        let url = &request.url;
        let method = &request.method;

        let mut req_builder = match method.as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "PATCH" => client.patch(url),
            "DELETE" => client.delete(url),
            "HEAD" => client.head(url),
            _ => client.get(url),
        };

        // Forward auth and custom headers.
        for h in &request.headers {
            if h.enabled {
                req_builder = req_builder.header(&h.key, &h.value);
            }
        }

        // Forward body for mutating methods.
        let body = request.body.content.clone();
        if !matches!(request.method, crate::request::HttpMethod::Get | crate::request::HttpMethod::Head) {
            req_builder = req_builder.body(body);
        }

        let started = std::time::Instant::now();
        let resp = req_builder
            .send()
            .await
            .map_err(|e| Error::Http(e))?;

        let status = resp.status();
        let mut headers = std::collections::HashMap::new();
        for (k, v) in resp.headers() {
            if let Ok(val) = v.to_str() {
                headers.insert(k.to_string(), val.to_string());
            }
        }

        let http_version = format!("{:?}", resp.version());
        let body_bytes = resp.bytes().await.map_err(|e| Error::Http(e))?;
        let elapsed = started.elapsed().as_millis() as u64;
        let is_text = body_bytes.is_empty()
            || String::from_utf8_lossy(&body_bytes)
                .chars()
                .all(|c| c.is_ascii() || c == '\n' || c == '\r' || c == '\t');

        Ok(crate::request::Response {
            status: status.as_u16(),
            status_text: status.canonical_reason().unwrap_or("").to_string(),
            headers,
            body: crate::request::ResponseBody {
                content: body_bytes.to_vec(),
                content_type: headers.get("content-type").cloned(),
                is_text,
            },
            cookies: Vec::new(),
            timing: crate::request::ResponseTiming {
                total_ms: elapsed,
                ..Default::default()
            },
            size: Default::default(),
            url: request.url,
            protocol: http_version,
        })
    }
}
