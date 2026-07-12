//! Git integration.
//!
//! Re-exports from the `watcher::git` module where the `GitRepo`
//! implementation lives alongside the file-watcher feature.
//!
//! This alias exists because the blueprint places git sync under
//! `sync::git` and some callers may prefer that path.

#[cfg(feature = "watcher")]
pub use crate::watcher::git::GitRepo;
