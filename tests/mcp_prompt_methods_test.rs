// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Tests for Prompt MCP Methods (list_prompts, get_prompt)
//!
//! These tests verify prompt template discovery and retrieval functionality,
//! including parameter validation and result formatting.

use ltmatrix::mcp::{
    Prompt, PromptArgument, PromptContent, PromptMessage,
    PromptsListParams, PromptsListResult, PromptsGetParams, PromptsGetResult,
};
use ltmatrix::mcp::protocol::wrappers::{
    McpMethod, PaginatedMethod, PromptsList, PromptsGet,
    McpMethodKind,
};
use ltmatrix::mcp::protocol::messages::{JsonRpcResponse, RequestId};
use ltmatrix::mcp::protocol::errors::JsonRpcError;
use ltmatrix::mcp::client::McpClient;
use serde_json::json;

// ============================================================================
// Prompts List Method Tests
// ============================================================================

mod prompts_list_method_tests {
    use super::*;

    /// Test that PromptsList method has correct method name
    #[test]
    fn test_prompts_list_method_name() {
        assert_eq!(PromptsList::METHOD_NAME, "prompts/list");
    }

    /// Test building prompts/list request without cursor
    #[test]
    fn test_prompts_list_build_request_no_cursor() {
        let params = PromptsListParams::default();
        let request = PromptsList::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "prompts/list");
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, RequestId::Number(1));
    }

    /// Test building prompts/list request with cursor
    #[test]
    fn test_prompts_list_build_request_with_cursor() {
        let params = PromptsListParams {
            cursor: Some("next-prompts-page".to_string()),
        };
        let request = PromptsList::build_request(RequestId::Number(2), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["cursor"], "next-prompts-page");
    }

    /// Test PromptsList::params helper
    #[test]
    fn test_prompts_list_params_helper() {
        let params = PromptsList::params();
        assert!(params.cursor.is_none());
    }

    /// Test PromptsList::params_with_cursor helper
    #[test]
    fn test_prompts_list_params_with_cursor_helper() {
        let params = PromptsList::params_with_cursor("page-token-123");
        assert_eq!(params.cursor, Some("page-token-123".to_string()));
    }

    /// Test parsing successful prompts/list response
    #[test]
    fn test_prompts_list_parse_success_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "prompts": [
                    {
                        "name": "code_review",
                        "description": "Generate a code review prompt",
                        "arguments": [
                            {
                                "name": "language",
                                "description": "Programming language",
                                "required": true
                            },
                            {
                                "name": "style",
                                "description": "Review style",
                                "required": false
                            }
                        ]
                    },
                    {
                        "name": "summarize",
                        "description": "Summarize text content"
                    }
                ]
            }),
        );

        let result = PromptsList::parse_response(response).unwrap();

        assert_eq!(result.prompts.len(), 2);
        assert_eq!(result.prompts[0].name, "code_review");
        assert_eq!(result.prompts[0].description, Some("Generate a code review prompt".to_string()));
        assert!(result.prompts[0].arguments.is_some());

        let args = result.prompts[0].arguments.as_ref().unwrap();
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].name, "language");
        assert!(args[0].required);
        assert_eq!(args[1].name, "style");
        assert!(!args[1].required);

        assert_eq!(result.prompts[1].name, "summarize");
        assert!(result.prompts[1].arguments.is_none());
    }

    /// Test parsing prompts/list response with pagination
    #[test]
    fn test_prompts_list_pagination_cursor() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "prompts": [
                    { "name": "prompt1" }
                ],
                "nextCursor": "next-page-token"
            }),
        );

        let result = PromptsList::parse_response(response).unwrap();

        assert_eq!(result.prompts.len(), 1);
        assert_eq!(result.next_cursor, Some("next-page-token".to_string()));
    }

    /// Test parsing prompts/list response without next cursor
    #[test]
    fn test_prompts_list_parse_no_next_cursor() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "prompts": [
                    { "name": "only_prompt" }
                ]
            }),
        );

        let result = PromptsList::parse_response(response).unwrap();

        assert_eq!(result.prompts.len(), 1);
        assert!(result.next_cursor.is_none());
    }

    /// Test parsing empty prompts/list response
    #[test]
    fn test_prompts_list_parse_empty_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "prompts": []
            }),
        );

        let result = PromptsList::parse_response(response).unwrap();

        assert_eq!(result.prompts.len(), 0);
    }

    /// Test prompts/list error response
    #[test]
    fn test_prompts_list_error_response() {
        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            JsonRpcError::internal_error("Internal error"),
        );

        let result = PromptsList::parse_response(response);
        assert!(result.is_err());
    }

    /// Test PaginatedMethod trait implementation
    #[test]
    fn test_prompts_list_pagination_trait() {
        let result = PromptsListResult {
            prompts: vec![],
            next_cursor: Some("token".to_string()),
        };

        assert_eq!(PromptsList::next_cursor(&result), Some("token"));

        let result_no_cursor = PromptsListResult {
            prompts: vec![],
            next_cursor: None,
        };

        assert_eq!(PromptsList::next_cursor(&result_no_cursor), None);
    }

    /// Test Prompt creation
    #[test]
    fn test_prompt_creation() {
        let prompt = Prompt::new("test_prompt");
        assert_eq!(prompt.name, "test_prompt");
        assert!(prompt.description.is_none());
        assert!(prompt.arguments.is_none());
    }

    /// Test PromptArgument creation
    #[test]
    fn test_prompt_argument_creation() {
        let arg = PromptArgument::new("test_arg");
        assert_eq!(arg.name, "test_arg");
        assert!(arg.description.is_none());
        assert!(!arg.required);

        let required_arg = PromptArgument::new("required_arg").required();
        assert!(required_arg.required);
    }
}

// ============================================================================
// Prompts Get Method Tests
// ============================================================================

mod prompts_get_method_tests {
    use super::*;

    /// Test that PromptsGet method has correct method name
    #[test]
    fn test_prompts_get_method_name() {
        assert_eq!(PromptsGet::METHOD_NAME, "prompts/get");
    }

    /// Test building prompts/get request without arguments
    #[test]
    fn test_prompts_get_build_request_no_args() {
        let params = PromptsGet::params("simple_prompt");
        let request = PromptsGet::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "prompts/get");
        assert_eq!(request.jsonrpc, "2.0");

        let params_value = request.params.unwrap();
        assert_eq!(params_value["name"], "simple_prompt");
    }

    /// Test building prompts/get request with arguments
    #[test]
    fn test_prompts_get_build_request_with_args() {
        let params = PromptsGet::params_with_args(
            "code_review",
            json!({
                "language": "rust",
                "style": "thorough"
            })
        );
        let request = PromptsGet::build_request(RequestId::Number(1), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["name"], "code_review");
        assert_eq!(params_value["arguments"]["language"], "rust");
        assert_eq!(params_value["arguments"]["style"], "thorough");
    }

    /// Test PromptsGet::params helper
    #[test]
    fn test_prompts_get_params_helper() {
        let params = PromptsGet::params("test_prompt");
        assert_eq!(params.name, "test_prompt");
        assert!(params.arguments.is_none());
    }

    /// Test PromptsGet::params_with_args helper
    #[test]
    fn test_prompts_get_params_with_args_helper() {
        let params = PromptsGet::params_with_args(
            "translate",
            json!({"from": "en", "to": "es"})
        );
        assert_eq!(params.name, "translate");
        assert!(params.arguments.is_some());
    }

    /// Test parsing prompts/get response with text messages
    #[test]
    fn test_prompts_get_parse_text_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "description": "Code review prompt for Rust",
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "Please review the following Rust code:"
                        }
                    },
                    {
                        "role": "assistant",
                        "content": {
                            "type": "text",
                            "text": "I will review the code for safety and idiomatic patterns."
                        }
                    }
                ]
            }),
        );

        let result = PromptsGet::parse_response(response).unwrap();

        assert_eq!(result.description, Some("Code review prompt for Rust".to_string()));
        assert_eq!(result.messages.len(), 2);

        assert_eq!(result.messages[0].role, "user");
        match &result.messages[0].content {
            PromptContent::Text { text } => assert_eq!(text, "Please review the following Rust code:"),
            _ => panic!("Expected text content"),
        }

        assert_eq!(result.messages[1].role, "assistant");
    }

    /// Test parsing prompts/get response with image content
    #[test]
    fn test_prompts_get_parse_image_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "image",
                            "data": "base64imagedata",
                            "mime_type": "image/png"
                        }
                    }
                ]
            }),
        );

        let result = PromptsGet::parse_response(response).unwrap();

        assert_eq!(result.messages.len(), 1);
        match &result.messages[0].content {
            PromptContent::Image { data, mime_type } => {
                assert_eq!(data, "base64imagedata");
                assert_eq!(mime_type, "image/png");
            }
            _ => panic!("Expected image content"),
        }
    }

    /// Test parsing prompts/get response with resource content
    #[test]
    fn test_prompts_get_parse_resource_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "resource",
                            "resource": {
                                "uri": "file:///code/main.rs",
                                "name": "main.rs",
                                "mime_type": "text/rust"
                            }
                        }
                    }
                ]
            }),
        );

        let result = PromptsGet::parse_response(response).unwrap();

        assert_eq!(result.messages.len(), 1);
        match &result.messages[0].content {
            PromptContent::Resource { resource } => {
                assert_eq!(resource.uri, "file:///code/main.rs");
                assert_eq!(resource.name, "main.rs");
            }
            _ => panic!("Expected resource content"),
        }
    }

    /// Test parsing prompts/get response with multiple messages
    #[test]
    fn test_prompts_get_parse_multiple_messages() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "description": "Multi-step prompt",
                "messages": [
                    {
                        "role": "system",
                        "content": { "type": "text", "text": "You are a helpful assistant." }
                    },
                    {
                        "role": "user",
                        "content": { "type": "text", "text": "Help me write code." }
                    },
                    {
                        "role": "assistant",
                        "content": { "type": "text", "text": "I'd be happy to help!" }
                    }
                ]
            }),
        );

        let result = PromptsGet::parse_response(response).unwrap();

        assert_eq!(result.messages.len(), 3);
        assert_eq!(result.messages[0].role, "system");
        assert_eq!(result.messages[1].role, "user");
        assert_eq!(result.messages[2].role, "assistant");
    }

    /// Test PromptsGet error response
    #[test]
    fn test_prompts_get_error_response() {
        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            JsonRpcError::invalid_params("unknown prompt"),
        );

        let result = PromptsGet::parse_response(response);
        assert!(result.is_err());
    }

    /// Test PromptMessage helper methods
    #[test]
    fn test_prompt_message_helpers() {
        let user_msg = PromptMessage::user("Hello, assistant!");
        assert_eq!(user_msg.role, "user");
        match user_msg.content {
            PromptContent::Text { text } => assert_eq!(text, "Hello, assistant!"),
            _ => panic!("Expected text content"),
        }

        let assistant_msg = PromptMessage::assistant("Hello, user!");
        assert_eq!(assistant_msg.role, "assistant");
    }
}

// ============================================================================
// Parameter Validation Tests
// ============================================================================

mod parameter_validation_tests {
    use super::*;

    /// Test valid prompt names
    #[test]
    fn test_valid_prompt_names() {
        let valid_names = vec![
            "simple",
            "code_review",
            "translate-text",
            "analyze.code",
            "v1_prompt",
        ];

        for name in valid_names {
            let params = PromptsGetParams::new(name);
            assert_eq!(params.name, name);
        }
    }

    /// Test various argument types
    #[test]
    fn test_various_argument_types() {
        // String arguments
        let params = PromptsGet::params_with_args("test", json!({"string_arg": "value"}));
        let args = params.arguments.unwrap();
        assert_eq!(args["string_arg"], "value");

        // Number arguments
        let params = PromptsGet::params_with_args("test", json!({"number_arg": 42}));
        let args = params.arguments.unwrap();
        assert_eq!(args["number_arg"], 42);

        // Boolean arguments
        let params = PromptsGet::params_with_args("test", json!({"bool_arg": true}));
        let args = params.arguments.unwrap();
        assert_eq!(args["bool_arg"], true);

        // Nested arguments
        let params = PromptsGet::params_with_args(
            "test",
            json!({
                "config": {
                    "language": "rust",
                    "strict": true
                }
            })
        );
        let args = params.arguments.unwrap();
        assert_eq!(args["config"]["language"], "rust");
    }
}

// ============================================================================
// Result Formatting Tests
// ============================================================================

mod result_formatting_tests {
    use super::*;

    /// Test extracting text messages from result
    #[test]
    fn test_extract_text_messages() {
        let result = PromptsGetResult {
            description: Some("Test prompt".to_string()),
            messages: vec![
                PromptMessage::user("Hello"),
                PromptMessage::assistant("Hi there"),
                PromptMessage::user("How are you?"),
            ],
        };

        let texts = McpClient::extract_text_messages(&result);
        assert_eq!(texts.len(), 3);
        assert_eq!(texts[0], ("user".to_string(), "Hello".to_string()));
        assert_eq!(texts[1], ("assistant".to_string(), "Hi there".to_string()));
    }

    /// Test extracting all text content
    #[test]
    fn test_extract_all_text() {
        let result = PromptsGetResult {
            description: Some("Test prompt".to_string()),
            messages: vec![
                PromptMessage::user("Line 1"),
                PromptMessage::assistant("Line 2"),
                PromptMessage::user("Line 3"),
            ],
        };

        let text = McpClient::extract_all_text(&result);
        assert_eq!(text, "Line 1\nLine 2\nLine 3");
    }

    /// Test checking if result is text only
    #[test]
    fn test_is_text_only() {
        let text_result = PromptsGetResult {
            description: None,
            messages: vec![PromptMessage::user("Text only")],
        };
        assert!(McpClient::is_text_only(&text_result));
    }

    /// Test content summary
    #[test]
    fn test_prompt_content_summary() {
        let result = PromptsGetResult {
            description: None,
            messages: vec![
                PromptMessage::user("Text 1"),
                PromptMessage::user("Text 2"),
            ],
        };

        let (text_count, image_count, resource_count) = McpClient::prompt_content_summary(&result);
        assert_eq!(text_count, 2);
        assert_eq!(image_count, 0);
        assert_eq!(resource_count, 0);
    }
}

// ============================================================================
// Method Kind Tests
// ============================================================================

mod prompts_method_kind_tests {
    use super::*;

    /// Test that PromptsList is recognized as a known method
    #[test]
    fn test_prompts_list_method_kind() {
        assert_eq!(
            McpMethodKind::from_method_name("prompts/list"),
            Some(McpMethodKind::PromptsList)
        );
        assert_eq!(McpMethodKind::PromptsList.method_name(), "prompts/list");
    }

    /// Test that PromptsGet is recognized as a known method
    #[test]
    fn test_prompts_get_method_kind() {
        assert_eq!(
            McpMethodKind::from_method_name("prompts/get"),
            Some(McpMethodKind::PromptsGet)
        );
        assert_eq!(McpMethodKind::PromptsGet.method_name(), "prompts/get");
    }
}

// ============================================================================
// Integration-like Tests
// ============================================================================

mod integration_like_tests {
    use super::*;

    /// Test simulated prompts/list flow
    #[test]
    fn test_simulated_prompts_list_flow() {
        let params = PromptsList::params();
        let request = PromptsList::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "prompts/list");
        assert_eq!(request.jsonrpc, "2.0");

        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "prompts": [
                    { "name": "code_review", "description": "Review code" },
                    { "name": "summarize", "description": "Summarize text" }
                ]
            }),
        );

        let result = PromptsList::parse_response(response).unwrap();
        assert_eq!(result.prompts.len(), 2);
    }

    /// Test simulated prompts/get flow
    #[test]
    fn test_simulated_prompts_get_flow() {
        let params = PromptsGet::params_with_args(
            "code_review",
            json!({"language": "rust", "strict": true})
        );
        let request = PromptsGet::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "prompts/get");
        let request_params = request.params.unwrap();
        assert_eq!(request_params["name"], "code_review");

        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "description": "Review Rust code with strict mode",
                "messages": [
                    {
                        "role": "user",
                        "content": { "type": "text", "text": "Review this Rust code:" }
                    }
                ]
            }),
        );

        let result = PromptsGet::parse_response(response).unwrap();
        assert_eq!(result.description, Some("Review Rust code with strict mode".to_string()));
        assert_eq!(result.messages.len(), 1);
    }

    /// Test simulated prompts/get error flow
    #[test]
    fn test_simulated_prompts_get_error_flow() {
        let params = PromptsGet::params("unknown_prompt");
        let request = PromptsGet::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "prompts/get");

        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            JsonRpcError::invalid_params("prompt not found"),
        );

        let result = PromptsGet::parse_response(response);
        assert!(result.is_err());
    }

    /// Test method routing detection
    #[test]
    fn test_method_routing_detection() {
        assert!(McpMethodKind::from_method_name("prompts/list").is_some());
        assert!(McpMethodKind::from_method_name("prompts/get").is_some());
        assert!(McpMethodKind::from_method_name("prompts/unknown").is_none());
    }
}

// ============================================================================
// Serialization Edge Cases
// ============================================================================

mod serialization_edge_cases {
    use super::*;

    /// Test unicode in prompt description
    #[test]
    fn test_unicode_in_prompt_description() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "prompts": [
                    {
                        "name": "unicode_prompt",
                        "description": "这是一个中文描述 🎉 Émojis and accents: café"
                    }
                ]
            }),
        );

        let result = PromptsList::parse_response(response).unwrap();
        assert!(result.prompts[0].description.as_ref().unwrap().contains("中文"));
        assert!(result.prompts[0].description.as_ref().unwrap().contains("🎉"));
    }

    /// Test large prompt output
    #[test]
    fn test_large_prompt_output() {
        let large_text = "x".repeat(10000);
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "messages": [
                    {
                        "role": "user",
                        "content": { "type": "text", "text": large_text.clone() }
                    }
                ]
            }),
        );

        let result = PromptsGet::parse_response(response).unwrap();
        match &result.messages[0].content {
            PromptContent::Text { text } => assert_eq!(text.len(), 10000),
            _ => panic!("Expected text content"),
        }
    }

    /// Test many prompts in list
    #[test]
    fn test_many_prompts_in_list() {
        let prompts: Vec<serde_json::Value> = (0..100)
            .map(|i| json!({ "name": format!("prompt_{}", i) }))
            .collect();

        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({ "prompts": prompts }),
        );

        let result = PromptsList::parse_response(response).unwrap();
        assert_eq!(result.prompts.len(), 100);
    }

    /// Test request id variations
    #[test]
    fn test_request_id_variations() {
        let request = PromptsList::build_request(RequestId::Number(1), PromptsListParams::default());
        assert_eq!(request.id, RequestId::Number(1));

        let request = PromptsList::build_request(RequestId::String("abc-123".to_string()), PromptsListParams::default());
        assert_eq!(request.id, RequestId::String("abc-123".to_string()));

        let request = PromptsList::build_request(RequestId::Number(i64::MAX), PromptsListParams::default());
        assert_eq!(request.id, RequestId::Number(i64::MAX));
    }

    /// Test special characters in prompt output
    #[test]
    fn test_special_chars_in_prompt_output() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "messages": [
                    {
                        "role": "user",
                        "content": { "type": "text", "text": "Special chars: \n\t\"'<>&" }
                    }
                ]
            }),
        );

        let result = PromptsGet::parse_response(response).unwrap();
        match &result.messages[0].content {
            PromptContent::Text { text } => assert!(text.contains('\n') && text.contains('\t')),
            _ => panic!("Expected text content"),
        }
    }
}

// ============================================================================
// JSON-RPC Format Tests
// ============================================================================

mod jsonrpc_format_tests {
    use super::*;

    /// Test JSON-RPC version in requests
    #[test]
    fn test_jsonrpc_version_in_requests() {
        let request = PromptsList::build_request(RequestId::Number(1), PromptsListParams::default());
        assert_eq!(request.jsonrpc, "2.0");

        let request = PromptsGet::build_request(RequestId::Number(1), PromptsGetParams::new("test"));
        assert_eq!(request.jsonrpc, "2.0");
    }

    /// Test request serialization format
    #[test]
    fn test_request_serialization_format() {
        let params = PromptsGet::params_with_args("test", json!({"arg": "value"}));
        let request = PromptsGet::build_request(RequestId::Number(1), params);

        let json = serde_json::to_string(&request).unwrap();

        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"prompts/get\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"params\""));
    }

    /// Test response serialization format
    #[test]
    fn test_response_serialization_format() {
        let result = PromptsListResult {
            prompts: vec![Prompt::new("test")],
            next_cursor: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"prompts\""));
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
            JsonRpcError::invalid_request("test error"),
        );

        let result = PromptsList::parse_response(response);
        assert!(result.is_err());
    }

    /// Test prompt not found error
    #[test]
    fn test_prompt_not_found_error() {
        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            JsonRpcError::invalid_params("prompt not found"),
        );

        let result = PromptsGet::parse_response(response);
        assert!(result.is_err());
    }

    /// Test parsing response with missing fields
    #[test]
    fn test_parse_response_missing_fields() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({}),
        );

        let result = PromptsList::parse_response(response);
        assert!(result.is_err());
    }

    /// Test parsing response with malformed JSON
    #[test]
    fn test_parse_response_malformed_json() {
        let result: Result<PromptsGetResult, _> = serde_json::from_str("{ invalid json }");
        assert!(result.is_err());
    }
}
