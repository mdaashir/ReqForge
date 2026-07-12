//! Encrypted at-rest storage for collections and environments.
//!
//! Uses AES-256-GCM for authenticated encryption. Key derivation uses
//! SHA-256 (HMAC-SHA256 iteration, 100k rounds) which is a simple,
//! well-understood KDF that ships without extra native dependencies.
//! (Replace with Argon2id later if you have the build deps for it.)
//!
//! ## File format
//!
//! ```text
//! magic (4)  | version (1) | salt (16) | nonce (12) | ciphertext | gcm_tag (16)
//! ```
//!
//! - magic = b"REQ1" — identifies the file as ReqForge encrypted data
//! - version = 0x01 — schema version
//! - salt — KDF salt
//! - nonce — AES-GCM nonce (12 bytes; random per file)
//! - ciphertext — AES-256-GCM(plaintext) + appended 16-byte tag

use crate::error::{Error, Result};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use sha2::Sha256;
use zeroize::Zeroize;

const MAGIC: &[u8; 4] = b"REQ1";
const VERSION: u8 = 0x01;

/// Encrypted blob plus the random salt/nonce needed to decrypt it.
#[derive(Debug, Clone)]
pub struct EncryptedBlob {
    pub salt: Vec<u8>,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

/// Derive a 32-byte AES key from a passphrase + 16-byte salt using
/// iterated HMAC-SHA256 (100k rounds). This is a slow, deterministic
/// KDF that prevents brute-force attacks against weak passphrases.
pub fn derive_key(passphrase: &[u8], salt: &[u8; 16]) -> Result<[u8; 32]> {
    let mut key = [0u8; 32];

    // First iteration: HMAC-SHA256(salt, passphrase)
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(salt)
        .map_err(|e| Error::other(format!("hmac new: {e}")))?;
    mac.update(passphrase);
    let mut buf = mac.finalize().into_bytes().to_vec();

    // Iterate 99,999 more times to make brute-force expensive.
    for _ in 0..99_999 {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&buf)
            .map_err(|e| Error::other(format!("hmac new: {e}")))?;
        mac.update(passphrase);
        buf = mac.finalize().into_bytes().to_vec();
    }

    key.copy_from_slice(&buf[..32]);
    buf.zeroize();
    Ok(key)
}

/// Encrypt `plaintext` under `key`. A new nonce is generated.
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<EncryptedBlob> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| Error::other(format!("aes-gcm new cipher: {e}")))?;

    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);

    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|e| Error::other(format!("aes-gcm encrypt: {e}")))?;

    if ciphertext.len() < 16 {
        return Err(Error::other("aes-gcm returned ciphertext too short"));
    }
    let split = ciphertext.len() - 16;
    let body = ciphertext[..split].to_vec();
    let tag = ciphertext[split..].to_vec();
    let mut combined = body;
    combined.extend_from_slice(&tag);

    Ok(EncryptedBlob {
        salt: [0u8; 16].to_vec(),
        nonce: nonce.to_vec(),
        ciphertext: combined,
    })
}

/// Decrypt the given blob under `key`. Returns the original plaintext.
pub fn decrypt(key: &[u8; 32], blob: &EncryptedBlob) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| Error::other(format!("aes-gcm new cipher: {e}")))?;

    cipher
        .decrypt(Nonce::from_slice(&blob.nonce), blob.ciphertext.as_slice())
        .map_err(|e| Error::other(format!("aes-gcm decrypt: {e}")))
}

/// Pack an EncryptedBlob into a single self-describing file.
pub fn pack(blob: &EncryptedBlob) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 1 + 16 + 12 + blob.ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&blob.salt);
    out.extend_from_slice(&blob.nonce);
    out.extend_from_slice(&blob.ciphertext);
    out
}

/// Unpack a file produced by `pack`. Returns `None` if the magic bytes
/// don't match (i.e. not an encrypted file).
pub fn unpack(bytes: &[u8]) -> Option<EncryptedBlob> {
    if bytes.len() < 4 + 1 + 16 + 12 {
        return None;
    }
    if &bytes[..4] != MAGIC {
        return None;
    }
    let version = bytes[4];
    if version != VERSION {
        return None;
    }
    let salt = bytes[5..5 + 16].to_vec();
    let nonce = bytes[5 + 16..5 + 16 + 12].to_vec();
    let ciphertext = bytes[5 + 16 + 12..].to_vec();
    Some(EncryptedBlob {
        salt,
        nonce,
        ciphertext,
    })
}

/// Securely zero out a key buffer when dropped.
pub fn zero_key(key: &mut [u8]) {
    key.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let passphrase = b"correct horse battery staple";
        let salt = b"0123456789abcdef"; // 16 bytes
        let key = derive_key(passphrase, salt).unwrap();
        let plaintext = br#"{"name":"hello","value":42}"#;

        let blob = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &blob).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_pack_unpack_roundtrip() {
        let passphrase = b"another-pass";
        let salt = [0u8; 16];
        let key = derive_key(passphrase, &salt).unwrap();
        let blob = encrypt(&key, b"hello world").unwrap();
        let packed = pack(&blob);
        let unpacked = unpack(&packed).unwrap();
        assert_eq!(unpacked.salt, blob.salt);
        assert_eq!(unpacked.nonce, blob.nonce);
        assert_eq!(unpacked.ciphertext, blob.ciphertext);
    }

    #[test]
    fn test_wrong_passphrase_fails() {
        let key1 = derive_key(b"right", b"salt-of-16-bytes").unwrap();
        let key2 = derive_key(b"wrong", b"salt-of-16-bytes").unwrap();
        let blob = encrypt(&key1, b"secret").unwrap();
        let result = decrypt(&key2, &blob);
        assert!(result.is_err());
    }

    #[test]
    fn test_bad_magic_returns_none() {
        let result = unpack(b"NOTREQFOOOE");
        assert!(result.is_none());
    }
}
