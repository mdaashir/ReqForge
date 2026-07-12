//! AWS Signature V4 (SigV4) authentication provider.
//!
//! Signs HTTP requests using the [AWS Signature V4](https://docs.aws.amazon.com/IAM/latest/UserGuide/create-signed-request.html)
//! algorithm. Used when connecting to AWS APIs (API Gateway, Lambda, S3, etc.)
//! that require request signing.
//!
//! ## Usage
//!
//! ```ignore
//! let sigv4 = AwsSigV4Auth::new("AKIDEXAMPLE", "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY")
//!     .with_region("us-east-1")
//!     .with_service("execute-api");
//! let signed = sigv4.apply(request).await?;
//! ```

use crate::error::{Error, Result};
use crate::request::{BodyMode, HttpMethod, KeyValue, Request};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// AWS Signature V4 auth provider.
#[derive(Debug, Clone)]
pub struct AwsSigV4Auth {
    access_key: String,
    secret_key: String,
    region: String,
    service: String,
    session_token: Option<String>,
}

impl AwsSigV4Auth {
    /// Create a new SigV4 signer with the given AWS credentials.
    pub fn new(access_key: impl Into<String>, secret_key: impl Into<String>) -> Self {
        Self {
            access_key: access_key.into(),
            secret_key: secret_key.into(),
            region: "us-east-1".to_string(),
            service: "execute-api".to_string(),
            session_token: None,
        }
    }

    /// Set the AWS region (default: `us-east-1`).
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = region.into();
        self
    }

    /// Set the AWS service name (default: `execute-api`).
    pub fn with_service(mut self, service: impl Into<String>) -> Self {
        self.service = service.into();
        self
    }

    /// Set an optional session token (e.g., from STS temporary credentials).
    pub fn with_session_token(mut self, token: impl Into<String>) -> Self {
        self.session_token = Some(token.into());
        self
    }

    /// Apply the SigV4 signature to the request by adding the `Authorization` header.
    pub fn apply(&self, mut request: Request) -> Result<Request> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::auth(format!("clock error: {e}")))?;

        let amz_date = format_amz_date(now);
        let date_stamp = &amz_date[..8]; // YYYYMMDD

        // Prepare canonical request components
        let method = request.method.to_string();
        let canonical_uri = canonical_path(&request.url);
        let canonical_querystring = canonical_query_string(&request.params, &request.url);
        let (signed_headers, canonical_headers) = canonical_headers_from_request(&request, &amz_date);

        let payload_hash = hex::encode(payload_hash_for_method(&request.body.content, &request.body.mode));

        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method, canonical_uri, canonical_querystring, canonical_headers, signed_headers, payload_hash
        );

        let canonical_request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));

        // String to sign
        let algorithm = "AWS4-HMAC-SHA256";
        let credential_scope = format!("{}/{}/{}/aws4_request", date_stamp, self.region, self.service);
        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            algorithm,
            amz_date,
            credential_scope,
            canonical_request_hash
        );

        // Signing key
        let signing_key = derive_signing_key(&self.secret_key, date_stamp, &self.region, &self.service);
        let signature = hex::encode(sign(&signing_key, string_to_sign.as_bytes()));

        let signed_headers_str = signed_headers.split_whitespace().collect::<Vec<_>>().join(";");

        let authorization = format!(
            "{} Credential={}/{}, SignedHeaders={}, Signature={}",
            algorithm, self.access_key, credential_scope, signed_headers_str, signature
        );

        // Add headers
        request.headers.push(KeyValue {
            key: "Authorization".to_string(),
            value: authorization,
            enabled: true,
            description: None,
        });
        request.headers.push(KeyValue {
            key: "X-Amz-Date".to_string(),
            value: amz_date.clone(),
            enabled: true,
            description: None,
        });
        if let Some(token) = &self.session_token {
            request.headers.push(KeyValue {
                key: "X-Amz-Security-Token".to_string(),
                value: token.clone(),
                enabled: true,
                description: None,
            });
        }

        Ok(request)
    }

    /// Sign a bare request (without needing a full Request struct).
    /// Used for testing or when signing raw HTTP params.
    pub fn sign_request(
        &self,
        _method: &str,
        _url: &str,
        _body: &str,
        _headers: &[(String, String)],
    ) -> Result<(String, String, String)> {
        // Simple wrapper for external use
        let mut req = Request::new(
            HttpMethod::Get,
            _url,
        );
        if let Some(pos) = _url.find('?') {
            let qs = _url[pos + 1..].to_string();
            for pair in qs.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    req.params.push(KeyValue {
                        key: k.to_string(),
                        value: v.to_string(),
                        enabled: true,
                        description: None,
                    });
                }
            }
            req.url = _url[..pos].to_string();
        }
        if !_body.is_empty() {
            req.body.content = _body.to_string();
            req.body.mode = BodyMode::Text;
        }
        for (k, v) in _headers {
            req.headers.push(KeyValue {
                key: k.clone(),
                value: v.clone(),
                enabled: true,
                description: None,
            });
        }

        let signed = self.apply(req)?;
        let auth = signed.headers.iter().find(|h| h.key == "Authorization").map(|h| h.value.clone()).unwrap_or_default();
        let date = signed.headers.iter().find(|h| h.key == "X-Amz-Date").map(|h| h.value.clone()).unwrap_or_default();
        Ok((auth, date, "AWS4-HMAC-SHA256".to_string()))
    }
}

// ── helpers ──────────────────────────────────────────────

fn format_amz_date(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    // Convert to civil time (simplified — fine for signing)
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Days since epoch to date (Gaussian algorithm sufficient for signing needs)
    let (y, m, d) = days_to_date(days);
    format!("{:04}{:02}{:02}T{:02}{:02}{:02}Z", y, m, d, hours, minutes, seconds)
}

fn days_to_date(days: u64) -> (u64, u64, u64) {
    // Algorithm from the FAQ of how many days in each month
    let y = 1970 + (days * 4 + 2) / 1461;
    let prior_days = (y - 1970) * 365 + (y - 1969) / 4;
    let day_of_year = days - prior_days;
    let month = (day_of_year * 12 + 6) / 367;
    let day = day_of_year - (month * 367 + 5) / 12 + 1;
    (y, month + 1, day)
}

fn canonical_path(url: &str) -> String {
    // Extract path portion from URL
    if let Ok(parsed) = url::Url::parse(url) {
        let path = parsed.path();
        if path.is_empty() { "/".to_string() } else { path.to_string() }
    } else {
        // Bare path
        "/".to_string()
    }
}

fn canonical_query_string(params: &[KeyValue], url: &str) -> String {
    let mut pairs: Vec<(String, String)> = params
        .iter()
        .filter(|p| p.enabled)
        .map(|p| (urlencoding_encode(&p.key), urlencoding_encode(&p.value)))
        .collect();

    // Also extract from URL
    if let Ok(parsed) = url::Url::parse(url) {
        for (k, v) in parsed.query_pairs() {
            if !pairs.iter().any(|(pk, _)| pk == &k.to_string()) {
                pairs.push((urlencoding_encode(&k), urlencoding_encode(&v)));
            }
        }
    }

    pairs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&")
}

fn canonical_headers_from_request(request: &Request, amz_date: &str) -> (String, String) {
    let mut header_map: HashMap<String, String> = HashMap::new();

    // Add X-Amz-Date if not present
    let has_date = request.headers.iter().any(|h| h.key.eq_ignore_ascii_case("x-amz-date"));
    if !has_date {
        header_map.insert("x-amz-date".to_string(), amz_date.to_string());
    }

    // Collect and lowercase all headers
    for h in &request.headers {
        if h.enabled {
            let key = h.key.to_lowercase();
            if key == "authorization" || key == "user-agent" {
                continue;
            }
            header_map.insert(key, h.value.trim().to_string());
        }
    }

    // Host header (derive from URL)
    if !header_map.contains_key("host") {
        if let Ok(parsed) = url::Url::parse(&request.url) {
            if let Some(host) = parsed.host_str() {
                let host_val = if let Some(port) = parsed.port() {
                    format!("{}:{}", host, port)
                } else {
                    host.to_string()
                };
                header_map.insert("host".to_string(), host_val);
            }
        }
    }

    // Sort by header name
    let mut sorted: Vec<(String, String)> = header_map.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    let signed_names: Vec<String> = sorted.iter().map(|(k, _)| k.clone()).collect();
    let headers_block: String = sorted
        .iter()
        .map(|(k, v)| format!("{}:{}\n", k, v))
        .collect();

    (signed_names.join(";"), headers_block)
}

fn payload_hash_for_method(body: &str, mode: &BodyMode) -> Vec<u8> {
    match mode {
        BodyMode::None | BodyMode::Graphql => {
            // For GET/HEAD/DELETE and certain other cases, use empty hash
            Sha256::digest(b"").to_vec()
        }
        _ => Sha256::digest(body.as_bytes()).to_vec(),
    }
}

fn derive_signing_key(secret_key: &str, date_stamp: &str, region: &str, service: &str) -> Vec<u8> {
    let k_secret = format!("AWS4{}", secret_key);
    let k_date = sign_str(&k_secret, date_stamp);
    let k_region = sign(&k_date, region.as_bytes());
    let k_service = sign(&k_region, service.as_bytes());
    
    sign(&k_service, b"aws4_request")
}

fn sign(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn sign_str(key: &str, data: &str) -> Vec<u8> {
    sign(key.as_bytes(), data.as_bytes())
}

/// Minimal URL percent-encoding (only for canonical query strings).
fn urlencoding_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_get_request() {
        let auth = AwsSigV4Auth::new("AKIDEXAMPLE", "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY")
            .with_region("us-east-1")
            .with_service("iam");
        let req = Request::new(HttpMethod::Get, "https://iam.amazonaws.com/?Action=ListUsers&Version=2010-05-08");
        let result = auth.apply(req).unwrap();

        let auth_header = result.headers.iter().find(|h| h.key == "Authorization").unwrap();
        assert!(auth_header.value.starts_with("AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE"));
        assert!(auth_header.value.contains("SignedHeaders=host;x-amz-date"));
    }

    #[test]
    fn test_sign_post_request() {
        let auth = AwsSigV4Auth::new("AKIDEXAMPLE", "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY")
            .with_region("us-east-1")
            .with_service("execute-api");
        let req = Request::new(HttpMethod::Post, "https://api.example.com/users");
        let result = auth.apply(req).unwrap();

        let auth_header = result.headers.iter().find(|h| h.key == "Authorization").unwrap();
        assert!(auth_header.value.starts_with("AWS4-HMAC-SHA256"));
        assert!(result.headers.iter().any(|h| h.key == "X-Amz-Date"));
    }
}
