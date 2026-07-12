use crate::auth::{ApiKeyAuth, ApiKeyLocation, AuthProvider, BasicAuth, BearerAuth, JwtAuth, OAuth2Auth};
use crate::environment::VariableResolver;
use crate::error::{Error, Result};
use crate::request::{AuthType, HttpMethod, Request, Response, ResponseBody, ResponseSize, ResponseTiming};
use crate::scripting;
use reqwest::Client;
use std::collections::HashMap;
use std::time::{Instant, SystemTime};

/// Request executor that handles sending HTTP requests.
///
/// Performs variable resolution and auth application before sending.
pub struct RequestExecutor {
    client: Client,
}

impl RequestExecutor {
    /// Create a new request executor with default settings
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .cookie_store(true)
            .user_agent("ReqForge/0.1.0")
            .build()
            .map_err(|e| Error::connection(e.to_string()))?;

        Ok(Self { client })
    }

    /// Execute a request with the given variable resolver
    pub async fn execute_with_resolver(
        &self,
        mut request: Request,
        resolver: &VariableResolver,
    ) -> Result<Response> {
        // 1. Resolve variables in URL, headers, params, body
        let resolved = self.resolve_variables(request.clone(), resolver)?;

        // 2. Run pre-request script (can mutate the request)
        let env_vars = extract_env_vars(resolver);
        let req_for_script = resolved;

        let script_result = if let Some(script) = &request.pre_request_script {
            if !script.trim().is_empty() {
                scripting::run_pre_request(script, &req_for_script, &env_vars)
            } else {
                Ok(scripting::PreRequestResult {
                    request: req_for_script,
                    cancelled: false,
                    cancel_reason: None,
                    logs: Vec::new(),
                })
            }
        } else {
            Ok(scripting::PreRequestResult {
                request: req_for_script,
                cancelled: false,
                cancel_reason: None,
                logs: Vec::new(),
            })
        }?;

        if script_result.cancelled {
            return Err(Error::script(
                script_result
                    .cancel_reason
                    .unwrap_or_else(|| "cancelled by script".to_string()),
            ));
        }

        // 3. Apply auth
        let authed = self.apply_auth(script_result.request).await?;

        // 4. Send the request
        let mut response = self.send(authed).await?;

        // 5. Run post-response script
        let post_script = request.post_response_script.clone();
        if let Some(script) = post_script {
            if !script.trim().is_empty() {
                if let Ok(result) =
                    scripting::run_post_response(&script, &request, &response, &env_vars)
                {
                    if let Some(body) = result.body {
                        response.body = crate::request::ResponseBody { content: body.into_bytes(), content_type: None, is_text: true };
                    }
                }
            }
        }

        Ok(response)
    }

    /// Execute a request without variable resolution (backward compatible)
    pub async fn execute(&self, request: Request) -> Result<Response> {
        self.execute_with_resolver(request, &VariableResolver::new()).await
    }

    /// Resolve `{{var}}` placeholders in URL, headers, params, and body
    fn resolve_variables(
        &self,
        mut request: Request,
        resolver: &VariableResolver,
    ) -> Result<Request> {
        // URL
        request.url = resolver
            .resolve(&request.url)
            .unwrap_or_else(|_| resolver.resolve_lenient(&request.url));

        // Headers
        request.headers = request
            .headers
            .into_iter()
            .map(|mut h| {
                h.key = resolver.resolve_lenient(&h.key);
                h.value = resolver.resolve_lenient(&h.value);
                h
            })
            .collect();

        // Query params
        request.params = request
            .params
            .into_iter()
            .map(|mut p| {
                p.key = resolver.resolve_lenient(&p.key);
                p.value = resolver.resolve_lenient(&p.value);
                p
            })
            .collect();

        // Body
        request.body.content = resolver.resolve_lenient(&request.body.content);

        Ok(request)
    }

    /// Apply auth provider based on request.auth configuration
    async fn apply_auth(&self, request: Request) -> Result<Request> {
        let Some(auth) = request.auth.clone() else {
            return Ok(request);
        };
        let auth_type = auth.auth_type;
        let config = auth.config;

        let provider: Box<dyn AuthProvider> = match auth_type {
            AuthType::None => return Ok(request),
            AuthType::ApiKey => {
                let key = config.get("key").cloned().unwrap_or_default();
                let value = config.get("value").cloned().unwrap_or_default();
                let location = match config.get("location").map(|s| s.as_str()) {
                    Some("query") => ApiKeyLocation::Query,
                    Some("cookie") => ApiKeyLocation::Cookie,
                    _ => ApiKeyLocation::Header,
                };
                Box::new(ApiKeyAuth::new(key, value, location))
            }
            AuthType::Bearer => {
                let token = config.get("token").cloned().unwrap_or_default();
                Box::new(BearerAuth::new(token))
            }
            AuthType::Basic => {
                let username = config.get("username").cloned().unwrap_or_default();
                let password = config.get("password").cloned().unwrap_or_default();
                Box::new(BasicAuth::new(username, password))
            }
            AuthType::OAuth2 => {
                let access_token = config.get("access_token").cloned().unwrap_or_default();
                let mut provider = OAuth2Auth::new(access_token);
                if let Some(refresh) = config.get("refresh_token") {
                    provider = OAuth2Auth::with_refresh(
                        provider.access_token.clone(),
                        refresh.clone(),
                    );
                }
                Box::new(provider)
            }
            AuthType::Jwt => {
                let token = config.get("token").cloned().unwrap_or_default();
                Box::new(JwtAuth::new(token))
            }
            AuthType::AwsSigV4 => {
                let access_key = config.get("access_key").cloned().unwrap_or_default();
                let secret_key = config.get("secret_key").cloned().unwrap_or_default();
                let region = config.get("region").cloned().unwrap_or_else(|| "us-east-1".to_string());
                let service = config.get("service").cloned().unwrap_or_else(|| "execute-api".to_string());
                let mut signer = crate::auth::aws_sig_v4::AwsSigV4Auth::new(access_key, secret_key)
                    .with_region(region)
                    .with_service(service);
                if let Some(token) = config.get("session_token") {
                    signer = signer.with_session_token(token.clone());
                }
                // SigV4 applies synchronously (not via the AuthProvider trait)
                return signer.apply(request);
            }
            AuthType::Custom(_) => {
                // Unknown auth type: leave request unchanged
                return Ok(request);
            }
        };

        provider.apply(request).await
    }

    /// Send the actual HTTP request
    async fn send(&self, request: Request) -> Result<Response> {
        let start = Instant::now();

        // Build the request
        let mut req_builder = self.client.request(
            Self::to_reqwest_method(request.method),
            &request.url,
        );

        // Add headers
        for header in &request.headers {
            if header.enabled && !header.key.is_empty() {
                req_builder = req_builder.header(&header.key, &header.value);
            }
        }

        // Add query parameters
        let mut query_params: Vec<(String, String)> = Vec::new();
        for param in &request.params {
            if param.enabled && !param.key.is_empty() {
                query_params.push((param.key.clone(), param.value.clone()));
            }
        }
        if !query_params.is_empty() {
            req_builder = req_builder.query(&query_params);
        }

        // Add body
        if let Some(content_type) = &request.body.content_type {
            req_builder = req_builder.header("Content-Type", content_type);
        }
        let body_content = request.body.content.clone();
        if !body_content.is_empty() {
            req_builder = req_builder.body(body_content);
        }

        // Apply timeout
        if let Some(timeout) = request.settings.timeout() {
            req_builder = req_builder.timeout(timeout);
        }

        // Send the request
        let response = req_builder
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    Error::Timeout
                } else if e.is_connect() {
                    Error::connection(e.to_string())
                } else {
                    Error::Http(e)
                }
            })?;

        let total_ms = start.elapsed().as_millis() as u64;

        // Read response
        let status = response.status();
        let status_code = status.as_u16();
        let status_text = status.canonical_reason().unwrap_or("").to_string();

        let mut headers: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(key.to_string(), v.to_string());
            }
        }

        // Capture negotiated HTTP version before consuming response body.
        let http_version = format!("{:?}", response.version());

        let body_bytes = response.bytes().await?;

        let content_type = headers.get("content-type").cloned();
        let is_text = Self::is_text_content(content_type.as_deref());

        let body = ResponseBody {
            content: body_bytes.to_vec(),
            content_type: content_type.clone(),
            is_text,
        };

        let body_size = body_bytes.len() as u64;
        let headers_size = Self::estimate_headers_size(&headers);

        let timing = ResponseTiming {
            dns_ms: 0,
            connect_ms: 0,
            tls_ms: 0,
            send_ms: 0,
            wait_ms: 0,
            receive_ms: 0,
            total_ms,
        };

        let size = ResponseSize {
            headers: headers_size,
            body: body_size,
            total: headers_size + body_size,
        };

        let _ = SystemTime::now(); // suppress unused warning

        Ok(Response {
            status: status_code,
            status_text,
            headers,
            body,
            cookies: Vec::new(),
            timing,
            size,
            url: request.url,
            protocol: http_version,
        })
    }

    fn to_reqwest_method(method: HttpMethod) -> reqwest::Method {
        match method {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Patch => reqwest::Method::PATCH,
            HttpMethod::Delete => reqwest::Method::DELETE,
            HttpMethod::Head => reqwest::Method::HEAD,
            HttpMethod::Options => reqwest::Method::OPTIONS,
            HttpMethod::Trace => reqwest::Method::TRACE,
            HttpMethod::Connect => reqwest::Method::CONNECT,
            HttpMethod::Custom(s) => reqwest::Method::from_bytes(s.as_bytes())
                .unwrap_or(reqwest::Method::GET),
        }
    }

    fn is_text_content(content_type: Option<&str>) -> bool {
        match content_type {
            Some(ct) => {
                ct.starts_with("text/")
                    || ct.contains("json")
                    || ct.contains("xml")
                    || ct.contains("javascript")
                    || ct.contains("form-urlencoded")
                    || ct.contains("graphql")
            }
            None => false,
        }
    }

    fn estimate_headers_size(headers: &std::collections::HashMap<String, String>) -> u64 {
        headers
            .iter()
            .map(|(k, v)| (k.len() + v.len() + 4) as u64)
            .sum()
    }
}

/// Extract a HashMap of environment variables from the resolver for script access.
fn extract_env_vars(resolver: &VariableResolver) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    // Try common variable patterns
    for key in ["base_url", "host", "port", "scheme", "token", "api_key"] {
        if let Some(v) = resolver.get(key) {
            vars.insert(key.to_string(), v);
        }
    }
    vars
}

impl Default for RequestExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create default RequestExecutor")
    }
}
