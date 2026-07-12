use crate::error::Result;
use crate::request::Response;
use crate::testing::assertion::{Assertion, AssertionResult, AssertionType};
use crate::testing::snapshot::SnapshotStorage;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Status of a single test run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestStatus::Passed => write!(f, "passed"),
            TestStatus::Failed => write!(f, "failed"),
            TestStatus::Skipped => write!(f, "skipped"),
            TestStatus::Error => write!(f, "error"),
        }
    }
}

/// Result of running a single test (collection of assertions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub status: TestStatus,
    pub assertions: Vec<AssertionResult>,
    pub duration_ms: u64,
}

impl TestResult {
    pub fn passed(name: impl Into<String>, assertions: Vec<AssertionResult>, duration_ms: u64) -> Self {
        Self {
            name: name.into(),
            status: TestStatus::Passed,
            assertions,
            duration_ms,
        }
    }

    pub fn failed(name: impl Into<String>, assertions: Vec<AssertionResult>, duration_ms: u64) -> Self {
        Self {
            name: name.into(),
            status: TestStatus::Failed,
            assertions,
            duration_ms,
        }
    }
}

/// Runs assertions against an HTTP response
pub struct TestRunner {
    pub name: String,
    pub assertions: Vec<Assertion>,
    snapshot_storage: Option<SnapshotStorage>,
}

impl TestRunner {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            assertions: Vec::new(),
            snapshot_storage: None,
        }
    }

    /// Append an assertion to this suite.
    #[allow(clippy::should_implement_trait)] // builder-style, not std::ops::Add
    pub fn add(mut self, assertion: Assertion) -> Self {
        self.assertions.push(assertion);
        self
    }

    /// Attach snapshot storage for golden-file comparison.
    pub fn with_snapshots(mut self, workspace_root: impl AsRef<Path>) -> Self {
        self.snapshot_storage = Some(SnapshotStorage::new(workspace_root));
        self
    }

    /// Enable or disable snapshot update mode.
    pub fn set_snapshot_update(&mut self, enabled: bool) {
        if let Some(ref mut snap) = self.snapshot_storage {
            snap.set_update_mode(enabled);
        }
    }

    /// Execute all assertions against the response and return a TestResult
    pub fn run(&self, response: &Response) -> Result<TestResult> {
        let start = std::time::Instant::now();

        let mut results = Vec::with_capacity(self.assertions.len());
        let mut all_passed = true;

        for assertion in &self.assertions {
            let result = assertion.run(response);
            if !result.passed {
                all_passed = false;
            }
            results.push(result);
        }

        // Run snapshot comparisons
        for assertion in &self.assertions {
            if let AssertionType::Snapshot { ref request_id } = assertion.assertion {
                if let Some(ref storage) = self.snapshot_storage {
                    let body_text = String::from_utf8_lossy(&response.body.content);
                    match storage.match_or_update(request_id, &body_text) {
                        Ok(true) => {
                            results.push(AssertionResult::passed(format!(
                                "Snapshot '{}' matches", request_id
                            )));
                        }
                        Ok(false) => {
                            all_passed = false;
                            results.push(AssertionResult::failed(format!(
                                "Snapshot '{}' differs from golden", request_id
                            )));
                        }
                        Err(e) => {
                            all_passed = false;
                            results.push(AssertionResult::failed(format!(
                                "Snapshot '{}': {}", request_id, e
                            )));
                        }
                    }
                } else {
                    results.push(AssertionResult::passed(format!(
                        "Snapshot '{}' — no storage configured, skipped", request_id
                    )));
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        let test_result = if all_passed {
            TestResult::passed(&self.name, results, duration_ms)
        } else {
            TestResult::failed(&self.name, results, duration_ms)
        };

        Ok(test_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{ResponseBody, ResponseSize, ResponseTiming};
    use crate::testing::assertion::AssertionType;

    fn make_response(status: u16, body: &str) -> Response {
        Response {
            status,
            status_text: "OK".to_string(),
            headers: [("content-type".to_string(), "application/json".to_string())]
                .into_iter()
                .collect(),
            body: ResponseBody {
                content: body.as_bytes().to_vec(),
                content_type: Some("application/json".to_string()),
                is_text: true,
            },
            cookies: vec![],
            timing: ResponseTiming {
                total_ms: 50,
                ..Default::default()
            },
            size: ResponseSize {
                headers: 50,
                body: body.len() as u64,
                total: 50 + body.len() as u64,
            },
            url: "https://api.example.com".to_string(),
            protocol: "HTTP/1.1".to_string(),
        }
    }

    #[test]
    fn test_runner_all_pass() {
        let runner = TestRunner::new("User API")
            .add(Assertion::new(
                "Status",
                AssertionType::StatusCode { expected: 200 },
            ))
            .add(Assertion::new(
                "Fast",
                AssertionType::ResponseTime { max_ms: 500 },
            ));

        let response = make_response(200, r#"{"id": 1}"#);
        let result = runner.run(&response).unwrap();

        assert_eq!(result.status, TestStatus::Passed);
        assert_eq!(result.assertions.len(), 2);
        assert!(result.assertions.iter().all(|a| a.passed));
    }

    #[test]
    fn test_runner_one_fail() {
        let runner = TestRunner::new("User API").add(Assertion::new(
            "Status",
            AssertionType::StatusCode { expected: 200 },
        ));

        let response = make_response(404, "Not Found");
        let result = runner.run(&response).unwrap();

        assert_eq!(result.status, TestStatus::Failed);
        assert_eq!(result.assertions.len(), 1);
        assert!(!result.assertions[0].passed);
    }

    #[test]
    fn test_runner_mixed() {
        let runner = TestRunner::new("User API")
            .add(Assertion::new(
                "Status",
                AssertionType::StatusCode { expected: 200 },
            ))
            .add(Assertion::new(
                "Wrong content",
                AssertionType::BodyContains {
                    substring: "wrong".to_string(),
                },
            ));

        let response = make_response(200, "hello world");
        let result = runner.run(&response).unwrap();

        assert_eq!(result.status, TestStatus::Failed);
        assert_eq!(result.assertions.len(), 2);
    }

    #[test]
    fn test_json_schema_assertion_pass() {
        let runner = TestRunner::new("Schema test").add(Assertion::new(
            "Schema",
            AssertionType::JsonSchema {
                schema: r#"{"type": "object", "properties": {"id": {"type": "number"}}}"#.to_string(),
            },
        ));
        let response = make_response(200, r#"{"id": 42}"#);
        let result = runner.run(&response).unwrap();
        assert_eq!(result.status, TestStatus::Passed);
    }

    #[test]
    fn test_json_schema_assertion_fail() {
        let runner = TestRunner::new("Schema test").add(Assertion::new(
            "Schema",
            AssertionType::JsonSchema {
                schema: r#"{"type": "object", "required": ["email"]}"#.to_string(),
            },
        ));
        let response = make_response(200, r#"{"id": 42}"#);
        let result = runner.run(&response).unwrap();
        assert_eq!(result.status, TestStatus::Failed);
    }
}
