use crate::auth::types::{AuthProvider, AuthType};
use crate::error::{Error, Result};
use crate::request::{KeyValue, Request};
use async_trait::async_trait;

/// JWT authentication provider
///
/// Adds an `Authorization: <prefix> <token>` header where the token
/// is a JSON Web Token. Also exposes a method to inspect the token's
/// claims for debugging.
pub struct JwtAuth {
    pub token: String,
    pub prefix: Option<String>,
}

impl JwtAuth {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            prefix: Some("Bearer".to_string()),
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
impl AuthProvider for JwtAuth {
    fn auth_type(&self) -> AuthType {
        AuthType::Jwt
    }

    fn validate(&self) -> Result<()> {
        if self.token.is_empty() {
            return Err(Error::auth("JWT token cannot be empty"));
        }
        Ok(())
    }

    async fn apply(&self, mut request: Request) -> Result<Request> {
        self.validate()?;
        let value = match &self.prefix {
            Some(prefix) => format!("{} {}", prefix, self.token),
            None => self.token.clone(),
        };
        request
            .headers
            .retain(|h| !h.key.eq_ignore_ascii_case("Authorization"));
        request.headers.push(KeyValue {
            key: "Authorization".to_string(),
            value,
            enabled: true,
            description: None,
        });
        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jwt_with_bearer_prefix() {
        let auth = JwtAuth::new("abc.def.ghi");
        let req = Request::new(crate::request::HttpMethod::Get, "https://x.test");
        let applied = auth.apply(req).await.unwrap();
        assert_eq!(applied.headers.last().unwrap().value, "Bearer abc.def.ghi");
    }

    #[tokio::test]
    async fn test_jwt_raw_no_prefix() {
        let auth = JwtAuth::raw("abc.def.ghi");
        let req = Request::new(crate::request::HttpMethod::Get, "https://x.test");
        let applied = auth.apply(req).await.unwrap();
        assert_eq!(applied.headers.last().unwrap().value, "abc.def.ghi");
    }
}
