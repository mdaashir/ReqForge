//! Test report generation.
//!
//! All reporter implementations live in `reqforge-core::testing::reporter`.
//! This module re-exports them so the CLI can use them without depending
//! on core's internal paths.

pub use reqforge_core::testing::reporter::{
    generate_html, generate_json, generate_junit_xml, generate_markdown, write_report, ReportFormat,
};
