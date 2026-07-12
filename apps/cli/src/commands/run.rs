//! `reqforge run` - Run a collection or a single request

use crate::output;
use crate::OutputFormat;
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use reqforge_core::collection::{Collection, CollectionItem, CollectionStorage};
use reqforge_core::request::{HttpMethod, Request as CoreRequest, Response as CoreResponse};
use reqforge_core::{HttpHandler, ProtocolHandler};
use serde::Serialize;
use std::path::Path;
use std::time::Instant;

#[derive(Serialize)]
struct RunResult {
    collection_id: String,
    collection_name: String,
    request_name: String,
    url: String,
    method: String,
    status: u16,
    duration_ms: u64,
    body_size: u64,
    success: bool,
}

/// Flatten all requests in a collection into a vec of (request, name)
fn flatten_requests(items: &[CollectionItem]) -> Vec<(String, CoreRequest)> {
    fn walk(items: &[CollectionItem], out: &mut Vec<(String, CoreRequest)>) {
        for item in items {
            match item {
                CollectionItem::Request { name, request, .. } => {
                    out.push((name.clone(), request.clone()));
                }
                CollectionItem::Folder { children, .. } => {
                    walk(children, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(items, &mut out);
    out
}

async fn execute_one(req: CoreRequest) -> Result<CoreResponse> {
    let handler = HttpHandler::new();
    handler.send(req).await.context("Request execution failed")
}

pub async fn execute(
    workspace: &str,
    collection_id: &str,
    request_name: Option<&str>,
    _env: Option<&str>,
    format: OutputFormat,
) -> Result<()> {
    let path = Path::new(workspace);
    let storage = CollectionStorage::new(path);

    let collection: Collection = storage
        .load(collection_id)
        .await
        .with_context(|| format!("Collection '{}' not found", collection_id))?;

    let requests = flatten_requests(&collection.items);

    if requests.is_empty() {
        return Err(anyhow!(
            "Collection '{}' contains no requests",
            collection.name
        ));
    }

    // Filter to a single request if requested
    let to_run: Vec<(String, CoreRequest)> = if let Some(name) = request_name {
        requests
            .into_iter()
            .filter(|(n, _)| n == name)
            .collect::<Vec<_>>()
            .into_iter()
            .filter(|v| !v.1.url.is_empty())
            .collect()
    } else {
        requests
            .into_iter()
            .filter(|(_, r)| !r.url.is_empty())
            .collect()
    };

    if to_run.is_empty() {
        return Err(anyhow!("No matching requests to run"));
    }

    output::print_header(&format!("Running {} request(s)", to_run.len()), format);

    let mut results: Vec<RunResult> = Vec::with_capacity(to_run.len());
    let mut all_passed = true;

    for (name, mut req) in to_run {
        if req.name.is_empty() {
            req.name = name.clone();
        }
        if req.id.is_empty() {
            req.id = uuid::Uuid::new_v4().to_string();
        }
        // Default to GET when missing
        if matches!(&req.method, HttpMethod::Custom(s) if s.is_empty()) {
            req.method = HttpMethod::Get;
        }

        let url = req.url.clone();
        let method = req.method.to_string();
        let start = Instant::now();

        match execute_one(req).await {
            Ok(response) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let success = (200..400).contains(&response.status);
                if !success {
                    all_passed = false;
                }

                let result = RunResult {
                    collection_id: collection.id.clone(),
                    collection_name: collection.name.clone(),
                    request_name: name.clone(),
                    url: url.clone(),
                    method: method.clone(),
                    status: response.status,
                    duration_ms,
                    body_size: response.size.body,
                    success,
                };

                if format == OutputFormat::Human {
                    let marker = if success {
                        "✓".green().to_string()
                    } else {
                        "✗".red().to_string()
                    };
                    println!(
                        "  {} {} {} ({}ms) - {}",
                        marker,
                        method.cyan(),
                        name,
                        duration_ms,
                        format!("{}", response.status).bold()
                    );
                }

                results.push(result);
            }
            Err(err) => {
                all_passed = false;
                output::print_error(&format!("{}: {}", name, err), format);
            }
        }
    }

    // Final summary
    output::print_header("Summary", format);
    let passed = results.iter().filter(|r| r.success).count();
    let total = results.len();
    if format == OutputFormat::Human {
        println!("  {}/{} passed", passed, total);
    } else {
        output::print_json(&serde_json::json!({
            "results": results,
            "summary": { "passed": passed, "total": total }
        }))?;
    }

    if !all_passed {
        std::process::exit(2);
    }

    Ok(())
}
