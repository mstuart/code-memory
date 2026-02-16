use anyhow::Result;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info};

use crate::mcp::protocol::*;
use crate::mcp::tools;

pub struct McpServer {
    tool_defs: Vec<ToolDefinition>,
    project_root: PathBuf,
}

impl McpServer {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            tool_defs: tools::definitions(),
            project_root,
        }
    }

    /// Run the MCP server reading JSON-RPC from stdin, writing to stdout.
    pub async fn run(&self) -> Result<()> {
        info!("claude-context MCP server starting (root: {:?})", self.project_root);

        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            debug!("Received: {}", line);

            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    error!("Failed to parse request: {}", e);
                    let resp = JsonRpcResponse::error(
                        None,
                        -32700,
                        format!("Parse error: {}", e),
                    );
                    Self::write_response(&mut stdout, &resp).await?;
                    continue;
                }
            };

            if let Some(response) = self.handle_request(request).await {
                Self::write_response(&mut stdout, &response).await?;
            }
        }

        info!("claude-context MCP server shutting down");
        Ok(())
    }

    async fn write_response(
        stdout: &mut io::Stdout,
        response: &JsonRpcResponse,
    ) -> Result<()> {
        let response_str = serde_json::to_string(response)?;
        debug!("Sending: {}", response_str);
        stdout.write_all(response_str.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
        Ok(())
    }

    /// Handle a single JSON-RPC request. Returns None for notifications.
    async fn handle_request(&self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = request.id.clone();

        match request.method.as_str() {
            // Lifecycle
            "initialize" => Some(self.handle_initialize(id)),
            "initialized" => {
                info!("Client initialized");
                None // Notification: no response per JSON-RPC spec
            }
            "notifications/cancelled" => None,

            // Tools
            "tools/list" => Some(self.handle_tools_list(id)),
            "tools/call" => Some(self.handle_tools_call(id, request.params).await),

            // Utility
            "ping" => Some(JsonRpcResponse::success(id, json!({}))),

            method => {
                error!("Unknown method: {}", method);
                Some(JsonRpcResponse::error(
                    id,
                    -32601,
                    format!("Method not found: {}", method),
                ))
            }
        }
    }

    fn handle_initialize(&self, id: Option<Value>) -> JsonRpcResponse {
        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: ToolsCapability {
                    list_changed: false,
                },
            },
            server_info: ServerInfo {
                name: "claude-context".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
    }

    fn handle_tools_list(&self, id: Option<Value>) -> JsonRpcResponse {
        JsonRpcResponse::success(id, json!({ "tools": self.tool_defs }))
    }

    async fn handle_tools_call(
        &self,
        id: Option<Value>,
        params: Option<Value>,
    ) -> JsonRpcResponse {
        let params: CallToolParams = match params {
            Some(p) => match serde_json::from_value(p) {
                Ok(p) => p,
                Err(e) => {
                    return JsonRpcResponse::error(
                        id,
                        -32602,
                        format!("Invalid params: {}", e),
                    );
                }
            },
            None => {
                return JsonRpcResponse::error(id, -32602, "Missing params".to_string());
            }
        };

        let result = tools::dispatch(
            &params.name,
            params.arguments,
            &self.project_root,
        )
        .await;

        JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
    }
}
