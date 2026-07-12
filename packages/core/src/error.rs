use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Request cancelled")]
    Cancelled,

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    #[error("Script execution error: {0}")]
    Script(String),

    #[error("Test assertion failed: {0}")]
    Assertion(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Protocol not supported: {0}")]
    UnsupportedProtocol(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl Error {
    pub fn other(msg: impl Into<String>) -> Self {
        Error::Other(msg.into())
    }

    pub fn connection(msg: impl Into<String>) -> Self {
        Error::Connection(msg.into())
    }

    pub fn tls(msg: impl Into<String>) -> Self {
        Error::Tls(msg.into())
    }

    pub fn auth(msg: impl Into<String>) -> Self {
        Error::Auth(msg.into())
    }

    pub fn script(msg: impl Into<String>) -> Self {
        Error::Script(msg.into())
    }

    pub fn assertion(msg: impl Into<String>) -> Self {
        Error::Assertion(msg.into())
    }

    pub fn storage(msg: impl Into<String>) -> Self {
        Error::Storage(msg.into())
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Error::Config(msg.into())
    }

    pub fn protocol(msg: impl Into<String>) -> Self {
        Error::Protocol(msg.into())
    }
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
