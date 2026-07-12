//! Lightweight HTTP load tester built into ReqForge.
//!
//! Spawns concurrent workers against a target URL, measures per-request
//! latency, and reports summary statistics (min, max, p50, p95, p99, RPS,
//! error rate). No external dependencies beyond reqwest + tokio.
//!
//! Usage:
//! ```ignore
//! let result = loadtest::run(LoadTestConfig {
//!     url: "https://api.example.com/users".into(),
//!     method: "GET".into(),
//!     concurrency: 10,
//!     total_requests: 100,
//!     ..Default::default()
//! }).await?;
//! println!("p50: {}ms, p99: {}ms, rps: {}", result.p50_ms, result.p99_ms, result.rps);
//! ```

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Semaphore;

/// Configuration for a load test run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestConfig {
    /// Target URL (e.g. "https://api.example.com/users")
    pub url: String,
    /// HTTP method (GET, POST, etc.)
    #[serde(default = "default_get")]
    pub method: String,
    /// Request body (empty for GET)
    #[serde(default)]
    pub body: String,
    /// Headers to include
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    /// Number of concurrent workers
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    /// Total number of requests to send
    #[serde(default = "default_total")]
    pub total_requests: usize,
    /// Seconds to wait before timing out a request
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Warm-up requests before measurement starts
    #[serde(default = "default_warmup")]
    pub warmup: usize,
}

fn default_get() -> String { "GET".into() }
fn default_concurrency() -> usize { 10 }
fn default_total() -> usize { 100 }
fn default_timeout() -> u64 { 30 }
fn default_warmup() -> usize { 5 }

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost".into(),
            method: "GET".into(),
            body: String::new(),
            headers: Vec::new(),
            concurrency: default_concurrency(),
            total_requests: default_total(),
            timeout_secs: default_timeout(),
            warmup: default_warmup(),
        }
    }
}

/// Summary results from a load test run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestResult {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub rps: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub avg_ms: f64,
    /// Duration of the test in milliseconds.
    pub duration_ms: u64,
}

/// Run a load test. Spawns `config.concurrency` workers that each send
/// a portion of the total requests through a shared reqwest Client.
pub async fn run(config: LoadTestConfig) -> Result<LoadTestResult> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .build()
        .map_err(|e| Error::other(format!("loadtest client: {e}")))?;

    let latencies: Arc<tokio::sync::Mutex<Vec<f64>>> =
        Arc::new(tokio::sync::Mutex::new(Vec::with_capacity(config.total_requests)));
    let succeeded = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(config.concurrency));
    let total = config.total_requests;
    let start = Instant::now();

    // Warmup phase.
    if config.warmup > 0 {
        for _ in 0..config.warmup {
            let _ = send_request(&client, &config).await;
        }
    }

    // Main test phase.
    let mut handles = Vec::with_capacity(config.concurrency);
    for _ in 0..config.concurrency {
        let client = client.clone();
        let config = config.clone();
        let latencies = latencies.clone();
        let succeeded = succeeded.clone();
        let failed = failed.clone();
        let semaphore = semaphore.clone();

        let handle = tokio::spawn(async move {
            loop {
                let _permit = semaphore.acquire().await.unwrap();
                let started = Instant::now();
                let result = send_request(&client, &config).await;
                let elapsed = started.elapsed().as_secs_f64() * 1000.0;

                let mut lat = latencies.lock().await;
                lat.push(elapsed);
                if result { succeeded.fetch_add(1, Ordering::Relaxed); }
                else { failed.fetch_add(1, Ordering::Relaxed); }

                // Stop when we've done our share.
                if lat.len() >= total {
                    break;
                }
            }
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.await;
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let lat = latencies.lock().await.clone();
    let success = succeeded.load(Ordering::Relaxed);
    let fail = failed.load(Ordering::Relaxed);

    if lat.is_empty() {
        return Err(Error::other("load test produced no data"));
    }

    let mut sorted = lat.clone();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = sorted.len();
    let min_ms = sorted.first().copied().unwrap_or(0.0);
    let max_ms = sorted.last().copied().unwrap_or(0.0);
    let avg_ms = sorted.iter().sum::<f64>() / len as f64;
    let p50 = percentile(&sorted, 50.0);
    let p95 = percentile(&sorted, 95.0);
    let p99 = percentile(&sorted, 99.0);

    let rps = if duration_ms > 0 {
        (success as f64) / (duration_ms as f64 / 1000.0)
    } else {
        0.0
    };

    Ok(LoadTestResult {
        total: success + fail,
        succeeded: success,
        failed: fail,
        rps,
        min_ms,
        max_ms,
        p50_ms: p50,
        p95_ms: p95,
        p99_ms: p99,
        avg_ms,
        duration_ms,
    })
}

async fn send_request(client: &reqwest::Client, config: &LoadTestConfig) -> bool {
    let method = match config.method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "PATCH" => reqwest::Method::PATCH,
        "DELETE" => reqwest::Method::DELETE,
        "HEAD" => reqwest::Method::HEAD,
        _ => reqwest::Method::GET,
    };

    let mut req = client.request(method, &config.url);
    for (k, v) in &config.headers {
        req = req.header(k, v);
    }
    if !config.body.is_empty() {
        req = req.body(config.body.clone());
    }

    match req.send().await {
        Ok(r) => r.status().is_success(),
        Err(_) => false,
    }
}

fn percentile(sorted: &[f64], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((pct / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_calculation() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        assert!((percentile(&data, 50.0) - 6.0).abs() < 0.01);
        assert!((percentile(&data, 95.0) - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_config_defaults() {
        let cfg = LoadTestConfig::default();
        assert_eq!(cfg.concurrency, 10);
        assert_eq!(cfg.total_requests, 100);
    }
}
