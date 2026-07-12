//! Importers for external collection formats
//!
//! Currently supports:
//! - Postman v2.1 collections
//! - cURL commands

pub mod bruno;
pub mod curl;
pub mod har;
pub mod insomnia;
pub mod openapi;
pub mod postman;

pub use bruno::BrunoImporter;
pub use curl::CurlImporter;
pub use insomnia::InsomniaImporter;
pub use postman::{PostmanImporter, PostmanV21};

use crate::collection::Collection;
use crate::environment::Environment;
use crate::error::Result;

/// Trait implemented by all importers
pub trait Importer: Send + Sync {
    /// The format this importer handles (e.g., "postman", "curl")
    fn format(&self) -> &'static str;

    /// Parse input and return a ReqForge Collection
    fn import(&self, input: &str) -> Result<Collection>;

    /// Optional file extension this importer recognises (e.g., "json", "txt")
    fn file_extension(&self) -> Option<&'static str> {
        None
    }

    /// Some formats (Insomnia) bundle environments alongside requests.
    /// Default impl returns an empty vec.
    fn import_environments(&self, _input: &str) -> Result<Vec<Environment>> {
        Ok(Vec::new())
    }
}

/// Detect the most likely importer for a given input string
pub fn detect_importer(input: &str) -> Option<Box<dyn Importer>> {
    let trimmed = input.trim();
    if trimmed.starts_with("curl ") {
        return Some(Box::new(CurlImporter));
    } else if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if value.get("info").and_then(|v| v.get("schema")).is_some()
            && value
                .get("info")
                .and_then(|v| v.get("schema"))
                .and_then(|s| s.as_str())
                .map(|s| s.contains("v2.1"))
                .unwrap_or(false)
        {
            return Some(Box::new(PostmanImporter));
        }
    }
    None
}
