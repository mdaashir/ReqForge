//! Collection runner — sequential and parallel request execution.
//!
//! Walks a collection tree, gathers leaf requests, resolves collection-level
//! variables/auth/headers, and executes them via the `RequestExecutor`.

use crate::collection::model::{Collection, CollectionItem};
use crate::environment::VariableResolver;
use crate::error::Result;
use crate::request::Request;
use crate::request::RequestExecutor;
use crate::testing::{Assertion, TestResult, TestRunner, TestStatus};
use std::collections::HashMap;

/// Result of running a single collection request.
#[derive(Debug, Clone)]
pub struct CollectionRunResult {
    pub request_id: String,
    pub request_name: String,
    pub url: String,
    pub method: String,
    pub status: u16,
    pub duration_ms: u64,
    pub size_bytes: u64,
    pub test_result: Option<TestResult>,
    pub error: Option<String>,
}

/// Summary of a full collection run.
#[derive(Debug, Clone)]
pub struct CollectionRunSummary {
    pub collection_name: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
    pub total_duration_ms: u64,
    pub results: Vec<CollectionRunResult>,
}

/// Execution mode for the collection runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Sequential,
    Parallel,
}

/// Runs all requests in a collection.
pub struct CollectionRunner {
    executor: RequestExecutor,
}

impl CollectionRunner {
    pub fn new(executor: RequestExecutor) -> Self {
        Self { executor }
    }

    /// Run a collection, optionally filtered to specific request IDs.
    pub async fn run(
        &self,
        collection: &Collection,
        filter_ids: Option<&[String]>,
        _mode: RunMode,
        environment: Option<&std::collections::HashMap<String, String>>,
        tests: Option<&[Assertion]>,
    ) -> Result<CollectionRunSummary> {
        let start = std::time::Instant::now();
        let requests = gather_requests(collection, filter_ids);

        if requests.is_empty() {
            return Ok(CollectionRunSummary {
                collection_name: collection.name.clone(),
                total: 0,
                passed: 0,
                failed: 0,
                errors: 0,
                total_duration_ms: 0,
                results: vec![],
            });
        }

        let test_runner = tests.map(|t| {
            let mut runner = TestRunner::new(&collection.name);
            for a in t {
                runner = runner.add(a.clone());
            }
            runner
        });

        // ponytail: parallel RunMode via tokio::spawn with semaphore, add when
        // we need it. Sequential is fine for MVP (collection runs are I/O bound
        // but typically small — < 100 requests).
        let mut results = Vec::with_capacity(requests.len());
        for (req, name, id) in &requests {
            let r = self
                .run_single(collection, req, name, id, environment, &test_runner)
                .await;
            results.push(r);
        }
        Ok(CollectionRunSummary::new(
            collection,
            results,
            start.elapsed().as_millis() as u64,
        ))
    }

    async fn run_single(
        &self,
        collection: &Collection,
        req: &Request,
        name: &str,
        id: &str,
        environment: Option<&HashMap<String, String>>,
        test_runner: &Option<TestRunner>,
    ) -> CollectionRunResult {
        let req_start = std::time::Instant::now();

        // Build variable resolver from collection variables
        let mut resolver = VariableResolver::new();
        let coll_vars: HashMap<String, String> = collection
            .variables
            .iter()
            .map(|kv| (kv.key.clone(), kv.value.clone()))
            .collect();
        resolver.set_collection(coll_vars);
        if let Some(env) = environment {
            for (k, v) in env {
                resolver.set_local(k, v);
            }
        }

        let result = self
            .executor
            .execute_with_resolver(req.clone(), &resolver)
            .await;

        match result {
            Ok(response) => {
                let duration_ms = req_start.elapsed().as_millis() as u64;
                let test_result = test_runner.as_ref().map(|tr| {
                    tr.run(&response).unwrap_or_else(|_| TestResult {
                        name: name.to_string(),
                        status: TestStatus::Error,
                        assertions: vec![],
                        duration_ms,
                    })
                });
                let _passed = test_result
                    .as_ref()
                    .map(|t| matches!(t.status, TestStatus::Passed))
                    .unwrap_or(true);

                CollectionRunResult {
                    request_id: id.to_string(),
                    request_name: name.to_string(),
                    url: response.url.clone(),
                    method: req.method.to_string(),
                    status: response.status,
                    duration_ms,
                    size_bytes: response.size.total,
                    test_result,
                    error: None,
                }
            }
            Err(e) => CollectionRunResult {
                request_id: id.to_string(),
                request_name: name.to_string(),
                url: req.url.clone(),
                method: req.method.to_string(),
                status: 0,
                duration_ms: req_start.elapsed().as_millis() as u64,
                size_bytes: 0,
                test_result: None,
                error: Some(e.to_string()),
            },
        }
    }
}

/// Walk the collection tree and return (Request, name, id) for each leaf.
fn gather_requests(
    collection: &Collection,
    filter_ids: Option<&[String]>,
) -> Vec<(Request, String, String)> {
    fn walk(items: &[CollectionItem]) -> Vec<(Request, String, String)> {
        let mut out = Vec::new();
        for item in items {
            match item {
                CollectionItem::Request { id, name, request } => {
                    out.push((request.clone(), name.clone(), id.clone()));
                }
                CollectionItem::Folder { children, .. } => {
                    out.extend(walk(children));
                }
            }
        }
        out
    }

    let mut all = walk(&collection.items);
    if let Some(filter) = filter_ids {
        all.retain(|(_, _, id)| filter.contains(id));
    }
    all
}

impl CollectionRunSummary {
    fn new(
        collection: &Collection,
        results: Vec<CollectionRunResult>,
        total_duration_ms: u64,
    ) -> Self {
        let total = results.len();
        let mut passed = 0;
        let mut failed = 0;
        let mut errors = 0;
        for r in &results {
            match &r.test_result {
                Some(tr) if matches!(tr.status, TestStatus::Passed) => passed += 1,
                Some(_) => failed += 1,
                None => {
                    if r.error.is_some() {
                        errors += 1;
                    } else {
                        passed += 1;
                    }
                }
            }
        }
        Self {
            collection_name: collection.name.clone(),
            total,
            passed,
            failed,
            errors,
            total_duration_ms,
            results,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::HttpMethod;

    fn sample_collection() -> Collection {
        let mut col = Collection::new("Test Collection");
        col.items.push(CollectionItem::Request {
            id: "r1".to_string(),
            name: "Get Users".to_string(),
            request: Request::new(HttpMethod::Get, "https://httpbin.org/get"),
        });
        col.items.push(CollectionItem::Folder {
            id: "f1".to_string(),
            name: "Users".to_string(),
            description: None,
            children: vec![CollectionItem::Request {
                id: "r2".to_string(),
                name: "Create User".to_string(),
                request: Request::new(HttpMethod::Post, "https://httpbin.org/post"),
            }],
            auth: None,
        });
        col
    }

    #[test]
    fn test_gather_requests() {
        let col = sample_collection();
        let items = gather_requests(&col, None);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].1, "Get Users");
        assert_eq!(items[1].1, "Create User");
    }

    #[test]
    fn test_gather_requests_filtered() {
        let col = sample_collection();
        let items = gather_requests(&col, Some(&["r1".to_string()]));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].1, "Get Users");
    }
}
