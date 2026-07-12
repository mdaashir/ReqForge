use crate::auth::types::{AuthProvider, AuthType};
use crate::error::{Error, Result};
use crate::request::{KeyValue, Request};
use async_trait::async_trait;

/// Bearer token authentication provider
///
/// Adds an `Authorization: Bearer <token>` header to the request.
pub struct BearerAuth {
    pub token: String,
    pub prefix: Option<String>,
}

impl BearerAuth {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            prefix: Some("Bearer".to_string()),
        }
    }

    pub fn with_prefix(token: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            prefix: Some(prefix.into()),
        }
    }

    pub fn raw(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            prefix: None,
        }
    }
}

#[async_trait]
impl AuthProvider for BearerAuth {
    fn auth_type(&self) -> AuthType {
        AuthType::Bearer
    }

    async fn apply(&self, mut request: Request) -> Result<Request> {
        self.validate()?;

        let header_value = match &self.prefix {
            Some(prefix) => format!("{} {}", prefix, self.token),
            None => self.token.clone(),
        };

        // Remove any existing Authorization header to avoid duplicates
        request
            .headers
            .retain(|h| !h.key.eq_ignore_ascii_case("Authorization"));

        request.headers.push(KeyValue {
            key: "Authorization".to_string(),
            value: header_value,
            enabled: true,
            description: None,
        });

        Ok(request)
    }

    fn validate(&self) -> Result<()> {
        if self.token.is_empty() {
            return Err(Error::auth("Bearer token cannot be empty"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::HttpMethod;

    #[tokio::test]
    async fn test_bearer_apply() {
        let auth = BearerAuth::new("my-token");
        let req = Request::new(HttpMethod::Get, "https://api.example.com");

        let authed = auth.apply(req).await.unwrap();

        let auth_header = authed
            .headers
            .iter()
            .find(|h| h.key == "Authorization")
            .expect("Authorization header should be set");

        assert_eq!(auth_header.value, "Bearer my-token");
    }

    #[tokio::test]
    async fn test_bearer_custom_prefix() {
        let auth = BearerAuth::with_prefix("abc", "Token");
        let req = Request::new(HttpMethod::Get, "https://api.example.com");

        let authed = auth.apply(req).await.unwrap();
        let auth_header = authed.headers.iter().find(|h| h.key == "Authorization").unwrap();
        assert_eq!(auth_header.value, "Token abc");
    }

    #[tokio::test]
    async fn test_bearer_raw() {
        let auth = BearerAuth::raw("custom-token");
        let req = Request::new(HttpMethod::Get, "https://api.example.com");

        let authed = auth.apply(req).await.unwrap();
        let auth_header = authed.headers.iter().find(|h| h.key == "Authorization").unwrap();
        assert_eq!(auth_header.value, "custom-token");
    }

    #[tokio::test]
    async fn test_bearer_empty_token() {
        let auth = BearerAuth::new("");
        let req = Request::new(HttpMethod::Get, "https://api.example.com");
        assert!(auth.apply(req).await.is_err());
    }
}
