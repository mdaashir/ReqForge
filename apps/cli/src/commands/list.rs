//! `reqforge list` - List all collections in the workspace

use crate::output;
use crate::OutputFormat;
use anyhow::Result;
use reqforge_core::collection::CollectionStorage;

pub async fn execute(workspace: &str, format: OutputFormat) -> Result<()> {
    let storage = CollectionStorage::new(workspace);
    let collections = storage.list_all().await?;

    if collections.is_empty() {
        output::print_info("No collections found in this workspace.", format);
        return Ok(());
    }

    output::print_header("Collections", format);

    let headers = &["ID", "Name", "Requests", "Description"];
    let rows: Vec<Vec<String>> = collections
        .iter()
        .map(|c| {
            vec![
                c.id.clone(),
                c.name.clone(),
                c.request_count().to_string(),
                c.description.clone().unwrap_or_default(),
            ]
        })
        .collect();

    output::print_table(headers, &rows, format);

    Ok(())
}
