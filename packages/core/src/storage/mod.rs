//! Storage adapters for persistence.
//!
//! This module aggregates all storage implementations used across the crate:
//!
//! - **SQLite** — `crate::history::SqliteHistoryStorage` for request history
//! - **File system** — `crate::collection::CollectionStorage` for YAML collections
//! - **File system** — `crate::environment::EnvironmentStorage` for YAML environments
//! - **Keychain** — `apps/desktop/src-tauri/src/keychain.rs` for OS-level secrets
//! - **Snapshots** — `crate::testing::snapshot::SnapshotStorage` for golden files
//!
//! Each storage type is re-exported at its functional module path.
//! This module exists to match the blueprint's `storage/` directory structure.

pub use crate::collection::CollectionStorage;
pub use crate::environment::EnvironmentStorage;
pub use crate::history::SqliteHistoryStorage;
pub use crate::testing::snapshot::SnapshotStorage;
