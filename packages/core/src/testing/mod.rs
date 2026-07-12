//! Testing engine
//!
//! Runs assertions and test scripts against API responses.
//! Supports status, header, body, timing, and schema validation assertions.

mod assertion;
pub mod reporter;
mod runner;
pub mod schema_validator;
pub mod snapshot;

pub use assertion::{Assertion, AssertionResult, AssertionType};
pub use runner::{TestResult, TestRunner, TestStatus};
