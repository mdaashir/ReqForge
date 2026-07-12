use crate::error::{Error, Result};
use crate::protocol::{ProtocolCapabilities, ProtocolHandler};
use crate::request::RequestExecutor;
use crate::request::{Body, BodyMode, KeyValue, Request};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// GraphQL operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum GraphQLOperationType {
    #[default]
    Query,
    Mutation,
    Subscription,
}

/// A parsed GraphQL request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLBody {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<serde_json::Value>,
    /// GraphQL spec uses camelCase (`operationName`). We rename on the wire
    /// while keeping snake_case in Rust for ergonomic access.
    #[serde(
        rename = "operationName",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub operation_name: Option<String>,
}

impl GraphQLBody {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            variables: None,
            operation_name: None,
        }
    }

    /// Parse a GraphQL query string into a request body
    pub fn from_query_string(query: &str) -> Self {
        let trimmed = query.trim();
        let operation_name = if trimmed.starts_with("mutation") {
            Some("mutation".to_string())
        } else if trimmed.starts_with("subscription") {
            Some("subscription".to_string())
        } else {
            Some("query".to_string())
        };

        Self {
            query: trimmed.to_string(),
            variables: None,
            operation_name,
        }
    }

    /// Detect the operation type from a query string
    pub fn detect_operation_type(query: &str) -> GraphQLOperationType {
        let trimmed = query.trim_start();
        if trimmed.starts_with("mutation") {
            GraphQLOperationType::Mutation
        } else if trimmed.starts_with("subscription") {
            GraphQLOperationType::Subscription
        } else {
            GraphQLOperationType::Query
        }
    }
}

/// GraphQL error response field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<GraphQLLocation>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLLocation {
    pub line: u32,
    pub column: u32,
}

/// GraphQL success response payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLResponse {
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub errors: Option<Vec<GraphQLError>>,
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
}

/// GraphQL protocol handler
///
/// Sends GraphQL queries/mutations over HTTP POST. Subscription support
/// requires WebSocket and is implemented in the WebSocketHandler.
pub struct GraphQLHandler {
    executor: RequestExecutor,
}

impl GraphQLHandler {
    pub fn new() -> Self {
        Self {
            executor: RequestExecutor::new().expect("Failed to create GraphQL handler"),
        }
    }
}

impl Default for GraphQLHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolHandler for GraphQLHandler {
    fn name(&self) -> &str {
        "GraphQL"
    }

    fn schemes(&self) -> &[&str] {
        &["graphql", "http", "https"]
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            can_send: true,
            can_receive: true,
            can_stream: false,
            can_subscribe: false, // Subscriptions use WebSocket
        }
    }

    async fn send(&self, mut request: Request) -> Result<crate::request::Response> {
        // Build GraphQL body from request body
        let body_content = request.body.content.clone();
        let graphql_body = if !body_content.is_empty() {
            // Try to parse as JSON first
            serde_json::from_str::<GraphQLBody>(&body_content).unwrap_or_else(|_| {
                // Treat as raw query string
                GraphQLBody::from_query_string(&body_content)
            })
        } else {
            return Err(Error::other("GraphQL request body is empty"));
        };

        let json = serde_json::to_string(&graphql_body)?;
        request.body = Body {
            content: json,
            content_type: Some("application/json".to_string()),
            mode: BodyMode::Json,
        };

        // Force POST for GraphQL (queries can technically use GET, but POST
        // is the universally supported option)
        request.method = crate::request::HttpMethod::Post;

        // Ensure Accept header
        let has_accept = request
            .headers
            .iter()
            .any(|h| h.key.eq_ignore_ascii_case("Accept"));
        if !has_accept {
            request.headers.push(KeyValue {
                key: "Accept".to_string(),
                value: "application/json".to_string(),
                enabled: true,
                description: None,
            });
        }

        self.executor.execute(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_query() {
        assert_eq!(
            GraphQLBody::detect_operation_type("{ user { id } }"),
            GraphQLOperationType::Query
        );
    }

    #[test]
    fn test_detect_mutation() {
        assert_eq!(
            GraphQLBody::detect_operation_type("mutation { createUser { id } }"),
            GraphQLOperationType::Mutation
        );
    }

    #[test]
    fn test_detect_subscription() {
        assert_eq!(
            GraphQLBody::detect_operation_type("subscription { onMessage { id } }"),
            GraphQLOperationType::Subscription
        );
    }

    #[test]
    fn test_from_query_string() {
        let body = GraphQLBody::from_query_string("{ user { id name } }");
        assert_eq!(body.operation_name.as_deref(), Some("query"));
        assert!(body.query.contains("user"));
    }

    #[test]
    fn test_serialize_graphql_body() {
        let body = GraphQLBody {
            query: "{ user { id } }".to_string(),
            variables: Some(serde_json::json!({"id": 1})),
            operation_name: Some("GetUser".to_string()),
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("query"));
        assert!(json.contains("variables"));
        assert!(json.contains("operationName"));
    }
}
