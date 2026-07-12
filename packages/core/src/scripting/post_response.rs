//! Post-response scripting — JSON path extraction + assertions.
//!
//! Simpler alternative to a full JS engine: rules that run after the
//! response arrives and can:
//! - Extract values from the JSON body and store them as environment vars
//! - Run assertions against status, headers, and body content
//! - Capture new headers/body for the response viewer
//!
//! This is the pragmatic layer that ships by default. Full JS-based
//! post-response scripting lives behind `script-engine`.

use crate::error::Result;
use crate::request::Request;
use crate::request::Response;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Single extraction rule: pull a JSON path value and store it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionRule {
    /// JSON path expression (e.g. `data.user.id`).
    pub path: String,
    /// Environment variable name to store the extracted value under.
    pub store_as: String,
}

/// Single assertion rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionRule {
    pub name: String,
    pub kind: AssertionKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssertionKind {
    /// HTTP status code equals the expected value.
    StatusEq(u16),
    /// Response body contains a substring.
    BodyContains(String),
    /// Response header contains the expected value.
    HeaderEq { header: String, expected: String },
    /// JSON path returns the expected value.
    JsonPathEq { path: String, expected: String },
    /// Response time under N ms (uses timing.total_ms).
    LatencyUnder(u64),
}

/// Post-response script: a set of extraction and assertion rules.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PostResponseScript {
    #[serde(default)]
    pub extractions: Vec<ExtractionRule>,
    #[serde(default)]
    pub assertions: Vec<AssertionRule>,
}

/// Result of running post-response extractions.
#[derive(Debug, Clone, Default)]
pub struct ExtractionResult {
    pub values: HashMap<String, String>,
}

/// Result of running post-response assertions.
#[derive(Debug, Clone, Default)]
pub struct AssertionOutcome {
    pub name: String,
    pub passed: bool,
    pub message: String,
}

/// Result of the full post-response run.
#[derive(Debug, Clone, Default)]
pub struct PostResponseRunResult {
    pub extractions: ExtractionResult,
    pub assertions: Vec<AssertionOutcome>,
    pub logs: Vec<String>,
}

/// Run extractions against the response body and store into the
/// returned `HashMap`. The caller decides whether to apply these
/// to the environment store.
pub fn run_extractions(script: &PostResponseScript, body_text: &str) -> ExtractionResult {
    let mut out = ExtractionResult::default();
    let parsed: Option<serde_json::Value> = serde_json::from_str(body_text).ok();

    for rule in &script.extractions {
        let value = parsed
            .as_ref()
            .and_then(|v| extract_path(v, &rule.path))
            .unwrap_or_default();
        out.values.insert(rule.store_as.clone(), value);
    }

    out
}

/// Run assertions and return outcomes (no failure short-circuits — we
/// run them all so the UI can show the full picture).
pub fn run_assertions(
    script: &PostResponseScript,
    request: &Request,
    response: &Response,
) -> Vec<AssertionOutcome> {
    let body_text = String::from_utf8_lossy(&response.body.content).to_string();
    let parsed: Option<serde_json::Value> = serde_json::from_str(&body_text).ok();
    let mut out = Vec::new();

    for assertion in &script.assertions {
        let outcome = match &assertion.kind {
            AssertionKind::StatusEq(expected) => AssertionOutcome {
                name: assertion.name.clone(),
                passed: response.status == *expected,
                message: format!("status {} == {}", response.status, expected),
            },
            AssertionKind::BodyContains(needle) => AssertionOutcome {
                name: assertion.name.clone(),
                passed: body_text.contains(needle.as_str()),
                message: format!("body contains '{}'", needle),
            },
            AssertionKind::HeaderEq { header, expected } => AssertionOutcome {
                name: assertion.name.clone(),
                passed: response
                    .headers
                    .get(header)
                    .map(|v| v == expected)
                    .unwrap_or(false),
                message: format!("{} == {}", header, expected),
            },
            AssertionKind::JsonPathEq { path, expected } => {
                let actual = parsed
                    .as_ref()
                    .and_then(|v| extract_path(v, path))
                    .unwrap_or_default();
                AssertionOutcome {
                    name: assertion.name.clone(),
                    passed: actual == *expected,
                    message: format!("{} == {} (got {})", path, expected, actual),
                }
            }
            AssertionKind::LatencyUnder(max_ms) => AssertionOutcome {
                name: assertion.name.clone(),
                passed: response.timing.total_ms <= *max_ms,
                message: format!("{} ms ≤ {} ms", response.timing.total_ms, max_ms),
            },
        };
        out.push(outcome);
    }

    let _ = request; // reserved for future use
    out
}

/// Full post-response rule run: extracts + assertions, returns a unified result.
pub fn run_rules(
    script: &PostResponseScript,
    request: &Request,
    response: &Response,
) -> PostResponseRunResult {
    let body_text = String::from_utf8_lossy(&response.body.content).to_string();
    let extractions = run_extractions(script, &body_text);
    let assertions = run_assertions(script, request, response);
    let mut logs = Vec::new();
    for a in &assertions {
        logs.push(format!(
            "[{}] {} — {}",
            if a.passed { "PASS" } else { "FAIL" },
            a.name,
            a.message
        ));
    }
    PostResponseRunResult {
        extractions,
        assertions,
        logs,
    }
}

/// Minimal JSON path extractor: dot-separated keys, array indices as
/// `[*]` or `[0]`. Returns `None` for missing paths.
pub fn extract_path(value: &serde_json::Value, path: &str) -> Option<String> {
    let mut current = value;
    for segment in path.split('.') {
        // Split on bracket to handle "items[1]" -> key "items" + index "[1]"
        let (key_part, bracket_part) = match segment.find('[') {
            Some(pos) => {
                let (k, rest) = segment.split_at(pos);
                (k, Some(rest))
            }
            None => (segment, None),
        };

        if !key_part.is_empty() {
            current = current.get(key_part)?;
        }

        // Now process bracket parts — possibly multiple: [0][1]
        if let Some(rest) = bracket_part {
            let mut s = rest;
            while let Some(idx_str) = s.strip_prefix('[').and_then(|x| x.split(']').next()) {
                let idx: usize = idx_str.parse().ok()?;
                current = current.get(idx)?;
                // Move past the [idx] we just consumed.
                let after = &s[1 + idx_str.len() + 1..];
                s = if after.is_empty() { "" } else { after };
                if s.is_empty() {
                    break;
                }
            }
        }
    }
    Some(match current {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => current.to_string(),
    })
}

/// Convenience: extract values directly into a map of env vars.
pub fn extract_into_env(
    script: &PostResponseScript,
    body_text: &str,
) -> Result<HashMap<String, String>> {
    Ok(run_extractions(script, body_text).values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{HttpMethod, Request, Response, ResponseBody, ResponseTiming};

    fn req() -> Request {
        Request::new(HttpMethod::Get, "https://x.test")
    }

    fn resp_with(body: &str) -> Response {
        Response {
            status: 200,
            status_text: "OK".into(),
            headers: HashMap::from([("content-type".into(), "application/json".into())]),
            body: ResponseBody {
                content: body.as_bytes().to_vec(),
                content_type: Some("application/json".into()),
                is_text: true,
            },
            cookies: Vec::new(),
            timing: ResponseTiming::default(),
            size: Default::default(),
            url: "https://x.test".into(),
            protocol: "HTTP/1.1".into(),
        }
    }

    #[test]
    fn test_extract_simple_path() {
        let body = r#"{"data":{"user":{"id":42,"name":"ada"}}}"#;
        let val = extract_path(&serde_json::from_str(body).unwrap(), "data.user.id");
        assert_eq!(val, Some("42".into()));
    }

    #[test]
    fn test_extract_with_array_index() {
        let body = r#"{"items":[{"id":1},{"id":2}]}"#;
        let val = extract_path(&serde_json::from_str(body).unwrap(), "items[1].id");
        assert_eq!(val, Some("2".into()));
    }

    #[test]
    fn test_extract_missing_path_returns_none() {
        let body = r#"{"a":1}"#;
        let val = extract_path(&serde_json::from_str(body).unwrap(), "missing.path");
        assert_eq!(val, None);
    }

    #[test]
    fn test_extractions_stored() {
        let script = PostResponseScript {
            extractions: vec![
                ExtractionRule {
                    path: "token".into(),
                    store_as: "AUTH_TOKEN".into(),
                },
                ExtractionRule {
                    path: "user.id".into(),
                    store_as: "USER_ID".into(),
                },
            ],
            assertions: vec![],
        };
        let body = r#"{"token":"abc123","user":{"id":7,"name":"x"}}"#;
        let result = run_extractions(&script, body);
        assert_eq!(result.values.get("AUTH_TOKEN"), Some(&"abc123".into()));
        assert_eq!(result.values.get("USER_ID"), Some(&"7".into()));
    }

    #[test]
    fn test_assert_status_eq() {
        let script = PostResponseScript {
            extractions: vec![],
            assertions: vec![AssertionRule {
                name: "is 200".into(),
                kind: AssertionKind::StatusEq(200),
            }],
        };
        let outcomes = run_assertions(&script, &req(), &resp_with("{}"));
        assert_eq!(outcomes.len(), 1);
        assert!(outcomes[0].passed);
    }

    #[test]
    fn test_assert_body_contains() {
        let script = PostResponseScript {
            extractions: vec![],
            assertions: vec![AssertionRule {
                name: "has hello".into(),
                kind: AssertionKind::BodyContains("hello".into()),
            }],
        };
        let outcomes = run_assertions(&script, &req(), &resp_with(r#"{"msg":"hello"}"#));
        assert!(outcomes[0].passed);
    }

    #[test]
    fn test_assert_json_path_eq() {
        let script = PostResponseScript {
            extractions: vec![],
            assertions: vec![AssertionRule {
                name: "id matches".into(),
                kind: AssertionKind::JsonPathEq {
                    path: "data.id".into(),
                    expected: "42".into(),
                },
            }],
        };
        let outcomes = run_assertions(&script, &req(), &resp_with(r#"{"data":{"id":42}}"#));
        assert!(outcomes[0].passed);
    }

    #[test]
    fn test_full_post_response_run() {
        let script = PostResponseScript {
            extractions: vec![ExtractionRule {
                path: "token".into(),
                store_as: "T".into(),
            }],
            assertions: vec![AssertionRule {
                name: "ok".into(),
                kind: AssertionKind::StatusEq(200),
            }],
        };
        let r = run_rules(&script, &req(), &resp_with(r#"{"token":"xyz"}"#));
        assert_eq!(r.extractions.values.get("T"), Some(&"xyz".into()));
        assert_eq!(r.assertions.len(), 1);
        assert!(r.assertions[0].passed);
        assert_eq!(r.logs.len(), 1);
    }
}
