// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! MCP Error Types and Error Handling
//!
//! This module provides a comprehensive error type hierarchy for the MCP (Model Context Protocol)
//! implementation, covering:
//!
//! - **Protocol Errors**: JSON-RPC 2.0 protocol violations and MCP-specific protocol errors
//! - **Serialization Errors**: JSON parsing, serialization, and type conversion errors
//! - **Communication Errors**: Transport-level errors (stdio, network, process)
//! - **MCP-Specific Errors**: Tool execution, resource access, capability errors
//!
//! # Error Categories
//!
//! The error system follows the MCP specification with these categories:
//!
//! | Code Range | Category |
//! |------------|----------|
//! | -32700 | Parse error (JSON parsing) |
//! | -32600 to -32603 | JSON-RPC standard errors |
//! | -32000 to -32099 | Server errors |
//! | -32500 to -32599 | MCP-specific errors |
//!
//! # Example
//!
//! ```
//! use ltmatrix::mcp::protocol::errors::{McpError, McpErrorCode};
//!
//! // Create a protocol error
//! let error = McpError::protocol(McpErrorCode::InvalidRequest, "Missing jsonrpc version");
//!
//! // Create a tool execution error
//! let error = McpError::tool_execution("playwright", "Browser launch failed");
//!
//! // Convert to JSON-RPC error response
//! let json_error = error.to_json_rpc_error();
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use std::io;
use std::time::Duration;

// ============================================================================
// Error Code Definitions
// ============================================================================

/// Standard JSON-RPC 2.0 error codes
///
/// These codes are defined by the JSON-RPC 2.0 specification and MUST be used
/// for the corresponding error conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsonRpcErrorCode {
    /// Invalid JSON was received by the server (-32700)
    ParseError,

    /// The JSON sent is not a valid Request object (-32600)
    InvalidRequest,

    /// The method does not exist / is not available (-32601)
    MethodNotFound,

    /// Invalid method parameter(s) (-32602)
    InvalidParams,

    /// Internal JSON-RPC error (-32603)
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

/// MCP-specific error codes
///
/// These error codes are specific to the MCP protocol and extend the JSON-RPC
/// error code space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum McpErrorCode {
    // JSON-RPC Standard Errors (same as JsonRpcErrorCode for convenience)
    /// Invalid JSON was received (-32700)
    ParseError,

    /// Invalid Request object (-32600)
    InvalidRequest,

    /// Method does not exist (-32601)
    MethodNotFound,

    /// Invalid method parameters (-32602)
    InvalidParams,

    /// Internal JSON-RPC error (-32603)
    InternalError,

    // Server Error Range (-32000 to -32099)
    /// Server is shutting down (-32000)
    ServerShutdown,

    /// Server is starting up (-32001)
    ServerStarting,

    /// Request timeout (-32002)
    RequestTimeout,

    /// Too many concurrent requests (-32003)
    TooManyRequests,

    /// Rate limit exceeded (-32004)
    RateLimitExceeded,

    /// Resource temporarily unavailable (-32005)
    ResourceUnavailable,

    /// Custom server error (-32006 to -32099)
    ServerError(i32),

    // MCP-Specific Error Range (-32500 to -32599)
    /// Unknown capability (-32500)
    UnknownCapability,

    /// Capability not supported (-32501)
    CapabilityNotSupported,

    /// Tool not found (-32502)
    ToolNotFound,

    /// Tool execution error (-32503)
    ToolExecutionError,

    /// Resource not found (-32504)
    ResourceNotFound,

    /// Resource access denied (-32505)
    ResourceAccessDenied,

    /// Prompt not found (-32506)
    PromptNotFound,

    /// Invalid prompt arguments (-32507)
    InvalidPromptArguments,

    /// Sampling not supported (-32508)
    SamplingNotSupported,

    /// Content type not supported (-32509)
    ContentTypeNotSupported,

    /// Invalid URI (-32510)
    InvalidUri,

    /// Subscription error (-32511)
    SubscriptionError,

    /// Transport error (-32512)
    TransportError,

    /// Protocol version mismatch (-32513)
    VersionMismatch,

    /// Session error (-32514)
    SessionError,

    /// Configuration error (-32515)
    ConfigurationError,

    /// Request cancelled (-32516)
    RequestCancelled,

    /// Custom MCP error (-32550 to -32599)
    Custom(i32),
}

impl McpErrorCode {
    /// Create from i32 value
    pub fn from_i32(code: i32) -> Self {
        match code {
            // JSON-RPC Standard
            -32700 => McpErrorCode::ParseError,
            -32600 => McpErrorCode::InvalidRequest,
            -32601 => McpErrorCode::MethodNotFound,
            -32602 => McpErrorCode::InvalidParams,
            -32603 => McpErrorCode::InternalError,
            // Server Errors
            -32000 => McpErrorCode::ServerShutdown,
            -32001 => McpErrorCode::ServerStarting,
            -32002 => McpErrorCode::RequestTimeout,
            -32003 => McpErrorCode::TooManyRequests,
            -32004 => McpErrorCode::RateLimitExceeded,
            -32005 => McpErrorCode::ResourceUnavailable,
            code if (-32099..=-32006).contains(&code) => McpErrorCode::ServerError(code),
            // MCP-Specific
            -32500 => McpErrorCode::UnknownCapability,
            -32501 => McpErrorCode::CapabilityNotSupported,
            -32502 => McpErrorCode::ToolNotFound,
            -32503 => McpErrorCode::ToolExecutionError,
            -32504 => McpErrorCode::ResourceNotFound,
            -32505 => McpErrorCode::ResourceAccessDenied,
            -32506 => McpErrorCode::PromptNotFound,
            -32507 => McpErrorCode::InvalidPromptArguments,
            -32508 => McpErrorCode::SamplingNotSupported,
            -32509 => McpErrorCode::ContentTypeNotSupported,
            -32510 => McpErrorCode::InvalidUri,
            -32511 => McpErrorCode::SubscriptionError,
            -32512 => McpErrorCode::TransportError,
            -32513 => McpErrorCode::VersionMismatch,
            -32514 => McpErrorCode::SessionError,
            -32515 => McpErrorCode::ConfigurationError,
            -32516 => McpErrorCode::RequestCancelled,
            code if (-32599..=-32550).contains(&code) => McpErrorCode::Custom(code),
            // Default to internal error
            _ => McpErrorCode::InternalError,
        }
    }

    /// Convert to i32 value
    pub fn as_i32(&self) -> i32 {
        match self {
            // JSON-RPC Standard
            McpErrorCode::ParseError => -32700,
            McpErrorCode::InvalidRequest => -32600,
            McpErrorCode::MethodNotFound => -32601,
            McpErrorCode::InvalidParams => -32602,
            McpErrorCode::InternalError => -32603,
            // Server Errors
            McpErrorCode::ServerShutdown => -32000,
            McpErrorCode::ServerStarting => -32001,
            McpErrorCode::RequestTimeout => -32002,
            McpErrorCode::TooManyRequests => -32003,
            McpErrorCode::RateLimitExceeded => -32004,
            McpErrorCode::ResourceUnavailable => -32005,
            McpErrorCode::ServerError(code) => *code,
            // MCP-Specific
            McpErrorCode::UnknownCapability => -32500,
            McpErrorCode::CapabilityNotSupported => -32501,
            McpErrorCode::ToolNotFound => -32502,
            McpErrorCode::ToolExecutionError => -32503,
            McpErrorCode::ResourceNotFound => -32504,
            McpErrorCode::ResourceAccessDenied => -32505,
            McpErrorCode::PromptNotFound => -32506,
            McpErrorCode::InvalidPromptArguments => -32507,
            McpErrorCode::SamplingNotSupported => -32508,
            McpErrorCode::ContentTypeNotSupported => -32509,
            McpErrorCode::InvalidUri => -32510,
            McpErrorCode::SubscriptionError => -32511,
            McpErrorCode::TransportError => -32512,
            McpErrorCode::VersionMismatch => -32513,
            McpErrorCode::SessionError => -32514,
            McpErrorCode::ConfigurationError => -32515,
            McpErrorCode::RequestCancelled => -32516,
            McpErrorCode::Custom(code) => *code,
        }
    }

    /// Get the error message
    pub fn message(&self) -> &str {
        match self {
            // JSON-RPC Standard
            McpErrorCode::ParseError => "Parse error",
            McpErrorCode::InvalidRequest => "Invalid Request",
            McpErrorCode::MethodNotFound => "Method not found",
            McpErrorCode::InvalidParams => "Invalid params",
            McpErrorCode::InternalError => "Internal error",
            // Server Errors
            McpErrorCode::ServerShutdown => "Server shutdown",
            McpErrorCode::ServerStarting => "Server starting",
            McpErrorCode::RequestTimeout => "Request timeout",
            McpErrorCode::TooManyRequests => "Too many requests",
            McpErrorCode::RateLimitExceeded => "Rate limit exceeded",
            McpErrorCode::ResourceUnavailable => "Resource unavailable",
            McpErrorCode::ServerError(_) => "Server error",
            // MCP-Specific
            McpErrorCode::UnknownCapability => "Unknown capability",
            McpErrorCode::CapabilityNotSupported => "Capability not supported",
            McpErrorCode::ToolNotFound => "Tool not found",
            McpErrorCode::ToolExecutionError => "Tool execution error",
            McpErrorCode::ResourceNotFound => "Resource not found",
            McpErrorCode::ResourceAccessDenied => "Resource access denied",
            McpErrorCode::PromptNotFound => "Prompt not found",
            McpErrorCode::InvalidPromptArguments => "Invalid prompt arguments",
            McpErrorCode::SamplingNotSupported => "Sampling not supported",
            McpErrorCode::ContentTypeNotSupported => "Content type not supported",
            McpErrorCode::InvalidUri => "Invalid URI",
            McpErrorCode::SubscriptionError => "Subscription error",
            McpErrorCode::TransportError => "Transport error",
            McpErrorCode::VersionMismatch => "Protocol version mismatch",
            McpErrorCode::SessionError => "Session error",
            McpErrorCode::ConfigurationError => "Configuration error",
            McpErrorCode::RequestCancelled => "Request cancelled",
            McpErrorCode::Custom(_) => "Custom error",
        }
    }

    /// Check if this is a JSON-RPC standard error
    pub fn is_json_rpc_standard(&self) -> bool {
        matches!(
            self,
            McpErrorCode::ParseError
                | McpErrorCode::InvalidRequest
                | McpErrorCode::MethodNotFound
                | McpErrorCode::InvalidParams
                | McpErrorCode::InternalError
        )
    }

    /// Check if this is a server error
    pub fn is_server_error(&self) -> bool {
        matches!(
            self,
            McpErrorCode::ServerShutdown
                | McpErrorCode::ServerStarting
                | McpErrorCode::RequestTimeout
                | McpErrorCode::TooManyRequests
                | McpErrorCode::RateLimitExceeded
                | McpErrorCode::ResourceUnavailable
                | McpErrorCode::ServerError(_)
        )
    }

    /// Check if this is an MCP-specific error
    pub fn is_mcp_specific(&self) -> bool {
        !self.is_json_rpc_standard() && !self.is_server_error()
    }

    /// Check if the error is recoverable (client could retry)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            McpErrorCode::RequestTimeout
                | McpErrorCode::TooManyRequests
                | McpErrorCode::RateLimitExceeded
                | McpErrorCode::ResourceUnavailable
                | McpErrorCode::ServerStarting
                | McpErrorCode::TransportError
        )
    }
}

impl fmt::Display for McpErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (code {})", self.message(), self.as_i32())
    }
}

impl From<JsonRpcErrorCode> for McpErrorCode {
    fn from(code: JsonRpcErrorCode) -> Self {
        match code {
            JsonRpcErrorCode::ParseError => McpErrorCode::ParseError,
            JsonRpcErrorCode::InvalidRequest => McpErrorCode::InvalidRequest,
            JsonRpcErrorCode::MethodNotFound => McpErrorCode::MethodNotFound,
            JsonRpcErrorCode::InvalidParams => McpErrorCode::InvalidParams,
            JsonRpcErrorCode::InternalError => McpErrorCode::InternalError,
            JsonRpcErrorCode::ServerError(code) => McpErrorCode::ServerError(code),
        }
    }
}

impl From<McpErrorCode> for JsonRpcErrorCode {
    fn from(code: McpErrorCode) -> Self {
        match code {
            McpErrorCode::ParseError => JsonRpcErrorCode::ParseError,
            McpErrorCode::InvalidRequest => JsonRpcErrorCode::InvalidRequest,
            McpErrorCode::MethodNotFound => JsonRpcErrorCode::MethodNotFound,
            McpErrorCode::InvalidParams => JsonRpcErrorCode::InvalidParams,
            McpErrorCode::InternalError => JsonRpcErrorCode::InternalError,
            McpErrorCode::ServerError(code) => JsonRpcErrorCode::ServerError(code),
            // Map MCP-specific errors to internal error
            _ => JsonRpcErrorCode::InternalError,
        }
    }
}

// ============================================================================
// Error Categories
// ============================================================================

/// Error source categories for diagnostics and handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Protocol-level errors (JSON-RPC, MCP protocol violations)
    Protocol,

    /// Serialization/deserialization errors
    Serialization,

    /// Transport/communication errors
    Communication,

    /// Tool execution errors
    ToolExecution,

    /// Resource access errors
    ResourceAccess,

    /// Configuration errors
    Configuration,

    /// Internal application errors
    Internal,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::Protocol => write!(f, "Protocol"),
            ErrorCategory::Serialization => write!(f, "Serialization"),
            ErrorCategory::Communication => write!(f, "Communication"),
            ErrorCategory::ToolExecution => write!(f, "ToolExecution"),
            ErrorCategory::ResourceAccess => write!(f, "ResourceAccess"),
            ErrorCategory::Configuration => write!(f, "Configuration"),
            ErrorCategory::Internal => write!(f, "Internal"),
        }
    }
}

// ============================================================================
// JSON-RPC Error Object
// ============================================================================

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

    /// Convert to MCP error
    pub fn to_mcp_error(&self) -> McpError {
        McpError::from_json_rpc(self.clone())
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

// ============================================================================
// MCP Error
// ============================================================================

/// Comprehensive MCP error type
///
/// This is the main error type for MCP operations, providing rich context
/// and categorization for error handling and debugging.
#[derive(Debug, Clone)]
pub struct McpError {
    /// Error code
    pub code: McpErrorCode,

    /// Human-readable error message
    pub message: String,

    /// Error category for classification
    pub category: ErrorCategory,

    /// Additional structured data
    pub data: Option<serde_json::Value>,

    /// Source of the error (what component/module produced it)
    pub source: Option<String>,

    /// Request ID that caused the error (if applicable)
    pub request_id: Option<serde_json::Value>,

    /// Whether this error can be recovered by retrying
    pub recoverable: bool,

    /// Suggested retry delay (for recoverable errors)
    pub retry_after: Option<Duration>,
}

impl McpError {
    /// Create a new MCP error
    pub fn new(code: McpErrorCode, message: impl Into<String>) -> Self {
        let recoverable = code.is_recoverable();
        McpError {
            code,
            message: message.into(),
            category: ErrorCategory::Internal,
            data: None,
            source: None,
            request_id: None,
            recoverable,
            retry_after: None,
        }
    }

    /// Create an error with a specific category
    pub fn with_category(
        code: McpErrorCode,
        message: impl Into<String>,
        category: ErrorCategory,
    ) -> Self {
        let recoverable = code.is_recoverable();
        McpError {
            code,
            message: message.into(),
            category,
            data: None,
            source: None,
            request_id: None,
            recoverable,
            retry_after: None,
        }
    }

    /// Create a protocol error
    pub fn protocol(code: McpErrorCode, message: impl Into<String>) -> Self {
        Self::with_category(code, message, ErrorCategory::Protocol)
    }

    /// Create a serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::with_category(
            McpErrorCode::ParseError,
            message,
            ErrorCategory::Serialization,
        )
    }

    /// Create a communication error
    pub fn communication(message: impl Into<String>) -> Self {
        Self::with_category(
            McpErrorCode::TransportError,
            message,
            ErrorCategory::Communication,
        )
    }

    /// Create a tool execution error
    pub fn tool_execution(tool_name: &str, message: impl Into<String>) -> Self {
        let mut error = Self::with_category(
            McpErrorCode::ToolExecutionError,
            format!("Tool '{}' failed: {}", tool_name, message.into()),
            ErrorCategory::ToolExecution,
        );
        error.data = Some(serde_json::json!({ "tool": tool_name }));
        error
    }

    /// Create a resource not found error
    pub fn resource_not_found(uri: &str) -> Self {
        let mut error = Self::with_category(
            McpErrorCode::ResourceNotFound,
            format!("Resource not found: {}", uri),
            ErrorCategory::ResourceAccess,
        );
        error.data = Some(serde_json::json!({ "uri": uri }));
        error
    }

    /// Create a resource access denied error
    pub fn resource_access_denied(uri: &str, reason: &str) -> Self {
        let mut error = Self::with_category(
            McpErrorCode::ResourceAccessDenied,
            format!("Access denied to resource '{}': {}", uri, reason),
            ErrorCategory::ResourceAccess,
        );
        error.data = Some(serde_json::json!({ "uri": uri, "reason": reason }));
        error
    }

    /// Create a tool not found error
    pub fn tool_not_found(name: &str) -> Self {
        let mut error = Self::with_category(
            McpErrorCode::ToolNotFound,
            format!("Tool not found: {}", name),
            ErrorCategory::ToolExecution,
        );
        error.data = Some(serde_json::json!({ "tool": name }));
        error
    }

    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::with_category(
            McpErrorCode::ConfigurationError,
            message,
            ErrorCategory::Configuration,
        )
    }

    /// Create a timeout error
    pub fn timeout(operation: &str, duration: Duration) -> Self {
        let mut error = Self::with_category(
            McpErrorCode::RequestTimeout,
            format!("Operation '{}' timed out after {:?}", operation, duration),
            ErrorCategory::Communication,
        );
        error.recoverable = true;
        error.retry_after = Some(duration);
        error.data = Some(serde_json::json!({
            "operation": operation,
            "duration_ms": duration.as_millis()
        }));
        error
    }

    /// Create from a JSON-RPC error
    pub fn from_json_rpc(error: JsonRpcError) -> Self {
        McpError {
            code: McpErrorCode::from_i32(error.code),
            message: error.message,
            category: ErrorCategory::Protocol,
            data: error.data,
            source: None,
            request_id: None,
            recoverable: false,
            retry_after: None,
        }
    }

    /// Add structured data to the error
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Add source information
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Add request ID
    pub fn with_request_id(mut self, id: serde_json::Value) -> Self {
        self.request_id = Some(id);
        self
    }

    /// Mark as recoverable with retry delay
    pub fn with_retry(mut self, retry_after: Duration) -> Self {
        self.recoverable = true;
        self.retry_after = Some(retry_after);
        self
    }

    /// Convert to JSON-RPC error format
    pub fn to_json_rpc_error(&self) -> JsonRpcError {
        JsonRpcError {
            code: self.code.as_i32(),
            message: self.message.clone(),
            data: self.data.clone(),
        }
    }

    /// Convert to JSON value for responses
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "code": self.code.as_i32(),
            "message": self.message,
            "data": self.data,
        })
    }

    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        self.recoverable
    }

    /// Get suggested retry delay
    pub fn retry_delay(&self) -> Option<Duration> {
        self.retry_after
    }
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} ({})",
            self.code.as_i32(),
            self.message,
            self.category
        )?;
        if let Some(ref source) = self.source {
            write!(f, " [source: {}]", source)?;
        }
        if let Some(ref data) = self.data {
            write!(f, " [data: {}]", data)?;
        }
        Ok(())
    }
}

impl std::error::Error for McpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<JsonRpcError> for McpError {
    fn from(error: JsonRpcError) -> Self {
        Self::from_json_rpc(error)
    }
}

impl From<McpError> for JsonRpcError {
    fn from(error: McpError) -> Self {
        error.to_json_rpc_error()
    }
}

impl From<serde_json::Error> for McpError {
    fn from(error: serde_json::Error) -> Self {
        McpError::serialization(error.to_string())
    }
}

impl From<io::Error> for McpError {
    fn from(error: io::Error) -> Self {
        let message = error.to_string();
        let code = match error.kind() {
            io::ErrorKind::TimedOut => McpErrorCode::RequestTimeout,
            io::ErrorKind::WouldBlock => McpErrorCode::ResourceUnavailable,
            io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::NotConnected => McpErrorCode::TransportError,
            _ => McpErrorCode::TransportError,
        };
        McpError::with_category(code, message, ErrorCategory::Communication).with_source("io")
    }
}

// ============================================================================
// Error Result Types
// ============================================================================

/// Result type alias for MCP operations
pub type McpResult<T> = Result<T, McpError>;

/// Result type alias for JSON-RPC operations
pub type JsonRpcResult<T> = Result<T, JsonRpcError>;

// ============================================================================
// Error Builder
// ============================================================================

/// Builder for constructing MCP errors with detailed context
#[derive(Debug, Default)]
pub struct ErrorBuilder {
    code: Option<McpErrorCode>,
    message: Option<String>,
    category: Option<ErrorCategory>,
    data: Option<serde_json::Value>,
    source: Option<String>,
    request_id: Option<serde_json::Value>,
    recoverable: Option<bool>,
    retry_after: Option<Duration>,
}

impl ErrorBuilder {
    /// Create a new error builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the error code
    pub fn code(mut self, code: McpErrorCode) -> Self {
        self.code = Some(code);
        self
    }

    /// Set the error message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set the error category
    pub fn category(mut self, category: ErrorCategory) -> Self {
        self.category = Some(category);
        self
    }

    /// Set the error data
    pub fn data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Set the error source
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the request ID
    pub fn request_id(mut self, id: serde_json::Value) -> Self {
        self.request_id = Some(id);
        self
    }

    /// Mark as recoverable
    pub fn recoverable(mut self, recoverable: bool) -> Self {
        self.recoverable = Some(recoverable);
        self
    }

    /// Set retry delay
    pub fn retry_after(mut self, duration: Duration) -> Self {
        self.retry_after = Some(duration);
        self
    }

    /// Build the error
    pub fn build(self) -> McpError {
        let code = self.code.unwrap_or(McpErrorCode::InternalError);
        let message = self.message.unwrap_or_else(|| code.message().to_string());
        let category = self.category.unwrap_or(ErrorCategory::Internal);
        let recoverable = self.recoverable.unwrap_or_else(|| code.is_recoverable());

        McpError {
            code,
            message,
            category,
            data: self.data,
            source: self.source,
            request_id: self.request_id,
            recoverable,
            retry_after: self.retry_after,
        }
    }
}

// ============================================================================
// Helper Macros
// ============================================================================

/// Create an MCP error with code and message
#[macro_export]
macro_rules! mcp_error {
    ($code:expr, $message:expr) => {
        $crate::protocol::errors::McpError::new($code, $message)
    };
    ($code:expr, $message:expr, $($key:expr => $value:expr),* $(,)?) => {{
        let mut error = $crate::protocol::errors::McpError::new($code, $message);
        error.data = Some(serde_json::json!({ $($key: $value),* }));
        error
    }};
}

/// Create a tool execution error
#[macro_export]
macro_rules! tool_error {
    ($tool:expr, $message:expr) => {
        $crate::protocol::errors::McpError::tool_execution($tool, $message)
    };
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- JsonRpcErrorCode Tests ----

    #[test]
    fn test_json_rpc_error_code_conversion() {
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
        assert_eq!(
            JsonRpcErrorCode::from_i32(-32001),
            JsonRpcErrorCode::ServerError(-32001)
        );
    }

    #[test]
    fn test_json_rpc_error_code_as_i32() {
        assert_eq!(JsonRpcErrorCode::ParseError.as_i32(), -32700);
        assert_eq!(JsonRpcErrorCode::InvalidRequest.as_i32(), -32600);
        assert_eq!(JsonRpcErrorCode::MethodNotFound.as_i32(), -32601);
        assert_eq!(JsonRpcErrorCode::InvalidParams.as_i32(), -32602);
        assert_eq!(JsonRpcErrorCode::InternalError.as_i32(), -32603);
        assert_eq!(JsonRpcErrorCode::ServerError(-32001).as_i32(), -32001);
    }

    #[test]
    fn test_json_rpc_error_code_message() {
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
    }

    // ---- McpErrorCode Tests ----

    #[test]
    fn test_mcp_error_code_from_i32() {
        // JSON-RPC standard
        assert_eq!(McpErrorCode::from_i32(-32700), McpErrorCode::ParseError);
        assert_eq!(McpErrorCode::from_i32(-32600), McpErrorCode::InvalidRequest);
        assert_eq!(McpErrorCode::from_i32(-32601), McpErrorCode::MethodNotFound);
        assert_eq!(McpErrorCode::from_i32(-32602), McpErrorCode::InvalidParams);
        assert_eq!(McpErrorCode::from_i32(-32603), McpErrorCode::InternalError);

        // Server errors
        assert_eq!(McpErrorCode::from_i32(-32000), McpErrorCode::ServerShutdown);
        assert_eq!(McpErrorCode::from_i32(-32002), McpErrorCode::RequestTimeout);

        // MCP-specific
        assert_eq!(McpErrorCode::from_i32(-32502), McpErrorCode::ToolNotFound);
        assert_eq!(
            McpErrorCode::from_i32(-32503),
            McpErrorCode::ToolExecutionError
        );
        assert_eq!(
            McpErrorCode::from_i32(-32504),
            McpErrorCode::ResourceNotFound
        );
    }

    #[test]
    fn test_mcp_error_code_as_i32() {
        assert_eq!(McpErrorCode::ParseError.as_i32(), -32700);
        assert_eq!(McpErrorCode::ToolNotFound.as_i32(), -32502);
        assert_eq!(McpErrorCode::ToolExecutionError.as_i32(), -32503);
        assert_eq!(McpErrorCode::RequestTimeout.as_i32(), -32002);
    }

    #[test]
    fn test_mcp_error_code_classification() {
        // JSON-RPC standard
        assert!(McpErrorCode::ParseError.is_json_rpc_standard());
        assert!(McpErrorCode::InvalidRequest.is_json_rpc_standard());
        assert!(!McpErrorCode::ParseError.is_server_error());
        assert!(!McpErrorCode::ParseError.is_mcp_specific());

        // Server errors
        assert!(McpErrorCode::RequestTimeout.is_server_error());
        assert!(!McpErrorCode::RequestTimeout.is_json_rpc_standard());
        assert!(!McpErrorCode::RequestTimeout.is_mcp_specific());

        // MCP-specific
        assert!(McpErrorCode::ToolNotFound.is_mcp_specific());
        assert!(!McpErrorCode::ToolNotFound.is_json_rpc_standard());
        assert!(!McpErrorCode::ToolNotFound.is_server_error());
    }

    #[test]
    fn test_mcp_error_code_recoverable() {
        assert!(McpErrorCode::RequestTimeout.is_recoverable());
        assert!(McpErrorCode::TooManyRequests.is_recoverable());
        assert!(McpErrorCode::RateLimitExceeded.is_recoverable());
        assert!(McpErrorCode::ResourceUnavailable.is_recoverable());
        assert!(!McpErrorCode::ParseError.is_recoverable());
        assert!(!McpErrorCode::ToolNotFound.is_recoverable());
    }

    #[test]
    fn test_mcp_error_code_conversion_between_types() {
        let json_code = JsonRpcErrorCode::ParseError;
        let mcp_code: McpErrorCode = json_code.into();
        assert_eq!(mcp_code, McpErrorCode::ParseError);

        let mcp_code = McpErrorCode::ToolNotFound;
        let json_code: JsonRpcErrorCode = mcp_code.into();
        // MCP-specific codes map to InternalError
        assert_eq!(json_code, JsonRpcErrorCode::InternalError);
    }

    // ---- JsonRpcError Tests ----

    #[test]
    fn test_json_rpc_error_creation() {
        let error = JsonRpcError::method_not_found("test_method");
        assert_eq!(error.code, -32601);
        assert!(error.message.contains("test_method"));
        assert!(error.data.is_some());
    }

    #[test]
    fn test_json_rpc_error_serialization() {
        let error = JsonRpcError::method_not_found("test_method");
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"code\":-32601"));
        assert!(json.contains("\"message\""));
    }

    #[test]
    fn test_json_rpc_error_deserialization() {
        let json = r#"{"code":-32601,"message":"Method not found","data":{"method":"test"}}"#;
        let error: JsonRpcError = serde_json::from_str(json).unwrap();
        assert_eq!(error.code, -32601);
        assert!(error.message.contains("Method not found"));
    }

    // ---- McpError Tests ----

    #[test]
    fn test_mcp_error_protocol() {
        let error = McpError::protocol(McpErrorCode::InvalidRequest, "Missing jsonrpc field");
        assert_eq!(error.code, McpErrorCode::InvalidRequest);
        assert_eq!(error.category, ErrorCategory::Protocol);
        assert!(!error.is_recoverable());
    }

    #[test]
    fn test_mcp_error_serialization() {
        let error = McpError::serialization("Unexpected token at line 5");
        assert_eq!(error.code, McpErrorCode::ParseError);
        assert_eq!(error.category, ErrorCategory::Serialization);
    }

    #[test]
    fn test_mcp_error_communication() {
        let error = McpError::communication("Connection refused");
        assert_eq!(error.code, McpErrorCode::TransportError);
        assert_eq!(error.category, ErrorCategory::Communication);
    }

    #[test]
    fn test_mcp_error_tool_execution() {
        let error = McpError::tool_execution("playwright", "Browser launch failed");
        assert_eq!(error.code, McpErrorCode::ToolExecutionError);
        assert_eq!(error.category, ErrorCategory::ToolExecution);
        assert!(error.data.is_some());
        assert!(error.message.contains("playwright"));
    }

    #[test]
    fn test_mcp_error_resource_not_found() {
        let error = McpError::resource_not_found("file:///nonexistent.txt");
        assert_eq!(error.code, McpErrorCode::ResourceNotFound);
        assert_eq!(error.category, ErrorCategory::ResourceAccess);
        assert!(error.data.unwrap()["uri"]
            .as_str()
            .unwrap()
            .contains("nonexistent"));
    }

    #[test]
    fn test_mcp_error_timeout() {
        let error = McpError::timeout("tool_execution", Duration::from_secs(30));
        assert_eq!(error.code, McpErrorCode::RequestTimeout);
        assert!(error.is_recoverable());
        assert_eq!(error.retry_delay(), Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_mcp_error_builder() {
        let error = ErrorBuilder::new()
            .code(McpErrorCode::ToolExecutionError)
            .message("Tool failed")
            .category(ErrorCategory::ToolExecution)
            .data(serde_json::json!({ "tool": "test" }))
            .source("test_module")
            .recoverable(true)
            .retry_after(Duration::from_secs(5))
            .build();

        assert_eq!(error.code, McpErrorCode::ToolExecutionError);
        assert_eq!(error.message, "Tool failed");
        assert_eq!(error.category, ErrorCategory::ToolExecution);
        assert!(error.data.is_some());
        assert_eq!(error.source, Some("test_module".to_string()));
        assert!(error.is_recoverable());
        assert_eq!(error.retry_delay(), Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_mcp_error_to_json_rpc() {
        let error = McpError::tool_execution("test", "failed");
        let json_rpc = error.to_json_rpc_error();

        assert_eq!(json_rpc.code, -32503);
        assert!(json_rpc.message.contains("test"));
        assert!(json_rpc.data.is_some());
    }

    #[test]
    fn test_mcp_error_from_io_error() {
        let io_error = io::Error::new(io::ErrorKind::TimedOut, "operation timed out");
        let mcp_error: McpError = io_error.into();

        assert_eq!(mcp_error.code, McpErrorCode::RequestTimeout);
        assert_eq!(mcp_error.category, ErrorCategory::Communication);
    }

    #[test]
    fn test_mcp_error_display() {
        let error = McpError::tool_execution("test", "failed").with_source("test_module");

        let display = format!("{}", error);
        assert!(display.contains("-32503"));
        assert!(display.contains("ToolExecution"));
        assert!(display.contains("test_module"));
    }

    // ---- Error Category Tests ----

    #[test]
    fn test_error_category_display() {
        assert_eq!(format!("{}", ErrorCategory::Protocol), "Protocol");
        assert_eq!(format!("{}", ErrorCategory::Serialization), "Serialization");
        assert_eq!(format!("{}", ErrorCategory::Communication), "Communication");
        assert_eq!(format!("{}", ErrorCategory::ToolExecution), "ToolExecution");
    }
}
