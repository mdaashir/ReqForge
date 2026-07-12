//! Desktop OAuth 2.0 Authorization Code flow with PKCE.
//!
//! Implements the browser popup + local loopback listener that completes
//! the OAuth dance. Steps:
//! 1. Generate PKCE verifier + challenge (uses reqforge-core primitives)
//! 2. Build authorization URL
//! 3. Open the user's default browser
//! 4. Start a local TCP listener on a random port for the redirect
//! 5. Extract the `code` and `state` from the callback
//! 6. Close the listener and return the auth code

use reqforge_core::auth::pkce::{build_authorize_url, exchange_code, CodeChallenge, CodeVerifier, OAuth2Config, generate_state};
use reqforge_core::error::Error;
use reqforge_core::Result as CoreResult;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Describes a pending OAuth flow. The frontend passes this to the
/// Tauri command which orchestrates the browser + listener.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthFlowRequest {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub extra_auth_params: Vec<(String, String)>,
    pub extra_token_params: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthFlowResult {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub token_type: String,
    pub scope: Option<String>,
}

/// Shared mutable cell that holds the received auth code from the
/// local HTTP listener.
struct PendingAuth {
    code: Option<String>,
    state: Option<String>,
}

/// Launch the full OAuth PKCE flow:
/// 1. Generate PKCE verifier + challenge
/// 2. Open browser to authorization URL
/// 3. Listen on localhost for the redirect
/// 4. Exchange the code for tokens
/// 5. Return the tokens to the frontend
pub async fn run_oauth_flow(
    req: OAuthFlowRequest,
) -> CoreResult<OAuthFlowResult> {
    let state = generate_state();

    // Step 1: PKCE setup
    let verifier = CodeVerifier::generate();
    let challenge = CodeChallenge::from_verifier(&verifier);

    let config = OAuth2Config {
        authorization_endpoint: req.authorization_endpoint,
        token_endpoint: req.token_endpoint,
        client_id: req.client_id,
        scopes: req.scopes,
        redirect_uri: format!("http://127.0.0.1:{}/callback", pick_port()),
        extra_auth_params: req.extra_auth_params,
        extra_token_params: req.extra_token_params,
    };

    // Step 2: Build the authorize URL.
    let auth_url = build_authorize_url(&config, &challenge, &state);

    // Step 3: Open in browser.
    if let Err(e) = open::that(&auth_url) {
        return Err(Error::auth(format!("Failed to open browser: {e}")));
    }

    // Step 4 + 5: Listen for callback.
    let (code, _state) = listen_for_callback(&config.redirect_uri, &state).await?;

    // Step 6: Exchange code for tokens.
    let token_resp = exchange_code(&config, &code, &verifier)
        .await
        .map_err(|e| Error::auth(format!("Token exchange failed: {e}")))?;

    Ok(OAuthFlowResult {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token,
        expires_in: token_resp.expires_in,
        token_type: token_resp.token_type,
        scope: token_resp.scope,
    })
}

/// Find an available port by binding to port 0.
fn pick_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(9876)
}

/// Start a raw TCP listener on localhost. Waits for one HTTP request
/// on the `/callback` path, extracts the `code` and `state` query
/// parameters, responds with a minimal HTML page telling the user to
/// close the window, and returns the code.
async fn listen_for_callback(
    redirect_uri: &str,
    expected_state: &str,
) -> CoreResult<(String, String)> {
    let addr = redirect_uri
        .strip_prefix("http://")
        .or_else(|| redirect_uri.strip_prefix("https://"))
        .ok_or_else(|| Error::auth("Invalid redirect URI"))?;

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| Error::auth(format!("Failed to bind callback listener: {e}")))?;

    // Accept one connection.
    let (mut stream, _) = listener
        .accept()
        .await
        .map_err(|e| Error::auth(format!("Failed to accept callback: {e}")))?;

    // Read the HTTP request (first 4096 bytes).
    let mut buf = [0u8; 4096];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| Error::auth(format!("Failed to read callback: {e}")))?;
    let request = String::from_utf8_lossy(&buf[..n]).to_string();

    // Parse GET /callback?code=...&state=...
    let (code, returned_state) = parse_callback_query(&request)
        .ok_or_else(|| Error::auth("Callback missing 'code' parameter"))?;

    if returned_state != expected_state {
        return Err(Error::auth("State parameter mismatch (CSRF detected)"));
    }

    // Respond with a friendly HTML page.
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
        <html><body><h1>ReqForge</h1>\
        <p>Authorization complete. You may close this window.</p></body></html>";
    let _ = stream.write_all(response.as_bytes()).await;

    Ok((code, returned_state))
}

/// Minimal query-string parser for `GET /callback?code=X&state=Y`.
fn parse_callback_query(request: &str) -> Option<(String, String)> {
    let first_line = request.lines().next()?;
    let query = first_line.split(' ').nth(1)?; // e.g. "/callback?code=X&state=Y"
    let query = query.split('?').nth(1)?;

    let mut code = None;
    let mut state = None;

    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next()?.trim();
        let value = parts.next()?.trim();
        match key {
            "code" => code = Some(url_decode(value)),
            "state" => state = Some(url_decode(value)),
            _ => {}
        }
    }

    Some((code?, state?))
}

fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '+' => out.push(' '),
            '%' => {
                let hi = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
                let lo = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
                out.push(char::from((hi * 16 + lo) as u8));
            }
            _ => out.push(ch),
        }
    }
    out
}

use tauri::command;
use open;
