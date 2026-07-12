//! OpenAPI 3.0/3.1 importer.
//!
//! Reads an OpenAPI/Swagger spec and produces a ReqForge collection.
//!
//! ponytail: implement when OpenAPI import is needed. The `Importer`
//! trait in `mod.rs` defines the API. Design: walk paths/methods from
//! the OpenAPI spec, generate `Request` objects, group by operation tag.
