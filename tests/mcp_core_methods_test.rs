// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Tests for Core MCP Methods (initialize, list_resources, read_resource)
//!
//! These tests verify the fundamental MCP protocol methods needed for
//! basic client-server communication.

use ltmatrix::mcp::protocol::messages::{JsonRpcResponse, RequestId};
use ltmatrix::mcp::protocol::wrappers::{
    Initialize, McpMethod, McpMethodKind, PaginatedMethod, ResourcesList, ResourcesRead,
};
use ltmatrix::mcp::{
    ClientCapabilities, ImplementationInfo, InitializeParams, InitializeResult, Resource,
    ResourceContents, ResourceReadParams, ResourcesListParams, ResourcesListResult,
    ServerCapabilities, ServerInfo, ToolsCapability, MCP_PROTOCOL_VERSION,
};
use serde_json::json;

// ============================================================================
// Initialize Method Tests
// ============================================================================

mod initialize_method_tests {
    use super::*;

    /// Test that Initialize method has correct method name
    #[test]
    fn test_initialize_method_name() {
        assert_eq!(Initialize::METHOD_NAME, "initialize");
    }

    /// Test building initialize request with basic params
    #[test]
    fn test_initialize_build_request_basic() {
        let params = InitializeParams::new("test-client", "1.0.0");
        let request = Initialize::build_request(RequestId::Number(1), params);

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "initialize");
        assert_eq!(request.id, RequestId::Number(1));
        assert!(request.params.is_some());

        let params_value = request.params.unwrap();
        assert_eq!(params_value["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert_eq!(params_value["clientInfo"]["name"], "test-client");
        assert_eq!(params_value["clientInfo"]["version"], "1.0.0");
    }

    /// Test building initialize request with custom capabilities
    #[test]
    fn test_initialize_build_request_with_capabilities() {
        let capabilities = ClientCapabilities {
            roots: Some(ltmatrix::mcp::RootsCapability::with_list_changed(true)),
            ..Default::default()
        };
        let params = InitializeParams::new("test-client", "1.0.0").with_capabilities(capabilities);

        let request = Initialize::build_request(RequestId::String("init-1".to_string()), params);

        assert_eq!(request.id, RequestId::String("init-1".to_string()));
        let params_value = request.params.unwrap();
        assert!(params_value["capabilities"]["roots"].is_object());
    }

    /// Test Initialize::params helper method
    #[test]
    fn test_initialize_params_helper() {
        let params = Initialize::params("my-client", "2.0.0");

        assert_eq!(params.protocol_version, MCP_PROTOCOL_VERSION);
        assert_eq!(params.client_info.name, "my-client");
        assert_eq!(params.client_info.version, "2.0.0");
    }

    /// Test Initialize::params_with_capabilities helper
    #[test]
    fn test_initialize_params_with_capabilities_helper() {
        let capabilities = ClientCapabilities::default();
        let params = Initialize::params_with_capabilities("client", "1.0", capabilities.clone());

        assert_eq!(params.capabilities, capabilities);
    }

    /// Test parsing successful initialize response
    #[test]
    fn test_initialize_parse_success_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {
                    "tools": { "listChanged": true },
                    "resources": { "subscribe": true, "listChanged": true }
                },
                "serverInfo": {
                    "name": "playwright-mcp",
                    "version": "1.0.0"
                },
                "instructions": "Welcome to the MCP server"
            }),
        );

        let result = Initialize::parse_response(response).unwrap();

        assert_eq!(result.protocol_version, "2025-11-25");
        assert_eq!(result.server_info.name, "playwright-mcp");
        assert_eq!(result.server_info.version, "1.0.0");
        assert_eq!(
            result.instructions,
            Some("Welcome to the MCP server".to_string())
        );
        assert!(result.capabilities.tools.is_some());
        assert!(result.capabilities.resources.is_some());
    }

    /// Test parsing initialize response without optional instructions
    #[test]
    fn test_initialize_parse_response_without_instructions() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "serverInfo": {
                    "name": "minimal-server",
                    "version": "0.1.0"
                }
            }),
        );

        let result = Initialize::parse_response(response).unwrap();

        assert_eq!(result.server_info.name, "minimal-server");
        assert_eq!(result.instructions, None);
    }

    /// Test parsing initialize error response
    #[test]
    fn test_initialize_parse_error_response() {
        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            ltmatrix::mcp::protocol::errors::JsonRpcError::invalid_params(
                "Invalid protocol version",
            ),
        );

        let result = Initialize::parse_response(response);
        assert!(result.is_err());
    }

    /// Test parsing initialize response with missing result
    #[test]
    fn test_initialize_parse_missing_result() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            result: None,
            error: None,
        };

        let result = Initialize::parse_response(response);
        assert!(result.is_err());
    }

    /// Test Initialize params serialization
    #[test]
    fn test_initialize_params_serialization() {
        let params = InitializeParams::new("test", "1.0");
        let json = serde_json::to_string(&params).unwrap();

        assert!(json.contains("\"protocolVersion\""));
        assert!(json.contains("\"capabilities\""));
        assert!(json.contains("\"clientInfo\""));
    }

    /// Test Initialize result deserialization with various capability combinations
    #[test]
    fn test_initialize_result_various_capabilities() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {
                    "tools": { "listChanged": true },
                    "resources": { "subscribe": true, "listChanged": true },
                    "prompts": { "listChanged": false },
                    "logging": {},
                    "completions": {}
                },
                "serverInfo": {
                    "name": "full-featured-server",
                    "version": "2.0.0"
                }
            }),
        );

        let result = Initialize::parse_response(response).unwrap();

        assert!(result.capabilities.tools.is_some());
        assert!(result.capabilities.resources.is_some());
        assert!(result.capabilities.prompts.is_some());
        assert!(result.capabilities.logging.is_some());
        assert!(result.capabilities.completions.is_some());
    }

    /// Test ServerInfo conversion from InitializeResult
    #[test]
    fn test_server_info_from_initialize_result() {
        let init_result = InitializeResult {
            protocol_version: "2025-11-25".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(true),
                }),
                ..Default::default()
            },
            server_info: ImplementationInfo::new("test-server", "1.0.0"),
            instructions: Some("Test instructions".to_string()),
        };

        let server_info = ServerInfo::from(init_result);

        assert_eq!(server_info.info.name, "test-server");
        assert_eq!(server_info.info.version, "1.0.0");
        assert_eq!(server_info.protocol_version, "2025-11-25");
        assert_eq!(
            server_info.instructions,
            Some("Test instructions".to_string())
        );
        assert!(server_info.capabilities.tools.is_some());
    }
}

// ============================================================================
// Resources List Method Tests
// ============================================================================

mod resources_list_method_tests {
    use super::*;

    /// Test that ResourcesList method has correct method name
    #[test]
    fn test_resources_list_method_name() {
        assert_eq!(ResourcesList::METHOD_NAME, "resources/list");
    }

    /// Test building resources/list request without cursor
    #[test]
    fn test_resources_list_build_request_no_cursor() {
        let params = ResourcesListParams::default();
        let request = ResourcesList::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "resources/list");
        // Default params should result in empty or no cursor field
        if let Some(p) = request.params {
            assert!(p.get("cursor").is_none() || p["cursor"].is_null());
        }
    }

    /// Test building resources/list request with cursor
    #[test]
    fn test_resources_list_build_request_with_cursor() {
        let params = ResourcesListParams {
            cursor: Some("next-page-token".to_string()),
        };
        let request = ResourcesList::build_request(RequestId::Number(2), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["cursor"], "next-page-token");
    }

    /// Test ResourcesList::params helper
    #[test]
    fn test_resources_list_params_helper() {
        let params = ResourcesList::params();
        assert!(params.cursor.is_none());
    }

    /// Test ResourcesList::params_with_cursor helper
    #[test]
    fn test_resources_list_params_with_cursor_helper() {
        let params = ResourcesList::params_with_cursor("page-2-token");
        assert_eq!(params.cursor, Some("page-2-token".to_string()));
    }

    /// Test parsing successful resources/list response
    #[test]
    fn test_resources_list_parse_success_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "resources": [
                    {
                        "uri": "file:///project/package.json",
                        "name": "package.json",
                        "description": "Node.js package manifest",
                        "mime_type": "application/json"
                    },
                    {
                        "uri": "file:///project/README.md",
                        "name": "README.md",
                        "mime_type": "text/markdown"
                    }
                ],
                "nextCursor": "next-page"
            }),
        );

        let result = ResourcesList::parse_response(response).unwrap();

        assert_eq!(result.resources.len(), 2);
        assert_eq!(result.resources[0].uri, "file:///project/package.json");
        assert_eq!(result.resources[0].name, "package.json");
        assert_eq!(
            result.resources[0].description,
            Some("Node.js package manifest".to_string())
        );
        assert_eq!(
            result.resources[0].mime_type,
            Some("application/json".to_string())
        );
        assert_eq!(result.resources[1].uri, "file:///project/README.md");
        assert_eq!(result.next_cursor, Some("next-page".to_string()));
    }

    /// Test parsing resources/list response with empty resources
    #[test]
    fn test_resources_list_parse_empty_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "resources": []
            }),
        );

        let result = ResourcesList::parse_response(response).unwrap();

        assert!(result.resources.is_empty());
        assert!(result.next_cursor.is_none());
    }

    /// Test parsing resources/list response without next cursor
    #[test]
    fn test_resources_list_parse_no_next_cursor() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "resources": [
                    {
                        "uri": "file:///test.txt",
                        "name": "test.txt"
                    }
                ]
            }),
        );

        let result = ResourcesList::parse_response(response).unwrap();

        assert_eq!(result.resources.len(), 1);
        assert!(result.next_cursor.is_none());
    }

    /// Test pagination cursor extraction via PaginatedMethod trait
    #[test]
    fn test_resources_list_pagination_cursor() {
        let result_with_cursor = ResourcesListResult {
            resources: vec![],
            next_cursor: Some("cursor-value".to_string()),
        };
        assert_eq!(
            ResourcesList::next_cursor(&result_with_cursor),
            Some("cursor-value")
        );

        let result_without_cursor = ResourcesListResult {
            resources: vec![],
            next_cursor: None,
        };
        assert_eq!(ResourcesList::next_cursor(&result_without_cursor), None);
    }

    /// Test Resource struct creation
    #[test]
    fn test_resource_creation() {
        let resource = Resource::new("file:///test.txt", "test.txt");

        assert_eq!(resource.uri, "file:///test.txt");
        assert_eq!(resource.name, "test.txt");
        assert!(resource.description.is_none());
        assert!(resource.mime_type.is_none());
    }

    /// Test Resource with all optional fields
    #[test]
    fn test_resource_with_all_fields() {
        let json = json!({
            "uri": "file:///project/src/main.rs",
            "name": "main.rs",
            "description": "Application entry point",
            "mime_type": "text/x-rust"
        });

        let resource: Resource = serde_json::from_value(json).unwrap();

        assert_eq!(resource.uri, "file:///project/src/main.rs");
        assert_eq!(resource.name, "main.rs");
        assert_eq!(
            resource.description,
            Some("Application entry point".to_string())
        );
        assert_eq!(resource.mime_type, Some("text/x-rust".to_string()));
    }

    /// Test various resource URI schemes
    #[test]
    fn test_resource_various_uri_schemes() {
        let uris = vec![
            "file:///path/to/file.txt",
            "http://example.com/resource",
            "https://api.example.com/data",
            "git:///repo/file.rs",
            "mcp://server/resource",
            "custom://resource/identifier",
        ];

        for uri in uris {
            let response = JsonRpcResponse::success(
                RequestId::Number(1),
                json!({
                    "resources": [{
                        "uri": uri,
                        "name": "test"
                    }]
                }),
            );

            let result = ResourcesList::parse_response(response).unwrap();
            assert_eq!(result.resources[0].uri, uri);
        }
    }

    /// Test ResourcesList error response
    #[test]
    fn test_resources_list_error_response() {
        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            ltmatrix::mcp::protocol::errors::JsonRpcError::method_not_found("resources/list"),
        );

        let result = ResourcesList::parse_response(response);
        assert!(result.is_err());
    }
}

// ============================================================================
// Resources Read Method Tests
// ============================================================================

mod resources_read_method_tests {
    use super::*;

    /// Test that ResourcesRead method has correct method name
    #[test]
    fn test_resources_read_method_name() {
        assert_eq!(ResourcesRead::METHOD_NAME, "resources/read");
    }

    /// Test building resources/read request
    #[test]
    fn test_resources_read_build_request() {
        let params = ResourceReadParams::new("file:///project/package.json");
        let request = ResourcesRead::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "resources/read");
        let params_value = request.params.unwrap();
        assert_eq!(params_value["uri"], "file:///project/package.json");
    }

    /// Test ResourcesRead::params helper
    #[test]
    fn test_resources_read_params_helper() {
        let params = ResourcesRead::params("file:///test.txt");
        assert_eq!(params.uri, "file:///test.txt");
    }

    /// Test parsing successful resources/read response with text content
    #[test]
    fn test_resources_read_parse_text_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "contents": [
                    {
                        "uri": "file:///project/README.md",
                        "mime_type": "text/markdown",
                        "text": "# Project Title\n\nThis is the README content."
                    }
                ]
            }),
        );

        let result = ResourcesRead::parse_response(response).unwrap();

        assert_eq!(result.contents.len(), 1);
        assert_eq!(result.contents[0].uri, "file:///project/README.md");
        assert_eq!(
            result.contents[0].mime_type,
            Some("text/markdown".to_string())
        );
        assert_eq!(
            result.contents[0].text,
            Some("# Project Title\n\nThis is the README content.".to_string())
        );
        assert!(result.contents[0].blob.is_none());
    }

    /// Test parsing resources/read response with binary content (blob)
    #[test]
    fn test_resources_read_parse_blob_response() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "contents": [
                    {
                        "uri": "file:///project/image.png",
                        "mime_type": "image/png",
                        "blob": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
                    }
                ]
            }),
        );

        let result = ResourcesRead::parse_response(response).unwrap();

        assert_eq!(result.contents.len(), 1);
        assert_eq!(result.contents[0].uri, "file:///project/image.png");
        assert_eq!(result.contents[0].mime_type, Some("image/png".to_string()));
        assert!(result.contents[0].text.is_none());
        assert!(result.contents[0].blob.is_some());
    }

    /// Test parsing resources/read response with multiple contents
    #[test]
    fn test_resources_read_parse_multiple_contents() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "contents": [
                    {
                        "uri": "file:///project/file1.txt",
                        "mimeType": "text/plain",
                        "text": "Content of file 1"
                    },
                    {
                        "uri": "file:///project/file2.txt",
                        "mimeType": "text/plain",
                        "text": "Content of file 2"
                    }
                ]
            }),
        );

        let result = ResourcesRead::parse_response(response).unwrap();

        assert_eq!(result.contents.len(), 2);
        assert_eq!(
            result.contents[0].text,
            Some("Content of file 1".to_string())
        );
        assert_eq!(
            result.contents[1].text,
            Some("Content of file 2".to_string())
        );
    }

    /// Test ResourceContents helper methods
    #[test]
    fn test_resource_contents_text_helper() {
        let contents = ResourceContents::text("file:///test.txt", "Hello, World!");

        assert_eq!(contents.uri, "file:///test.txt");
        assert_eq!(contents.mime_type, Some("text/plain".to_string()));
        assert_eq!(contents.text, Some("Hello, World!".to_string()));
        assert!(contents.blob.is_none());
    }

    /// Test ResourceContents blob helper
    #[test]
    fn test_resource_contents_blob_helper() {
        let contents = ResourceContents::blob(
            "file:///binary.bin",
            "base64encodeddata",
            "application/octet-stream",
        );

        assert_eq!(contents.uri, "file:///binary.bin");
        assert_eq!(
            contents.mime_type,
            Some("application/octet-stream".to_string())
        );
        assert!(contents.text.is_none());
        assert_eq!(contents.blob, Some("base64encodeddata".to_string()));
    }

    /// Test resources/read error response for non-existent resource
    #[test]
    fn test_resources_read_not_found_error() {
        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            ltmatrix::mcp::protocol::errors::JsonRpcError::invalid_params(
                "Resource not found: file:///nonexistent.txt",
            ),
        );

        let result = ResourcesRead::parse_response(response);
        assert!(result.is_err());
    }

    /// Test resources/read with various MIME types
    #[test]
    fn test_resources_read_various_mime_types() {
        let mime_types = vec![
            ("text/plain", true),
            ("text/html", true),
            ("application/json", true),
            ("application/xml", true),
            ("image/png", false),
            ("image/jpeg", false),
            ("application/pdf", false),
            ("application/octet-stream", false),
        ];

        for (mime_type, is_text) in mime_types {
            let response = if is_text {
                JsonRpcResponse::success(
                    RequestId::Number(1),
                    json!({
                        "contents": [{
                            "uri": "file:///test",
                            "mime_type": mime_type,
                            "text": "sample content"
                        }]
                    }),
                )
            } else {
                JsonRpcResponse::success(
                    RequestId::Number(1),
                    json!({
                        "contents": [{
                            "uri": "file:///test",
                            "mime_type": mime_type,
                            "blob": "base64data"
                        }]
                    }),
                )
            };

            let result = ResourcesRead::parse_response(response).unwrap();
            assert_eq!(result.contents[0].mime_type, Some(mime_type.to_string()));
            if is_text {
                assert!(result.contents[0].text.is_some());
            } else {
                assert!(result.contents[0].blob.is_some());
            }
        }
    }

    /// Test empty content handling
    #[test]
    fn test_resources_read_empty_content() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "contents": [{
                    "uri": "file:///empty.txt",
                    "mimeType": "text/plain",
                    "text": ""
                }]
            }),
        );

        let result = ResourcesRead::parse_response(response).unwrap();

        assert_eq!(result.contents[0].text, Some("".to_string()));
    }
}

// ============================================================================
// Method Kind Registry Tests
// ============================================================================

mod method_kind_tests {
    use super::*;

    /// Test that Initialize is recognized as a known method
    #[test]
    fn test_initialize_method_kind() {
        assert_eq!(
            McpMethodKind::from_method_name("initialize"),
            Some(McpMethodKind::Initialize)
        );
        assert_eq!(McpMethodKind::Initialize.method_name(), "initialize");
    }

    /// Test that ResourcesList is recognized as a known method
    #[test]
    fn test_resources_list_method_kind() {
        assert_eq!(
            McpMethodKind::from_method_name("resources/list"),
            Some(McpMethodKind::ResourcesList)
        );
        assert_eq!(McpMethodKind::ResourcesList.method_name(), "resources/list");
    }

    /// Test that ResourcesRead is recognized as a known method
    #[test]
    fn test_resources_read_method_kind() {
        assert_eq!(
            McpMethodKind::from_method_name("resources/read"),
            Some(McpMethodKind::ResourcesRead)
        );
        assert_eq!(McpMethodKind::ResourcesRead.method_name(), "resources/read");
    }

    /// Test unknown method handling
    #[test]
    fn test_unknown_method_kind() {
        assert_eq!(McpMethodKind::from_method_name("unknown/method"), None);
        assert_eq!(McpMethodKind::from_method_name("custom"), None);
    }
}

// ============================================================================
// Protocol Version Tests
// ============================================================================

mod protocol_version_tests {
    use super::*;

    /// Test that the protocol version constant is correct
    #[test]
    fn test_protocol_version_constant() {
        assert_eq!(MCP_PROTOCOL_VERSION, "2025-11-25");
    }

    /// Test that InitializeParams uses correct protocol version by default
    #[test]
    fn test_initialize_params_default_protocol_version() {
        let params = InitializeParams::new("test", "1.0");
        assert_eq!(params.protocol_version, MCP_PROTOCOL_VERSION);
    }
}

// ============================================================================
// Serialization/Deserialization Edge Cases
// ============================================================================

mod serialization_edge_cases {
    use super::*;

    /// Test handling of unicode in resource URIs
    #[test]
    fn test_unicode_in_resource_uri() {
        let params = ResourceReadParams::new("file:///项目/文件.txt");
        let request = ResourcesRead::build_request(RequestId::Number(1), params);

        let params_value = request.params.unwrap();
        assert!(params_value["uri"].as_str().unwrap().contains("项目"));
    }

    /// Test handling of special characters in resource names
    #[test]
    fn test_special_chars_in_resource_name() {
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "resources": [{
                    "uri": "file:///test-file_v2.0 (copy).txt",
                    "name": "test-file_v2.0 (copy).txt"
                }]
            }),
        );

        let result = ResourcesList::parse_response(response).unwrap();
        assert_eq!(result.resources[0].name, "test-file_v2.0 (copy).txt");
    }

    /// Test handling of moderately long resource content (10KB)
    #[test]
    fn test_moderate_resource_content() {
        let long_text = "x".repeat(10_000); // 10KB of text (much faster than 1MB)

        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "contents": [{
                    "uri": "file:///large.txt",
                    "mimeType": "text/plain",
                    "text": long_text
                }]
            }),
        );

        let result = ResourcesRead::parse_response(response).unwrap();
        assert_eq!(result.contents[0].text.as_ref().unwrap().len(), 10_000);
    }

    /// Test handling of a reasonable number of resources
    #[test]
    fn test_many_resources_in_list() {
        let resources: Vec<serde_json::Value> = (0..100)
            .map(|i| {
                json!({
                    "uri": format!("file:///resource-{}.txt", i),
                    "name": format!("resource-{}.txt", i)
                })
            })
            .collect();

        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "resources": resources
            }),
        );

        let result = ResourcesList::parse_response(response).unwrap();
        assert_eq!(result.resources.len(), 100);
    }

    /// Test request ID variations
    #[test]
    fn test_request_id_variations() {
        let ids = vec![
            RequestId::Number(1),
            RequestId::Number(i64::MAX),
            RequestId::Number(i64::MIN),
            RequestId::String("uuid-1234-5678".to_string()),
            RequestId::String("".to_string()),
            RequestId::String("rocket-id".to_string()),
        ];

        for id in ids {
            let params = ResourcesListParams::default();
            let request = ResourcesList::build_request(id.clone(), params);
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
        let params = InitializeParams::new("test", "1.0");
        let request = Initialize::build_request(RequestId::Number(1), params);

        assert_eq!(request.jsonrpc, "2.0");
    }

    /// Test request serialization format
    #[test]
    fn test_request_serialization_format() {
        let params = ResourceReadParams::new("file:///test.txt");
        let request = ResourcesRead::build_request(RequestId::Number(42), params);

        let json = serde_json::to_string(&request).unwrap();

        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"method\":\"resources/read\""));
        assert!(json.contains("\"uri\":\"file:///test.txt\""));
    }

    /// Test response serialization format
    #[test]
    fn test_response_serialization_format() {
        let response = JsonRpcResponse::success(RequestId::Number(1), json!({"resources": []}));

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
    use ltmatrix::mcp::protocol::errors::McpErrorCode;

    /// Test error response parsing
    #[test]
    fn test_error_response_parsing() {
        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            ltmatrix::mcp::protocol::errors::JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
                data: Some(json!({ "details": "Missing required field" })),
            },
        );

        let result = Initialize::parse_response(response);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.code, McpErrorCode::InvalidParams);
    }

    /// Test method not found error
    #[test]
    fn test_method_not_found_error() {
        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            ltmatrix::mcp::protocol::errors::JsonRpcError::method_not_found("unknown"),
        );

        let result = ResourcesList::parse_response(response);
        assert!(result.is_err());
    }

    /// Test internal error response
    #[test]
    fn test_internal_error_response() {
        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            ltmatrix::mcp::protocol::errors::JsonRpcError::internal_error("Server error"),
        );

        let result = ResourcesRead::parse_response(response);
        assert!(result.is_err());
    }

    /// Test parse response with malformed JSON
    #[test]
    fn test_parse_response_malformed_json() {
        let result: Result<ResourcesListResult, _> = serde_json::from_str("{invalid json}");
        assert!(result.is_err());
    }
}

// ============================================================================
// Integration-like Tests (without actual transport)
// ============================================================================

mod integration_like_tests {
    use super::*;

    /// Simulate a full initialize flow
    #[test]
    fn test_simulated_initialize_flow() {
        // 1. Client builds request
        let client_params = Initialize::params("ltmatrix", "0.1.0");
        let request = Initialize::build_request(RequestId::Number(1), client_params);

        assert_eq!(request.method, "initialize");

        // 2. Server would process and respond
        let server_response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {
                    "tools": { "listChanged": true },
                    "resources": { "subscribe": true, "listChanged": true }
                },
                "serverInfo": {
                    "name": "test-server",
                    "version": "1.0.0"
                }
            }),
        );

        // 3. Client parses response
        let result = Initialize::parse_response(server_response).unwrap();

        assert_eq!(result.protocol_version, MCP_PROTOCOL_VERSION);
        assert_eq!(result.server_info.name, "test-server");
    }

    /// Simulate a full resources/list flow
    #[test]
    fn test_simulated_resources_list_flow() {
        // 1. Client builds request
        let params = ResourcesList::params();
        let request = ResourcesList::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "resources/list");

        // 2. Server responds with resources
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "resources": [
                    { "uri": "file:///file1.txt", "name": "file1.txt" },
                    { "uri": "file:///file2.txt", "name": "file2.txt" }
                ],
                "nextCursor": "page2"
            }),
        );

        // 3. Client parses response
        let result = ResourcesList::parse_response(response).unwrap();

        assert_eq!(result.resources.len(), 2);
        assert_eq!(result.next_cursor, Some("page2".to_string()));

        // 4. Client fetches next page
        let params_page2 = ResourcesList::params_with_cursor("page2");
        let request_page2 = ResourcesList::build_request(RequestId::Number(2), params_page2);

        let params_value = request_page2.params.unwrap();
        assert_eq!(params_value["cursor"], "page2");
    }

    /// Simulate a full resources/read flow
    #[test]
    fn test_simulated_resources_read_flow() {
        // 1. Client builds request for a specific resource
        let params = ResourcesRead::params("file:///project/package.json");
        let request = ResourcesRead::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "resources/read");

        // 2. Server responds with resource contents
        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "contents": [{
                    "uri": "file:///project/package.json",
                    "mime_type": "application/json",
                    "text": "{\n  \"name\": \"my-project\",\n  \"version\": \"1.0.0\"\n}"
                }]
            }),
        );

        // 3. Client parses and uses content
        let result = ResourcesRead::parse_response(response).unwrap();

        assert_eq!(result.contents.len(), 1);
        assert_eq!(
            result.contents[0].mime_type,
            Some("application/json".to_string())
        );

        let text = result.contents[0].text.as_ref().unwrap();
        assert!(text.contains("my-project"));
    }

    /// Test method kind detection for routing
    #[test]
    fn test_method_routing_detection() {
        let methods = vec![
            ("initialize", McpMethodKind::Initialize),
            ("resources/list", McpMethodKind::ResourcesList),
            ("resources/read", McpMethodKind::ResourcesRead),
        ];

        for (method_name, expected_kind) in methods {
            let detected = McpMethodKind::from_method_name(method_name);
            assert_eq!(detected, Some(expected_kind));

            // Verify round-trip
            assert_eq!(detected.unwrap().method_name(), method_name);
        }
    }
}
