//! Starter collections bundled with ReqForge.
//!
//! Seeded into a new workspace on first launch so the user has something
//! to click on immediately.

use crate::collection::{Collection, CollectionItem};
use crate::request::{Auth, AuthType, Body, BodyMode, HttpMethod, KeyValue, Request};

fn req(name: &str, method: HttpMethod, url: &str, body: Option<Body>) -> CollectionItem {
    CollectionItem::Request {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        request: Request {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            method,
            url: url.to_string(),
            headers: Vec::new(),
            params: Vec::new(),
            body: body.unwrap_or_default(),
            auth: None,
            settings: Default::default(),
            pre_request_script: None,
            post_response_script: None,
            test_script: None,
            description: None,
        },
    }
}

fn json_body(s: &str) -> Body {
    Body {
        mode: BodyMode::Json,
        content_type: Some("application/json".to_string()),
        content: s.to_string(),
    }
}

/// Returns the list of starter collections that ship with ReqForge.
pub fn bundled_collections() -> Vec<Collection> {
    vec![
        Collection {
            id: uuid::Uuid::new_v4().to_string(),
            name: "JSONPlaceholder".to_string(),
            description: Some("Free fake API for testing and prototyping.".to_string()),
            auth: None,
            headers: Vec::new(),
            variables: vec![KeyValue {
                key: "baseUrl".to_string(),
                value: "https://jsonplaceholder.typicode.com".to_string(),
                enabled: true,
                description: Some("Base URL for all JSONPlaceholder requests".to_string()),
            }],
            items: vec![
                CollectionItem::Folder {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "Posts".to_string(),
                    description: None,
                    children: vec![
                        req("List posts", HttpMethod::Get, "{{baseUrl}}/posts", None),
                        req("Get post #1", HttpMethod::Get, "{{baseUrl}}/posts/1", None),
                        req(
                            "Create post",
                            HttpMethod::Post,
                            "{{baseUrl}}/posts",
                            Some(json_body(
                                "{\n  \"title\": \"hello\",\n  \"body\": \"world\",\n  \"userId\": 1\n}",
                            )),
                        ),
                        req("Delete post #1", HttpMethod::Delete, "{{baseUrl}}/posts/1", None),
                    ],
                    auth: None,
                },
                CollectionItem::Folder {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "Users".to_string(),
                    description: None,
                    children: vec![
                        req("List users", HttpMethod::Get, "{{baseUrl}}/users", None),
                        req("Get user #1", HttpMethod::Get, "{{baseUrl}}/users/1", None),
                    ],
                    auth: None,
                },
            ],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
        Collection {
            id: uuid::Uuid::new_v4().to_string(),
            name: "httpbin".to_string(),
            description: Some("HTTP Request & Response service.".to_string()),
            auth: None,
            headers: Vec::new(),
            variables: vec![KeyValue {
                key: "baseUrl".to_string(),
                value: "https://httpbin.org".to_string(),
                enabled: true,
                description: None,
            }],
            items: vec![
                req("GET (any)", HttpMethod::Get, "{{baseUrl}}/get", None),
                req(
                    "POST JSON",
                    HttpMethod::Post,
                    "{{baseUrl}}/post",
                    Some(json_body(r#"{"hello":"world"}"#)),
                ),
                req(
                    "Headers echo",
                    HttpMethod::Get,
                    "{{baseUrl}}/headers",
                    None,
                ),
                req(
                    "Status 200",
                    HttpMethod::Get,
                    "{{baseUrl}}/status/200",
                    None,
                ),
                req(
                    "Status 404",
                    HttpMethod::Get,
                    "{{baseUrl}}/status/404",
                    None,
                ),
            ],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
        Collection {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Public APIs".to_string(),
            description: Some("A grab-bag of free public APIs.".to_string()),
            auth: None,
            headers: Vec::new(),
            variables: Vec::new(),
            items: vec![
                req(
                    "GitHub Zen",
                    HttpMethod::Get,
                    "https://api.github.com/zen",
                    None,
                ),
                req(
                    "Cat fact",
                    HttpMethod::Get,
                    "https://catfact.ninja/fact",
                    None,
                ),
                req(
                    "Random user",
                    HttpMethod::Get,
                    "https://randomuser.me/api/",
                    None,
                ),
                req(
                    "IP echo",
                    HttpMethod::Get,
                    "https://api.ipify.org?format=json",
                    None,
                ),
            ],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    ]
}

/// Seed the workspace with bundled collections if no collections exist yet.
/// Returns the number of collections written.
pub async fn seed_into(
    storage: &crate::collection::CollectionStorage,
) -> Result<usize, crate::error::Error> {
    let existing = storage.list_ids().await?;
    if !existing.is_empty() {
        return Ok(0);
    }
    let mut count = 0;
    for col in bundled_collections() {
        storage.save(&col).await?;
        count += 1;
    }
    Ok(count)
}

// `Auth` is intentionally unused in the starter data so the import stays
// dead-simple. Re-export to silence dead_code on the Auth struct when this
// module is included in the build.
#[allow(dead_code)]
fn _unused() {
    let _ = Auth {
        auth_type: AuthType::None,
        config: Default::default(),
    };
}
