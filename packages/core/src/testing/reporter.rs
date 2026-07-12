//! Test report generation for assertions and test runs.
//!
//! Supports three output formats:
//! - JSON — machine-readable structured report
//! - JUnit XML — CI pipeline integration
//! - HTML — human-readable report

use crate::error::{Error, Result};
use crate::testing::TestResult;
use std::path::Path;

/// Output format selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Json,
    Junit,
    Html,
    Markdown,
}

/// Generate a test report and write it to a file.
pub fn write_report(results: &[TestResult], path: &Path, format: ReportFormat) -> Result<()> {
    let content = match format {
        ReportFormat::Json => generate_json(results)?,
        ReportFormat::Junit => generate_junit_xml(results)?,
        ReportFormat::Html => generate_html(results)?,
        ReportFormat::Markdown => generate_markdown(results)?,
    };
    std::fs::write(path, content)
        .map_err(|e| Error::storage(format!("failed to write report: {e}")))?;
    Ok(())
}

/// Generate a JSON report from test results.
pub fn generate_json(results: &[TestResult]) -> Result<String> {
    let passed = results.iter().filter(|r| matches!(r.status, crate::testing::TestStatus::Passed)).count();
    let failed = results.iter().filter(|r| matches!(r.status, crate::testing::TestStatus::Failed)).count();
    let total = results.len();

    let report = serde_json::json!({
        "summary": {
            "total": total,
            "passed": passed,
            "failed": failed,
            "passed_pct": if total > 0 { (passed as f64 / total as f64) * 100.0 } else { 0.0 },
        },
        "tests": results.iter().map(|r| serde_json::json!({
            "name": r.name,
            "status": r.status,
            "duration_ms": r.duration_ms,
            "assertions": r.assertions.iter().map(|a| serde_json::json!({
                "name": a.message,
                "passed": a.passed,
                "message": a.message,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    });
    Ok(serde_json::to_string_pretty(&report)?)
}

/// Generate a JUnit XML report from test results.
pub fn generate_junit_xml(results: &[TestResult]) -> Result<String> {
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    let total = results.len();
    let failures = results.iter().filter(|r| matches!(r.status, crate::testing::TestStatus::Failed)).count();
    xml.push_str(&format!(
        r#"<testsuite name="reqforge" tests="{}" failures="{}" errors="0">"#,
        total, failures
    ));
    xml.push('\n');
    for r in results {
        let class = r.name.replace(' ', "_");
        let status = r.status;
        let passed = matches!(status, crate::testing::TestStatus::Passed);
        if passed {
            xml.push_str(&format!(
                r#"  <testcase name="{}" classname="{}" time="{:.3}" />"#,
                r.name, class, r.duration_ms as f64 / 1000.0
            ));
        } else {
            xml.push_str(&format!(
                r#"  <testcase name="{}" classname="{}" time="{:.3}">"#,
                r.name, class, r.duration_ms as f64 / 1000.0
            ));
            xml.push_str(r#"<failure>"#);
            xml.push_str("Test failed:");
            for a in &r.assertions {
                if !a.passed {
                    xml.push_str(&format!("\n  {}: {}", a.message, a.message));
                }
            }
            xml.push_str(r#"</failure>"#);
            xml.push_str("</testcase>\n");
        }
    }
    xml.push_str("</testsuite>\n");
    Ok(xml)
}

/// Generate a self-contained HTML report from test results.
pub fn generate_html(results: &[TestResult]) -> Result<String> {
    let passed = results.iter().filter(|r| matches!(r.status, crate::testing::TestStatus::Passed)).count();
    let failed = results.iter().filter(|r| matches!(r.status, crate::testing::TestStatus::Failed)).count();
    let total = results.len();
    let pass_pct = if total > 0 { (passed as f64 / total as f64) * 100.0 } else { 0.0 };

    let mut rows = String::new();
    for r in results {
        let status_icon = match r.status {
            crate::testing::TestStatus::Passed => "✅",
            crate::testing::TestStatus::Failed => "❌",
            _ => "⏭",
        };
        let mut detail = String::new();
        for a in &r.assertions {
            let a_icon = if a.passed { "✅" } else { "❌" };
            detail.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                a_icon, a.message, a.message
            ));
        }
        rows.push_str(&format!(
            r#"
            <tr>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}ms</td>
            </tr>
            <tr class="detail-row">
                <td colspan="4">
                    <table class="inner"><thead><tr><th></th><th>Assertion</th><th>Message</th></tr></thead><tbody>{}</tbody></table>
                </td>
            </tr>"#,
            status_icon, r.name, r.status, r.duration_ms, detail
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>ReqForge Test Report</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 960px; margin: 0 auto; padding: 20px; background: #f8f9fa; color: #333; }}
  h1 {{ color: #1a1a2e; }}
  .summary {{ display: flex; gap: 20px; margin: 20px 0; }}
  .stat {{ background: white; padding: 15px 24px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }}
  .stat .num {{ font-size: 28px; font-weight: bold; }}
  .stat.pass .num {{ color: #22c55e; }}
  .stat.fail .num {{ color: #ef4444; }}
  table {{ width: 100%; border-collapse: collapse; background: white; border-radius: 8px; overflow: hidden; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }}
  th {{ background: #1a1a2e; color: white; padding: 12px 16px; text-align: left; }}
  td {{ padding: 10px 16px; border-bottom: 1px solid #eee; }}
  .detail-row td {{ padding: 0; background: #f1f5f9; }}
  .inner {{ margin: 12px 16px; width: calc(100% - 32px); }}
  .inner th {{ background: #475569; font-size: 13px; }}
</style>
</head>
<body>
<h1>ReqForge Test Report</h1>
<div class="summary">
  <div class="stat"><div>Total</div><div class="num">{}</div></div>
  <div class="stat pass"><div>Passed</div><div class="num">{}</div></div>
  <div class="stat fail"><div>Failed</div><div class="num">{}</div></div>
  <div class="stat"><div>Pass Rate</div><div class="num">{:.1}%</div></div>
</div>
<table>
<thead><tr><th></th><th>Test</th><th>Status</th><th>Duration</th></tr></thead>
<tbody>{}</tbody>
</table>
</body>
</html>"#,
        total, passed, failed, pass_pct, rows
    );

    Ok(html)
}

/// Generate a Markdown report from test results.
pub fn generate_markdown(results: &[TestResult]) -> Result<String> {
    let total = results.len();
    let passed = results.iter().filter(|r| matches!(r.status, crate::testing::TestStatus::Passed)).count();
    let failed = results.iter().filter(|r| matches!(r.status, crate::testing::TestStatus::Failed)).count();
    let pass_pct = if total > 0 { (passed as f64 / total as f64) * 100.0 } else { 0.0 };

    let mut md = String::new();
    md.push_str("# Test Report\n\n");
    md.push_str(&format!("**Total:** {} | **Passed:** {} | **Failed:** {} | **Pass rate:** {:.1}%\n\n", total, passed, failed, pass_pct));
    md.push_str("| Status | Test | Duration |\n");
    md.push_str("|--------|------|----------|\n");
    for r in results {
        let icon = match r.status {
            crate::testing::TestStatus::Passed => "✅",
            crate::testing::TestStatus::Failed => "❌",
            _ => "⏭",
        };
        md.push_str(&format!("| {} | {} | {}ms |\n", icon, r.name, r.duration_ms));
        for a in &r.assertions {
            if !a.passed {
                md.push_str(&format!("| | ⚠ {} | |\n", a.message));
            }
        }
    }
    Ok(md)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Assertion, AssertionType, TestResult, TestStatus};
    use crate::request::Response;

    fn dummy_result(name: &str, passed: bool) -> TestResult {
        let status = if passed { TestStatus::Passed } else { TestStatus::Failed };
        TestResult {
            name: name.to_string(),
            status,
            assertions: vec![],
            duration_ms: 42,
        }
    }

    #[test]
    fn test_generate_json() {
        let results = vec![dummy_result("test-a", true), dummy_result("test-b", false)];
        let json = generate_json(&results).unwrap();
        assert!(json.contains("test-a"));
        assert!(json.contains("test-b"));
        assert!(json.contains("passed"));
    }

    #[test]
    fn test_junit_xml() {
        let results = vec![dummy_result("test-a", true), dummy_result("test-b", false)];
        let xml = generate_junit_xml(&results).unwrap();
        assert!(xml.contains("testsuite"));
        assert!(xml.contains("failure"));
    }

    #[test]
    fn test_html_report() {
        let results = vec![dummy_result("test-a", true), dummy_result("test-b", false)];
        let html = generate_html(&results).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("test-a"));
        assert!(html.contains("50.0%"));
    }
}
