//! Stub sync implementation used when the `sync` feature is disabled.

use crate::sync::{AwarenessSnapshot, AwarenessState};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("cloud sync is not compiled in (build reqforge-core with --features sync)")]
    Disabled,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// A no-op document. Holds state locally but doesn't sync anywhere.
pub struct SyncDocument {
    state: HashMap<String, Vec<u8>>,
}

impl SyncDocument {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
        }
    }

    pub fn put(&mut self, key: impl Into<String>, value: Vec<u8>) {
        self.state.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<&[u8]> {
        self.state.get(key).map(|v| v.as_slice())
    }

    pub fn snapshot(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (k, v) in &self.state {
            out.extend_from_slice(&(k.len() as u32).to_le_bytes());
            out.extend_from_slice(k.as_bytes());
            out.extend_from_slice(&(v.len() as u32).to_le_bytes());
            out.extend_from_slice(v);
        }
        out
    }

    pub fn load_snapshot(&mut self, bytes: &[u8]) -> Result<(), SyncError> {
        self.state.clear();
        let mut i = 0;
        while i < bytes.len() {
            if i + 4 > bytes.len() {
                return Ok(());
            }
            let klen = u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap_or([0; 4])) as usize;
            i += 4;
            if i + klen > bytes.len() {
                return Ok(());
            }
            let k = String::from_utf8_lossy(&bytes[i..i + klen]).to_string();
            i += klen;
            if i + 4 > bytes.len() {
                return Ok(());
            }
            let vlen = u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap_or([0; 4])) as usize;
            i += 4;
            if i + vlen > bytes.len() {
                return Ok(());
            }
            let v = bytes[i..i + vlen].to_vec();
            i += vlen;
            self.state.insert(k, v);
        }
        Ok(())
    }

    pub fn set_local_awareness(&mut self, _state: AwarenessState) -> Result<(), SyncError> {
        Ok(())
    }

    pub fn awareness(&self) -> AwarenessSnapshot {
        AwarenessSnapshot::default()
    }
}

impl Default for SyncDocument {
    fn default() -> Self {
        Self::new()
    }
}
