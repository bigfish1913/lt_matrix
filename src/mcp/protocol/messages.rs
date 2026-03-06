//! JSON-RPC 2.0 message types for MCP protocol
//!
//! This module implements the three core message types defined by JSON-RPC 2.0:
//! - Request: A call from client to server expecting a response
//! - Response: A reply from server to client
//! - Notification: A one-way message from client to server or vice versa

use crate::mcp::protocol::errors::JsonRpcError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;

/// JSON-RPC 2.0 request identifier
///
/// The ID can be a string, number, or null. Each request must have a unique ID
/// to match responses.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RequestId {
    /// String identifier
    String(String),
    /// Number identifier
    Number(i64),
    /// Null identifier (rare, but allowed by spec)
    Null,
}

impl Serialize for RequestId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            RequestId::String(s) => s.serialize(serializer),
            RequestId::Number(n) => n.serialize(serializer),
            RequestId::Null => serializer.serialize_none(),
        }
    }
}

impl<'de> Deserialize<'de> for RequestId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        let value = Value::deserialize(deserializer)?;

        match value {
            Value::String(s) => Ok(RequestId::String(s)),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(RequestId::Number(i))
                } else if let Some(u) = n.as_u64() {
                    // Convert u64 to i64 (handle potential overflow)
                    Ok(RequestId::Number(u as i64))
                } else if let Some(f) = n.as_f64() {
                    // JSON doesn't have floats for IDs, but handle it
                    Ok(RequestId::Number(f as i64))
                } else {
                    Err(Error::custom("Invalid number format for request ID"))
                }
            }
            Value::Null => Ok(RequestId::Null),
            _ => Err(Error::custom("Request ID must be string, number, or null")),
        }
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestId::String(s) => write!(f, "\"{}\"", s),
            RequestId::Number(n) => write!(f, "{}", n),
            RequestId::Null => write!(f, "null"),
        }
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        RequestId::String(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        RequestId::String(s.to_string())
    }
}

impl From<i64> for RequestId {
    fn from(n: i64) -> Self {
        RequestId::Number(n)
    }
}

impl From<i32> for RequestId {
    fn from(n: i32) -> Self {
        RequestId::Number(n as i64)
    }
}

/// JSON-RPC 2.0 request message
///
/// A request is a call from client to server that expects a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Request identifier
    pub id: RequestId,

    /// Method name to invoke
    pub method: String,

    /// Method parameters (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request
    pub fn new(id: RequestId, method: impl Into<String>) -> Self {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params: None,
        }
    }

    /// Create a new request with parameters
    pub fn with_params(id: RequestId, method: impl Into<String>, params: Value) -> Self {
        let mut request = Self::new(id, method);
        request.params = Some(params);
        request
    }

    /// Create a new request without parameters
    pub fn without_params(id: RequestId, method: impl Into<String>) -> Self {
        Self::new(id, method)
    }

    /// Set the request parameters
    pub fn set_params(&mut self, params: Value) {
        self.params = Some(params);
    }

    /// Convert request to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Parse request from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// JSON-RPC 2.0 response message
///
/// A response is a reply from server to client for a previous request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Request identifier (must match the request)
    pub id: RequestId,

    /// Result on success (mutually exclusive with error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error on failure (mutually exclusive with result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a successful response
    pub fn success(id: RequestId, result: Value) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: RequestId, error: JsonRpcError) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    /// Check if this response is a success
    pub fn is_success(&self) -> bool {
        self.result.is_some() && self.error.is_none()
    }

    /// Check if this response is an error
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Get the result, if successful
    pub fn get_result(&self) -> Option<&Value> {
        self.result.as_ref()
    }

    /// Get the error, if failed
    pub fn get_error(&self) -> Option<&JsonRpcError> {
        self.error.as_ref()
    }

    /// Convert response to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Parse response from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// JSON-RPC 2.0 notification message
///
/// A notification is a one-way message that does not expect a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Method name
    pub method: String,

    /// Method parameters (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcNotification {
    /// Create a new notification
    pub fn new(method: impl Into<String>) -> Self {
        JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: None,
        }
    }

    /// Create a new notification with parameters
    pub fn with_params(method: impl Into<String>, params: Value) -> Self {
        let mut notification = Self::new(method);
        notification.params = Some(params);
        notification
    }

    /// Set the notification parameters
    pub fn set_params(&mut self, params: Value) {
        self.params = Some(params);
    }

    /// Convert notification to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Parse notification from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// JSON-RPC 2.0 message (enum of all message types)
///
/// This enum represents any JSON-RPC 2.0 message and is useful for
/// generic message handling.
#[derive(Debug, Clone)]
pub enum JsonRpcMessage {
    /// Request message (expects response)
    Request(JsonRpcRequest),

    /// Response message (reply to request)
    Response(JsonRpcResponse),

    /// Notification message (one-way)
    Notification(JsonRpcNotification),
}

impl JsonRpcMessage {
    /// Parse a JSON string into a JsonRpcMessage
    ///
    /// This method automatically detects the message type by checking
    /// for the presence of an "id" field.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let value: Value = serde_json::from_str(json)?;

        // Check if it's a notification (no "id" field)
        if value.get("id").is_none() {
            let notification = JsonRpcNotification::from_json(json)?;
            Ok(JsonRpcMessage::Notification(notification))
        } else {
            // Has "id" field, could be request or response
            // Check for "result" or "error" field to determine
            if value.get("result").is_some() || value.get("error").is_some() {
                let response = JsonRpcResponse::from_json(json)?;
                Ok(JsonRpcMessage::Response(response))
            } else {
                let request = JsonRpcRequest::from_json(json)?;
                Ok(JsonRpcMessage::Request(request))
            }
        }
    }

    /// Convert message to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        match self {
            JsonRpcMessage::Request(req) => req.to_json(),
            JsonRpcMessage::Response(resp) => resp.to_json(),
            JsonRpcMessage::Notification(notif) => notif.to_json(),
        }
    }

    /// Check if this is a request message
    pub fn is_request(&self) -> bool {
        matches!(self, JsonRpcMessage::Request(_))
    }

    /// Check if this is a response message
    pub fn is_response(&self) -> bool {
        matches!(self, JsonRpcMessage::Response(_))
    }

    /// Check if this is a notification message
    pub fn is_notification(&self) -> bool {
        matches!(self, JsonRpcMessage::Notification(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_from_string() {
        let id: RequestId = "test-id".into();
        assert_eq!(id, RequestId::String("test-id".to_string()));
    }

    #[test]
    fn test_request_id_from_number() {
        let id: RequestId = 42i64.into();
        assert_eq!(id, RequestId::Number(42));
    }

    #[test]
    fn test_request_id_serialization() {
        let id = RequestId::String("test".to_string());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"test\"");

        let id = RequestId::Number(42);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "42");

        let id = RequestId::Null;
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "null");
    }

    #[test]
    fn test_request_id_deserialization() {
        let id: RequestId = serde_json::from_str("\"test\"").unwrap();
        assert_eq!(id, RequestId::String("test".to_string()));

        let id: RequestId = serde_json::from_str("42").unwrap();
        assert_eq!(id, RequestId::Number(42));

        let id: RequestId = serde_json::from_str("null").unwrap();
        assert_eq!(id, RequestId::Null);
    }

    #[test]
    fn test_request_creation() {
        let request = JsonRpcRequest::new(RequestId::Number(1), "test_method");
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, RequestId::Number(1));
        assert_eq!(request.method, "test_method");
        assert!(request.params.is_none());
    }

    #[test]
    fn test_request_with_params() {
        let params = serde_json::json!({"arg1": "value1"});
        let request = JsonRpcRequest::with_params(RequestId::Number(1), "test_method", params);
        assert!(request.params.is_some());
    }

    #[test]
    fn test_request_serialization() {
        let request = JsonRpcRequest::new(RequestId::Number(1), "test_method");
        let json = request.to_json().unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"test_method\""));
    }

    #[test]
    fn test_request_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
        let request = JsonRpcRequest::from_json(json).unwrap();
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "test");
        assert!(request.params.is_some());
    }

    #[test]
    fn test_response_success() {
        let result = serde_json::json!({"status": "ok"});
        let response = JsonRpcResponse::success(RequestId::Number(1), result);
        assert!(response.is_success());
        assert!(!response.is_error());
        assert!(response.get_result().is_some());
        assert!(response.get_error().is_none());
    }

    #[test]
    fn test_response_error() {
        let error = JsonRpcError::method_not_found("test");
        let response = JsonRpcResponse::error(RequestId::Number(1), error);
        assert!(!response.is_success());
        assert!(response.is_error());
        assert!(response.get_result().is_none());
        assert!(response.get_error().is_some());
    }

    #[test]
    fn test_response_serialization() {
        let result = serde_json::json!({"answer": 42});
        let response = JsonRpcResponse::success(RequestId::Number(1), result);
        let json = response.to_json().unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"result\""));
    }

    #[test]
    fn test_response_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"answer":42}}"#;
        let response = JsonRpcResponse::from_json(json).unwrap();
        assert!(response.is_success());
        assert!(response.get_result().is_some());
    }

    #[test]
    fn test_notification_creation() {
        let notification = JsonRpcNotification::new("test_event");
        assert_eq!(notification.jsonrpc, "2.0");
        assert_eq!(notification.method, "test_event");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_notification_with_params() {
        let params = serde_json::json!({"value": 123});
        let notification = JsonRpcNotification::with_params("test_event", params);
        assert!(notification.params.is_some());
    }

    #[test]
    fn test_notification_serialization() {
        let notification = JsonRpcNotification::new("test_event");
        let json = notification.to_json().unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"test_event\""));
        // Notifications don't have "id" field
        assert!(!json.contains("\"id\""));
    }

    #[test]
    fn test_notification_deserialization() {
        let json = r#"{"jsonrpc":"2.0","method":"test","params":{}}"#;
        let notification = JsonRpcNotification::from_json(json).unwrap();
        assert_eq!(notification.method, "test");
        assert!(notification.params.is_some());
    }

    #[test]
    fn test_message_from_json_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
        let message = JsonRpcMessage::from_json(json).unwrap();
        assert!(message.is_request());
        assert!(!message.is_response());
        assert!(!message.is_notification());
    }

    #[test]
    fn test_message_from_json_response() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
        let message = JsonRpcMessage::from_json(json).unwrap();
        assert!(!message.is_request());
        assert!(message.is_response());
        assert!(!message.is_notification());
    }

    #[test]
    fn test_message_from_json_notification() {
        let json = r#"{"jsonrpc":"2.0","method":"test"}"#;
        let message = JsonRpcMessage::from_json(json).unwrap();
        assert!(!message.is_request());
        assert!(!message.is_response());
        assert!(message.is_notification());
    }

    #[test]
    fn test_message_roundtrip() {
        let request = JsonRpcRequest::new(RequestId::Number(42), "test_method");
        let json = request.to_json().unwrap();
        let message = JsonRpcMessage::from_json(&json).unwrap();
        assert!(message.is_request());
    }

    #[test]
    fn test_request_id_display() {
        let id = RequestId::String("test".to_string());
        assert_eq!(format!("{}", id), "\"test\"");

        let id = RequestId::Number(42);
        assert_eq!(format!("{}", id), "42");

        let id = RequestId::Null;
        assert_eq!(format!("{}", id), "null");
    }
}
