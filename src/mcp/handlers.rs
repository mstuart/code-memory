//! MCP tool execution handlers.
//!
//! This module re-exports the dispatch function from `tools` and provides
//! the handler entry point for tool execution. The actual handler
//! implementations live in `tools.rs` alongside the tool definitions
//! to keep definition and implementation co-located.

use serde_json::Value;
use std::path::Path;

use crate::mcp::protocol::CallToolResult;
use crate::mcp::tools;

/// Execute a tool by name with the given arguments.
/// This is the main entry point for MCP tool execution.
pub async fn execute_tool(name: &str, args: Option<Value>, project_root: &Path) -> CallToolResult {
    tools::dispatch(name, args, project_root).await
}
