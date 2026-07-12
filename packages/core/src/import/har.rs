//! HAR (HTTP Archive) importer.
//!
//! Reads a `.har` file and produces a ReqForge collection.
//!
//! ponytail: implement when HAR import is needed. The `Importer` trait
//! in `mod.rs` defines the API. Design: parse the HAR JSON `entries[]`
//! array, map `request`/`response` objects to ReqForge types.
