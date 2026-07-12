//! Collection types and storage
//!
//! Collections group related requests into a folder tree. They are persisted
//! to disk as YAML files for Git-friendly versioning.

mod model;
mod runner;
mod storage;

pub use model::{Collection, CollectionItem, CollectionMap};
pub use runner::{CollectionRunResult, CollectionRunSummary, CollectionRunner, RunMode};
pub use storage::CollectionStorage;
