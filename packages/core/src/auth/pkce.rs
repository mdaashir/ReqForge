//! OAuth 2.0 Authorization Code flow with PKCE (RFC 7636)
//!
//! Implements the secure variant of the auth-code flow that doesn't require
//! a client secret. The desktop app:
//! 1. Generates a `code_verifier` (43-128 char random string).
//! 2. Computes `code_challenge = base64url(SHA256(verifier))`.
//! 3. Opens the authorization URL in the user's browser.
//! 4. Listens on a local loopback for the redirect carrying `code`.
//! 5. Exchanges the code + verifier at the token endpoint.
//! 6. Optionally refreshes via the `refresh_token`.
//!
//! The PKCE bits are pure functions; the browser open + loopback listener
//! live in the desktop shell.

use crate::error::{Error, Result};
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// PKCE code-verifier (RFC 7636 §4.1): unreserved chars, 43–128 long.
#[derive(Debug, Clone)]
pub struct CodeVerifier(String);

impl CodeVerifier {
    pub fn generate() -> Self {
        // 64 bytes → 86-char base64url. Plenty of entropy, well above the
        // RFC minimum of 256 bits.
        let mut bytes = [0u8; 64];
        rand::thread_rng().fill_bytes(&mut bytes);
        Self(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// PKCE code-challenge derived from the verifier (RFC 7636 §4.2, S256).
#[derive(Debug, Clone)]
pub struct CodeChallenge(String);

impl CodeChallenge {
    pub fn from_verifier(verifier: &CodeVerifier) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_str().as_bytes());
        let digest = hasher.finalize();
        Self(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Configuration for an OAuth 2.0 client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Config {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub client_id: String,
    /// Space-separated scopes, e.g. `"openid profile email"`.
    pub scopes: Vec<String>,
    /// Redirect URI; typically `http://127.0.0.1:<port>/callback`.
    pub redirect_uri: String,
    /// Optional extra params to merge into the auth URL (e.g. `audience`).
    #[serde(default)]
    pub extra_auth_params: Vec<(String, String)>,
    /// Optional extra params to merge into the token request.
    #[serde(default)]
    pub extra_token_params: Vec<(String, String)>,
}

/// Generates the URL the user should be sent to in order to authorise the
/// app. Includes a CSRF `state` nonce that the caller must verify when the
/// redirect comes back.
pub fn build_authorize_url(
    config: &OAuth2Config,
    code_challenge: &CodeChallenge,
    state: &str,
) -> String {
    let mut url = config.authorization_endpoint.clone();
    url.push_str(if config.authorization_endpoint.contains('?') { "&" } else { "?" });
    url.push_str("response_type=code");
    url.push_str("&client_id=");
    url.push_str(&url_encode(&config.client_id));
    url.push_str("&redirect_uri=");
    url.push_str(&url_encode(&config.redirect_uri));
    url.push_str("&scope=");
    url.push_str(&url_encode(&config.scopes.join(" ")));
    url.push_str("&state=");
    url.push_str(&url_encode(state));
    url.push_str("&code_challenge=");
    url.push_str(code_challenge.as_str());
    url.push_str("&code_challenge_method=S256");
    for (k, v) in &config.extra_auth_params {
        url.push('&');
        url.push_str(&url_encode(k));
        url.push('=');
        url.push_str(&url_encode(v));
    }
    url
}

/// Token endpoint response (RFC 6749 §5.1 + OpenID extras)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    /// OIDC ID token (JWT)
    #[serde(default)]
    pub id_token: Option<String>,
}

/// Exchange an auth code for tokens at the token endpoint.
pub async fn exchange_code(
    config: &OAuth2Config,
    code: &str,
    verifier: &CodeVerifier,
) -> Result<TokenResponse> {
    let mut form: Vec<(&str, &str)> = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", config.redirect_uri.as_str()),
        ("client_id", config.client_id.as_str()),
        ("code_verifier", verifier.as_str()),
    ];
    for (k, v) in &config.extra_token_params {
        form.push((k.as_str(), v.as_str()));
    }
    let client = reqwest::Client::new();
    let resp = client
        .post(&config.token_endpoint)
        .form(&form)
        .send()
        .await
        .map_err(|e| Error::auth(format!("Token exchange failed: {e}")))?;
    let status = resp.status();
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| Error::auth(format!("Invalid token response: {e}")))?;
    if !status.is_success() {
        return Err(Error::auth(format!(
            "Token endpoint returned {}: {}",
            status, body
        )));
    }
    serde_json::from_value(body).map_err(|e| Error::auth(format!("Malformed token response: {e}")))
}

/// Refresh an access token using a stored `refresh_token`.
pub async fn refresh_token(
    config: &OAuth2Config,
    refresh_token: &str,
) -> Result<TokenResponse> {
    let mut form: Vec<(&str, &str)> = vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", config.client_id.as_str()),
    ];
    for (k, v) in &config.extra_token_params {
        form.push((k.as_str(), v.as_str()));
    }
    let client = reqwest::Client::new();
    let resp = client
        .post(&config.token_endpoint)
        .form(&form)
        .send()
        .await
        .map_err(|e| Error::auth(format!("Refresh failed: {e}")))?;
    let status = resp.status();
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| Error::auth(format!("Invalid refresh response: {e}")))?;
    if !status.is_success() {
        return Err(Error::auth(format!(
            "Refresh endpoint returned {}: {}",
            status, body
        )));
    }
    serde_json::from_value(body).map_err(|e| Error::auth(format!("Malformed refresh response: {e}")))
}

/// Generate a CSRF `state` parameter (URL-safe, ≥128 bits entropy).
pub fn generate_state() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn url_encode(s: &str) -> String {
    // Minimal RFC 3986 percent-encoder for the chars that actually appear
    // in our config. Good enough for query parameters.
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~' => out.push(b as char),
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_verifier_length_and_chars() {
        let v = CodeVerifier::generate();
        assert!(v.as_str().len() >= 43);
        assert!(v.as_str().len() <= 128);
        // Should be URL-safe base64
        for c in v.as_str().chars() {
            assert!(c.is_ascii_alphanumeric() || c == '-' || c == '_');
        }
    }

    #[test]
    fn test_code_challenge_is_deterministic() {
        let v = CodeVerifier::generate();
        let c1 = CodeChallenge::from_verifier(&v);
        let c2 = CodeChallenge::from_verifier(&v);
        assert_eq!(c1.as_str(), c2.as_str());
    }

    #[test]
    fn test_authorize_url_contains_pkce_params() {
        let cfg = OAuth2Config {
            authorization_endpoint: "https://example.com/oauth/authorize".into(),
            token_endpoint: "https://example.com/oauth/token".into(),
            client_id: "abc".into(),
            scopes: vec!["read".into(), "write".into()],
            redirect_uri: "http://127.0.0.1:9876/callback".into(),
            extra_auth_params: vec![],
            extra_token_params: vec![],
        };
        let v = CodeVerifier::generate();
        let c = CodeChallenge::from_verifier(&v);
        let state = "teststate123";
        let url = build_authorize_url(&cfg, &c, state);
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=abc"));
        assert!(url.contains("scope=read+write") || url.contains("scope=read%20write"));
        assert!(url.contains("state=teststate123"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains(&format!("code_challenge={}", c.as_str())));
    }

    #[test]
    fn test_state_is_url_safe_and_long() {
        let s = generate_state();
        assert!(s.len() >= 32);
        for c in s.chars() {
            assert!(c.is_ascii_alphanumeric() || c == '-' || c == '_');
        }
    }
}
