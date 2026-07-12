use crate::auth::types::{AuthProvider, AuthType};
use crate::error::{Error, Result};
use crate::request::{KeyValue, Request};
use async_trait::async_trait;

/// OAuth 2.0 authentication provider
///
/// This implementation handles Bearer-token-style OAuth 2.0 flows where
/// an access token has already been obtained. The full OAuth 2.0 dance
/// (authorization code, PKCE, etc.) lives in the desktop app layer.
pub struct OAuth2Auth {
    pub access_token: String,
    pub token_type: Option<String>,
    pub refresh_token: Option<String>,
}

impl OAuth2Auth {
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            token_type: Some("Bearer".to_string()),
            refresh_token: None,
        }
    }

    pub fn with_refresh(access_token: impl Into<String>, refresh_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            token_type: Some("Bearer".to_string()),
            refresh_token: Some(refresh_token.into()),
        }
    }

    pub fn custom_token_type(mut self, token_type: impl Into<String>) -> Self {
        self.token_type = Some(token_type.into());
        self
    }
}

#[async_trait]
impl AuthProvider for OAuth2Auth {
    fn auth_type(&self) -> AuthType {
        AuthType::OAuth2
    }

    async fn apply(&self, mut request: Request) -> Result<Request> {
        self.validate()?;

        let token_type = self.token_type.as_deref().unwrap_or("Bearer");
        let header_value = format!("{} {}", token_type, self.access_token);

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
        if self.access_token.is_empty() {
            return Err(Error::auth("OAuth2 access token cannot be empty"));
        }
        Ok(())
    }
}

/// OAuth 2.0 grant types supported by ReqForge
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum OAuth2GrantType {
    AuthorizationCode,
    AuthorizationCodePkce,
    ClientCredentials,
    Password,
    RefreshToken,
}

/// OAuth 2.0 token response from the authorisation server
#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
pub struct OAuth2TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub token_type: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub scope: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::HttpMethod;

    #[tokio::test]
    async fn test_oauth2_apply() {
        let auth = OAuth2Auth::new("ya29.AccessToken");
        let req = Request::new(HttpMethod::Get, "https://api.example.com");

        let authed = auth.apply(req).await.unwrap();

        let auth_header = authed
            .headers
            .iter()
            .find(|h| h.key == "Authorization")
            .unwrap();
        assert_eq!(auth_header.value, "Bearer ya29.AccessToken");
    }

    #[tokio::test]
    async fn test_oauth2_custom_token_type() {
        let auth = OAuth2Auth::new("abc").custom_token_type("MAC");
        let req = Request::new(HttpMethod::Get, "https://api.example.com");

        let authed = auth.apply(req).await.unwrap();
        let auth_header = authed
            .headers
            .iter()
            .find(|h| h.key == "Authorization")
            .unwrap();
        assert_eq!(auth_header.value, "MAC abc");
    }

    #[tokio::test]
    async fn test_oauth2_empty_token() {
        let auth = OAuth2Auth::new("");
        let req = Request::new(HttpMethod::Get, "https://api.example.com");
        assert!(auth.apply(req).await.is_err());
    }
}
