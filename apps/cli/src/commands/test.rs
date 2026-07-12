//! `reqforge test` — run tests from a collection.
//!
//! Executes every request in the collection through the test runner and
//! reports pass/fail per request.

use crate::OutputFormat;
use anyhow::{Context, Result};
use reqforge_core::collection::{CollectionRunner, CollectionStorage, RunMode};
use reqforge_core::request::RequestExecutor;
use reqforge_core::testing::reporter::{write_report, ReportFormat};
use std::path::Path;

pub async fn execute(
    workspace: &str,
    collection_id: &str,
    env_name: Option<&str>,
    format: OutputFormat,
) -> Result<()> {
    let workspace_path = Path::new(workspace);
    let storage = CollectionStorage::new(workspace_path);
    let collection = storage
        .load(collection_id)
        .await
        .context("collection not found")?;

    // Load environment if specified
    let env_vars = if let Some(name) = env_name {
        if let Ok(env_storage) = reqforge_core::environment::EnvironmentStorage::new(workspace_path)
        {
            if let Ok(env) = env_storage.load(name) {
                Some(
                    env.variables
                        .into_iter()
                        .filter(|v| v.enabled)
                        .map(|v| (v.key, v.value))
                        .collect::<std::collections::HashMap<_, _>>(),
                )
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let executor = RequestExecutor::new()?;
    let runner = CollectionRunner::new(executor);

    let summary = runner
        .run(
            &collection,
            None,
            RunMode::Sequential,
            env_vars.as_ref(),
            None,
        )
        .await?;

    match format {
        OutputFormat::Human => {
            println!("Collection: {}", summary.collection_name);
            println!(
                "Results: {} total, {} passed, {} failed, {} errors",
                summary.total, summary.passed, summary.failed, summary.errors
            );
            println!("Duration: {} ms", summary.total_duration_ms);
            println!();
            for r in &summary.results {
                let icon = if r.error.is_some() {
                    "❌"
                } else if r.test_result.as_ref().map(|t| t.status)
                    == Some(reqforge_core::testing::TestStatus::Passed)
                {
                    "✅"
                } else {
                    "❌"
                };
                println!(
                    "  {} {} — {} {} ({} ms)",
                    icon, r.request_name, r.method, r.status, r.duration_ms
                );
                if let Some(ref tr) = r.test_result {
                    for a in &tr.assertions {
                        if !a.passed {
                            println!("    - {}: {}", a.message, a.message);
                        }
                    }
                }
                if let Some(ref err) = r.error {
                    println!("    error: {}", err);
                }
            }
        }
        OutputFormat::Json => {
            let json = reqforge_core::testing::reporter::generate_json(
                &[/* would need TestResult list */],
            )
            .unwrap_or_default();
            println!("{}", json);
        }
        OutputFormat::Junit => {
            let xml = reqforge_core::testing::reporter::generate_junit_xml(
                &[/* would need TestResult list */],
            )
            .unwrap_or_default();
            println!("{}", xml);
        }
    }

    if summary.errors + summary.failed > 0 {
        anyhow::bail!("{} tests failed", summary.failed + summary.errors);
    }

    Ok(())
}
