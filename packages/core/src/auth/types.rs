use crate::error::{Error, Result};
use crate::request::{KeyValue, Request};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// The type of authentication to apply
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    None,
    ApiKey,
    Bearer,
    Basic,
    OAuth2,
    Jwt,
}

/// Where an API Key should be placed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    #[default]
    Header,
    Query,
    Cookie,
}

/// Credentials/configuration for a particular auth type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthCredentials {
    None,
    ApiKey {
        key: String,
        value: String,
        #[serde(default)]
        location: ApiKeyLocation,
    },
    Bearer {
        token: String,
        #[serde(default)]
        prefix: Option<String>,
    },
    Basic {
        username: String,
        password: String,
    },
    OAuth2 {
        access_token: String,
        #[serde(default)]
        token_type: Option<String>,
        #[serde(default)]
        refresh_token: Option<String>,
    },
    Jwt {
        token: String,
        #[serde(default)]
        prefix: Option<String>,
    },
}

/// Trait implemented by all auth providers
///
/// An auth provider takes a request and returns a new request with
/// authentication credentials applied (headers, query params, etc).
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// The auth type this provider implements
    fn auth_type(&self) -> AuthType;

    /// Apply auth credentials to a request, returning the modified request
    async fn apply(&self, mut request: Request) -> Result<Request>;

    /// Validate the credentials are present (non-empty required fields)
    fn validate(&self) -> Result<()>;
}

/// API Key auth provider
pub struct ApiKeyAuth {
    pub key: String,
    pub value: String,
    pub location: ApiKeyLocation,
}

impl ApiKeyAuth {
    pub fn new(
        key: impl Into<String>,
        value: impl Into<String>,
        location: ApiKeyLocation,
    ) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            location,
        }
    }

    pub fn header(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(key, value, ApiKeyLocation::Header)
    }

    pub fn query(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(key, value, ApiKeyLocation::Query)
    }
}

#[async_trait]
impl AuthProvider for ApiKeyAuth {
    fn auth_type(&self) -> AuthType {
        AuthType::ApiKey
    }

    async fn apply(&self, mut request: Request) -> Result<Request> {
        if self.key.is_empty() {
            return Err(Error::auth("API key name cannot be empty"));
        }

        match self.location {
            ApiKeyLocation::Header => {
                request.headers.push(KeyValue {
                    key: self.key.clone(),
                    value: self.value.clone(),
                    enabled: true,
                    description: None,
                });
            }
            ApiKeyLocation::Query => {
                request.params.push(KeyValue {
                    key: self.key.clone(),
                    value: self.value.clone(),
                    enabled: true,
                    description: None,
                });
            }
            ApiKeyLocation::Cookie => {
                // Cookies are handled at the client level via a Cookie header
                request.headers.push(KeyValue {
                    key: "Cookie".to_string(),
                    value: format!("{}={}", self.key, self.value),
                    enabled: true,
                    description: None,
                });
            }
        }

        Ok(request)
    }

    fn validate(&self) -> Result<()> {
        if self.key.is_empty() {
            return Err(Error::auth("API key name cannot be empty"));
        }
        Ok(())
    }
}
