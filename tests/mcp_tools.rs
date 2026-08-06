use serde_json::json;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper: create a temp project directory with sample code files and a git repo.
fn setup_test_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    // Create source files
    let src_dir = path.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("auth.rs"),
        "pub fn authenticate_user(username: &str, password: &str) -> bool {\n    true\n}\n\
         pub struct AuthToken {\n    pub token: String,\n}\n",
    )
    .unwrap();

    fs::write(
        src_dir.join("database.rs"),
        "pub fn connect_database(url: &str) -> Result<(), String> {\n    Ok(())\n}\n\
         pub struct DatabasePool {\n    pub connections: Vec<String>,\n}\n",
    )
    .unwrap();

    fs::write(
        src_dir.join("api.rs"),
        "pub fn handle_request() {}\npub fn parse_json() {}\n",
    )
    .unwrap();

    fs::write(
        src_dir.join("main.ts"),
        "import { Router } from './router';\nimport { Database } from './database';\n\
         export function startServer() {}\n",
    )
    .unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args([
            "commit",
            "-m",
            "decision: chose Rust for core, TypeScript for API layer",
        ])
        .current_dir(path)
        .output()
        .unwrap();

    dir
}

// ---------------------------------------------------------------------------
// Tool definition tests
// ---------------------------------------------------------------------------

#[test]
fn test_tool_definitions_count() {
    let defs = code_memory::mcp::tools::definitions();
    // We expect 7 tools: search_code, explain_code, trace_decision,
    // find_related, remember, index_project, get_session_patterns
    assert_eq!(defs.len(), 7);
}

#[test]
fn test_tool_definitions_names() {
    let defs = code_memory::mcp::tools::definitions();
    let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"search_code"));
    assert!(names.contains(&"explain_code"));
    assert!(names.contains(&"trace_decision"));
    assert!(names.contains(&"find_related"));
    assert!(names.contains(&"remember"));
    assert!(names.contains(&"index_project"));
    assert!(names.contains(&"get_session_patterns"));
}

#[test]
fn test_tool_definitions_have_schemas() {
    let defs = code_memory::mcp::tools::definitions();
    for def in &defs {
        assert!(!def.name.is_empty(), "Tool name should not be empty");
        assert!(
            !def.description.is_empty(),
            "Tool {} should have a description",
            def.name
        );
        assert!(
            def.input_schema.get("type").is_some(),
            "Tool {} should have an input schema with 'type'",
            def.name
        );
    }
}

#[test]
fn test_search_code_schema_requires_query() {
    let defs = code_memory::mcp::tools::definitions();
    let search = defs.iter().find(|d| d.name == "search_code").unwrap();
    let required = search.input_schema["required"].as_array().unwrap();
    assert!(
        required.iter().any(|v| v.as_str() == Some("query")),
        "search_code should require 'query'"
    );
}

#[test]
fn test_remember_schema_requires_topic_and_content() {
    let defs = code_memory::mcp::tools::definitions();
    let remember = defs.iter().find(|d| d.name == "remember").unwrap();
    let required = remember.input_schema["required"].as_array().unwrap();
    let required_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert!(required_strs.contains(&"topic"));
    assert!(required_strs.contains(&"content"));
}

// ---------------------------------------------------------------------------
// Dispatch tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_dispatch_unknown_tool() {
    let dir = TempDir::new().unwrap();
    let result = code_memory::mcp::tools::dispatch("nonexistent_tool", None, dir.path()).await;
    assert!(result.is_error.unwrap_or(false));
    let text = match &result.content[0] {
        code_memory::mcp::protocol::ToolContent::Text { text } => text.clone(),
    };
    assert!(text.contains("Unknown tool"));
}

#[tokio::test]
async fn test_dispatch_search_code_missing_query() {
    let dir = TempDir::new().unwrap();
    let result =
        code_memory::mcp::tools::dispatch("search_code", Some(json!({})), dir.path()).await;
    assert!(result.is_error.unwrap_or(false));
    let text = match &result.content[0] {
        code_memory::mcp::protocol::ToolContent::Text { text } => text.clone(),
    };
    assert!(text.contains("Missing required parameter"));
}

#[tokio::test]
async fn test_dispatch_explain_code_missing_symbol() {
    let dir = TempDir::new().unwrap();
    let result =
        code_memory::mcp::tools::dispatch("explain_code", Some(json!({})), dir.path()).await;
    assert!(result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_dispatch_trace_decision_missing_topic() {
    let dir = TempDir::new().unwrap();
    let result =
        code_memory::mcp::tools::dispatch("trace_decision", Some(json!({})), dir.path()).await;
    assert!(result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_dispatch_find_related_missing_file() {
    let dir = TempDir::new().unwrap();
    let result =
        code_memory::mcp::tools::dispatch("find_related", Some(json!({})), dir.path()).await;
    assert!(result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_dispatch_remember_missing_params() {
    let dir = TempDir::new().unwrap();
    let result = code_memory::mcp::tools::dispatch("remember", Some(json!({})), dir.path()).await;
    assert!(result.is_error.unwrap_or(false));

    let result =
        code_memory::mcp::tools::dispatch("remember", Some(json!({"topic": "test"})), dir.path())
            .await;
    assert!(result.is_error.unwrap_or(false));
}

// ---------------------------------------------------------------------------
// Integration tests with actual indexing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_index_project_tool() {
    let dir = setup_test_project();
    let result = code_memory::mcp::tools::dispatch(
        "index_project",
        Some(json!({"force": true})),
        dir.path(),
    )
    .await;
    assert!(result.is_error.is_none(), "index_project should succeed");
    let text = match &result.content[0] {
        code_memory::mcp::protocol::ToolContent::Text { text } => text.clone(),
    };
    assert!(text.contains("Reindexed project"));
    assert!(text.contains("Files indexed:"));
}

#[tokio::test]
async fn test_index_project_incremental() {
    let dir = setup_test_project();
    // First force index
    code_memory::mcp::tools::dispatch("index_project", Some(json!({"force": true})), dir.path())
        .await;

    // Second call without force should report up to date
    let result =
        code_memory::mcp::tools::dispatch("index_project", Some(json!({})), dir.path()).await;
    assert!(result.is_error.is_none());
    let text = match &result.content[0] {
        code_memory::mcp::protocol::ToolContent::Text { text } => text.clone(),
    };
    assert!(text.contains("Index is up to date") || text.contains("Files indexed"));
}

#[tokio::test]
async fn test_remember_and_recall() {
    let dir = TempDir::new().unwrap();
    let config_dir = dir.path().join(".code-memory");
    fs::create_dir_all(&config_dir).unwrap();

    let result = code_memory::mcp::tools::dispatch(
        "remember",
        Some(json!({
            "topic": "auth-pattern",
            "content": "Use JWT with refresh tokens for authentication",
            "tags": ["security", "auth"]
        })),
        dir.path(),
    )
    .await;
    assert!(result.is_error.is_none());
    let text = match &result.content[0] {
        code_memory::mcp::protocol::ToolContent::Text { text } => text.clone(),
    };
    assert!(text.contains("Remembered 'auth-pattern'"));
    assert!(text.contains("1 total entries"));

    // Verify the file was created
    let knowledge_file = config_dir.join("knowledge.json");
    assert!(knowledge_file.exists());

    let data: Vec<serde_json::Value> =
        serde_json::from_str(&fs::read_to_string(&knowledge_file).unwrap()).unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["topic"], "auth-pattern");
    assert_eq!(
        data[0]["content"],
        "Use JWT with refresh tokens for authentication"
    );
}

#[tokio::test]
async fn test_remember_updates_existing() {
    let dir = TempDir::new().unwrap();
    let config_dir = dir.path().join(".code-memory");
    fs::create_dir_all(&config_dir).unwrap();

    // First entry
    code_memory::mcp::tools::dispatch(
        "remember",
        Some(json!({
            "topic": "db-strategy",
            "content": "Use SQLite"
        })),
        dir.path(),
    )
    .await;

    // Update same topic
    code_memory::mcp::tools::dispatch(
        "remember",
        Some(json!({
            "topic": "db-strategy",
            "content": "Use PostgreSQL instead"
        })),
        dir.path(),
    )
    .await;

    let knowledge_file = config_dir.join("knowledge.json");
    let data: Vec<serde_json::Value> =
        serde_json::from_str(&fs::read_to_string(&knowledge_file).unwrap()).unwrap();
    assert_eq!(data.len(), 1, "Should update, not duplicate");
    assert_eq!(data[0]["content"], "Use PostgreSQL instead");
}

#[tokio::test]
async fn test_get_session_patterns() {
    let dir = TempDir::new().unwrap();
    let result =
        code_memory::mcp::tools::dispatch("get_session_patterns", Some(json!({})), dir.path())
            .await;
    assert!(result.is_error.is_none());
    let text = match &result.content[0] {
        code_memory::mcp::protocol::ToolContent::Text { text } => text.clone(),
    };
    // Should return some output even with no history
    assert!(
        text.contains("Session Analysis Summary")
            || text.contains("No Claude Code session history")
    );
}

#[tokio::test]
async fn test_trace_decision_with_git() {
    let dir = setup_test_project();

    let result = code_memory::mcp::tools::dispatch(
        "trace_decision",
        Some(json!({"topic": "Rust"})),
        dir.path(),
    )
    .await;
    assert!(result.is_error.is_none());
    let text = match &result.content[0] {
        code_memory::mcp::protocol::ToolContent::Text { text } => text.clone(),
    };
    // Should find the "decision: chose Rust" commit
    assert!(
        text.contains("Rust") || text.contains("decision"),
        "Should find decision about Rust: {}",
        text
    );
}

#[tokio::test]
async fn test_trace_decision_no_results() {
    let dir = setup_test_project();

    let result = code_memory::mcp::tools::dispatch(
        "trace_decision",
        Some(json!({"topic": "quantum_computing_xyz_nothing"})),
        dir.path(),
    )
    .await;
    assert!(result.is_error.is_none());
    let text = match &result.content[0] {
        code_memory::mcp::protocol::ToolContent::Text { text } => text.clone(),
    };
    assert!(text.contains("No decisions found"));
}

// ---------------------------------------------------------------------------
// Protocol / serialization tests
// ---------------------------------------------------------------------------

#[test]
fn test_call_tool_result_text_serialization() {
    let result = code_memory::mcp::protocol::CallToolResult::text("hello world".to_string());
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["content"][0]["type"], "text");
    assert_eq!(json["content"][0]["text"], "hello world");
    // isError should be absent when not an error (skip_serializing_if = None)
    assert!(
        json.get("isError").is_none(),
        "isError should not be present for success results"
    );
}

#[test]
fn test_call_tool_result_error_serialization() {
    let result = code_memory::mcp::protocol::CallToolResult::error("something broke".to_string());
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["content"][0]["type"], "text");
    assert_eq!(json["content"][0]["text"], "something broke");
    assert_eq!(json["isError"], true);
}

#[test]
fn test_jsonrpc_response_success() {
    let resp =
        code_memory::mcp::protocol::JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["jsonrpc"], "2.0");
    assert_eq!(json["id"], 1);
    assert_eq!(json["result"]["ok"], true);
    assert!(json.get("error").is_none());
}

#[test]
fn test_jsonrpc_response_error() {
    let resp = code_memory::mcp::protocol::JsonRpcResponse::error(
        Some(json!(2)),
        -32600,
        "Invalid Request".to_string(),
    );
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["jsonrpc"], "2.0");
    assert_eq!(json["id"], 2);
    assert_eq!(json["error"]["code"], -32600);
    assert_eq!(json["error"]["message"], "Invalid Request");
    assert!(json.get("result").is_none());
}

#[test]
fn test_jsonrpc_request_deserialization() {
    let input = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "search_code",
            "arguments": {
                "query": "authentication"
            }
        }
    });
    let req: code_memory::mcp::protocol::JsonRpcRequest = serde_json::from_value(input).unwrap();
    assert_eq!(req.method, "tools/call");
    assert!(req.params.is_some());
}

#[test]
fn test_call_tool_params_deserialization() {
    let input = json!({
        "name": "search_code",
        "arguments": {
            "query": "test",
            "max_results": 5
        }
    });
    let params: code_memory::mcp::protocol::CallToolParams = serde_json::from_value(input).unwrap();
    assert_eq!(params.name, "search_code");
    assert!(params.arguments.is_some());
    assert_eq!(params.arguments.unwrap()["query"], "test");
}

// ---------------------------------------------------------------------------
// Handler module tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_handler_execute_tool() {
    let dir = TempDir::new().unwrap();
    let result = code_memory::mcp::handlers::execute_tool(
        "search_code",
        Some(json!({"query": "test"})),
        dir.path(),
    )
    .await;
    // Should either succeed or fail gracefully (no panic)
    assert!(!result.content.is_empty());
}

#[tokio::test]
async fn test_handler_execute_unknown_tool() {
    let dir = TempDir::new().unwrap();
    let result = code_memory::mcp::handlers::execute_tool("does_not_exist", None, dir.path()).await;
    assert!(result.is_error.unwrap_or(false));
}
