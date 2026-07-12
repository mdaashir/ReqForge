//! Plugin marketplace commands: search, info, install.

use crate::OutputFormat;
use serde::{Deserialize, Serialize};

use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
struct PluginEntry {
    id: String,
    name: String,
    version: String,
    author: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
    download_url: String,
}

#[derive(Debug, Deserialize)]
struct ListResponse {
    total: usize,
    plugins: Vec<PluginEntry>,
}

pub async fn execute(cmd: &crate::PluginCommands, format: OutputFormat) -> Result<()> {
    match cmd {
        crate::PluginCommands::Search { query, tag, server } => {
            let mut url = format!("{}/v1/plugins", server.trim_end_matches('/'));
            let mut params = Vec::new();
            if let Some(q) = query {
                params.push(format!("q={}", urlencoding(&q)));
            }
            if let Some(t) = tag {
                params.push(format!("tag={}", urlencoding(&t)));
            }
            if !params.is_empty() {
                url.push('?');
                url.push_str(&params.join("&"));
            }

            let resp = reqwest::get(&url).await?;
            let data: ListResponse = resp.json().await?;

            if format == OutputFormat::Json {
                println!("{}", serde_json::to_string_pretty(&data.plugins)?);
            } else {
                print_human_list(&data);
            }
        }
        crate::PluginCommands::Info { id, server } => {
            let url = format!("{}/v1/plugins/{}", server.trim_end_matches('/'), id);
            let resp = reqwest::get(&url).await?;
            if !resp.status().is_success() {
                anyhow::bail!("Plugin '{}' not found", id);
            }
            let entry: PluginEntry = resp.json().await?;

            if format == OutputFormat::Json {
                println!("{}", serde_json::to_string_pretty(&entry)?);
            } else {
                print_human_info(&entry);
            }
        }
    }
    Ok(())
}

fn print_human_list(data: &ListResponse) {
    if data.plugins.is_empty() {
        println!("  No plugins found.");
        return;
    }
    println!("  {} plugin(s) found:\n", data.total);
    for p in &data.plugins {
        let author = p.author.as_deref().unwrap_or("unknown");
        println!(
            "  {:<20} v{:<8} by {:<16} {}",
            p.name,
            p.version,
            author,
            p.description.as_deref().unwrap_or("")
        );
    }
}

fn print_human_info(entry: &PluginEntry) {
    println!("  Name:        {}", entry.name);
    println!("  ID:          {}", entry.id);
    println!("  Version:     {}", entry.version);
    if let Some(a) = &entry.author {
        println!("  Author:      {}", a);
    }
    if let Some(d) = &entry.description {
        println!("  Description: {}", d);
    }
    if !entry.tags.is_empty() {
        println!("  Tags:        {}", entry.tags.join(", "));
    }
    println!("  Download:    {}", entry.download_url);
}

fn urlencoding(s: &str) -> String {
    // Simple percent-encode for query params. We only need to handle
    // spaces and a few special chars.
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

use clap::Args;
