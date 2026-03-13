// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Integration tests for MCP request routing and response parsing

use ltmatrix::mcp::protocol::{McpError, McpErrorCode};
use ltmatrix::mcp::{
    ClientCapabilities,
    ImplementationInfo,
    // Method types
    Initialize,
    // Parameter and result types
    InitializeParams,
    InitializeResult,
    JsonRpcError,
    JsonRpcErrorCode,
    JsonRpcNotification,
    // Protocol types
    JsonRpcRequest,
    JsonRpcResponse,
    MessageClassifier,
    MessageKind,
    Ping,
    RequestBuilder,
    RequestId,
    // Router types
    RequestRouter,
    // Correlation types
    RequestTracker,
    ResponseCorrelator,
    ResponseParser,
    ServerCapabilities,
    ToolsList,
    ToolsListResult,
    TypedResponse,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// ResponseParser Tests
// ============================================================================

#[test]
fn test_response_parser_initialize_result() {
    let parser = ResponseParser::new();

    let response = JsonRpcResponse::success(
        RequestId::Number(1),
        json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {
                "tools": {},
                "resources": {}
            },
            "serverInfo": {
                "name": "test-server",
                "version": "2.0.0"
            }
        }),
    );

    let result: InitializeResult = parser.parse_response(response).unwrap();
    assert_eq!(result.protocol_version, "2025-11-25");
    assert_eq!(result.server_info.name, "test-server");
    assert_eq!(result.server_info.version, "2.0.0");
}

#[test]
fn test_response_parser_tools_list() {
    let parser = ResponseParser::new();

    let response = JsonRpcResponse::success(
        RequestId::Number(2),
        json!({
            "tools": [
                {
                    "name": "browser_navigate",
                    "description": "Navigate to a URL",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "url": {"type": "string"}
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
                            "selector": {"type": "string"}
                        },
                        "required": ["selector"]
                    }
                }
            ]
        }),
    );

    let result: ToolsListResult = parser.parse_response(response).unwrap();
    assert_eq!(result.tools.len(), 2);
    assert_eq!(result.tools[0].name, "browser_navigate");
    assert_eq!(result.tools[1].name, "browser_click");
}

#[test]
fn test_response_parser_error_handling() {
    let parser = ResponseParser::new();

    let error = JsonRpcError::new(
        JsonRpcErrorCode::InvalidRequest,
        "Invalid Request".to_string(),
    );
    let response = JsonRpcResponse::error(RequestId::Number(3), error);

    let result = parser.parse_response::<InitializeResult>(response);
    assert!(result.is_err());
}

#[test]
fn test_typed_response_creation() {
    let raw_response = JsonRpcResponse::success(
        RequestId::Number(1),
        json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "serverInfo": {
                "name": "test",
                "version": "1.0"
            }
        }),
    );

    let result = InitializeResult {
        protocol_version: "2025-11-25".to_string(),
        capabilities: ServerCapabilities::default(),
        server_info: ImplementationInfo {
            name: "test".to_string(),
            version: "1.0".to_string(),
        },
        instructions: None,
    };

    let typed = TypedResponse::new(raw_response, result, "initialize".to_string());
    assert_eq!(typed.method, "initialize");
    assert_eq!(typed.result.server_info.name, "test");
}

// ============================================================================
// RequestBuilder Tests
// ============================================================================

#[test]
fn test_request_builder_initialize() {
    let params = InitializeParams {
        protocol_version: ltmatrix::mcp::MCP_PROTOCOL_VERSION.to_string(),
        capabilities: ClientCapabilities::default(),
        client_info: ImplementationInfo {
            name: "test-client".to_string(),
            version: "1.0.0".to_string(),
        },
    };

    let builder = RequestBuilder::new(RequestId::Number(1), "initialize")
        .params(&params)
        .unwrap();

    let request = builder.build();
    assert_eq!(request.id, RequestId::Number(1));
    assert_eq!(request.method, "initialize");
}

#[test]
fn test_request_builder_serialization() {
    let builder = RequestBuilder::new(RequestId::Number(42), "tools/list")
        .params(&json!({"cursor": "abc123"}))
        .unwrap();

    let json_str = builder.to_json().unwrap();
    assert!(json_str.contains("\"id\":42"));
    assert!(json_str.contains("\"method\":\"tools/list\""));
    assert!(json_str.contains("\"cursor\":\"abc123\""));
}

// ============================================================================
// MessageClassifier Tests
// ============================================================================

#[test]
fn test_message_classifier_request() {
    let json_str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let kind = MessageClassifier::classify(json_str).unwrap();
    assert!(matches!(kind, MessageKind::Request));
}

#[test]
fn test_message_classifier_response() {
    let json_str = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
    let kind = MessageClassifier::classify(json_str).unwrap();
    assert!(matches!(kind, MessageKind::Response));
}

#[test]
fn test_message_classifier_notification() {
    let json_str = r#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}"#;
    let kind = MessageClassifier::classify(json_str).unwrap();
    assert!(matches!(kind, MessageKind::Notification));
}

#[test]
fn test_message_classifier_is_mcp_method() {
    assert!(MessageClassifier::is_mcp_method("initialize"));
    assert!(MessageClassifier::is_mcp_method("tools/list"));
    assert!(MessageClassifier::is_mcp_method("tools/call"));
    assert!(MessageClassifier::is_mcp_method("resources/list"));
    assert!(MessageClassifier::is_mcp_method("prompts/list"));
    assert!(MessageClassifier::is_mcp_method("ping"));

    assert!(!MessageClassifier::is_mcp_method("unknown/method"));
    assert!(!MessageClassifier::is_mcp_method(""));
}

// ============================================================================
// RequestRouter Tests (Async)
// ============================================================================

#[tokio::test]
async fn test_router_register_and_dispatch() {
    let router = RequestRouter::new();
    let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // Register handler for tools/list
    {
        let counter_clone = counter.clone();
        router
            .register_handler("tools/list", move |_request| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(json!({"tools": []}))
                }
            })
            .await;
    }

    // Dispatch a request
    let mut request = JsonRpcRequest::new(RequestId::Number(1), "tools/list");
    request.set_params(json!({}));
    let result = router.dispatch(request).await.unwrap();

    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(result.is_object());
}

#[tokio::test]
async fn test_router_unknown_method() {
    let router = RequestRouter::new();

    let mut request = JsonRpcRequest::new(RequestId::Number(1), "unknown/method");
    request.set_params(json!({}));
    let result = router.dispatch(request).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_router_notification_handlers() {
    let router = RequestRouter::new();
    let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // Register notification handler
    {
        let counter_clone = counter.clone();
        router
            .register_notification_handler("notifications/initialized", move |_params| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            })
            .await;
    }

    // Dispatch notification
    let notification = JsonRpcNotification::new("notifications/initialized");
    router.dispatch_notification(notification).await;

    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_router_stats() {
    let router = RequestRouter::new();

    // Register some handlers
    router
        .register_handler("tools/list", |_| async { Ok(json!({})) })
        .await;
    router
        .register_handler("tools/call", |_| async { Ok(json!({})) })
        .await;
    router
        .register_notification_handler("notifications/initialized", |_| async {})
        .await;

    // Dispatch some requests to update stats
    let mut req1 = JsonRpcRequest::new(RequestId::Number(1), "tools/list");
    req1.set_params(json!({}));
    router.dispatch(req1).await.unwrap();

    let mut req2 = JsonRpcRequest::new(RequestId::Number(2), "tools/call");
    req2.set_params(json!({}));
    router.dispatch(req2).await.unwrap();

    let stats = router.stats().await;
    assert_eq!(stats.requests_handled, 2);
    assert_eq!(stats.requests_by_method.len(), 2);
}

// ============================================================================
// ResponseCorrelator Tests
// ============================================================================

#[test]
fn test_correlator_register_request() {
    let tracker = Arc::new(RequestTracker::new(Duration::from_secs(30)));
    let correlator = ResponseCorrelator::new(tracker);

    let (id, _handle) = correlator.register("initialize");

    // ID should be numeric and positive
    match id {
        RequestId::Number(n) => assert!(n > 0),
        _ => panic!("Expected numeric request ID"),
    }
}

#[test]
fn test_correlator_register_with_timeout() {
    let tracker = Arc::new(RequestTracker::new(Duration::from_secs(30)));
    let correlator = ResponseCorrelator::new(tracker);

    let custom_timeout = Duration::from_secs(60);
    let (_id, handle) = correlator.register_with_timeout("tools/call", custom_timeout);

    assert_eq!(handle.method(), "tools/call");
    // Check that remaining time is close to the timeout (it should be almost full)
    let remaining = handle.remaining();
    assert!(
        remaining > Duration::from_secs(55),
        "Remaining time should be close to timeout"
    );
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_request_response_flow() {
    let parser = ResponseParser::new();
    let tracker = Arc::new(RequestTracker::new(Duration::from_secs(30)));
    let correlator = ResponseCorrelator::new(tracker.clone());

    // 1. Register a request
    let (request_id, handle) = correlator.register("initialize");

    // 2. Build the request
    let params = InitializeParams {
        protocol_version: ltmatrix::mcp::MCP_PROTOCOL_VERSION.to_string(),
        capabilities: ClientCapabilities::default(),
        client_info: ImplementationInfo {
            name: "test-client".to_string(),
            version: "1.0.0".to_string(),
        },
    };

    let builder = RequestBuilder::new(request_id.clone(), "initialize")
        .params(&params)
        .unwrap();

    let request = builder.build();
    assert_eq!(request.method, "initialize");

    // 3. Simulate server response
    let response = JsonRpcResponse::success(
        request_id.clone(),
        json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "serverInfo": {
                "name": "test-server",
                "version": "1.0.0"
            }
        }),
    );

    // 4. Parse response
    let result: InitializeResult = parser.parse_response(response).unwrap();
    assert_eq!(result.server_info.name, "test-server");

    // 5. Verify handle info
    assert_eq!(handle.method(), "initialize");
}

#[tokio::test]
async fn test_router_with_multiple_handlers() {
    let router = RequestRouter::new();

    // Register handlers for different methods
    router
        .register_handler("initialize", |_| async {
            Ok(json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "serverInfo": {"name": "test", "version": "1.0"}
            }))
        })
        .await;

    router
        .register_handler("tools/list", |_| async {
            Ok(json!({
                "tools": [
                    {"name": "tool1", "description": "Test tool", "inputSchema": {}}
                ]
            }))
        })
        .await;

    router
        .register_handler("ping", |_| async { Ok(json!({})) })
        .await;

    // Test each handler
    let mut init_request = JsonRpcRequest::new(RequestId::Number(1), "initialize");
    init_request.set_params(json!({}));
    let init_result = router.dispatch(init_request).await.unwrap();
    assert!(init_result.is_object());

    let mut tools_request = JsonRpcRequest::new(RequestId::Number(2), "tools/list");
    tools_request.set_params(json!({}));
    let tools_result = router.dispatch(tools_request).await.unwrap();
    assert!(tools_result.is_object());

    let mut ping_request = JsonRpcRequest::new(RequestId::Number(3), "ping");
    ping_request.set_params(json!({}));
    let ping_result = router.dispatch(ping_request).await.unwrap();
    assert!(ping_result.is_object());
}

#[test]
fn test_message_classification_and_routing() {
    // Test classification
    let request_json = r#"{"jsonrpc":"2.0","id":1,"method":"test/method","params":{}}"#;
    let kind = MessageClassifier::classify(request_json).unwrap();
    assert!(matches!(kind, MessageKind::Request));

    let response_json = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
    let kind = MessageClassifier::classify(response_json).unwrap();
    assert!(matches!(kind, MessageKind::Response));

    let notification_json = r#"{"jsonrpc":"2.0","method":"test/notification","params":{}}"#;
    let kind = MessageClassifier::classify(notification_json).unwrap();
    assert!(matches!(kind, MessageKind::Notification));
}

// ============================================================================
// Type-safe method parsing tests
// ============================================================================

#[test]
fn test_parse_method_initialize() {
    let parser = ResponseParser::new();

    let response = JsonRpcResponse::success(
        RequestId::Number(1),
        json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "serverInfo": {
                "name": "test-server",
                "version": "1.0.0"
            }
        }),
    );

    let result = parser.parse_method::<Initialize>(response).unwrap();
    assert_eq!(result.protocol_version, "2025-11-25");
    assert_eq!(result.server_info.name, "test-server");
}

#[test]
fn test_parse_method_tools_list() {
    let parser = ResponseParser::new();

    let response = JsonRpcResponse::success(
        RequestId::Number(1),
        json!({
            "tools": [
                {
                    "name": "test_tool",
                    "description": "A test tool",
                    "inputSchema": {"type": "object"}
                }
            ]
        }),
    );

    let result = parser.parse_method::<ToolsList>(response).unwrap();
    assert_eq!(result.tools.len(), 1);
    assert_eq!(result.tools[0].name, "test_tool");
}

#[test]
fn test_parse_method_ping() {
    let parser = ResponseParser::new();

    let response = JsonRpcResponse::success(RequestId::Number(1), json!({}));

    let result = parser.parse_method::<Ping>(response).unwrap();
    // PingResult is empty, just verify it parses
    let _ = result;
}

// ============================================================================
// Additional ResponseParser Tests
// ============================================================================

#[test]
fn test_response_parser_parse_value() {
    let parser = ResponseParser::new();

    let value = json!({"name": "test-client", "version": "2.0.0"});
    let info: ImplementationInfo = parser.parse_value(value).unwrap();
    assert_eq!(info.name, "test-client");
    assert_eq!(info.version, "2.0.0");
}

#[test]
fn test_response_parser_parse_malformed_json() {
    let parser = ResponseParser::new();

    let result = parser.parse::<InitializeResult>("not valid json");
    assert!(result.is_err());
}

#[test]
fn test_response_parser_missing_result_field() {
    let parser = ResponseParser::new();

    // Create a response without a result field (this shouldn't normally happen but let's test)
    let response = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        result: None,
        error: None,
    };

    let result = parser.parse_response::<InitializeResult>(response);
    assert!(result.is_err());
}

#[test]
fn test_response_parser_try_parse_success() {
    let parser = ResponseParser::new();

    let response = JsonRpcResponse::success(
        RequestId::Number(1),
        json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "serverInfo": {"name": "test", "version": "1.0"}
        }),
    );

    let result: InitializeResult = parser.try_parse(response).unwrap();
    assert_eq!(result.protocol_version, "2025-11-25");
}

#[test]
fn test_response_parser_try_parse_error() {
    let parser = ResponseParser::new();

    let error = JsonRpcError::new(
        JsonRpcErrorCode::MethodNotFound,
        "Method not found".to_string(),
    );
    let response = JsonRpcResponse::error(RequestId::Number(1), error);

    let result = parser.try_parse::<InitializeResult>(response);
    assert!(result.is_err());
    let (_, err) = result.unwrap_err();
    // Verify we got an error
    let _ = err;
}

// ============================================================================
// TypedResponse Additional Tests
// ============================================================================

#[test]
fn test_typed_response_with_response_time() {
    let raw_response = JsonRpcResponse::success(RequestId::Number(1), json!({}));

    let typed = TypedResponse::new(raw_response, 42i32, "test".to_string()).with_response_time(150);

    assert_eq!(typed.response_time_ms, Some(150));
}

#[test]
fn test_typed_response_is_success() {
    let success_response = JsonRpcResponse::success(RequestId::Number(1), json!({}));
    let typed_success = TypedResponse::new(success_response, 42i32, "test".to_string());
    assert!(typed_success.is_success());
}

#[test]
fn test_typed_response_result_accessors() {
    let raw_response = JsonRpcResponse::success(RequestId::Number(1), json!({}));
    let typed = TypedResponse::new(
        raw_response,
        "test_result".to_string(),
        "method".to_string(),
    );

    // Test reference accessor
    assert_eq!(typed.result(), "test_result");

    // Test into_result
    let result = typed.into_result();
    assert_eq!(result, "test_result");
}

#[test]
fn test_typed_response_clone() {
    let raw_response = JsonRpcResponse::success(RequestId::Number(1), json!({}));
    let typed = TypedResponse::new(raw_response, "result".to_string(), "method".to_string())
        .with_response_time(100);

    let cloned = typed.clone();
    assert_eq!(cloned.result, "result");
    assert_eq!(cloned.method, "method");
    assert_eq!(cloned.response_time_ms, Some(100));
}

// ============================================================================
// RequestRouter Additional Tests
// ============================================================================

#[tokio::test]
async fn test_router_has_handler() {
    let router = RequestRouter::new();

    assert!(!router.has_handler("initialize").await);

    router
        .register_handler("initialize", |_| async { Ok(json!({})) })
        .await;

    assert!(router.has_handler("initialize").await);
}

#[tokio::test]
async fn test_router_registered_methods() {
    let router = RequestRouter::new();

    router
        .register_handler("initialize", |_| async { Ok(json!({})) })
        .await;
    router
        .register_handler("tools/list", |_| async { Ok(json!({})) })
        .await;
    router
        .register_handler("ping", |_| async { Ok(json!({})) })
        .await;

    let methods = router.registered_methods().await;
    assert_eq!(methods.len(), 3);
    assert!(methods.contains(&"initialize".to_string()));
    assert!(methods.contains(&"tools/list".to_string()));
    assert!(methods.contains(&"ping".to_string()));
}

#[tokio::test]
async fn test_router_handler_overwrite() {
    let router = RequestRouter::new();

    // Register first handler
    router
        .register_handler("test", |_| async { Ok(json!({"version": 1})) })
        .await;

    // Overwrite with second handler
    router
        .register_handler("test", |_| async { Ok(json!({"version": 2})) })
        .await;

    let mut request = JsonRpcRequest::new(RequestId::Number(1), "test");
    request.set_params(json!({}));
    let result = router.dispatch(request).await.unwrap();

    // Should use the second handler
    assert_eq!(result["version"], 2);
}

#[tokio::test]
async fn test_router_error_stats() {
    let router = RequestRouter::new();

    router
        .register_handler("failing", |_| async {
            Err(McpError::protocol(
                McpErrorCode::InternalError,
                "Test error",
            ))
        })
        .await;

    let mut request = JsonRpcRequest::new(RequestId::Number(1), "failing");
    request.set_params(json!({}));
    let _ = router.dispatch(request).await;

    let stats = router.stats().await;
    assert_eq!(stats.errors, 1);
}

#[tokio::test]
async fn test_router_parse_response_typed() {
    let router = RequestRouter::new();

    let response = JsonRpcResponse::success(
        RequestId::Number(1),
        json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "serverInfo": {"name": "test", "version": "1.0"}
        }),
    );

    let typed = router.parse_response::<Initialize>(response).unwrap();
    assert_eq!(typed.method, "initialize");
    assert_eq!(typed.result.protocol_version, "2025-11-25");
}

#[tokio::test]
async fn test_router_parse_raw() {
    let router = RequestRouter::new();

    let response = JsonRpcResponse::success(
        RequestId::Number(1),
        json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "serverInfo": {"name": "test", "version": "1.0"}
        }),
    );

    let typed = router
        .parse_raw::<InitializeResult>(response, "initialize")
        .unwrap();
    assert_eq!(typed.method, "initialize");
    assert_eq!(typed.result.protocol_version, "2025-11-25");
}

#[tokio::test]
async fn test_router_dispatch_unknown_notification() {
    let router = RequestRouter::new();

    // Dispatch notification without handler - should not panic
    let notification = JsonRpcNotification::new("unknown/notification");
    router.dispatch_notification(notification).await;

    let stats = router.stats().await;
    assert_eq!(stats.notifications_handled, 1);
}

// ============================================================================
// RequestBuilder Additional Tests
// ============================================================================

#[test]
fn test_request_builder_params_raw() {
    let request = RequestBuilder::new(RequestId::Number(1), "tools/call")
        .params_raw(json!({"tool": "test"}))
        .build();

    assert_eq!(request.method, "tools/call");
    assert_eq!(request.params, Some(json!({"tool": "test"})));
}

#[test]
fn test_request_builder_string_id() {
    let request = RequestBuilder::new(RequestId::String("custom-id".to_string()), "ping").build();

    assert_eq!(request.id, RequestId::String("custom-id".to_string()));
}

// ============================================================================
// MessageClassifier Additional Tests
// ============================================================================

#[test]
fn test_message_classifier_parse_request() {
    let json_str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let message = MessageClassifier::parse(json_str).unwrap();

    match message {
        ltmatrix::mcp::JsonRpcMessage::Request(req) => {
            assert_eq!(req.method, "initialize");
        }
        _ => panic!("Expected request message"),
    }
}

#[test]
fn test_message_classifier_parse_response() {
    let json_str = r#"{"jsonrpc":"2.0","id":1,"result":{"status":"ok"}}"#;
    let message = MessageClassifier::parse(json_str).unwrap();

    match message {
        ltmatrix::mcp::JsonRpcMessage::Response(res) => {
            assert!(res.result.is_some());
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_message_classifier_parse_notification() {
    let json_str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let message = MessageClassifier::parse(json_str).unwrap();

    match message {
        ltmatrix::mcp::JsonRpcMessage::Notification(notif) => {
            assert_eq!(notif.method, "notifications/initialized");
        }
        _ => panic!("Expected notification message"),
    }
}

#[test]
fn test_message_classifier_get_method_kind() {
    assert_eq!(
        MessageClassifier::get_method_kind("initialize"),
        Some(ltmatrix::mcp::McpMethodKind::Initialize)
    );
    assert_eq!(
        MessageClassifier::get_method_kind("tools/list"),
        Some(ltmatrix::mcp::McpMethodKind::ToolsList)
    );
    assert_eq!(
        MessageClassifier::get_method_kind("ping"),
        Some(ltmatrix::mcp::McpMethodKind::Ping)
    );
    assert_eq!(MessageClassifier::get_method_kind("unknown"), None);
}

#[test]
fn test_message_classifier_error_response() {
    // Response with error field should be classified as Response
    let json_str =
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
    let kind = MessageClassifier::classify(json_str).unwrap();
    assert!(matches!(kind, MessageKind::Response));
}

// ============================================================================
// ResponseCorrelator Additional Tests
// ============================================================================

// Note: test_correlator_correlate_async is tested in the correlation module tests
// since the underlying correlate method uses blocking operations that conflict
// with the async runtime. The ResponseCorrelator wrapper is tested through
// other integration tests.

#[test]
fn test_correlator_accessors() {
    let tracker = Arc::new(RequestTracker::new(Duration::from_secs(30)));
    let correlator = ResponseCorrelator::new(tracker);

    // Verify tracker accessor
    let _ = correlator.tracker();

    // Verify parser accessor
    let _ = correlator.parser();
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_response_parser_empty_tools_list() {
    let parser = ResponseParser::new();

    let response = JsonRpcResponse::success(RequestId::Number(1), json!({"tools": []}));

    let result = parser.parse_method::<ToolsList>(response).unwrap();
    assert!(result.tools.is_empty());
}

#[tokio::test]
async fn test_router_concurrent_dispatch() {
    let router = Arc::new(RequestRouter::new());

    router
        .register_handler("test", |_| async { Ok(json!({})) })
        .await;

    let mut handles = vec![];

    for i in 0..10 {
        let router_clone = Arc::clone(&router);
        let handle = tokio::spawn(async move {
            let mut request = JsonRpcRequest::new(RequestId::Number(i), "test");
            request.set_params(json!({}));
            router_clone.dispatch(request).await
        });
        handles.push(handle);
    }

    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }

    let stats = router.stats().await;
    assert_eq!(stats.requests_handled, 10);
}

#[test]
fn test_message_classifier_invalid_json() {
    let result = MessageClassifier::classify("not valid json");
    assert!(result.is_err());
}

#[test]
fn test_typed_response_with_string_request_id() {
    let response = JsonRpcResponse::success(
        RequestId::String("custom-uuid".to_string()),
        json!({"status": "ok"}),
    );

    let typed = TypedResponse::new(response, "result".to_string(), "test".to_string());
    assert_eq!(
        typed.request_id,
        RequestId::String("custom-uuid".to_string())
    );
}

// ============================================================================
// Router Default Implementation Test
// ============================================================================

#[tokio::test]
async fn test_router_default() {
    let router = RequestRouter::default();
    let methods = router.registered_methods().await;
    assert!(methods.is_empty());
}

// ============================================================================
// Response Parser Default Implementation Test
// ============================================================================

#[test]
fn test_response_parser_default() {
    let parser = ResponseParser::default();
    let value = json!({"name": "test", "version": "1.0"});
    let info: ImplementationInfo = parser.parse_value(value).unwrap();
    assert_eq!(info.name, "test");
}
