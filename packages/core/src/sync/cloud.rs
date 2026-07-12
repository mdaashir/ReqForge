//! Cloud sync service (Pro/Enterprise feature).
//!
//! Stub — cloud sync is optional and not part of the open-source core.
//! The CRDT sync engine in `yrs_sync` provides the building blocks;
//! a cloud transport (WebSocket, REST, or similar) connects to the
//! ReqForge Cloud or a self-hosted backend.
//!
//! ponytail: implement when cloud-sync feature is added.
//! Design: `trait CloudTransport { async fn push(doc: &[u8]); async fn pull() -> Vec<u8>; }`
