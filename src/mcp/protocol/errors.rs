// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! JSON-RPC 2.0 error types for MCP protocol

use serde::{Deserialize, Serialize};
use std::fmt;

/// Standard JSON-RPC 2.0 error codes
///
/// These codes are defined by the JSON-RPC 2.0 specification and MUST be used
/// for the corresponding error conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsonRpcErrorCode {
    /// Invalid JSON was received by the server
    ParseError,

    /// The JSON sent is not a valid Request object
    InvalidRequest,

    /// The method does not exist / is not available
    MethodNotFound,

    /// Invalid method parameter(s)
    InvalidParams,

    /// Internal JSON-RPC error
    InternalError,

    /// Server error (reserved range -32000 to -32099)
    ServerError(i32),
}

impl JsonRpcErrorCode {
    /// Create an error code from an i32 value
    pub fn from_i32(code: i32) -> Self {
        match code {
            -32700 => JsonRpcErrorCode::ParseError,
            -32600 => JsonRpcErrorCode::InvalidRequest,
            -32601 => JsonRpcErrorCode::MethodNotFound,
            -32602 => JsonRpcErrorCode::InvalidParams,
            -32603 => JsonRpcErrorCode::InternalError,
            code if (-32099..=-32000).contains(&code) => JsonRpcErrorCode::ServerError(code),
            _ => JsonRpcErrorCode::InternalError,
        }
    }

    /// Convert to i32 value
    pub fn as_i32(&self) -> i32 {
        match self {
            JsonRpcErrorCode::ParseError => -32700,
            JsonRpcErrorCode::InvalidRequest => -32600,
            JsonRpcErrorCode::MethodNotFound => -32601,
            JsonRpcErrorCode::InvalidParams => -32602,
            JsonRpcErrorCode::InternalError => -32603,
            JsonRpcErrorCode::ServerError(code) => *code,
        }
    }

    /// Get the error message for standard codes
    pub fn message(&self) -> &str {
        match self {
            JsonRpcErrorCode::ParseError => "Parse error",
            JsonRpcErrorCode::InvalidRequest => "Invalid Request",
            JsonRpcErrorCode::MethodNotFound => "Method not found",
            JsonRpcErrorCode::InvalidParams => "Invalid params",
            JsonRpcErrorCode::InternalError => "Internal error",
            JsonRpcErrorCode::ServerError(_) => "Server error",
        }
    }
}

impl fmt::Display for JsonRpcErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (code {})", self.message(), self.as_i32())
    }
}

/// JSON-RPC 2.0 error object
///
/// Contains error details for failed JSON-RPC requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// The error type that occurred
    pub code: i32,

    /// A short description of the error
    pub message: String,

    /// Additional information about the error (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// Create a new JSON-RPC error
    pub fn new(code: JsonRpcErrorCode, message: String) -> Self {
        JsonRpcError {
            code: code.as_i32(),
            message,
            data: None,
        }
    }

    /// Create a new error with additional data
    pub fn with_data(code: JsonRpcErrorCode, message: String, data: serde_json::Value) -> Self {
        JsonRpcError {
            code: code.as_i32(),
            message,
            data: Some(data),
        }
    }

    /// Create a method not found error
    pub fn method_not_found(method: &str) -> Self {
        Self::with_data(
            JsonRpcErrorCode::MethodNotFound,
            format!("Method not found: {}", method),
            serde_json::json!({ "method": method }),
        )
    }

    /// Create an invalid params error
    pub fn invalid_params(reason: &str) -> Self {
        Self::new(
            JsonRpcErrorCode::InvalidParams,
            format!("Invalid params: {}", reason),
        )
    }

    /// Create a parse error
    pub fn parse_error(detail: &str) -> Self {
        Self::new(
            JsonRpcErrorCode::ParseError,
            format!("Parse error: {}", detail),
        )
    }

    /// Create an invalid request error
    pub fn invalid_request(reason: &str) -> Self {
        Self::new(
            JsonRpcErrorCode::InvalidRequest,
            format!("Invalid request: {}", reason),
        )
    }

    /// Create an internal error
    pub fn internal_error(detail: &str) -> Self {
        Self::new(
            JsonRpcErrorCode::InternalError,
            format!("Internal error: {}", detail),
        )
    }

    /// Get the error code as an enum
    pub fn code_enum(&self) -> JsonRpcErrorCode {
        JsonRpcErrorCode::from_i32(self.code)
    }
}

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if let Some(ref data) = self.data {
            write!(f, " (data: {})", data)?;
        }
        Ok(())
    }
}

impl std::error::Error for JsonRpcError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_conversion() {
        assert_eq!(JsonRpcErrorCode::from_i32(-32700), JsonRpcErrorCode::ParseError);
        assert_eq!(JsonRpcErrorCode::from_i32(-32600), JsonRpcErrorCode::InvalidRequest);
        assert_eq!(JsonRpcErrorCode::from_i32(-32601), JsonRpcErrorCode::MethodNotFound);
        assert_eq!(JsonRpcErrorCode::from_i32(-32602), JsonRpcErrorCode::InvalidParams);
        assert_eq!(JsonRpcErrorCode::from_i32(-32603), JsonRpcErrorCode::InternalError);
        assert_eq!(JsonRpcErrorCode::from_i32(-32001), JsonRpcErrorCode::ServerError(-32001));
    }

    #[test]
    fn test_error_code_as_i32() {
        assert_eq!(JsonRpcErrorCode::ParseError.as_i32(), -32700);
        assert_eq!(JsonRpcErrorCode::InvalidRequest.as_i32(), -32600);
        assert_eq!(JsonRpcErrorCode::MethodNotFound.as_i32(), -32601);
        assert_eq!(JsonRpcErrorCode::InvalidParams.as_i32(), -32602);
        assert_eq!(JsonRpcErrorCode::InternalError.as_i32(), -32603);
        assert_eq!(JsonRpcErrorCode::ServerError(-32001).as_i32(), -32001);
    }

    #[test]
    fn test_error_code_message() {
        assert_eq!(JsonRpcErrorCode::ParseError.message(), "Parse error");
        assert_eq!(JsonRpcErrorCode::InvalidRequest.message(), "Invalid Request");
        assert_eq!(JsonRpcErrorCode::MethodNotFound.message(), "Method not found");
        assert_eq!(JsonRpcErrorCode::InvalidParams.message(), "Invalid params");
        assert_eq!(JsonRpcErrorCode::InternalError.message(), "Internal error");
        assert_eq!(JsonRpcErrorCode::ServerError(-32001).message(), "Server error");
    }

    #[test]
    fn test_error_creation() {
        let error = JsonRpcError::method_not_found("test_method");
        assert_eq!(error.code, -32601);
        assert!(error.message.contains("test_method"));
        assert!(error.data.is_some());

        let error = JsonRpcError::invalid_params("missing field");
        assert_eq!(error.code, -32602);
        assert!(error.message.contains("missing field"));

        let error = JsonRpcError::parse_error("unexpected token");
        assert_eq!(error.code, -32700);
        assert!(error.message.contains("unexpected token"));
    }

    #[test]
    fn test_error_serialization() {
        let error = JsonRpcError::method_not_found("test_method");
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"code\":-32601"));
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"data\""));
    }

    #[test]
    fn test_error_deserialization() {
        let json = r#"{"code":-32601,"message":"Method not found","data":{"method":"test"}}"#;
        let error: JsonRpcError = serde_json::from_str(json).unwrap();
        assert_eq!(error.code, -32601);
        assert!(error.message.contains("Method not found"));
        assert!(error.data.is_some());
    }

    #[test]
    fn test_error_display() {
        let error = JsonRpcError::method_not_found("test");
        let display = format!("{}", error);
        assert!(display.contains("-32601"));
        assert!(display.contains("Method not found"));
    }
}
