//! Server-Sent Events (SSE) client.
//!
//! Streams `text/event-stream` responses line-by-line using reqwest's
//! streaming body API. Each event is emitted to a channel the caller
//! can consume asynchronously.

use crate::error::{Error, Result};
use crate::protocol::{ProtocolCapabilities, ProtocolHandler};
use crate::request::Request;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    pub id: Option<String>,
    pub event: Option<String>,
    pub data: Vec<String>,
    pub retry: Option<u64>,
}

pub struct SseHandler;

impl SseHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SseHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolHandler for SseHandler {
    fn name(&self) -> &str {
        "SSE"
    }

    fn schemes(&self) -> &[&str] {
        &["http", "https", "sse"]
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            can_send: false,
            can_receive: true,
            can_stream: true,
            can_subscribe: true,
        }
    }

    async fn send(&self, request: Request) -> Result<crate::request::Response> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<SseEvent>(1024);

        // Spawn a background task that reads the SSE stream.
        let client = Client::new();
        let url = request.url.clone();
        tokio::spawn(async move {
            let response = match client.get(&url).send().await {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx
                        .send(SseEvent {
                            id: None,
                            event: Some("error".into()),
                            data: vec![format!("connection failed: {e}")],
                            retry: None,
                        })
                        .await;
                    return;
                }
            };

            let mut stream = response.bytes_stream();
            let mut buf = String::new();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        buf.push_str(&String::from_utf8_lossy(&bytes));
                        // Process complete lines (SSE fields are newline-separated).
                        while let Some(newline) = buf.find('\n') {
                            let line = buf[..newline].to_string();
                            buf = buf[newline + 1..].to_string();
                            if line.starts_with("data: ") {
                                let data = line[6..].to_string();
                                let _ = tx
                                    .send(SseEvent {
                                        id: None,
                                        event: None,
                                        data: vec![data],
                                        retry: None,
                                    })
                                    .await;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(SseEvent {
                                id: None,
                                event: Some("error".into()),
                                data: vec![format!("stream error: {e}")],
                                retry: None,
                            })
                            .await;
                        break;
                    }
                }
            }
            drop(tx);
        });

        // Read all events from the channel and render into a response body.
        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            events.push(event);
        }

        let json = serde_json::to_string_pretty(&events)?;
        Ok(crate::request::Response {
            status: 200,
            status_text: "OK".into(),
            headers: std::collections::HashMap::from([(
                "content-type".into(),
                "application/json".into(),
            )]),
            body: crate::request::ResponseBody {
                content: json.into_bytes(),
                content_type: Some("application/json".into()),
                is_text: true,
            },
            cookies: Vec::new(),
            timing: Default::default(),
            size: Default::default(),
            url: request.url,
            protocol: "SSE".to_string(),
        })
    }
}
