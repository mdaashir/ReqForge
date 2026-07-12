use crate::request::Response;
use serde::{Deserialize, Serialize};

/// Type of assertion to perform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssertionType {
    /// HTTP status code equals expected value
    StatusCode { expected: u16 },
    /// Response time under N milliseconds
    ResponseTime { max_ms: u64 },
    /// Response body contains a substring
    BodyContains { substring: String },
    /// Response body matches a regex pattern
    BodyMatches { pattern: String },
    /// Response header exists with expected value
    HeaderEquals { header: String, expected: String },
    /// Response header contains a substring
    HeaderContains { header: String, substring: String },
    /// JSON path expression returns expected value
    JsonPath { path: String, expected: String },
    /// Content-Type header matches
    ContentType { expected: String },
    /// JSON Schema validation of response body
    JsonSchema { schema: String },
    /// Snapshot comparison against a golden file
    Snapshot { request_id: String },
}

/// Result of running an assertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    pub passed: bool,
    pub message: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

impl AssertionResult {
    pub fn passed(message: impl Into<String>) -> Self {
        Self {
            passed: true,
            message: message.into(),
            expected: None,
            actual: None,
        }
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            passed: false,
            message: message.into(),
            expected: None,
            actual: None,
        }
    }

    pub fn failed_with(
        message: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self {
            passed: false,
            message: message.into(),
            expected: Some(expected.into()),
            actual: Some(actual.into()),
        }
    }
}

/// A named assertion to run against a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    pub name: String,
    #[serde(flatten)]
    pub assertion: AssertionType,
}

impl Assertion {
    pub fn new(name: impl Into<String>, assertion: AssertionType) -> Self {
        Self {
            name: name.into(),
            assertion,
        }
    }

    /// Run the assertion against a response
    pub fn run(&self, response: &Response) -> AssertionResult {
        match &self.assertion {
            AssertionType::StatusCode { expected } => {
                if response.status == *expected {
                    AssertionResult::passed(format!("Status is {}", expected))
                } else {
                    AssertionResult::failed_with(
                        format!("Expected status {} but got {}", expected, response.status),
                        expected.to_string(),
                        response.status.to_string(),
                    )
                }
            }

            AssertionType::ResponseTime { max_ms } => {
                if response.timing.total_ms <= *max_ms {
                    AssertionResult::passed(format!(
                        "Response time {}ms <= {}ms",
                        response.timing.total_ms, max_ms
                    ))
                } else {
                    AssertionResult::failed_with(
                        format!(
                            "Response time {}ms exceeded limit {}ms",
                            response.timing.total_ms, max_ms
                        ),
                        format!("<= {}ms", max_ms),
                        format!("{}ms", response.timing.total_ms),
                    )
                }
            }

            AssertionType::BodyContains { substring } => {
                let body_text = String::from_utf8_lossy(&response.body.content);
                if body_text.contains(substring) {
                    AssertionResult::passed(format!("Body contains '{}'", substring))
                } else {
                    AssertionResult::failed_with(
                        format!("Body does not contain '{}'", substring),
                        substring.clone(),
                        truncate(&body_text, 100),
                    )
                }
            }

            AssertionType::BodyMatches { pattern } => match regex::Regex::new(pattern) {
                Ok(re) => {
                    let body_text = String::from_utf8_lossy(&response.body.content);
                    if re.is_match(&body_text) {
                        AssertionResult::passed(format!("Body matches pattern '{}'", pattern))
                    } else {
                        AssertionResult::failed(format!(
                            "Body does not match pattern '{}'",
                            pattern
                        ))
                    }
                }
                Err(e) => AssertionResult::failed(format!("Invalid regex: {}", e)),
            },

            AssertionType::HeaderEquals { header, expected } => {
                let actual = find_header(&response.headers, header);
                match actual {
                    Some(v) if v == *expected => AssertionResult::passed(format!(
                        "Header '{}' equals '{}'",
                        header, expected
                    )),
                    Some(v) => AssertionResult::failed_with(
                        format!("Header '{}' mismatch", header),
                        expected.clone(),
                        v,
                    ),
                    None => AssertionResult::failed(format!("Header '{}' not found", header)),
                }
            }

            AssertionType::HeaderContains { header, substring } => {
                let actual = find_header(&response.headers, header);
                match actual {
                    Some(v) if v.contains(substring) => AssertionResult::passed(format!(
                        "Header '{}' contains '{}'",
                        header, substring
                    )),
                    Some(v) => AssertionResult::failed_with(
                        format!("Header '{}' does not contain '{}'", header, substring),
                        substring.clone(),
                        v,
                    ),
                    None => AssertionResult::failed(format!("Header '{}' not found", header)),
                }
            }

            AssertionType::JsonPath { path, expected } => {
                let body_text = String::from_utf8_lossy(&response.body.content);
                match serde_json::from_str::<serde_json::Value>(&body_text) {
                    Ok(json) => match extract_json_path(&json, path) {
                        Some(v) => {
                            let actual = v.to_string();
                            let expected_trim = expected.trim_matches('"');
                            if actual == expected_trim
                                || v.to_string().trim_matches('"') == expected_trim
                            {
                                AssertionResult::passed(format!(
                                    "JSON path '{}' = '{}'",
                                    path, expected
                                ))
                            } else {
                                AssertionResult::failed_with(
                                    format!("JSON path '{}' mismatch", path),
                                    expected.clone(),
                                    actual,
                                )
                            }
                        }
                        None => AssertionResult::failed(format!("JSON path '{}' not found", path)),
                    },
                    Err(e) => AssertionResult::failed(format!("Invalid JSON body: {}", e)),
                }
            }

            AssertionType::ContentType { expected } => {
                let actual = response
                    .headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                    .map(|(_, v)| v.as_str());
                match actual {
                    Some(v) if v.contains(expected) => {
                        AssertionResult::passed(format!("Content-Type contains '{}'", expected))
                    }
                    Some(v) => AssertionResult::failed_with(
                        "Content-Type mismatch".to_string(),
                        expected.clone(),
                        v,
                    ),
                    None => AssertionResult::failed("Content-Type header not found".to_string()),
                }
            }

            AssertionType::JsonSchema { schema } => {
                let body_text = String::from_utf8_lossy(&response.body.content);
                match super::schema_validator::validate_json_schema(schema, &body_text) {
                    Ok(_) => AssertionResult::passed("Response matches JSON Schema".to_string()),
                    Err(e) => AssertionResult::failed(format!("JSON Schema: {e}")),
                }
            }

            AssertionType::Snapshot { request_id } => {
                // Snapshot comparison is handled by the TestRunner (which has
                // access to SnapshotStorage). The assertion itself is a marker.
                AssertionResult::passed(format!("Snapshot comparison queued for '{}'", request_id))
            }
        }
    }
}

fn find_header(headers: &std::collections::HashMap<String, String>, name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

/// Extract a value from JSON using a simple path like `data.users[0].name`
fn extract_json_path<'a>(json: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = json;
    let mut remaining = path.trim_start_matches('$').trim_start_matches('.');

    while !remaining.is_empty() {
        // Try array index
        if let Some(stripped) = remaining.strip_prefix('[') {
            let end = stripped.find(']')?;
            let idx: usize = stripped[..end].parse().ok()?;
            current = current.get(idx)?;
            remaining = stripped[end + 1..].trim_start_matches('.');
        } else {
            // Object key
            let end = remaining.find(['.', '[']).unwrap_or(remaining.len());
            let key = &remaining[..end];
            current = current.get(key)?;
            remaining = remaining[end..].trim_start_matches('.');
        }
    }

    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{ResponseBody, ResponseSize, ResponseTiming};

    fn make_response(status: u16, body: &str, headers: Vec<(&str, &str)>) -> Response {
        let header_map: std::collections::HashMap<String, String> = headers
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        Response {
            status,
            status_text: "OK".to_string(),
            headers: header_map,
            body: ResponseBody {
                content: body.as_bytes().to_vec(),
                content_type: Some("application/json".to_string()),
                is_text: true,
            },
            cookies: vec![],
            timing: ResponseTiming {
                total_ms: 150,
                ..Default::default()
            },
            size: ResponseSize {
                headers: 100,
                body: body.len() as u64,
                total: 100 + body.len() as u64,
            },
            url: "https://api.example.com".to_string(),
            protocol: "HTTP/1.1".to_string(),
        }
    }

    #[test]
    fn test_status_code_pass() {
        let a = Assertion::new("Status", AssertionType::StatusCode { expected: 200 });
        let r = make_response(200, "", vec![]);
        assert!(a.run(&r).passed);
    }

    #[test]
    fn test_status_code_fail() {
        let a = Assertion::new("Status", AssertionType::StatusCode { expected: 404 });
        let r = make_response(200, "", vec![]);
        assert!(!a.run(&r).passed);
    }

    #[test]
    fn test_response_time_pass() {
        let a = Assertion::new("Fast", AssertionType::ResponseTime { max_ms: 500 });
        let r = make_response(200, "", vec![]);
        assert!(a.run(&r).passed);
    }

    #[test]
    fn test_response_time_fail() {
        let a = Assertion::new("Fast", AssertionType::ResponseTime { max_ms: 100 });
        let r = make_response(200, "", vec![]);
        assert!(!a.run(&r).passed);
    }

    #[test]
    fn test_body_contains() {
        let a = Assertion::new(
            "Has users",
            AssertionType::BodyContains {
                substring: "users".to_string(),
            },
        );
        let r = make_response(200, r#"{"data": "users list"}"#, vec![]);
        assert!(a.run(&r).passed);
    }

    #[test]
    fn test_header_equals() {
        let a = Assertion::new(
            "CT",
            AssertionType::HeaderEquals {
                header: "content-type".to_string(),
                expected: "application/json".to_string(),
            },
        );
        let r = make_response(200, "", vec![("Content-Type", "application/json")]);
        assert!(a.run(&r).passed);
    }

    #[test]
    fn test_json_path() {
        let a = Assertion::new(
            "ID",
            AssertionType::JsonPath {
                path: "$.data.id".to_string(),
                expected: "42".to_string(),
            },
        );
        let r = make_response(200, r#"{"data": {"id": 42}}"#, vec![]);
        assert!(a.run(&r).passed);
    }

    #[test]
    fn test_json_path_array() {
        let a = Assertion::new(
            "First user",
            AssertionType::JsonPath {
                path: "$.users[0].name".to_string(),
                expected: "Alice".to_string(),
            },
        );
        let r = make_response(
            200,
            r#"{"users": [{"name": "Alice"}, {"name": "Bob"}]}"#,
            vec![],
        );
        assert!(a.run(&r).passed);
    }
}
