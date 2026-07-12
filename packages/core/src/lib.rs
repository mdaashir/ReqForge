//! ReqForge Core
//!
//! Shared library for request execution, protocol handling, storage, and more.
//! Used by both the Tauri desktop app and the CLI companion.

pub mod auth;
pub mod crypto;
pub mod collection;
pub mod environment;
pub mod error;
pub mod history;
pub mod loadtest;
pub mod mock;
pub mod import;
pub mod plugin;
pub mod protocol;
pub mod request;
pub mod samples;
pub mod scripting;
pub mod storage;
pub mod sync;
pub mod telemetry;
pub mod testing;
#[cfg(feature = "watcher")]
pub mod watcher;

pub use auth::{
    ApiKeyAuth, ApiKeyLocation, AuthCredentials, AuthProvider, AuthType, BasicAuth, BearerAuth,
    JwtAuth, OAuth2Auth, aws_sig_v4::AwsSigV4Auth,
};
pub use collection::{Collection, CollectionItem, CollectionMap, CollectionStorage};
pub use environment::{
    Environment, EnvironmentError, EnvironmentStorage, GlobalVariables, Variable,
    VariableResolver, VariableScope, VariableType,
};
pub use error::{Error, Result};
#[cfg(feature = "plugins")]
pub use plugin::PluginHost;
pub use history::{HistoryEntry, HistoryStorage, DEFAULT_HISTORY_LIMIT};
pub use import::{BrunoImporter, CurlImporter, Importer, InsomniaImporter, PostmanImporter};
pub use protocol::{
    graphql::{GraphQLBody, GraphQLError, GraphQLHandler, GraphQLOperationType, GraphQLResponse},
    http::HttpHandler,
    websocket::{
        ConnectionState, MessageDirection, WebSocketConfig, WebSocketConnection,
        WebSocketHandler, WebSocketMessage, WebSocketMessageType,
    },
    ProtocolCapabilities, ProtocolHandler,
};
pub use request::{HttpMethod, Request, Response, ResponseTiming};
pub use testing::{Assertion, AssertionResult, AssertionType, TestResult, TestRunner, TestStatus};
