//! Request builder — fluent API for constructing requests.
//!
//! The `Request::new(method, url)` constructor covers common cases.
//! This module can host a builder pattern if complex request
//! construction is needed (e.g., `RequestBuilder::new().method("POST")
//! .url("...").header("key", "val").body::<Json>(data).build()`).
//!
//! ponytail: add builder when the constructor pattern grows past 5+ optional
//! fields that need validation or defaults beyond what `Request::new` provides.
