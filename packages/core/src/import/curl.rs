//! cURL command importer
//!
//! Parses a single cURL command line into a ReqForge `Collection` with one request.
//! Supports common flags: `-X`, `-H`, `-d/--data`, `--data-raw`, `-u`, `-k`,
//! `--cookie`, `-G`, `--url`, `-L`.

use crate::collection::{Collection, CollectionItem};
use crate::error::{Error, Result};
use crate::import::Importer;
use crate::request::{Auth as CoreAuth, AuthType as CoreAuthType, Body, BodyMode, HttpMethod, KeyValue, Request};
use uuid::Uuid;

pub struct CurlImporter;

impl CurlImporter {
    pub fn new() -> Self {
        Self
    }

    /// Parse a cURL command string into a single-request Collection
    pub fn parse(&self, input: &str) -> Result<Collection> {
        let args = tokenize(input);
        if args.is_empty() || args[0] != "curl" {
            return Err(Error::other("Not a cURL command"));
        }

        let mut method: Option<String> = None;
        let mut url: Option<String> = None;
        let mut headers: Vec<KeyValue> = Vec::new();
        let mut body = Body::default();
        let mut basic_auth: Option<(String, String)> = None;
        let mut insecure = false;
        let mut follow_redirects = true;
        let mut use_get_for_data = false;

        let mut iter = args.into_iter().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-X" | "--request" => {
                    method = iter.next();
                }
                "-H" | "--header" => {
                    if let Some(h) = iter.next() {
                        if let Some((k, v)) = h.split_once(':') {
                            headers.push(KeyValue {
                                key: k.trim().to_string(),
                                value: v.trim().to_string(),
                                enabled: true,
                                description: None,
                            });
                        }
                    }
                }
                "-d" | "--data" | "--data-raw" | "--data-binary" => {
                    if let Some(value) = iter.next() {
                        body.content.push_str(&value);
                        body.mode = BodyMode::Text;
                        body.content_type = Some("application/x-www-form-urlencoded".to_string());
                        if method.is_none() {
                            method = Some("POST".to_string());
                        }
                    }
                }
                "-u" | "--user" => {
                    if let Some(userinfo) = iter.next() {
                        if let Some((u, p)) = userinfo.split_once(':') {
                            basic_auth = Some((u.to_string(), p.to_string()));
                        } else {
                            basic_auth = Some((userinfo, String::new()));
                        }
                    }
                }
                "-k" | "--insecure" => {
                    insecure = true;
                }
                "-L" | "--location" => {
                    follow_redirects = true;
                }
                "--no-location" => {
                    follow_redirects = false;
                }
                "-G" | "--get" => {
                    use_get_for_data = true;
                    if method.is_none() {
                        method = Some("GET".to_string());
                    }
                }
                "--url" => {
                    url = iter.next();
                }
                "--cookie" | "-b" => {
                    if let Some(cookie) = iter.next() {
                        headers.push(KeyValue {
                            key: "Cookie".to_string(),
                            value: cookie,
                            enabled: true,
                            description: None,
                        });
                    }
                }
                _ => {
                    // Bare positional URL (skip flags already handled)
                    if !arg.starts_with('-') && url.is_none() {
                        url = Some(arg);
                    }
                }
            }
        }

        let url = url.ok_or_else(|| Error::other("cURL command has no URL"))?;

        let method = method
            .unwrap_or_else(|| {
                if !body.content.is_empty() {
                    "POST".to_string()
                } else {
                    "GET".to_string()
                }
            })
            .parse::<HttpMethod>()
            .map_err(|e| Error::other(format!("Invalid HTTP method: {}", e)))?;

        // Convert body to query params if -G was used
        let (final_url, final_params, final_method) = if use_get_for_data && !body.content.is_empty() {
            let sep = if url.contains('?') { '&' } else { '?' };
            let new_url = format!("{}{}{}", url, sep, body.content);
            let mut params = Vec::new();
            for pair in body.content.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    params.push(KeyValue {
                        key: k.to_string(),
                        value: v.to_string(),
                        enabled: true,
                        description: None,
                    });
                }
            }
            (new_url, params, HttpMethod::Get)
        } else {
            (url, Vec::new(), method)
        };

        let auth = basic_auth.map(|(u, p)| CoreAuth {
            auth_type: CoreAuthType::Basic,
            config: [
                ("username".to_string(), u),
                ("password".to_string(), p),
            ]
            .into_iter()
            .collect(),
        });

        let request = Request {
            id: Uuid::new_v4().to_string(),
            name: "Imported cURL".to_string(),
            method: final_method,
            url: final_url,
            headers,
            params: final_params,
            body: if body.content.is_empty() {
                Body::default()
            } else {
                body
            },
            auth,
            settings: crate::request::RequestSettings {
                follow_redirects,
                verify_ssl: !insecure,
                ..Default::default()
            },
            pre_request_script: None,
            post_response_script: None,
            test_script: None,
            description: Some("Imported from cURL".to_string()),
        };

        Ok(Collection {
            id: Uuid::new_v4().to_string(),
            name: "Imported cURL".to_string(),
            description: Some("Single request imported from a cURL command".to_string()),
            auth: None,
            headers: Vec::new(),
            variables: Vec::new(),
            items: vec![CollectionItem::Request {
                id: Uuid::new_v4().to_string(),
                name: "Imported cURL".to_string(),
                request,
            }],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }
}

impl Default for CurlImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Importer for CurlImporter {
    fn format(&self) -> &'static str {
        "curl"
    }

    fn import(&self, input: &str) -> Result<Collection> {
        self.parse(input)
    }
}

/// Tokenize a cURL command line, respecting single and double quotes.
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut had_content = false;

    for c in input.chars() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
                had_content = true;
            }
            '"' if !in_single => {
                in_double = !in_double;
                had_content = true;
            }
            '\\' if in_double => {
                if let Some(next) = input.chars().next() {
                    current.push(next);
                }
                had_content = true;
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if had_content {
                    tokens.push(std::mem::take(&mut current));
                    had_content = false;
                }
            }
            c => {
                current.push(c);
                had_content = true;
            }
        }
    }
    if had_content {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::CollectionItem;

    #[test]
    fn test_import_simple_get() {
        let importer = CurlImporter;
        let collection = importer.import("curl https://api.example.com/users").unwrap();

        assert_eq!(collection.items.len(), 1);
        match &collection.items[0] {
            CollectionItem::Request { request, .. } => {
                assert_eq!(request.method, HttpMethod::Get);
                assert_eq!(request.url, "https://api.example.com/users");
            }
            _ => panic!("Expected request"),
        }
    }

    #[test]
    fn test_import_post_with_data() {
        let importer = CurlImporter;
        let cmd = r#"curl -X POST -H "Content-Type: application/json" -d '{"name":"Alice"}' https://api.example.com/users"#;
        let collection = importer.import(cmd).unwrap();

        match &collection.items[0] {
            CollectionItem::Request { request, .. } => {
                assert_eq!(request.method, HttpMethod::Post);
                assert_eq!(request.body.content, r#"{"name":"Alice"}"#);
                assert_eq!(request.headers.len(), 1);
                assert_eq!(request.headers[0].key, "Content-Type");
            }
            _ => panic!("Expected request"),
        }
    }

    #[test]
    fn test_import_basic_auth() {
        let importer = CurlImporter;
        let cmd = "curl -u alice:secret https://api.example.com";
        let collection = importer.import(cmd).unwrap();

        match &collection.items[0] {
            CollectionItem::Request { request, .. } => {
                let auth = request.auth.as_ref().unwrap();
                assert_eq!(auth.config.get("username").unwrap(), "alice");
                assert_eq!(auth.config.get("password").unwrap(), "secret");
            }
            _ => panic!("Expected request"),
        }
    }

    #[test]
    fn test_import_with_get_data() {
        let importer = CurlImporter;
        let cmd = "curl -G -d 'q=hello&page=1' https://api.example.com/search";
        let collection = importer.import(cmd).unwrap();

        match &collection.items[0] {
            CollectionItem::Request { request, .. } => {
                assert_eq!(request.method, HttpMethod::Get);
                assert!(request.url.contains("q=hello"));
                assert!(request.url.contains("page=1"));
            }
            _ => panic!("Expected request"),
        }
    }

    #[test]
    fn test_import_with_insecure() {
        let importer = CurlImporter;
        let cmd = "curl -k https://self-signed.example.com";
        let collection = importer.import(cmd).unwrap();

        match &collection.items[0] {
            CollectionItem::Request { request, .. } => {
                assert!(!request.settings.verify_ssl);
            }
            _ => panic!("Expected request"),
        }
    }

    #[test]
    fn test_tokenize_with_quotes() {
        let tokens = tokenize(r#"curl -H "Content-Type: application/json" -d '{"name":1}' https://x"#);
        assert_eq!(
            tokens,
            vec![
                "curl",
                "-H",
                "Content-Type: application/json",
                "-d",
                r#"{"name":1}"#,
                "https://x"
            ]
        );
    }
}
