//! `reqforge import` — import collections from other tools.
//!
//! Supports Postman v2.1, cURL, Insomnia, and Bruno formats.

use anyhow::{Context, Result};
use reqforge_core::collection::CollectionStorage;
use reqforge_core::import::{detect_importer, CurlImporter, InsomniaImporter, PostmanImporter};
use std::path::Path;

pub async fn execute(
    workspace: &str,
    input_file: &str,
    format: Option<&str>,
    output_name: Option<&str>,
) -> Result<()> {
    let content = tokio::fs::read_to_string(input_file)
        .await
        .context("failed to read input file")?;

    let importer: Box<dyn reqforge_core::import::Importer> = match format {
        Some("postman") => Box::new(PostmanImporter),
        Some("curl") => Box::new(CurlImporter),
        Some("insomnia") => Box::new(InsomniaImporter),
        Some(other) => anyhow::bail!("unsupported format: {other}"),
        None => detect_importer(&content).context("could not auto-detect format; use --format")?,
    };

    let mut collection = importer.import(&content)?;
    if let Some(name) = output_name {
        collection.name = name.to_string();
    }

    let workspace_path = Path::new(workspace);
    let storage = CollectionStorage::new(workspace_path);
    storage.save(&collection).await?;

    println!(
        "Imported: {} ({} requests)",
        collection.name,
        collection.request_count()
    );
    println!("  ID: {}", collection.id);
    Ok(())
}
