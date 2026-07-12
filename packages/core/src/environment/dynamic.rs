//! Dynamic variable generators.
//!
//! Provides `{{$uuid}}`, `{{$timestamp}}`, `{{$randomInt}}`, etc.
//! Implemented as part of `VariableResolver` in `resolver.rs`.
//!
//! This module re-exports the `DynamicVariables` type for callers that
//! want to use it independently of the resolver.

pub use crate::environment::resolver::DynamicVariables;
