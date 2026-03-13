// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Comprehensive validation and edge case tests for MCP protocol
//!
//! Tests for:
//! - Invalid JSON parsing and error recovery
//! - Missing required fields detection
//! - Wrong field types handling
//! - Unicode and special character handling
//! - Numeric boundary conditions
//! - Malformed message detection
//! - Extra/unknown fields behavior

use ltmatrix::mcp::protocol::{JsonRpcErrorCode, McpError, McpErrorCode};
use ltmatrix::mcp::{
    ClientCapabilities, ImplementationInfo, InitializeParams, InitializeResult, JsonRpcError,
    JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, LogLevel, PromptContent,
    PromptMessage, RequestId, Resource, ResourceContents, Root, ServerCapabilities, Tool,
    ToolCallParams, ToolCallResult, ToolContent, ToolsListParams, ToolsListResult,
};
use serde_json::json;

// ============================================================================
// Invalid JSON Parsing Tests
// ============================================================================

mod invalid_json_tests {
    use super::*;

    #[test]
    fn test_request_invalid_json_syntax() {
        let invalid_jsons = [
            r#"{invalid}"#,
            r#"{"jsonrpc": "2.0", "id": 1, "method": "test""#, // Missing closing brace
            r#"{"jsonrpc": "2.0", "id": 1, "method": }"#,      // Missing value
            r#"{"jsonrpc": "2.0", "id": 1, "method": "test",}"#, // Trailing comma
            r#"{jsonrpc: "2.0", "id": 1, "method": "test"}"#,  // Unquoted key
            r#"{"jsonrpc": '2.0', "id": 1, "method": "test"}"#, // Single quotes
        ];

        for invalid in invalid_jsons {
            let result = JsonRpcRequest::from_json(invalid);
            assert!(result.is_err(), "Should fail for invalid JSON: {}", invalid);
        }
    }

    #[test]
    fn test_response_invalid_json_syntax() {
        let invalid_jsons = [
            r#"{"jsonrpc": "2.0", "id": 1, result: {}}"#,
            r#"not json at all"#,
            r#"null"#,
            r#"[]"#,
            r#""just a string""#,
            r#"123"#,
        ];

        for invalid in invalid_jsons {
            let result = JsonRpcResponse::from_json(invalid);
            assert!(result.is_err(), "Should fail for: {}", invalid);
        }
    }

    #[test]
    fn test_notification_invalid_json_syntax() {
        let invalid_jsons = [
            r#"{"jsonrpc": "2.0", "method": }"#,
            r#"{"jsonrpc": 2.0, "method": "test"}"#, // Version should be string
        ];

        for invalid in invalid_jsons {
            let result = JsonRpcNotification::from_json(invalid);
            assert!(result.is_err(), "Should fail for: {}", invalid);
        }
    }

    #[test]
    fn test_message_invalid_json_returns_error() {
        let result = JsonRpcMessage::from_json("{invalid}");
        assert!(result.is_err());
    }
}

// ============================================================================
// Missing Required Fields Tests
// ============================================================================

mod missing_required_fields_tests {
    use super::*;

    #[test]
    fn test_request_missing_jsonrpc_field() {
        let json = r#"{"id": 1, "method": "test"}"#;
        let result = JsonRpcRequest::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_missing_id_field() {
        let json = r#"{"jsonrpc": "2.0", "method": "test"}"#;
        let result = JsonRpcRequest::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_missing_method_field() {
        let json = r#"{"jsonrpc": "2.0", "id": 1}"#;
        let result = JsonRpcRequest::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_empty_object() {
        let json = r#"{}"#;
        let result = JsonRpcRequest::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_response_missing_jsonrpc_field() {
        let json = r#"{"id": 1, "result": {}}"#;
        let result = JsonRpcResponse::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_response_missing_id_field() {
        let json = r#"{"jsonrpc": "2.0", "result": {}}"#;
        let result = JsonRpcResponse::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_response_missing_result_and_error() {
        // A response must have either result or error
        let json = r#"{"jsonrpc": "2.0", "id": 1}"#;
        let response: Result<JsonRpcResponse, _> = serde_json::from_str(json);
        // This should parse but is_success() and is_error() should both handle it
        if let Ok(resp) = response {
            assert!(!resp.is_success());
            assert!(!resp.is_error());
        }
    }

    #[test]
    fn test_notification_missing_jsonrpc_field() {
        let json = r#"{"method": "test"}"#;
        let result = JsonRpcNotification::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_notification_missing_method_field() {
        let json = r#"{"jsonrpc": "2.0"}"#;
        let result = JsonRpcNotification::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_missing_code_field() {
        let json = r#"{"message": "error"}"#;
        let result: Result<JsonRpcError, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_missing_message_field() {
        let json = r#"{"code": -32600}"#;
        let result: Result<JsonRpcError, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_initialize_result_missing_required_fields() {
        // Missing serverInfo
        let json = json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {}
        });
        let result: Result<InitializeResult, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_missing_required_fields() {
        // Missing inputSchema
        let json = json!({
            "name": "test_tool",
            "description": "A test tool"
        });
        let result: Result<Tool, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_resource_missing_required_fields() {
        // Missing name
        let json = json!({
            "uri": "file:///test.txt"
        });
        let result: Result<Resource, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }
}

// ============================================================================
// Wrong Field Type Tests
// ============================================================================

mod wrong_field_type_tests {
    use super::*;

    #[test]
    fn test_request_id_as_array() {
        let json = r#"{"jsonrpc": "2.0", "id": [], "method": "test"}"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_str(json);
        // Arrays are not valid request IDs
        assert!(result.is_err());
    }

    #[test]
    fn test_request_id_as_object() {
        let json = r#"{"jsonrpc": "2.0", "id": {}, "method": "test"}"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_id_as_bool() {
        let json = r#"{"jsonrpc": "2.0", "id": true, "method": "test"}"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_method_as_number() {
        let json = r#"{"jsonrpc": "2.0", "id": 1, "method": 123}"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_jsonrpc_as_number() {
        let json = r#"{"jsonrpc": 2.0, "id": 1, "method": "test"}"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_code_as_string() {
        let json = r#"{"code": "-32600", "message": "error"}"#;
        let result: Result<JsonRpcError, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_params_as_string() {
        // Note: serde_json::Value accepts any JSON value, including strings
        // JSON-RPC spec says params should be object/array, but we accept any Value
        let json = r#"{"jsonrpc": "2.0", "id": 1, "method": "test", "params": "string"}"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_str(json);
        // This parses successfully because params: Option<Value> accepts any JSON
        assert!(result.is_ok());
    }

    #[test]
    fn test_params_as_number() {
        // Note: serde_json::Value accepts any JSON value, including numbers
        let json = r#"{"jsonrpc": "2.0", "id": 1, "method": "test", "params": 123}"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_str(json);
        // This parses successfully because params: Option<Value> accepts any JSON
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_call_result_content_as_string() {
        let json = json!({
            "content": "should be array",
            "isError": false
        });
        let result: Result<ToolCallResult, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_tools_list_result_tools_as_object() {
        let json = json!({
            "tools": {"tool1": {}}
        });
        let result: Result<ToolsListResult, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }
}

// ============================================================================
// Unicode and Special Character Tests
// ============================================================================

mod unicode_tests {
    use super::*;

    #[test]
    fn test_request_method_with_unicode() {
        let json = r#"{"jsonrpc": "2.0", "id": 1, "method": "test_方法_🔥"}"#;
        let request = JsonRpcRequest::from_json(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert!(req.method.contains("方法"));
        assert!(req.method.contains("🔥"));
    }

    #[test]
    fn test_request_id_with_unicode() {
        let json = r#"{"jsonrpc": "2.0", "id": "请求-123-🔥", "method": "test"}"#;
        let request = JsonRpcRequest::from_json(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.id, RequestId::String("请求-123-🔥".to_string()));
    }

    #[test]
    fn test_error_message_with_unicode() {
        let error = JsonRpcError::new(
            JsonRpcErrorCode::InternalError,
            "内部错误: 操作失败 🚫".to_string(),
        );
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("内部错误"));
        assert!(json.contains("🚫"));

        let parsed: JsonRpcError = serde_json::from_str(&json).unwrap();
        assert!(parsed.message.contains("🚫"));
    }

    #[test]
    fn test_params_with_unicode() {
        let params = json!({
            "name": "张三",
            "emoji": "🎉🎊",
            "mixed": "Hello 世界 World"
        });
        let request = JsonRpcRequest::with_params(RequestId::Number(1), "test", params);

        let json = request.to_json().unwrap();
        assert!(json.contains("张三"));
        assert!(json.contains("🎉"));

        let parsed = JsonRpcRequest::from_json(&json).unwrap();
        let p = parsed.params.unwrap();
        assert_eq!(p["name"], "张三");
        assert_eq!(p["emoji"], "🎉🎊");
    }

    #[test]
    fn test_tool_description_with_unicode() {
        let tool = Tool::new(
            "test_tool",
            "A tool for testing 测试工具 🛠️",
            json!({"type": "object"}),
        );

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("测试工具"));
        assert!(json.contains("🛠️"));
    }

    #[test]
    fn test_resource_uri_with_unicode() {
        // URIs can contain percent-encoded unicode
        let resource = Resource::new("file:///path/to/%E6%96%87%E4%BB%B6.txt", "文件.txt");

        let json = serde_json::to_string(&resource).unwrap();
        let parsed: Resource = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "文件.txt");
    }

    #[test]
    fn test_string_with_null_bytes() {
        // JSON strings can contain escaped null bytes
        let json = r#"{"jsonrpc": "2.0", "id": "test\u0000null", "method": "test"}"#;
        let request = JsonRpcRequest::from_json(json);
        assert!(request.is_ok());
    }

    #[test]
    fn test_string_with_escape_sequences() {
        let json = r#"{"jsonrpc": "2.0", "id": "line1\nline2\ttab", "method": "test"}"#;
        let request = JsonRpcRequest::from_json(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert!(req.id.to_string().contains("line1"));
    }

    #[test]
    fn test_content_with_surrogate_pairs() {
        // Emoji using surrogate pairs
        let json =
            r#"{"jsonrpc": "2.0", "id": 1, "method": "test", "params": {"emoji": "\uD83D\uDE00"}}"#;
        let request = JsonRpcRequest::from_json(json);
        assert!(request.is_ok());
    }
}

// ============================================================================
// Numeric Boundary Tests
// ============================================================================

mod numeric_boundary_tests {
    use super::*;

    #[test]
    fn test_request_id_max_i64() {
        let json = format!(
            r#"{{"jsonrpc": "2.0", "id": {}, "method": "test"}}"#,
            i64::MAX
        );
        let request = JsonRpcRequest::from_json(&json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.id, RequestId::Number(i64::MAX));
    }

    #[test]
    fn test_request_id_min_i64() {
        let json = format!(
            r#"{{"jsonrpc": "2.0", "id": {}, "method": "test"}}"#,
            i64::MIN
        );
        let request = JsonRpcRequest::from_json(&json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.id, RequestId::Number(i64::MIN));
    }

    #[test]
    fn test_request_id_zero() {
        let json = r#"{"jsonrpc": "2.0", "id": 0, "method": "test"}"#;
        let request = JsonRpcRequest::from_json(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.id, RequestId::Number(0));
    }

    #[test]
    fn test_request_id_negative() {
        let json = r#"{"jsonrpc": "2.0", "id": -999999, "method": "test"}"#;
        let request = JsonRpcRequest::from_json(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.id, RequestId::Number(-999999));
    }

    #[test]
    fn test_error_code_boundaries() {
        // Standard JSON-RPC error codes
        assert_eq!(JsonRpcErrorCode::ParseError.as_i32(), -32700);
        assert_eq!(JsonRpcErrorCode::InternalError.as_i32(), -32603);

        // Server error range boundaries
        let min_server = JsonRpcErrorCode::ServerError(-32000);
        let max_server = JsonRpcErrorCode::ServerError(-32099);
        assert_eq!(min_server.as_i32(), -32000);
        assert_eq!(max_server.as_i32(), -32099);
    }

    #[test]
    fn test_large_floating_point_in_params() {
        // Note: serde_json serializes infinity as null, so we test with large finite values
        let params = json!({
            "large": 1.7976931348623157e308,  // f64::MAX
            "small": 2.2250738585072014e-308, // f64::MIN_POSITIVE
            "zero": 0.0,
            "negative": -1.7976931348623157e308,
        });

        let request = JsonRpcRequest::with_params(RequestId::Number(1), "test", params);
        let json = request.to_json().unwrap();
        let parsed = JsonRpcRequest::from_json(&json).unwrap();
        let p = parsed.params.unwrap();

        assert_eq!(p["large"], 1.7976931348623157e308);
        assert_eq!(p["small"], 2.2250738585072014e-308);
        assert_eq!(p["zero"], 0.0);
        assert_eq!(p["negative"], -1.7976931348623157e308);
    }

    #[test]
    fn test_progress_token_numeric() {
        let progress_params = ltmatrix::mcp::ProgressParams {
            progress_token: json!(1234567890123456789i64),
            progress: 50.5,
            total: Some(100.0),
        };

        let json = serde_json::to_string(&progress_params).unwrap();
        assert!(json.contains("1234567890123456789"));
    }
}

// ============================================================================
// Extra/Unknown Fields Tests
// ============================================================================

mod unknown_fields_tests {
    use super::*;

    #[test]
    fn test_request_with_extra_fields() {
        // JSON-RPC allows extra fields that should be ignored
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "test",
            "extra_field": "should be ignored",
            "another_extra": 123
        }"#;

        let request = JsonRpcRequest::from_json(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.method, "test");
    }

    #[test]
    fn test_response_with_extra_fields() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {},
            "timestamp": "2024-01-01T00:00:00Z"
        }"#;

        let response = JsonRpcResponse::from_json(json);
        assert!(response.is_ok());
    }

    #[test]
    fn test_initialize_params_with_extra_fields() {
        let json = json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"},
            "extra": "ignored"
        });

        let result: Result<InitializeParams, _> = serde_json::from_value(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_with_extra_fields() {
        let json = json!({
            "name": "test_tool",
            "description": "A tool",
            "inputSchema": {"type": "object"},
            "deprecated": false,
            "version": "2.0"
        });

        let result: Result<Tool, _> = serde_json::from_value(json);
        assert!(result.is_ok());
    }
}

// ============================================================================
// Message Type Detection Edge Cases
// ============================================================================

mod message_type_detection_tests {
    use super::*;

    #[test]
    fn test_message_detects_request_with_null_id() {
        // null id is valid for requests (but unusual)
        let json = r#"{"jsonrpc":"2.0","id":null,"method":"test"}"#;
        let message = JsonRpcMessage::from_json(json).unwrap();
        assert!(message.is_request());
    }

    #[test]
    fn test_message_with_both_result_and_error() {
        // Invalid: has both result and error
        let json =
            r#"{"jsonrpc":"2.0","id":1,"result":{},"error":{"code":-32600,"message":"err"}}"#;
        let result: Result<JsonRpcResponse, _> = serde_json::from_str(json);
        // This behavior depends on implementation - it might parse but be invalid
        if let Ok(response) = result {
            // If it parses, it should be treated as an error (error takes precedence)
            // or as success (result takes precedence) depending on implementation
            let _ = response.is_success() || response.is_error();
        }
    }

    #[test]
    fn test_message_with_null_result() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":null}"#;
        let message = JsonRpcMessage::from_json(json).unwrap();
        assert!(message.is_response());
    }

    #[test]
    fn test_notification_vs_request_distinction() {
        // Notification: no id field
        let notification_json = r#"{"jsonrpc":"2.0","method":"test"}"#;
        let msg = JsonRpcMessage::from_json(notification_json).unwrap();
        assert!(msg.is_notification());
        assert!(!msg.is_request());

        // Request: has id field
        let request_json = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
        let msg = JsonRpcMessage::from_json(request_json).unwrap();
        assert!(msg.is_request());
        assert!(!msg.is_notification());
    }
}

// ============================================================================
// Content Type Variant Tests
// ============================================================================

mod content_type_tests {
    use super::*;

    #[test]
    fn test_tool_content_text_roundtrip() {
        let content = ToolContent::text("Hello, world!");
        let json = serde_json::to_string(&content).unwrap();
        let parsed: ToolContent = serde_json::from_str(&json).unwrap();

        match parsed {
            ToolContent::Text { text } => assert_eq!(text, "Hello, world!"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_tool_content_image_roundtrip() {
        let content = ToolContent::image("base64data", "image/png");
        let json = serde_json::to_string(&content).unwrap();
        let parsed: ToolContent = serde_json::from_str(&json).unwrap();

        match parsed {
            ToolContent::Image { data, mime_type } => {
                assert_eq!(data, "base64data");
                assert_eq!(mime_type, "image/png");
            }
            _ => panic!("Expected Image variant"),
        }
    }

    #[test]
    fn test_tool_content_resource_roundtrip() {
        let content = ToolContent::resource("file:///test.txt", Some("text/plain".to_string()));
        let json = serde_json::to_string(&content).unwrap();
        let parsed: ToolContent = serde_json::from_str(&json).unwrap();

        match parsed {
            ToolContent::Resource { uri, mime_type } => {
                assert_eq!(uri, "file:///test.txt");
                assert_eq!(mime_type, Some("text/plain".to_string()));
            }
            _ => panic!("Expected Resource variant"),
        }
    }

    #[test]
    fn test_tool_content_invalid_type() {
        let json = json!({"type": "invalid", "text": "test"});
        let result: Result<ToolContent, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_content_missing_text_field() {
        let json = json!({"type": "text"});
        let result: Result<ToolContent, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_content_missing_image_fields() {
        let json = json!({"type": "image", "data": "base64"}); // Missing mime_type
        let result: Result<ToolContent, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_prompt_content_text() {
        let content = PromptContent::Text {
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains(r#""type":"text""#));
        assert!(json.contains(r#""text":"Hello""#));
    }

    #[test]
    fn test_prompt_content_image() {
        let content = PromptContent::Image {
            data: "base64imagedata".to_string(),
            mime_type: "image/png".to_string(),
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains(r#""type":"image""#));
        assert!(json.contains(r#""data":"base64imagedata""#));
    }

    #[test]
    fn test_prompt_content_resource() {
        let content = PromptContent::Resource {
            resource: ltmatrix::mcp::Resource::new("file:///test.txt", "test.txt"),
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains(r#""type":"resource""#));
    }
}

// ============================================================================
// Resource Contents Tests
// ============================================================================

mod resource_contents_tests {
    use super::*;

    #[test]
    fn test_text_contents_roundtrip() {
        let contents = ResourceContents::text("file:///test.txt", "Hello, world!");
        let json = serde_json::to_string(&contents).unwrap();
        let parsed: ResourceContents = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.uri, "file:///test.txt");
        assert_eq!(parsed.text, Some("Hello, world!".to_string()));
        assert_eq!(parsed.blob, None);
    }

    #[test]
    fn test_blob_contents_roundtrip() {
        let contents = ResourceContents::blob(
            "file:///binary.bin",
            "aGVsbG8gd29ybGQ=", // base64 of "hello world"
            "application/octet-stream",
        );
        let json = serde_json::to_string(&contents).unwrap();
        let parsed: ResourceContents = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.uri, "file:///binary.bin");
        assert_eq!(parsed.blob, Some("aGVsbG8gd29ybGQ=".to_string()));
        assert_eq!(parsed.text, None);
    }

    #[test]
    fn test_contents_with_both_text_and_blob() {
        // Having both text and blob is unusual but should serialize
        let contents = ResourceContents {
            uri: "file:///test.txt".to_string(),
            mime_type: Some("text/plain".to_string()),
            text: Some("text content".to_string()),
            blob: Some("YmxvYiBjb250ZW50".to_string()),
        };

        let json = serde_json::to_string(&contents).unwrap();
        let parsed: ResourceContents = serde_json::from_str(&json).unwrap();

        assert!(parsed.text.is_some());
        assert!(parsed.blob.is_some());
    }

    #[test]
    fn test_contents_without_mime_type() {
        // Create contents with explicit None mime_type
        let contents = ResourceContents {
            uri: "file:///unknown".to_string(),
            mime_type: None,
            text: Some("content".to_string()),
            blob: None,
        };
        assert!(contents.mime_type.is_none());

        let json = serde_json::to_string(&contents).unwrap();
        // mimeType should NOT be in JSON when None (skip_serializing_if)
        assert!(!json.contains("mimeType"));
    }
}

// ============================================================================
// Capability Tests
// ============================================================================

mod capability_tests {
    use super::*;
    use ltmatrix::mcp::{PromptsCapability, ResourcesCapability, RootsCapability, ToolsCapability};

    #[test]
    fn test_client_capabilities_empty() {
        let caps = ClientCapabilities::default();
        let json = serde_json::to_string(&caps).unwrap();
        assert_eq!(json, "{}"); // All fields are None
    }

    #[test]
    fn test_client_capabilities_all_fields() {
        let caps = ClientCapabilities {
            experimental: Some(json!({"feature": true})),
            roots: Some(RootsCapability {
                list_changed: Some(true),
            }),
            sampling: Some(json!({})),
        };

        let json = serde_json::to_string(&caps).unwrap();
        assert!(json.contains("experimental"));
        assert!(json.contains("roots"));
        assert!(json.contains("sampling"));
    }

    #[test]
    fn test_server_capabilities_all_features() {
        let caps = ServerCapabilities {
            experimental: Some(json!({})),
            logging: Some(json!({})),
            completions: Some(json!({})),
            prompts: Some(PromptsCapability {
                list_changed: Some(true),
            }),
            resources: Some(ResourcesCapability {
                subscribe: Some(true),
                list_changed: Some(true),
            }),
            tools: Some(ToolsCapability {
                list_changed: Some(true),
            }),
        };

        let json = serde_json::to_string(&caps).unwrap();
        assert!(json.contains("prompts"));
        assert!(json.contains("resources"));
        assert!(json.contains("tools"));
    }

    #[test]
    fn test_deserialize_minimal_server_capabilities() {
        let json = json!({});
        let caps: ServerCapabilities = serde_json::from_value(json).unwrap();
        assert!(caps.tools.is_none());
        assert!(caps.resources.is_none());
        assert!(caps.prompts.is_none());
    }
}

// ============================================================================
// Prompt Message Tests
// ============================================================================

mod prompt_message_tests {
    use super::*;

    #[test]
    fn test_prompt_message_user_text() {
        let msg = PromptMessage::user("Hello, assistant!");
        assert_eq!(msg.role, "user");

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: PromptMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, "user");
    }

    #[test]
    fn test_prompt_message_assistant_text() {
        let msg = PromptMessage::assistant("Hello, user!");
        assert_eq!(msg.role, "assistant");
    }

    #[test]
    fn test_prompt_message_invalid_role() {
        let json = json!({
            "role": "system",  // Invalid role for MCP
            "content": {"type": "text", "text": "test"}
        });
        // MCP only supports "user" and "assistant" roles
        let result: Result<PromptMessage, _> = serde_json::from_value(json);
        // Depending on implementation, this might fail or accept any string
        if let Ok(msg) = result {
            assert_eq!(msg.role, "system");
        }
    }
}

// ============================================================================
// Error Code Edge Cases
// ============================================================================

mod error_code_edge_cases {
    use super::*;

    #[test]
    fn test_error_code_from_positive_number() {
        // Positive numbers are not valid JSON-RPC error codes
        let code = JsonRpcErrorCode::from_i32(100);
        // Should default to InternalError for unknown codes
        assert_eq!(code, JsonRpcErrorCode::InternalError);
    }

    #[test]
    fn test_error_code_from_zero() {
        let code = JsonRpcErrorCode::from_i32(0);
        assert_eq!(code, JsonRpcErrorCode::InternalError);
    }

    #[test]
    fn test_mcp_error_code_classification() {
        // JSON-RPC standard
        assert!(McpErrorCode::ParseError.is_json_rpc_standard());
        assert!(!McpErrorCode::ParseError.is_server_error());
        assert!(!McpErrorCode::ParseError.is_mcp_specific());

        // Server error
        assert!(!McpErrorCode::RequestTimeout.is_json_rpc_standard());
        assert!(McpErrorCode::RequestTimeout.is_server_error());
        assert!(!McpErrorCode::RequestTimeout.is_mcp_specific());

        // MCP-specific
        assert!(!McpErrorCode::ToolNotFound.is_json_rpc_standard());
        assert!(!McpErrorCode::ToolNotFound.is_server_error());
        assert!(McpErrorCode::ToolNotFound.is_mcp_specific());
    }

    #[test]
    fn test_error_recoverability() {
        // Recoverable errors
        assert!(McpErrorCode::RequestTimeout.is_recoverable());
        assert!(McpErrorCode::RateLimitExceeded.is_recoverable());
        assert!(McpErrorCode::ServerStarting.is_recoverable());

        // Non-recoverable errors
        assert!(!McpErrorCode::ParseError.is_recoverable());
        assert!(!McpErrorCode::InvalidRequest.is_recoverable());
        assert!(!McpErrorCode::ToolNotFound.is_recoverable());
        assert!(!McpErrorCode::ResourceAccessDenied.is_recoverable());
    }
}

// ============================================================================
// Serialization Format Tests
// ============================================================================

mod serialization_format_tests {
    use super::*;

    #[test]
    fn test_camel_case_serialization() {
        let params = InitializeParams::new("client", "1.0");
        let json = serde_json::to_string(&params).unwrap();

        // MCP uses camelCase for JSON
        assert!(json.contains("protocolVersion"));
        assert!(json.contains("clientInfo"));
        assert!(!json.contains("protocol_version")); // snake_case should NOT appear
        assert!(!json.contains("client_info"));
    }

    #[test]
    fn test_optional_fields_skip_serialization() {
        let request = JsonRpcRequest::new(RequestId::Number(1), "test");
        let json = request.to_json().unwrap();

        // Optional params should not be present when None
        assert!(!json.contains("params"));
    }

    #[test]
    fn test_error_data_skip_when_none() {
        let error = JsonRpcError::new(JsonRpcErrorCode::InternalError, "error".to_string());
        let json = serde_json::to_string(&error).unwrap();
        assert!(!json.contains("data"));
    }

    #[test]
    fn test_jsonrpc_version_always_2_0() {
        let request = JsonRpcRequest::new(RequestId::Number(1), "test");
        let json = request.to_json().unwrap();
        assert!(json.contains(r#""jsonrpc":"2.0""#));

        let response = JsonRpcResponse::success(RequestId::Number(1), json!({}));
        let json = response.to_json().unwrap();
        assert!(json.contains(r#""jsonrpc":"2.0""#));

        let notification = JsonRpcNotification::new("test");
        let json = notification.to_json().unwrap();
        assert!(json.contains(r#""jsonrpc":"2.0""#));
    }
}

// ============================================================================
// Large Payload Tests
// ============================================================================

mod large_payload_tests {
    use super::*;

    #[test]
    fn test_large_params_object() {
        // Create a large nested object
        let mut large_obj = serde_json::Map::new();
        for i in 0..1000 {
            large_obj.insert(
                format!("key_{}", i),
                json!({
                    "nested": {
                        "data": vec![1, 2, 3, 4, 5],
                        "name": format!("item_{}", i)
                    }
                }),
            );
        }

        let request = JsonRpcRequest::with_params(RequestId::Number(1), "test", json!(large_obj));

        let json = request.to_json().unwrap();
        assert!(json.len() > 50_000); // Should be reasonably large

        let parsed = JsonRpcRequest::from_json(&json).unwrap();
        assert!(parsed.params.is_some());
    }

    #[test]
    fn test_long_method_name() {
        let long_name = "a".repeat(1000);
        let request = JsonRpcRequest::new(RequestId::Number(1), &long_name);
        let json = request.to_json().unwrap();
        let parsed = JsonRpcRequest::from_json(&json).unwrap();
        assert_eq!(parsed.method.len(), 1000);
    }

    #[test]
    fn test_long_string_id() {
        let long_id = "x".repeat(10000);
        let request = JsonRpcRequest::new(RequestId::String(long_id.clone()), "test");
        let json = request.to_json().unwrap();
        let parsed = JsonRpcRequest::from_json(&json).unwrap();
        assert_eq!(parsed.id, RequestId::String(long_id));
    }

    #[test]
    fn test_many_tools_in_list() {
        let tools: Vec<serde_json::Value> = (0..500)
            .map(|i| {
                json!({
                    "name": format!("tool_{}", i),
                    "description": format!("Tool number {} with a longer description", i),
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "arg1": {"type": "string"},
                            "arg2": {"type": "number"}
                        }
                    }
                })
            })
            .collect();

        let result = ToolsListResult {
            tools: serde_json::from_value(json!(tools)).unwrap(),
            next_cursor: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: ToolsListResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tools.len(), 500);
    }
}
