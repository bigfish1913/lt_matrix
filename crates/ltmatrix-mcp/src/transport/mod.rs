// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Transport Layer Abstraction for MCP
//!
//! This module provides the transport layer for MCP (Model Context Protocol) communication.
//! It supports multiple transport types including:
//!
//! - **Stdio Transport**: Communication via stdin/stdout with child processes
//! - **WebSocket Transport**: Communication over WebSocket connections (future)
//!
//! # Architecture
//!
//! The transport layer is designed around the [`Transport`] trait which provides:
//!
//! - Connection establishment and lifecycle management
//! - Bidirectional message streaming
//! - Message framing (delimiting messages in the byte stream)
//! - Error handling and recovery
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::mcp::transport::{Transport, StdioTransport, StdioConfig};
//! use ltmatrix::mcp::JsonRpcRequest;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create stdio transport for a local MCP server
//!     let config = StdioConfig::new("playwright-mcp-server");
//!     let mut transport = StdioTransport::new(config);
//!
//!     // Start the transport
//!     transport.start().await?;
//!
//!     // Send a message
//!     let request = JsonRpcRequest::new(
//!         ltmatrix::mcp::RequestId::Number(1),
//!         "initialize"
//!     );
//!     transport.send_request(request).await?;
//!
//!     // Receive a response
//!     let response = transport.receive().await?;
//!
//!     // Close the transport
//!     transport.close().await?;
//!
//!     Ok(())
//! }
//! ```

pub mod framing;
pub mod stdio;

use crate::protocol::errors::{McpError, McpErrorCode, McpResult};
use crate::protocol::messages::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use async_trait::async_trait;
use std::time::Duration;
use tokio::sync::mpsc;

// Re-export main types
pub use framing::{ContentLengthFramer, FramingError, LineDelimitedFramer, MessageFramer};
pub use stdio::{ChildProcess, StdioConfig, StdioTransport};

// ============================================================================
// Transport Configuration
// ============================================================================

/// Configuration for transport connections
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Transport type
    pub transport_type: TransportType,

    /// Connection timeout
    pub connect_timeout: Duration,

    /// Read timeout
    pub read_timeout: Duration,

    /// Write timeout
    pub write_timeout: Duration,

    /// Maximum message size (in bytes)
    pub max_message_size: usize,

    /// Buffer size for message channels
    pub channel_buffer_size: usize,

    /// Enable message logging for debugging
    pub debug_logging: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            transport_type: TransportType::Stdio(StdioConfig::default()),
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
            max_message_size: 10 * 1024 * 1024, // 10 MB
            channel_buffer_size: 100,
            debug_logging: false,
        }
    }
}

impl TransportConfig {
    /// Create a stdio transport config from a command
    pub fn stdio_command(command: impl Into<String>) -> Self {
        Self {
            transport_type: TransportType::Stdio(StdioConfig::new(command)),
            ..Default::default()
        }
    }

    /// Create a stdio transport config with command and arguments
    pub fn stdio_command_with_args(command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            transport_type: TransportType::Stdio(StdioConfig::new(command).with_args(args)),
            ..Default::default()
        }
    }

    /// Set connection timeout
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set read timeout
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout;
        self
    }

    /// Set write timeout
    pub fn with_write_timeout(mut self, timeout: Duration) -> Self {
        self.write_timeout = timeout;
        self
    }

    /// Set maximum message size
    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = size;
        self
    }

    /// Enable debug logging
    pub fn with_debug_logging(mut self, enabled: bool) -> Self {
        self.debug_logging = enabled;
        self
    }
}

/// Transport type enumeration
#[derive(Debug, Clone)]
pub enum TransportType {
    /// Stdio transport (local process communication)
    Stdio(StdioConfig),

    /// WebSocket transport (future implementation)
    #[allow(dead_code)]
    WebSocket(WebSocketConfig),
}

/// WebSocket configuration (placeholder for future implementation)
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct WebSocketConfig {
    /// WebSocket URL
    pub url: String,

    /// Enable TLS
    pub tls: bool,

    /// Headers to include in handshake
    pub headers: Vec<(String, String)>,
}

// ============================================================================
// Transport Trait
// ============================================================================

/// Trait for MCP transport implementations
///
/// This trait defines the interface for all transport types used in MCP communication.
/// Implementations must support:
///
/// - Asynchronous start/stop lifecycle
/// - Sending JSON-RPC requests and notifications
/// - Receiving JSON-RPC responses and notifications
/// - Proper error handling and connection management
#[async_trait]
pub trait Transport: Send + Sync {
    /// Start the transport connection
    ///
    /// This method initializes the transport and prepares it for communication.
    /// For stdio transport, this spawns the child process.
    /// For network transports, this establishes the connection.
    async fn start(&mut self) -> McpResult<()>;

    /// Close the transport connection
    ///
    /// Gracefully shuts down the transport, flushing any pending messages
    /// and releasing resources.
    async fn close(&mut self) -> McpResult<()>;

    /// Check if the transport is connected
    fn is_connected(&self) -> bool;

    /// Send a JSON-RPC request
    ///
    /// Sends a request message to the MCP server. The method returns once
    /// the message has been written to the transport.
    async fn send_request(&self, request: JsonRpcRequest) -> McpResult<()>;

    /// Send a JSON-RPC notification
    ///
    /// Sends a notification message to the MCP server. Notifications do not
    /// expect a response.
    async fn send_notification(&self, notification: JsonRpcNotification) -> McpResult<()>;

    /// Receive a message from the transport
    ///
    /// Waits for and returns the next message from the MCP server.
    /// This can be either a response or a notification.
    async fn receive(&self) -> McpResult<TransportMessage>;

    /// Get the sender channel for outgoing messages
    ///
    /// Returns a sender that can be used to send messages from other tasks.
    fn sender(&self) -> mpsc::Sender<OutgoingMessage>;

    /// Get the receiver channel for incoming messages
    ///
    /// Returns a receiver that can be used to receive messages in other tasks.
    fn receiver(&self) -> mpsc::Receiver<TransportMessage>;

    /// Get transport statistics
    fn stats(&self) -> TransportStats;
}

// ============================================================================
// Message Types
// ============================================================================

/// Messages received from the transport
#[derive(Debug, Clone)]
pub enum TransportMessage {
    /// JSON-RPC response (for requests)
    Response(JsonRpcResponse),

    /// JSON-RPC notification (server-initiated)
    Notification(JsonRpcNotification),

    /// Transport-level error
    Error(TransportError),
}

impl TransportMessage {
    /// Check if this is a response
    pub fn is_response(&self) -> bool {
        matches!(self, TransportMessage::Response(_))
    }

    /// Check if this is a notification
    pub fn is_notification(&self) -> bool {
        matches!(self, TransportMessage::Notification(_))
    }

    /// Check if this is an error
    pub fn is_error(&self) -> bool {
        matches!(self, TransportMessage::Error(_))
    }

    /// Get the response if this is one
    pub fn as_response(&self) -> Option<&JsonRpcResponse> {
        match self {
            TransportMessage::Response(response) => Some(response),
            _ => None,
        }
    }

    /// Get the notification if this is one
    pub fn as_notification(&self) -> Option<&JsonRpcNotification> {
        match self {
            TransportMessage::Notification(notification) => Some(notification),
            _ => None,
        }
    }
}

/// Messages to be sent through the transport
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum OutgoingMessage {
    /// JSON-RPC request
    Request(JsonRpcRequest),

    /// JSON-RPC notification
    Notification(JsonRpcNotification),

    /// Shutdown signal
    Shutdown,
}

// ============================================================================
// Transport Errors
// ============================================================================

/// Transport-level errors
#[derive(Debug, Clone)]
pub enum TransportError {
    /// Connection failed
    ConnectionFailed(String),

    /// Connection closed unexpectedly
    ConnectionClosed,

    /// Timeout while waiting for message
    Timeout(Duration),

    /// Message framing error
    FramingError(String),

    /// Message too large
    MessageTooLarge { actual: usize, max: usize },

    /// Serialization error
    SerializationError(String),

    /// Deserialization error
    DeserializationError(String),

    /// IO error
    IoError(String),

    /// Process error (for stdio transport)
    ProcessError { code: Option<i32>, message: String },

    /// Transport not connected
    NotConnected,

    /// Transport already started
    AlreadyStarted,
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::ConnectionFailed(msg) => {
                write!(f, "Connection failed: {}", msg)
            }
            TransportError::ConnectionClosed => {
                write!(f, "Connection closed unexpectedly")
            }
            TransportError::Timeout(duration) => {
                write!(f, "Timeout after {:?}", duration)
            }
            TransportError::FramingError(msg) => {
                write!(f, "Message framing error: {}", msg)
            }
            TransportError::MessageTooLarge { actual, max } => {
                write!(f, "Message too large: {} bytes (max: {})", actual, max)
            }
            TransportError::SerializationError(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
            TransportError::DeserializationError(msg) => {
                write!(f, "Deserialization error: {}", msg)
            }
            TransportError::IoError(msg) => {
                write!(f, "IO error: {}", msg)
            }
            TransportError::ProcessError { code, message } => {
                if let Some(code) = code {
                    write!(f, "Process error (exit code {}): {}", code, message)
                } else {
                    write!(f, "Process error: {}", message)
                }
            }
            TransportError::NotConnected => {
                write!(f, "Transport not connected")
            }
            TransportError::AlreadyStarted => {
                write!(f, "Transport already started")
            }
        }
    }
}

impl std::error::Error for TransportError {}

impl From<TransportError> for McpError {
    fn from(error: TransportError) -> Self {
        McpError::communication(error.to_string())
    }
}

// ============================================================================
// Transport Statistics
// ============================================================================

/// Statistics for a transport connection
#[derive(Debug, Clone, Default)]
pub struct TransportStats {
    /// Number of messages sent
    pub messages_sent: u64,

    /// Number of messages received
    pub messages_received: u64,

    /// Number of bytes sent
    pub bytes_sent: u64,

    /// Number of bytes received
    pub bytes_received: u64,

    /// Number of errors encountered
    pub error_count: u64,

    /// Connection start time (None if not connected)
    pub connected_since: Option<std::time::Instant>,

    /// Last message sent time
    pub last_sent: Option<std::time::Instant>,

    /// Last message received time
    pub last_received: Option<std::time::Instant>,
}

impl TransportStats {
    /// Create new stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a sent message
    pub fn record_sent(&mut self, bytes: usize) {
        self.messages_sent += 1;
        self.bytes_sent += bytes as u64;
        self.last_sent = Some(std::time::Instant::now());
    }

    /// Record a received message
    pub fn record_received(&mut self, bytes: usize) {
        self.messages_received += 1;
        self.bytes_received += bytes as u64;
        self.last_received = Some(std::time::Instant::now());
    }

    /// Record an error
    pub fn record_error(&mut self) {
        self.error_count += 1;
    }

    /// Mark as connected
    pub fn mark_connected(&mut self) {
        self.connected_since = Some(std::time::Instant::now());
    }

    /// Mark as disconnected
    pub fn mark_disconnected(&mut self) {
        self.connected_since = None;
    }

    /// Get connection duration
    pub fn connection_duration(&self) -> Option<Duration> {
        self.connected_since.map(|t| t.elapsed())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a transport from configuration
pub fn create_transport(config: TransportConfig) -> Box<dyn Transport> {
    let TransportConfig {
        transport_type,
        connect_timeout: _,
        read_timeout: _,
        write_timeout: _,
        max_message_size: _,
        channel_buffer_size,
        debug_logging,
    } = config;

    match transport_type {
        TransportType::Stdio(stdio_config) => {
            // Create a new config for the transport
            let transport_config = TransportConfig {
                transport_type: TransportType::Stdio(stdio_config.clone()),
                channel_buffer_size,
                debug_logging,
                ..Default::default()
            };
            Box::new(StdioTransport::with_config(stdio_config, transport_config))
        }
        TransportType::WebSocket(_) => {
            unimplemented!("WebSocket transport not yet implemented")
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_config_default() {
        let config = TransportConfig::default();
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.read_timeout, Duration::from_secs(30));
        assert_eq!(config.max_message_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_transport_config_builder() {
        let config = TransportConfig::stdio_command("test-server")
            .with_connect_timeout(Duration::from_secs(5))
            .with_read_timeout(Duration::from_secs(15))
            .with_max_message_size(1024)
            .with_debug_logging(true);

        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.read_timeout, Duration::from_secs(15));
        assert_eq!(config.max_message_size, 1024);
        assert!(config.debug_logging);
    }

    #[test]
    fn test_transport_message_classification() {
        let response = TransportMessage::Response(JsonRpcResponse::success(
            crate::RequestId::Number(1),
            serde_json::Value::Null,
        ));
        assert!(response.is_response());
        assert!(!response.is_notification());
        assert!(!response.is_error());

        let notification = TransportMessage::Notification(JsonRpcNotification::new("test"));
        assert!(!notification.is_response());
        assert!(notification.is_notification());
        assert!(!notification.is_error());

        let error = TransportMessage::Error(TransportError::ConnectionClosed);
        assert!(!error.is_response());
        assert!(!error.is_notification());
        assert!(error.is_error());
    }

    #[test]
    fn test_transport_error_display() {
        let error = TransportError::ConnectionFailed("test error".to_string());
        assert!(error.to_string().contains("Connection failed"));
        assert!(error.to_string().contains("test error"));

        let error = TransportError::MessageTooLarge {
            actual: 1000,
            max: 500,
        };
        assert!(error.to_string().contains("1000"));
        assert!(error.to_string().contains("500"));
    }

    #[test]
    fn test_transport_stats() {
        let mut stats = TransportStats::new();

        stats.mark_connected();
        assert!(stats.connected_since.is_some());

        stats.record_sent(100);
        assert_eq!(stats.messages_sent, 1);
        assert_eq!(stats.bytes_sent, 100);
        assert!(stats.last_sent.is_some());

        stats.record_received(200);
        assert_eq!(stats.messages_received, 1);
        assert_eq!(stats.bytes_received, 200);
        assert!(stats.last_received.is_some());

        stats.record_error();
        assert_eq!(stats.error_count, 1);

        stats.mark_disconnected();
        assert!(stats.connected_since.is_none());
    }

    #[test]
    fn test_transport_error_to_mcp_error() {
        let transport_error = TransportError::Timeout(Duration::from_secs(30));
        let mcp_error: McpError = transport_error.into();

        assert_eq!(mcp_error.code, McpErrorCode::TransportError);
    }
}
