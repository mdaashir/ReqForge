use crate::request::{Auth, KeyValue, Request};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A single item inside a collection: either a folder or a request.
///
/// The `Request` variant is intentionally large because it carries every
/// detail needed to fire an HTTP request without further lookups. The
/// size mismatch with `Folder` is a deliberate API tradeoff — serialising
/// entire requests inline keeps the collection model self-contained.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CollectionItem {
    Folder {
        id: String,
        name: String,
        description: Option<String>,
        #[serde(default)]
        children: Vec<CollectionItem>,
        #[serde(default)]
        auth: Option<Auth>,
    },
    Request {
        id: String,
        name: String,
        #[serde(flatten)]
        request: Request,
    },
}

impl CollectionItem {
    pub fn id(&self) -> &str {
        match self {
            CollectionItem::Folder { id, .. } => id,
            CollectionItem::Request { id, .. } => id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            CollectionItem::Folder { name, .. } => name,
            CollectionItem::Request { name, .. } => name,
        }
    }

    pub fn is_folder(&self) -> bool {
        matches!(self, CollectionItem::Folder { .. })
    }

    pub fn is_request(&self) -> bool {
        matches!(self, CollectionItem::Request { .. })
    }
}

/// A ReqForge collection: a named tree of requests/folders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub auth: Option<Auth>,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub variables: Vec<KeyValue>,
    #[serde(default)]
    pub items: Vec<CollectionItem>,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

impl Collection {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: None,
            auth: None,
            headers: Vec::new(),
            variables: Vec::new(),
            items: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Recursively count requests in this collection
    pub fn request_count(&self) -> usize {
        fn count(items: &[CollectionItem]) -> usize {
            items
                .iter()
                .map(|item| match item {
                    CollectionItem::Request { .. } => 1,
                    CollectionItem::Folder { children, .. } => count(children),
                })
                .sum()
        }
        count(&self.items)
    }

    /// Find a request by id (recursive)
    pub fn find_request(&self, id: &str) -> Option<&Request> {
        fn find<'a>(items: &'a [CollectionItem], id: &str) -> Option<&'a Request> {
            for item in items {
                match item {
                    CollectionItem::Request {
                        id: rid, request, ..
                    } if rid == id => {
                        return Some(request);
                    }
                    CollectionItem::Request { .. } => {}
                    CollectionItem::Folder { children, .. } => {
                        if let Some(found) = find(children, id) {
                            return Some(found);
                        }
                    }
                }
            }
            None
        }
        find(&self.items, id)
    }

    /// Find a request by id (recursive, mutable)
    pub fn find_request_mut(&mut self, id: &str) -> Option<&mut Request> {
        fn find<'a>(items: &'a mut [CollectionItem], id: &str) -> Option<&'a mut Request> {
            for item in items {
                match item {
                    CollectionItem::Request {
                        id: rid, request, ..
                    } if rid == id => {
                        return Some(request);
                    }
                    CollectionItem::Request { .. } => {}
                    CollectionItem::Folder { children, .. } => {
                        if let Some(found) = find(children, id) {
                            return Some(found);
                        }
                    }
                }
            }
            None
        }
        find(&mut self.items, id)
    }
}

/// A map of collection id -> collection (for in-memory workspace state)
pub type CollectionMap = BTreeMap<String, Collection>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::HttpMethod;

    #[test]
    fn test_new_collection() {
        let col = Collection::new("Test API");
        assert_eq!(col.name, "Test API");
        assert_eq!(col.request_count(), 0);
    }

    #[test]
    fn test_request_count_with_folders() {
        let mut col = Collection::new("Test");
        col.items.push(CollectionItem::Folder {
            id: "f1".to_string(),
            name: "Folder 1".to_string(),
            description: None,
            children: vec![
                CollectionItem::Request {
                    id: "r1".to_string(),
                    name: "Get Users".to_string(),
                    request: Request::new(HttpMethod::Get, "https://api.example.com/users"),
                },
                CollectionItem::Request {
                    id: "r2".to_string(),
                    name: "Get Posts".to_string(),
                    request: Request::new(HttpMethod::Get, "https://api.example.com/posts"),
                },
            ],
            auth: None,
        });
        col.items.push(CollectionItem::Request {
            id: "r3".to_string(),
            name: "Get Comments".to_string(),
            request: Request::new(HttpMethod::Get, "https://api.example.com/comments"),
        });

        assert_eq!(col.request_count(), 3);
    }

    #[test]
    fn test_find_request() {
        let mut col = Collection::new("Test");
        let request = Request::new(HttpMethod::Get, "https://api.example.com/users");
        col.items.push(CollectionItem::Request {
            id: "abc".to_string(),
            name: "Get Users".to_string(),
            request,
        });

        let found = col.find_request("abc");
        assert!(found.is_some());
        assert_eq!(found.unwrap().url, "https://api.example.com/users");
    }
}
