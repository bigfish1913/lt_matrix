// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Integration Tests with Mock MCP Server
//!
//! These tests verify end-to-end flows using a mock MCP server implementation:
//! - Client connection lifecycle
//! - Tool discovery and execution
//! - Resource access and reading
//! - Prompt retrieval
//! - Error scenarios and recovery
//!
//! # Test Categories
//!
//! 1. **Connection Tests**: Client initialization, handshake, and disconnection
//! 2. **Tool Tests**: Listing tools, calling tools, handling tool errors
//! 3. **Resource Tests**: Listing resources, reading resource contents
//! 4. **Prompt Tests**: Listing prompts, getting prompt templates
//! 5. **Error Tests**: Invalid methods, invalid parameters, server errors
//! 6. **Concurrency Tests**: Multiple concurrent requests
//! 7. **Timeout Tests**: Request timeout handling

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use ltmatrix::mcp::protocol::methods::{Resource, ResourceContents, Tool, MCP_PROTOCOL_VERSION};
use ltmatrix::mcp::{
    JsonRpcError, JsonRpcErrorCode, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
    RequestId, Transport,
};

// Import mock server types - using the module from tests/mock_mcp_server.rs
mod mock_mcp_server;
use mock_mcp_server::{MockMcpServer, MockServerConfig, MockTransport, RequestLog};

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a default mock server for testing
fn create_test_server() -> Arc<MockMcpServer> {
    Arc::new(MockMcpServer::default_server())
}

/// Create a mock server with custom configuration
fn create_custom_server(config: MockServerConfig) -> Arc<MockMcpServer> {
    Arc::new(MockMcpServer::new(config))
}

/// Create a started transport for testing
async fn create_started_transport(server: Arc<MockMcpServer>) -> MockTransport {
    let mut transport = MockTransport::new(server);
    transport.start().await.expect("Failed to start transport");
    transport
}

/// Helper to receive and assert success response
async fn receive_success_response(transport: &MockTransport) -> JsonRpcResponse {
    let message = transport
        .receive()
        .await
        .expect("Failed to receive message");
    let response = message.as_response().expect("Expected response message");
    assert!(response.is_success(), "Expected successful response");
    response.clone()
}

/// Helper to receive and assert error response
async fn receive_error_response(transport: &MockTransport) -> (JsonRpcResponse, JsonRpcError) {
    let message = transport
        .receive()
        .await
        .expect("Failed to receive message");
    let response = message.as_response().expect("Expected response message");
    let error = response.error.clone().expect("Expected error in response");
    (response.clone(), error)
}

// ============================================================================
// Connection Lifecycle Tests
// ============================================================================

mod connection_lifecycle_tests {
    use super::*;

    #[tokio::test]
    async fn test_transport_start_and_close() {
        let server = create_test_server();
        let mut transport = MockTransport::new(server);

        // Initially not connected
        assert!(!transport.is_connected());

        // Start transport
        transport.start().await.unwrap();
        assert!(transport.is_connected());

        // Close transport
        transport.close().await.unwrap();
        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn test_server_started_state() {
        let server = create_test_server();

        // Initially not started
        assert!(!server.is_started().await);

        // Start via transport
        let mut transport = MockTransport::new(server.clone());
        transport.start().await.unwrap();

        // Now started
        assert!(server.is_started().await);

        // Close transport
        transport.close().await.unwrap();

        // No longer started
        assert!(!server.is_started().await);
    }

    #[tokio::test]
    async fn test_multiple_start_calls() {
        let server = create_test_server();
        let mut transport = MockTransport::new(server);

        // First start
        transport.start().await.unwrap();
        assert!(transport.is_connected());

        // Second start should still work (idempotent in mock)
        transport.start().await.unwrap();
        assert!(transport.is_connected());
    }

    #[tokio::test]
    async fn test_close_without_start() {
        let server = create_test_server();
        let mut transport = MockTransport::new(server);

        // Close without start should work
        transport.close().await.unwrap();
        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn test_transport_stats_tracking() {
        let server = create_test_server();
        let mut transport = create_started_transport(server).await;

        let initial_stats = transport.stats();
        assert_eq!(initial_stats.messages_sent, 0);
        assert!(initial_stats.connected_since.is_some());

        // Send a request
        let request = JsonRpcRequest::new(RequestId::Number(1), "ping");
        transport.send_request(request).await.unwrap();

        let stats = transport.stats();
        assert!(stats.messages_sent > 0);

        transport.close().await.unwrap();
    }
}

// ============================================================================
// Initialize Handshake Tests
// ============================================================================

mod initialize_tests {
    use super::*;

    #[tokio::test]
    async fn test_initialize_handshake() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        // Send initialize request
        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "initialize",
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }),
        );

        transport.send_request(request).await.unwrap();

        // Receive response
        let response = receive_success_response(&transport).await;

        let result = response.result.as_ref().unwrap();
        assert_eq!(result["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert!(result.get("capabilities").is_some());
        assert!(result.get("serverInfo").is_some());
    }

    #[tokio::test]
    async fn test_initialize_server_info() {
        let config = MockServerConfig::new("custom-server", "2.0.0");
        let server = create_custom_server(config);
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "initialize",
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "1.0" }
            }),
        );

        transport.send_request(request).await.unwrap();
        let response = receive_success_response(&transport).await;

        let server_info = &response.result.as_ref().unwrap()["serverInfo"];
        assert_eq!(server_info["name"], "custom-server");
        assert_eq!(server_info["version"], "2.0.0");
    }

    #[tokio::test]
    async fn test_initialize_missing_params() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        // Send initialize without params
        let request = JsonRpcRequest::new(RequestId::Number(1), "initialize");
        transport.send_request(request).await.unwrap();

        let (response, error) = receive_error_response(&transport).await;
        assert_eq!(error.code, JsonRpcErrorCode::InvalidParams.as_i32());
    }

    #[tokio::test]
    async fn test_initialize_capabilities_negotiation() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "initialize",
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {
                    "roots": { "listChanged": true },
                    "sampling": {}
                },
                "clientInfo": { "name": "test", "version": "1.0" }
            }),
        );

        transport.send_request(request).await.unwrap();
        let response = receive_success_response(&transport).await;

        // Server should return its capabilities
        let capabilities = &response.result.as_ref().unwrap()["capabilities"];
        assert!(capabilities.get("tools").is_some());
        assert!(capabilities.get("resources").is_some());
        assert!(capabilities.get("prompts").is_some());
    }

    #[tokio::test]
    async fn test_request_logged_during_initialize() {
        let server = create_test_server();
        let request_log = server.request_log();
        let transport = create_started_transport(server.clone()).await;

        let request = JsonRpcRequest::with_params(
            RequestId::Number(42),
            "initialize",
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "1.0" }
            }),
        );

        transport.send_request(request).await.unwrap();
        let _ = transport.receive().await.unwrap();

        // Check request was logged
        let log = request_log.lock().await;
        assert_eq!(log.requests.len(), 1);
        assert_eq!(log.requests[0].method, "initialize");
        assert_eq!(log.requests[0].id, RequestId::Number(42));
    }
}

// ============================================================================
// Tool Discovery and Execution Tests
// ============================================================================

mod tool_tests {
    use super::*;

    #[tokio::test]
    async fn test_list_tools() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::new(RequestId::Number(1), "tools/list");
        transport.send_request(request).await.unwrap();

        let response = receive_success_response(&transport).await;
        let result = response.result.as_ref().unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();

        // Should have default tools
        assert!(!tools.is_empty());

        // Check tool structure
        for tool in tools {
            assert!(tool.get("name").is_some());
            assert!(tool.get("description").is_some());
            assert!(tool.get("inputSchema").is_some());
        }
    }

    #[tokio::test]
    async fn test_list_tools_with_custom_tools() {
        let config = MockServerConfig::default().with_tool(Tool::new(
            "custom_tool",
            "A custom test tool",
            json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                }
            }),
        ));

        let server = create_custom_server(config);
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::new(RequestId::Number(1), "tools/list");
        transport.send_request(request).await.unwrap();

        let response = receive_success_response(&transport).await;
        let result = response.result.as_ref().unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();

        // Find custom tool
        let custom_tool = tools
            .iter()
            .find(|t| t["name"] == "custom_tool")
            .expect("Custom tool not found");
        assert_eq!(custom_tool["description"], "A custom test tool");
    }

    #[tokio::test]
    async fn test_call_tool_echo() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "tools/call",
            json!({
                "name": "echo",
                "arguments": { "text": "Hello, World!" }
            }),
        );

        transport.send_request(request).await.unwrap();
        let response = receive_success_response(&transport).await;

        let result = response.result.as_ref().unwrap();
        let content = result.get("content").unwrap().as_array().unwrap();
        assert!(!content.is_empty());
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "Hello, World!");
    }

    #[tokio::test]
    async fn test_call_tool_test_tool() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "tools/call",
            json!({
                "name": "test_tool",
                "arguments": { "message": "Testing" }
            }),
        );

        transport.send_request(request).await.unwrap();
        let response = receive_success_response(&transport).await;

        let result = response.result.as_ref().unwrap();
        let content = result.get("content").unwrap().as_array().unwrap();
        assert!(content[0]["text"].as_str().unwrap().contains("Testing"));
    }

    #[tokio::test]
    async fn test_call_tool_missing_name() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "tools/call",
            json!({ "arguments": {} }),
        );

        transport.send_request(request).await.unwrap();
        let (_, error) = receive_error_response(&transport).await;
        assert_eq!(error.code, JsonRpcErrorCode::InvalidParams.as_i32());
    }

    #[tokio::test]
    async fn test_call_unknown_tool() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "tools/call",
            json!({ "name": "nonexistent_tool" }),
        );

        transport.send_request(request).await.unwrap();
        let (_, error) = receive_error_response(&transport).await;
        assert_eq!(error.code, JsonRpcErrorCode::MethodNotFound.as_i32());
    }

    #[tokio::test]
    async fn test_call_tool_with_missing_required_arg() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        // Call echo without required "text" argument
        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "tools/call",
            json!({ "name": "echo", "arguments": {} }),
        );

        transport.send_request(request).await.unwrap();
        let (_, error) = receive_error_response(&transport).await;
        assert_eq!(error.code, JsonRpcErrorCode::InvalidParams.as_i32());
    }
}

// ============================================================================
// Resource Tests
// ============================================================================

mod resource_tests {
    use super::*;

    #[tokio::test]
    async fn test_list_resources() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::new(RequestId::Number(1), "resources/list");
        transport.send_request(request).await.unwrap();

        let response = receive_success_response(&transport).await;
        let result = response.result.as_ref().unwrap();
        let resources = result.get("resources").unwrap().as_array().unwrap();

        // Should have default resources
        assert!(!resources.is_empty());

        // Check resource structure
        for resource in resources {
            assert!(resource.get("uri").is_some());
            assert!(resource.get("name").is_some());
        }
    }

    #[tokio::test]
    async fn test_list_resources_with_custom() {
        let config = MockServerConfig::default().with_resource(
            Resource::new("file:///custom/data.json", "data.json"),
            ResourceContents::text("file:///custom/data.json", r#"{"key": "value"}"#),
        );

        let server = create_custom_server(config);
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::new(RequestId::Number(1), "resources/list");
        transport.send_request(request).await.unwrap();

        let response = receive_success_response(&transport).await;
        let result = response.result.as_ref().unwrap();
        let resources = result.get("resources").unwrap().as_array().unwrap();

        let custom_resource = resources
            .iter()
            .find(|r| r["uri"] == "file:///custom/data.json")
            .expect("Custom resource not found");
        assert_eq!(custom_resource["name"], "data.json");
    }

    #[tokio::test]
    async fn test_read_resource() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "resources/read",
            json!({ "uri": "file:///test.txt" }),
        );

        transport.send_request(request).await.unwrap();
        let response = receive_success_response(&transport).await;

        let result = response.result.as_ref().unwrap();
        let contents = result.get("contents").unwrap().as_array().unwrap();
        assert!(!contents.is_empty());
        assert_eq!(contents[0]["uri"], "file:///test.txt");
        assert_eq!(contents[0]["text"], "Hello, World!");
    }

    #[tokio::test]
    async fn test_read_resource_json() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "resources/read",
            json!({ "uri": "file:///config.json" }),
        );

        transport.send_request(request).await.unwrap();
        let response = receive_success_response(&transport).await;

        let result = response.result.as_ref().unwrap();
        let contents = result.get("contents").unwrap().as_array().unwrap();
        assert_eq!(contents[0]["text"], r#"{"version":"1.0"}"#);
    }

    #[tokio::test]
    async fn test_read_nonexistent_resource() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "resources/read",
            json!({ "uri": "file:///nonexistent.txt" }),
        );

        transport.send_request(request).await.unwrap();
        let (_, error) = receive_error_response(&transport).await;
        assert_eq!(error.code, JsonRpcErrorCode::InvalidParams.as_i32());
    }

    #[tokio::test]
    async fn test_read_resource_missing_uri() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request =
            JsonRpcRequest::with_params(RequestId::Number(1), "resources/read", json!({}));

        transport.send_request(request).await.unwrap();
        let (_, error) = receive_error_response(&transport).await;
        assert_eq!(error.code, JsonRpcErrorCode::InvalidParams.as_i32());
    }
}

// ============================================================================
// Prompt Tests
// ============================================================================

mod prompt_tests {
    use super::*;

    #[tokio::test]
    async fn test_list_prompts() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::new(RequestId::Number(1), "prompts/list");
        transport.send_request(request).await.unwrap();

        let response = receive_success_response(&transport).await;
        let result = response.result.as_ref().unwrap();
        let prompts = result.get("prompts").unwrap().as_array().unwrap();

        // Should have default prompts
        assert!(!prompts.is_empty());

        // Check prompt structure
        for prompt in prompts {
            assert!(prompt.get("name").is_some());
        }
    }

    #[tokio::test]
    async fn test_get_prompt() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "prompts/get",
            json!({ "name": "greeting" }),
        );

        transport.send_request(request).await.unwrap();
        let response = receive_success_response(&transport).await;

        let result = response.result.as_ref().unwrap();
        let messages = result.get("messages").unwrap().as_array().unwrap();
        assert!(!messages.is_empty());

        // Check message structure
        for msg in messages {
            assert!(msg.get("role").is_some());
            assert!(msg.get("content").is_some());
        }
    }

    #[tokio::test]
    async fn test_get_prompt_with_arguments() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "prompts/get",
            json!({
                "name": "code_review",
                "arguments": {
                    "language": "rust",
                    "code": "fn main() {}"
                }
            }),
        );

        transport.send_request(request).await.unwrap();
        let response = receive_success_response(&transport).await;

        let result = response.result.as_ref().unwrap();
        let messages = result.get("messages").unwrap().as_array().unwrap();
        assert!(!messages.is_empty());
    }

    #[tokio::test]
    async fn test_get_nonexistent_prompt() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "prompts/get",
            json!({ "name": "nonexistent_prompt" }),
        );

        transport.send_request(request).await.unwrap();
        let (_, error) = receive_error_response(&transport).await;
        assert_eq!(error.code, JsonRpcErrorCode::InvalidParams.as_i32());
    }

    #[tokio::test]
    async fn test_get_prompt_missing_name() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(RequestId::Number(1), "prompts/get", json!({}));

        transport.send_request(request).await.unwrap();
        let (_, error) = receive_error_response(&transport).await;
        assert_eq!(error.code, JsonRpcErrorCode::InvalidParams.as_i32());
    }
}

// ============================================================================
// Error Scenario Tests
// ============================================================================

mod error_scenario_tests {
    use super::*;

    #[tokio::test]
    async fn test_unknown_method() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::new(RequestId::Number(1), "unknown/method");
        transport.send_request(request).await.unwrap();
        let (_, error) = receive_error_response(&transport).await;
        assert_eq!(error.code, JsonRpcErrorCode::MethodNotFound.as_i32());
    }

    #[tokio::test]
    async fn test_server_configured_error() {
        let config = MockServerConfig::default().with_error_method("tools/list");

        let server = create_custom_server(config);
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::new(RequestId::Number(1), "tools/list");
        transport.send_request(request).await.unwrap();
        let (_, error) = receive_error_response(&transport).await;
        assert_eq!(error.code, JsonRpcErrorCode::InternalError.as_i32());
    }
}

// ============================================================================
// Configuration Tests
// ============================================================================

mod configuration_tests {
    use super::*;

    #[test]
    fn test_mock_server_config_default() {
        let config = MockServerConfig::default();

        assert_eq!(config.server_name, "mock-mcp-server");
        assert_eq!(config.server_version, "1.0.0");
        assert_eq!(config.protocol_version, MCP_PROTOCOL_VERSION);
        assert!(config.response_delay.is_zero());
        assert!(config.error_methods.is_empty());
    }

    #[test]
    fn test_mock_server_config_builder() {
        let config = MockServerConfig::new("test-server", "2.0.0")
            .with_response_delay(Duration::from_millis(100))
            .with_error_method("test/error");

        assert_eq!(config.server_name, "test-server");
        assert_eq!(config.server_version, "2.0.0");
        assert_eq!(config.response_delay, Duration::from_millis(100));
        assert!(config.error_methods.contains(&"test/error".to_string()));
    }

    #[tokio::test]
    async fn test_delayed_response() {
        let config = MockServerConfig::default().with_response_delay(Duration::from_millis(50));

        let server = create_custom_server(config);
        let transport = create_started_transport(server).await;

        let start = std::time::Instant::now();
        let request = JsonRpcRequest::new(RequestId::Number(1), "ping");
        transport.send_request(request).await.unwrap();

        let _ = transport.receive().await.unwrap();
        let elapsed = start.elapsed();

        // Should have at least the configured delay
        assert!(elapsed >= Duration::from_millis(50));
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_params() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        // Ping with empty params should work
        let request = JsonRpcRequest::with_params(RequestId::Number(1), "ping", json!({}));

        transport.send_request(request).await.unwrap();
        let _ = receive_success_response(&transport).await;
    }

    #[tokio::test]
    async fn test_large_request_id() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let large_id = i64::MAX;
        let request = JsonRpcRequest::new(RequestId::Number(large_id), "ping");
        transport.send_request(request).await.unwrap();
        let response = receive_success_response(&transport).await;

        assert_eq!(response.id, RequestId::Number(large_id));
    }

    #[tokio::test]
    async fn test_string_request_id() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let string_id = RequestId::String("custom-id-123".to_string());
        let request = JsonRpcRequest::new(string_id.clone(), "ping");
        transport.send_request(request).await.unwrap();
        let response = receive_success_response(&transport).await;

        assert_eq!(response.id, string_id);
    }

    #[tokio::test]
    async fn test_special_characters_in_params() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "tools/call",
            json!({
                "name": "echo",
                "arguments": {
                    "text": "Special: \n\t\r\"quotes\" 'apostrophes' unicode: 你好 🎉"
                }
            }),
        );

        transport.send_request(request).await.unwrap();
        let _ = receive_success_response(&transport).await;
    }

    #[tokio::test]
    async fn test_null_params() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::with_params(RequestId::Number(1), "ping", json!(null));

        transport.send_request(request).await.unwrap();

        // Should handle null params gracefully
        let message = transport
            .receive()
            .await
            .expect("Failed to receive message");
        assert!(message.is_response());
    }
}

// ============================================================================
// Transport Stats Tests
// ============================================================================

mod transport_stats_tests {
    use super::*;

    #[tokio::test]
    async fn test_stats_initial_state() {
        let server = create_test_server();
        let transport = MockTransport::new(server);

        let stats = transport.stats();
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert!(stats.connected_since.is_none());
    }

    #[tokio::test]
    async fn test_stats_after_connection() {
        let server = create_test_server();
        let mut transport = MockTransport::new(server);

        transport.start().await.unwrap();

        let stats = transport.stats();
        assert!(stats.connected_since.is_some());
    }

    #[tokio::test]
    async fn test_stats_after_messages() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        // Send and receive a message
        let request = JsonRpcRequest::new(RequestId::Number(1), "ping");
        transport.send_request(request).await.unwrap();
        let _ = transport.receive().await.unwrap();

        let stats = transport.stats();
        assert!(stats.messages_sent > 0);
        assert!(stats.messages_received > 0);
        assert!(stats.last_sent.is_some());
        assert!(stats.last_received.is_some());
    }

    #[tokio::test]
    async fn test_stats_after_disconnect() {
        let server = create_test_server();
        let mut transport = create_started_transport(server).await;

        transport.close().await.unwrap();

        let stats = transport.stats();
        assert!(stats.connected_since.is_none());
    }
}

// ============================================================================
// Full Integration Flow Tests
// ============================================================================

mod full_integration_tests {
    use super::*;

    /// Test a complete client session flow
    #[tokio::test]
    async fn test_full_client_session() {
        let server = create_test_server();
        let mut transport = create_started_transport(server).await;

        // 1. Initialize
        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "initialize",
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "test-client", "version": "1.0" }
            }),
        );
        transport.send_request(request).await.unwrap();
        let _ = receive_success_response(&transport).await;

        // 2. List tools
        let request = JsonRpcRequest::new(RequestId::Number(2), "tools/list");
        transport.send_request(request).await.unwrap();
        let _ = receive_success_response(&transport).await;

        // 3. Call a tool
        let request = JsonRpcRequest::with_params(
            RequestId::Number(3),
            "tools/call",
            json!({ "name": "echo", "arguments": { "text": "test" } }),
        );
        transport.send_request(request).await.unwrap();
        let _ = receive_success_response(&transport).await;

        // 4. List resources
        let request = JsonRpcRequest::new(RequestId::Number(4), "resources/list");
        transport.send_request(request).await.unwrap();
        let _ = receive_success_response(&transport).await;

        // 5. Read a resource
        let request = JsonRpcRequest::with_params(
            RequestId::Number(5),
            "resources/read",
            json!({ "uri": "file:///test.txt" }),
        );
        transport.send_request(request).await.unwrap();
        let _ = receive_success_response(&transport).await;

        // 6. Disconnect
        transport.close().await.unwrap();
        assert!(!transport.is_connected());
    }

    /// Test error recovery during session
    #[tokio::test]
    async fn test_session_error_recovery() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        // Initialize
        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "initialize",
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "1.0" }
            }),
        );
        transport.send_request(request).await.unwrap();
        let _ = transport.receive().await.unwrap();

        // Send invalid request
        let request = JsonRpcRequest::new(RequestId::Number(2), "invalid/method");
        transport.send_request(request).await.unwrap();
        let (response, _error) = receive_error_response(&transport).await;
        assert!(response.error.is_some());

        // Session should still work after error
        let request = JsonRpcRequest::new(RequestId::Number(3), "ping");
        transport.send_request(request).await.unwrap();
        let _ = receive_success_response(&transport).await;
    }

    /// Test multiple sequential requests
    #[tokio::test]
    async fn test_multiple_sequential_requests() {
        let server = create_test_server();
        let request_log = server.request_log();
        let transport = create_started_transport(server).await;

        for i in 1..=10 {
            let request = JsonRpcRequest::new(RequestId::Number(i), "ping");
            transport.send_request(request).await.unwrap();
            let _ = receive_success_response(&transport).await;
        }

        // Check request log
        let log = request_log.lock().await;
        assert_eq!(log.requests.len(), 10);
        assert_eq!(log.by_method("ping").len(), 10);
    }

    /// Test notification handling
    #[tokio::test]
    async fn test_notification_handling() {
        let server = create_test_server();
        let request_log = server.request_log();
        let transport = create_started_transport(server).await;

        // Send notification (no response expected)
        let notification = JsonRpcNotification::new("notifications/initialized");
        let result = transport.send_notification(notification).await;
        assert!(result.is_ok());

        // Verify notification was logged
        let log = request_log.lock().await;
        assert!(log
            .requests
            .iter()
            .any(|r| r.method == "notifications/initialized"));
    }
}

// ============================================================================
// Ping Tests
// ============================================================================

mod ping_tests {
    use super::*;

    #[tokio::test]
    async fn test_ping_basic() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::new(RequestId::Number(1), "ping");
        transport.send_request(request).await.unwrap();
        let _ = receive_success_response(&transport).await;
    }

    #[tokio::test]
    async fn test_ping_with_params_ignored() {
        let server = create_test_server();
        let transport = create_started_transport(server).await;

        // Ping with params should still work (params ignored)
        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "ping",
            json!({ "unused": "params" }),
        );

        transport.send_request(request).await.unwrap();
        let _ = receive_success_response(&transport).await;
    }
}

// ============================================================================
// Request Log Tests
// ============================================================================

mod request_log_tests {
    use super::*;

    #[test]
    fn test_request_log_creation() {
        let log = RequestLog::new();
        assert!(log.requests.is_empty());
    }

    #[tokio::test]
    async fn test_request_log_recording() {
        let server = create_test_server();
        let log = server.request_log();
        let transport = create_started_transport(server).await;

        let request =
            JsonRpcRequest::with_params(RequestId::Number(1), "ping", json!({ "test": "value" }));

        transport.send_request(request).await.unwrap();
        let _ = transport.receive().await.unwrap();

        let log_guard = log.lock().await;
        assert_eq!(log_guard.requests.len(), 1);

        let record = &log_guard.requests[0];
        assert_eq!(record.method, "ping");
        assert!(record.params.is_some());
    }

    #[tokio::test]
    async fn test_request_log_by_method() {
        let server = create_test_server();
        let log = server.request_log();
        let transport = create_started_transport(server).await;

        // Send different method types
        let request = JsonRpcRequest::new(RequestId::Number(1), "ping");
        transport.send_request(request).await.unwrap();
        let _ = transport.receive().await.unwrap();

        let request = JsonRpcRequest::new(RequestId::Number(2), "tools/list");
        transport.send_request(request).await.unwrap();
        let _ = transport.receive().await.unwrap();

        let request = JsonRpcRequest::new(RequestId::Number(3), "ping");
        transport.send_request(request).await.unwrap();
        let _ = transport.receive().await.unwrap();

        let log_guard = log.lock().await;
        assert_eq!(log_guard.by_method("ping").len(), 2);
        assert_eq!(log_guard.by_method("tools/list").len(), 1);
        assert_eq!(log_guard.by_method("nonexistent").len(), 0);
    }

    #[tokio::test]
    async fn test_request_log_clear() {
        let server = create_test_server();
        let log = server.request_log();
        let transport = create_started_transport(server).await;

        let request = JsonRpcRequest::new(RequestId::Number(1), "ping");
        transport.send_request(request).await.unwrap();
        let _ = transport.receive().await.unwrap();

        log.lock().await.clear();

        let log_guard = log.lock().await;
        assert!(log_guard.requests.is_empty());
    }
}
