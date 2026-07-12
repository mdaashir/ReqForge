//! Request history
//!
//! Lightweight, append-only log of every request/response pair sent.
//! Backed by an append-only JSON-lines file on disk. Easy to migrate to
//! SQLite later by swapping `HistoryStorage` without changing the API.

mod sqlite;
mod storage;

pub use sqlite::SqliteHistoryStorage;
pub use storage::{HistoryEntry, HistoryStorage};

/// Maximum number of history entries to retain
pub const DEFAULT_HISTORY_LIMIT: usize = 1000;
