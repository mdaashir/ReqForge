//! Git integration — auto-commit, status, and sync operations.
//!
//! Lightweight wrapper around `git2` for the ReqForge workspace.
//! Designed for the "watch + auto-commit" workflow: when files
//! change, we detect the repo root and create an auto-commit.

use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

/// A handle to a git repository rooted at the workspace.
pub struct GitRepo {
    repo: git2::Repository,
    root: PathBuf,
}

impl GitRepo {
    /// Open the git repository that contains `workspace_path`.
    /// Returns `None` if the path is not inside a git repo.
    pub fn open(workspace_path: &Path) -> Option<Self> {
        let repo = git2::Repository::discover(workspace_path).ok()?;
        let root = repo.workdir()?.to_path_buf();
        Some(Self { repo, root })
    }

    /// Returns true if the workspace is inside a git repository.
    pub fn is_available(&self) -> bool {
        true
    }

    /// Check if there are any unstaged or uncommitted changes.
    pub fn has_changes(&self) -> Result<bool> {
        let mut status_opts = git2::StatusOptions::new();
        status_opts.include_untracked(true);
        let statuses = self.repo
            .statuses(Some(&mut status_opts))
            .map_err(|e| Error::other(format!("git status: {e}")))?;
        Ok(!statuses.is_empty())
    }

    /// Stage all changed files and auto-commit with a descriptive message.
    pub fn commit(&self, message: &str) -> Result<()> {
        // Stage everything (collections, environments, etc)
        let mut idx = self.repo
            .index()
            .map_err(|e| Error::other(format!("git index: {e}")))?;
        idx.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(|e| Error::other(format!("git add: {e}")))?;
        idx.write().map_err(|e| Error::other(format!("git write index: {e}")))?;

        let tree_id = idx
            .write_tree()
            .map_err(|e| Error::other(format!("git write tree: {e}")))?;
        let tree = self.repo
            .find_tree(tree_id)
            .map_err(|e| Error::other(format!("git find tree: {e}")))?;

        let signature = git2::Signature::now("ReqForge", "auto@reqforge.io")
            .map_err(|e| Error::other(format!("git signature: {e}")))?;

        // Look for existing HEAD to use as parent
        let parent = self.repo.head().ok().and_then(|head| {
            head.peel_to_commit().ok()
        });

        let _commit = if let Some(p) = &parent {
            self.repo
                .commit(Some("HEAD"), &signature, &signature, message, &tree, &[p])
        } else {
            self.repo
                .commit(Some("HEAD"), &signature, &signature, message, &tree, &[])
        };

        match _commit {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::other(format!("git commit: {e}"))),
        }
    }

    /// Get a short status summary of the repo.
    pub fn status_summary(&self) -> Result<String> {
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true);
        let statuses = self.repo
            .statuses(Some(&mut opts))
            .map_err(|e| Error::other(format!("git status: {e}")))?;

        let mut modified = 0;
        let mut added = 0;
        let mut deleted = 0;
        let mut untracked = 0;

        for entry in statuses.iter() {
            let s = entry.status();
            if s.contains(git2::Status::CURRENT) { continue; }
            if s.contains(git2::Status::INDEX_NEW) || s.contains(git2::Status::WT_NEW) {
                added += 1;
            }
            if s.intersects(git2::Status::INDEX_MODIFIED | git2::Status::WT_MODIFIED) {
                modified += 1;
            }
            if s.intersects(git2::Status::INDEX_DELETED | git2::Status::WT_DELETED) {
                deleted += 1;
            }
            if s.contains(git2::Status::WT_NEW) {
                untracked += 1;
            }
        }

        let parts: Vec<String> = [
            (modified > 0).then(|| format!("{} modified", modified)),
            (added > 0).then(|| format!("{} added", added)),
            (deleted > 0).then(|| format!("{} deleted", deleted)),
            (untracked > 0).then(|| format!("{} untracked", untracked)),
        ]
        .into_iter()
        .flatten()
        .collect();

        if parts.is_empty() {
            Ok("clean".to_string())
        } else {
            Ok(parts.join(", "))
        }
    }

    /// Return the current branch name.
    pub fn current_branch(&self) -> Option<String> {
        let head = self.repo.head().ok()?;
        let name = head.shorthand()?.to_string();
        Some(name)
    }

    /// Root path of the git repository.
    pub fn root(&self) -> &Path {
        &self.root
    }
}
