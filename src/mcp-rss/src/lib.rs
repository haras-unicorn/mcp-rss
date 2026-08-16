//! mcp-rss - MCP server that provides RSS tooling.

#![deny(unsafe_code)]
#![deny(
  clippy::unwrap_used,
  clippy::expect_used,
  clippy::panic,
  clippy::unreachable
)]
#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::todo)]
#![deny(clippy::allow_attributes_without_reason)]

use rmcp::tool_router;

/// MCP server that provides RSS tooling.
#[derive(Debug, Clone)]
pub struct RssServer;

#[tool_router(server_handler)]
impl RssServer {}
