// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Tests for Tool-Related MCP Methods (list_tools, call_tool)
//!
//! These tests verify tool discovery and execution functionality,
//! including parameter validation and result formatting.

use ltmatrix::mcp::client::McpClient;
use ltmatrix::mcp::protocol::messages::{JsonRpcResponse, RequestId};
use ltmatrix::mcp::protocol::wrappers::{
    McpMethod, McpMethodKind, PaginatedMethod, ToolsCall, ToolsList,
};
use ltmatrix::mcp::{
    Tool, ToolCallParams, ToolCallResult, ToolContent, ToolsCapability, ToolsListParams,
    ToolsListResult,
};
use serde_json::json;

// ============================================================================
// Tools List Method Tests
// ============================================================================

mod tools_list_method_tests {
    use super::*;

    /// Test that ToolsList method has correct method name
    #[test]
    fn test_tools_list_method_name() {
        assert_eq!(ToolsList::METHOD_NAME, "tools/list");
    }

    /// Test building tools/list request without cursor
    #[test]
    fn test_tools_list_build_request_no_cursor() {
        let params = ToolsListParams::default();
        let request = ToolsList::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "tools/list");
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, RequestId::Number(1));
        // Default params should result in empty or no cursor field
        if let Some(p) = request.params {
            assert!(p.get("cursor").is_none() || p["cursor"].is_null());
        }
    }

    /// Test building tools/list request with cursor
    #[test]
    fn test_tools_list_build_request_with_cursor() {
        let params = ToolsListParams {
            cursor: Some("next-tools-page".to_string()),
        };
        let request = ToolsList::build_request(RequestId::Number(2), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["cursor"], "next-tools-page");
    }

    /// Test ToolsList::params helper
    #[test]
    fn test_tools_list_params_helper() {
        let params = ToolsList::params();
        assert!(params.cursor.is_none());
    }

    /// Test ToolsList::params_with_cursor helper
    #[test]
    fn test_tools_list_params_with_cursor_helper() {
        let params = ToolsList::params_with_cursor("page-token-123");
        assert_eq!(params.cursor, Some("page-token-123".to_string()));
    }

    /// Test parsing successful tools/list response
    #[test]
    fn test_tools_list_parse_success_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "tools": [
                    {
                        "name": "browser_navigate",
                        "description": "Navigate to a URL in the browser",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "url": {
                                    "type": "string",
                                    "description": "The URL to navigate to"
                                }
                            },
                            "required": ["url"]
                        }
                    },
                    {
                        "name": "browser_click",
                        "description": "Click on an element",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "selector": {
                                    "type": "string",
                                    "description": "CSS selector"
                                }
                            },
                            "required": ["selector"]
                        }
                    }
                ],
                "nextCursor": "more-tools"
            }),
        );

        let result = ToolsList::parse_response(response).unwrap();

        assert_eq!(result.tools.len(), 2);
        assert_eq!(result.tools[0].name, "browser_navigate");
        assert_eq!(
            result.tools[0].description,
            "Navigate to a URL in the browser"
        );
        assert!(result.tools[0].input_schema["properties"]["url"].is_object());
        assert_eq!(result.tools[1].name, "browser_click");
        assert_eq!(result.next_cursor, Some("more-tools".to_string()));
    }

    /// Test parsing tools/list response with empty tools
    #[test]
    fn test_tools_list_parse_empty_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "tools": []
            }),
        );

        let result = ToolsList::parse_response(response).unwrap();

        assert!(result.tools.is_empty());
        assert!(result.next_cursor.is_none());
    }

    /// Test parsing tools/list response without next cursor
    #[test]
    fn test_tools_list_parse_no_next_cursor() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "tools": [
                    {
                        "name": "simple_tool",
                        "description": "A simple tool",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    }
                ]
            }),
        );

        let result = ToolsList::parse_response(response).unwrap();

        assert_eq!(result.tools.len(), 1);
        assert!(result.next_cursor.is_none());
    }

    /// Test pagination cursor extraction via PaginatedMethod trait
    #[test]
    fn test_tools_list_pagination_cursor() {
        let result_with_cursor = ToolsListResult {
            tools: vec![],
            next_cursor: Some("cursor-value".to_string()),
        };
        assert_eq!(
            ToolsList::next_cursor(&result_with_cursor),
            Some("cursor-value")
        );

        let result_without_cursor = ToolsListResult {
            tools: vec![],
            next_cursor: None,
        };
        assert_eq!(ToolsList::next_cursor(&result_without_cursor), None);
    }

    /// Test Tool struct creation
    #[test]
    fn test_tool_creation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "url": { "type": "string" }
            }
        });
        let tool = Tool::new("test_tool", "A test tool", schema.clone());

        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, "A test tool");
        assert_eq!(tool.input_schema, schema);
    }

    /// Test tools/list error response
    #[test]
    fn test_tools_list_error_response() {
        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            ltmatrix::mcp::protocol::errors::JsonRpcError::method_not_found("tools/list"),
        );

        let result = ToolsList::parse_response(response);
        assert!(result.is_err());
    }

    /// Test various tool input schemas
    #[test]
    fn test_various_tool_schemas() {
        let schemas = vec![
            // Simple string parameter
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
            // Complex nested schema
            json!({
                "type": "object",
                "properties": {
                    "config": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "options": {
                                "type": "array",
                                "items": { "type": "string" }
                            }
                        }
                    }
                }
            }),
            // Schema with enums
            json!({
                "type": "object",
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["fast", "normal", "thorough"]
                    }
                }
            }),
        ];

        for (i, schema) in schemas.into_iter().enumerate() {
            let response = JsonRpcResponse::success(
                RequestId::Number(i as i64),
                json!({
                    "tools": [{
                        "name": format!("tool_{}", i),
                        "description": "Test tool",
                        "inputSchema": schema
                    }]
                }),
            );

            let result = ToolsList::parse_response(response).unwrap();
            assert_eq!(result.tools.len(), 1);
        }
    }
}

// ============================================================================
// Tools Call Method Tests
// ============================================================================

mod tools_call_method_tests {
    use super::*;

    /// Test that ToolsCall method has correct method name
    #[test]
    fn test_tools_call_method_name() {
        assert_eq!(ToolsCall::METHOD_NAME, "tools/call");
    }

    /// Test building tools/call request without arguments
    #[test]
    fn test_tools_call_build_request_no_args() {
        let params = ToolCallParams::new("get_status");
        let request = ToolsCall::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "tools/call");
        let params_value = request.params.unwrap();
        assert_eq!(params_value["name"], "get_status");
        assert!(params_value.get("arguments").is_none() || params_value["arguments"].is_null());
    }

    /// Test building tools/call request with arguments
    #[test]
    fn test_tools_call_build_request_with_args() {
        let params = ToolCallParams::new("browser_navigate").with_arguments(json!({
            "url": "https://example.com"
        }));
        let request = ToolsCall::build_request(RequestId::Number(1), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["name"], "browser_navigate");
        assert_eq!(params_value["arguments"]["url"], "https://example.com");
    }

    /// Test ToolsCall::params helper
    #[test]
    fn test_tools_call_params_helper() {
        let params = ToolsCall::params("my_tool");
        assert_eq!(params.name, "my_tool");
        assert!(params.arguments.is_none());
    }

    /// Test ToolsCall::params_with_args helper
    #[test]
    fn test_tools_call_params_with_args_helper() {
        let params = ToolsCall::params_with_args(
            "browser_click",
            json!({
                "selector": "#submit-button"
            }),
        );
        assert_eq!(params.name, "browser_click");
        assert_eq!(params.arguments.unwrap()["selector"], "#submit-button");
    }

    /// Test parsing successful tools/call response with text content
    #[test]
    fn test_tools_call_parse_text_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": "Successfully navigated to https://example.com"
                    }
                ],
                "isError": false
            }),
        );

        let result = ToolsCall::parse_response(response).unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            ToolContent::Text { text } => {
                assert_eq!(text, "Successfully navigated to https://example.com");
            }
            _ => panic!("Expected text content"),
        }
    }

    /// Test parsing tools/call response with error
    #[test]
    fn test_tools_call_parse_error_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": "Error: Invalid URL provided"
                    }
                ],
                "isError": true
            }),
        );

        let result = ToolsCall::parse_response(response).unwrap();

        assert!(result.is_error);
        assert_eq!(result.content.len(), 1);
    }

    /// Test parsing tools/call response with image content
    #[test]
    fn test_tools_call_parse_image_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [
                    {
                        "type": "image",
                        "data": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJ",
                        "mime_type": "image/png"
                    }
                ],
                "isError": false
            }),
        );

        let result = ToolsCall::parse_response(response).unwrap();

        assert!(!result.is_error);
        match &result.content[0] {
            ToolContent::Image { data, mime_type } => {
                assert_eq!(data, "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJ");
                assert_eq!(mime_type, "image/png");
            }
            _ => panic!("Expected image content"),
        }
    }

    /// Test parsing tools/call response with resource reference
    #[test]
    fn test_tools_call_parse_resource_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [
                    {
                        "type": "resource",
                        "uri": "file:///screenshot.png",
                        "mime_type": "image/png"
                    }
                ],
                "isError": false
            }),
        );

        let result = ToolsCall::parse_response(response).unwrap();

        assert!(!result.is_error);
        match &result.content[0] {
            ToolContent::Resource { uri, .. } => {
                assert_eq!(uri, "file:///screenshot.png");
            }
            _ => panic!("Expected resource content"),
        }
    }

    /// Test parsing tools/call response with multiple content items
    #[test]
    fn test_tools_call_parse_multiple_content() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": "Page loaded successfully"
                    },
                    {
                        "type": "image",
                        "data": "base64screenshot",
                        "mime_type": "image/png"
                    }
                ],
                "isError": false
            }),
        );

        let result = ToolsCall::parse_response(response).unwrap();

        assert_eq!(result.content.len(), 2);
        assert!(matches!(result.content[0], ToolContent::Text { .. }));
        assert!(matches!(result.content[1], ToolContent::Image { .. }));
    }

    /// Test ToolCallResult helper methods
    #[test]
    fn test_tool_call_result_helpers() {
        // Test text() helper
        let result = ToolCallResult::text("Success!");
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);

        // Test error() helper
        let error_result = ToolCallResult::error("Failed!");
        assert!(error_result.is_error);
        assert_eq!(error_result.content.len(), 1);
    }

    /// Test ToolContent helper methods
    #[test]
    fn test_tool_content_helpers() {
        // Test text() helper
        let text_content = ToolContent::text("Hello");
        match text_content {
            ToolContent::Text { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected text content"),
        }

        // Test image() helper
        let image_content = ToolContent::image("base64data", "image/jpeg");
        match image_content {
            ToolContent::Image { data, mime_type } => {
                assert_eq!(data, "base64data");
                assert_eq!(mime_type, "image/jpeg");
            }
            _ => panic!("Expected image content"),
        }
    }

    /// Test tools/call with complex arguments
    #[test]
    fn test_tools_call_complex_arguments() {
        let complex_args = json!({
            "options": {
                "timeout": 30000,
                "retries": 3,
                "headers": {
                    "Authorization": "Bearer token123",
                    "Content-Type": "application/json"
                }
            },
            "data": {
                "name": "test",
                "values": [1, 2, 3, 4, 5]
            }
        });

        let params = ToolCallParams::new("api_request").with_arguments(complex_args.clone());
        let request = ToolsCall::build_request(RequestId::Number(1), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["arguments"]["options"]["timeout"], 30000);
        assert_eq!(
            params_value["arguments"]["data"]["values"]
                .as_array()
                .unwrap()
                .len(),
            5
        );
    }
}

// ============================================================================
// Parameter Validation Tests
// ============================================================================

mod parameter_validation_tests {
    use super::*;

    /// Test tool name validation - valid names
    #[test]
    fn test_valid_tool_names() {
        let valid_names = vec![
            "simple_tool",
            "browser_navigate",
            "api-call",
            "tool123",
            "UPPERCASE_TOOL",
            "mixedCaseTool",
            "a",
            "tool_with_underscores_and_numbers_123",
        ];

        for name in valid_names {
            let params = ToolCallParams::new(name);
            let request = ToolsCall::build_request(RequestId::Number(1), params);
            assert_eq!(request.params.unwrap()["name"], name);
        }
    }

    /// Test argument validation - empty object
    #[test]
    fn test_empty_arguments() {
        let params = ToolCallParams::new("test_tool").with_arguments(json!({}));

        let request = ToolsCall::build_request(RequestId::Number(1), params);
        let params_value = request.params.unwrap();

        assert!(params_value["arguments"].is_object());
        assert_eq!(params_value["arguments"].as_object().unwrap().len(), 0);
    }

    /// Test argument validation - nested structures
    #[test]
    fn test_nested_arguments() {
        let params = ToolCallParams::new("test_tool").with_arguments(json!({
            "level1": {
                "level2": {
                    "level3": {
                        "value": "deep"
                    }
                }
            }
        }));

        let request = ToolsCall::build_request(RequestId::Number(1), params);
        let params_value = request.params.unwrap();

        assert_eq!(
            params_value["arguments"]["level1"]["level2"]["level3"]["value"],
            "deep"
        );
    }

    /// Test argument validation - arrays
    #[test]
    fn test_array_arguments() {
        let params = ToolCallParams::new("batch_tool").with_arguments(json!({
            "items": ["item1", "item2", "item3"],
            "count": 3
        }));

        let request = ToolsCall::build_request(RequestId::Number(1), params);
        let params_value = request.params.unwrap();

        let items = params_value["arguments"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 3);
    }

    /// Test argument validation - various types
    #[test]
    fn test_various_argument_types() {
        let params = ToolCallParams::new("multi_type_tool").with_arguments(json!({
            "string_val": "text",
            "number_val": 42,
            "float_val": 3.14,
            "bool_val": true,
            "null_val": null,
            "array_val": [1, "two", true],
            "object_val": {"key": "value"}
        }));

        let request = ToolsCall::build_request(RequestId::Number(1), params);
        let params_value = request.params.unwrap();
        let args = &params_value["arguments"];

        assert_eq!(args["string_val"], "text");
        assert_eq!(args["number_val"], 42);
        assert_eq!(args["float_val"], 3.14);
        assert_eq!(args["bool_val"], true);
        assert!(args["null_val"].is_null());
        assert!(args["array_val"].is_array());
        assert!(args["object_val"].is_object());
    }
}

// ============================================================================
// Result Formatting Tests
// ============================================================================

mod result_formatting_tests {
    use super::*;

    /// Test extracting text from successful result
    #[test]
    fn test_extract_text_from_result() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [
                    { "type": "text", "text": "Line 1" },
                    { "type": "text", "text": "Line 2" }
                ],
                "isError": false
            }),
        );

        let result = ToolsCall::parse_response(response).unwrap();

        let texts: Vec<String> = result
            .content
            .iter()
            .filter_map(|c| match c {
                ToolContent::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(texts.len(), 2);
        assert_eq!(texts[0], "Line 1");
        assert_eq!(texts[1], "Line 2");
    }

    /// Test checking for error in result
    #[test]
    fn test_check_error_in_result() {
        let success_response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [{ "type": "text", "text": "OK" }],
                "isError": false
            }),
        );

        let error_response = JsonRpcResponse::success(
            RequestId::Number(2),
            json!({
                "content": [{ "type": "text", "text": "Failed" }],
                "isError": true
            }),
        );

        let success_result = ToolsCall::parse_response(success_response).unwrap();
        let error_result = ToolsCall::parse_response(error_response).unwrap();

        assert!(!success_result.is_error);
        assert!(error_result.is_error);
    }

    /// Test extracting images from result
    #[test]
    fn test_extract_images_from_result() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [
                    { "type": "text", "text": "Screenshot taken:" },
                    { "type": "image", "data": "img1", "mime_type": "image/png" },
                    { "type": "image", "data": "img2", "mime_type": "image/jpeg" }
                ],
                "isError": false
            }),
        );

        let result = ToolsCall::parse_response(response).unwrap();

        let images: Vec<(&str, &str)> = result
            .content
            .iter()
            .filter_map(|c| match c {
                ToolContent::Image { data, mime_type } => Some((data.as_str(), mime_type.as_str())),
                _ => None,
            })
            .collect();

        assert_eq!(images.len(), 2);
        assert_eq!(images[0], ("img1", "image/png"));
        assert_eq!(images[1], ("img2", "image/jpeg"));
    }

    /// Test extracting resource references from result
    #[test]
    fn test_extract_resources_from_result() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [
                    {
                        "type": "resource",
                        "uri": "file:///output1.txt",
                        "mime_type": "text/plain"
                    },
                    {
                        "type": "resource",
                        "uri": "file:///output2.json",
                        "mime_type": "application/json"
                    }
                ],
                "isError": false
            }),
        );

        let result = ToolsCall::parse_response(response).unwrap();

        let uris: Vec<&str> = result
            .content
            .iter()
            .filter_map(|c| match c {
                ToolContent::Resource { uri, .. } => Some(uri.as_str()),
                _ => None,
            })
            .collect();

        assert_eq!(uris.len(), 2);
        assert_eq!(uris[0], "file:///output1.txt");
        assert_eq!(uris[1], "file:///output2.json");
    }

    /// Test mixed content result
    #[test]
    fn test_mixed_content_result() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [
                    { "type": "text", "text": "Processing complete" },
                    { "type": "image", "data": "screenshot", "mime_type": "image/png" },
                    { "type": "text", "text": "See attached image" },
                    {
                        "type": "resource",
                        "uri": "file:///report.html",
                        "mime_type": "text/html"
                    }
                ],
                "isError": false
            }),
        );

        let result = ToolsCall::parse_response(response).unwrap();

        assert_eq!(result.content.len(), 4);

        // Count content types
        let text_count = result
            .content
            .iter()
            .filter(|c| matches!(c, ToolContent::Text { .. }))
            .count();
        let image_count = result
            .content
            .iter()
            .filter(|c| matches!(c, ToolContent::Image { .. }))
            .count();
        let resource_count = result
            .content
            .iter()
            .filter(|c| matches!(c, ToolContent::Resource { .. }))
            .count();

        assert_eq!(text_count, 2);
        assert_eq!(image_count, 1);
        assert_eq!(resource_count, 1);
    }
}

// ============================================================================
// Method Kind Registry Tests
// ============================================================================

mod tools_method_kind_tests {
    use super::*;

    /// Test that ToolsList is recognized as a known method
    #[test]
    fn test_tools_list_method_kind() {
        assert_eq!(
            McpMethodKind::from_method_name("tools/list"),
            Some(McpMethodKind::ToolsList)
        );
        assert_eq!(McpMethodKind::ToolsList.method_name(), "tools/list");
    }

    /// Test that ToolsCall is recognized as a known method
    #[test]
    fn test_tools_call_method_kind() {
        assert_eq!(
            McpMethodKind::from_method_name("tools/call"),
            Some(McpMethodKind::ToolsCall)
        );
        assert_eq!(McpMethodKind::ToolsCall.method_name(), "tools/call");
    }
}

// ============================================================================
// Integration-like Tests (without actual transport)
// ============================================================================

mod integration_like_tests {
    use super::*;

    /// Simulate a full tools/list flow
    #[test]
    fn test_simulated_tools_list_flow() {
        // 1. Client builds request
        let params = ToolsList::params();
        let request = ToolsList::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "tools/list");

        // 2. Server responds with tools
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "tools": [
                    {
                        "name": "browser_navigate",
                        "description": "Navigate browser",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "url": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "browser_click",
                        "description": "Click element",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "selector": { "type": "string" }
                            }
                        }
                    }
                ],
                "nextCursor": "more"
            }),
        );

        // 3. Client parses response
        let result = ToolsList::parse_response(response).unwrap();

        assert_eq!(result.tools.len(), 2);
        assert_eq!(result.next_cursor, Some("more".to_string()));

        // 4. Client fetches next page
        let params_page2 = ToolsList::params_with_cursor("more");
        let request_page2 = ToolsList::build_request(RequestId::Number(2), params_page2);

        let params_value = request_page2.params.unwrap();
        assert_eq!(params_value["cursor"], "more");
    }

    /// Simulate a full tools/call flow
    #[test]
    fn test_simulated_tools_call_flow() {
        // 1. Client builds request
        let params = ToolsCall::params_with_args(
            "browser_navigate",
            json!({
                "url": "https://example.com"
            }),
        );
        let request = ToolsCall::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "tools/call");
        let params_value = request.params.unwrap();
        assert_eq!(params_value["name"], "browser_navigate");
        assert_eq!(params_value["arguments"]["url"], "https://example.com");

        // 2. Server responds with result
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": "Successfully navigated to https://example.com"
                    }
                ],
                "isError": false
            }),
        );

        // 3. Client parses and uses result
        let result = ToolsCall::parse_response(response).unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);

        // 4. Extract text from result
        let text = match &result.content[0] {
            ToolContent::Text { text } => text.clone(),
            _ => panic!("Expected text"),
        };

        assert!(text.contains("Successfully navigated"));
    }

    /// Simulate tool call with error response
    #[test]
    fn test_simulated_tools_call_error_flow() {
        // 1. Client calls a tool that will fail
        let params = ToolsCall::params_with_args(
            "browser_navigate",
            json!({
                "url": "not-a-valid-url"
            }),
        );
        let request = ToolsCall::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "tools/call");

        // 2. Server responds with error
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": "Error: Invalid URL format"
                    }
                ],
                "isError": true
            }),
        );

        // 3. Client parses result and detects error
        let result = ToolsCall::parse_response(response).unwrap();

        assert!(result.is_error);

        // 4. Extract error message
        let error_text = match &result.content[0] {
            ToolContent::Text { text } => text.clone(),
            _ => panic!("Expected text"),
        };

        assert!(error_text.contains("Error"));
    }

    /// Test method kind detection for routing
    #[test]
    fn test_method_routing_detection() {
        let methods = vec![
            ("tools/list", McpMethodKind::ToolsList),
            ("tools/call", McpMethodKind::ToolsCall),
        ];

        for (method_name, expected_kind) in methods {
            let detected = McpMethodKind::from_method_name(method_name);
            assert_eq!(detected, Some(expected_kind));

            // Verify round-trip
            assert_eq!(detected.unwrap().method_name(), method_name);
        }
    }
}

// ============================================================================
// Serialization/Deserialization Edge Cases
// ============================================================================

mod serialization_edge_cases {
    use super::*;

    /// Test handling of unicode in tool names
    #[test]
    fn test_unicode_in_tool_description() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "tools": [{
                    "name": "unicode_tool",
                    "description": "工具描述 - Tool description with 中文 and emoji 🎉",
                    "inputSchema": { "type": "object" }
                }]
            }),
        );

        let result = ToolsList::parse_response(response).unwrap();
        assert!(result.tools[0].description.contains("中文"));
        assert!(result.tools[0].description.contains("🎉"));
    }

    /// Test handling of special characters in tool output
    #[test]
    fn test_special_chars_in_tool_output() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [{
                    "type": "text",
                    "text": "Special chars: <>&\"'\\n\\t\\r and more!"
                }],
                "isError": false
            }),
        );

        let result = ToolsCall::parse_response(response).unwrap();
        let text = match &result.content[0] {
            ToolContent::Text { text } => text.as_str(),
            _ => panic!("Expected text"),
        };

        assert!(text.contains("<>&\"'"));
    }

    /// Test handling of large tool output (10KB)
    #[test]
    fn test_large_tool_output() {
        let large_text = "x".repeat(10_000);

        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [{
                    "type": "text",
                    "text": large_text
                }],
                "isError": false
            }),
        );

        let result = ToolsCall::parse_response(response).unwrap();
        let text = match &result.content[0] {
            ToolContent::Text { text } => text.clone(),
            _ => panic!("Expected text"),
        };

        assert_eq!(text.len(), 10_000);
    }

    /// Test handling of many tools in list
    #[test]
    fn test_many_tools_in_list() {
        let tools: Vec<serde_json::Value> = (0..100)
            .map(|i| {
                json!({
                    "name": format!("tool_{}", i),
                    "description": format!("Tool number {}", i),
                    "inputSchema": { "type": "object" }
                })
            })
            .collect();

        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "tools": tools
            }),
        );

        let result = ToolsList::parse_response(response).unwrap();
        assert_eq!(result.tools.len(), 100);
    }

    /// Test request ID variations
    #[test]
    fn test_request_id_variations() {
        let ids = vec![
            RequestId::Number(1),
            RequestId::Number(i64::MAX),
            RequestId::String("uuid-1234".to_string()),
            RequestId::String("tool-call-abc".to_string()),
        ];

        for id in ids {
            let params = ToolsCall::params("test_tool");
            let request = ToolsCall::build_request(id.clone(), params);
            assert_eq!(request.id, id);
        }
    }
}

// ============================================================================
// JSON-RPC Message Format Tests
// ============================================================================

mod jsonrpc_format_tests {
    use super::*;

    /// Test that requests have correct JSON-RPC version
    #[test]
    fn test_jsonrpc_version_in_requests() {
        let params = ToolsCall::params("test_tool");
        let request = ToolsCall::build_request(RequestId::Number(1), params);

        assert_eq!(request.jsonrpc, "2.0");
    }

    /// Test request serialization format
    #[test]
    fn test_request_serialization_format() {
        let params = ToolsCall::params_with_args("browser_click", json!({"selector": "#btn"}));
        let request = ToolsCall::build_request(RequestId::Number(42), params);

        let json = serde_json::to_string(&request).unwrap();

        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"method\":\"tools/call\""));
        assert!(json.contains("\"name\":\"browser_click\""));
        assert!(json.contains("\"selector\":\"#btn\""));
    }

    /// Test response serialization format
    #[test]
    fn test_response_serialization_format() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "content": [{ "type": "text", "text": "OK" }],
                "isError": false
            }),
        );

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"result\""));
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

mod error_handling_tests {
    use super::*;

    /// Test error response parsing
    #[test]
    fn test_error_response_parsing() {
        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            ltmatrix::mcp::protocol::errors::JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
                data: Some(json!({ "details": "Missing required argument" })),
            },
        );

        let result = ToolsCall::parse_response(response);
        assert!(result.is_err());
    }

    /// Test tool not found error
    #[test]
    fn test_tool_not_found_error() {
        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            ltmatrix::mcp::protocol::errors::JsonRpcError::method_not_found("tools/call"),
        );

        let result = ToolsCall::parse_response(response);
        assert!(result.is_err());
    }

    /// Test parse response with malformed JSON
    #[test]
    fn test_parse_response_malformed_json() {
        let result: Result<ToolCallResult, _> = serde_json::from_str("{invalid json}");
        assert!(result.is_err());
    }

    /// Test parse response with missing required fields
    #[test]
    fn test_parse_response_missing_fields() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                // Missing "content" field
                "isError": false
            }),
        );

        let result = ToolsCall::parse_response(response);
        assert!(result.is_err());
    }
}

// ============================================================================
// Client Helper Method Tests
// ============================================================================

mod client_helper_tests {
    use super::*;

    /// Test extract_text_from_result with text content
    #[test]
    fn test_extract_text_from_result_single() {
        let result = ToolCallResult {
            content: vec![ToolContent::text("Hello, World!")],
            is_error: false,
        };

        let text = McpClient::extract_text_from_result(&result);
        assert_eq!(text, "Hello, World!");
    }

    /// Test extract_text_from_result with multiple text items
    #[test]
    fn test_extract_text_from_result_multiple() {
        let result = ToolCallResult {
            content: vec![
                ToolContent::text("Line 1"),
                ToolContent::text("Line 2"),
                ToolContent::text("Line 3"),
            ],
            is_error: false,
        };

        let text = McpClient::extract_text_from_result(&result);
        assert_eq!(text, "Line 1\nLine 2\nLine 3");
    }

    /// Test extract_text_from_result with mixed content
    #[test]
    fn test_extract_text_from_result_mixed() {
        let result = ToolCallResult {
            content: vec![
                ToolContent::text("Text before"),
                ToolContent::image("base64data", "image/png"),
                ToolContent::text("Text after"),
            ],
            is_error: false,
        };

        let text = McpClient::extract_text_from_result(&result);
        assert_eq!(text, "Text before\nText after");
    }

    /// Test extract_text_from_result with no text content
    #[test]
    fn test_extract_text_from_result_no_text() {
        let result = ToolCallResult {
            content: vec![
                ToolContent::image("data", "image/png"),
                ToolContent::resource("file:///test", None),
            ],
            is_error: false,
        };

        let text = McpClient::extract_text_from_result(&result);
        assert_eq!(text, "");
    }

    /// Test extract_images_from_result with images
    #[test]
    fn test_extract_images_from_result() {
        let result = ToolCallResult {
            content: vec![
                ToolContent::image("img1", "image/png"),
                ToolContent::text("separator"),
                ToolContent::image("img2", "image/jpeg"),
            ],
            is_error: false,
        };

        let images = McpClient::extract_images_from_result(&result);
        assert_eq!(images.len(), 2);
        assert_eq!(images[0], ("img1".to_string(), "image/png".to_string()));
        assert_eq!(images[1], ("img2".to_string(), "image/jpeg".to_string()));
    }

    /// Test extract_images_from_result with no images
    #[test]
    fn test_extract_images_from_result_none() {
        let result = ToolCallResult {
            content: vec![ToolContent::text("No images here")],
            is_error: false,
        };

        let images = McpClient::extract_images_from_result(&result);
        assert!(images.is_empty());
    }

    /// Test extract_resources_from_result with resources
    #[test]
    fn test_extract_resources_from_result() {
        let result = ToolCallResult {
            content: vec![
                ToolContent::resource("file:///output.txt", Some("text/plain".to_string())),
                ToolContent::text("separator"),
                ToolContent::resource("file:///data.json", Some("application/json".to_string())),
                ToolContent::resource("file:///unknown", None),
            ],
            is_error: false,
        };

        let resources = McpClient::extract_resources_from_result(&result);
        assert_eq!(resources.len(), 3);
        assert_eq!(
            resources[0],
            (
                "file:///output.txt".to_string(),
                Some("text/plain".to_string())
            )
        );
        assert_eq!(
            resources[1],
            (
                "file:///data.json".to_string(),
                Some("application/json".to_string())
            )
        );
        assert_eq!(resources[2], ("file:///unknown".to_string(), None));
    }

    /// Test is_result_error with explicit error flag
    #[test]
    fn test_is_result_error_explicit() {
        let result = ToolCallResult {
            content: vec![ToolContent::text("Success")],
            is_error: true,
        };

        assert!(McpClient::is_result_error(&result));
    }

    /// Test is_result_error with error text content
    #[test]
    fn test_is_result_error_text_content() {
        let result = ToolCallResult {
            content: vec![ToolContent::text("An error occurred")],
            is_error: false,
        };

        assert!(McpClient::is_result_error(&result));
    }

    /// Test is_result_error with "failed" text
    #[test]
    fn test_is_result_error_failed_text() {
        let result = ToolCallResult {
            content: vec![ToolContent::text("Operation failed")],
            is_error: false,
        };

        assert!(McpClient::is_result_error(&result));
    }

    /// Test is_result_error with "exception" text
    #[test]
    fn test_is_result_error_exception_text() {
        let result = ToolCallResult {
            content: vec![ToolContent::text("NullPointerException thrown")],
            is_error: false,
        };

        assert!(McpClient::is_result_error(&result));
    }

    /// Test is_result_error with case-insensitive matching
    #[test]
    fn test_is_result_error_case_insensitive() {
        let result = ToolCallResult {
            content: vec![ToolContent::text("ERROR: Something went wrong")],
            is_error: false,
        };

        assert!(McpClient::is_result_error(&result));

        let result2 = ToolCallResult {
            content: vec![ToolContent::text("FAILED to complete")],
            is_error: false,
        };

        assert!(McpClient::is_result_error(&result2));
    }

    /// Test is_result_error with successful result
    #[test]
    fn test_is_result_error_success() {
        let result = ToolCallResult {
            content: vec![ToolContent::text("Operation completed successfully")],
            is_error: false,
        };

        assert!(!McpClient::is_result_error(&result));
    }

    /// Test result_content_summary with various content types
    #[test]
    fn test_result_content_summary() {
        let result = ToolCallResult {
            content: vec![
                ToolContent::text("Text 1"),
                ToolContent::image("img1", "image/png"),
                ToolContent::text("Text 2"),
                ToolContent::resource("file:///res", None),
                ToolContent::image("img2", "image/jpeg"),
            ],
            is_error: false,
        };

        let (text_count, image_count, resource_count) = McpClient::result_content_summary(&result);

        assert_eq!(text_count, 2);
        assert_eq!(image_count, 2);
        assert_eq!(resource_count, 1);
    }

    /// Test result_content_summary with empty content
    #[test]
    fn test_result_content_summary_empty() {
        let result = ToolCallResult {
            content: vec![],
            is_error: false,
        };

        let (text_count, image_count, resource_count) = McpClient::result_content_summary(&result);

        assert_eq!(text_count, 0);
        assert_eq!(image_count, 0);
        assert_eq!(resource_count, 0);
    }

    /// Test result_content_summary with single type content
    #[test]
    fn test_result_content_summary_single_type() {
        let text_only = ToolCallResult {
            content: vec![
                ToolContent::text("A"),
                ToolContent::text("B"),
                ToolContent::text("C"),
            ],
            is_error: false,
        };

        let (text_count, image_count, resource_count) =
            McpClient::result_content_summary(&text_only);

        assert_eq!(text_count, 3);
        assert_eq!(image_count, 0);
        assert_eq!(resource_count, 0);
    }
}

// ============================================================================
// ToolsCapability Tests
// ============================================================================

mod tools_capability_tests {
    use super::*;

    /// Test ToolsCapability default
    #[test]
    fn test_tools_capability_default() {
        let cap = ToolsCapability::default();
        assert!(cap.list_changed.is_none());
    }

    /// Test ToolsCapability with list_changed
    #[test]
    fn test_tools_capability_with_list_changed() {
        let cap = ToolsCapability {
            list_changed: Some(true),
        };

        assert_eq!(cap.list_changed, Some(true));
    }

    /// Test ToolsCapability serialization
    #[test]
    fn test_tools_capability_serialization() {
        let cap = ToolsCapability {
            list_changed: Some(true),
        };

        let json = serde_json::to_string(&cap).unwrap();
        assert!(json.contains("listChanged"));
    }

    /// Test ToolsCapability deserialization
    #[test]
    fn test_tools_capability_deserialization() {
        let json = json!({ "listChanged": false });
        let cap: ToolsCapability = serde_json::from_value(json).unwrap();

        assert_eq!(cap.list_changed, Some(false));
    }

    /// Test ToolsCapability empty deserialization
    #[test]
    fn test_tools_capability_empty_deserialization() {
        let json = json!({});
        let cap: ToolsCapability = serde_json::from_value(json).unwrap();

        assert!(cap.list_changed.is_none());
    }
}

// ============================================================================
// Tool Struct Advanced Tests
// ============================================================================

mod tool_struct_advanced_tests {
    use super::*;

    /// Test Tool with complex input schema
    #[test]
    fn test_tool_complex_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "options": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "nested": {
                            "type": "object",
                            "properties": {
                                "deep": { "type": "boolean" }
                            }
                        }
                    },
                    "required": ["name"]
                },
                "mode": {
                    "type": "string",
                    "enum": ["fast", "normal", "thorough"]
                }
            },
            "required": ["config"]
        });

        let tool = Tool::new("complex_tool", "A complex tool", schema.clone());

        assert_eq!(tool.name, "complex_tool");
        assert_eq!(tool.description, "A complex tool");
        assert!(tool.input_schema["properties"]["config"]["properties"]["nested"].is_object());
    }

    /// Test Tool serialization round-trip
    #[test]
    fn test_tool_serialization_roundtrip() {
        let original = Tool::new(
            "test_tool",
            "Test tool description",
            json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string" }
                },
                "required": ["url"]
            }),
        );

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Tool = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, original.name);
        assert_eq!(deserialized.description, original.description);
        assert_eq!(deserialized.input_schema, original.input_schema);
    }

    /// Test Tool with empty description
    #[test]
    fn test_tool_empty_description() {
        let tool = Tool::new("minimal_tool", "", json!({"type": "object"}));

        assert_eq!(tool.name, "minimal_tool");
        assert_eq!(tool.description, "");
    }

    /// Test Tool with very long description
    #[test]
    fn test_tool_long_description() {
        let long_desc = "x".repeat(1000);
        let tool = Tool::new("tool", &long_desc, json!({"type": "object"}));

        assert_eq!(tool.description.len(), 1000);
    }
}

// ============================================================================
// Edge Cases and Boundary Conditions
// ============================================================================

mod edge_case_tests {
    use super::*;

    /// Test ToolCallParams with empty tool name
    #[test]
    fn test_tool_call_params_empty_name() {
        let params = ToolCallParams::new("");
        assert_eq!(params.name, "");
        assert!(params.arguments.is_none());
    }

    /// Test ToolCallParams with null arguments
    #[test]
    fn test_tool_call_params_null_arguments() {
        let params = ToolCallParams::new("test").with_arguments(json!(null));

        assert!(params.arguments.is_some());
        assert!(params.arguments.unwrap().is_null());
    }

    /// Test ToolCallResult with empty content
    #[test]
    fn test_tool_call_result_empty_content() {
        let result = ToolCallResult {
            content: vec![],
            is_error: false,
        };

        assert!(result.content.is_empty());
        assert!(!result.is_error);

        // Helper methods should handle empty content
        assert_eq!(McpClient::extract_text_from_result(&result), "");
        assert!(McpClient::extract_images_from_result(&result).is_empty());
        assert!(McpClient::extract_resources_from_result(&result).is_empty());

        let (text, img, res) = McpClient::result_content_summary(&result);
        assert_eq!((text, img, res), (0, 0, 0));
    }

    /// Test ToolContent with empty text
    #[test]
    fn test_tool_content_empty_text() {
        let content = ToolContent::text("");
        match content {
            ToolContent::Text { text } => assert!(text.is_empty()),
            _ => panic!("Expected text content"),
        }
    }

    /// Test ToolContent with empty image data
    #[test]
    fn test_tool_content_empty_image_data() {
        let content = ToolContent::image("", "image/png");
        match content {
            ToolContent::Image { data, mime_type } => {
                assert!(data.is_empty());
                assert_eq!(mime_type, "image/png");
            }
            _ => panic!("Expected image content"),
        }
    }

    /// Test ToolContent::resource with optional mime_type
    #[test]
    fn test_tool_content_resource_with_mime() {
        let content = ToolContent::resource("file:///test.txt", Some("text/plain".to_string()));
        match content {
            ToolContent::Resource { uri, mime_type } => {
                assert_eq!(uri, "file:///test.txt");
                assert_eq!(mime_type, Some("text/plain".to_string()));
            }
            _ => panic!("Expected resource content"),
        }

        let content_no_mime = ToolContent::resource("file:///test", None);
        match content_no_mime {
            ToolContent::Resource { uri, mime_type } => {
                assert_eq!(uri, "file:///test");
                assert_eq!(mime_type, None);
            }
            _ => panic!("Expected resource content"),
        }
    }

    /// Test ToolsListParams with empty cursor
    #[test]
    fn test_tools_list_params_empty_cursor() {
        let params = ToolsListParams {
            cursor: Some("".to_string()),
        };

        assert_eq!(params.cursor, Some("".to_string()));
    }

    /// Test ToolsListResult with empty tools and cursor
    #[test]
    fn test_tools_list_result_empty() {
        let result = ToolsListResult {
            tools: vec![],
            next_cursor: None,
        };

        assert!(result.tools.is_empty());
        assert!(result.next_cursor.is_none());
    }

    /// Test handling of concurrent content types in same result
    #[test]
    fn test_concurrent_content_types() {
        let result = ToolCallResult {
            content: vec![
                ToolContent::text("Result: "),
                ToolContent::image("screenshot", "image/png"),
                ToolContent::text(" Above is the screenshot"),
                ToolContent::resource("file:///log.txt", Some("text/plain".to_string())),
                ToolContent::image("chart", "image/svg+xml"),
                ToolContent::resource("file:///data.json", None),
            ],
            is_error: false,
        };

        // Verify extraction
        let text = McpClient::extract_text_from_result(&result);
        assert!(text.contains("Result:"));
        assert!(text.contains("Above is the screenshot"));

        let images = McpClient::extract_images_from_result(&result);
        assert_eq!(images.len(), 2);

        let resources = McpClient::extract_resources_from_result(&result);
        assert_eq!(resources.len(), 2);

        let summary = McpClient::result_content_summary(&result);
        assert_eq!(summary, (2, 2, 2));
    }
}
