//! Credential vault abstraction.
//!
//! The primary vault implementation lives in the desktop app at
//! `apps/desktop/src-tauri/src/keychain.rs` and uses the OS keychain
//! via `keyring-rs`.
//!
//! This module defines the trait that the desktop vault implements,
//! so that call sites in `reqforge-core` can depend on the abstraction
//! without knowing the platform-specific backend.
//!
//! ponytail: add `CredentialVault` trait + `MemoryVault` test double here
//! when core needs to store credentials without a desktop dependency.
//! For now, the desktop app handles keychain ops directly via Tauri commands.
