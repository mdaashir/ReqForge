//! Request types and execution
//!
//! This module provides the core types for building, executing, and processing
//! HTTP requests. It includes:
//! - Request and Response types
//! - HTTP method enum
//! - Request builder for fluent construction
//! - Executor for sending requests

pub mod builder;

mod executor;
pub mod interceptor;
mod types;

pub use executor::RequestExecutor;
pub use types::{
    Auth, AuthType, Body, BodyMode, Cookie, HttpMethod, KeyValue, Request, RequestSettings,
    Response, ResponseBody, ResponseSize, ResponseTiming, ResponseTimingDetail,
};
