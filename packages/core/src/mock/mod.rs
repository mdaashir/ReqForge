//! In-process mock server.
//!
//! Matches incoming request attributes (method, URL pattern, headers)
//! against a set of user-defined rules and returns a canned response.
//! Runs on a local TCP port so the client can hit it like a real server.
//!
//! Rules are an ordered list — the first matching rule wins.
//! This is NOT a standalone server — it's embedded in the app process.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRule {
    pub id: String,
    pub name: String,
    /// HTTP method to match (`*` for any).
    #[serde(default = "default_any")]
    pub method: String,
    /// URL pattern to match (substring). `*` matches everything.
    #[serde(default = "default_any")]
    pub url_pattern: String,
    /// HTTP status code to return.
    #[serde(default = "default_status")]
    pub status: u16,
    /// Response headers.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Response body.
    #[serde(default)]
    pub body: String,
    /// Delay in milliseconds before responding.
    #[serde(default)]
    pub delay_ms: u64,
    /// Whether this rule is currently active.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_any() -> String {
    "*".to_string()
}

fn default_status() -> u16 {
    200
}

fn default_true() -> bool {
    true
}

impl MockRule {
    pub fn matches(&self, method: &str, url: &str) -> bool {
        if !self.enabled {
            return false;
        }
        let method_ok = self.method == "*" || self.method.eq_ignore_ascii_case(method);
        let url_ok = self.url_pattern == "*" || url.contains(&self.url_pattern);
        method_ok && url_ok
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub struct MockServer {
    rules: Arc<RwLock<Vec<MockRule>>>,
    addr: Option<SocketAddr>,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

impl MockServer {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            addr: None,
            shutdown: None,
        }
    }

    /// Add or replace a rule.
    pub async fn set_rule(&self, rule: MockRule) {
        let mut rules = self.rules.write().await;
        if let Some(pos) = rules.iter().position(|r| r.id == rule.id) {
            rules[pos] = rule;
        } else {
            rules.push(rule);
        }
    }

    pub async fn remove_rule(&self, id: &str) {
        let mut rules = self.rules.write().await;
        rules.retain(|r| r.id != id);
    }

    pub async fn rules(&self) -> Vec<MockRule> {
        self.rules.read().await.clone()
    }

    pub fn addr(&self) -> Option<SocketAddr> {
        self.addr
    }

    /// Start the mock server on a random available port.
    /// Returns the address so callers can configure their client.
    #[cfg(feature = "mock-server")]
pub async fn start(&mut self) -> Result<SocketAddr, crate::error::Error> {
        use axum::extract::State as AxState;
        use axum::routing::any;
        use axum::{Json, Router, response::IntoResponse};

        let rules = self.rules.clone();
        let app = Router::new()
            .route("/*path", any(move |method: axum::http::Method, uri: axum::http::Uri, AxState(rules): AxState<Arc<RwLock<Vec<MockRule>>>>| async move {
                let method = method.to_string();
                let uri = uri.to_string();
                let rules = rules.read().await;
                let matched = rules.iter().find(|r| r.matches(&method, &uri));
                let resp = match matched {
                    Some(rule) => {
                        if rule.delay_ms > 0 {
                            tokio::time::sleep(tokio::time::Duration::from_millis(rule.delay_ms)).await;
                        }
                        (rule.status, rule.headers.clone(), rule.body.clone())
                    }
                    None => {
                        (404, HashMap::new(), "{\"error\":\"no mock rule matched\"}".to_string())
                    }
                };
                let mut response = axum::response::Response::new(axum::body::Body::from(resp.2));
                *response.status_mut() = axum::http::StatusCode::from_u16(resp.0).unwrap_or(axum::http::StatusCode::NOT_FOUND);
                for (k, v) in &resp.1 {
                    response.headers_mut().insert(axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap_or(axum::http::HeaderName::from_static("x-default")), axum::http::HeaderValue::from_str(v).unwrap_or(axum::http::HeaderValue::from_static("")));
                }
                response
            }))
            .with_state(rules);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| crate::error::Error::other(format!("mock server bind: {e}")))?;
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async { let _ = rx.await; })
                .await
                .ok();
        });

        self.addr = Some(addr);
        self.shutdown = Some(tx);
        Ok(addr)
    }

    /// Stop the mock server.
    #[cfg(not(feature = "mock-server"))]
    pub async fn start(&mut self) -> Result<SocketAddr, crate::error::Error> {
        Err(crate::error::Error::other("mock server requires `mock-server` feature on reqforge-core"))
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        self.addr = None;
    }
}

impl Default for MockServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "mock-server"))]
mod tests {
    use super::*;

    fn sample_rule(name: &str, url_pattern: &str, status: u16) -> MockRule {
        MockRule {
            id: name.to_string(),
            name: name.to_string(),
            method: "*".into(),
            url_pattern: url_pattern.into(),
            status,
            headers: HashMap::new(),
            body: format!("{{\"status\":{status}}}"),
            delay_ms: 0,
            enabled: true,
        }
    }

    #[test]
    fn test_matches_wildcard() {
        let r = sample_rule("all", "*", 200);
        assert!(r.matches("GET", "anything"));
        assert!(r.matches("POST", "anything"));
    }

    #[test]
    fn test_matches_url_pattern() {
        let r = sample_rule("users", "/users", 200);
        assert!(r.matches("GET", "https://api.example.com/users"));
        assert!(!r.matches("GET", "https://api.example.com/posts"));
    }

    #[test]
    fn test_disabled_doesnt_match() {
        let mut r = sample_rule("x", "*", 200);
        r.enabled = false;
        assert!(!r.matches("GET", "anything"));
    }

    #[tokio::test]
    async fn test_server_start_stop() {
        let mut server = MockServer::new();
        let addr = server.start().await.unwrap();
        assert!(addr.port() > 0);
        server.stop();
    }

    #[tokio::test]
    async fn test_server_matches_rule() {
        let mut server = MockServer::new();
        let addr = server.start().await.unwrap();
        server
            .set_rule(sample_rule("hello", "/hello", 200))
            .await;

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{addr}/hello"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        // Unmatched route returns 404.
        let resp = client
            .get(format!("http://{addr}/goodbye"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 404);

        server.stop();
    }
}
