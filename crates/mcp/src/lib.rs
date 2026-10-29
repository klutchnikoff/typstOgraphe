//! Structured MCP adapter. Documentary logic belongs to content::api in the core.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
pub mod args;
pub mod local;
mod mcp;
pub mod targets;
pub use mcp::catalogue::{attached_command, attached_id, attached_step, attached_support, tools};
pub use mcp::{instructions, serve};

pub use mcp::validation::check as validate_request;
