//! Output formatting helpers for the CLI

use crate::OutputFormat;
use colored::Colorize;
use std::fmt::Write;
use comfy_table::Table;

pub fn print_header(text: &str, format: OutputFormat) {
    match format {
        OutputFormat::Human => {
            println!("\n{}", text.bold().underline());
        }
        OutputFormat::Json | OutputFormat::Junit => {}
    }
}

pub fn print_success(text: &str, format: OutputFormat) {
    match format {
        OutputFormat::Human => println!("{} {}", "✓".green().bold(), text),
        _ => {}
    }
}

pub fn print_error(text: &str, format: OutputFormat) {
    match format {
        OutputFormat::Human => eprintln!("{} {}", "✗".red().bold(), text),
        _ => eprintln!("{}", serde_json::json!({ "error": text })),
    }
}

pub fn print_info(text: &str, format: OutputFormat) {
    match format {
        OutputFormat::Human => println!("{} {}", "→".cyan(), text),
        _ => {}
    }
}

pub fn print_json<T: serde::Serialize>(value: &T) -> Result<(), serde_json::Error> {
    let json = serde_json::to_string_pretty(value)?;
    println!("{}", json);
    Ok(())
}

pub fn print_table(headers: &[&str], rows: &[Vec<String>], format: OutputFormat) {
    match format {
        OutputFormat::Human => {
            let mut table = Table::new();
            table.set_header(headers);
            for row in rows {
                let cells: Vec<&str> = row.iter().map(|s| s.as_str()).collect();
                table.add_row(cells);
            }
            println!("{table}");
        }
        _ => {
            let as_objects: Vec<serde_json::Value> = rows
                .iter()
                .map(|row| {
                    let mut obj = serde_json::Map::new();
                    for (i, cell) in row.iter().enumerate() {
                        let key = headers.get(i).unwrap_or(&"").to_string();
                        obj.insert(key, serde_json::Value::String(cell.clone()));
                    }
                    serde_json::Value::Object(obj)
                })
                .collect();
            let _ = print_json(&as_objects);
        }
    }
}
