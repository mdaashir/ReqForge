//! `reqforge export` — export collections to standard formats.
//!
//! Currently supports JSON and YAML export.

use anyhow::{Context, Result};
use reqforge_core::collection::CollectionStorage;
use std::path::Path;

pub async fn execute(
    workspace: &str,
    collection_id: &str,
    format: &str,
    output: Option<&str>,
) -> Result<()> {
    let workspace_path = Path::new(workspace);
    let storage = CollectionStorage::new(workspace_path);
    let collection = storage
        .load(collection_id)
        .await
        .context("collection not found")?;

    let output_path = if let Some(path) = output {
        path.to_string()
    } else {
        format!(
            "{}-{}.{}",
            collection.name.to_lowercase().replace(' ', "-"),
            collection_id.chars().take(8).collect::<String>(),
            if format == "json" { "json" } else { "yaml" }
        )
    };

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&collection)?;
            tokio::fs::write(&output_path, json).await?;
        }
        "yaml" | "yml" => {
            let yaml = serde_yaml::to_string(&collection)?;
            tokio::fs::write(&output_path, yaml).await?;
        }
        _ => anyhow::bail!("unsupported export format: {format} (use json or yaml)"),
    }

    println!("Exported collection to: {}", output_path);
    Ok(())
}
