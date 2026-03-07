// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Integration tests for MCP protocol error types
//!
//! Tests for:
//! - JsonRpcErrorCode: Standard JSON-RPC 2.0 error codes
//! - McpErrorCode: MCP-specific error codes
//! - JsonRpcError: JSON-RPC error object serialization
//! - McpError: Comprehensive MCP error with context
//! - ErrorBuilder: Builder pattern for error construction
//! - Error conversions and interoperability
//! - Error categories and classification
//! - Recovery and retry semantics

use ltmatrix::mcp::protocol::errors::{
    ErrorBuilder, ErrorCategory, JsonRpcError, JsonRpcErrorCode, McpError, McpErrorCode,
};
use serde_json::json;
use std::time::Duration;

// ============================================================================
// JsonRpcErrorCode Tests
// ============================================================================

mod json_rpc_error_code_tests {
    use super::*;

    #[test]
    fn test_standard_codes_roundtrip() {
        let codes = [
            (-32700, JsonRpcErrorCode::ParseError),
            (-32600, JsonRpcErrorCode::InvalidRequest),
            (-32601, JsonRpcErrorCode::MethodNotFound),
            (-32602, JsonRpcErrorCode::InvalidParams),
            (-32603, JsonRpcErrorCode::InternalError),
        ];

        for (code, expected) in codes {
            assert_eq!(JsonRpcErrorCode::from_i32(code), expected);
            assert_eq!(expected.as_i32(), code);
        }
    }

    #[test]
    fn test_server_error_range() {
        // Server errors range: -32000 to -32099
        for code in -32099..=-32000 {
            let error_code = JsonRpcErrorCode::from_i32(code);
            assert!(matches!(error_code, JsonRpcErrorCode::ServerError(_)));
            assert_eq!(error_code.as_i32(), code);
        }
    }

    #[test]
    fn test_unknown_code_defaults_to_internal() {
        // Unknown codes should default to InternalError
        assert_eq!(
            JsonRpcErrorCode::from_i32(-99999),
            JsonRpcErrorCode::InternalError
        );
        assert_eq!(
            JsonRpcErrorCode::from_i32(0),
            JsonRpcErrorCode::InternalError
        );
    }

    #[test]
    fn test_display_format() {
        let display = format!("{}", JsonRpcErrorCode::ParseError);
        assert!(display.contains("Parse error"));
        assert!(display.contains("-32700"));
    }

    #[test]
    fn test_message_method() {
        assert_eq!(JsonRpcErrorCode::ParseError.message(), "Parse error");
        assert_eq!(
            JsonRpcErrorCode::InvalidRequest.message(),
            "Invalid Request"
        );
        assert_eq!(
            JsonRpcErrorCode::MethodNotFound.message(),
            "Method not found"
        );
        assert_eq!(JsonRpcErrorCode::InvalidParams.message(), "Invalid params");
        assert_eq!(JsonRpcErrorCode::InternalError.message(), "Internal error");
        assert_eq!(
            JsonRpcErrorCode::ServerError(-32050).message(),
            "Server error"
        );
    }
}

// ============================================================================
// McpErrorCode Tests
// ============================================================================

mod mcp_error_code_tests {
    use super::*;

    #[test]
    fn test_json_rpc_standard_codes() {
        // JSON-RPC standard codes should work
        assert_eq!(McpErrorCode::ParseError.as_i32(), -32700);
        assert_eq!(McpErrorCode::InvalidRequest.as_i32(), -32600);
        assert_eq!(McpErrorCode::MethodNotFound.as_i32(), -32601);
        assert_eq!(McpErrorCode::InvalidParams.as_i32(), -32602);
        assert_eq!(McpErrorCode::InternalError.as_i32(), -32603);
    }

    #[test]
    fn test_server_error_codes() {
        assert_eq!(McpErrorCode::ServerShutdown.as_i32(), -32000);
        assert_eq!(McpErrorCode::ServerStarting.as_i32(), -32001);
        assert_eq!(McpErrorCode::RequestTimeout.as_i32(), -32002);
        assert_eq!(McpErrorCode::TooManyRequests.as_i32(), -32003);
        assert_eq!(McpErrorCode::RateLimitExceeded.as_i32(), -32004);
        assert_eq!(McpErrorCode::ResourceUnavailable.as_i32(), -32005);
    }

    #[test]
    fn test_mcp_specific_codes() {
        assert_eq!(McpErrorCode::UnknownCapability.as_i32(), -32500);
        assert_eq!(McpErrorCode::CapabilityNotSupported.as_i32(), -32501);
        assert_eq!(McpErrorCode::ToolNotFound.as_i32(), -32502);
        assert_eq!(McpErrorCode::ToolExecutionError.as_i32(), -32503);
        assert_eq!(McpErrorCode::ResourceNotFound.as_i32(), -32504);
        assert_eq!(McpErrorCode::ResourceAccessDenied.as_i32(), -32505);
        assert_eq!(McpErrorCode::PromptNotFound.as_i32(), -32506);
        assert_eq!(McpErrorCode::InvalidPromptArguments.as_i32(), -32507);
        assert_eq!(McpErrorCode::SamplingNotSupported.as_i32(), -32508);
        assert_eq!(McpErrorCode::ContentTypeNotSupported.as_i32(), -32509);
        assert_eq!(McpErrorCode::InvalidUri.as_i32(), -32510);
        assert_eq!(McpErrorCode::SubscriptionError.as_i32(), -32511);
        assert_eq!(McpErrorCode::TransportError.as_i32(), -32512);
        assert_eq!(McpErrorCode::VersionMismatch.as_i32(), -32513);
        assert_eq!(McpErrorCode::SessionError.as_i32(), -32514);
        assert_eq!(McpErrorCode::ConfigurationError.as_i32(), -32515);
    }

    #[test]
    fn test_custom_server_error_range() {
        // Custom server errors: -32006 to -32099
        for code in [-32050, -32075, -32099] {
            let error_code = McpErrorCode::from_i32(code);
            assert!(matches!(error_code, McpErrorCode::ServerError(_)));
            assert_eq!(error_code.as_i32(), code);
        }
    }

    #[test]
    fn test_custom_mcp_error_range() {
        // Custom MCP errors: -32550 to -32599
        for code in [-32550, -32575, -32599] {
            let error_code = McpErrorCode::from_i32(code);
            assert!(matches!(error_code, McpErrorCode::Custom(_)));
            assert_eq!(error_code.as_i32(), code);
        }
    }

    #[test]
    fn test_is_json_rpc_standard() {
        assert!(McpErrorCode::ParseError.is_json_rpc_standard());
        assert!(McpErrorCode::InvalidRequest.is_json_rpc_standard());
        assert!(McpErrorCode::MethodNotFound.is_json_rpc_standard());
        assert!(McpErrorCode::InvalidParams.is_json_rpc_standard());
        assert!(McpErrorCode::InternalError.is_json_rpc_standard());

        assert!(!McpErrorCode::RequestTimeout.is_json_rpc_standard());
        assert!(!McpErrorCode::ToolNotFound.is_json_rpc_standard());
    }

    #[test]
    fn test_is_server_error() {
        assert!(McpErrorCode::ServerShutdown.is_server_error());
        assert!(McpErrorCode::ServerStarting.is_server_error());
        assert!(McpErrorCode::RequestTimeout.is_server_error());
        assert!(McpErrorCode::TooManyRequests.is_server_error());
        assert!(McpErrorCode::RateLimitExceeded.is_server_error());
        assert!(McpErrorCode::ResourceUnavailable.is_server_error());
        assert!(McpErrorCode::ServerError(-32050).is_server_error());

        assert!(!McpErrorCode::ParseError.is_server_error());
        assert!(!McpErrorCode::ToolNotFound.is_server_error());
    }

    #[test]
    fn test_is_mcp_specific() {
        assert!(McpErrorCode::ToolNotFound.is_mcp_specific());
        assert!(McpErrorCode::ToolExecutionError.is_mcp_specific());
        assert!(McpErrorCode::ResourceNotFound.is_mcp_specific());
        assert!(McpErrorCode::ResourceAccessDenied.is_mcp_specific());
        assert!(McpErrorCode::PromptNotFound.is_mcp_specific());
        assert!(McpErrorCode::InvalidUri.is_mcp_specific());

        assert!(!McpErrorCode::ParseError.is_mcp_specific());
        assert!(!McpErrorCode::RequestTimeout.is_mcp_specific());
    }

    #[test]
    fn test_is_recoverable() {
        // Recoverable errors
        assert!(McpErrorCode::RequestTimeout.is_recoverable());
        assert!(McpErrorCode::TooManyRequests.is_recoverable());
        assert!(McpErrorCode::RateLimitExceeded.is_recoverable());
        assert!(McpErrorCode::ResourceUnavailable.is_recoverable());
        assert!(McpErrorCode::ServerStarting.is_recoverable());
        assert!(McpErrorCode::TransportError.is_recoverable());

        // Non-recoverable errors
        assert!(!McpErrorCode::ParseError.is_recoverable());
        assert!(!McpErrorCode::InvalidRequest.is_recoverable());
        assert!(!McpErrorCode::MethodNotFound.is_recoverable());
        assert!(!McpErrorCode::ToolNotFound.is_recoverable());
        assert!(!McpErrorCode::ResourceAccessDenied.is_recoverable());
    }

    #[test]
    fn test_conversion_from_json_rpc_code() {
        let json_code = JsonRpcErrorCode::ParseError;
        let mcp_code: McpErrorCode = json_code.into();
        assert_eq!(mcp_code, McpErrorCode::ParseError);

        let json_code = JsonRpcErrorCode::ServerError(-32050);
        let mcp_code: McpErrorCode = json_code.into();
        assert_eq!(mcp_code, McpErrorCode::ServerError(-32050));
    }

    #[test]
    fn test_conversion_to_json_rpc_code() {
        // Standard codes should convert directly
        let mcp_code = McpErrorCode::ParseError;
        let json_code: JsonRpcErrorCode = mcp_code.into();
        assert_eq!(json_code, JsonRpcErrorCode::ParseError);

        // MCP-specific codes should map to InternalError
        let mcp_code = McpErrorCode::ToolNotFound;
        let json_code: JsonRpcErrorCode = mcp_code.into();
        assert_eq!(json_code, JsonRpcErrorCode::InternalError);
    }

    #[test]
    fn test_display_format() {
        let display = format!("{}", McpErrorCode::ToolNotFound);
        assert!(display.contains("Tool not found"));
        assert!(display.contains("-32502"));
    }
}

// ============================================================================
// ErrorCategory Tests
// ============================================================================

mod error_category_tests {
    use super::*;

    #[test]
    fn test_all_categories_display() {
        assert_eq!(format!("{}", ErrorCategory::Protocol), "Protocol");
        assert_eq!(format!("{}", ErrorCategory::Serialization), "Serialization");
        assert_eq!(
            format!("{}", ErrorCategory::Communication),
            "Communication"
        );
        assert_eq!(
            format!("{}", ErrorCategory::ToolExecution),
            "ToolExecution"
        );
        assert_eq!(
            format!("{}", ErrorCategory::ResourceAccess),
            "ResourceAccess"
        );
        assert_eq!(format!("{}", ErrorCategory::Configuration), "Configuration");
        assert_eq!(format!("{}", ErrorCategory::Internal), "Internal");
    }

    #[test]
    fn test_category_equality() {
        assert_eq!(ErrorCategory::Protocol, ErrorCategory::Protocol);
        assert_ne!(ErrorCategory::Protocol, ErrorCategory::Internal);
    }
}

// ============================================================================
// JsonRpcError Tests
// ============================================================================

mod json_rpc_error_tests {
    use super::*;

    #[test]
    fn test_basic_creation() {
        let error = JsonRpcError::new(JsonRpcErrorCode::InternalError, "Something went wrong".to_string());

        assert_eq!(error.code, -32603);
        assert_eq!(error.message, "Something went wrong");
        assert!(error.data.is_none());
    }

    #[test]
    fn test_with_data() {
        let error = JsonRpcError::with_data(
            JsonRpcErrorCode::InvalidParams,
            "Missing required field".to_string(),
            json!({ "field": "url", "required": true }),
        );

        assert_eq!(error.code, -32602);
        assert!(error.data.is_some());
        let data = error.data.unwrap();
        assert_eq!(data["field"], "url");
    }

    #[test]
    fn test_helper_constructors() {
        let parse_error = JsonRpcError::parse_error("Unexpected EOF");
        assert_eq!(parse_error.code, -32700);
        assert!(parse_error.message.contains("Unexpected EOF"));

        let invalid_request = JsonRpcError::invalid_request("Missing jsonrpc version");
        assert_eq!(invalid_request.code, -32600);

        let method_not_found = JsonRpcError::method_not_found("unknown_method");
        assert_eq!(method_not_found.code, -32601);
        assert!(method_not_found.data.is_some());

        let invalid_params = JsonRpcError::invalid_params("url is required");
        assert_eq!(invalid_params.code, -32602);

        let internal = JsonRpcError::internal_error("Timeout");
        assert_eq!(internal.code, -32603);
    }

    #[test]
    fn test_serialization() {
        let error = JsonRpcError::method_not_found("test_method");
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("\"code\":-32601"));
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"data\""));
        assert!(json.contains("\"method\":\"test_method\""));
    }

    #[test]
    fn test_serialization_skip_none_data() {
        let error = JsonRpcError::new(
            JsonRpcErrorCode::InternalError,
            "No data".to_string(),
        );
        let json = serde_json::to_string(&error).unwrap();

        assert!(!json.contains("\"data\""));
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{"code":-32601,"message":"Method not found: test","data":{"method":"test"}}"#;
        let error: JsonRpcError = serde_json::from_str(json).unwrap();

        assert_eq!(error.code, -32601);
        assert!(error.message.contains("Method not found"));
        assert!(error.data.is_some());
    }

    #[test]
    fn test_deserialization_without_data() {
        let json = r#"{"code":-32603,"message":"Internal error"}"#;
        let error: JsonRpcError = serde_json::from_str(json).unwrap();

        assert_eq!(error.code, -32603);
        assert!(error.data.is_none());
    }

    #[test]
    fn test_code_enum_conversion() {
        let error = JsonRpcError::new(JsonRpcErrorCode::MethodNotFound, "test".to_string());
        assert_eq!(error.code_enum(), JsonRpcErrorCode::MethodNotFound);
    }

    #[test]
    fn test_to_mcp_error() {
        let json_error = JsonRpcError::method_not_found("test");
        let mcp_error = json_error.to_mcp_error();

        assert_eq!(mcp_error.code, McpErrorCode::MethodNotFound);
        assert_eq!(mcp_error.category, ErrorCategory::Protocol);
    }

    #[test]
    fn test_display_format() {
        let error = JsonRpcError::method_not_found("test");
        let display = format!("{}", error);

        assert!(display.contains("-32601"));
        assert!(display.contains("Method not found"));
    }

    #[test]
    fn test_is_std_error() {
        let error = JsonRpcError::internal_error("test");
        let _: &dyn std::error::Error = &error;
    }
}

// ============================================================================
// McpError Tests
// ============================================================================

mod mcp_error_tests {
    use super::*;

    #[test]
    fn test_basic_creation() {
        let error = McpError::new(McpErrorCode::InternalError, "Something failed");

        assert_eq!(error.code, McpErrorCode::InternalError);
        assert_eq!(error.message, "Something failed");
        assert_eq!(error.category, ErrorCategory::Internal);
        assert!(error.data.is_none());
        assert!(error.source.is_none());
        assert!(error.request_id.is_none());
    }

    #[test]
    fn test_protocol_error() {
        let error = McpError::protocol(McpErrorCode::InvalidRequest, "Missing jsonrpc field");

        assert_eq!(error.code, McpErrorCode::InvalidRequest);
        assert_eq!(error.category, ErrorCategory::Protocol);
    }

    #[test]
    fn test_serialization_error() {
        let error = McpError::serialization("Invalid JSON at line 5");

        assert_eq!(error.code, McpErrorCode::ParseError);
        assert_eq!(error.category, ErrorCategory::Serialization);
    }

    #[test]
    fn test_communication_error() {
        let error = McpError::communication("Connection refused");

        assert_eq!(error.code, McpErrorCode::TransportError);
        assert_eq!(error.category, ErrorCategory::Communication);
    }

    #[test]
    fn test_tool_execution_error() {
        let error = McpError::tool_execution("playwright", "Browser launch failed");

        assert_eq!(error.code, McpErrorCode::ToolExecutionError);
        assert_eq!(error.category, ErrorCategory::ToolExecution);
        assert!(error.message.contains("playwright"));
        assert!(error.message.contains("Browser launch failed"));
        assert!(error.data.is_some());
        assert_eq!(error.data.unwrap()["tool"], "playwright");
    }

    #[test]
    fn test_tool_not_found_error() {
        let error = McpError::tool_not_found("nonexistent_tool");

        assert_eq!(error.code, McpErrorCode::ToolNotFound);
        assert_eq!(error.category, ErrorCategory::ToolExecution);
        assert!(error.message.contains("nonexistent_tool"));
    }

    #[test]
    fn test_resource_not_found_error() {
        let error = McpError::resource_not_found("file:///nonexistent/path.txt");

        assert_eq!(error.code, McpErrorCode::ResourceNotFound);
        assert_eq!(error.category, ErrorCategory::ResourceAccess);
        assert!(error.message.contains("file:///nonexistent/path.txt"));
        assert_eq!(error.data.unwrap()["uri"], "file:///nonexistent/path.txt");
    }

    #[test]
    fn test_resource_access_denied_error() {
        let error = McpError::resource_access_denied("file:///secure/file.txt", "Permission denied");

        assert_eq!(error.code, McpErrorCode::ResourceAccessDenied);
        assert_eq!(error.category, ErrorCategory::ResourceAccess);
        assert!(error.message.contains("Permission denied"));
        let data = error.data.unwrap();
        assert_eq!(data["uri"], "file:///secure/file.txt");
        assert_eq!(data["reason"], "Permission denied");
    }

    #[test]
    fn test_configuration_error() {
        let error = McpError::configuration("Invalid MCP server configuration");

        assert_eq!(error.code, McpErrorCode::ConfigurationError);
        assert_eq!(error.category, ErrorCategory::Configuration);
    }

    #[test]
    fn test_timeout_error() {
        let duration = Duration::from_secs(30);
        let error = McpError::timeout("tool_execution", duration);

        assert_eq!(error.code, McpErrorCode::RequestTimeout);
        assert_eq!(error.category, ErrorCategory::Communication);
        assert!(error.is_recoverable());
        assert_eq!(error.retry_delay(), Some(duration));
        assert!(error.data.is_some());
    }

    #[test]
    fn test_with_data_builder() {
        let error = McpError::tool_not_found("test")
            .with_data(json!({ "additional": "info" }));

        assert!(error.data.is_some());
        assert_eq!(error.data.unwrap()["additional"], "info");
    }

    #[test]
    fn test_with_source_builder() {
        let error = McpError::communication("Failed")
            .with_source("transport_layer");

        assert_eq!(error.source, Some("transport_layer".to_string()));
    }

    #[test]
    fn test_with_request_id_builder() {
        let error = McpError::protocol(McpErrorCode::InvalidRequest, "Bad request")
            .with_request_id(json!("req-123"));

        assert_eq!(error.request_id, Some(json!("req-123")));
    }

    #[test]
    fn test_with_retry_builder() {
        let duration = Duration::from_secs(5);
        let error = McpError::communication("Temporary failure")
            .with_retry(duration);

        assert!(error.is_recoverable());
        assert_eq!(error.retry_delay(), Some(duration));
    }

    #[test]
    fn test_to_json_rpc_error() {
        let mcp_error = McpError::tool_execution("test_tool", "failed");
        let json_error = mcp_error.to_json_rpc_error();

        assert_eq!(json_error.code, -32503);
        assert!(json_error.message.contains("test_tool"));
        assert!(json_error.data.is_some());
    }

    #[test]
    fn test_to_json() {
        let error = McpError::tool_not_found("my_tool");
        let json = error.to_json();

        assert_eq!(json["code"], -32502);
        assert!(json["message"].as_str().unwrap().contains("my_tool"));
    }

    #[test]
    fn test_from_json_rpc_error() {
        let json_error = JsonRpcError::method_not_found("test");
        let mcp_error: McpError = json_error.into();

        assert_eq!(mcp_error.code, McpErrorCode::MethodNotFound);
        assert_eq!(mcp_error.category, ErrorCategory::Protocol);
    }

    #[test]
    fn test_from_serde_json_error() {
        let serde_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let mcp_error: McpError = serde_error.into();

        assert_eq!(mcp_error.code, McpErrorCode::ParseError);
        assert_eq!(mcp_error.category, ErrorCategory::Serialization);
    }

    #[test]
    fn test_from_io_error_timeout() {
        let io_error = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
        let mcp_error: McpError = io_error.into();

        assert_eq!(mcp_error.code, McpErrorCode::RequestTimeout);
        assert_eq!(mcp_error.category, ErrorCategory::Communication);
    }

    #[test]
    fn test_from_io_error_connection_refused() {
        let io_error = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
        let mcp_error: McpError = io_error.into();

        assert_eq!(mcp_error.code, McpErrorCode::TransportError);
        assert_eq!(mcp_error.category, ErrorCategory::Communication);
    }

    #[test]
    fn test_from_io_error_would_block() {
        let io_error = std::io::Error::new(std::io::ErrorKind::WouldBlock, "would block");
        let mcp_error: McpError = io_error.into();

        assert_eq!(mcp_error.code, McpErrorCode::ResourceUnavailable);
    }

    #[test]
    fn test_display_format() {
        let error = McpError::tool_execution("test", "failed")
            .with_source("test_module")
            .with_data(json!({ "extra": "info" }));

        let display = format!("{}", error);

        assert!(display.contains("-32503"));
        assert!(display.contains("ToolExecution"));
        assert!(display.contains("test_module"));
        assert!(display.contains("extra"));
    }

    #[test]
    fn test_is_std_error() {
        let error = McpError::new(McpErrorCode::InternalError, "test");
        let _: &dyn std::error::Error = &error;
    }

    #[test]
    fn test_recoverable_from_code() {
        // Recoverable codes should set recoverable automatically
        let error = McpError::new(McpErrorCode::RequestTimeout, "timeout");
        assert!(error.is_recoverable());

        // Non-recoverable codes should not
        let error = McpError::new(McpErrorCode::ToolNotFound, "not found");
        assert!(!error.is_recoverable());
    }
}

// ============================================================================
// ErrorBuilder Tests
// ============================================================================

mod error_builder_tests {
    use super::*;

    #[test]
    fn test_builder_all_fields() {
        let error = ErrorBuilder::new()
            .code(McpErrorCode::ToolExecutionError)
            .message("Custom error message")
            .category(ErrorCategory::ToolExecution)
            .data(json!({ "tool": "my_tool", "attempt": 3 }))
            .source("tool_executor")
            .request_id(json!("req-456"))
            .recoverable(true)
            .retry_after(Duration::from_secs(10))
            .build();

        assert_eq!(error.code, McpErrorCode::ToolExecutionError);
        assert_eq!(error.message, "Custom error message");
        assert_eq!(error.category, ErrorCategory::ToolExecution);
        assert!(error.data.is_some());
        assert_eq!(error.source, Some("tool_executor".to_string()));
        assert_eq!(error.request_id, Some(json!("req-456")));
        assert!(error.is_recoverable());
        assert_eq!(error.retry_delay(), Some(Duration::from_secs(10)));
    }

    #[test]
    fn test_builder_minimal() {
        let error = ErrorBuilder::new()
            .code(McpErrorCode::InternalError)
            .build();

        // Default message should come from code
        assert_eq!(error.code, McpErrorCode::InternalError);
        assert!(!error.message.is_empty());
        assert_eq!(error.category, ErrorCategory::Internal);
    }

    #[test]
    fn test_builder_default_values() {
        let error = ErrorBuilder::new().build();

        assert_eq!(error.code, McpErrorCode::InternalError);
        assert_eq!(error.category, ErrorCategory::Internal);
        assert!(!error.is_recoverable());
    }

    #[test]
    fn test_builder_recoverable_defaults_from_code() {
        let error = ErrorBuilder::new()
            .code(McpErrorCode::RequestTimeout)
            .build();

        // Should be recoverable because RequestTimeout is recoverable
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_builder_override_recoverable() {
        let error = ErrorBuilder::new()
            .code(McpErrorCode::RequestTimeout) // Normally recoverable
            .recoverable(false) // Override
            .build();

        assert!(!error.is_recoverable());
    }

    #[test]
    fn test_builder_chaining() {
        // Verify builder returns Self for chaining
        let error = ErrorBuilder::new()
            .code(McpErrorCode::ToolNotFound)
            .message("Tool missing")
            .category(ErrorCategory::ToolExecution)
            .data(json!({}))
            .source("test")
            .request_id(json!(1))
            .recoverable(false)
            .retry_after(Duration::ZERO)
            .build();

        assert_eq!(error.code, McpErrorCode::ToolNotFound);
    }
}

// ============================================================================
// Error Conversion Chain Tests
// ============================================================================

mod error_conversion_tests {
    use super::*;

    #[test]
    fn test_json_rpc_to_mcp_and_back() {
        let original = JsonRpcError::method_not_found("test_method");
        let mcp: McpError = original.clone().into();
        let back: JsonRpcError = mcp.into();

        assert_eq!(back.code, original.code);
        assert_eq!(back.message, original.message);
    }

    #[test]
    fn test_io_to_mcp_to_json_rpc_chain() {
        let io_error = std::io::Error::new(std::io::ErrorKind::TimedOut, "operation timed out");
        let mcp_error: McpError = io_error.into();
        let json_error: JsonRpcError = mcp_error.into();

        assert_eq!(json_error.code, -32002); // RequestTimeout
    }

    #[test]
    fn test_serde_to_mcp_to_json_rpc_chain() {
        let serde_error = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let mcp_error: McpError = serde_error.into();
        let json_error: JsonRpcError = mcp_error.into();

        assert_eq!(json_error.code, -32700); // ParseError
    }
}

// ============================================================================
// MCP-Specific Error Scenarios Tests
// ============================================================================

mod mcp_error_scenarios_tests {
    use super::*;

    #[test]
    fn test_tool_execution_failure_scenario() {
        // Simulate a tool execution failure
        fn execute_tool(name: &str, _args: &serde_json::Value) -> Result<String, McpError> {
            if name == "valid_tool" {
                Ok("success".to_string())
            } else {
                Err(McpError::tool_not_found(name))
            }
        }

        let result = execute_tool("invalid_tool", &json!({}));
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, McpErrorCode::ToolNotFound);
    }

    #[test]
    fn test_resource_access_scenario() {
        // Simulate resource access with permission check
        fn read_resource(uri: &str, has_permission: bool) -> Result<String, McpError> {
            if !has_permission {
                return Err(McpError::resource_access_denied(uri, "Insufficient permissions"));
            }
            if uri.contains("nonexistent") {
                return Err(McpError::resource_not_found(uri));
            }
            Ok("content".to_string())
        }

        // Access denied case
        let result = read_resource("file:///secure.txt", false);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, McpErrorCode::ResourceAccessDenied);

        // Not found case
        let result = read_resource("file:///nonexistent.txt", true);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, McpErrorCode::ResourceNotFound);
    }

    #[test]
    fn test_timeout_with_retry_scenario() {
        // Simulate an operation that times out
        fn operation_with_timeout(duration: Duration, timeout: Duration) -> Result<String, McpError> {
            if duration > timeout {
                Err(McpError::timeout("long_operation", timeout))
            } else {
                Ok("completed".to_string())
            }
        }

        let result = operation_with_timeout(Duration::from_secs(60), Duration::from_secs(30));
        assert!(result.is_err());
        let error = result.unwrap_err();

        assert!(error.is_recoverable());
        assert!(error.retry_delay().is_some());
    }

    #[test]
    fn test_protocol_version_mismatch_scenario() {
        // Simulate protocol version mismatch
        fn check_version(client_version: &str, server_version: &str) -> Result<(), McpError> {
            if client_version != server_version {
                Err(McpError::with_category(
                    McpErrorCode::VersionMismatch,
                    format!(
                        "Client version {} does not match server version {}",
                        client_version, server_version
                    ),
                    ErrorCategory::Protocol,
                ))
            } else {
                Ok(())
            }
        }

        let result = check_version("2024-01-01", "2025-11-25");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, McpErrorCode::VersionMismatch);
        assert_eq!(error.category, ErrorCategory::Protocol);
    }
}

// ============================================================================
// Result Type Alias Tests
// ============================================================================

mod result_type_tests {
    use ltmatrix::mcp::protocol::errors::{JsonRpcResult, McpResult};

    use super::*;

    #[test]
    fn test_mcp_result_ok() {
        let result: McpResult<String> = Ok("success".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[test]
    fn test_mcp_result_err() {
        let result: McpResult<String> = Err(McpError::tool_not_found("test"));
        assert!(result.is_err());
    }

    #[test]
    fn test_json_rpc_result_ok() {
        let result: JsonRpcResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_json_rpc_result_err() {
        let result: JsonRpcResult<String> = Err(JsonRpcError::internal_error("failed"));
        assert!(result.is_err());
    }
}

// ============================================================================
// Edge Cases and Boundary Tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_error_message() {
        let error = McpError::new(McpErrorCode::InternalError, "");
        assert_eq!(error.message, "");
    }

    #[test]
    fn test_very_long_error_message() {
        let long_message = "x".repeat(10000);
        let error = McpError::new(McpErrorCode::InternalError, long_message.clone());
        assert_eq!(error.message.len(), 10000);
    }

    #[test]
    fn test_unicode_in_error_message() {
        let error = McpError::tool_execution("测试工具", "执行失败 🚫");
        assert!(error.message.contains("测试工具"));
        assert!(error.message.contains("🚫"));
    }

    #[test]
    fn test_nested_json_in_data() {
        let nested_data = json!({
            "level1": {
                "level2": {
                    "level3": ["a", "b", "c"]
                }
            }
        });
        let error = McpError::tool_execution("test", "failed")
            .with_data(nested_data.clone());

        let json = error.to_json();
        assert_eq!(json["data"]["level1"]["level2"]["level3"], json!(["a", "b", "c"]));
    }

    #[test]
    fn test_error_with_null_request_id() {
        let error = McpError::protocol(McpErrorCode::InvalidRequest, "test")
            .with_request_id(json!(null));

        assert_eq!(error.request_id, Some(json!(null)));
    }

    #[test]
    fn test_error_with_numeric_request_id() {
        let error = McpError::protocol(McpErrorCode::InvalidRequest, "test")
            .with_request_id(json!(12345));

        assert_eq!(error.request_id, Some(json!(12345)));
    }

    #[test]
    fn test_zero_duration_retry() {
        let error = McpError::timeout("test", Duration::ZERO);
        assert_eq!(error.retry_delay(), Some(Duration::ZERO));
    }

    #[test]
    fn test_large_duration_retry() {
        // Use a large but serializable duration (584 years in seconds)
        let large_duration = Duration::from_secs(u32::MAX as u64);
        let error = McpError::timeout("test", large_duration);
        assert_eq!(error.retry_delay(), Some(large_duration));
    }
}
