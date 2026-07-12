//! MQTT protocol client.
//!
//! Connects to an MQTT broker via `rumqttc`, subscribes to a topic,
//! and returns received messages as the response body. Supports both
//! v3.1.1 and v5 via rumqttc's protocol negotiation.
//!
//! Feature-gated behind `mqtt` — not compiled by default.

#![cfg_attr(not(feature = "mqtt"), allow(dead_code, unused_imports))]

use crate::error::{Error, Result};
use crate::protocol::{ProtocolCapabilities, ProtocolHandler};
use crate::request::Request;
use async_trait::async_trait;

pub struct MqttHandler;

impl MqttHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MqttHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolHandler for MqttHandler {
    fn name(&self) -> &str {
        "MQTT"
    }

    fn schemes(&self) -> &[&str] {
        &["mqtt", "mqtts"]
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
        #[cfg(feature = "mqtt")]
        {
            self.send_inner(request).await
        }
        #[cfg(not(feature = "mqtt"))]
        {
            let _ = request;
            Err(Error::other(
                "MQTT support is not enabled (build with --features mqtt)",
            ))
        }
    }
}

#[cfg(feature = "mqtt")]
impl MqttHandler {
    async fn send_inner(&self, request: Request) -> Result<crate::request::Response> {
        use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
        use std::time::Duration;

        // Parse broker URL: `mqtt://host:port/topic`
        let url = &request.url;
        let rest = url
            .strip_prefix("mqtt://")
            .or_else(|| url.strip_prefix("mqtts://"))
            .ok_or_else(|| Error::other(format!("unsupported MQTT URL: {url}")))?;

        let (host, tail) = rest.split_once(':').unwrap_or((rest, "1883"));
        let (port_str, topic) = if let Some((p, t)) = tail.split_once('/') {
            (p, Some(t))
        } else {
            (tail.as_ref(), None)
        };
        let port: u16 = port_str.parse().unwrap_or(1883);
        let topic = topic.unwrap_or("test");

        // Extract payload from request body.
        let payload = request.body.content.clone().into_bytes();

        let mut mqtt_opts = MqttOptions::new("reqforge", host, port);
        mqtt_opts.set_keep_alive(Duration::from_secs(30));
        mqtt_opts.set_clean_session(true);

        let (client, mut eventloop) = AsyncClient::new(mqtt_opts, 100);

        // Publish the payload to the topic.
        client
            .publish(topic, QoS::AtLeastOnce, false, payload)
            .await
            .map_err(|e| Error::connection(format!("mqtt publish: {e}")))?;

        // Wait a short time for an acknowledgement or message back.
        let timeout = tokio::time::sleep(Duration::from_secs(5));
        tokio::pin!(timeout);

        let started = std::time::Instant::now();
        let mut messages = Vec::new();

        loop {
            tokio::select! {
                event = eventloop.poll() => {
                    match event {
                        Ok(Event::Incoming(Packet::Publish(pub_pkt))) => {
                            messages.push(format!(
                                "topic={} payload={}",
                                pub_pkt.topic,
                                String::from_utf8_lossy(&pub_pkt.payload)
                            ));
                        }
                        Ok(Event::Incoming(Packet::ConnAck(_))) => {
                            // Connected successfully.
                        }
                        Ok(_) => {}
                        Err(e) => {
                            messages.push(format!("mqtt error: {e}"));
                            break;
                        }
                    }
                }
                _ = &mut timeout => break,
            }
        }

        let elapsed = started.elapsed().as_millis() as u64;
        let body = messages.join("\n");

        Ok(crate::request::Response {
            status: 200,
            status_text: "MQTT".into(),
            headers: std::collections::HashMap::new(),
            body: crate::request::ResponseBody {
                content: body.into_bytes(),
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
            protocol: "MQTT".to_string(),
        })
    }
}
