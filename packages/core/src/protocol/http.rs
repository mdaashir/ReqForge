use crate::error::Result;
use crate::protocol::{ProtocolCapabilities, ProtocolHandler};
use crate::request::RequestExecutor;
use crate::request::{Request, Response};
use async_trait::async_trait;

/// HTTP protocol handler
pub struct HttpHandler {
    executor: RequestExecutor,
}

impl HttpHandler {
    pub fn new() -> Self {
        Self {
            executor: RequestExecutor::new().expect("Failed to create HTTP handler"),
        }
    }
}

impl Default for HttpHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolHandler for HttpHandler {
    fn name(&self) -> &str {
        "HTTP"
    }

    fn schemes(&self) -> &[&str] {
        &["http", "https"]
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            can_send: true,
            can_receive: true,
            can_stream: true,
            can_subscribe: false,
        }
    }

    async fn send(&self, request: Request) -> Result<Response> {
        self.executor.execute(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::HttpMethod;

    #[tokio::test]
    async fn test_http_handler_creation() {
        let handler = HttpHandler::new();
        assert_eq!(handler.name(), "HTTP");
        assert_eq!(handler.schemes(), &["http", "https"]);
    }

    #[tokio::test]
    async fn test_http_handler_capabilities() {
        let handler = HttpHandler::new();
        let caps = handler.capabilities();
        assert!(caps.can_send);
        assert!(caps.can_receive);
        assert!(caps.can_stream);
        assert!(!caps.can_subscribe);
    }

    #[test]
    fn test_http_request_creation() {
        let request = Request::new(HttpMethod::Get, "https://api.example.com/users");
        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.url, "https://api.example.com/users");
    }
}
