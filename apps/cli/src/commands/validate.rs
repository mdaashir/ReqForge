//! `reqforge validate` - Validate all collections in the workspace

use crate::output;
use crate::OutputFormat;
use anyhow::Result;
use colored::Colorize;
use reqforge_core::collection::CollectionStorage;
use serde::Serialize;

#[derive(Serialize)]
struct ValidationReport {
    total: usize,
    valid: usize,
    invalid: usize,
    errors: Vec<ValidationError>,
}

#[derive(Serialize)]
struct ValidationError {
    collection_id: String,
    collection_name: String,
    error: String,
}

pub async fn execute(workspace: &str, format: OutputFormat) -> Result<()> {
    let storage = CollectionStorage::new(workspace);
    let collections = storage.list_all().await?;

    output::print_header(
        &format!("Validating {} collection(s)", collections.len()),
        format,
    );

    let mut valid = 0;
    let mut errors = Vec::new();

    for collection in &collections {
        match validate_collection(collection) {
            Ok(()) => {
                valid += 1;
                if format == OutputFormat::Human {
                    println!("  {} {}", "✓".green(), collection.name);
                }
            }
            Err(err) => {
                errors.push(ValidationError {
                    collection_id: collection.id.clone(),
                    collection_name: collection.name.clone(),
                    error: err.to_string(),
                });
                if format == OutputFormat::Human {
                    println!("  {} {} - {}", "✗".red(), collection.name, err);
                }
            }
        }
    }

    let report = ValidationReport {
        total: collections.len(),
        valid,
        invalid: errors.len(),
        errors,
    };

    output::print_header("Result", format);
    if format == OutputFormat::Human {
        println!("  {}/{} collections valid", report.valid, report.total);
    } else {
        output::print_json(&report)?;
    }

    if report.invalid > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn validate_collection(c: &reqforge_core::Collection) -> anyhow::Result<()> {
    if c.name.trim().is_empty() {
        anyhow::bail!("Collection name is empty");
    }
    if c.id.trim().is_empty() {
        anyhow::bail!("Collection id is empty");
    }
    // Walk and ensure each request has a method + url
    fn walk(items: &[reqforge_core::collection::CollectionItem]) -> anyhow::Result<()> {
        for item in items {
            match item {
                reqforge_core::collection::CollectionItem::Folder { children, .. } => {
                    walk(children)?;
                }
                reqforge_core::collection::CollectionItem::Request { name, request, .. } => {
                    if name.trim().is_empty() {
                        anyhow::bail!("Request has empty name");
                    }
                    if request.url.trim().is_empty() {
                        anyhow::bail!("Request '{}' has empty URL", name);
                    }
                }
            }
        }
        Ok(())
    }
    walk(&c.items)
}
