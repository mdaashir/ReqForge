//! Bruno collection importer
//!
//! Parses Bruno `.bru` files (text DSL) into a ReqForge `Collection`.
//! Each imported string is treated as a single `.bru` request — the
//! importing UI is responsible for iterating a folder of files and
//! grouping them into folders if needed.
//!
//! Bruno spec: https://docs.usebruno.com/bru-spec/

use crate::collection::{Collection, CollectionItem};
use crate::error::{Error, Result};
use crate::import::Importer;
use crate::request::{Auth, AuthType, Body, BodyMode, HttpMethod, KeyValue, Request};
use std::str::FromStr;

/// Top-level collection name when a single `.bru` is imported standalone.
const STANDALONE_COLLECTION: &str = "Bruno Import";

pub struct BrunoImporter;

impl Importer for BrunoImporter {
    fn format(&self) -> &'static str {
        "bruno"
    }

    fn file_extension(&self) -> Option<&'static str> {
        Some("bru")
    }

    fn import(&self, input: &str) -> Result<Collection> {
        let parsed = parse_bru(input)?;
        let request = parsed.into_request()?;
        Ok(Collection {
            id: uuid::Uuid::new_v4().to_string(),
            name: STANDALONE_COLLECTION.to_string(),
            description: None,
            auth: None,
            headers: Vec::new(),
            variables: Vec::new(),
            items: vec![CollectionItem::Request {
                id: request.id.clone(),
                name: request.name.clone(),
                request,
            }],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }
}

#[derive(Debug, Default)]
struct ParsedBru {
    name: Option<String>,
    method: Option<String>,
    url: Option<String>,
    body_mode: Option<String>,
    body_text: Option<String>,
    auth_type: Option<String>,
    auth_token: Option<String>,
    auth_user: Option<String>,
    auth_pass: Option<String>,
    auth_key: Option<String>,
    auth_value: Option<String>,
    auth_in: Option<String>,
    headers: Vec<KeyValue>,
    query: Vec<KeyValue>,
    path_vars: Vec<KeyValue>,
    description: Option<String>,
}

impl ParsedBru {
    fn into_request(self) -> Result<Request> {
        let method = self
            .method
            .as_deref()
            .and_then(|m| HttpMethod::from_str(m).ok())
            .unwrap_or(HttpMethod::Get);

        let body = build_body(self.body_mode.as_deref(), self.body_text.as_deref())?;
        let auth = build_auth(&self);

        let name = self
            .name
            .clone()
            .unwrap_or_else(|| self.url.clone().unwrap_or_else(|| "Untitled".to_string()));

        Ok(Request {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            method,
            url: self.url.unwrap_or_default(),
            headers: self.headers,
            params: self.query,
            body,
            auth,
            settings: Default::default(),
            pre_request_script: None,
            post_response_script: None,
            test_script: None,
            description: self.description,
        })
    }
}

fn build_body(mode: Option<&str>, text: Option<&str>) -> Result<Body> {
    let text = text.unwrap_or("").to_string();
    let (content_type, body_mode) = match mode.unwrap_or("none") {
        "json" => (Some("application/json".to_string()), BodyMode::Json),
        "xml" => (Some("application/xml".to_string()), BodyMode::Xml),
        "text" => (Some("text/plain".to_string()), BodyMode::Text),
        "formUrlEncoded" => (
            Some("application/x-www-form-urlencoded".to_string()),
            BodyMode::Form,
        ),
        "multipartForm" => (Some("multipart/form-data".to_string()), BodyMode::Multipart),
        "graphql" => (Some("application/json".to_string()), BodyMode::Graphql),
        "none" | "" => (None, BodyMode::None),
        other => {
            return Err(Error::other(format!(
                "Unsupported Bruno body type: {}",
                other
            )));
        }
    };
    Ok(Body {
        mode: body_mode,
        content_type,
        content: text,
    })
}

fn build_auth(parsed: &ParsedBru) -> Option<Auth> {
    let auth_type = parsed.auth_type.as_deref().unwrap_or("none");
    let mut config = std::collections::HashMap::new();
    let atype = match auth_type {
        "bearer" => {
            config.insert(
                "token".to_string(),
                parsed.auth_token.clone().unwrap_or_default(),
            );
            config.insert("prefix".to_string(), "Bearer".to_string());
            AuthType::Bearer
        }
        "basic" => {
            config.insert(
                "username".to_string(),
                parsed.auth_user.clone().unwrap_or_default(),
            );
            config.insert(
                "password".to_string(),
                parsed.auth_pass.clone().unwrap_or_default(),
            );
            AuthType::Basic
        }
        "apikey" => {
            config.insert(
                "key".to_string(),
                parsed.auth_key.clone().unwrap_or_default(),
            );
            config.insert(
                "value".to_string(),
                parsed.auth_value.clone().unwrap_or_default(),
            );
            config.insert(
                "location".to_string(),
                match parsed.auth_in.as_deref() {
                    Some("queryParams") => "query".to_string(),
                    Some("cookie") => "cookie".to_string(),
                    _ => "header".to_string(),
                },
            );
            AuthType::ApiKey
        }
        _ => return None,
    };
    Some(Auth {
        auth_type: atype,
        config,
    })
}

/// Parse a `.bru` file. The format is a series of named blocks where each
/// block is `name { ... }` with key-value lines inside.
fn parse_bru(input: &str) -> Result<ParsedBru> {
    let mut out = ParsedBru::default();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Skip blanks and comments
        if trimmed.is_empty() || trimmed.starts_with("//") {
            i += 1;
            continue;
        }

        // Look for a block opener: "name {" or "name:variant {"
        let block_open = find_block_open(trimmed);
        let (header, body_start) = match block_open {
            Some(h) => h,
            None => {
                i += 1;
                continue;
            }
        };

        // Find matching close brace at same indent level
        let close_idx = find_block_close(&lines, i + body_start);
        let close = match close_idx {
            Some(c) => c,
            None => {
                return Err(Error::other(format!(
                    "Unclosed Bruno block `{}` at line {}",
                    header,
                    i + 1
                )));
            }
        };

        let inner = &lines[(i + body_start)..close];
        apply_block(&mut out, &header, inner);

        i = close + 1;
    }

    Ok(out)
}

/// If `line` looks like `<header> {`, return `(header, lines_to_consume)`.
/// `lines_to_consume` is 1 when the brace is on the same line.
fn find_block_open(trimmed: &str) -> Option<(String, usize)> {
    if !trimmed.ends_with('{') {
        return None;
    }
    let header = trimmed.trim_end_matches('{').trim();
    Some((header.to_string(), 1))
}

fn find_block_close(lines: &[&str], start: usize) -> Option<usize> {
    // The block's opening `{` was already consumed by `find_block_open`, so we
    // start at depth = 1 and look for the matching `}`. Track nesting so JSON
    // bodies and other braced content don't terminate the block prematurely.
    let mut depth: i32 = 1;
    for (offset, line) in lines.iter().enumerate().skip(start) {
        for ch in line.chars() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(offset);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn apply_block(out: &mut ParsedBru, header: &str, lines: &[&str]) {
    let lower = header.to_lowercase();
    match lower.as_str() {
        "meta" => {
            for line in lines {
                if let Some((k, v)) = split_kv(line) {
                    match k.to_lowercase().as_str() {
                        "name" => out.name = Some(v.to_string()),
                        "type" => { /* http, grpc, etc. — ignore for now */ }
                        _ => {}
                    }
                }
            }
        }
        m @ ("get" | "post" | "put" | "patch" | "delete" | "head" | "options") => {
            out.method = Some(m.to_uppercase());
            for line in lines {
                if let Some((k, v)) = split_kv(line) {
                    match k.to_lowercase().as_str() {
                        "url" => out.url = Some(v.to_string()),
                        "body" => out.body_mode = Some(v.to_string()),
                        "auth" => out.auth_type = Some(v.to_string()),
                        _ => {}
                    }
                }
            }
        }
        "headers" => {
            for line in lines {
                if let Some((k, v)) = split_kv(line) {
                    out.headers.push(KeyValue {
                        key: k.to_string(),
                        value: v.to_string(),
                        enabled: true,
                        description: None,
                    });
                }
            }
        }
        "params:query" | "params:queryparams" => {
            for line in lines {
                if let Some((k, v)) = split_kv(line) {
                    out.query.push(KeyValue {
                        key: k.to_string(),
                        value: v.to_string(),
                        enabled: true,
                        description: None,
                    });
                }
            }
        }
        "params:path" | "params:pathparams" => {
            for line in lines {
                if let Some((k, v)) = split_kv(line) {
                    out.path_vars.push(KeyValue {
                        key: k.to_string(),
                        value: v.to_string(),
                        enabled: true,
                        description: None,
                    });
                }
            }
        }
        "auth" => {
            for line in lines {
                if let Some((k, v)) = split_kv(line) {
                    match k.to_lowercase().as_str() {
                        "username" => out.auth_user = Some(v.to_string()),
                        "password" => out.auth_pass = Some(v.to_string()),
                        "token" => out.auth_token = Some(v.to_string()),
                        "key" => out.auth_key = Some(v.to_string()),
                        "value" => out.auth_value = Some(v.to_string()),
                        "in" => out.auth_in = Some(v.to_string()),
                        _ => {}
                    }
                }
            }
        }
        _ => {
            // Body block: everything is content. Strip per-line indent so the
            // saved body matches what the user wrote (no leading 2-spaces).
            if lower.starts_with("body") {
                let text = lines
                    .iter()
                    .map(|l| l.trim())
                    .collect::<Vec<_>>()
                    .join("\n");
                out.body_text = Some(text);
            }
        }
    }
}

fn split_kv(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return None;
    }
    let idx = trimmed.find(':')?;
    let key = trimmed[..idx].trim();
    let value = trimmed[idx + 1..].trim().trim_matches('"');
    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
meta {
  name: Get Users
  type: http
  seq: 1
}

get {
  url: https://api.example.com/users
  body: none
  auth: bearer
}

headers {
  Accept: application/json
  X-Trace: req-123
}

params:query {
  limit: 10
  active: true
}

auth {
  token: abc.def.ghi
}
"#;

    #[test]
    fn test_parse_basic_bru() {
        let parsed = parse_bru(SAMPLE).unwrap();
        assert_eq!(parsed.name.as_deref(), Some("Get Users"));
        assert_eq!(parsed.method.as_deref(), Some("GET"));
        assert_eq!(parsed.url.as_deref(), Some("https://api.example.com/users"));
        assert_eq!(parsed.headers.len(), 2);
        assert_eq!(parsed.query.len(), 2);
        assert_eq!(parsed.auth_token.as_deref(), Some("abc.def.ghi"));
        assert_eq!(parsed.auth_type.as_deref(), Some("bearer"));
    }

    #[test]
    fn test_import_returns_collection() {
        let imp = BrunoImporter;
        let col = imp.import(SAMPLE).unwrap();
        assert_eq!(col.name, "Bruno Import");
        assert_eq!(col.items.len(), 1);
        match &col.items[0] {
            CollectionItem::Request { name, request, .. } => {
                assert_eq!(name, "Get Users");
                assert_eq!(request.method, HttpMethod::Get);
                assert!(request.auth.is_some());
            }
            _ => panic!("expected request"),
        }
    }

    #[test]
    fn test_post_with_json_body() {
        let bru = r#"
meta { name: Create }
post {
  url: https://api.example.com/users
  body: json
  auth: none
}

body:json {
  {"name":"x"}
}
"#;
        let parsed = parse_bru(bru).unwrap();
        assert_eq!(parsed.method.as_deref(), Some("POST"));
        assert_eq!(parsed.body_mode.as_deref(), Some("json"));
        assert_eq!(parsed.body_text.as_deref(), Some("{\"name\":\"x\"}"));
    }
}
