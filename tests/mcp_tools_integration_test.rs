//! Integration tests for MCP tools
//!
//! This test module verifies:
//! 1. Mock MCP server responses
//! 2. Tool injection into agent sessions
//! 3. Playwright and other tools availability
//! 4. Tool failure handling
//!
//! MCP (Model Context Protocol) uses JSON-RPC 2.0 for communication
//! between the orchestrator and tool servers (like Playwright).

use ltmatrix::config::mcp::{LoadedMcpConfig, McpConfig};
use ltmatrix::mcp::protocol::{
    JsonRpcError, JsonRpcErrorCode, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse, RequestId,
};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// Mock MCP Server Response Tests
// ============================================================================

/// Simulates a successful tool list response from an MCP server
fn mock_tool_list_response(tools: Vec<&str>) -> JsonRpcResponse {
    let tools_json: Vec<serde_json::Value> = tools
        .iter()
        .map(|name| {
            json!({
                "name": name,
                "description": format!("{} tool for testing", name),
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            })
        })
        .collect();

    JsonRpcResponse::success(
        RequestId::Number(1),
        json!({
            "tools": tools_json
        }),
    )
}

/// Simulates a successful tool execution response
fn mock_tool_execution_response(result: &str) -> JsonRpcResponse {
    JsonRpcResponse::success(
        RequestId::Number(2),
        json!({
            "content": [{
                "type": "text",
                "text": result
            }]
        }),
    )
}

/// Simulates an error response from an MCP server
fn mock_tool_error_response(code: JsonRpcErrorCode, message: &str) -> JsonRpcResponse {
    JsonRpcResponse::error(
        RequestId::Number(1),
        JsonRpcError::new(code, message.to_string()),
    )
}

#[test]
fn test_mock_mcp_server_tool_list_response() {
    // Simulate Playwright MCP server returning available tools
    let response = mock_tool_list_response(vec![
        "browser_navigate",
        "browser_click",
        "browser_type",
        "browser_snapshot",
        "browser_take_screenshot",
    ]);

    assert!(response.is_success());
    let result = response.get_result().unwrap();
    let tools = result.get("tools").unwrap().as_array().unwrap();

    assert_eq!(tools.len(), 5);

    // Verify tool names
    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();

    assert!(tool_names.contains(&"browser_navigate"));
    assert!(tool_names.contains(&"browser_click"));
    assert!(tool_names.contains(&"browser_snapshot"));
}

#[test]
fn test_mock_mcp_server_tool_execution_response() {
    // Simulate a successful tool execution
    let response = mock_tool_execution_response("Successfully navigated to https://example.com");

    assert!(response.is_success());

    let result = response.get_result().unwrap();
    let content = result.get("content").unwrap().as_array().unwrap();
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "text");
    assert!(content[0]["text"]
        .as_str()
        .unwrap()
        .contains("Successfully"));
}

#[test]
fn test_mock_mcp_server_error_response() {
    // Simulate tool execution failure
    let response = mock_tool_error_response(
        JsonRpcErrorCode::InvalidParams,
        "Missing required parameter: url",
    );

    assert!(response.is_error());
    let error = response.get_error().unwrap();
    assert_eq!(error.code, -32602); // Invalid params code
    assert!(error.message.contains("Missing required parameter"));
}

#[test]
fn test_mock_mcp_server_timeout_error() {
    // Simulate a timeout error
    let response = mock_tool_error_response(
        JsonRpcErrorCode::ServerError(-32001),
        "Tool execution timed out after 30 seconds",
    );

    assert!(response.is_error());
    let error = response.get_error().unwrap();
    assert_eq!(error.code, -32001);
    assert!(error.message.contains("timed out"));
}

#[test]
fn test_mock_mcp_server_method_not_found() {
    // Simulate calling a non-existent tool
    let response = mock_tool_error_response(
        JsonRpcErrorCode::MethodNotFound,
        "Tool 'browser_fly' not found",
    );

    assert!(response.is_error());
    let error = response.get_error().unwrap();
    assert_eq!(error.code, -32601); // Method not found code
}

// ============================================================================
// JSON-RPC Message Serialization Tests
// ============================================================================

#[test]
fn test_mcp_request_serialization_for_tool_call() {
    // Create a proper MCP tool call request
    let request = JsonRpcRequest::with_params(
        RequestId::String("tool-call-001".into()),
        "tools/call",
        json!({
            "name": "browser_navigate",
            "arguments": {
                "url": "https://example.com"
            }
        }),
    );

    let json = request.to_json().unwrap();

    // Verify JSON structure
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"id\":\"tool-call-001\""));
    assert!(json.contains("\"method\":\"tools/call\""));
    assert!(json.contains("browser_navigate"));
    assert!(json.contains("https://example.com"));
}

#[test]
fn test_mcp_request_for_tool_list() {
    // Create a request to list available tools
    let request = JsonRpcRequest::new(RequestId::Number(1), "tools/list");

    let json = request.to_json().unwrap();

    assert!(json.contains("\"method\":\"tools/list\""));
    assert!(json.contains("\"id\":1"));
}

#[test]
fn test_mcp_notification_for_server_events() {
    // Create a notification for server lifecycle events
    let notification = JsonRpcNotification::with_params(
        "notifications/initialized",
        json!({
            "serverName": "playwright",
            "version": "1.0.0"
        }),
    );

    let json = notification.to_json().unwrap();

    // Notifications should NOT have an id field
    assert!(!json.contains("\"id\""));
    assert!(json.contains("\"method\":\"notifications/initialized\""));
    assert!(json.contains("playwright"));
}

#[test]
fn test_mcp_message_parsing_from_server_response() {
    // Parse a response from an MCP server
    let json = r#"{
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "tools": [
                {
                    "name": "browser_click",
                    "description": "Click an element",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "selector": {"type": "string"}
                        }
                    }
                }
            ]
        }
    }"#;

    let message = JsonRpcMessage::from_json(json).unwrap();

    assert!(message.is_response());
    if let JsonRpcMessage::Response(response) = message {
        assert!(response.is_success());
        let result = response.get_result().unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "browser_click");
    } else {
        panic!("Expected response message");
    }
}

// ============================================================================
// MCP Server Configuration Tests
// ============================================================================

#[test]
fn test_playwright_server_configuration() {
    let config_content = r#"
[mcp.servers.playwright]
type = "playwright"
command = "npx"
args = ["-y", "@playwright/mcp@latest"]
timeout = 60
enabled = true

[mcp.servers.playwright.env]
HEADLESS = "true"
BROWSER = "chromium"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("playwright").unwrap();

    assert_eq!(server.server_type, "playwright");
    assert_eq!(server.command, Some("npx".to_string()));
    assert_eq!(server.args, vec!["-y", "@playwright/mcp@latest"]);
    assert_eq!(server.timeout, 60);
    assert!(server.enabled);
    assert_eq!(server.env.get("HEADLESS"), Some(&"true".to_string()));
    assert_eq!(server.env.get("BROWSER"), Some(&"chromium".to_string()));
}

#[test]
fn test_multiple_tool_servers_configuration() {
    let config_content = r#"
[mcp.servers.playwright]
type = "playwright"
command = "npx"
args = ["-y", "@playwright/mcp@latest"]

[mcp.servers.filesystem]
type = "filesystem"
command = "mcp-server-filesystem"
args = ["--root", "/workspace"]

[mcp.servers.fetch]
type = "fetch"
command = "mcp-server-fetch"
timeout = 30
"#;

    let config = McpConfig::from_str(config_content).unwrap();

    assert_eq!(config.mcp.servers.len(), 3);
    assert!(config.get_server("playwright").is_some());
    assert!(config.get_server("filesystem").is_some());
    assert!(config.get_server("fetch").is_some());
}

#[test]
fn test_server_disabled_gracefully() {
    let config_content = r#"
[mcp.servers.playwright]
type = "playwright"
enabled = false

[mcp.servers.browser]
type = "browser"
enabled = true
"#;

    let config = McpConfig::from_str(config_content).unwrap();

    // Check individual server enabled status
    assert!(!config.is_server_enabled("playwright"));
    assert!(config.is_server_enabled("browser"));

    // Check enabled_servers filter
    let enabled = config.enabled_servers();
    assert_eq!(enabled.len(), 1);
    assert!(enabled.contains_key("browser"));
    assert!(!enabled.contains_key("playwright"));
}

// ============================================================================
// Tool Availability Verification Tests
// ============================================================================

/// Mock MCP server tool registry for testing
struct MockToolRegistry {
    servers: HashMap<String, Vec<String>>,
}

impl MockToolRegistry {
    fn new() -> Self {
        MockToolRegistry {
            servers: HashMap::new(),
        }
    }

    fn register_server(&mut self, name: &str, tools: Vec<&str>) {
        self.servers.insert(
            name.to_string(),
            tools.iter().map(|s| s.to_string()).collect(),
        );
    }

    fn get_tools(&self, server: &str) -> Option<&Vec<String>> {
        self.servers.get(server)
    }

    fn has_tool(&self, server: &str, tool: &str) -> bool {
        self.servers
            .get(server)
            .map(|tools| tools.iter().any(|t| t == tool))
            .unwrap_or(false)
    }

    fn all_tools(&self) -> Vec<&String> {
        self.servers
            .values()
            .flat_map(|tools| tools.iter())
            .collect()
    }
}

#[test]
fn test_tool_registry_playwright_tools() {
    let mut registry = MockToolRegistry::new();

    // Register Playwright tools
    registry.register_server(
        "playwright",
        vec![
            "browser_navigate",
            "browser_click",
            "browser_type",
            "browser_snapshot",
            "browser_take_screenshot",
            "browser_evaluate",
            "browser_file_upload",
        ],
    );

    // Verify Playwright tools are available
    assert!(registry.has_tool("playwright", "browser_navigate"));
    assert!(registry.has_tool("playwright", "browser_click"));
    assert!(registry.has_tool("playwright", "browser_snapshot"));
    assert!(!registry.has_tool("playwright", "nonexistent_tool"));

    let tools = registry.get_tools("playwright").unwrap();
    assert_eq!(tools.len(), 7);
}

#[test]
fn test_tool_registry_multiple_servers() {
    let mut registry = MockToolRegistry::new();

    registry.register_server("playwright", vec!["browser_click", "browser_navigate"]);
    registry.register_server(
        "filesystem",
        vec!["read_file", "write_file", "list_directory"],
    );
    registry.register_server("fetch", vec!["fetch_url", "fetch_json"]);

    // Verify tools across servers
    assert!(registry.has_tool("playwright", "browser_click"));
    assert!(registry.has_tool("filesystem", "read_file"));
    assert!(registry.has_tool("fetch", "fetch_url"));

    // Verify cross-server tool isolation
    assert!(!registry.has_tool("playwright", "read_file"));
    assert!(!registry.has_tool("filesystem", "browser_click"));

    // Verify all tools collection
    let all_tools = registry.all_tools();
    assert_eq!(all_tools.len(), 7);
}

#[test]
fn test_playwright_browser_automation_tools() {
    // Define expected Playwright tools for browser automation
    let playwright_tools = vec![
        "browser_navigate",
        "browser_click",
        "browser_type",
        "browser_snapshot",
        "browser_take_screenshot",
        "browser_hover",
        "browser_drag",
        "browser_evaluate",
        "browser_file_upload",
        "browser_select_option",
        "browser_wait_for",
        "browser_tabs",
        "browser_console_messages",
        "browser_network_requests",
    ];

    let mut registry = MockToolRegistry::new();
    registry.register_server("playwright", playwright_tools.clone());

    // Verify core browser automation tools
    let core_tools = [
        "browser_navigate",
        "browser_click",
        "browser_type",
        "browser_snapshot",
    ];
    for tool in core_tools {
        assert!(
            registry.has_tool("playwright", tool),
            "Playwright should have {} tool",
            tool
        );
    }

    // Verify advanced tools
    let advanced_tools = [
        "browser_evaluate",
        "browser_file_upload",
        "browser_wait_for",
    ];
    for tool in advanced_tools {
        assert!(
            registry.has_tool("playwright", tool),
            "Playwright should have {} tool",
            tool
        );
    }
}

// ============================================================================
// Tool Failure Handling Tests
// ============================================================================

/// Represents possible tool execution outcomes
#[derive(Debug, Clone)]
enum ToolExecutionResult {
    Success(String),
    Timeout(String),
    NotFound(String),
    InvalidParams(String),
    InternalError(String),
    ServerError(i32, String),
}

impl ToolExecutionResult {
    fn to_response(&self, request_id: RequestId) -> JsonRpcResponse {
        match self {
            ToolExecutionResult::Success(result) => {
                JsonRpcResponse::success(request_id, json!({ "result": result }))
            }
            ToolExecutionResult::Timeout(msg) => JsonRpcResponse::error(
                request_id,
                JsonRpcError::new(JsonRpcErrorCode::ServerError(-32001), msg.clone()),
            ),
            ToolExecutionResult::NotFound(msg) => {
                JsonRpcResponse::error(request_id, JsonRpcError::method_not_found(msg))
            }
            ToolExecutionResult::InvalidParams(msg) => {
                JsonRpcResponse::error(request_id, JsonRpcError::invalid_params(msg))
            }
            ToolExecutionResult::InternalError(msg) => {
                JsonRpcResponse::error(request_id, JsonRpcError::internal_error(msg))
            }
            ToolExecutionResult::ServerError(code, msg) => JsonRpcResponse::error(
                request_id,
                JsonRpcError::with_data(
                    JsonRpcErrorCode::ServerError(*code),
                    msg.clone(),
                    json!({ "serverError": true }),
                ),
            ),
        }
    }
}

#[test]
fn test_tool_execution_success() {
    let result = ToolExecutionResult::Success("Element clicked successfully".to_string());
    let response = result.to_response(RequestId::Number(1));

    assert!(response.is_success());
    let result = response.get_result().unwrap();
    assert_eq!(result["result"], "Element clicked successfully");
}

#[test]
fn test_tool_execution_timeout() {
    let result =
        ToolExecutionResult::Timeout("Tool execution exceeded 30 second timeout".to_string());
    let response = result.to_response(RequestId::Number(1));

    assert!(response.is_error());
    let error = response.get_error().unwrap();
    assert_eq!(error.code, -32001); // Server error for timeout
    assert!(error.message.contains("timeout"));
}

#[test]
fn test_tool_not_found_error() {
    let result = ToolExecutionResult::NotFound("Tool 'browser_magic' not found".to_string());
    let response = result.to_response(RequestId::Number(1));

    assert!(response.is_error());
    let error = response.get_error().unwrap();
    assert_eq!(error.code, -32601); // Method not found
}

#[test]
fn test_tool_invalid_params_error() {
    let result =
        ToolExecutionResult::InvalidParams("Missing required parameter: selector".to_string());
    let response = result.to_response(RequestId::Number(1));

    assert!(response.is_error());
    let error = response.get_error().unwrap();
    assert_eq!(error.code, -32602); // Invalid params
    assert!(error.message.contains("Missing required parameter"));
}

#[test]
fn test_tool_internal_error() {
    let result =
        ToolExecutionResult::InternalError("Browser process crashed unexpectedly".to_string());
    let response = result.to_response(RequestId::Number(1));

    assert!(response.is_error());
    let error = response.get_error().unwrap();
    assert_eq!(error.code, -32603); // Internal error
}

#[test]
fn test_tool_server_error_with_data() {
    let result = ToolExecutionResult::ServerError(-32050, "Rate limit exceeded".to_string());
    let response = result.to_response(RequestId::Number(1));

    assert!(response.is_error());
    let error = response.get_error().unwrap();
    assert_eq!(error.code, -32050);
    assert!(error.data.is_some()); // Should have additional data
}

// ============================================================================
// MCP Config Integration with Agent Session Tests
// ============================================================================

/// Mock agent session that can use MCP tools
struct MockAgentSession {
    session_id: String,
    available_tools: Vec<String>,
    mcp_config: Option<McpConfig>,
}

impl MockAgentSession {
    fn new(session_id: &str) -> Self {
        let id = session_id.to_string();
        MockAgentSession {
            session_id: id,
            available_tools: Vec::new(),
            mcp_config: None,
        }
    }

    fn id(&self) -> &str {
        &self.session_id
    }

    fn inject_mcp_config(&mut self, config: McpConfig) {
        // Extract tool names from configured servers
        // In a real implementation, this would query the MCP servers
        for (name, _server) in config.enabled_servers() {
            self.available_tools.push(format!("mcp_{}_tools", name));
        }
        self.mcp_config = Some(config);
    }

    fn has_tools_available(&self) -> bool {
        !self.available_tools.is_empty()
    }

    fn get_available_tools(&self) -> &[String] {
        &self.available_tools
    }
}

#[test]
fn test_mcp_tools_injection_into_agent_session() {
    let config_content = r#"
[mcp.servers.playwright]
type = "playwright"
command = "npx"
args = ["-y", "@playwright/mcp@latest"]

[mcp.servers.filesystem]
type = "filesystem"
command = "mcp-server-filesystem"
"#;

    let config = McpConfig::from_str(config_content).unwrap();

    let mut session = MockAgentSession::new("test-session-001");
    assert_eq!(session.id(), "test-session-001");
    assert!(!session.has_tools_available());

    // Inject MCP config
    session.inject_mcp_config(config);

    // Verify tools are now available
    assert!(session.has_tools_available());
    let tools = session.get_available_tools();
    assert_eq!(tools.len(), 2);
}

#[test]
fn test_mcp_tools_injection_disabled_servers() {
    let config_content = r#"
[mcp.servers.playwright]
type = "playwright"
enabled = true

[mcp.servers.disabled]
type = "test"
enabled = false
"#;

    let config = McpConfig::from_str(config_content).unwrap();

    let mut session = MockAgentSession::new("test-session-002");
    session.inject_mcp_config(config);

    // Only enabled servers should inject tools
    let tools = session.get_available_tools();
    assert_eq!(tools.len(), 1);
    assert!(tools[0].contains("playwright"));
}

#[test]
fn test_mcp_tools_injection_empty_config() {
    let config_content = r#"
[mcp.servers]
"#;

    let config = McpConfig::from_str(config_content).unwrap();

    let mut session = MockAgentSession::new("test-session-003");
    session.inject_mcp_config(config);

    // No tools should be available with empty config
    assert!(!session.has_tools_available());
}

// ============================================================================
// MCP Tool Call Request Building Tests
// ============================================================================

#[test]
fn test_build_browser_navigate_request() {
    let request = JsonRpcRequest::with_params(
        RequestId::String("nav-001".into()),
        "tools/call",
        json!({
            "name": "browser_navigate",
            "arguments": {
                "url": "https://example.com"
            }
        }),
    );

    assert_eq!(request.method, "tools/call");

    let params = request.params.unwrap();
    assert_eq!(params["name"], "browser_navigate");
    assert_eq!(params["arguments"]["url"], "https://example.com");
}

#[test]
fn test_build_browser_click_request() {
    let request = JsonRpcRequest::with_params(
        RequestId::String("click-001".into()),
        "tools/call",
        json!({
            "name": "browser_click",
            "arguments": {
                "element": "Submit button",
                "ref": "button[type='submit']"
            }
        }),
    );

    let params = request.params.unwrap();
    assert_eq!(params["name"], "browser_click");
    assert_eq!(params["arguments"]["element"], "Submit button");
}

#[test]
fn test_build_browser_type_request() {
    let request = JsonRpcRequest::with_params(
        RequestId::String("type-001".into()),
        "tools/call",
        json!({
            "name": "browser_type",
            "arguments": {
                "element": "Username field",
                "ref": "input[name='username']",
                "text": "testuser"
            }
        }),
    );

    let params = request.params.unwrap();
    assert_eq!(params["name"], "browser_type");
    assert_eq!(params["arguments"]["text"], "testuser");
}

#[test]
fn test_build_browser_snapshot_request() {
    // Snapshot doesn't require arguments
    let request = JsonRpcRequest::with_params(
        RequestId::String("snapshot-001".into()),
        "tools/call",
        json!({
            "name": "browser_snapshot",
            "arguments": {}
        }),
    );

    let params = request.params.unwrap();
    assert_eq!(params["name"], "browser_snapshot");
}

// ============================================================================
// MCP Response Parsing Tests
// ============================================================================

#[test]
fn test_parse_browser_snapshot_response() {
    let response_json = r#"{
        "jsonrpc": "2.0",
        "id": "snapshot-001",
        "result": {
            "content": [{
                "type": "text",
                "text": "- button \"Submit\"\n- textbox \"Username\"\n- textbox \"Password\""
            }]
        }
    }"#;

    let message = JsonRpcMessage::from_json(response_json).unwrap();

    if let JsonRpcMessage::Response(response) = message {
        assert!(response.is_success());
        let result = response.get_result().unwrap();
        let content = result["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "text");
        assert!(content[0]["text"].as_str().unwrap().contains("Submit"));
    } else {
        panic!("Expected response message");
    }
}

#[test]
fn test_parse_browser_click_success_response() {
    let response_json = r#"{
        "jsonrpc": "2.0",
        "id": "click-001",
        "result": {
            "content": [{
                "type": "text",
                "text": "Clicked element: Submit button"
            }]
        }
    }"#;

    let response = JsonRpcResponse::from_json(response_json).unwrap();
    assert!(response.is_success());
}

#[test]
fn test_parse_browser_error_response() {
    let response_json = r#"{
        "jsonrpc": "2.0",
        "id": "click-002",
        "error": {
            "code": -32602,
            "message": "Element not found: button[type='submit']",
            "data": {
                "selector": "button[type='submit']",
                "timeout": 30000
            }
        }
    }"#;

    let response = JsonRpcResponse::from_json(response_json).unwrap();
    assert!(response.is_error());

    let error = response.get_error().unwrap();
    assert_eq!(error.code, -32602);
    assert!(error.message.contains("Element not found"));
    assert!(error.data.is_some());

    let data = error.data.as_ref().unwrap();
    assert_eq!(data["selector"], "button[type='submit']");
}

// ============================================================================
// MCP Config File Loading Integration Tests
// ============================================================================

#[test]
fn test_load_mcp_config_from_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("mcp-config.toml");

    let config_content = r#"
[mcp.servers.playwright]
type = "playwright"
command = "npx"
args = ["-y", "@playwright/mcp@latest"]
timeout = 60
"#;

    fs::write(&config_path, config_content).unwrap();

    let loaded = LoadedMcpConfig::from_file(&config_path).unwrap();
    assert_eq!(loaded.config.mcp.servers.len(), 1);
    assert!(loaded.config.get_server("playwright").is_some());
}

#[test]
fn test_mcp_config_file_not_found() {
    let result = LoadedMcpConfig::from_file("/nonexistent/path/mcp-config.toml");
    assert!(result.is_err());
}

#[test]
fn test_mcp_config_validation_catches_errors() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid-config.toml");

    // Invalid: zero timeout
    let config_content = r#"
[mcp.servers.bad]
type = "test"
timeout = 0
"#;

    fs::write(&config_path, config_content).unwrap();

    let result = LoadedMcpConfig::from_file(&config_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("zero timeout"));
}

// ============================================================================
// Edge Cases and Error Scenarios
// ============================================================================

#[test]
fn test_empty_mcp_config_is_valid() {
    let config = McpConfig::default();
    assert!(config.mcp.servers.is_empty());
    assert!(config.validate().is_ok());
}

#[test]
fn test_mcp_config_server_with_no_command() {
    // Command is optional - some servers might be configured differently
    let config_content = r#"
[mcp.servers.server_no_command]
type = "builtin"
timeout = 30
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("server_no_command").unwrap();
    assert_eq!(server.command, None);
}

#[test]
fn test_mcp_request_id_types() {
    // String ID
    let request = JsonRpcRequest::new(RequestId::String("abc-123".into()), "test");
    assert_eq!(request.id, RequestId::String("abc-123".to_string()));

    // Number ID
    let request = JsonRpcRequest::new(RequestId::Number(42), "test");
    assert_eq!(request.id, RequestId::Number(42));

    // Null ID (allowed but rare)
    let request = JsonRpcRequest::new(RequestId::Null, "test");
    assert_eq!(request.id, RequestId::Null);
}

#[test]
fn test_mcp_response_id_matches_request() {
    let request_id = RequestId::String("req-001".into());
    let response = JsonRpcResponse::success(request_id.clone(), json!({ "ok": true }));

    assert_eq!(response.id, request_id);
}

#[test]
fn test_mcp_error_with_additional_data() {
    let error = JsonRpcError::with_data(
        JsonRpcErrorCode::InvalidParams,
        "Invalid parameter value".to_string(),
        json!({
            "parameter": "url",
            "expected": "valid URL string",
            "received": "not-a-url"
        }),
    );

    assert_eq!(error.code, -32602);
    assert!(error.data.is_some());

    let data = error.data.unwrap();
    assert_eq!(data["parameter"], "url");
    assert_eq!(data["received"], "not-a-url");
}

// ============================================================================
// Tool Chain Execution Simulation Tests
// ============================================================================

/// Simulates a chain of tool calls for a typical browser automation workflow
#[test]
fn test_tool_chain_browser_login_workflow() {
    // Simulate a login workflow: navigate -> type username -> type password -> click submit

    // Step 1: Navigate to login page
    let navigate_request = JsonRpcRequest::with_params(
        RequestId::Number(1),
        "tools/call",
        json!({
            "name": "browser_navigate",
            "arguments": { "url": "https://example.com/login" }
        }),
    );
    assert_eq!(navigate_request.method, "tools/call");

    // Step 2: Type username
    let type_request = JsonRpcRequest::with_params(
        RequestId::Number(2),
        "tools/call",
        json!({
            "name": "browser_type",
            "arguments": {
                "element": "Username",
                "ref": "#username",
                "text": "testuser"
            }
        }),
    );
    let params = type_request.params.unwrap();
    assert_eq!(params["name"], "browser_type");

    // Step 3: Click submit
    let click_request = JsonRpcRequest::with_params(
        RequestId::Number(3),
        "tools/call",
        json!({
            "name": "browser_click",
            "arguments": {
                "element": "Submit button",
                "ref": "button[type='submit']"
            }
        }),
    );
    let params = click_request.params.unwrap();
    assert_eq!(params["name"], "browser_click");
}

#[test]
fn test_tool_chain_with_error_recovery() {
    // Simulate error recovery: click fails -> retry with different selector

    // First attempt fails
    let error_response = JsonRpcResponse::error(
        RequestId::Number(1),
        JsonRpcError::new(
            JsonRpcErrorCode::ServerError(-32001),
            "Element not found: #old-button".to_string(),
        ),
    );
    assert!(error_response.is_error());

    // Retry with new selector
    let _retry_request = JsonRpcRequest::with_params(
        RequestId::Number(2),
        "tools/call",
        json!({
            "name": "browser_click",
            "arguments": {
                "element": "Submit button",
                "ref": "#new-button"
            }
        }),
    );

    // This time succeeds
    let success_response =
        JsonRpcResponse::success(RequestId::Number(2), json!({ "clicked": true }));
    assert!(success_response.is_success());
}

// ============================================================================
// MCP Protocol Version and Capability Tests
// ============================================================================

#[test]
fn test_mcp_protocol_version() {
    // All MCP messages should use JSON-RPC 2.0
    let request = JsonRpcRequest::new(RequestId::Number(1), "initialize");
    assert_eq!(request.jsonrpc, "2.0");

    let response = JsonRpcResponse::success(RequestId::Number(1), json!({}));
    assert_eq!(response.jsonrpc, "2.0");

    let notification = JsonRpcNotification::new("test");
    assert_eq!(notification.jsonrpc, "2.0");
}

#[test]
fn test_mcp_initialize_handshake() {
    // Simulate MCP initialization handshake
    let init_request = JsonRpcRequest::with_params(
        RequestId::Number(1),
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "clientInfo": {
                "name": "ltmatrix",
                "version": "0.1.0"
            },
            "capabilities": {}
        }),
    );

    assert_eq!(init_request.method, "initialize");
    let params = init_request.params.unwrap();
    assert_eq!(params["protocolVersion"], "2024-11-05");

    // Server response
    let init_response = JsonRpcResponse::success(
        RequestId::Number(1),
        json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {
                "name": "playwright-mcp",
                "version": "1.0.0"
            },
            "capabilities": {
                "tools": {}
            }
        }),
    );

    let result = init_response.get_result().unwrap();
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert!(result.get("capabilities").is_some());
}
