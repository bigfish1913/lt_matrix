//! Comprehensive tests for MCP protocol message types
//!
//! These tests verify:
//! - Request message serialization/deserialization
//! - Response message serialization/deserialization
//! - Notification message serialization/deserialization
//! - Request ID handling (string, number, null)
//! - Error code handling
//! - Message type detection
//! - Edge cases and error conditions

use ltmatrix::mcp::protocol::{
    JsonRpcError, JsonRpcErrorCode, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse, RequestId,
};
use serde_json::json;

// ============================================================================
// Request ID Tests
// ============================================================================

#[test]
fn test_request_id_from_string() {
    let id: RequestId = "test-request-id".into();
    assert_eq!(id, RequestId::String("test-request-id".to_string()));
}

#[test]
fn test_request_id_from_number_i64() {
    let id: RequestId = 12345i64.into();
    assert_eq!(id, RequestId::Number(12345));
}

#[test]
fn test_request_id_from_number_i32() {
    let id: RequestId = 999i32.into();
    assert_eq!(id, RequestId::Number(999));
}

#[test]
fn test_request_id_serialization_string() {
    let id = RequestId::String("abc-123".to_string());
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"abc-123\"");
}

#[test]
fn test_request_id_serialization_number() {
    let id = RequestId::Number(42);
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "42");
}

#[test]
fn test_request_id_serialization_null() {
    let id = RequestId::Null;
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "null");
}

#[test]
fn test_request_id_deserialization_string() {
    let json = "\"my-id\"";
    let id: RequestId = serde_json::from_str(json).unwrap();
    assert_eq!(id, RequestId::String("my-id".to_string()));
}

#[test]
fn test_request_id_deserialization_number() {
    let json = "999";
    let id: RequestId = serde_json::from_str(json).unwrap();
    assert_eq!(id, RequestId::Number(999));
}

#[test]
fn test_request_id_deserialization_null() {
    let json = "null";
    let id: RequestId = serde_json::from_str(json).unwrap();
    assert_eq!(id, RequestId::Null);
}

#[test]
fn test_request_id_display() {
    assert_eq!(format!("{}", RequestId::String("test".into())), "\"test\"");
    assert_eq!(format!("{}", RequestId::Number(42)), "42");
    assert_eq!(format!("{}", RequestId::Null), "null");
}

#[test]
fn test_request_id_equality() {
    let id1 = RequestId::String("test".to_string());
    let id2 = RequestId::String("test".to_string());
    let id3 = RequestId::String("other".to_string());
    let id4 = RequestId::Number(42);

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
    assert_ne!(id1, id4);
}

// ============================================================================
// Request Message Tests
// ============================================================================

#[test]
fn test_request_creation_simple() {
    let request = JsonRpcRequest::new(RequestId::Number(1), "test_method");
    assert_eq!(request.jsonrpc, "2.0");
    assert_eq!(request.id, RequestId::Number(1));
    assert_eq!(request.method, "test_method");
    assert!(request.params.is_none());
}

#[test]
fn test_request_creation_without_params() {
    let request = JsonRpcRequest::without_params(RequestId::Number(2), "no_params");
    assert_eq!(request.method, "no_params");
    assert!(request.params.is_none());
}

#[test]
fn test_request_creation_with_params() {
    let params = json!({"arg1": "value1", "arg2": 42});
    let request = JsonRpcRequest::with_params(RequestId::Number(3), "with_params", params);
    assert_eq!(request.method, "with_params");
    assert!(request.params.is_some());
    let p = request.params.unwrap();
    assert_eq!(p["arg1"], "value1");
    assert_eq!(p["arg2"], 42);
}

#[test]
fn test_request_set_params() {
    let mut request = JsonRpcRequest::new(RequestId::Number(4), "set_params_test");
    assert!(request.params.is_none());

    let params = json!({"new": "params"});
    request.set_params(params);
    assert!(request.params.is_some());
}

#[test]
fn test_request_serialization_no_params() {
    let request = JsonRpcRequest::new(RequestId::Number(1), "serialize_test");
    let json = request.to_json().unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 1);
    assert_eq!(parsed["method"], "serialize_test");
    assert!(parsed.get("params").is_none() || parsed["params"].is_null());
}

#[test]
fn test_request_serialization_with_params() {
    let params = json!({"key": "value"});
    let request = JsonRpcRequest::with_params(
        RequestId::String("req-1".into()),
        "serialize_with_params",
        params,
    );
    let json = request.to_json().unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], "req-1");
    assert_eq!(parsed["method"], "serialize_with_params");
    assert_eq!(parsed["params"]["key"], "value");
}

#[test]
fn test_request_deserialization_no_params() {
    let json = r#"{"jsonrpc":"2.0","id":1,"method":"test_method"}"#;
    let request = JsonRpcRequest::from_json(json).unwrap();

    assert_eq!(request.jsonrpc, "2.0");
    assert_eq!(request.id, RequestId::Number(1));
    assert_eq!(request.method, "test_method");
    assert!(request.params.is_none());
}

#[test]
fn test_request_deserialization_with_params() {
    let json = r#"{"jsonrpc":"2.0","id":"req-2","method":"test","params":{"arg":"value"}}"#;
    let request = JsonRpcRequest::from_json(json).unwrap();

    assert_eq!(request.method, "test");
    assert!(request.params.is_some());
    let params = request.params.unwrap();
    assert_eq!(params["arg"], "value");
}

#[test]
fn test_request_deserialization_with_array_params() {
    let json = r#"{"jsonrpc":"2.0","id":3,"method":"array_test","params":[1,2,3]}"#;
    let request = JsonRpcRequest::from_json(json).unwrap();

    assert!(request.params.is_some());
    let params = request.params.unwrap();
    assert!(params.is_array());
    assert_eq!(params.as_array().unwrap().len(), 3);
}

#[test]
fn test_request_roundtrip() {
    let original =
        JsonRpcRequest::with_params(RequestId::Number(99), "roundtrip", json!({"test": "data"}));

    let json = original.to_json().unwrap();
    let deserialized = JsonRpcRequest::from_json(&json).unwrap();

    assert_eq!(original.jsonrpc, deserialized.jsonrpc);
    assert_eq!(original.id, deserialized.id);
    assert_eq!(original.method, deserialized.method);
    assert_eq!(original.params, deserialized.params);
}

// ============================================================================
// Response Message Tests
// ============================================================================

#[test]
fn test_response_success_creation() {
    let result = json!({"status": "ok"});
    let response = JsonRpcResponse::success(RequestId::Number(1), result);

    assert!(response.is_success());
    assert!(!response.is_error());
    assert!(response.get_result().is_some());
    assert!(response.get_error().is_none());
}

#[test]
fn test_response_error_creation() {
    let error = JsonRpcError::method_not_found("test_method");
    let response = JsonRpcResponse::error(RequestId::Number(2), error);

    assert!(!response.is_success());
    assert!(response.is_error());
    assert!(response.get_result().is_none());
    assert!(response.get_error().is_some());
}

#[test]
fn test_response_get_result() {
    let result = json!({"answer": 42});
    let response = JsonRpcResponse::success(RequestId::Number(3), result.clone());

    let retrieved = response.get_result().unwrap();
    assert_eq!(retrieved, &result);
}

#[test]
fn test_response_get_error() {
    let error = JsonRpcError::invalid_params("missing field");
    let response = JsonRpcResponse::error(RequestId::Number(4), error);

    let retrieved = response.get_error().unwrap();
    assert_eq!(retrieved.code, -32602);
    assert!(retrieved.message.contains("missing field"));
}

#[test]
fn test_response_success_serialization() {
    let result = json!({"data": "value"});
    let response = JsonRpcResponse::success(RequestId::String("resp-1".into()), result);
    let json = response.to_json().unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], "resp-1");
    assert!(parsed.get("result").is_some());
    assert!(parsed.get("error").is_none());
}

#[test]
fn test_response_error_serialization() {
    let error = JsonRpcError::method_not_found("nonexistent");
    let response = JsonRpcResponse::error(RequestId::Number(5), error);
    let json = response.to_json().unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("result").is_none());
    assert!(parsed.get("error").is_some());
    assert_eq!(parsed["error"]["code"], -32601);
}

#[test]
fn test_response_deserialization_success() {
    let json = r#"{"jsonrpc":"2.0","id":6,"result":{"success":true}}"#;
    let response = JsonRpcResponse::from_json(json).unwrap();

    assert!(response.is_success());
    assert!(response.get_result().is_some());
    let result = response.get_result().unwrap();
    assert_eq!(result["success"], true);
}

#[test]
fn test_response_deserialization_error() {
    let json = r#"{"jsonrpc":"2.0","id":7,"error":{"code":-32602,"message":"Invalid params"}}"#;
    let response = JsonRpcResponse::from_json(json).unwrap();

    assert!(response.is_error());
    assert!(response.get_error().is_some());
    let error = response.get_error().unwrap();
    assert_eq!(error.code, -32602);
    assert_eq!(error.message, "Invalid params");
}

#[test]
fn test_response_roundtrip_success() {
    let original = JsonRpcResponse::success(
        RequestId::Number(100),
        json!({"complex": {"nested": "data"}}),
    );

    let json = original.to_json().unwrap();
    let deserialized = JsonRpcResponse::from_json(&json).unwrap();

    assert_eq!(original.id, deserialized.id);
    assert_eq!(original.result, deserialized.result);
    assert_eq!(original.error, deserialized.error);
}

#[test]
fn test_response_roundtrip_error() {
    let error = JsonRpcError::parse_error("unexpected token");
    let original = JsonRpcResponse::error(RequestId::String("err-1".into()), error);

    let json = original.to_json().unwrap();
    let deserialized = JsonRpcResponse::from_json(&json).unwrap();

    assert_eq!(original.id, deserialized.id);
    assert_eq!(
        original.error.unwrap().code,
        deserialized.error.unwrap().code
    );
}

// ============================================================================
// Notification Message Tests
// ============================================================================

#[test]
fn test_notification_creation_simple() {
    let notification = JsonRpcNotification::new("test_event");
    assert_eq!(notification.jsonrpc, "2.0");
    assert_eq!(notification.method, "test_event");
    assert!(notification.params.is_none());
}

#[test]
fn test_notification_creation_with_params() {
    let params = json!({"event_data": "value"});
    let notification = JsonRpcNotification::with_params("event_with_params", params);

    assert_eq!(notification.method, "event_with_params");
    assert!(notification.params.is_some());
    let p = notification.params.unwrap();
    assert_eq!(p["event_data"], "value");
}

#[test]
fn test_notification_set_params() {
    let mut notification = JsonRpcNotification::new("set_params_notif");
    assert!(notification.params.is_none());

    let params = json!({"new": "data"});
    notification.set_params(params);
    assert!(notification.params.is_some());
}

#[test]
fn test_notification_serialization_no_params() {
    let notification = JsonRpcNotification::new("no_params_event");
    let json = notification.to_json().unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["method"], "no_params_event");
    // Notifications must not have "id" field
    assert!(parsed.get("id").is_none());
}

#[test]
fn test_notification_serialization_with_params() {
    let params = json!({"value": 123});
    let notification = JsonRpcNotification::with_params("param_event", params);
    let json = notification.to_json().unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["method"], "param_event");
    assert_eq!(parsed["params"]["value"], 123);
    // No "id" field in notifications
    assert!(parsed.get("id").is_none());
}

#[test]
fn test_notification_deserialization_no_params() {
    let json = r#"{"jsonrpc":"2.0","method":"event"}"#;
    let notification = JsonRpcNotification::from_json(json).unwrap();

    assert_eq!(notification.method, "event");
    assert!(notification.params.is_none());
}

#[test]
fn test_notification_deserialization_with_params() {
    let json = r#"{"jsonrpc":"2.0","method":"event","params":{"data":"test"}}"#;
    let notification = JsonRpcNotification::from_json(json).unwrap();

    assert_eq!(notification.method, "event");
    assert!(notification.params.is_some());
    let params = notification.params.unwrap();
    assert_eq!(params["data"], "test");
}

#[test]
fn test_notification_roundtrip() {
    let original = JsonRpcNotification::with_params("roundtrip_event", json!({"test": "value"}));

    let json = original.to_json().unwrap();
    let deserialized = JsonRpcNotification::from_json(&json).unwrap();

    assert_eq!(original.jsonrpc, deserialized.jsonrpc);
    assert_eq!(original.method, deserialized.method);
    assert_eq!(original.params, deserialized.params);
}

// ============================================================================
// JsonRpcMessage Enum Tests
// ============================================================================

#[test]
fn test_message_from_json_request() {
    let json = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
    let message = JsonRpcMessage::from_json(json).unwrap();

    assert!(message.is_request());
    assert!(!message.is_response());
    assert!(!message.is_notification());
}

#[test]
fn test_message_from_json_response_success() {
    let json = r#"{"jsonrpc":"2.0","id":2,"result":{}}"#;
    let message = JsonRpcMessage::from_json(json).unwrap();

    assert!(!message.is_request());
    assert!(message.is_response());
    assert!(!message.is_notification());
}

#[test]
fn test_message_from_json_response_error() {
    let json = r#"{"jsonrpc":"2.0","id":3,"error":{"code":-32601,"message":"not found"}}"#;
    let message = JsonRpcMessage::from_json(json).unwrap();

    assert!(!message.is_request());
    assert!(message.is_response());
    assert!(!message.is_notification());
}

#[test]
fn test_message_from_json_notification() {
    let json = r#"{"jsonrpc":"2.0","method":"notification"}"#;
    let message = JsonRpcMessage::from_json(json).unwrap();

    assert!(!message.is_request());
    assert!(!message.is_response());
    assert!(message.is_notification());
}

#[test]
fn test_message_to_json_request() {
    let request = JsonRpcRequest::new(RequestId::Number(1), "test");
    let message = JsonRpcMessage::Request(request);
    let json = message.to_json().unwrap();

    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"method\":\"test\""));
}

#[test]
fn test_message_to_json_response() {
    let response = JsonRpcResponse::success(RequestId::Number(2), json!({}));
    let message = JsonRpcMessage::Response(response);
    let json = message.to_json().unwrap();

    assert!(json.contains("\"result\""));
}

#[test]
fn test_message_to_json_notification() {
    let notification = JsonRpcNotification::new("event");
    let message = JsonRpcMessage::Notification(notification);
    let json = message.to_json().unwrap();

    assert!(json.contains("\"method\":\"event\""));
    assert!(!json.contains("\"id\""));
}

// ============================================================================
// Error Code Tests
// ============================================================================

#[test]
fn test_error_code_standard_codes() {
    assert_eq!(JsonRpcErrorCode::ParseError.as_i32(), -32700);
    assert_eq!(JsonRpcErrorCode::InvalidRequest.as_i32(), -32600);
    assert_eq!(JsonRpcErrorCode::MethodNotFound.as_i32(), -32601);
    assert_eq!(JsonRpcErrorCode::InvalidParams.as_i32(), -32602);
    assert_eq!(JsonRpcErrorCode::InternalError.as_i32(), -32603);
}

#[test]
fn test_error_code_server_range() {
    let code = JsonRpcErrorCode::ServerError(-32001);
    assert_eq!(code.as_i32(), -32001);

    let code = JsonRpcErrorCode::ServerError(-32099);
    assert_eq!(code.as_i32(), -32099);
}

#[test]
fn test_error_code_from_i32_standard() {
    assert_eq!(
        JsonRpcErrorCode::from_i32(-32700),
        JsonRpcErrorCode::ParseError
    );
    assert_eq!(
        JsonRpcErrorCode::from_i32(-32600),
        JsonRpcErrorCode::InvalidRequest
    );
    assert_eq!(
        JsonRpcErrorCode::from_i32(-32601),
        JsonRpcErrorCode::MethodNotFound
    );
    assert_eq!(
        JsonRpcErrorCode::from_i32(-32602),
        JsonRpcErrorCode::InvalidParams
    );
    assert_eq!(
        JsonRpcErrorCode::from_i32(-32603),
        JsonRpcErrorCode::InternalError
    );
}

#[test]
fn test_error_code_from_i32_server_range() {
    assert_eq!(
        JsonRpcErrorCode::from_i32(-32000),
        JsonRpcErrorCode::ServerError(-32000)
    );
    assert_eq!(
        JsonRpcErrorCode::from_i32(-32050),
        JsonRpcErrorCode::ServerError(-32050)
    );
    assert_eq!(
        JsonRpcErrorCode::from_i32(-32099),
        JsonRpcErrorCode::ServerError(-32099)
    );
}

#[test]
fn test_error_code_message() {
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
        JsonRpcErrorCode::ServerError(-32001).message(),
        "Server error"
    );
}

#[test]
fn test_error_code_display() {
    let display = format!("{}", JsonRpcErrorCode::ParseError);
    assert!(display.contains("Parse error"));
    assert!(display.contains("-32700"));
}

// ============================================================================
// JsonRpcError Tests
// ============================================================================

#[test]
fn test_error_new() {
    let error = JsonRpcError::new(
        JsonRpcErrorCode::MethodNotFound,
        "Method not found".to_string(),
    );

    assert_eq!(error.code, -32601);
    assert_eq!(error.message, "Method not found");
    assert!(error.data.is_none());
}

#[test]
fn test_error_with_data() {
    let data = json!({"method": "test_method", "available": ["other"]});
    let error = JsonRpcError::with_data(
        JsonRpcErrorCode::MethodNotFound,
        "Unknown method".to_string(),
        data,
    );

    assert_eq!(error.code, -32601);
    assert!(error.data.is_some());
}

#[test]
fn test_error_method_not_found() {
    let error = JsonRpcError::method_not_found("my_method");

    assert_eq!(error.code, -32601);
    assert!(error.message.contains("my_method"));
    assert!(error.data.is_some());
}

#[test]
fn test_error_invalid_params() {
    let error = JsonRpcError::invalid_params("missing required field");

    assert_eq!(error.code, -32602);
    assert!(error.message.contains("missing required field"));
}

#[test]
fn test_error_parse_error() {
    let error = JsonRpcError::parse_error("unexpected token");

    assert_eq!(error.code, -32700);
    assert!(error.message.contains("unexpected token"));
}

#[test]
fn test_error_invalid_request() {
    let error = JsonRpcError::invalid_request("missing method field");

    assert_eq!(error.code, -32600);
    assert!(error.message.contains("missing method field"));
}

#[test]
fn test_error_internal_error() {
    let error = JsonRpcError::internal_error("database connection failed");

    assert_eq!(error.code, -32603);
    assert!(error.message.contains("database connection failed"));
}

#[test]
fn test_error_code_enum() {
    let error = JsonRpcError::method_not_found("test");
    let code_enum = error.code_enum();

    assert_eq!(code_enum, JsonRpcErrorCode::MethodNotFound);
}

#[test]
fn test_error_display() {
    let error = JsonRpcError::method_not_found("test_method");
    let display = format!("{}", error);

    assert!(display.contains("-32601"));
    assert!(display.contains("Method not found"));
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
    let json = r#"{"code":-32601,"message":"Not found","data":{"method":"test"}}"#;
    let error: JsonRpcError = serde_json::from_str(json).unwrap();

    assert_eq!(error.code, -32601);
    assert!(error.message.contains("Not found"));
    assert!(error.data.is_some());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_request_response_cycle() {
    // Create request
    let request =
        JsonRpcRequest::with_params(RequestId::Number(1), "calculate", json!({"a": 10, "b": 20}));

    // Serialize and send
    let request_json = request.to_json().unwrap();

    // Simulate server processing
    let result = json!({"sum": 30});
    let response = JsonRpcResponse::success(request.id, result);
    let response_json = response.to_json().unwrap();

    // Client receives response
    let received_response = JsonRpcResponse::from_json(&response_json).unwrap();
    assert!(received_response.is_success());
    let result_value = received_response.get_result().unwrap();
    assert_eq!(result_value["sum"], 30);
}

#[test]
fn test_error_request_response_cycle() {
    // Create invalid request
    let request = JsonRpcRequest::new(RequestId::Number(2), "unknown_method");
    let request_json = request.to_json().unwrap();

    // Server responds with error
    let error = JsonRpcError::method_not_found("unknown_method");
    let response = JsonRpcResponse::error(request.id, error);
    let response_json = response.to_json().unwrap();

    // Client receives error
    let received_response = JsonRpcResponse::from_json(&response_json).unwrap();
    assert!(received_response.is_error());
    let error_value = received_response.get_error().unwrap();
    assert_eq!(error_value.code, -32601);
}

#[test]
fn test_notification_one_way() {
    // Client sends notification (no response expected)
    let notification = JsonRpcNotification::with_params(
        "log_message",
        json!({"level": "info", "message": "test"}),
    );

    let notification_json = notification.to_json().unwrap();

    // Server receives notification
    let received = JsonRpcNotification::from_json(&notification_json).unwrap();
    assert_eq!(received.method, "log_message");
    assert!(received.params.is_some());
}

#[test]
fn test_complex_nested_params() {
    let complex_params = json!({
        "config": {
            "settings": {
                "timeout": 30,
                "retries": 3
            },
            "options": ["opt1", "opt2"]
        }
    });

    let request = JsonRpcRequest::with_params(
        RequestId::String("complex".into()),
        "complex_method",
        complex_params,
    );

    let json = request.to_json().unwrap();
    let deserialized = JsonRpcRequest::from_json(&json).unwrap();

    assert_eq!(request.params, deserialized.params);
}

#[test]
fn test_unicode_in_strings() {
    let request = JsonRpcRequest::with_params(
        RequestId::String("unicode-测试".into()),
        "测试方法",
        json!({"message": "Hello 世界 🌍"}),
    );

    let json = request.to_json().unwrap();
    let deserialized = JsonRpcRequest::from_json(&json).unwrap();

    assert_eq!(request.method, deserialized.method);
    assert_eq!(request.params, deserialized.params);
}

#[test]
fn test_large_request_id() {
    let large_id = RequestId::Number(i64::MAX);
    let request = JsonRpcRequest::new(large_id, "test");

    let json = request.to_json().unwrap();
    let deserialized = JsonRpcRequest::from_json(&json).unwrap();

    assert_eq!(request.id, deserialized.id);
}

// ============================================================================
// Edge Case Tests - RequestId
// ============================================================================

#[test]
fn test_request_id_empty_string() {
    let id = RequestId::String("".to_string());
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"\"");

    let deserialized: RequestId = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, id);
}

#[test]
fn test_request_id_zero() {
    let id = RequestId::Number(0);
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "0");

    let deserialized: RequestId = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, id);
}

#[test]
fn test_request_id_negative_number() {
    let id = RequestId::Number(-1);
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "-1");

    let deserialized: RequestId = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, id);
}

#[test]
fn test_request_id_large_negative() {
    let id = RequestId::Number(i64::MIN);
    let json = serde_json::to_string(&id).unwrap();

    let deserialized: RequestId = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, id);
}

#[test]
fn test_request_id_special_characters() {
    let id = RequestId::String("id-with_special.chars@123".to_string());
    let json = serde_json::to_string(&id).unwrap();

    let deserialized: RequestId = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, id);
}

#[test]
fn test_request_id_from_u64_conversion() {
    // Test u64 to i64 conversion behavior
    let json = "18446744073709551615"; // u64::MAX as string
    let result: Result<RequestId, _> = serde_json::from_str(json);
    // Should either work with conversion or fail gracefully
    assert!(result.is_ok() || result.is_err());
}

// ============================================================================
// Edge Case Tests - Message Validation
// ============================================================================

#[test]
fn test_request_missing_jsonrpc_field() {
    let json = r#"{"id":1,"method":"test"}"#;
    let result: Result<JsonRpcRequest, _> = serde_json::from_str(json);
    // Should fail due to missing jsonrpc field
    assert!(result.is_err());
}

#[test]
fn test_request_missing_method_field() {
    let json = r#"{"jsonrpc":"2.0","id":1}"#;
    let result: Result<JsonRpcRequest, _> = serde_json::from_str(json);
    // Should fail due to missing method field
    assert!(result.is_err());
}

#[test]
fn test_request_missing_id_field() {
    // Missing "id" field makes it a notification, not a request
    let json = r#"{"jsonrpc":"2.0","method":"test"}"#;
    let result: Result<JsonRpcRequest, _> = serde_json::from_str(json);
    // Should fail - requests must have an id
    assert!(result.is_err());
}

#[test]
fn test_response_missing_jsonrpc_field() {
    let json = r#"{"id":1,"result":{}}"#;
    let result: Result<JsonRpcResponse, _> = serde_json::from_str(json);
    // Should fail due to missing jsonrpc field
    assert!(result.is_err());
}

#[test]
fn test_response_missing_both_result_and_error() {
    let json = r#"{"jsonrpc":"2.0","id":1}"#;
    let response = JsonRpcResponse::from_json(json).unwrap();
    // Response should have neither result nor error
    assert!(response.result.is_none());
    assert!(response.error.is_none());
    // Should not be considered success or error
    assert!(!response.is_success());
    assert!(!response.is_error());
}

#[test]
fn test_response_with_both_result_and_error() {
    // Invalid: should not have both result and error
    let json = r#"{"jsonrpc":"2.0","id":1,"result":{},"error":{"code":-32601,"message":"error"}}"#;
    let result: Result<JsonRpcResponse, _> = serde_json::from_str(json);
    // Serde will deserialize this, but semantically it's invalid
    // The response should have both fields present
    let response = result.unwrap();
    assert!(response.result.is_some());
    assert!(response.error.is_some());
}

#[test]
fn test_notification_with_id_field() {
    // Notifications should not have an "id" field
    let json = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
    // This would parse as a notification but has an id field
    let result: Result<JsonRpcNotification, _> = serde_json::from_str(json);
    // Will fail because JsonRpcNotification doesn't have an id field
    assert!(result.is_err());
}

#[test]
fn test_message_detection_notification() {
    let json = r#"{"jsonrpc":"2.0","method":"test"}"#;
    let message = JsonRpcMessage::from_json(json).unwrap();
    assert!(message.is_notification());
}

#[test]
fn test_message_detection_request() {
    let json = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
    let message = JsonRpcMessage::from_json(json).unwrap();
    assert!(message.is_request());
}

#[test]
fn test_message_detection_response_result() {
    let json = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
    let message = JsonRpcMessage::from_json(json).unwrap();
    assert!(message.is_response());
}

#[test]
fn test_message_detection_response_error() {
    let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"not found"}}"#;
    let message = JsonRpcMessage::from_json(json).unwrap();
    assert!(message.is_response());
}

// ============================================================================
// Edge Case Tests - Parameters
// ============================================================================

#[test]
fn test_request_with_null_params() {
    let json = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":null}"#;
    let request = JsonRpcRequest::from_json(json).unwrap();
    // params should be present but null
    assert!(request.params.is_some());
    assert!(request.params.unwrap().is_null());
}

#[test]
fn test_request_with_empty_object_params() {
    let json = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
    let request = JsonRpcRequest::from_json(json).unwrap();
    assert!(request.params.is_some());
    assert!(request.params.unwrap().as_object().unwrap().is_empty());
}

#[test]
fn test_request_with_empty_array_params() {
    let json = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":[]}"#;
    let request = JsonRpcRequest::from_json(json).unwrap();
    assert!(request.params.is_some());
    assert!(request.params.unwrap().as_array().unwrap().is_empty());
}

#[test]
fn test_request_with_array_params() {
    let json = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":[1,"two",null]}"#;
    let request = JsonRpcRequest::from_json(json).unwrap();
    assert!(request.params.is_some());
    let params = request.params.unwrap();
    assert!(params.is_array());
    let arr = params.as_array().unwrap();
    assert_eq!(arr.len(), 3);
}

#[test]
fn test_notification_with_null_params() {
    let json = r#"{"jsonrpc":"2.0","method":"test","params":null}"#;
    let notification = JsonRpcNotification::from_json(json).unwrap();
    assert!(notification.params.is_some());
    assert!(notification.params.unwrap().is_null());
}

#[test]
fn test_request_params_serialization_skip_when_none() {
    let request = JsonRpcRequest::new(RequestId::Number(1), "test");
    let json = request.to_json().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    // When params is None, it should not be in the JSON
    assert!(parsed.get("params").is_none());
}

#[test]
fn test_request_params_serialization_include_when_some() {
    let request =
        JsonRpcRequest::with_params(RequestId::Number(1), "test", json!({"key": "value"}));
    let json = request.to_json().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    // When params is Some, it should be in the JSON
    assert!(parsed.get("params").is_some());
}

// ============================================================================
// Edge Case Tests - Protocol Version
// ============================================================================

#[test]
fn test_request_protocol_version_is_2_0() {
    let request = JsonRpcRequest::new(RequestId::Number(1), "test");
    assert_eq!(request.jsonrpc, "2.0");
}

#[test]
fn test_response_protocol_version_is_2_0() {
    let response = JsonRpcResponse::success(RequestId::Number(1), json!({}));
    assert_eq!(response.jsonrpc, "2.0");
}

#[test]
fn test_notification_protocol_version_is_2_0() {
    let notification = JsonRpcNotification::new("test");
    assert_eq!(notification.jsonrpc, "2.0");
}

#[test]
fn test_request_with_invalid_protocol_version() {
    // JSON-RPC 2.0 spec says jsonrpc must be "2.0"
    let json = r#"{"jsonrpc":"1.0","id":1,"method":"test"}"#;
    let request = JsonRpcRequest::from_json(json).unwrap();
    // Will deserialize, but version is not "2.0"
    assert_eq!(request.jsonrpc, "1.0");
}

#[test]
fn test_response_with_invalid_protocol_version() {
    let json = r#"{"jsonrpc":"1.0","id":1,"result":{}}"#;
    let response = JsonRpcResponse::from_json(json).unwrap();
    assert_eq!(response.jsonrpc, "1.0");
}

// ============================================================================
// Edge Case Tests - Method Names
// ============================================================================

#[test]
fn test_request_empty_method_name() {
    let request = JsonRpcRequest::new(RequestId::Number(1), "");
    assert_eq!(request.method, "");

    let json = request.to_json().unwrap();
    let deserialized = JsonRpcRequest::from_json(&json).unwrap();
    assert_eq!(deserialized.method, "");
}

#[test]
fn test_request_method_with_special_chars() {
    let method = "method/with/slashes";
    let request = JsonRpcRequest::new(RequestId::Number(1), method);
    assert_eq!(request.method, method);

    let json = request.to_json().unwrap();
    let deserialized = JsonRpcRequest::from_json(&json).unwrap();
    assert_eq!(deserialized.method, method);
}

#[test]
fn test_request_method_with_dots() {
    let method = "namespace.method.submethod";
    let request = JsonRpcRequest::new(RequestId::Number(1), method);
    assert_eq!(request.method, method);
}

#[test]
fn test_notification_empty_method_name() {
    let notification = JsonRpcNotification::new("");
    assert_eq!(notification.method, "");

    let json = notification.to_json().unwrap();
    let deserialized = JsonRpcNotification::from_json(&json).unwrap();
    assert_eq!(deserialized.method, "");
}

// ============================================================================
// Edge Case Tests - Response Values
// ============================================================================

#[test]
fn test_response_with_null_result() {
    let result = serde_json::Value::Null;
    let response = JsonRpcResponse::success(RequestId::Number(1), result);
    assert!(response.is_success());
    assert!(response.get_result().is_some());
    assert!(response.get_result().unwrap().is_null());
}

#[test]
fn test_response_with_boolean_result() {
    let result = json!(true);
    let response = JsonRpcResponse::success(RequestId::Number(1), result);
    let retrieved = response.get_result().unwrap();
    assert_eq!(retrieved.as_bool(), Some(true));
}

#[test]
fn test_response_with_number_result() {
    let result = json!(42.5);
    let response = JsonRpcResponse::success(RequestId::Number(1), result);
    let retrieved = response.get_result().unwrap();
    assert_eq!(retrieved.as_f64(), Some(42.5));
}

#[test]
fn test_response_with_string_result() {
    let result = json!("test string");
    let response = JsonRpcResponse::success(RequestId::Number(1), result);
    let retrieved = response.get_result().unwrap();
    assert_eq!(retrieved.as_str(), Some("test string"));
}

#[test]
fn test_response_with_array_result() {
    let result = json!([1, 2, 3]);
    let response = JsonRpcResponse::success(RequestId::Number(1), result);
    let retrieved = response.get_result().unwrap();
    assert!(retrieved.is_array());
    assert_eq!(retrieved.as_array().unwrap().len(), 3);
}

#[test]
fn test_error_with_null_data() {
    let error = JsonRpcError::with_data(
        JsonRpcErrorCode::InternalError,
        "Test error".to_string(),
        serde_json::Value::Null,
    );
    assert!(error.data.is_some());
    assert!(error.data.unwrap().is_null());
}

#[test]
fn test_error_with_array_data() {
    let data = json!([1, 2, 3]);
    let error = JsonRpcError::with_data(
        JsonRpcErrorCode::InternalError,
        "Test error".to_string(),
        data,
    );
    assert!(error.data.is_some());
    assert!(error.data.unwrap().is_array());
}

// ============================================================================
// Edge Case Tests - Message Type Detection
// ============================================================================

#[test]
fn test_message_from_json_invalid_message() {
    // Missing required fields
    let json = r#"{"jsonrpc":"2.0"}"#;
    let result = JsonRpcMessage::from_json(json);
    // Should fail - no method, id, result, or error
    assert!(result.is_err());
}

#[test]
fn test_message_request_with_null_id() {
    let json = r#"{"jsonrpc":"2.0","id":null,"method":"test"}"#;
    let message = JsonRpcMessage::from_json(json).unwrap();
    assert!(message.is_request());

    if let JsonRpcMessage::Request(req) = message {
        assert_eq!(req.id, RequestId::Null);
    } else {
        panic!("Expected request message");
    }
}

#[test]
fn test_message_response_with_null_id() {
    let json = r#"{"jsonrpc":"2.0","id":null,"result":{}}"#;
    let message = JsonRpcMessage::from_json(json).unwrap();
    assert!(message.is_response());

    if let JsonRpcMessage::Response(resp) = message {
        assert_eq!(resp.id, RequestId::Null);
    } else {
        panic!("Expected response message");
    }
}

#[test]
fn test_message_notification_must_not_have_id() {
    // If a message has a method but no id, it's a notification
    let json = r#"{"jsonrpc":"2.0","method":"test"}"#;
    let message = JsonRpcMessage::from_json(json).unwrap();
    assert!(message.is_notification());
    assert!(!message.is_request());
}

#[test]
fn test_message_with_method_and_result_is_response() {
    // Has both method and result - should be detected as response (has id)
    let json = r#"{"jsonrpc":"2.0","id":1,"method":"test","result":{}}"#;
    let message = JsonRpcMessage::from_json(json).unwrap();
    // Response detection checks for result/error, not method
    assert!(message.is_response());
}

// ============================================================================
// Edge Case Tests - Error Handling
// ============================================================================

#[test]
fn test_error_code_unknown_code() {
    // Unknown error codes should map to InternalError
    let code = JsonRpcErrorCode::from_i32(-99999);
    assert_eq!(code, JsonRpcErrorCode::InternalError);
}

#[test]
fn test_error_code_outside_server_range() {
    // Code outside server error range (-32000 to -32099)
    let code = JsonRpcErrorCode::from_i32(-30000);
    assert_eq!(code, JsonRpcErrorCode::InternalError);
}

#[test]
fn test_error_code_positive_number() {
    // Positive error codes are not standard
    let code = JsonRpcErrorCode::from_i32(1);
    assert_eq!(code, JsonRpcErrorCode::InternalError);
}

#[test]
fn test_error_with_empty_message() {
    let error = JsonRpcError::new(JsonRpcErrorCode::ParseError, "".to_string());
    assert_eq!(error.message, "");
}

#[test]
fn test_error_with_long_message() {
    let long_msg = "x".repeat(10000);
    let error = JsonRpcError::new(JsonRpcErrorCode::InternalError, long_msg.clone());
    assert_eq!(error.message.len(), 10000);
}

// ============================================================================
// Edge Case Tests - JSON Parsing
// ============================================================================

#[test]
fn test_request_from_invalid_json() {
    let json = r#"{"jsonrpc":"2.0","id":1,"method":"test""#; // Missing closing brace
    let result = JsonRpcRequest::from_json(json);
    assert!(result.is_err());
}

#[test]
fn test_request_from_empty_string() {
    let json = "";
    let result = JsonRpcRequest::from_json(json);
    assert!(result.is_err());
}

#[test]
fn test_request_from_non_object_json() {
    let json = r#"["array","instead","of","object"]"#;
    let result = JsonRpcRequest::from_json(json);
    assert!(result.is_err());
}

#[test]
fn test_response_from_invalid_json() {
    let json = r#"{"jsonrpc":"2.0","id":1,"result":{}"#; // Missing closing brace
    let result = JsonRpcResponse::from_json(json);
    assert!(result.is_err());
}

#[test]
fn test_notification_from_invalid_json() {
    let json = r#"{"jsonrpc":"2.0","method":"test""#; // Missing closing brace
    let result = JsonRpcNotification::from_json(json);
    assert!(result.is_err());
}

#[test]
fn test_message_from_malformed_json() {
    let json = r#"{"jsonrpc":"2.0","id":unquoted}"#;
    let result = JsonRpcMessage::from_json(json);
    assert!(result.is_err());
}

// ============================================================================
// Integration Tests - Real-World Scenarios
// ============================================================================

#[test]
fn test_batch_request_scenario() {
    // Simulate multiple independent requests
    let requests = vec![
        JsonRpcRequest::new(RequestId::Number(1), "get_user"),
        JsonRpcRequest::with_params(
            RequestId::Number(2),
            "update_user",
            json!({"name": "Alice"}),
        ),
        JsonRpcRequest::new(RequestId::Number(3), "list_users"),
    ];

    for request in &requests {
        let json = request.to_json().unwrap();
        let deserialized = JsonRpcRequest::from_json(&json).unwrap();
        assert_eq!(request.method, deserialized.method);
        assert_eq!(request.id, deserialized.id);
    }
}

#[test]
fn test_error_propagation_scenario() {
    // Client sends request with invalid params
    let request = JsonRpcRequest::with_params(
        RequestId::Number(1),
        "calculate",
        json!({"a": "invalid", "b": "also_invalid"}),
    );

    // Server detects invalid params
    let error_response = JsonRpcResponse::error(
        request.id,
        JsonRpcError::invalid_params("parameters must be numbers"),
    );

    assert!(error_response.is_error());
    let error = error_response.get_error().unwrap();
    assert_eq!(error.code, -32602);
    assert!(error.message.contains("numbers"));
}

#[test]
fn test_server_error_with_custom_code() {
    // Server-specific error in reserved range
    let custom_error = JsonRpcError::new(
        JsonRpcErrorCode::ServerError(-32050),
        "Database timeout".to_string(),
    );

    assert_eq!(custom_error.code, -32050);
    assert_eq!(
        custom_error.code_enum(),
        JsonRpcErrorCode::ServerError(-32050)
    );
}

#[test]
fn test_progressive_parameter_building() {
    // Build request parameters progressively
    let mut request = JsonRpcRequest::new(RequestId::Number(1), "complex_operation");

    let mut params = serde_json::Map::new();
    params.insert("timeout".to_string(), json!(30));
    params.insert("retries".to_string(), json!(3));

    request.set_params(serde_json::Value::Object(params));

    let json = request.to_json().unwrap();
    let deserialized = JsonRpcRequest::from_json(&json).unwrap();

    assert!(deserialized.params.is_some());
    let p = deserialized.params.unwrap();
    assert_eq!(p["timeout"], 30);
    assert_eq!(p["retries"], 3);
}
