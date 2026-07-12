//! JWT signing (HS256, RS256) and verification.
//!
//! Implements the small slice of RFC 7519 we actually need: produce and
//! parse signed tokens. The HMAC-SHA256 implementation is inline so we
//! don't pull in extra deps; RSA signing is feature-gated behind
//! `rsa-signing` so it stays opt-in.

use crate::error::{Error, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::Digest;

/// JWT header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtHeader {
    pub alg: String,
    #[serde(default = "default_typ")]
    pub typ: String,
}

fn default_typ() -> String {
    "JWT".to_string()
}

/// Standard claims (RFC 7519 §4.1).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Claims {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iat: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Key material for signing operations
#[derive(Debug, Clone)]
pub enum JwtKey {
    /// HMAC secret (≥16 bytes enforced by `sign_hs256`).
    Hmac(Vec<u8>),
    /// RSA private key in PEM format (PKCS#8 or PKCS#1).
    RsaPem(String),
}

/// Sign a JWT with HS256 (HMAC-SHA256).
pub fn sign_hs256(claims: &Claims, secret: &[u8]) -> Result<String> {
    if secret.len() < 16 {
        return Err(Error::auth(
            "HS256 secret is too short — use at least 16 bytes (32 recommended)",
        ));
    }
    let header = JwtHeader {
        alg: "HS256".to_string(),
        typ: "JWT".to_string(),
    };
    let header_b64 = b64url_encode(&serde_json::to_vec(&header)?);
    let payload_b64 = b64url_encode(&serde_json::to_vec(claims)?);
    let signing_input = format!("{}.{}", header_b64, payload_b64);
    let mut mac = HmacSha256State::new(secret);
    mac.update(signing_input.as_bytes());
    let sig = mac.finalize();
    Ok(format!("{}.{}", signing_input, b64url_encode(&sig)))
}

/// Sign a JWT with RS256 (RSA-PKCS#1 v1.5 + SHA256).
///
/// Requires the `rsa-signing` feature flag.
pub fn sign_rs256(claims: &Claims, pem_private_key: &str) -> Result<String> {
    #[cfg(feature = "rsa-signing")]
    {
        use rsa::pkcs1v15::{Signature, SigningKey};
        use rsa::pkcs8::DecodePrivateKey;
        use rsa::signature::{SignatureEncoding, Signer};
        use rsa::RsaPrivateKey;
        use sha2::Sha256;

        let key = RsaPrivateKey::from_pkcs8_pem(pem_private_key)
            .or_else(|_| {
                use rsa::pkcs1::DecodeRsaPrivateKey;
                RsaPrivateKey::from_pkcs1_pem(pem_private_key)
            })
            .map_err(|e| Error::auth(format!("Invalid RSA private key: {e}")))?;

        let header = JwtHeader {
            alg: "RS256".to_string(),
            typ: "JWT".to_string(),
        };
        let header_b64 = b64url_encode(&serde_json::to_vec(&header)?);
        let payload_b64 = b64url_encode(&serde_json::to_vec(claims)?);
        let signing_input = format!("{}.{}", header_b64, payload_b64);
        let signing_key = SigningKey::<Sha256>::new(key);
        let sig: Signature = signing_key.sign(signing_input.as_bytes());
        Ok(format!(
            "{}.{}",
            signing_input,
            b64url_encode(&sig.to_bytes())
        ))
    }
    #[cfg(not(feature = "rsa-signing"))]
    {
        let _ = (claims, pem_private_key);
        Err(Error::other(
            "RS256 signing requires the `rsa-signing` feature on reqforge-core",
        ))
    }
}

/// Decode (without verifying) the JWT header and payload.
pub fn decode_unverified(token: &str) -> Result<(JwtHeader, Claims)> {
    let mut parts = token.split('.');
    let header_b64 = parts
        .next()
        .ok_or_else(|| Error::auth("JWT missing header"))?;
    let payload_b64 = parts
        .next()
        .ok_or_else(|| Error::auth("JWT missing payload"))?;
    let header: JwtHeader = serde_json::from_slice(&b64url_decode(header_b64)?)?;
    let claims: Claims = serde_json::from_slice(&b64url_decode(payload_b64)?)?;
    Ok((header, claims))
}

/// Verify an HS256-signed JWT.
pub fn verify_hs256(token: &str, secret: &[u8]) -> Result<Claims> {
    let (header, claims) = decode_unverified(token)?;
    if header.alg != "HS256" {
        return Err(Error::auth(format!(
            "Unexpected JWT alg: {} (expected HS256)",
            header.alg
        )));
    }
    let mut parts = token.split('.');
    let header_b64 = parts.next().unwrap_or("");
    let payload_b64 = parts.next().unwrap_or("");
    let signature_b64 = parts.next().unwrap_or("");

    let signing_input = format!("{}.{}", header_b64, payload_b64);
    let mut mac = HmacSha256State::new(secret);
    mac.update(signing_input.as_bytes());
    let expected = mac.finalize();

    let provided = b64url_decode(signature_b64)?;
    if !constant_time_eq(&expected, &provided) {
        return Err(Error::auth("JWT signature mismatch"));
    }
    Ok(claims)
}

fn b64url_encode(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn b64url_decode(s: &str) -> Result<Vec<u8>> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|e| Error::auth(format!("Base64 decode failed: {e}")))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ---- HMAC-SHA256 (RFC 2104) ----------------------------------------------
// Inline implementation so we don't add another dep. The construction is
// the canonical one: H((K ⊕ opad) || H((K ⊕ ipad) || message)).

/// Minimal HMAC-SHA256 implementation. Renamed to avoid colliding with
/// `sha2::Hmac<Sha256>` which also exports an `HmacSha256` name in some
/// re-export configurations.
struct HmacSha256State {
    key_block: [u8; 64],
    buffer: Vec<u8>,
}

impl HmacSha256State {
    fn new(key: &[u8]) -> Self {
        let mut key_block = [0u8; 64];
        if key.len() > 64 {
            let hashed = sha2::Sha256::digest(key);
            key_block[..hashed.len()].copy_from_slice(&hashed);
        } else {
            key_block[..key.len()].copy_from_slice(key);
        }
        Self {
            key_block,
            buffer: Vec::new(),
        }
    }

    fn update(&mut self, data: &[u8]) -> &mut Self {
        self.buffer.extend_from_slice(data);
        self
    }

    fn finalize(&self) -> Vec<u8> {
        let mut inner_hasher = sha2::Sha256::new();
        for b in &self.key_block {
            inner_hasher.update([b ^ 0x36]);
        }
        inner_hasher.update(&self.buffer);
        let inner = inner_hasher.finalize();

        let mut outer_hasher = sha2::Sha256::new();
        for b in &self.key_block {
            outer_hasher.update([b ^ 0x5c]);
        }
        outer_hasher.update(inner);
        outer_hasher.finalize().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify_hs256() {
        let claims = Claims {
            sub: Some("user-1".into()),
            exp: Some(2_000_000_000),
            ..Default::default()
        };
        let secret = b"this-is-a-test-secret-32bytes!!";
        let token = sign_hs256(&claims, secret).unwrap();
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        let decoded = verify_hs256(&token, secret).unwrap();
        assert_eq!(decoded.sub.as_deref(), Some("user-1"));
    }

    #[test]
    fn test_signature_mismatch_detected() {
        let claims = Claims::default();
        let token = sign_hs256(&claims, b"good-secret-very-long-yes!").unwrap();
        let err = verify_hs256(&token, b"different-secret-also-long!").unwrap_err();
        assert!(format!("{err}").contains("signature"));
    }

    #[test]
    fn test_decode_unverified_extracts_claims() {
        let claims = Claims {
            iss: Some("https://example.com".into()),
            iat: Some(1_700_000_000),
            ..Default::default()
        };
        let token = sign_hs256(&claims, b"any-secret-of-sufficient-length!").unwrap();
        let (header, decoded) = decode_unverified(&token).unwrap();
        assert_eq!(header.alg, "HS256");
        assert_eq!(decoded.iss.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn test_extra_claims_round_trip() {
        let json = r#"{"sub":"x","role":"admin","tenant":42}"#;
        let claims: Claims = serde_json::from_str(json).unwrap();
        let token = sign_hs256(&claims, b"secret-key-of-sufficient-length!").unwrap();
        let decoded = verify_hs256(&token, b"secret-key-of-sufficient-length!").unwrap();
        assert_eq!(
            decoded.extra.get("role").unwrap(),
            &serde_json::json!("admin")
        );
        assert_eq!(decoded.extra.get("tenant").unwrap(), &serde_json::json!(42));
    }

    #[test]
    fn test_weak_secret_rejected() {
        let claims = Claims::default();
        let err = sign_hs256(&claims, b"short").unwrap_err();
        assert!(format!("{err}").contains("too short"));
    }
}
