use crate::auth::types::{AuthProvider, AuthType};
use crate::error::{Error, Result};
use crate::request::{KeyValue, Request};
use async_trait::async_trait;
use base64::Engine;

/// HTTP Basic Authentication provider
///
/// Adds an `Authorization: Basic <base64(username:password)>` header.
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

impl BasicAuth {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Encode credentials as base64 for the Authorization header
    fn encode(&self) -> String {
        let raw = format!("{}:{}", self.username, self.password);
        base64::engine::general_purpose::STANDARD.encode(raw.as_bytes())
    }
}

#[async_trait]
impl AuthProvider for BasicAuth {
    fn auth_type(&self) -> AuthType {
        AuthType::Basic
    }

    async fn apply(&self, mut request: Request) -> Result<Request> {
        self.validate()?;

        let encoded = self.encode();

        request
            .headers
            .retain(|h| !h.key.eq_ignore_ascii_case("Authorization"));

        request.headers.push(KeyValue {
            key: "Authorization".to_string(),
            value: format!("Basic {}", encoded),
            enabled: true,
            description: None,
        });

        Ok(request)
    }

    fn validate(&self) -> Result<()> {
        if self.username.is_empty() {
            return Err(Error::auth("Basic auth username cannot be empty"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::HttpMethod;

    #[tokio::test]
    async fn test_basic_apply() {
        let auth = BasicAuth::new("alice", "secret");
        let req = Request::new(HttpMethod::Get, "https://api.example.com");

        let authed = auth.apply(req).await.unwrap();

        let auth_header = authed.headers.iter().find(|h| h.key == "Authorization").unwrap();
        // base64("alice:secret") = "YWxpY2U6c2VjcmV0"
        assert_eq!(auth_header.value, "Basic YWxpY2U6c2VjcmV0");
    }

    #[tokio::test]
    async fn test_basic_empty_username() {
        let auth = BasicAuth::new("", "secret");
        assert!(auth.validate().is_err());
    }
}
