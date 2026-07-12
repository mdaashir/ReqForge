//! SQLite-backed history storage.
//!
//! Faster queries and better concurrency than the JSONL-based store.
//! Same public API as `super::storage` so the rest of the crate can use
//! either interchangeably. On first open we auto-migrate any existing
//! JSONL file at `<workspace>/.reqforge/history.jsonl` into the SQLite
//! database.

use crate::error::{Error, Result};
use crate::history::storage::HistoryEntry;
use crate::request::{HttpMethod, Request, Response};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// SQLite-backed history store.
///
/// Threadsafe — wraps a single `Mutex<Connection>`. `Connection` from
/// rusqlite isn't `Sync`, so we can't share it across threads; the
/// mutex is sufficient for our async workload.
#[derive(Clone)]
pub struct SqliteHistoryStorage {
    conn: Arc<Mutex<Connection>>,
    jsonl_path: PathBuf,
}

impl SqliteHistoryStorage {
    /// Open or create the history database at `<workspace>/.reqforge/history.db`.
    /// Also looks for an existing JSONL file at the same path to migrate.
    pub async fn open(workspace_root: impl AsRef<Path>) -> Result<Self> {
        let dir = workspace_root.as_ref().join(".reqforge");
        tokio::fs::create_dir_all(&dir).await?;
        let db_path = dir.join("history.db");
        let jsonl_path = dir.join("history.jsonl");

        let conn = tokio::task::spawn_blocking({
            let db_path = db_path.clone();
            move || Connection::open(&db_path).map_err(|e| Error::other(e.to_string()))
        })
        .await
        .map_err(|e| Error::other(format!("sqlite task join: {e}")))??;

        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
            jsonl_path,
        };
        storage.migrate().await?;
        storage.create_schema().await?;
        Ok(storage)
    }

    async fn create_schema(&self) -> Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.blocking_lock();
            conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS history (
                    id          TEXT PRIMARY KEY,
                    timestamp   INTEGER NOT NULL,
                    method      TEXT NOT NULL,
                    url         TEXT NOT NULL,
                    status      INTEGER NOT NULL,
                    status_text TEXT NOT NULL,
                    duration_ms INTEGER NOT NULL,
                    size_bytes  INTEGER NOT NULL,
                    request     TEXT NOT NULL,
                    response    TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_history_timestamp ON history(timestamp DESC);
                CREATE INDEX IF NOT EXISTS idx_history_method ON history(method);
                "#,
            )
            .map_err(|e| Error::other(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| Error::other(format!("create_schema task: {e}")))?
    }

    /// If a JSONL file exists from a previous install, import its rows
    /// into SQLite. Skips rows that already exist (by id).
    async fn migrate(&self) -> Result<()> {
        if !self.jsonl_path.exists() {
            return Ok(());
        }
        let path = self.jsonl_path.clone();
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.blocking_lock();
            let file = std::fs::File::open(&path)?;
            let reader = std::io::BufReader::new(file);
            use std::io::BufRead;
            let mut total = 0;
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let entry: HistoryEntry = match serde_json::from_str(&line) {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                self::migrate_row(&conn, entry);
                total += 1;
            }
            tracing::info!(imported = total, "migrated JSONL history");
            Ok(())
        })
        .await
        .map_err(|e| Error::other(format!("migration task: {e}")))?
    }

    /// Append a new entry to the history.
    pub async fn append(&self, entry: HistoryEntry) -> Result<()> {
        let conn = self.conn.clone();
        let id = entry.id.clone();
        let timestamp_ms = entry.timestamp.timestamp_millis();
        let method = entry.method.clone();
        let url = entry.url.clone();
        let status = entry.status;
        let status_text = entry.status_text.clone();
        let duration_ms = entry.duration_ms;
        let size_bytes = entry.size_bytes;
        let request_json = serde_json::to_string(&entry.request)?;
        let response_json = serde_json::to_string(&entry.response)?;

        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.blocking_lock();
            conn.execute(
                "INSERT OR REPLACE INTO history
                 (id, timestamp, method, url, status, status_text, duration_ms, size_bytes, request, response)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    id, timestamp_ms, method, url, status, status_text,
                    duration_ms as i64, size_bytes as i64, request_json, response_json
                ],
            )
            .map_err(|e| Error::other(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| Error::other(format!("insert task: {e}")))?
    }

    /// List recent entries, newest first.
    pub async fn list(&self, limit: Option<usize>) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<HistoryEntry>> {
            let conn = conn.blocking_lock();
            let limit = limit.unwrap_or(1000) as i64;
            let mut stmt = conn
                .prepare(
                    "SELECT id, timestamp, method, url, status, status_text, duration_ms, size_bytes, request, response
                     FROM history ORDER BY timestamp DESC LIMIT ?1",
                )
                .map_err(|e| Error::other(e.to_string()))?;
            let entries = stmt
                .query_map(params![limit], |row| Ok(row_to_entry(row)))
                .map_err(|e| Error::other(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect::<Vec<_>>();
            Ok(entries)
        })
        .await
        .map_err(|e| Error::other(format!("list task: {e}")))?
    }

    /// Search entries by needle (matches against URL).
    pub async fn search(&self, needle: &str, limit: usize) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn.clone();
        let needle = needle.to_string();
        let limit = limit as i64;
        tokio::task::spawn_blocking(move || -> Result<Vec<HistoryEntry>> {
            let conn = conn.blocking_lock();
            let pattern = format!("%{}%", needle.to_lowercase());
            let mut stmt = conn
                .prepare(
                    "SELECT id, timestamp, method, url, status, status_text, duration_ms, size_bytes, request, response
                     FROM history WHERE LOWER(url) LIKE ?1 ORDER BY timestamp DESC LIMIT ?2",
                )
                .map_err(|e| Error::other(e.to_string()))?;
            let entries = stmt
                .query_map(params![pattern, limit], |row| Ok(row_to_entry(row)))
                .map_err(|e| Error::other(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect::<Vec<_>>();
            Ok(entries)
        })
        .await
        .map_err(|e| Error::other(format!("search task: {e}")))?
    }

    /// Clear all history.
    pub async fn clear(&self) -> Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.blocking_lock();
            conn.execute("DELETE FROM history", [])
                .map_err(|e| Error::other(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| Error::other(format!("clear task: {e}")))?
    }

    /// Prune entries older than `max_age_days`.
    pub async fn prune(&self, max_age_days: u32) -> Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.blocking_lock();
            let cutoff_ms = (chrono::Utc::now().timestamp_millis())
                - (max_age_days as i64 * 86_400 * 1000);
            conn.execute(
                "DELETE FROM history WHERE timestamp < ?1",
                params![cutoff_ms],
            )
            .map_err(|e| Error::other(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| Error::other(format!("prune task: {e}")))?
    }

    /// Return total row count. Useful for the UI.
    pub async fn count(&self) -> Result<u64> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || -> Result<u64> {
            let conn = conn.blocking_lock();
            let n: i64 = conn
                .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
                .map_err(|e| Error::other(e.to_string()))?;
            Ok(n as u64)
        })
        .await
        .map_err(|e| Error::other(format!("count task: {e}")))?
    }
}

fn row_to_entry(row: &rusqlite::Row) -> HistoryEntry {
    let ts_ms: i64 = row.get(1).unwrap_or(0);
    let timestamp = DateTime::<Utc>::from_timestamp_millis(ts_ms).unwrap_or_else(Utc::now);
    let request_json: String = row.get(8).unwrap_or_default();
    let response_json: String = row.get(9).unwrap_or_default();
    HistoryEntry {
        id: row.get(0).unwrap_or_default(),
        timestamp,
        method: row.get(2).unwrap_or_default(),
        url: row.get(3).unwrap_or_default(),
        status: row.get::<_, i64>(4).unwrap_or(0) as u16,
        status_text: row.get(5).unwrap_or_default(),
        duration_ms: row.get::<_, i64>(6).unwrap_or(0) as u64,
        size_bytes: row.get::<_, i64>(7).unwrap_or(0) as u64,
        request: serde_json::from_str(&request_json).unwrap_or_else(|_| {
            // Fallback: empty request. This shouldn't happen if the row was
            // inserted via append().
            Request::new(HttpMethod::Get, String::new())
        }),
        response: serde_json::from_str(&response_json).unwrap_or_else(|_| {
            // Fallback empty response. Same caveat.
            Response {
                status: 0,
                status_text: String::new(),
                headers: Default::default(),
                body: Default::default(),
                cookies: Vec::new(),
                timing: Default::default(),
                size: Default::default(),
                url: String::new(),
                protocol: String::new(),
            }
        }),
    }
}

/// Helper: insert one row into history. Returns silently if the row
/// already exists (id is the PK so this means a duplicate UUID, very
/// unlikely but we handle it).
fn migrate_row(conn: &Connection, entry: HistoryEntry) {
    let _ = conn.execute(
        "INSERT OR IGNORE INTO history
         (id, timestamp, method, url, status, status_text, duration_ms, size_bytes, request, response)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            entry.id,
            entry.timestamp.timestamp_millis(),
            entry.method,
            entry.url,
            entry.status as i64,
            entry.status_text,
            entry.duration_ms as i64,
            entry.size_bytes as i64,
            serde_json::to_string(&entry.request).unwrap_or_default(),
            serde_json::to_string(&entry.response).unwrap_or_default(),
        ],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{BodyMode, HttpMethod};
    use tempfile::TempDir;

    fn sample_request() -> Request {
        Request {
            id: "test".into(),
            name: "test".into(),
            method: HttpMethod::Get,
            url: "https://api.example.com/users".into(),
            headers: vec![],
            params: vec![],
            body: crate::request::Body { mode: BodyMode::None, content: String::new(), content_type: None },
            auth: None,
            settings: Default::default(),
            pre_request_script: None,
            post_response_script: None,
            test_script: None,
            description: None,
        }
    }

    fn sample_response() -> Response {
        Response {
            status: 200,
            status_text: "OK".into(),
            headers: Default::default(),
            body: Default::default(),
            cookies: Vec::new(),
            timing: Default::default(),
            size: Default::default(),
            url: "https://api.example.com/users".into(),
            protocol: "HTTP/1.1".into(),
        }
    }

    #[tokio::test]
    async fn test_append_and_list() {
        let tmp = TempDir::new().unwrap();
        let store = SqliteHistoryStorage::open(tmp.path()).await.unwrap();
        let req = sample_request();
        let resp = sample_response();
        let entry = HistoryEntry::from_response(req.clone(), resp.clone());
        store.append(entry).await.unwrap();

        let entries = store.list(Some(10)).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].method, "GET");
        assert_eq!(entries[0].url, req.url);
    }

    #[tokio::test]
    async fn test_search_by_url() {
        let tmp = TempDir::new().unwrap();
        let store = SqliteHistoryStorage::open(tmp.path()).await.unwrap();

        let mut req = sample_request();
        req.url = "https://api.example.com/users".into();
        store.append(HistoryEntry::from_response(req, sample_response())).await.unwrap();

        let mut req = sample_request();
        req.url = "https://api.example.com/posts".into();
        store.append(HistoryEntry::from_response(req, sample_response())).await.unwrap();

        let results = store.search("users", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].url.contains("users"));
    }

    #[tokio::test]
    async fn test_clear() {
        let tmp = TempDir::new().unwrap();
        let store = SqliteHistoryStorage::open(tmp.path()).await.unwrap();
        store.append(HistoryEntry::from_response(sample_request(), sample_response())).await.unwrap();
        store.clear().await.unwrap();
        let n = store.count().await.unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn test_count() {
        let tmp = TempDir::new().unwrap();
        let store = SqliteHistoryStorage::open(tmp.path()).await.unwrap();
        assert_eq!(store.count().await.unwrap(), 0);
        store.append(HistoryEntry::from_response(sample_request(), sample_response())).await.unwrap();
        store.append(HistoryEntry::from_response(sample_request(), sample_response())).await.unwrap();
        assert_eq!(store.count().await.unwrap(), 2);
    }
}
