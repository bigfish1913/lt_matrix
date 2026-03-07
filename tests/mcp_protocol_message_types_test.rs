// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Integration tests for MCP protocol message types
//!
//! Tests for:
//! - Request, Response, and Notification message types
//! - Proper serde serialization/deserialization
//! - Protocol version fields
//! - Message IDs (string, number, null)
//! - Payload structures

use ltmatrix::mcp::{
    JsonRpcError, JsonRpcErrorCode, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse, RequestId,
};
use ltmatrix::mcp::{
    ClientCapabilities, ImplementationInfo, InitializeParams, InitializeResult,
    LoggingSetLevelParams, LogLevel, Prompt, PromptArgument, PromptContent, PromptMessage,
    PromptsGetParams, PromptsGetResult, PromptsListParams, PromptsListResult,
    Resource, ResourceContents, ResourceReadParams, ResourceReadResult, ResourcesCapability,
    ResourcesListParams, ResourcesListResult, Root, RootsCapability, RootsListParams, RootsListResult,
    ServerCapabilities, Tool, ToolCallParams, ToolCallResult, ToolContent, ToolsCapability,
    ToolsListParams, ToolsListResult, MCP_PROTOCOL_VERSION,
};
use serde_json::json;

// ============================================================================
// Request ID Tests
// ============================================================================

mod request_id_tests {
    use super::*;

    #[test]
    fn test_request_id_string_serialization() {
        let id = RequestId::String("unique-request-id".to_string());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"unique-request-id\"");

        // Round-trip
        let parsed: RequestId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn test_request_id_number_serialization() {
        let id = RequestId::Number(42);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "42");

        // Round-trip
        let parsed: RequestId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn test_request_id_null_serialization() {
        let id = RequestId::Null;
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "null");

        // Round-trip
        let parsed: RequestId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn test_request_id_from_conversions() {
        // From &str
        let id: RequestId = "string-id".into();
        assert_eq!(id, RequestId::String("string-id".to_string()));

        // From String
        let id: RequestId = String::from("string-id-2").into();
        assert_eq!(id, RequestId::String("string-id-2".to_string()));

        // From i64
        let id: RequestId = 123i64.into();
        assert_eq!(id, RequestId::Number(123));

        // From i32
        let id: RequestId = 456i32.into();
        assert_eq!(id, RequestId::Number(456));
    }

    #[test]
    fn test_request_id_large_number() {
        let id = RequestId::Number(i64::MAX);
        let json = serde_json::to_string(&id).unwrap();
        let parsed: RequestId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn test_request_id_negative_number() {
        let id = RequestId::Number(-999);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "-999");

        let parsed: RequestId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, id);
    }
}

// ============================================================================
// Request Message Tests
// ============================================================================

mod request_tests {
    use super::*;

    #[test]
    fn test_request_basic_creation() {
        let request = JsonRpcRequest::new(RequestId::Number(1), "initialize");

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, RequestId::Number(1));
        assert_eq!(request.method, "initialize");
        assert!(request.params.is_none());
    }

    #[test]
    fn test_request_with_params() {
        let params = json!({"protocolVersion": "2025-11-25"});
        let request = JsonRpcRequest::with_params(
            RequestId::String("req-1".to_string()),
            "initialize",
            params.clone(),
        );

        assert!(request.params.is_some());
        assert_eq!(request.params.unwrap(), params);
    }

    #[test]
    fn test_request_serialization_format() {
        let request = JsonRpcRequest::new(RequestId::Number(1), "test_method");
        let json = request.to_json().unwrap();

        // Verify JSON-RPC 2.0 format
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"test_method\""));
    }

    #[test]
    fn test_request_serialization_with_params() {
        let params = json!({"key": "value"});
        let request = JsonRpcRequest::with_params(RequestId::Number(1), "method", params);
        let json = request.to_json().unwrap();

        assert!(json.contains("\"params\""));
        assert!(json.contains("\"key\":\"value\""));
    }

    #[test]
    fn test_request_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":"abc123","method":"tools/call","params":{"name":"test"}}"#;
        let request = JsonRpcRequest::from_json(json).unwrap();

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, RequestId::String("abc123".to_string()));
        assert_eq!(request.method, "tools/call");
        assert!(request.params.is_some());
    }

    #[test]
    fn test_request_requires_id_field() {
        // A request without an "id" field should fail to deserialize as JsonRpcRequest
        let json = r#"{"jsonrpc":"2.0","method":"test"}"#;
        let result = JsonRpcRequest::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_requires_method_field() {
        let json = r#"{"jsonrpc":"2.0","id":1}"#;
        let result = JsonRpcRequest::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_requires_jsonrpc_field() {
        let json = r#"{"id":1,"method":"test"}"#;
        let result = JsonRpcRequest::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_roundtrip() {
        let original = JsonRpcRequest::with_params(
            RequestId::String("complex-id".to_string()),
            "complex/method",
            json!({"nested": {"deep": "value"}, "array": [1, 2, 3]}),
        );

        let json = original.to_json().unwrap();
        let parsed = JsonRpcRequest::from_json(&json).unwrap();

        assert_eq!(parsed.jsonrpc, original.jsonrpc);
        assert_eq!(parsed.id, original.id);
        assert_eq!(parsed.method, original.method);
        assert_eq!(parsed.params, original.params);
    }

    #[test]
    fn test_request_set_params() {
        let mut request = JsonRpcRequest::new(RequestId::Number(1), "test");
        assert!(request.params.is_none());

        request.set_params(json!({"arg": "value"}));
        assert!(request.params.is_some());
    }
}

// ============================================================================
// Response Message Tests
// ============================================================================

mod response_tests {
    use super::*;

    #[test]
    fn test_response_success_creation() {
        let result = json!({"status": "ok"});
        let response = JsonRpcResponse::success(RequestId::Number(1), result.clone());

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, RequestId::Number(1));
        assert!(response.is_success());
        assert!(!response.is_error());
        assert!(response.get_result().is_some());
        assert!(response.get_error().is_none());
        assert_eq!(response.get_result().unwrap(), &result);
    }

    #[test]
    fn test_response_error_creation() {
        let error = JsonRpcError::method_not_found("unknown_method");
        let response = JsonRpcResponse::error(RequestId::Number(1), error.clone());

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, RequestId::Number(1));
        assert!(!response.is_success());
        assert!(response.is_error());
        assert!(response.get_result().is_none());
        assert!(response.get_error().is_some());
    }

    #[test]
    fn test_response_serialization_success() {
        let response = JsonRpcResponse::success(
            RequestId::String("req-123".to_string()),
            json!({"answer": 42}),
        );
        let json = response.to_json().unwrap();

        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":\"req-123\""));
        assert!(json.contains("\"result\""));
        assert!(json.contains("\"answer\":42"));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_response_serialization_error() {
        let error = JsonRpcError::invalid_params("missing required field");
        let response = JsonRpcResponse::error(RequestId::Number(1), error);
        let json = response.to_json().unwrap();

        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"error\""));
        assert!(json.contains("\"code\":-32602"));
        assert!(!json.contains("\"result\""));
    }

    #[test]
    fn test_response_deserialization_success() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let response = JsonRpcResponse::from_json(json).unwrap();

        assert!(response.is_success());
        assert!(response.get_result().is_some());
    }

    #[test]
    fn test_response_deserialization_error() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
        let response = JsonRpcResponse::from_json(json).unwrap();

        assert!(response.is_error());
        assert!(response.get_error().is_some());
        assert_eq!(response.get_error().unwrap().code, -32601);
    }

    #[test]
    fn test_response_roundtrip() {
        let original = JsonRpcResponse::success(
            RequestId::String("test-id".to_string()),
            json!({
                "nested": {
                    "data": [1, 2, 3]
                }
            }),
        );

        let json = original.to_json().unwrap();
        let parsed = JsonRpcResponse::from_json(&json).unwrap();

        assert_eq!(parsed.jsonrpc, original.jsonrpc);
        assert_eq!(parsed.id, original.id);
        assert!(parsed.is_success());
    }
}

// ============================================================================
// Notification Message Tests
// ============================================================================

mod notification_tests {
    use super::*;

    #[test]
    fn test_notification_basic_creation() {
        let notification = JsonRpcNotification::new("notifications/initialized");

        assert_eq!(notification.jsonrpc, "2.0");
        assert_eq!(notification.method, "notifications/initialized");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_notification_with_params() {
        let params = json!({"level": "info"});
        let notification = JsonRpcNotification::with_params("notifications/message", params.clone());

        assert!(notification.params.is_some());
        assert_eq!(notification.params.unwrap(), params);
    }

    #[test]
    fn test_notification_serialization_no_id() {
        // Notifications must NOT have an "id" field
        let notification = JsonRpcNotification::new("test_event");
        let json = notification.to_json().unwrap();

        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"test_event\""));
        // Verify "id" field is NOT present
        assert!(!json.contains("\"id\""));
    }

    #[test]
    fn test_notification_serialization_with_params() {
        let notification =
            JsonRpcNotification::with_params("event", json!({"data": "value"}));
        let json = notification.to_json().unwrap();

        assert!(json.contains("\"params\""));
        assert!(json.contains("\"data\":\"value\""));
    }

    #[test]
    fn test_notification_deserialization() {
        let json = r#"{"jsonrpc":"2.0","method":"notifications/canceled","params":{"reason":"timeout"}}"#;
        let notification = JsonRpcNotification::from_json(json).unwrap();

        assert_eq!(notification.jsonrpc, "2.0");
        assert_eq!(notification.method, "notifications/canceled");
        assert!(notification.params.is_some());
    }

    #[test]
    fn test_notification_rejects_id_field() {
        // A message with an "id" field should NOT deserialize as a notification
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
        let result = JsonRpcNotification::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_notification_requires_method_field() {
        let json = r#"{"jsonrpc":"2.0"}"#;
        let result = JsonRpcNotification::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_notification_roundtrip() {
        let original =
            JsonRpcNotification::with_params("complex/event", json!({"nested": {"key": "value"}}));

        let json = original.to_json().unwrap();
        let parsed = JsonRpcNotification::from_json(&json).unwrap();

        assert_eq!(parsed.jsonrpc, original.jsonrpc);
        assert_eq!(parsed.method, original.method);
        assert_eq!(parsed.params, original.params);
    }
}

// ============================================================================
// JsonRpcMessage (Generic Message) Tests
// ============================================================================

mod generic_message_tests {
    use super::*;

    #[test]
    fn test_message_detects_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
        let message = JsonRpcMessage::from_json(json).unwrap();

        assert!(message.is_request());
        assert!(!message.is_response());
        assert!(!message.is_notification());
    }

    #[test]
    fn test_message_detects_response_with_result() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
        let message = JsonRpcMessage::from_json(json).unwrap();

        assert!(!message.is_request());
        assert!(message.is_response());
        assert!(!message.is_notification());
    }

    #[test]
    fn test_message_detects_response_with_error() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"error"}}"#;
        let message = JsonRpcMessage::from_json(json).unwrap();

        assert!(!message.is_request());
        assert!(message.is_response());
        assert!(!message.is_notification());
    }

    #[test]
    fn test_message_detects_notification() {
        let json = r#"{"jsonrpc":"2.0","method":"test"}"#;
        let message = JsonRpcMessage::from_json(json).unwrap();

        assert!(!message.is_request());
        assert!(!message.is_response());
        assert!(message.is_notification());
    }

    #[test]
    fn test_message_to_json_roundtrip() {
        // Test request roundtrip
        let request = JsonRpcRequest::with_params(RequestId::Number(42), "method", json!({"a": 1}));
        let message = JsonRpcMessage::Request(request);
        let json = message.to_json().unwrap();
        let parsed = JsonRpcMessage::from_json(&json).unwrap();
        assert!(parsed.is_request());

        // Test response roundtrip
        let response = JsonRpcResponse::success(RequestId::Number(42), json!({"b": 2}));
        let message = JsonRpcMessage::Response(response);
        let json = message.to_json().unwrap();
        let parsed = JsonRpcMessage::from_json(&json).unwrap();
        assert!(parsed.is_response());

        // Test notification roundtrip
        let notification = JsonRpcNotification::with_params("event", json!({"c": 3}));
        let message = JsonRpcMessage::Notification(notification);
        let json = message.to_json().unwrap();
        let parsed = JsonRpcMessage::from_json(&json).unwrap();
        assert!(parsed.is_notification());
    }
}

// ============================================================================
// Error Types Tests
// ============================================================================

mod error_tests {
    use super::*;

    #[test]
    fn test_error_code_values() {
        // Standard JSON-RPC 2.0 error codes
        assert_eq!(JsonRpcErrorCode::ParseError.as_i32(), -32700);
        assert_eq!(JsonRpcErrorCode::InvalidRequest.as_i32(), -32600);
        assert_eq!(JsonRpcErrorCode::MethodNotFound.as_i32(), -32601);
        assert_eq!(JsonRpcErrorCode::InvalidParams.as_i32(), -32602);
        assert_eq!(JsonRpcErrorCode::InternalError.as_i32(), -32603);
    }

    #[test]
    fn test_error_code_from_i32() {
        assert_eq!(JsonRpcErrorCode::from_i32(-32700), JsonRpcErrorCode::ParseError);
        assert_eq!(JsonRpcErrorCode::from_i32(-32600), JsonRpcErrorCode::InvalidRequest);
        assert_eq!(JsonRpcErrorCode::from_i32(-32601), JsonRpcErrorCode::MethodNotFound);
        assert_eq!(JsonRpcErrorCode::from_i32(-32602), JsonRpcErrorCode::InvalidParams);
        assert_eq!(JsonRpcErrorCode::from_i32(-32603), JsonRpcErrorCode::InternalError);
    }

    #[test]
    fn test_server_error_range() {
        // Server errors are in range -32000 to -32099
        let server_error = JsonRpcErrorCode::ServerError(-32050);
        assert_eq!(server_error.as_i32(), -32050);
        assert_eq!(JsonRpcErrorCode::from_i32(-32050), JsonRpcErrorCode::ServerError(-32050));
    }

    #[test]
    fn test_error_serialization() {
        let error = JsonRpcError::method_not_found("test");
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("\"code\":-32601"));
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"data\""));
    }

    #[test]
    fn test_error_deserialization() {
        let json = r#"{"code":-32602,"message":"Invalid params","data":{"field":"url"}}"#;
        let error: JsonRpcError = serde_json::from_str(json).unwrap();

        assert_eq!(error.code, -32602);
        assert_eq!(error.message, "Invalid params");
        assert!(error.data.is_some());
    }

    #[test]
    fn test_error_without_data() {
        let error = JsonRpcError::new(
            JsonRpcErrorCode::InternalError,
            "Something went wrong".to_string(),
        );
        let json = serde_json::to_string(&error).unwrap();

        // data field should not be present when None
        assert!(!json.contains("\"data\""));
    }

    #[test]
    fn test_error_helper_constructors() {
        let parse_error = JsonRpcError::parse_error("unexpected EOF");
        assert_eq!(parse_error.code, -32700);
        assert!(parse_error.message.contains("unexpected EOF"));

        let invalid_request = JsonRpcError::invalid_request("missing jsonrpc");
        assert_eq!(invalid_request.code, -32600);

        let method_not_found = JsonRpcError::method_not_found("unknown");
        assert_eq!(method_not_found.code, -32601);

        let invalid_params = JsonRpcError::invalid_params("required field missing");
        assert_eq!(invalid_params.code, -32602);

        let internal = JsonRpcError::internal_error("timeout");
        assert_eq!(internal.code, -32603);
    }
}

// ============================================================================
// MCP Method Types Tests
// ============================================================================

mod mcp_method_types_tests {
    use super::*;

    #[test]
    fn test_protocol_version() {
        assert_eq!(MCP_PROTOCOL_VERSION, "2025-11-25");
    }

    #[test]
    fn test_implementation_info() {
        let info = ImplementationInfo::new("ltmatrix", "0.1.0");

        assert_eq!(info.name, "ltmatrix");
        assert_eq!(info.version, "0.1.0");

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"name\":\"ltmatrix\""));
        assert!(json.contains("\"version\":\"0.1.0\""));
    }

    #[test]
    fn test_initialize_params() {
        let params = InitializeParams::new("test-client", "1.0.0");

        assert_eq!(params.protocol_version, MCP_PROTOCOL_VERSION);
        assert_eq!(params.client_info.name, "test-client");
    }

    #[test]
    fn test_initialize_params_serialization() {
        let params = InitializeParams::new("client", "1.0")
            .with_capabilities(ClientCapabilities {
                roots: Some(RootsCapability::with_list_changed(true)),
                ..Default::default()
            });

        let json = serde_json::to_string(&params).unwrap();

        // Verify camelCase serialization
        assert!(json.contains("\"protocolVersion\""));
        assert!(json.contains("\"clientInfo\""));
    }

    #[test]
    fn test_initialize_result_deserialization() {
        let json = json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {
                "tools": { "listChanged": true },
                "resources": { "subscribe": true, "listChanged": true }
            },
            "serverInfo": {
                "name": "test-server",
                "version": "1.0.0"
            },
            "instructions": "Use this server for testing"
        });

        let result: InitializeResult = serde_json::from_value(json).unwrap();

        assert_eq!(result.protocol_version, "2025-11-25");
        assert_eq!(result.server_info.name, "test-server");
        assert!(result.instructions.is_some());
    }

    #[test]
    fn test_tool_definition() {
        let tool = Tool::new(
            "browser_navigate",
            "Navigate to a URL",
            json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string" }
                },
                "required": ["url"]
            }),
        );

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("\"inputSchema\"")); // camelCase
        assert!(json.contains("\"browser_navigate\""));
    }

    #[test]
    fn test_tool_call_params() {
        let params = ToolCallParams::new("test_tool").with_arguments(json!({"url": "https://example.com"}));

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("\"name\":\"test_tool\""));
        assert!(json.contains("\"arguments\""));
    }

    #[test]
    fn test_tool_content_text() {
        let content = ToolContent::text("Hello, world!");

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello, world!\""));
    }

    #[test]
    fn test_tool_content_image() {
        let content = ToolContent::image("base64imagedata", "image/png");

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"image\""));
        assert!(json.contains("\"data\":\"base64imagedata\""));
        assert!(json.contains("\"mime_type\":\"image/png\""));
    }

    #[test]
    fn test_tool_content_resource() {
        let content = ToolContent::resource("file:///test.txt", Some("text/plain".to_string()));

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"resource\""));
        assert!(json.contains("\"uri\":\"file:///test.txt\""));
    }

    #[test]
    fn test_tool_call_result_success() {
        let result = ToolCallResult::text("Operation completed");

        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn test_tool_call_result_error() {
        let result = ToolCallResult::error("Operation failed");

        assert!(result.is_error);
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn test_resource() {
        let resource = Resource::new("file:///project/package.json", "package.json");

        let json = serde_json::to_string(&resource).unwrap();
        assert!(json.contains("\"uri\":\"file:///project/package.json\""));
        assert!(json.contains("\"name\":\"package.json\""));
    }

    #[test]
    fn test_resource_contents_text() {
        let contents = ResourceContents::text("file:///test.txt", "Hello, world!");

        let json = serde_json::to_string(&contents).unwrap();
        assert!(json.contains("\"text\":\"Hello, world!\""));
        assert!(json.contains("\"mime_type\":\"text/plain\""));
    }

    #[test]
    fn test_resource_contents_blob() {
        let contents = ResourceContents::blob("file:///binary.bin", "base64data", "application/octet-stream");

        let json = serde_json::to_string(&contents).unwrap();
        assert!(json.contains("\"blob\":\"base64data\""));
        assert!(json.contains("\"mime_type\":\"application/octet-stream\""));
    }

    #[test]
    fn test_prompt_message_user() {
        let msg = PromptMessage::user("Hello");

        assert_eq!(msg.role, "user");
        match msg.content {
            PromptContent::Text { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_prompt_message_assistant() {
        let msg = PromptMessage::assistant("Hi there!");

        assert_eq!(msg.role, "assistant");
        match msg.content {
            PromptContent::Text { text } => assert_eq!(text, "Hi there!"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_log_level_serialization() {
        assert_eq!(serde_json::to_string(&LogLevel::Debug).unwrap(), "\"debug\"");
        assert_eq!(serde_json::to_string(&LogLevel::Info).unwrap(), "\"info\"");
        assert_eq!(serde_json::to_string(&LogLevel::Warning).unwrap(), "\"warning\"");
        assert_eq!(serde_json::to_string(&LogLevel::Error).unwrap(), "\"error\"");
    }

    #[test]
    fn test_log_level_deserialization() {
        let level: LogLevel = serde_json::from_str("\"debug\"").unwrap();
        assert_eq!(level, LogLevel::Debug);

        let level: LogLevel = serde_json::from_str("\"warning\"").unwrap();
        assert_eq!(level, LogLevel::Warning);
    }

    #[test]
    fn test_log_level_default() {
        assert_eq!(LogLevel::default(), LogLevel::Info);
    }
}

// ============================================================================
// Server/Client Capabilities Tests
// ============================================================================

mod capabilities_tests {
    use super::*;

    #[test]
    fn test_client_capabilities_default() {
        let caps = ClientCapabilities::default();

        // All fields should be None by default
        assert!(caps.experimental.is_none());
        assert!(caps.roots.is_none());
        assert!(caps.sampling.is_none());
    }

    #[test]
    fn test_client_capabilities_with_roots() {
        let caps = ClientCapabilities {
            roots: Some(RootsCapability::with_list_changed(true)),
            ..Default::default()
        };

        let json = serde_json::to_string(&caps).unwrap();
        assert!(json.contains("\"roots\""));
        assert!(json.contains("\"listChanged\":true"));
        // Optional fields should not be serialized when None
        assert!(!json.contains("\"experimental\""));
        assert!(!json.contains("\"sampling\""));
    }

    #[test]
    fn test_server_capabilities_default() {
        let caps = ServerCapabilities::default();

        assert!(caps.experimental.is_none());
        assert!(caps.logging.is_none());
        assert!(caps.completions.is_none());
        assert!(caps.prompts.is_none());
        assert!(caps.resources.is_none());
        assert!(caps.tools.is_none());
    }

    #[test]
    fn test_server_capabilities_with_tools() {
        let caps = ServerCapabilities {
            tools: Some(ToolsCapability {
                list_changed: Some(true),
            }),
            ..Default::default()
        };

        let json = serde_json::to_string(&caps).unwrap();
        assert!(json.contains("\"tools\""));
        assert!(json.contains("\"listChanged\":true"));
    }

    #[test]
    fn test_server_capabilities_with_resources() {
        let caps = ServerCapabilities {
            resources: Some(ResourcesCapability {
                subscribe: Some(true),
                list_changed: Some(true),
            }),
            ..Default::default()
        };

        let json = serde_json::to_string(&caps).unwrap();
        assert!(json.contains("\"resources\""));
        assert!(json.contains("\"subscribe\":true"));
        assert!(json.contains("\"listChanged\":true"));
    }
}

// ============================================================================
// Protocol Compliance Tests
// ============================================================================

mod protocol_compliance_tests {
    use super::*;

    /// JSON-RPC 2.0 requires "jsonrpc" field to be exactly "2.0"
    #[test]
    fn test_jsonrpc_version_is_2_0() {
        let request = JsonRpcRequest::new(RequestId::Number(1), "test");
        assert_eq!(request.jsonrpc, "2.0");

        let response = JsonRpcResponse::success(RequestId::Number(1), json!({}));
        assert_eq!(response.jsonrpc, "2.0");

        let notification = JsonRpcNotification::new("test");
        assert_eq!(notification.jsonrpc, "2.0");
    }

    /// Request ID must be preserved in response
    #[test]
    fn test_response_preserves_request_id() {
        let request_id = RequestId::String("unique-correlation-id".to_string());
        let response = JsonRpcResponse::success(request_id.clone(), json!({"result": "ok"}));

        assert_eq!(response.id, request_id);
    }

    /// Verify skip_serializing_if behavior for optional fields
    #[test]
    fn test_optional_fields_not_serialized_when_none() {
        // Request without params
        let request = JsonRpcRequest::new(RequestId::Number(1), "test");
        let json = request.to_json().unwrap();
        assert!(!json.contains("\"params\""));

        // Error without data
        let error = JsonRpcError::new(
            JsonRpcErrorCode::InternalError,
            "error".to_string(),
        );
        let json = serde_json::to_string(&error).unwrap();
        assert!(!json.contains("\"data\""));
    }

    /// Verify MCP protocol version is set correctly
    #[test]
    fn test_mcp_protocol_version_in_initialize() {
        let params = InitializeParams::new("client", "1.0");
        assert_eq!(params.protocol_version, MCP_PROTOCOL_VERSION);

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains(&format!("\"protocolVersion\":\"{}\"", MCP_PROTOCOL_VERSION)));
    }
}
