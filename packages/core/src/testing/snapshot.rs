//! Snapshot testing — golden file comparison for API responses.
//!
//! A snapshot captures a response body (formatted JSON) and stores it at
//! `.reqforge/snapshots/{request_id}.snap.json`. On subsequent runs the
//! response is compared against the golden file; any diff is reported as a
//! test failure.
//!
//! ## Usage
//!
//! ```ignore
//! let snap = SnapshotStorage::new(workspace_root);
//! snap.save("req-001", &response_body).unwrap();
//! assert!(snap.match_or_update("req-001", &new_response).unwrap());
//! ```

use crate::error::{Error, Result};
use serde_json::Value;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

/// Manages golden snapshot files on disk.
pub struct SnapshotStorage {
    snapshots_dir: PathBuf,
    update_mode: bool,
}

impl SnapshotStorage {
    /// Create storage rooted at `<workspace>/.reqforge/snapshots/`.
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        let snapshots_dir = workspace_root.as_ref().join(".reqforge").join("snapshots");
        Self {
            snapshots_dir,
            update_mode: false,
        }
    }

    /// Enable update mode: instead of failing, overwrite golden files.
    pub fn set_update_mode(&mut self, enabled: bool) {
        self.update_mode = enabled;
    }

    /// Save a response as the golden snapshot for `request_id`.
    /// The response is pretty-printed JSON — non-JSON responses are stored raw.
    pub fn save(&self, request_id: &str, response_body: &str) -> Result<()> {
        std::fs::create_dir_all(&self.snapshots_dir)
            .map_err(|e| Error::other(format!("snapshot dir: {e}")))?;

        let formatted = if let Ok(val) = serde_json::from_str::<Value>(response_body) {
            serde_json::to_string_pretty(&val)?
        } else {
            response_body.to_string()
        };

        let path = self.snapshots_dir.join(format!("{}.snap.json", request_id));
        std::fs::write(&path, &formatted)
            .map_err(|e| Error::other(format!("snapshot write: {e}")))?;
        Ok(())
    }

    /// Load the golden snapshot for `request_id`.
    pub fn load(&self, request_id: &str) -> Result<Option<String>> {
        let path = self.snapshots_dir.join(format!("{}.snap.json", request_id));
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| Error::other(format!("snapshot read: {e}")))?;
        Ok(Some(content))
    }

    /// Delete the golden snapshot for `request_id`.
    pub fn delete(&self, request_id: &str) -> Result<()> {
        let path = self.snapshots_dir.join(format!("{}.snap.json", request_id));
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| Error::other(format!("snapshot delete: {e}")))?;
        }
        Ok(())
    }

    /// List all snapshot IDs.
    pub fn list(&self) -> Result<Vec<String>> {
        let mut ids = Vec::new();
        if !self.snapshots_dir.exists() {
            return Ok(ids);
        }
        for entry in std::fs::read_dir(&self.snapshots_dir)
            .map_err(|e| Error::other(format!("snapshot list: {e}")))?
        {
            let entry = entry.map_err(|e| Error::other(format!("snapshot entry: {e}")))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(id) = name.strip_suffix(".snap.json") {
                ids.push(id.to_string());
            }
        }
        Ok(ids)
    }

    /// Compare a new response against the golden snapshot.
    ///
    /// Returns `Ok(true)` if they match, `Ok(false)` if they differ, or `Err`
    /// if no golden file exists or IO fails.
    ///
    /// In update mode, always saves the new response as the golden file and
    /// returns `true`.
    pub fn match_or_update(&self, request_id: &str, response_body: &str) -> Result<bool> {
        if self.update_mode {
            self.save(request_id, response_body)?;
            return Ok(true);
        }

        let golden = match self.load(request_id)? {
            Some(g) => g,
            None => return Err(Error::assertion(format!(
                "no snapshot found for request '{}' — send the request first to create one",
                request_id
            ))),
        };

        // Normalise both sides (pretty-print JSON) before comparing
        let golden_normalised = normalise_json(&golden);
        let actual_normalised = normalise_json(response_body);

        Ok(golden_normalised == actual_normalised)
    }

    /// Return the path to the snapshots directory.
    pub fn dir(&self) -> &Path {
        &self.snapshots_dir
    }
}

/// Normalise a response body for comparison: parse and re-serialize JSON,
/// or trim whitespace for non-JSON.
fn normalise_json(input: &str) -> String {
    if let Ok(val) = serde_json::from_str::<Value>(input) {
        serde_json::to_string_pretty(&val).unwrap_or_else(|_| input.to_string())
    } else {
        input.trim().to_string()
    }
}

/// Snapshot assertion result.
#[derive(Debug, Clone)]
pub struct SnapshotDiff {
    pub request_id: String,
    pub golden: String,
    pub actual: String,
    pub matched: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let storage = SnapshotStorage::new(tmp.path());
        storage.save("req-1", r#"{"name":"Alice"}"#).unwrap();
        let loaded = storage.load("req-1").unwrap().unwrap();
        assert!(loaded.contains("Alice"));
    }

    #[test]
    fn test_match_identical() {
        let tmp = TempDir::new().unwrap();
        let storage = SnapshotStorage::new(tmp.path());
        storage.save("req-1", r#"{"id":1}"#).unwrap();
        assert!(storage.match_or_update("req-1", r#"{"id":1}"#).unwrap());
    }

    #[test]
    fn test_match_rejects_different() {
        let tmp = TempDir::new().unwrap();
        let storage = SnapshotStorage::new(tmp.path());
        storage.save("req-1", r#"{"id":1}"#).unwrap();
        assert!(!storage.match_or_update("req-1", r#"{"id":2}"#).unwrap());
    }

    #[test]
    fn test_match_no_snapshot_errors() {
        let tmp = TempDir::new().unwrap();
        let storage = SnapshotStorage::new(tmp.path());
        let result = storage.match_or_update("unknown", r#"{}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_mode_creates_snapshot() {
        let tmp = TempDir::new().unwrap();
        let mut storage = SnapshotStorage::new(tmp.path());
        storage.set_update_mode(true);
        assert!(storage.match_or_update("req-1", r#"{"new":true}"#).unwrap());
        // Now a snapshot exists
        storage.set_update_mode(false);
        assert!(storage.match_or_update("req-1", r#"{"new":true}"#).unwrap());
    }

    #[test]
    fn test_list_snapshots() {
        let tmp = TempDir::new().unwrap();
        let storage = SnapshotStorage::new(tmp.path());
        storage.save("req-a", "a").unwrap();
        storage.save("req-b", "b").unwrap();
        let ids = storage.list().unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"req-a".to_string()));
        assert!(ids.contains(&"req-b".to_string()));
    }

    #[test]
    fn test_delete() {
        let tmp = TempDir::new().unwrap();
        let storage = SnapshotStorage::new(tmp.path());
        storage.save("req-1", "data").unwrap();
        storage.delete("req-1").unwrap();
        assert!(storage.load("req-1").unwrap().is_none());
    }
}
