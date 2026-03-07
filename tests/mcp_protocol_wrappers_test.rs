// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Integration tests for type-safe MCP protocol method wrappers
//!
//! Tests for:
//! - McpMethod trait implementations for all method types
//! - PaginatedMethod trait for pagination support
//! - McpNotification trait for notification types
//! - McpMethodKind enum for method dispatch
//! - Type-safe request building and response parsing
//! - Error handling for malformed responses
//! - Serialization/deserialization roundtrips

use ltmatrix::mcp::{
    ClientCapabilities, InitializeParams, InitializeResult, ImplementationInfo, JsonRpcResponse,
    LogLevel, PromptMessage, PromptsGetParams, PromptsGetResult, PromptsListParams,
    PromptsListResult, RequestId, ResourceContents, ResourceReadParams, ResourceReadResult,
    ResourcesListParams, ResourcesListResult, Root, RootsListParams, RootsListResult,
    ServerCapabilities, Tool, ToolCallParams, ToolCallResult, ToolContent, ToolsListParams,
    ToolsListResult,
};
use ltmatrix::mcp::{
    CompletionArgument, CompletionComplete, CompletionCompleteParams,
    CompletionReference, Initialize, JsonRpcNotification,
    LogMessageParams, LoggingSetLevel, LoggingSetLevelParams,
    McpMethod, McpMethodKind, McpNotification, ModelHint, ModelPreferences,
    NotificationsInitialized, NotificationsMessage, NotificationsProgress,
    NotificationsPromptsListChanged, NotificationsResourcesListChanged, NotificationsRootsListChanged,
    NotificationsToolsListChanged, PaginatedMethod, Ping, PingParams, PingResult,
    ProgressParams, PromptsGet, PromptsList,
    ResourcesList, ResourcesRead, ResourcesSubscribe, ResourcesSubscribeParams,
    ResourcesUnsubscribe, ResourcesUnsubscribeParams,
    RootsList, SamplingContent, SamplingCreateMessage,
    SamplingCreateMessageParams, SamplingMessage, ToolsCall,
    ToolsList,
};
use ltmatrix::mcp::protocol::{McpError, McpResult};
use serde_json::json;

// ============================================================================
// McpMethod Trait Tests
// ============================================================================

mod mcp_method_trait_tests {
    use super::*;

    #[test]
    fn test_method_name_constant_initialize() {
        assert_eq!(Initialize::METHOD_NAME, "initialize");
    }

    #[test]
    fn test_method_name_constant_ping() {
        assert_eq!(Ping::METHOD_NAME, "ping");
    }

    #[test]
    fn test_method_name_constant_tools_list() {
        assert_eq!(ToolsList::METHOD_NAME, "tools/list");
    }

    #[test]
    fn test_method_name_constant_tools_call() {
        assert_eq!(ToolsCall::METHOD_NAME, "tools/call");
    }

    #[test]
    fn test_method_name_constant_resources_list() {
        assert_eq!(ResourcesList::METHOD_NAME, "resources/list");
    }

    #[test]
    fn test_method_name_constant_resources_read() {
        assert_eq!(ResourcesRead::METHOD_NAME, "resources/read");
    }

    #[test]
    fn test_method_name_constant_resources_subscribe() {
        assert_eq!(ResourcesSubscribe::METHOD_NAME, "resources/subscribe");
    }

    #[test]
    fn test_method_name_constant_resources_unsubscribe() {
        assert_eq!(ResourcesUnsubscribe::METHOD_NAME, "resources/unsubscribe");
    }

    #[test]
    fn test_method_name_constant_prompts_list() {
        assert_eq!(PromptsList::METHOD_NAME, "prompts/list");
    }

    #[test]
    fn test_method_name_constant_prompts_get() {
        assert_eq!(PromptsGet::METHOD_NAME, "prompts/get");
    }

    #[test]
    fn test_method_name_constant_roots_list() {
        assert_eq!(RootsList::METHOD_NAME, "roots/list");
    }

    #[test]
    fn test_method_name_constant_logging_set_level() {
        assert_eq!(LoggingSetLevel::METHOD_NAME, "logging/setLevel");
    }

    #[test]
    fn test_method_name_constant_completion_complete() {
        assert_eq!(CompletionComplete::METHOD_NAME, "completion/complete");
    }

    #[test]
    fn test_method_name_constant_sampling_create_message() {
        assert_eq!(SamplingCreateMessage::METHOD_NAME, "sampling/createMessage");
    }
}

// ============================================================================
// Request Building Tests
// ============================================================================

mod request_building_tests {
    use super::*;

    // ---- Initialize Method ----

    #[test]
    fn test_initialize_build_request_basic() {
        let params = Initialize::params("test-client", "1.0.0");
        let request = Initialize::build_request(RequestId::Number(1), params);

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, RequestId::Number(1));
        assert_eq!(request.method, "initialize");
        assert!(request.params.is_some());
    }

    #[test]
    fn test_initialize_build_request_with_capabilities() {
        let capabilities = ClientCapabilities::default();
        let params = Initialize::params_with_capabilities("ltmatrix", "0.1.0", capabilities);
        let request = Initialize::build_request(RequestId::String("init-req".into()), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["clientInfo"]["name"], "ltmatrix");
        assert_eq!(params_value["clientInfo"]["version"], "0.1.0");
    }

    #[test]
    fn test_initialize_request_serialization() {
        let params = Initialize::params("my-client", "2.0.0");
        let request = Initialize::build_request(RequestId::Number(42), params);
        let json = request.to_json().unwrap();

        assert!(json.contains("\"method\":\"initialize\""));
        assert!(json.contains("\"protocolVersion\""));
        assert!(json.contains("\"clientInfo\""));
    }

    // ---- Ping Method ----

    #[test]
    fn test_ping_build_request() {
        let params = PingParams::default();
        let request = Ping::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "ping");
    }

    #[test]
    fn test_ping_build_request_optional() {
        let request = Ping::build_request_optional(RequestId::Number(1), None);
        assert_eq!(request.method, "ping");
        assert!(request.params.is_none());
    }

    // ---- Tools List Method ----

    #[test]
    fn test_tools_list_build_request_no_cursor() {
        let params = ToolsList::params();
        let request = ToolsList::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "tools/list");
        // Default params should still serialize (even if empty object)
    }

    #[test]
    fn test_tools_list_build_request_with_cursor() {
        let params = ToolsList::params_with_cursor("next-page-token");
        let request = ToolsList::build_request(RequestId::Number(2), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["cursor"], "next-page-token");
    }

    // ---- Tools Call Method ----

    #[test]
    fn test_tools_call_build_request_no_args() {
        let params = ToolsCall::params("browser_navigate");
        let request = ToolsCall::build_request(RequestId::Number(1), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["name"], "browser_navigate");
        assert!(params_value.get("arguments").is_none());
    }

    #[test]
    fn test_tools_call_build_request_with_args() {
        let args = json!({"url": "https://example.com", "timeout": 30000});
        let params = ToolsCall::params_with_args("browser_navigate", args.clone());
        let request = ToolsCall::build_request(RequestId::String("tool-1".into()), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["name"], "browser_navigate");
        assert_eq!(params_value["arguments"]["url"], "https://example.com");
        assert_eq!(params_value["arguments"]["timeout"], 30000);
    }

    // ---- Resources List Method ----

    #[test]
    fn test_resources_list_build_request() {
        let params = ResourcesList::params();
        let request = ResourcesList::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "resources/list");
    }

    #[test]
    fn test_resources_list_build_request_with_cursor() {
        let params = ResourcesList::params_with_cursor("resource-cursor-123");
        let request = ResourcesList::build_request(RequestId::Number(1), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["cursor"], "resource-cursor-123");
    }

    // ---- Resources Read Method ----

    #[test]
    fn test_resources_read_build_request() {
        let params = ResourcesRead::params("file:///project/src/main.rs");
        let request = ResourcesRead::build_request(RequestId::Number(1), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["uri"], "file:///project/src/main.rs");
    }

    // ---- Resources Subscribe/Unsubscribe ----

    #[test]
    fn test_resources_subscribe_build_request() {
        let params = ResourcesSubscribeParams {
            uri: "file:///watched/file.txt".to_string(),
        };
        let request = ResourcesSubscribe::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "resources/subscribe");
        let params_value = request.params.unwrap();
        assert_eq!(params_value["uri"], "file:///watched/file.txt");
    }

    #[test]
    fn test_resources_unsubscribe_build_request() {
        let params = ResourcesUnsubscribeParams {
            uri: "file:///unwatched/file.txt".to_string(),
        };
        let request = ResourcesUnsubscribe::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "resources/unsubscribe");
        let params_value = request.params.unwrap();
        assert_eq!(params_value["uri"], "file:///unwatched/file.txt");
    }

    // ---- Prompts List Method ----

    #[test]
    fn test_prompts_list_build_request() {
        let params = PromptsList::params();
        let request = PromptsList::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "prompts/list");
    }

    #[test]
    fn test_prompts_list_build_request_with_cursor() {
        let params = PromptsList::params_with_cursor("prompt-cursor");
        let request = PromptsList::build_request(RequestId::Number(1), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["cursor"], "prompt-cursor");
    }

    // ---- Prompts Get Method ----

    #[test]
    fn test_prompts_get_build_request_no_args() {
        let params = PromptsGet::params("code_review");
        let request = PromptsGet::build_request(RequestId::Number(1), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["name"], "code_review");
    }

    #[test]
    fn test_prompts_get_build_request_with_args() {
        let params = PromptsGet::params_with_args("code_review", json!({"language": "rust"}));
        let request = PromptsGet::build_request(RequestId::Number(1), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["name"], "code_review");
        assert_eq!(params_value["arguments"]["language"], "rust");
    }

    // ---- Roots List Method ----

    #[test]
    fn test_roots_list_build_request() {
        let params = RootsListParams::default();
        let request = RootsList::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "roots/list");
    }

    // ---- Logging Set Level Method ----

    #[test]
    fn test_logging_set_level_build_request() {
        let params = LoggingSetLevel::params(LogLevel::Debug);
        let request = LoggingSetLevel::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "logging/setLevel");
        let params_value = request.params.unwrap();
        assert_eq!(params_value["level"], "debug");
    }

    #[test]
    fn test_logging_set_level_all_levels() {
        for level in [LogLevel::Debug, LogLevel::Info, LogLevel::Warning, LogLevel::Error] {
            let params = LoggingSetLevel::params(level);
            let request = LoggingSetLevel::build_request(RequestId::Number(1), params);
            let params_value = request.params.unwrap();
            let level_str = serde_json::to_string(&level).unwrap();
            assert_eq!(params_value["level"].to_string(), level_str);
        }
    }

    // ---- Completion Complete Method ----

    #[test]
    fn test_completion_complete_build_request() {
        let params = CompletionCompleteParams {
            r#ref: CompletionReference {
                ref_type: "ref/prompt".to_string(),
                uri: None,
                name: Some("code_review".to_string()),
            },
            argument: CompletionArgument {
                name: "language".to_string(),
                value: "ru".to_string(),
            },
        };
        let request = CompletionComplete::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "completion/complete");
    }

    // ---- Sampling Create Message Method ----

    #[test]
    fn test_sampling_create_message_build_request() {
        let params = SamplingCreateMessageParams {
            messages: vec![SamplingMessage {
                role: "user".to_string(),
                content: SamplingContent::Text {
                    text: "Hello".to_string(),
                },
            }],
            model_preferences: Some(ModelPreferences {
                hints: Some(vec![ModelHint {
                    name: Some("claude-3".to_string()),
                }]),
                cost_priority: None,
                speed_priority: Some(0.5),
                intelligence_priority: None,
            }),
            system_prompt: Some("You are a helpful assistant.".to_string()),
            include_context: None,
            temperature: Some(0.7),
            max_tokens: 1024,
            stop_sequences: None,
            metadata: None,
        };
        let request = SamplingCreateMessage::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "sampling/createMessage");
    }
}

// ============================================================================
// Response Parsing Tests
// ============================================================================

mod response_parsing_tests {
    use super::*;

    #[test]
    fn test_initialize_parse_response_success() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2025-11-25",
                "capabilities": {
                    "tools": { "listChanged": true },
                    "resources": { "subscribe": true, "listChanged": true }
                },
                "serverInfo": {
                    "name": "test-server",
                    "version": "1.0.0"
                },
                "instructions": "Welcome to the test server"
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result: McpResult<InitializeResult> = Initialize::parse_response(response);

        assert!(result.is_ok());
        let init_result = result.unwrap();
        assert_eq!(init_result.protocol_version, "2025-11-25");
        assert_eq!(init_result.server_info.name, "test-server");
        assert_eq!(init_result.server_info.version, "1.0.0");
        assert!(init_result.instructions.is_some());
    }

    #[test]
    fn test_initialize_parse_response_minimal() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "serverInfo": {
                    "name": "minimal-server",
                    "version": "0.1.0"
                }
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = Initialize::parse_response(response).unwrap();

        assert_eq!(result.protocol_version, "2025-11-25");
        assert!(result.instructions.is_none());
    }

    #[test]
    fn test_ping_parse_response() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {}
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = Ping::parse_response(response);

        assert!(result.is_ok());
    }

    #[test]
    fn test_tools_list_parse_response() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [
                    {
                        "name": "browser_navigate",
                        "description": "Navigate to a URL",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "url": { "type": "string" }
                            },
                            "required": ["url"]
                        }
                    },
                    {
                        "name": "browser_click",
                        "description": "Click an element",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "selector": { "type": "string" }
                            }
                        }
                    }
                ],
                "nextCursor": "page-2"
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ToolsList::parse_response(response).unwrap();

        assert_eq!(result.tools.len(), 2);
        assert_eq!(result.tools[0].name, "browser_navigate");
        assert_eq!(result.tools[1].name, "browser_click");
        assert_eq!(result.next_cursor, Some("page-2".to_string()));
    }

    #[test]
    fn test_tools_list_parse_response_no_cursor() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": []
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ToolsList::parse_response(response).unwrap();

        assert!(result.tools.is_empty());
        assert!(result.next_cursor.is_none());
    }

    #[test]
    fn test_tools_call_parse_response_text() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": "Successfully navigated to https://example.com"
                    }
                ],
                "isError": false
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ToolsCall::parse_response(response).unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            ToolContent::Text { text } => {
                assert!(text.contains("Successfully navigated"));
            }
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_tools_call_parse_response_error() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": "Failed to navigate: timeout"
                    }
                ],
                "isError": true
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ToolsCall::parse_response(response).unwrap();

        assert!(result.is_error);
    }

    #[test]
    fn test_tools_call_parse_response_image() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [
                    {
                        "type": "image",
                        "data": "base64imagedata",
                        "mime_type": "image/png"
                    }
                ],
                "isError": false
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ToolsCall::parse_response(response).unwrap();

        match &result.content[0] {
            ToolContent::Image { data, mime_type } => {
                assert_eq!(data, "base64imagedata");
                assert_eq!(mime_type, "image/png");
            }
            _ => panic!("Expected image content"),
        }
    }

    #[test]
    fn test_resources_list_parse_response() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "resources": [
                    {
                        "uri": "file:///project/package.json",
                        "name": "package.json",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "file:///project/README.md",
                        "name": "README"
                    }
                ]
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ResourcesList::parse_response(response).unwrap();

        assert_eq!(result.resources.len(), 2);
        assert_eq!(result.resources[0].uri, "file:///project/package.json");
        assert_eq!(result.resources[1].name, "README");
    }

    #[test]
    fn test_resources_read_parse_response() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "contents": [
                    {
                        "uri": "file:///test.txt",
                        "mimeType": "text/plain",
                        "text": "Hello, world!"
                    }
                ]
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ResourcesRead::parse_response(response).unwrap();

        assert_eq!(result.contents.len(), 1);
        assert_eq!(result.contents[0].text, Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_prompts_list_parse_response() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "prompts": [
                    {
                        "name": "code_review",
                        "description": "Review code for issues",
                        "arguments": [
                            {
                                "name": "language",
                                "description": "Programming language",
                                "required": true
                            }
                        ]
                    }
                ]
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = PromptsList::parse_response(response).unwrap();

        assert_eq!(result.prompts.len(), 1);
        assert_eq!(result.prompts[0].name, "code_review");
        assert!(result.prompts[0].arguments.is_some());
    }

    #[test]
    fn test_prompts_get_parse_response() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "description": "Review Rust code",
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "Please review this code"
                        }
                    }
                ]
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = PromptsGet::parse_response(response).unwrap();

        assert!(result.description.is_some());
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].role, "user");
    }

    #[test]
    fn test_roots_list_parse_response() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "roots": [
                    {
                        "uri": "file:///project",
                        "name": "Project Root"
                    }
                ]
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = RootsList::parse_response(response).unwrap();

        assert_eq!(result.roots.len(), 1);
        assert_eq!(result.roots[0].uri, "file:///project");
    }

    #[test]
    fn test_resources_subscribe_parse_response() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {}
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ResourcesSubscribe::parse_response(response);
        assert!(result.is_ok());
    }

    #[test]
    fn test_logging_set_level_parse_response() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {}
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = LoggingSetLevel::parse_response(response);
        assert!(result.is_ok());
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

mod error_handling_tests {
    use super::*;

    #[test]
    fn test_parse_response_with_json_rpc_error() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32601,
                "message": "Method not found"
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = Initialize::parse_response(response);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_response_with_error_and_data() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32602,
                "message": "Invalid params",
                "data": {
                    "field": "url",
                    "reason": "required field missing"
                }
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = Initialize::parse_response(response);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_response_missing_result_field() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = Initialize::parse_response(response);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_response_invalid_result_type() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": "invalid string instead of object"
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = Initialize::parse_response(response);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_response_json_invalid_json() {
        let json = r#"{"jsonrpc": "2.0", "id": 1, invalid}"#;
        let result = Initialize::parse_response_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_response_from_json_success() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "serverInfo": {
                    "name": "test",
                    "version": "1.0.0"
                }
            }
        }"#;

        let result = Initialize::parse_response_json(json);
        assert!(result.is_ok());
        let init_result = result.unwrap();
        assert_eq!(init_result.server_info.name, "test");
    }

    #[test]
    fn test_params_to_value_success() {
        let params = ToolCallParams::new("test_tool");
        let result = ToolsCall::params_to_value(params);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["name"], "test_tool");
    }

    #[test]
    fn test_result_from_value_success() {
        let value = json!({
            "content": [{"type": "text", "text": "ok"}],
            "isError": false
        });
        let result = ToolsCall::result_from_value(value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_result_from_value_invalid() {
        let value = json!("invalid");
        let result = ToolsCall::result_from_value(value);
        assert!(result.is_err());
    }
}

// ============================================================================
// PaginatedMethod Trait Tests
// ============================================================================

mod pagination_tests {
    use super::*;

    #[test]
    fn test_tools_list_pagination_with_cursor() {
        let result = ToolsListResult {
            tools: vec![],
            next_cursor: Some("next-token".to_string()),
        };

        assert_eq!(ToolsList::next_cursor(&result), Some("next-token"));
    }

    #[test]
    fn test_tools_list_pagination_no_cursor() {
        let result = ToolsListResult {
            tools: vec![],
            next_cursor: None,
        };

        assert_eq!(ToolsList::next_cursor(&result), None);
    }

    #[test]
    fn test_resources_list_pagination_with_cursor() {
        let result = ResourcesListResult {
            resources: vec![],
            next_cursor: Some("resource-token".to_string()),
        };

        assert_eq!(ResourcesList::next_cursor(&result), Some("resource-token"));
    }

    #[test]
    fn test_resources_list_pagination_no_cursor() {
        let result = ResourcesListResult {
            resources: vec![],
            next_cursor: None,
        };

        assert_eq!(ResourcesList::next_cursor(&result), None);
    }

    #[test]
    fn test_prompts_list_pagination_with_cursor() {
        let result = PromptsListResult {
            prompts: vec![],
            next_cursor: Some("prompt-token".to_string()),
        };

        assert_eq!(PromptsList::next_cursor(&result), Some("prompt-token"));
    }

    #[test]
    fn test_prompts_list_pagination_no_cursor() {
        let result = PromptsListResult {
            prompts: vec![],
            next_cursor: None,
        };

        assert_eq!(PromptsList::next_cursor(&result), None);
    }
}

// ============================================================================
// McpNotification Trait Tests
// ============================================================================

mod notification_tests {
    use super::*;

    #[test]
    fn test_notifications_initialized_method_name() {
        assert_eq!(NotificationsInitialized::METHOD_NAME, "notifications/initialized");
    }

    #[test]
    fn test_notifications_initialized_build_empty() {
        let notification = NotificationsInitialized::build_notification_empty();

        assert_eq!(notification.jsonrpc, "2.0");
        assert_eq!(notification.method, "notifications/initialized");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_notifications_tools_list_changed_method_name() {
        assert_eq!(
            NotificationsToolsListChanged::METHOD_NAME,
            "notifications/tools/list_changed"
        );
    }

    #[test]
    fn test_notifications_tools_list_changed_build() {
        let notification = NotificationsToolsListChanged::build_notification_empty();
        assert_eq!(notification.method, "notifications/tools/list_changed");
    }

    #[test]
    fn test_notifications_resources_list_changed_method_name() {
        assert_eq!(
            NotificationsResourcesListChanged::METHOD_NAME,
            "notifications/resources/list_changed"
        );
    }

    #[test]
    fn test_notifications_prompts_list_changed_method_name() {
        assert_eq!(
            NotificationsPromptsListChanged::METHOD_NAME,
            "notifications/prompts/list_changed"
        );
    }

    #[test]
    fn test_notifications_roots_list_changed_method_name() {
        assert_eq!(
            NotificationsRootsListChanged::METHOD_NAME,
            "notifications/roots/list_changed"
        );
    }

    #[test]
    fn test_notifications_progress_method_name() {
        assert_eq!(NotificationsProgress::METHOD_NAME, "notifications/progress");
    }

    #[test]
    fn test_notifications_progress_build_with_params() {
        let params = ProgressParams {
            progress_token: json!(1),
            progress: 50.0,
            total: Some(100.0),
        };

        let notification = NotificationsProgress::build_notification(params);

        assert_eq!(notification.method, "notifications/progress");
        assert!(notification.params.is_some());

        let params_value = notification.params.unwrap();
        assert_eq!(params_value["progress"], 50.0);
        assert_eq!(params_value["total"], 100.0);
    }

    #[test]
    fn test_notifications_progress_build_without_total() {
        let params = ProgressParams {
            progress_token: json!("token-123"),
            progress: 25.0,
            total: None,
        };

        let notification = NotificationsProgress::build_notification(params);
        let params_value = notification.params.unwrap();
        assert!(!params_value.as_object().unwrap().contains_key("total"));
    }

    #[test]
    fn test_notifications_message_method_name() {
        assert_eq!(NotificationsMessage::METHOD_NAME, "notifications/message");
    }

    #[test]
    fn test_notifications_message_build_with_params() {
        let params = LogMessageParams {
            level: LogLevel::Info,
            logger: Some("mcp-server".to_string()),
            data: "Server started".to_string(),
        };

        let notification = NotificationsMessage::build_notification(params);

        assert_eq!(notification.method, "notifications/message");
        let params_value = notification.params.unwrap();
        assert_eq!(params_value["level"], "info");
        assert_eq!(params_value["data"], "Server started");
    }

    #[test]
    fn test_notification_serialization_no_id() {
        let notification = NotificationsInitialized::build_notification_empty();
        let json = notification.to_json().unwrap();

        // Notifications must NOT have an "id" field
        assert!(!json.contains("\"id\""));
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"notifications/initialized\""));
    }
}

// ============================================================================
// McpMethodKind Enum Tests
// ============================================================================

mod method_kind_tests {
    use super::*;

    #[test]
    fn test_method_kind_method_name_lifecycle() {
        assert_eq!(McpMethodKind::Initialize.method_name(), "initialize");
        assert_eq!(McpMethodKind::Ping.method_name(), "ping");
    }

    #[test]
    fn test_method_kind_method_name_tools() {
        assert_eq!(McpMethodKind::ToolsList.method_name(), "tools/list");
        assert_eq!(McpMethodKind::ToolsCall.method_name(), "tools/call");
    }

    #[test]
    fn test_method_kind_method_name_resources() {
        assert_eq!(McpMethodKind::ResourcesList.method_name(), "resources/list");
        assert_eq!(McpMethodKind::ResourcesRead.method_name(), "resources/read");
        assert_eq!(McpMethodKind::ResourcesSubscribe.method_name(), "resources/subscribe");
        assert_eq!(McpMethodKind::ResourcesUnsubscribe.method_name(), "resources/unsubscribe");
    }

    #[test]
    fn test_method_kind_method_name_prompts() {
        assert_eq!(McpMethodKind::PromptsList.method_name(), "prompts/list");
        assert_eq!(McpMethodKind::PromptsGet.method_name(), "prompts/get");
    }

    #[test]
    fn test_method_kind_method_name_roots() {
        assert_eq!(McpMethodKind::RootsList.method_name(), "roots/list");
    }

    #[test]
    fn test_method_kind_method_name_logging() {
        assert_eq!(McpMethodKind::LoggingSetLevel.method_name(), "logging/setLevel");
    }

    #[test]
    fn test_method_kind_method_name_completion() {
        assert_eq!(McpMethodKind::CompletionComplete.method_name(), "completion/complete");
    }

    #[test]
    fn test_method_kind_method_name_sampling() {
        assert_eq!(
            McpMethodKind::SamplingCreateMessage.method_name(),
            "sampling/createMessage"
        );
    }

    #[test]
    fn test_method_kind_from_method_name_lifecycle() {
        assert_eq!(
            McpMethodKind::from_method_name("initialize"),
            Some(McpMethodKind::Initialize)
        );
        assert_eq!(
            McpMethodKind::from_method_name("ping"),
            Some(McpMethodKind::Ping)
        );
    }

    #[test]
    fn test_method_kind_from_method_name_tools() {
        assert_eq!(
            McpMethodKind::from_method_name("tools/list"),
            Some(McpMethodKind::ToolsList)
        );
        assert_eq!(
            McpMethodKind::from_method_name("tools/call"),
            Some(McpMethodKind::ToolsCall)
        );
    }

    #[test]
    fn test_method_kind_from_method_name_resources() {
        assert_eq!(
            McpMethodKind::from_method_name("resources/list"),
            Some(McpMethodKind::ResourcesList)
        );
        assert_eq!(
            McpMethodKind::from_method_name("resources/read"),
            Some(McpMethodKind::ResourcesRead)
        );
        assert_eq!(
            McpMethodKind::from_method_name("resources/subscribe"),
            Some(McpMethodKind::ResourcesSubscribe)
        );
        assert_eq!(
            McpMethodKind::from_method_name("resources/unsubscribe"),
            Some(McpMethodKind::ResourcesUnsubscribe)
        );
    }

    #[test]
    fn test_method_kind_from_method_name_prompts() {
        assert_eq!(
            McpMethodKind::from_method_name("prompts/list"),
            Some(McpMethodKind::PromptsList)
        );
        assert_eq!(
            McpMethodKind::from_method_name("prompts/get"),
            Some(McpMethodKind::PromptsGet)
        );
    }

    #[test]
    fn test_method_kind_from_method_name_roots() {
        assert_eq!(
            McpMethodKind::from_method_name("roots/list"),
            Some(McpMethodKind::RootsList)
        );
    }

    #[test]
    fn test_method_kind_from_method_name_logging() {
        assert_eq!(
            McpMethodKind::from_method_name("logging/setLevel"),
            Some(McpMethodKind::LoggingSetLevel)
        );
    }

    #[test]
    fn test_method_kind_from_method_name_completion() {
        assert_eq!(
            McpMethodKind::from_method_name("completion/complete"),
            Some(McpMethodKind::CompletionComplete)
        );
    }

    #[test]
    fn test_method_kind_from_method_name_sampling() {
        assert_eq!(
            McpMethodKind::from_method_name("sampling/createMessage"),
            Some(McpMethodKind::SamplingCreateMessage)
        );
    }

    #[test]
    fn test_method_kind_from_method_name_unknown() {
        assert_eq!(McpMethodKind::from_method_name("unknown/method"), None);
        assert_eq!(McpMethodKind::from_method_name("nonexistent"), None);
        assert_eq!(McpMethodKind::from_method_name(""), None);
    }

    #[test]
    fn test_method_kind_equality() {
        assert_eq!(McpMethodKind::Initialize, McpMethodKind::Initialize);
        assert_ne!(McpMethodKind::Initialize, McpMethodKind::Ping);
        assert_ne!(McpMethodKind::ToolsList, McpMethodKind::ToolsCall);
    }

    #[test]
    fn test_method_kind_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(McpMethodKind::Initialize);
        set.insert(McpMethodKind::ToolsList);
        set.insert(McpMethodKind::Initialize); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&McpMethodKind::Initialize));
        assert!(set.contains(&McpMethodKind::ToolsList));
    }

    #[test]
    fn test_method_kind_copy() {
        let kind = McpMethodKind::Initialize;
        let copied = kind; // Copy trait
        assert_eq!(kind, copied);
    }

    #[test]
    fn test_method_kind_debug() {
        let kind = McpMethodKind::Initialize;
        let debug_str = format!("{:?}", kind);
        assert!(debug_str.contains("Initialize"));
    }
}

// ============================================================================
// Type Safety Tests
// ============================================================================

mod type_safety_tests {
    use super::*;

    /// Verify that the associated types are correct for Initialize
    #[test]
    fn test_initialize_associated_types() {
        // This is a compile-time check - if this compiles, the types are correct
        fn _check_params(_: <Initialize as McpMethod>::Params) {}
        fn _check_result(_: <Initialize as McpMethod>::Result) {}

        _check_params(InitializeParams::new("test", "1.0"));
        _check_result(InitializeResult {
            protocol_version: "2025-11-25".to_string(),
            capabilities: ServerCapabilities::default(),
            server_info: ImplementationInfo::new("test", "1.0"),
            instructions: None,
        });
    }

    /// Verify that the associated types are correct for ToolsList
    #[test]
    fn test_tools_list_associated_types() {
        fn _check_params(_: <ToolsList as McpMethod>::Params) {}
        fn _check_result(_: <ToolsList as McpMethod>::Result) {}

        _check_params(ToolsListParams::default());
        _check_result(ToolsListResult {
            tools: vec![],
            next_cursor: None,
        });
    }

    /// Verify that the associated types are correct for ToolsCall
    #[test]
    fn test_tools_call_associated_types() {
        fn _check_params(_: <ToolsCall as McpMethod>::Params) {}
        fn _check_result(_: <ToolsCall as McpMethod>::Result) {}

        _check_params(ToolCallParams::new("test"));
        _check_result(ToolCallResult::text("ok"));
    }

    /// Verify that the associated types are correct for Ping
    #[test]
    fn test_ping_associated_types() {
        fn _check_params(_: <Ping as McpMethod>::Params) {}
        fn _check_result(_: <Ping as McpMethod>::Result) {}

        _check_params(PingParams::default());
        _check_result(PingResult::default());
    }

    /// Verify round-trip serialization for all method params
    #[test]
    fn test_params_roundtrip_initialize() {
        let params = Initialize::params("test-client", "1.0.0");
        let value = serde_json::to_value(&params).unwrap();
        let parsed: InitializeParams = serde_json::from_value(value).unwrap();

        assert_eq!(parsed.client_info.name, "test-client");
        assert_eq!(parsed.client_info.version, "1.0.0");
    }

    #[test]
    fn test_params_roundtrip_tools_call() {
        let params = ToolsCall::params_with_args("browser_click", json!({"x": 100, "y": 200}));
        let value = serde_json::to_value(&params).unwrap();
        let parsed: ToolCallParams = serde_json::from_value(value).unwrap();

        assert_eq!(parsed.name, "browser_click");
        assert!(parsed.arguments.is_some());
    }

    #[test]
    fn test_params_roundtrip_resources_read() {
        let params = ResourcesRead::params("file:///path/to/file.txt");
        let value = serde_json::to_value(&params).unwrap();
        let parsed: ResourceReadParams = serde_json::from_value(value).unwrap();

        assert_eq!(parsed.uri, "file:///path/to/file.txt");
    }
}

// ============================================================================
// Integration Tests (End-to-End Scenarios)
// ============================================================================

mod integration_tests {
    use super::*;

    /// Simulate a full initialize handshake
    #[test]
    fn test_initialize_handshake_flow() {
        // Client sends initialize request
        let params = Initialize::params("ltmatrix", "0.1.0");
        let request = Initialize::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "initialize");

        // Server responds
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2025-11-25",
                "capabilities": {
                    "tools": { "listChanged": true },
                    "resources": { "subscribe": true }
                },
                "serverInfo": {
                    "name": "playwright-mcp",
                    "version": "1.0.0"
                }
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = Initialize::parse_response(response).unwrap();

        assert_eq!(result.server_info.name, "playwright-mcp");

        // Client sends initialized notification
        let notification = NotificationsInitialized::build_notification_empty();
        assert_eq!(notification.method, "notifications/initialized");
    }

    /// Simulate listing and calling a tool
    #[test]
    fn test_list_and_call_tool_flow() {
        // 1. List tools
        let list_params = ToolsList::params();
        let list_request = ToolsList::build_request(RequestId::Number(1), list_params);

        let list_response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [
                    {
                        "name": "browser_navigate",
                        "description": "Navigate browser to URL",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "url": { "type": "string" }
                            },
                            "required": ["url"]
                        }
                    }
                ]
            }
        });

        let list_response: JsonRpcResponse = serde_json::from_value(list_response_json).unwrap();
        let list_result = ToolsList::parse_response(list_response).unwrap();

        assert_eq!(list_result.tools.len(), 1);

        // 2. Call the tool
        let call_params = ToolsCall::params_with_args(
            "browser_navigate",
            json!({"url": "https://example.com"}),
        );
        let call_request = ToolsCall::build_request(RequestId::Number(2), call_params);

        let call_response_json = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": "Navigated to https://example.com"
                    }
                ],
                "isError": false
            }
        });

        let call_response: JsonRpcResponse = serde_json::from_value(call_response_json).unwrap();
        let call_result = ToolsCall::parse_response(call_response).unwrap();

        assert!(!call_result.is_error);
    }

    /// Simulate reading a resource
    #[test]
    fn test_read_resource_flow() {
        // Read a resource
        let params = ResourcesRead::params("file:///project/Cargo.toml");
        let request = ResourcesRead::build_request(RequestId::Number(1), params);

        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "contents": [
                    {
                        "uri": "file:///project/Cargo.toml",
                        "mimeType": "text/plain",
                        "text": "[package]\nname = \"ltmatrix\"\nversion = \"0.1.0\""
                    }
                ]
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ResourcesRead::parse_response(response).unwrap();

        assert_eq!(result.contents.len(), 1);
        assert!(result.contents[0].text.as_ref().unwrap().contains("ltmatrix"));
    }

    /// Test pagination flow
    #[test]
    fn test_pagination_flow() {
        // First page
        let page1_response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [
                    {"name": "tool1", "description": "First tool", "inputSchema": {}}
                ],
                "nextCursor": "page-2-token"
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(page1_response_json).unwrap();
        let result = ToolsList::parse_response(response).unwrap();

        assert_eq!(result.tools.len(), 1);
        let cursor = ToolsList::next_cursor(&result);
        assert!(cursor.is_some());

        // Second page using cursor
        let page2_params = ToolsList::params_with_cursor(cursor.unwrap());
        let page2_request = ToolsList::build_request(RequestId::Number(2), page2_params);

        let params_value = page2_request.params.unwrap();
        assert_eq!(params_value["cursor"], "page-2-token");
    }

    /// Test method dispatch using McpMethodKind
    #[test]
    fn test_method_dispatch() {
        let method_names = [
            "initialize", "ping", "tools/list", "tools/call",
            "resources/list", "resources/read", "resources/subscribe",
            "resources/unsubscribe", "prompts/list", "prompts/get",
            "roots/list", "logging/setLevel", "completion/complete",
            "sampling/createMessage",
        ];

        for name in method_names {
            let kind = McpMethodKind::from_method_name(name);
            assert!(kind.is_some(), "Failed to parse method: {}", name);
            assert_eq!(kind.unwrap().method_name(), name);
        }
    }
}

// ============================================================================
// Edge Cases and Corner Cases
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_tools_list() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": []
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ToolsList::parse_response(response).unwrap();

        assert!(result.tools.is_empty());
        assert!(result.next_cursor.is_none());
    }

    #[test]
    fn test_tool_call_multiple_content_items() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [
                    {"type": "text", "text": "First result"},
                    {"type": "text", "text": "Second result"}
                ],
                "isError": false
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ToolsCall::parse_response(response).unwrap();

        assert_eq!(result.content.len(), 2);
    }

    #[test]
    fn test_tool_call_resource_content() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [
                    {
                        "type": "resource",
                        "uri": "file:///output.txt",
                        "mime_type": "text/plain"
                    }
                ],
                "isError": false
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ToolsCall::parse_response(response).unwrap();

        match &result.content[0] {
            ToolContent::Resource { uri, mime_type } => {
                assert_eq!(uri, "file:///output.txt");
                assert_eq!(mime_type, &Some("text/plain".to_string()));
            }
            _ => panic!("Expected resource content"),
        }
    }

    #[test]
    fn test_binary_resource_contents() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "contents": [
                    {
                        "uri": "file:///image.png",
                        "mimeType": "image/png",
                        "blob": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAAB"
                    }
                ]
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ResourcesRead::parse_response(response).unwrap();

        assert!(result.contents[0].blob.is_some());
        assert!(result.contents[0].text.is_none());
    }

    #[test]
    fn test_string_request_id() {
        let params = Initialize::params("test", "1.0");
        let request = Initialize::build_request(
            RequestId::String("uuid-12345-abcde".to_string()),
            params,
        );

        assert_eq!(request.id, RequestId::String("uuid-12345-abcde".to_string()));
    }

    #[test]
    fn test_special_characters_in_tool_args() {
        let params = ToolsCall::params_with_args(
            "test_tool",
            json!({
                "query": "SELECT * FROM users WHERE name = 'O'Brien'",
                "regex": "\\d+\\.\\d+",
                "unicode": "Hello 世界 🌍"
            }),
        );

        let value = ToolsCall::params_to_value(params).unwrap();
        // Arguments are nested inside the "arguments" field
        let args = &value["arguments"];
        assert!(args["query"].as_str().unwrap().contains("O'Brien"));
        assert!(args["unicode"].as_str().unwrap().contains("世界"));
    }

    #[test]
    fn test_large_tool_list() {
        let tools: Vec<serde_json::Value> = (1..=100)
            .map(|i| {
                json!({
                    "name": format!("tool_{}", i),
                    "description": format!("Tool number {}", i),
                    "inputSchema": {"type": "object"}
                })
            })
            .collect();

        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": tools
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ToolsList::parse_response(response).unwrap();

        assert_eq!(result.tools.len(), 100);
    }
}
