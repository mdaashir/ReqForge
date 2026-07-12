//! gRPC via tonic — placeholder.
//!
//! ## Status
//!
//! The feature-gated module architecture is in place. `tonic` 0.14 is
//! available as a dependency but the generic client API (`Grpc::new`)
//! is behind the `codegen` feature and the exact API differs between
//! tonic versions. Building the dynamic-call wrapper requires matching
//! the running tonic's internal `Grpc<T>` API.
//!
//! ## How to enable
//!
//! 1. Identify the correct `Grpc::new(channel)` signature for the pinned
//!    tonic version (the generic `T: GrpcService<BoxBody>` constraint).
//! 2. Replace the stub methods below.
//! 3. Enable `[features] grpc-tonic` in Cargo.toml and add `dep:tonic`.
//!
//! Meanwhile, the gRPC-Web JSON backend (`grpc_web.rs`) handles all
//! unary calls and is active by default.

use crate::error::{Error, Result};
use crate::protocol::grpc::GrpcStreamMessage;
use crate::protocol::ProtocolCapabilities;
use crate::request::{Request, Response};
use tokio::sync::mpsc;

pub struct Backend;

impl Backend {
    pub fn new() -> Self { Self }

    pub fn name(&self) -> &str { "gRPC (tonic — placeholder)" }

    pub fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities { can_send: true, can_receive: true, can_stream: false, can_subscribe: false }
    }

    pub async fn invoke_unary(&self, _request: Request) -> Result<Response> {
        Err(Error::other("tonic unary not yet implemented; use default grpc-web backend"))
    }

    pub async fn invoke_server_streaming(&self, _request: Request) -> Result<mpsc::Receiver<std::result::Result<GrpcStreamMessage, Error>>> {
        Err(Error::other("tonic streaming not yet implemented"))
    }

    pub async fn invoke_bidi_stream(&self, _request: Request) -> Result<(mpsc::Sender<Vec<u8>>, mpsc::Receiver<std::result::Result<GrpcStreamMessage, Error>>)> {
        Err(Error::other("tonic bidi not yet implemented"))
    }

    pub async fn resolve_method(&self, _target: &str, _service: &str, _method: &str) -> Result<crate::protocol::grpc::GrpcMethodDescriptor> {
        Err(Error::other("tonic reflection not yet implemented"))
    }
}
