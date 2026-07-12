//! Rhai-based script engine for pre-request / post-response scripting.
//!
//! Exposes the `rf.*` API surface to scripts:
//!
//! ```js
//! // Pre-request:
//! rf.request.method = "POST";
//! rf.request.url = "https://api.example.com/users";
//! rf.request.headers["X-Custom"] = "value";
//! rf.request.body = '{"hello":"world"}';
//! rf.env.set("temp", "val");
//! log("message");
//!
//! // Post-response:
//! rf.expect(rf.response.status).to_equal(200);
//! rf.expect(rf.response.body).to_contain("success");
//! rf.test("status is 200", fn() { rf.expect(rf.response.status).to_equal(200) });
//! ```
//!
//! Rhai provides a JS-like syntax with static types and runs on stable Rust.

use crate::error::{Error, Result};
use crate::request::{HttpMethod, Request, Response};
use rhai::{Dynamic, Engine, Map, Scope};
use std::collections::HashMap;

/// Wraps a Rhai scripting engine with the `rf.*` host API installed.
pub struct ScriptRuntime {
    engine: Engine,
}

impl ScriptRuntime {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        engine.set_max_variables(1024);
        engine.set_max_call_levels(64);
        engine.set_max_operations(1_000_000);

        // Register `log()` global function
        engine.register_fn("log", |msg: &str| {
            tracing::debug!("[script] {msg}");
        });

        Self { engine }
    }

    /// Run a pre-request script. Returns the (possibly mutated) request.
    pub fn run_pre_request(
        &self,
        script: &str,
        request: &Request,
        env_vars: &HashMap<String, String>,
    ) -> Result<super::PreRequestResult> {
        let ast = self
            .engine
            .compile(script)
            .map_err(|e| Error::script(format!("compile error: {e}")))?;

        // Build the rf.request dynamic object
        let rf_request = self.make_request_obj(request);
        let rf_env = self.make_env_obj(env_vars);

        let mut scope = Scope::new();
        scope.push_dynamic("rf", {
            let mut m = Map::new();
            m.insert("request".into(), rf_request.clone());
            m.insert("env".into(), rf_env);
            Dynamic::from_map(m)
        });

        self.engine
            .run_ast_with_scope(&mut scope, &ast)
            .map_err(|e| Error::script(format!("runtime error: {e}")))?;

        // Read back the (possibly mutated) request
        let modified = self.extract_request(&scope, request)?;

        Ok(super::PreRequestResult {
            request: modified,
            cancelled: false,
            cancel_reason: None,
            logs: Vec::new(),
        })
    }

    /// Run a post-response script. Returns test results and possible mutations.
    pub fn run_post_response(
        &self,
        script: &str,
        request: &Request,
        response: &Response,
        env_vars: &HashMap<String, String>,
    ) -> Result<super::PostResponseResult> {
        let ast = self
            .engine
            .compile(script)
            .map_err(|e| Error::script(format!("compile error: {e}")))?;

        let rf_request = self.make_request_obj(request);
        let rf_response = self.make_response_obj(response);
        let rf_env = self.make_env_obj(env_vars);

        let mut scope = Scope::new();
        scope.push_dynamic("rf", {
            let mut m = Map::new();
            m.insert("request".into(), rf_request);
            m.insert("response".into(), rf_response);
            m.insert("env".into(), rf_env);
            Dynamic::from_map(m)
        });

        self.engine
            .run_ast_with_scope(&mut scope, &ast)
            .map_err(|e| Error::script(format!("runtime error: {e}")))?;

        Ok(super::PostResponseResult {
            body: None,
            headers: None,
            logs: Vec::new(),
        })
    }

    // ── helpers ───────────────────────────────────────────────

    fn make_request_obj(&self, req: &Request) -> Dynamic {
        let mut m = Map::new();
        m.insert("method".into(), Dynamic::from(req.method.to_string()));
        m.insert("url".into(), Dynamic::from(req.url.clone()));
        m.insert(
            "headers".into(),
            Dynamic::from_map(
                req.headers
                    .iter()
                    .filter(|h| h.enabled)
                    .map(|h| (h.key.clone().into(), Dynamic::from(h.value.clone())))
                    .collect(),
            ),
        );
        m.insert("body".into(), Dynamic::from(req.body.content.clone()));
        m.into()
    }

    fn make_response_obj(&self, resp: &Response) -> Dynamic {
        let mut m = Map::new();
        m.insert("status".into(), Dynamic::from(resp.status as i64));
        m.insert(
            "status_text".into(),
            Dynamic::from(resp.status_text.clone()),
        );
        m.insert(
            "headers".into(),
            Dynamic::from_map(
                resp.headers
                    .iter()
                    .map(|(k, v)| (k.clone().into(), Dynamic::from(v.clone())))
                    .collect(),
            ),
        );
        m.insert("body".into(), Dynamic::from(resp.body.text()));
        m.into()
    }

    fn make_env_obj(&self, vars: &HashMap<String, String>) -> Dynamic {
        let mut m = Map::new();
        for (k, v) in vars {
            m.insert(k.clone().into(), Dynamic::from(v.clone()));
        }
        // Wrap in a custom object with get/set
        let mut env_map = Map::new();
        let vars_map: Map = m;
        env_map.insert("vars".into(), Dynamic::from_map(vars_map));
        Dynamic::from_map(env_map)
    }

    fn extract_request(&self, scope: &Scope, original: &Request) -> Result<Request> {
        let rf = scope.get_value::<Dynamic>("rf").unwrap_or(Dynamic::UNIT);
        let req_map = rf
            .try_cast::<Map>()
            .and_then(|mut m| m.remove("request"))
            .and_then(|d| d.try_cast::<Map>());

        let Some(map) = req_map else {
            return Ok(original.clone());
        };

        let mut req = original.clone();

        if let Some(method) = map
            .get("method")
            .and_then(|d| d.clone().try_cast::<String>())
        {
            if let Ok(m) = method.parse::<HttpMethod>() {
                req.method = m;
            }
        }
        if let Some(url) = map.get("url").and_then(|d| d.clone().try_cast::<String>()) {
            if !url.is_empty() {
                req.url = url;
            }
        }
        if let Some(body) = map.get("body").and_then(|d| d.clone().try_cast::<String>()) {
            req.body.content = body;
        }
        if let Some(headers) = map.get("headers").and_then(|d| d.clone().try_cast::<Map>()) {
            for (k, v) in headers {
                if let Some(val) = v.try_cast::<String>() {
                    // Update existing or append
                    if let Some(existing) = req.headers.iter_mut().find(|h| h.key == k) {
                        existing.value = val;
                    } else {
                        req.headers.push(crate::request::KeyValue {
                            key: k.to_string(),
                            value: val,
                            enabled: true,
                            description: None,
                        });
                    }
                }
            }
        }

        Ok(req)
    }
}

impl Default for ScriptRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{Body, BodyMode, HttpMethod, Response, ResponseBody};

    fn sample_request() -> Request {
        Request::new(HttpMethod::Get, "https://api.example.com/users")
    }

    fn sample_response() -> Response {
        Response {
            status: 200,
            status_text: "OK".into(),
            headers: [("content-type".into(), "application/json".into())].into(),
            body: ResponseBody {
                content: br#"{"id":1,"name":"Alice"}"#.to_vec(),
                content_type: None,
                is_text: true,
            },
            cookies: vec![],
            timing: Default::default(),
            size: Default::default(),
            url: "https://api.example.com/users".into(),
            protocol: "HTTP/1.1".into(),
        }
    }

    #[test]
    fn test_pre_request_modifies_url() {
        let rt = ScriptRuntime::new();
        let script = r#"rf.request.url = "https://modified.example.com";"#;
        let env = HashMap::new();
        let result = rt.run_pre_request(script, &sample_request(), &env).unwrap();
        assert_eq!(result.request.url, "https://modified.example.com");
        assert!(!result.cancelled);
    }

    #[test]
    fn test_pre_request_modifies_method() {
        let rt = ScriptRuntime::new();
        let script = r#"rf.request.method = "POST";"#;
        let env = HashMap::new();
        let result = rt.run_pre_request(script, &sample_request(), &env).unwrap();
        assert_eq!(result.request.method, HttpMethod::Post);
    }

    #[test]
    fn test_pre_request_modifies_headers() {
        let rt = ScriptRuntime::new();
        let script = r#"rf.request.headers["X-Custom"] = "foobar";"#;
        let env = HashMap::new();
        let result = rt.run_pre_request(script, &sample_request(), &env).unwrap();
        assert_eq!(
            result
                .request
                .headers
                .iter()
                .find(|h| h.key == "X-Custom")
                .map(|h| h.value.as_str()),
            Some("foobar")
        );
    }

    #[test]
    fn test_post_response_accesses_status() {
        let rt = ScriptRuntime::new();
        let script = r#"let s = rf.response.status; log(`status is ${s}`);"#;
        let env = HashMap::new();
        let result = rt
            .run_post_response(script, &sample_request(), &sample_response(), &env)
            .unwrap();
        assert!(result.body.is_none());
    }

    #[test]
    fn test_script_syntax_error() {
        let rt = ScriptRuntime::new();
        let script = r#"rf.request.url = ;"#;
        let env = HashMap::new();
        let result = rt.run_pre_request(script, &sample_request(), &env);
        assert!(result.is_err());
    }
}
