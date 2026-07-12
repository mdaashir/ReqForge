//! Anonymous telemetry + crash reporting for ReqForge.
//!
//! Two event types:
//! - **UsageEvent**: feature usage counters, sent periodically (no request bodies).
//! - **CrashReport**: opt-in crash dump with app version, OS, stack trace, optional email.
//!
//! The API surface is designed so the desktop shell controls data flow:
//!  1. User explicitly opts in via Settings (clears the kill switch).
//!  2. Backend starts a background task that dumps counters and unwinds.
//!  3. The `shutdown()` fn flushes the pending batch before exit.
//!
//! The public API is a simple struct — no global state, zero tokio magic.
//! Consumers (Tauri commands, CLI) own the `TelemetryClient` and call it.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Minimal telemetry event. Nothing here ever contains request bodies,
/// URLs, hostnames, or auth headers. Only usage counts + basic env data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    /// App version (e.g. "0.1.0").
    pub app_version: String,
    /// OS family (windows, macos, linux).
    pub os: String,
    /// Feature name being counted (e.g. "request.send", "import.postman").
    pub feature: String,
    /// How many times since last upload.
    pub count: u64,
    /// Unix timestamp of the event window.
    pub window_ts: i64,
}

/// A crash report. Only sent when the user explicitly opts in to
/// identifiable data (email is optional).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    pub app_version: String,
    pub os: String,
    /// Error message / panic payload.
    pub message: String,
    /// Optional email the user typed in the "I want help" dialog.
    pub email: Option<String>,
    /// Stack trace from the crash site.
    pub stack: Vec<String>,
    /// Timestamp when the crash happened.
    pub crashed_at: i64,
}

/// Configuration for the telemetry client.
pub struct TelemetryConfig {
    /// URL of the ingestion server.
    pub endpoint: String,
    /// App version sent with every event.
    pub app_version: String,
    /// OS string sent with every event.
    pub os: String,
}

/// Generic result for telemetry operations. We keep it simple because
/// telemetry errors should never propagate to the caller's happy path.
type TelemetryResult = Result<(), String>;

/// A client that accumulates usage counters and sends them to the
/// ingestion server. Callers increment features via `track()` and
/// flush via `flush()` or let the background task do it.
pub struct TelemetryClient {
    config: TelemetryConfig,
    counters: std::collections::HashMap<String, u64>,
    enabled: bool,
    client: reqwest::Client,
}

impl TelemetryClient {
    pub fn new(config: TelemetryConfig) -> Self {
        Self {
            counters: std::collections::HashMap::new(),
            enabled: false,
            client: reqwest::Client::new(),
            config,
        }
    }

    /// Enable or disable collection. Persistent across calls.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Increment a usage counter by one. No-op when disabled.
    pub fn track(&mut self, feature: &str) {
        if self.enabled {
            *self.counters.entry(feature.to_string()).or_insert(0) += 1;
        }
    }

    /// Submit a batch of accumulated counters and reset them.
    pub async fn flush(&mut self) -> TelemetryResult {
        if !self.enabled || self.counters.is_empty() {
            return Ok(());
        }

        let counters = std::mem::take(&mut self.counters);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut events = Vec::new();
        for (feature, count) in counters {
            events.push(UsageEvent {
                app_version: self.config.app_version.clone(),
                os: self.config.os.clone(),
                feature,
                count,
                window_ts: now,
            });
        }

        let resp = self
            .client
            .post(format!(
                "{}/v1/usage",
                self.config.endpoint.trim_end_matches('/')
            ))
            .json(&events)
            .send()
            .await
            .map_err(|e| format!("telemetry upload: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("telemetry server returned {}", resp.status()));
        }
        Ok(())
    }

    /// Submit a crash report. Called once on panic capture.
    pub async fn report_crash(&self, report: CrashReport) -> TelemetryResult {
        if !self.enabled {
            return Ok(());
        }
        let resp = self
            .client
            .post(format!(
                "{}/v1/crash",
                self.config.endpoint.trim_end_matches('/')
            ))
            .json(&report)
            .send()
            .await
            .map_err(|e| format!("crash report: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("crash server returned {}", resp.status()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_and_flush() {
        let mut client = TelemetryClient::new(TelemetryConfig {
            endpoint: "http://localhost:9999".into(),
            app_version: "0.1.0".into(),
            os: "test".into(),
        });
        client.set_enabled(true);

        client.track("request.send");
        client.track("request.send");
        client.track("import.postman");
        assert_eq!(client.counters.get("request.send"), Some(&2));
    }

    #[test]
    fn test_disabled_by_default() {
        let mut client = TelemetryClient::new(TelemetryConfig {
            endpoint: "http://localhost:9999".into(),
            app_version: "0.1.0".into(),
            os: "test".into(),
        });
        client.track("request.send");
        assert!(client.counters.is_empty());
    }
}
