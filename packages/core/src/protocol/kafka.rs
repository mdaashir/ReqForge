//! Kafka protocol client.
//!
//! Connects to a Kafka broker, produces a message to a topic, and returns
//! the acknowledgement. Uses the pure-Rust `kafka-rust` crate — no C deps.
//!
//! Feature-gated behind `kafka` — not compiled by default.

#![cfg_attr(not(feature = "kafka"), allow(dead_code, unused_imports))]

use crate::error::{Error, Result};
use crate::protocol::{ProtocolCapabilities, ProtocolHandler};
use crate::request::Request;
use async_trait::async_trait;

pub struct KafkaHandler;

impl KafkaHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KafkaHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolHandler for KafkaHandler {
    fn name(&self) -> &str {
        "Kafka"
    }

    fn schemes(&self) -> &[&str] {
        &["kafka", "kafkassl"]
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            can_send: true,
            can_receive: true,
            can_stream: true,
            can_subscribe: true,
        }
    }

    async fn send(&self, request: Request) -> Result<crate::request::Response> {
        #[cfg(feature = "kafka")]
        {
            self.send_inner(request).await
        }
        #[cfg(not(feature = "kafka"))]
        {
            let _ = request;
            Err(Error::other(
                "Kafka support is not enabled (build with --features kafka)",
            ))
        }
    }
}

#[cfg(feature = "kafka")]
impl KafkaHandler {
    async fn send_inner(&self, request: Request) -> Result<crate::request::Response> {
        use kafka::client::RequiredAcks;
        use kafka::producer::{Producer, Record};
        use std::time::Duration;

        // Parse broker URL: `kafka://host:port/topic` or `kafkassl://...`
        let url = &request.url;
        let rest = url
            .strip_prefix("kafka://")
            .or_else(|| url.strip_prefix("kafkassl://"))
            .ok_or_else(|| Error::other(format!("unsupported Kafka URL: {url}")))?;

        let (host, tail) = rest.split_once(':').unwrap_or((rest, "9092"));
        let (port_str, topic) = if let Some((p, t)) = tail.split_once('/') {
            (p, Some(t))
        } else {
            (tail, None)
        };
        let _port: u16 = port_str.parse().unwrap_or(9092);
        let topic = topic.unwrap_or("test").to_string();

        let broker = format!("{host}:{port_str}");
        let payload = request.body.content.clone().into_bytes();
        let started = std::time::Instant::now();

        let mut producer = Producer::from_hosts(vec![broker.clone()])
            .with_ack_timeout(Duration::from_secs(5))
            .with_required_acks(RequiredAcks::One)
            .create()
            .map_err(|e| Error::connection(format!("kafka create producer: {e}")))?;

        producer
            .send(&Record {
                topic: &topic,
                partition: -1,
                key: (),
                value: payload,
            })
            .map_err(|e| Error::connection(format!("kafka send: {e}")))?;

        let elapsed = started.elapsed().as_millis() as u64;

        Ok(crate::request::Response {
            status: 200,
            status_text: "Kafka".into(),
            headers: std::collections::HashMap::new(),
            body: crate::request::ResponseBody {
                content: format!("Produced to topic '{}' via broker '{}'", topic, broker)
                    .into_bytes(),
                content_type: Some("text/plain".into()),
                is_text: true,
            },
            cookies: Vec::new(),
            timing: crate::request::ResponseTiming {
                total_ms: elapsed,
                ..Default::default()
            },
            size: Default::default(),
            url: request.url,
            protocol: "Kafka".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_reports_capabilities() {
        let h = KafkaHandler::new();
        let caps = h.capabilities();
        // Kafka is a streaming protocol — must support subscribe
        assert!(caps.can_stream);
        assert!(caps.can_subscribe);
    }
}
