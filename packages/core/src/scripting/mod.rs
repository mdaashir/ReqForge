//! Script engine for pre-request and post-response JavaScript.
//!
//! Feature-gated behind `script-engine`, this module provides a JS
//! evaluation context using Boa. Each script runs in its own `Context`
//! with a minimal host API injected into the global scope.
//!
//! ## Host API
//!
//! ```js
//! // In pre-request scripts:
//! req.method    // "GET"
//! req.url       // "https://api.example.com/users"
//! req.headers   // { "Authorization": "Bearer ..." }
//! req.body      // "{\"name\":\"x\"}"
//! req.setMethod("POST")
//! req.setUrl("https://other.com")
//! req.setHeader("X-Custom", "value")
//! req.setBody('{"hello":"world"}')
//! req.removeHeader("Authorization")
//!
//! // In post-response scripts:
//! resp.status    // 200
//! resp.statusText // "OK"
//! resp.headers   // { "content-type": "application/json" }
//! resp.body      // "..."
//!
//! // Environment access:
//! env.get("API_KEY")     // "sk-..."
//! env.set("temp", "val") // set a transient variable
//!
//! // Utility:
//! log("message")     // logs to host console
//! crypto.randomUUID() // "550e8400-..."
//! JSON.stringify()   // standard JS
//! ```

use crate::request::Request;
use std::collections::HashMap;

pub mod post_response;

mod engine;

pub use engine::ScriptRuntime;

use crate::error::Result;

/// Result of running a pre-request script. The script can mutate the
/// request or cancel it entirely.
#[derive(Debug, Clone)]
pub struct PreRequestResult {
    pub request: Request,
    pub cancelled: bool,
    pub cancel_reason: Option<String>,
    pub logs: Vec<String>,
}

/// Result of running a post-response script. The script can mutate the
/// response body and headers for preview.
#[derive(Debug, Clone)]
pub struct PostResponseResult {
    pub body: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub logs: Vec<String>,
}

/// Run a pre-request script. Returns the (possibly mutated) request.
pub fn run_pre_request(
    script: &str,
    request: &Request,
    env_vars: &HashMap<String, String>,
) -> Result<PreRequestResult> {
    let rt = ScriptRuntime::new();
    rt.run_pre_request(script, request, env_vars)
}

/// Run a post-response script. Returns test results and possible mutations.
pub fn run_post_response(
    script: &str,
    request: &Request,
    response: &crate::request::Response,
    env_vars: &HashMap<String, String>,
) -> Result<PostResponseResult> {
    let rt = ScriptRuntime::new();
    rt.run_post_response(script, request, response, env_vars)
}
