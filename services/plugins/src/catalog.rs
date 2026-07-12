//! Plugin catalog — a file-backed registry of community plugins.
//!
//! The registry is a JSON array loaded at startup. Each entry describes
//! one plugin: name, version, author, download URL, tags, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// SPDX license identifier.
    #[serde(default)]
    pub license: Option<String>,
    /// Tags / categories for browsing.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Base URL for downloading the signed `.wasm` + `.sig`.
    pub download_url: String,
    /// SHA-256 of the `.wasm` file (hex).
    #[serde(default)]
    pub checksum: Option<String>,
    /// Unix timestamp when this entry was published.
    pub published_at: i64,
    /// Version history for upgrade support.
    #[serde(default)]
    pub versions: Vec<PluginVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginVersion {
    pub version: String,
    pub download_url: String,
    #[serde(default)]
    pub checksum: Option<String>,
    pub published_at: i64,
    #[serde(default)]
    pub changelog: Option<String>,
}

/// In-memory catalog. Holds the full registry in a hash map so lookups
/// are O(1) and listing is O(n).
#[derive(Debug)]
pub struct Catalog {
    items: HashMap<String, PluginEntry>,
    list: Vec<PluginEntry>,
}

impl Catalog {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let entries: Vec<PluginEntry> = serde_json::from_str(&content)?;
        let mut items = HashMap::new();
        for entry in entries.iter() {
            items.insert(entry.id.clone(), entry.clone());
        }
        Ok(Self {
            items,
            list: entries,
        })
    }

    pub fn all(&self) -> &[PluginEntry] {
        &self.list
    }

    pub fn get(&self, id: &str) -> Option<&PluginEntry> {
        self.items.get(id)
    }

    pub fn search(&self, query: &str, tag: Option<&str>) -> Vec<&PluginEntry> {
        let q = query.to_lowercase();
        self.list
            .iter()
            .filter(|e| {
                let matches_query = q.is_empty()
                    || e.name.to_lowercase().contains(&q)
                    || e.id.to_lowercase().contains(&q)
                    || e.description
                        .as_deref()
                        .map(|d| d.to_lowercase().contains(&q))
                        .unwrap_or(false);
                let matches_tag = tag.map_or(true, |t| e.tags.iter().any(|et| et == t));
                matches_query && matches_tag
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_catalog() -> Catalog {
        let entries = vec![
            PluginEntry {
                id: "header-juggler".into(),
                name: "Header Juggler".into(),
                version: "0.1.0".into(),
                author: Some("reqforge".into()),
                description: Some("Mutate request headers with regex.".into()),
                license: Some("MIT".into()),
                tags: vec!["transform".into(), "headers".into()],
                download_url: "https://example.com/header-juggler.wasm".into(),
                checksum: None,
                published_at: 1700000000,
                versions: vec![],
            },
            PluginEntry {
                id: "morgan".into(),
                name: "Morgan".into(),
                version: "0.1.0".into(),
                author: None,
                description: Some("Request logging middleware.".into()),
                license: Some("Apache-2.0".into()),
                tags: vec!["logging".into(), "debug".into()],
                download_url: "https://example.com/morgan.wasm".into(),
                checksum: None,
                published_at: 1700000001,
                versions: vec![],
            },
        ];
        let mut items = HashMap::new();
        for e in entries.iter() {
            items.insert(e.id.clone(), e.clone());
        }
        Catalog {
            items,
            list: entries,
        }
    }

    #[test]
    fn test_get() {
        let c = test_catalog();
        assert!(c.get("header-juggler").is_some());
        assert!(c.get("nonexistent").is_none());
    }

    #[test]
    fn test_search_by_name() {
        let c = test_catalog();
        let results = c.search("juggler", None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "header-juggler");
    }

    #[test]
    fn test_search_by_tag() {
        let c = test_catalog();
        let results = c.search("", Some("headers"));
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_matches_description() {
        let c = test_catalog();
        let results = c.search("logging", None);
        assert_eq!(results.len(), 1);
    }
}
