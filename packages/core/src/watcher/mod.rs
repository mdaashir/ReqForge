//! File system watcher and git auto-committer.
//!
//! Watches `collections/` and `environments/` directories for file changes,
//! fires reload events, and auto-commits to git when configured.

pub mod git;

use crate::error::{Error, Result};
use notify::Watcher as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Types of file change events the watcher emits.
#[derive(Debug, Clone)]
pub enum FileEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}

/// A file system watcher for ReqForge workspace directories.
///
/// Watches `collections/` and `environments/` under the workspace root.
/// Emits events over a channel so the caller can react.
pub struct FileWatcher {
    _watcher: notify::RecommendedWatcher,
    rx: mpsc::Receiver<FileEvent>,
}

/// Shared state accessible by the reload handler.
struct WatcherInner {
    workspace: PathBuf,
    tx: mpsc::Sender<FileEvent>,
    /// Optional git repo for auto-commit.
    git: std::sync::Mutex<Option<git::GitRepo>>,
}

impl FileWatcher {
    /// Start watching the workspace for file changes.
    ///
    /// `auto_commit` — if true, auto-commit changes to git after a brief
    /// debounce (500ms). Requires the workspace to be inside a git repo.
    pub fn start(
        workspace: &Path,
        auto_commit: bool,
    ) -> Result<Self> {
        let (tx, rx) = mpsc::channel(256);

        let workspace = workspace.to_path_buf();
        let collections_dir = workspace.join("collections");
        let environments_dir = workspace.join("environments");

        // Create directories if they don't exist
        std::fs::create_dir_all(&collections_dir).ok();
        std::fs::create_dir_all(&environments_dir).ok();

        // Set up git repo
        let git_repo = if auto_commit {
            git::GitRepo::open(&workspace)
        } else {
            None
        };

        let inner = Arc::new(WatcherInner {
            workspace,
            tx,
            git: std::sync::Mutex::new(git_repo),
        });

        // Last commit time for debouncing
        let last_commit: Arc<std::sync::Mutex<std::time::Instant>> =
            Arc::new(std::sync::Mutex::new(std::time::Instant::now()));

        let inner_clone = inner.clone();
        let last_commit_clone = last_commit.clone();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let event = match res {
                Ok(e) => e,
                Err(_) => return,
            };

            let path = match event.paths.first() {
                Some(p) => p.clone(),
                None => return,
            };

            // Only watch .yaml and .json files
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "yaml" | "yml" | "json") {
                return;
            }

            // Debounce: don't process more than once per 200ms
            let now = std::time::Instant::now();
            {
                let mut last = last_commit_clone.lock().unwrap();
                if now.duration_since(*last).as_millis() < 200 {
                    return;
                }
                *last = now;
            }

            let fe = match event.kind {
                notify::EventKind::Create(_) => FileEvent::Created(path.clone()),
                notify::EventKind::Modify(_) => FileEvent::Modified(path.clone()),
                notify::EventKind::Remove(_) => FileEvent::Deleted(path.clone()),
                _ => return,
            };

            // Send event to channel (best-effort)
            let inner = inner_clone.clone();
            if let Ok(git_lock) = inner.git.lock() {
                if let Some(ref repo) = *git_lock {
                    if repo.has_changes().unwrap_or(false) {
                        let summary = repo.status_summary().unwrap_or_else(|_| "changes".to_string());
                        let msg = format!("[auto] {}", summary);
                        let _ = repo.commit(&msg);
                    }
                }
            }

            let _ = inner_clone.tx.try_send(fe);
        })
        .map_err(|e| Error::other(format!("failed to start file watcher: {e}")))?;

        // Start watching directories
        watcher
            .watch(&collections_dir, notify::RecursiveMode::Recursive)
            .map_err(|e| Error::other(format!("failed to watch collections: {e}")))?;
        watcher
            .watch(&environments_dir, notify::RecursiveMode::Recursive)
            .map_err(|e| Error::other(format!("failed to watch environments: {e}")))?;

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    /// Receive the next file change event.
    pub async fn recv(&mut self) -> Option<FileEvent> {
        self.rx.recv().await
    }

    /// Stop the watcher. Drops the internal watcher which unregisters watches.
    pub fn stop(self) {
        drop(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_git_repo_open_not_found() {
        let tmp = TempDir::new().unwrap();
        let repo = git::GitRepo::open(tmp.path());
        assert!(repo.is_none());
    }

    #[test]
    fn test_git_status_clean() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let repo = git::GitRepo::open(tmp.path()).unwrap();
        assert!(!repo.has_changes().unwrap());
        assert_eq!(repo.status_summary().unwrap(), "clean");
    }

    #[test]
    fn test_git_status_dirty() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        std::fs::write(tmp.path().join("test.yaml"), "hello").unwrap();
        let repo = git::GitRepo::open(tmp.path()).unwrap();
        assert!(repo.has_changes().unwrap());
    }

    #[test]
    fn test_git_commit_and_branch() {
        let tmp = TempDir::new().unwrap();
        let repo_obj = git2::Repository::init(tmp.path()).unwrap();

        // Configure user for the test repo
        let mut cfg = repo_obj.config().unwrap();
        cfg.set_str("user.name", "Test").unwrap();
        cfg.set_str("user.email", "test@test.com").unwrap();

        // Initial commit needed before we can commit again
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree = repo_obj.find_tree(repo_obj.index().unwrap().write_tree().unwrap()).unwrap();
        repo_obj.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();

        // Now test our GitRepo
        let repo = git::GitRepo::open(tmp.path()).unwrap();
        assert!(repo.current_branch().is_some());

        std::fs::write(tmp.path().join("collection.yaml"), "name: test").unwrap();
        assert!(repo.has_changes().unwrap());
        repo.commit("[auto] 1 modified").unwrap();
        assert!(!repo.has_changes().unwrap());
    }
}
