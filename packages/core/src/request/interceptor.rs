//! Request/response interceptors.
//!
//! ponytail: interceptor chaining (middleware-like) is not yet implemented.
//! When added, this module will host the `Interceptor` trait and pipeline.
//! For now the executor runs variable resolution → pre-request script → auth → send → post-response script,
//! which covers the same use cases without a formal interceptor abstraction.
//!
//! Add when: a plugin or user needs to insert custom logic between any two pipeline stages.
//! Design: `trait Interceptor: Send + Sync { async fn intercept(req: Request) -> Result<Request> }` + a `Vec<Box<dyn Interceptor>>` in the executor.
