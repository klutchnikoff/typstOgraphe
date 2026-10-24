//! Adaptateur MCP partagé par la CLI autonome et l’application.
//! Les transports et les sélecteurs restent en aval du cœur.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
pub mod args;
mod calls;
mod mcp;
pub mod targets;
pub use calls::compile_render;
pub use mcp::serve;

pub mod local;
