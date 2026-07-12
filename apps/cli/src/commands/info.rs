//! `reqforge info` - Show workspace information

use crate::output;
use crate::OutputFormat;
use anyhow::{Context, Result};
use reqforge_core::collection::CollectionStorage;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
struct WorkspaceInfo {
    workspace_path: String,
    exists: bool,
    collection_count: usize,
    collections: Vec<String>,
}

pub async fn execute(workspace: &str, format: OutputFormat) -> Result<()> {
    let path = Path::new(workspace);
    let exists = path.exists() && path.join("collections").exists();

    let storage = CollectionStorage::new(path);
    let collection_ids = storage
        .list_ids()
        .await
        .context("Failed to read workspace collections")?;

    let info = WorkspaceInfo {
        workspace_path: path.canonicalize()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| workspace.to_string()),
        exists,
        collection_count: collection_ids.len(),
        collections: collection_ids,
    };

    if format == OutputFormat::Json {
        output::print_json(&info)?;
    } else {
        output::print_header("Workspace Info", format);
        println!("  Path:          {}", info.workspace_path);
        println!("  Initialised:   {}", if info.exists { "yes" } else { "no" });
        println!("  Collections:   {}", info.collection_count);
        if !info.collections.is_empty() {
            println!();
            for id in &info.collections {
                println!("    - {}", id);
            }
        }
    }

    Ok(())
}
