//! Optional cloud sync via a Yrs (Yjs-compatible) document.
//!
//! When the `sync` feature is enabled, collections and environments can be
//! synced across devices using Yrs' state-based CRDT — the same doc format
//! Yjs uses in the browser. This means a desktop app can sync to a web
//! client (and vice versa) through any binary transport (WebSocket, IPC,
//! file drop).
//!
//! When the feature is disabled, the types still exist so call sites don't
//! need to change — they just hold a `SyncDocument` that is a no-op.

pub mod cloud;
#[cfg(feature = "watcher")]
pub mod git;

#[cfg(feature = "sync")]
pub mod crdt;

#[cfg(feature = "sync")]
mod yrs_sync;
#[cfg(feature = "sync")]
pub use yrs_sync::*;

#[cfg(not(feature = "sync"))]
mod stub;
#[cfg(not(feature = "sync"))]
pub use stub::*;

/// Awareness state broadcast alongside a document for live cursors and
/// presence. Identical shape across the `sync` and stub implementations
/// so call sites are uniform.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct AwarenessState {
    /// Optional human-readable name (e.g. "Ada Lovelace").
    pub user_name: Option<String>,
    /// Hex colour for the user's cursor, e.g. "#ff6b6b".
    pub user_color: Option<String>,
    /// Most recently focused request id, if any.
    pub focused_request: Option<String>,
    /// Cursor position in the request URL bar (character offset). Used by
    /// the collab-cursors overlay.
    pub cursor: Option<CursorPosition>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CursorPosition {
    pub request_id: String,
    /// Field name, e.g. "url", "headers[0]", "body".
    pub field: String,
    /// Character offset within the field.
    pub offset: u32,
}

/// A single snapshot of who-is-where at a moment in time.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AwarenessSnapshot {
    pub peers: Vec<Peer>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Peer {
    pub client_id: u64,
    pub state: AwarenessState,
    /// When this peer was last seen (milliseconds since epoch).
    pub last_seen_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_awareness_round_trip() {
        let state = AwarenessState {
            user_name: Some("Ada".into()),
            user_color: Some("#ff6b6b".into()),
            focused_request: Some("req-1".into()),
            cursor: Some(CursorPosition {
                request_id: "req-1".into(),
                field: "url".into(),
                offset: 7,
            }),
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: AwarenessState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, state);
    }
}
