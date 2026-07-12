//! `reqforge mock` — start a local mock server from a collection.
//!
//! Serves mock responses from collection rules on a local port.

use anyhow::Result;
use reqforge_core::mock::MockServer;

pub async fn execute(workspace: &str, port: Option<u16>) -> Result<()> {
    let _ = workspace;
    let mut server = MockServer::new();
    // ponytail: load rules from collection or file;
    // for now starts with default fallback rule (404).
    let addr = server.start().await?;
    println!("Mock server started: http://localhost:{}/", addr.port());
    println!("Press Ctrl+C to stop...");
    tokio::signal::ctrl_c().await?;
    // server.stop() — not yet exposed on MockServer
    println!("Mock server stopped.");
    Ok(())
}
