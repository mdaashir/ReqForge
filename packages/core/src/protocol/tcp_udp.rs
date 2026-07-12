//! Raw TCP and UDP socket client.
//!
//! Opens a TcpStream or UdpSocket to the target, sends raw bytes, reads
//! the response, and returns it as a response body. Useful for debugging
//! low-level protocols or when HTTP is not an option.
//!
//! TCP mode: `tcp://host:port`
//! UDP mode: `udp://host:port`

use crate::error::{Error, Result};
use crate::protocol::{ProtocolCapabilities, ProtocolHandler};
use crate::request::Request;
use async_trait::async_trait;

pub struct RawSocketHandler;

impl RawSocketHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RawSocketHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolHandler for RawSocketHandler {
    fn name(&self) -> &str {
        "Raw Socket"
    }

    fn schemes(&self) -> &[&str] {
        &["tcp", "udp"]
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
        let url = &request.url;
        let body = request.body.content.clone().into_bytes();

        // Parse scheme to choose TCP or UDP.
        let is_udp = url.starts_with("udp://");

        // Build a host:port from the URL.
        let host_port = url
            .strip_prefix("tcp://")
            .or_else(|| url.strip_prefix("udp://"))
            .ok_or_else(|| Error::other(format!("unsupported URL scheme: {url}")))?;

        let started = std::time::Instant::now();

        if is_udp {
            let socket = tokio::net::UdpSocket::bind("0.0.0.0:0")
                .await
                .map_err(|e| Error::connection(format!("udp bind: {e}")))?;
            socket
                .connect(host_port)
                .await
                .map_err(|e| Error::connection(format!("udp connect: {e}")))?;
            socket
                .send(&body)
                .await
                .map_err(|e| Error::connection(format!("udp send: {e}")))?;

            let mut buf = vec![0u8; 65535];
            let n = socket
                .recv(&mut buf)
                .await
                .map_err(|e| Error::connection(format!("udp recv: {e}")))?;
            buf.truncate(n);

            let elapsed = started.elapsed().as_millis() as u64;
            let is_text = buf.is_empty() || buf.iter().all(|&b| b.is_ascii() || b == b'\n');
            return Ok(crate::request::Response {
                status: 200,
                status_text: "UDP".into(),
                headers: std::collections::HashMap::new(),
                body: crate::request::ResponseBody {
                    content: buf,
                    content_type: Some("application/octet-stream".into()),
                    is_text,
                },
                cookies: Vec::new(),
                timing: crate::request::ResponseTiming {
                    total_ms: elapsed,
                    ..Default::default()
                },
                size: Default::default(),
                url: request.url,
                protocol: "UDP".to_string(),
            });
        }

        // TCP
        let stream = tokio::net::TcpStream::connect(host_port)
            .await
            .map_err(|e| Error::connection(format!("tcp connect: {e}")))?;

        let (mut reader, mut writer) = stream.into_split();

        // Send body.
        tokio::io::AsyncWriteExt::write_all(&mut writer, &body)
            .await
            .map_err(|e| Error::connection(format!("tcp write: {e}")))?;

        // Read response (up to 64 KiB).
        let mut buf = Vec::with_capacity(65536);
        let mut tmp = [0u8; 4096];
        loop {
            let n = tokio::io::AsyncReadExt::read(&mut reader, &mut tmp)
                .await
                .map_err(|e| Error::connection(format!("tcp read: {e}")))?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
            if buf.len() >= 65536 {
                break;
            }
        }

        let elapsed = started.elapsed().as_millis() as u64;
        let is_text = buf.is_empty() || buf.iter().all(|&b| b.is_ascii() || b == b'\n');

        Ok(crate::request::Response {
            status: 200,
            status_text: "TCP".into(),
            headers: std::collections::HashMap::new(),
            body: crate::request::ResponseBody {
                content: buf,
                content_type: Some("application/octet-stream".into()),
                is_text,
            },
            cookies: Vec::new(),
            timing: crate::request::ResponseTiming {
                total_ms: elapsed,
                ..Default::default()
            },
            size: Default::default(),
            url: request.url,
            protocol: "TCP".to_string(),
        })
    }
}
