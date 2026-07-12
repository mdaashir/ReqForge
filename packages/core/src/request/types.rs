use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Trace,
    Connect,
    Custom(String),
}

impl HttpMethod {
    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Trace => "TRACE",
            HttpMethod::Connect => "CONNECT",
            HttpMethod::Custom(s) => s,
        }
    }

    pub fn is_safe(&self) -> bool {
        matches!(
            self,
            HttpMethod::Get | HttpMethod::Head | HttpMethod::Options
        )
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for HttpMethod {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "PATCH" => HttpMethod::Patch,
            "DELETE" => HttpMethod::Delete,
            "HEAD" => HttpMethod::Head,
            "OPTIONS" => HttpMethod::Options,
            "TRACE" => HttpMethod::Trace,
            "CONNECT" => HttpMethod::Connect,
            other => HttpMethod::Custom(other.to_string()),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Request {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<KeyValue>,
    pub params: Vec<KeyValue>,
    pub body: Body,
    pub auth: Option<Auth>,
    pub settings: RequestSettings,
    pub pre_request_script: Option<String>,
    pub post_response_script: Option<String>,
    pub test_script: Option<String>,
    pub description: Option<String>,
}

impl Request {
    pub fn new(method: HttpMethod, url: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            method,
            url: url.into(),
            headers: Vec::new(),
            params: Vec::new(),
            body: Body::default(),
            auth: None,
            settings: RequestSettings::default(),
            pre_request_script: None,
            post_response_script: None,
            test_script: None,
            description: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Body {
    pub content_type: Option<String>,
    pub content: String,
    pub mode: BodyMode,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BodyMode {
    #[default]
    None,
    Json,
    Xml,
    Text,
    Form,
    Multipart,
    Binary,
    Graphql,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Auth {
    pub auth_type: AuthType,
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    #[default]
    None,
    ApiKey,
    Bearer,
    Basic,
    OAuth2,
    Jwt,
    AwsSigV4,
    Custom(String),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RequestSettings {
    pub timeout_ms: Option<u64>,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub verify_ssl: bool,
    pub proxy_url: Option<String>,
    pub retry_count: u32,
    pub retry_delay_ms: u64,
}

impl RequestSettings {
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout_ms.map(Duration::from_millis)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: ResponseBody,
    pub cookies: Vec<Cookie>,
    pub timing: ResponseTiming,
    pub size: ResponseSize,
    pub url: String,
    pub protocol: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseBody {
    pub content: Vec<u8>,
    pub content_type: Option<String>,
    pub is_text: bool,
}

impl ResponseBody {
    pub fn text(&self) -> String {
        if self.is_text {
            String::from_utf8_lossy(&self.content).to_string()
        } else {
            String::new()
        }
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, crate::Error> {
        Ok(serde_json::from_str(&self.text())?)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseTiming {
    pub dns_ms: u64,
    pub connect_ms: u64,
    pub tls_ms: u64,
    pub send_ms: u64,
    pub wait_ms: u64,
    pub receive_ms: u64,
    pub total_ms: u64,
}

impl ResponseTiming {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTimingDetail {
    pub start: std::time::SystemTime,
    pub dns: Option<std::time::SystemTime>,
    pub connect: Option<std::time::SystemTime>,
    pub tls: Option<std::time::SystemTime>,
    pub first_byte: Option<std::time::SystemTime>,
    pub end: Option<std::time::SystemTime>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseSize {
    pub headers: u64,
    pub body: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub expires: Option<i64>,
}
