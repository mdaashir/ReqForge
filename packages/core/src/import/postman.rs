//! Postman v2.1 collection importer
//!
//! Parses a Postman collection JSON document and converts it to a
//! ReqForge `Collection`. Only the fields used by ReqForge are mapped;
//! everything else is dropped (Postman has many optional fields).

use crate::collection::{Collection, CollectionItem};
use crate::error::{Error, Result};
use crate::import::Importer;
use crate::request::{Auth as CoreAuth, AuthType as CoreAuthType, Body, BodyMode, HttpMethod, KeyValue, Request};
use serde::Deserialize;

/// Re-export of the public type alias for downstream users
pub type PostmanV21 = serde_json::Value;

#[derive(Debug, Deserialize)]
struct PostmanCollection {
    info: PostmanInfo,
    item: Vec<PostmanItem>,
    #[serde(default)]
    auth: Option<PostmanAuth>,
    #[serde(default)]
    variable: Vec<PostmanVariable>,
}

#[derive(Debug, Deserialize)]
struct PostmanInfo {
    name: String,
    #[serde(default)]
    description: Option<String>,
    /// Postman's stable identifier for the collection. Accepted but not
    /// surfaced in ReqForge today.
    #[serde(default)]
    #[allow(dead_code)]
    _postman_id: Option<String>,
    /// Schema URL is validated upstream by `detect_importer`; we keep it
    /// on the struct so future versions can inspect it.
    #[serde(default)]
    #[allow(dead_code)]
    schema: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PostmanItem {
    // NOTE: `item` is intentionally NOT `#[serde(default)]` here. With
    // untagged enums, serde tries variants in order. If `item` defaulted
    // to an empty vec, then a leaf request (which has no `item` field)
    // would match this variant first and the `request` field would be
    // silently dropped. Requiring `item` forces serde to fall through
    // to `Leaf` when the JSON is actually a request, not a folder.
    Item {
        name: String,
        item: Vec<PostmanItem>,
        /// A folder can in theory also carry a request in Postman's format,
        /// but we model folders as folders-only. Kept for round-trip safety.
        #[serde(default)]
        #[allow(dead_code)]
        request: Option<PostmanRequest>,
    },
    Leaf {
        name: String,
        #[serde(default)]
        request: Option<PostmanRequest>,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct PostmanRequest {
    method: Option<String>,
    #[serde(default)]
    url: Option<PostmanUrl>,
    #[serde(default)]
    header: Vec<PostmanHeader>,
    #[serde(default)]
    body: Option<PostmanBody>,
    #[serde(default)]
    auth: Option<PostmanAuth>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum PostmanUrl {
    String(String),
    Object {
        raw: Option<String>,
        #[serde(default)]
        host: Vec<String>,
        #[serde(default)]
        path: Vec<String>,
        #[serde(default)]
        query: Vec<PostmanQuery>,
        /// URL template variables (`{{baseUrl}}` etc.). Used by Postman
        /// for variable substitution; kept for round-trip compatibility.
        #[serde(default)]
        #[allow(dead_code)]
        variable: Vec<PostmanVariable>,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct PostmanQuery {
    key: String,
    value: String,
    #[serde(default)]
    disabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct PostmanHeader {
    key: String,
    value: String,
    #[serde(default)]
    disabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct PostmanBody {
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    raw: Option<String>,
    #[serde(default)]
    urlencoded: Option<Vec<PostmanQuery>>,
    #[serde(default)]
    formdata: Option<Vec<PostmanFormField>>,
}

#[derive(Debug, Clone, Deserialize)]
struct PostmanFormField {
    key: String,
    value: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PostmanAuth {
    #[serde(rename = "type")]
    auth_type: Option<String>,
    #[serde(default)]
    bearer: Vec<PostmanAuthField>,
    #[serde(default)]
    basic: Vec<PostmanAuthField>,
    #[serde(default)]
    apikey: Vec<PostmanAuthField>,
}

#[derive(Debug, Clone, Deserialize)]
struct PostmanAuthField {
    key: String,
    value: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PostmanVariable {
    key: String,
    value: Option<String>,
}

pub struct PostmanImporter;

impl PostmanImporter {
    pub fn new() -> Self {
        Self
    }

    fn convert_item(item: PostmanItem) -> Result<CollectionItem> {
        match item {
            PostmanItem::Item { name, item, request: _ } => {
                let children = item
                    .into_iter()
                    .map(Self::convert_item)
                    .collect::<Result<Vec<_>>>()?;
                Ok(CollectionItem::Folder {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    description: None,
                    children,
                    auth: None,
                })
            }
            PostmanItem::Leaf { name, request } => {
                let req = request
                    .ok_or_else(|| Error::other(format!("Leaf '{}' has no request", name)))?;
                let core_request = Self::convert_request(name.clone(), req)?;
                Ok(CollectionItem::Request {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    request: core_request,
                })
            }
        }
    }

    fn convert_request(name: String, pr: PostmanRequest) -> Result<Request> {
        let method = pr
            .method
            .as_deref()
            .unwrap_or("GET")
            .parse::<HttpMethod>()
            .map_err(|e| Error::other(format!("Invalid HTTP method: {}", e)))?;

        let url = Self::extract_url(&pr.url);
        let headers: Vec<KeyValue> = pr
            .header
            .into_iter()
            .map(|h| KeyValue {
                key: h.key,
                value: h.value,
                enabled: !h.disabled.unwrap_or(false),
                description: None,
            })
            .collect();

        let params: Vec<KeyValue> = match pr.url.as_ref() {
            Some(PostmanUrl::Object { query, .. }) => query
                .iter()
                .map(|q| KeyValue {
                    key: q.key.clone(),
                    value: q.value.clone(),
                    enabled: !q.disabled.unwrap_or(false),
                    description: None,
                })
                .collect(),
            _ => Vec::new(),
        };

        let body = Self::convert_body(pr.body.as_ref());

        let auth = pr.auth.as_ref().and_then(Self::convert_auth);

        Ok(Request {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            method,
            url,
            headers,
            params,
            body,
            auth,
            settings: Default::default(),
            pre_request_script: None,
            post_response_script: None,
            test_script: None,
            description: None,
        })
    }

    fn extract_url(url: &Option<PostmanUrl>) -> String {
        match url {
            Some(PostmanUrl::String(s)) => s.clone(),
            Some(PostmanUrl::Object { raw, host, path, .. }) => {
                if let Some(raw) = raw {
                    return raw.clone();
                }
                let host = host.join(".");
                let path = path.join("/");
                if path.is_empty() {
                    host
                } else {
                    format!("{}/{}", host, path)
                }
            }
            None => String::new(),
        }
    }

    fn convert_body(body: Option<&PostmanBody>) -> Body {
        let Some(body) = body else {
            return Body::default();
        };

        match body.mode.as_deref() {
            Some("raw") => {
                let raw = body.raw.clone().unwrap_or_default();
                let content_type = if raw.trim_start().starts_with('{') {
                    Some("application/json".to_string())
                } else if raw.trim_start().starts_with('<') {
                    Some("application/xml".to_string())
                } else {
                    Some("text/plain".to_string())
                };
                Body {
                    content: raw,
                    content_type,
                    mode: BodyMode::Text,
                }
            }
            Some("urlencoded") => {
                let raw = body
                    .urlencoded
                    .as_ref()
                    .map(|fields| {
                        fields
                            .iter()
                            .map(|f| format!("{}={}", f.key, f.value))
                            .collect::<Vec<_>>()
                            .join("&")
                    })
                    .unwrap_or_default();
                Body {
                    content: raw,
                    content_type: Some("application/x-www-form-urlencoded".to_string()),
                    mode: BodyMode::Form,
                }
            }
            Some("formdata") => {
                let raw = body
                    .formdata
                    .as_ref()
                    .map(|fields| {
                        fields
                            .iter()
                            .map(|f| format!("{}={}", f.key, f.value.clone().unwrap_or_default()))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                Body {
                    content: raw,
                    content_type: Some("multipart/form-data".to_string()),
                    mode: BodyMode::Multipart,
                }
            }
            _ => Body::default(),
        }
    }

    fn convert_auth(auth: &PostmanAuth) -> Option<CoreAuth> {
        let auth_type = auth.auth_type.as_deref()?;
        match auth_type {
            "bearer" => {
                let token = auth
                    .bearer
                    .iter()
                    .find(|f| f.key == "token")
                    .and_then(|f| f.value.clone())
                    .unwrap_or_default();
                Some(CoreAuth {
                    auth_type: CoreAuthType::Bearer,
                    config: [("token".to_string(), token)].into_iter().collect(),
                })
            }
            "basic" => {
                let username = auth
                    .basic
                    .iter()
                    .find(|f| f.key == "username")
                    .and_then(|f| f.value.clone())
                    .unwrap_or_default();
                let password = auth
                    .basic
                    .iter()
                    .find(|f| f.key == "password")
                    .and_then(|f| f.value.clone())
                    .unwrap_or_default();
                Some(CoreAuth {
                    auth_type: CoreAuthType::Basic,
                    config: [
                        ("username".to_string(), username),
                        ("password".to_string(), password),
                    ]
                    .into_iter()
                    .collect(),
                })
            }
            "apikey" => {
                let key = auth
                    .apikey
                    .iter()
                    .find(|f| f.key == "key")
                    .and_then(|f| f.value.clone())
                    .unwrap_or_default();
                let value = auth
                    .apikey
                    .iter()
                    .find(|f| f.key == "value")
                    .and_then(|f| f.value.clone())
                    .unwrap_or_default();
                let location = auth
                    .apikey
                    .iter()
                    .find(|f| f.key == "in")
                    .and_then(|f| f.value.clone())
                    .unwrap_or_else(|| "header".to_string());
                Some(CoreAuth {
                    auth_type: CoreAuthType::ApiKey,
                    config: [
                        ("key".to_string(), key),
                        ("value".to_string(), value),
                        ("location".to_string(), location),
                    ]
                    .into_iter()
                    .collect(),
                })
            }
            _ => None,
        }
    }
}

impl Default for PostmanImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Importer for PostmanImporter {
    fn format(&self) -> &'static str {
        "postman"
    }

    fn import(&self, input: &str) -> Result<Collection> {
        let parsed: PostmanCollection = serde_json::from_str(input)
            .map_err(|e| Error::other(format!("Invalid Postman JSON: {}", e)))?;

        let items = parsed
            .item
            .into_iter()
            .map(PostmanImporter::convert_item)
            .collect::<Result<Vec<_>>>()?;

        let auth = parsed.auth.as_ref().and_then(PostmanImporter::convert_auth);

        Ok(Collection {
            id: uuid::Uuid::new_v4().to_string(),
            name: parsed.info.name,
            description: parsed.info.description,
            auth,
            headers: Vec::new(),
            variables: parsed
                .variable
                .into_iter()
                .map(|v| KeyValue {
                    key: v.key,
                    value: v.value.unwrap_or_default(),
                    enabled: true,
                    description: None,
                })
                .collect(),
            items,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    fn file_extension(&self) -> Option<&'static str> {
        Some("json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "info": {
            "name": "Sample API",
            "description": "A sample",
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "item": [
            {
                "name": "Users",
                "item": [
                    {
                        "name": "Get All Users",
                        "request": {
                            "method": "GET",
                            "url": {
                                "raw": "https://api.example.com/users",
                                "host": ["api", "example", "com"],
                                "path": ["users"]
                            }
                        }
                    }
                ]
            },
            {
                "name": "Create User",
                "request": {
                    "method": "POST",
                    "header": [{"key": "Content-Type", "value": "application/json"}],
                    "url": {"raw": "https://api.example.com/users"},
                    "body": {
                        "mode": "raw",
                        "raw": "{\"name\": \"Alice\"}"
                    }
                }
            }
        ],
        "variable": [
            {"key": "base_url", "value": "https://api.example.com"}
        ]
    }"#;

    #[test]
    fn test_import_postman_v21() {
        let importer = PostmanImporter;
        let collection = importer.import(SAMPLE).unwrap();

        assert_eq!(collection.name, "Sample API");
        assert_eq!(collection.description.as_deref(), Some("A sample"));
        assert_eq!(collection.request_count(), 2);
        assert_eq!(collection.variables.len(), 1);
        assert_eq!(collection.variables[0].key, "base_url");

        // Check nested folder structure
        assert_eq!(collection.items.len(), 2);
        match &collection.items[0] {
            CollectionItem::Folder { name, children, .. } => {
                assert_eq!(name, "Users");
                assert_eq!(children.len(), 1);
            }
            _ => panic!("Expected folder"),
        }
    }

    #[test]
    fn test_import_with_bearer_auth() {
        let json = r#"{
            "info": {"name": "Auth", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
            "item": [{
                "name": "Get",
                "request": {
                    "method": "GET",
                    "url": "https://api.example.com",
                    "auth": {
                        "type": "bearer",
                        "bearer": [{"key": "token", "value": "abc123"}]
                    }
                }
            }]
        }"#;
        let collection = PostmanImporter.import(json).unwrap();
        match &collection.items[0] {
            CollectionItem::Request { request, .. } => {
                let auth = request.auth.as_ref().unwrap();
                assert_eq!(auth.config.get("token"), Some(&"abc123".to_string()));
            }
            _ => panic!("Expected request"),
        }
    }
}
