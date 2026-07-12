use crate::error::Result;
use crate::request::{Request, Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// A single row in the request history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub url: String,
    pub status: u16,
    pub status_text: String,
    pub duration_ms: u64,
    pub size_bytes: u64,
    /// The full request that was sent (for replay)
    pub request: Request,
    /// The full response (for inspection without re-sending)
    pub response: Response,
}

impl HistoryEntry {
    pub fn from_response(request: Request, response: Response) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            method: request.method.to_string(),
            url: request.url.clone(),
            status: response.status,
            status_text: response.status_text.clone(),
            duration_ms: response.timing.total_ms,
            size_bytes: response.size.total,
            request,
            response,
        }
    }

    pub fn method_color(&self) -> &'static str {
        match self.method.as_str() {
            "GET" => "green",
            "POST" => "orange",
            "PUT" => "blue",
            "PATCH" => "purple",
            "DELETE" => "red",
            _ => "gray",
        }
    }

    /// True if the response status is in the 2xx-3xx range
    pub fn is_success(&self) -> bool {
        (200..400).contains(&self.status)
    }
}

/// Append-only history backed by a JSON-lines file
///
/// Each entry is written as a single line of JSON followed by `\n`.
/// Reading the file streams lines back in chronological order.
pub struct HistoryStorage {
    file_path: PathBuf,
    limit: usize,
}

impl HistoryStorage {
    /// Create a storage rooted at the given directory. The file is created
    /// on first write.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            file_path: workspace_root.into().join("history.jsonl"),
            limit: super::DEFAULT_HISTORY_LIMIT,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn path(&self) -> &Path {
        &self.file_path
    }

    /// Append a new history entry. Prunes oldest entries beyond the limit.
    pub async fn append(&self, entry: HistoryEntry) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let line = serde_json::to_string(&entry)?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .await?;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        // Best-effort prune (ignore errors)
        let _ = self.prune().await;

        Ok(())
    }

    /// Read up to `limit` most-recent entries (newest first).
    pub async fn list(&self, limit: Option<usize>) -> Result<Vec<HistoryEntry>> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&self.file_path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        let mut entries: Vec<HistoryEntry> = Vec::new();
        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<HistoryEntry>(&line) {
                entries.push(entry);
            }
        }

        // Newest first
        entries.reverse();

        if let Some(limit) = limit {
            entries.truncate(limit);
        }

        Ok(entries)
    }

    /// Clear all history
    pub async fn clear(&self) -> Result<()> {
        if self.file_path.exists() {
            fs::remove_file(&self.file_path).await?;
        }
        Ok(())
    }

    /// Trim to the configured limit, keeping the newest entries
    async fn prune(&self) -> Result<()> {
        let all = self.list(None).await?;
        if all.len() <= self.limit {
            return Ok(());
        }

        let to_keep: Vec<&HistoryEntry> = all.iter().take(self.limit).collect();
        let tmp = self.file_path.with_extension("jsonl.tmp");
        let mut file = fs::File::create(&tmp).await?;
        for entry in to_keep.iter().rev() {
            let line = serde_json::to_string(entry)?;
            file.write_all(line.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }
        file.flush().await?;
        drop(file);

        fs::rename(&tmp, &self.file_path).await?;
        Ok(())
    }

    /// Filter helpers
    pub async fn search(&self, needle: &str, limit: usize) -> Result<Vec<HistoryEntry>> {
        let needle_lower = needle.to_lowercase();
        let all = self.list(Some(limit.max(100))).await?;
        Ok(all
            .into_iter()
            .filter(|e| {
                e.url.to_lowercase().contains(&needle_lower)
                    || e.method.to_lowercase().contains(&needle_lower)
                    || e.status_text.to_lowercase().contains(&needle_lower)
            })
            .take(limit)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{Body, HttpMethod, ResponseBody, ResponseSize, ResponseTiming};

    fn make_entry(method: HttpMethod, url: &str, status: u16) -> HistoryEntry {
        let request = Request::new(method, url);
        let response = Response {
            status,
            status_text: if status < 300 { "OK" } else { "Err" }.to_string(),
            headers: Default::default(),
            body: ResponseBody {
                content: Vec::new(),
                content_type: Some("application/json".to_string()),
                is_text: true,
            },
            cookies: Vec::new(),
            timing: ResponseTiming {
                total_ms: 50,
                ..Default::default()
            },
            size: ResponseSize::default(),
            url: url.to_string(),
            protocol: "HTTP/1.1".to_string(),
        };
        HistoryEntry::from_response(request, response)
    }

    #[tokio::test]
    async fn test_append_and_list() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = HistoryStorage::new(tmp.path());

        storage
            .append(make_entry(HttpMethod::Get, "https://a.com", 200))
            .await
            .unwrap();
        storage
            .append(make_entry(HttpMethod::Post, "https://b.com", 201))
            .await
            .unwrap();
        let _ = Body::default();

        let entries = storage.list(None).await.unwrap();
        assert_eq!(entries.len(), 2);
        // Newest first
        assert_eq!(entries[0].method, "POST");
        assert_eq!(entries[1].method, "GET");
    }

    #[tokio::test]
    async fn test_prune() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = HistoryStorage::new(tmp.path()).with_limit(3);

        for i in 0..5 {
            storage
                .append(make_entry(HttpMethod::Get, &format!("https://x.com/{}", i), 200))
                .await
                .unwrap();
        }

        let entries = storage.list(None).await.unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[tokio::test]
    async fn test_search() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = HistoryStorage::new(tmp.path());

        storage
            .append(make_entry(HttpMethod::Get, "https://api.example.com/users", 200))
            .await
            .unwrap();
        storage
            .append(make_entry(HttpMethod::Get, "https://other.com", 200))
            .await
            .unwrap();

        let found = storage.search("example", 50).await.unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].url.contains("example"));
    }

    #[tokio::test]
    async fn test_clear() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = HistoryStorage::new(tmp.path());

        storage
            .append(make_entry(HttpMethod::Get, "https://x.com", 200))
            .await
            .unwrap();
        storage.clear().await.unwrap();
        let entries = storage.list(None).await.unwrap();
        assert_eq!(entries.len(), 0);
    }
}
