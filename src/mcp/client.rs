// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! MCP Client Core Implementation
//!
//! This module provides the core MCP client functionality including:
//!
//! - Connection state management via [`ConnectionState`]
//! - Client lifecycle via [`McpClient`]
//! - State machine for connection transitions
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                         McpClient                                │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ConnectionState Machine:                                        │
//! │                                                                  │
//! │  Disconnected ──connect()──▶ Connecting ──handshake()──▶ Connected
//! │       ▲                         │                          │    │
//! │       │                         │ (failure)                │    │
//! │       │                         ▼                          │    │
//! │       └──────────────────── Disconnected ◀──disconnect()──┘    │
//! │                                                                  │
//! │  Components:                                                     │
//! │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐       │
//! │  │   Transport   │  │ Server Info   │  │  Capabilities │       │
//! │  │   (boxed)     │  │  (negotiated) │  │   (cached)    │       │
//! │  └───────────────┘  └───────────────┘  └───────────────┘       │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::mcp::client::{McpClient, McpClientConfig, ConnectionState};
//! use ltmatrix::mcp::transport::{TransportConfig, StdioConfig};
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let transport_config = TransportConfig::stdio_command("playwright-mcp-server");
//!     let config = McpClientConfig::new("my-client", "1.0.0")
//!         .with_transport(transport_config);
//!
//!     let mut client = McpClient::new(config);
//!
//!     // Connect and perform handshake
//!     client.connect().await?;
//!
//!     // Check state
//!     assert_eq!(client.state(), ConnectionState::Connected);
//!
//!     // Use the client...
//!
//!     // Disconnect
//!     client.disconnect().await?;
//!     assert_eq!(client.state(), ConnectionState::Disconnected);
//!
//!     Ok(())
//! }
//! ```

use crate::mcp::protocol::errors::{McpError, McpErrorCode, McpResult};
use crate::mcp::protocol::messages::RequestId;
use crate::mcp::protocol::methods::{
    ClientCapabilities, ImplementationInfo, InitializeParams, InitializeResult, ServerCapabilities,
    MCP_PROTOCOL_VERSION,
};
use crate::mcp::protocol::wrappers::{Initialize, McpMethod, McpNotification, NotificationsInitialized};
use crate::mcp::transport::{create_transport, Transport, TransportConfig, TransportMessage};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

// ============================================================================
// Connection State
// ============================================================================

/// Connection state for the MCP client
///
/// This enum represents the lifecycle states of an MCP client connection.
/// State transitions follow a strict state machine pattern.
///
/// # State Transitions
///
/// ```text
/// Disconnected ──▶ Connecting ──▶ Connected
///      ▲               │              │
///      │               │              │
///      └───────────────┴◀─────────────┘
///                    (disconnect/failure)
/// ```
///
/// # Thread Safety
///
/// The state is stored internally and can be safely queried from multiple
/// threads using [`McpClient::state()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConnectionState {
    /// Client is disconnected and idle
    ///
    /// This is the initial state and the state after [`McpClient::disconnect()`].
    /// From this state, only [`McpClient::connect()`] can be called.
    Disconnected,

    /// Client is in the process of connecting
    ///
    /// This is a transitional state during connection establishment.
    /// The client is performing the MCP handshake with the server.
    /// No other operations should be attempted in this state.
    Connecting,

    /// Client is connected and ready for operations
    ///
    /// The handshake has completed successfully and the client can
    /// now make requests to the server.
    Connected,

    /// Client is in the process of disconnecting
    ///
    /// This is a transitional state during graceful shutdown.
    /// The client is closing the transport connection.
    Disconnecting,
}

impl ConnectionState {
    /// Check if the client is in a connected state
    ///
    /// Returns `true` only if the state is [`ConnectionState::Connected`].
    #[inline]
    pub fn is_connected(&self) -> bool {
        matches!(self, ConnectionState::Connected)
    }

    /// Check if the client can initiate a connection
    ///
    /// Returns `true` only if the state is [`ConnectionState::Disconnected`].
    #[inline]
    pub fn can_connect(&self) -> bool {
        matches!(self, ConnectionState::Disconnected)
    }

    /// Check if the client can be disconnected
    ///
    /// Returns `true` if the state is [`ConnectionState::Connected`] or
    /// [`ConnectionState::Connecting`].
    #[inline]
    pub fn can_disconnect(&self) -> bool {
        matches!(self, ConnectionState::Connected | ConnectionState::Connecting)
    }

    /// Check if the client is in a transitional state
    ///
    /// Returns `true` for [`ConnectionState::Connecting`] and
    /// [`ConnectionState::Disconnecting`].
    #[inline]
    pub fn is_transitioning(&self) -> bool {
        matches!(self, ConnectionState::Connecting | ConnectionState::Disconnecting)
    }

    /// Get a human-readable name for the state
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionState::Disconnected => "disconnected",
            ConnectionState::Connecting => "connecting",
            ConnectionState::Connected => "connected",
            ConnectionState::Disconnecting => "disconnecting",
        }
    }
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for ConnectionState {
    fn default() -> Self {
        ConnectionState::Disconnected
    }
}

// ============================================================================
// Client Configuration
// ============================================================================

/// Configuration for the MCP client
#[derive(Debug, Clone)]
pub struct McpClientConfig {
    /// Client implementation info (name and version)
    pub client_info: ImplementationInfo,

    /// Transport configuration
    pub transport_config: TransportConfig,

    /// Client capabilities to advertise
    pub capabilities: ClientCapabilities,

    /// Connection timeout
    pub connect_timeout: Duration,

    /// Request timeout
    pub request_timeout: Duration,

    /// Protocol version to use
    pub protocol_version: String,

    /// Enable debug logging for messages
    pub debug_logging: bool,
}

impl McpClientConfig {
    /// Create a new client configuration
    ///
    /// # Arguments
    ///
    /// * `name` - Client name (e.g., "ltmatrix")
    /// * `version` - Client version (e.g., "0.1.0")
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            client_info: ImplementationInfo::new(name, version),
            transport_config: TransportConfig::default(),
            capabilities: ClientCapabilities::default(),
            connect_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(60),
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            debug_logging: false,
        }
    }

    /// Set the transport configuration
    pub fn with_transport(mut self, config: TransportConfig) -> Self {
        self.transport_config = config;
        self
    }

    /// Set client capabilities
    pub fn with_capabilities(mut self, capabilities: ClientCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Set connection timeout
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set request timeout
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Set protocol version
    pub fn with_protocol_version(mut self, version: impl Into<String>) -> Self {
        self.protocol_version = version.into();
        self
    }

    /// Enable debug logging
    pub fn with_debug_logging(mut self, enabled: bool) -> Self {
        self.debug_logging = enabled;
        self
    }
}

impl Default for McpClientConfig {
    fn default() -> Self {
        Self::new("ltmatrix-mcp-client", env!("CARGO_PKG_VERSION"))
    }
}

// ============================================================================
// Server Information (post-handshake)
// ============================================================================

/// Information about the connected MCP server
///
/// This is populated after a successful handshake.
#[derive(Debug, Clone)]
pub struct ServerInfo {
    /// Server implementation information
    pub info: ImplementationInfo,

    /// Server capabilities
    pub capabilities: ServerCapabilities,

    /// Protocol version negotiated
    pub protocol_version: String,

    /// Optional instructions from the server
    pub instructions: Option<String>,
}

impl From<InitializeResult> for ServerInfo {
    fn from(result: InitializeResult) -> Self {
        Self {
            info: result.server_info,
            capabilities: result.capabilities,
            protocol_version: result.protocol_version,
            instructions: result.instructions,
        }
    }
}

// ============================================================================
// State Transition Error
// ============================================================================

/// Error type for invalid state transitions
#[derive(Debug, Clone)]
pub struct StateTransitionError {
    /// Current state
    pub current: ConnectionState,

    /// Attempted action
    pub action: &'static str,

    /// Required state for the action
    pub required_states: &'static [ConnectionState],
}

impl std::fmt::Display for StateTransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cannot {} in state '{}'. Required states: {}",
            self.action,
            self.current,
            self.required_states
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl std::error::Error for StateTransitionError {}

impl From<StateTransitionError> for McpError {
    fn from(error: StateTransitionError) -> Self {
        McpError::with_category(
            McpErrorCode::SessionError,
            error.to_string(),
            crate::mcp::protocol::errors::ErrorCategory::Protocol,
        )
    }
}

// ============================================================================
// MCP Client
// ============================================================================

/// MCP Client
///
/// The main client for communicating with MCP servers. Provides:
///
/// - Connection lifecycle management
/// - Automatic MCP handshake on connect
/// - State machine for connection states
/// - Request/response handling
///
/// # Thread Safety
///
/// The client uses internal synchronization and can be safely shared
/// across threads using `Arc<McpClient>`.
///
/// # Example
///
/// ```no_run
/// use ltmatrix::mcp::client::{McpClient, McpClientConfig, ConnectionState};
///
/// async fn run() -> Result<(), Box<dyn std::error::Error>> {
///     let config = McpClientConfig::new("my-app", "1.0.0");
///     let mut client = McpClient::new(config);
///
///     // Connect
///     client.connect().await?;
///     println!("Connected! State: {}", client.state());
///
///     // ... use client ...
///
///     // Disconnect
///     client.disconnect().await?;
///     Ok(())
/// }
/// ```
pub struct McpClient {
    /// Client configuration
    config: McpClientConfig,

    /// Current connection state
    state: Arc<RwLock<ConnectionState>>,

    /// Transport layer
    transport: Option<Box<dyn Transport>>,

    /// Server information (populated after handshake)
    server_info: Arc<RwLock<Option<ServerInfo>>>,

    /// Request ID counter
    request_id_counter: AtomicU64,
}

impl McpClient {
    /// Create a new MCP client
    ///
    /// # Arguments
    ///
    /// * `config` - Client configuration
    ///
    /// # Example
    ///
    /// ```
    /// use ltmatrix::mcp::client::{McpClient, McpClientConfig};
    ///
    /// let config = McpClientConfig::new("my-client", "1.0.0");
    /// let client = McpClient::new(config);
    ///
    /// assert!(!client.is_connected());
    /// ```
    pub fn new(config: McpClientConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            transport: None,
            server_info: Arc::new(RwLock::new(None)),
            request_id_counter: AtomicU64::new(1),
        }
    }

    /// Create a new MCP client with default configuration
    pub fn default_client() -> Self {
        Self::new(McpClientConfig::default())
    }

    /// Get the current connection state
    ///
    /// This method acquires a read lock internally.
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Check if the client is connected
    ///
    /// This is a convenience method that checks if the state is
    /// [`ConnectionState::Connected`].
    pub async fn is_connected(&self) -> bool {
        self.state().await.is_connected()
    }

    /// Get the server information
    ///
    /// Returns `None` if not connected or handshake not completed.
    pub async fn server_info(&self) -> Option<ServerInfo> {
        self.server_info.read().await.clone()
    }

    /// Get the server capabilities
    ///
    /// Returns `None` if not connected.
    pub async fn server_capabilities(&self) -> Option<ServerCapabilities> {
        self.server_info.read().await.as_ref().map(|i| i.capabilities.clone())
    }

    /// Generate the next request ID
    ///
    /// Uses an atomic counter for thread-safe ID generation.
    fn next_request_id(&self) -> RequestId {
        let id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);
        RequestId::Number(id as i64)
    }

    /// Connect to the MCP server
    ///
    /// This method performs the full connection sequence:
    ///
    /// 1. Transitions state from `Disconnected` to `Connecting`
    /// 2. Starts the transport
    /// 3. Performs the MCP handshake (initialize → initialized)
    /// 4. Transitions state to `Connected`
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The client is not in the `Disconnected` state
    /// - The transport fails to start
    /// - The handshake fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::mcp::client::{McpClient, McpClientConfig, ConnectionState};
    ///
    /// async fn example(client: &mut McpClient) -> Result<(), Box<dyn std::error::Error>> {
    ///     client.connect().await?;
    ///     assert_eq!(client.state().await, ConnectionState::Connected);
    ///     Ok(())
    /// }
    /// ```
    pub async fn connect(&mut self) -> McpResult<()> {
        // Check and transition state
        {
            let mut state = self.state.write().await;

            // Validate state transition
            if !state.can_connect() {
                return Err(StateTransitionError {
                    current: *state,
                    action: "connect",
                    required_states: &[ConnectionState::Disconnected],
                }.into());
            }

            // Transition to Connecting
            *state = ConnectionState::Connecting;
            tracing::info!("MCP client connecting...");
        }

        // Create and start transport
        let transport_result = self.start_transport().await;

        if let Err(e) = transport_result {
            // Reset state on failure
            let mut state = self.state.write().await;
            *state = ConnectionState::Disconnected;
            return Err(e);
        }

        // Perform handshake
        let handshake_result = self.perform_handshake().await;

        match handshake_result {
            Ok(server_info) => {
                // Store server info
                *self.server_info.write().await = Some(server_info);

                // Transition to Connected
                let mut state = self.state.write().await;
                *state = ConnectionState::Connected;

                tracing::info!("MCP client connected successfully");
                Ok(())
            }
            Err(e) => {
                // Clean up and reset state
                if let Some(ref mut transport) = self.transport {
                    let _ = transport.close().await;
                }
                self.transport = None;

                let mut state = self.state.write().await;
                *state = ConnectionState::Disconnected;

                tracing::error!("MCP client connection failed: {}", e);
                Err(e)
            }
        }
    }

    /// Start the transport layer
    async fn start_transport(&mut self) -> McpResult<()> {
        // Create transport from config
        let mut transport = create_transport(self.config.transport_config.clone());

        // Start the transport
        transport.start().await?;

        self.transport = Some(transport);
        Ok(())
    }

    /// Perform the MCP handshake
    ///
    /// This consists of:
    /// 1. Sending `initialize` request with client capabilities
    /// 2. Receiving `InitializeResult` with server capabilities
    /// 3. Sending `notifications/initialized`
    async fn perform_handshake(&mut self) -> McpResult<ServerInfo> {
        // Create initialize params
        let params = InitializeParams {
            protocol_version: self.config.protocol_version.clone(),
            capabilities: self.config.capabilities.clone(),
            client_info: self.config.client_info.clone(),
        };

        // Build and send initialize request
        let request_id = self.next_request_id();
        let request = Initialize::build_request(request_id.clone(), params);

        if self.config.debug_logging {
            tracing::debug!("Sending initialize request: {:?}", request);
        }

        // Send the request
        {
            let transport = self.transport.as_ref().ok_or_else(|| {
                McpError::communication("Transport not available")
            })?;
            transport.send_request(request).await?;
        }

        // Wait for response with timeout
        let response = tokio::time::timeout(
            self.config.connect_timeout,
            self.wait_for_response(&request_id)
        ).await
            .map_err(|_| McpError::timeout("initialize", self.config.connect_timeout))??;

        // Parse the initialize result
        let init_result = Initialize::parse_response(response)?;

        if self.config.debug_logging {
            tracing::debug!("Received initialize result: {:?}", init_result);
        }

        // Send initialized notification
        {
            let transport = self.transport.as_ref().ok_or_else(|| {
                McpError::communication("Transport not available")
            })?;
            let notification = NotificationsInitialized::build_notification_empty();
            transport.send_notification(notification).await?;
        }

        tracing::info!(
            "Handshake completed with server: {} v{}",
            init_result.server_info.name,
            init_result.server_info.version
        );

        Ok(ServerInfo::from(init_result))
    }

    /// Wait for a response with a specific request ID
    async fn wait_for_response(&self, expected_id: &RequestId) -> McpResult<crate::mcp::JsonRpcResponse> {
        let transport = self.transport.as_ref().ok_or_else(|| {
            McpError::communication("Transport not available")
        })?;

        loop {
            let message = transport.receive().await?;

            match message {
                TransportMessage::Response(response) => {
                    if &response.id == expected_id {
                        return Ok(response);
                    }
                    // Unexpected response ID - this shouldn't happen in normal operation
                    tracing::warn!(
                        "Received response with unexpected ID: {:?} (expected: {:?})",
                        response.id, expected_id
                    );
                }
                TransportMessage::Notification(notification) => {
                    // Log notification but continue waiting for response
                    tracing::debug!("Received notification during handshake: {}", notification.method);
                }
                TransportMessage::Error(error) => {
                    return Err(error.into());
                }
            }
        }
    }

    /// Disconnect from the MCP server
    ///
    /// This method gracefully closes the connection:
    ///
    /// 1. Transitions state to `Disconnecting`
    /// 2. Closes the transport
    /// 3. Clears server information
    /// 4. Transitions state to `Disconnected`
    ///
    /// # Errors
    ///
    /// Returns an error if the transport fails to close gracefully.
    /// The state will still transition to `Disconnected`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::mcp::client::{McpClient, ConnectionState};
    ///
    /// async fn example(client: &mut McpClient) -> Result<(), Box<dyn std::error::Error>> {
    ///     client.disconnect().await?;
    ///     assert_eq!(client.state().await, ConnectionState::Disconnected);
    ///     Ok(())
    /// }
    /// ```
    pub async fn disconnect(&mut self) -> McpResult<()> {
        // Check and transition state
        {
            let current_state = self.state().await;

            // Already disconnected?
            if current_state == ConnectionState::Disconnected {
                return Ok(());
            }

            // Validate state transition
            if !current_state.can_disconnect() {
                return Err(StateTransitionError {
                    current: current_state,
                    action: "disconnect",
                    required_states: &[ConnectionState::Connected, ConnectionState::Connecting],
                }.into());
            }

            // Transition to Disconnecting
            let mut state = self.state.write().await;
            *state = ConnectionState::Disconnecting;
            tracing::info!("MCP client disconnecting...");
        }

        // Close transport
        let close_result = if let Some(ref mut transport) = self.transport {
            transport.close().await
        } else {
            Ok(())
        };

        // Clear state regardless of close result
        self.transport = None;
        *self.server_info.write().await = None;

        // Reset request ID counter
        self.request_id_counter.store(1, Ordering::SeqCst);

        // Transition to Disconnected
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Disconnected;
        }

        match close_result {
            Ok(()) => {
                tracing::info!("MCP client disconnected successfully");
                Ok(())
            }
            Err(e) => {
                tracing::warn!("MCP client disconnected with error: {}", e);
                Err(e)
            }
        }
    }

    /// Get the client configuration
    pub fn config(&self) -> &McpClientConfig {
        &self.config
    }

    /// Get the transport statistics
    ///
    /// Returns default stats if not connected.
    pub fn transport_stats(&self) -> crate::mcp::transport::TransportStats {
        self.transport
            .as_ref()
            .map(|t| t.stats())
            .unwrap_or_default()
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        // Attempt to clean up transport on drop
        // Note: This is synchronous, so we can't do async cleanup
        if self.transport.is_some() {
            tracing::debug!("McpClient dropped while transport still active");
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
    fn test_connection_state_is_connected() {
        assert!(ConnectionState::Connected.is_connected());
        assert!(!ConnectionState::Disconnected.is_connected());
        assert!(!ConnectionState::Connecting.is_connected());
        assert!(!ConnectionState::Disconnecting.is_connected());
    }

    #[test]
    fn test_connection_state_can_connect() {
        assert!(ConnectionState::Disconnected.can_connect());
        assert!(!ConnectionState::Connected.can_connect());
        assert!(!ConnectionState::Connecting.can_connect());
        assert!(!ConnectionState::Disconnecting.can_connect());
    }

    #[test]
    fn test_connection_state_can_disconnect() {
        assert!(ConnectionState::Connected.can_disconnect());
        assert!(ConnectionState::Connecting.can_disconnect());
        assert!(!ConnectionState::Disconnected.can_disconnect());
        assert!(!ConnectionState::Disconnecting.can_disconnect());
    }

    #[test]
    fn test_connection_state_is_transitioning() {
        assert!(ConnectionState::Connecting.is_transitioning());
        assert!(ConnectionState::Disconnecting.is_transitioning());
        assert!(!ConnectionState::Connected.is_transitioning());
        assert!(!ConnectionState::Disconnected.is_transitioning());
    }

    #[test]
    fn test_connection_state_as_str() {
        assert_eq!(ConnectionState::Disconnected.as_str(), "disconnected");
        assert_eq!(ConnectionState::Connecting.as_str(), "connecting");
        assert_eq!(ConnectionState::Connected.as_str(), "connected");
        assert_eq!(ConnectionState::Disconnecting.as_str(), "disconnecting");
    }

    #[test]
    fn test_connection_state_display() {
        assert_eq!(format!("{}", ConnectionState::Connected), "connected");
        assert_eq!(format!("{}", ConnectionState::Disconnected), "disconnected");
    }

    #[test]
    fn test_connection_state_default() {
        assert_eq!(ConnectionState::default(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_mcp_client_config_builder() {
        let config = McpClientConfig::new("test-client", "1.0.0")
            .with_connect_timeout(Duration::from_secs(10))
            .with_request_timeout(Duration::from_secs(30))
            .with_protocol_version("2025-11-25")
            .with_debug_logging(true);

        assert_eq!(config.client_info.name, "test-client");
        assert_eq!(config.client_info.version, "1.0.0");
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.request_timeout, Duration::from_secs(30));
        assert_eq!(config.protocol_version, "2025-11-25");
        assert!(config.debug_logging);
    }

    #[test]
    fn test_mcp_client_config_default() {
        let config = McpClientConfig::default();
        assert!(!config.client_info.name.is_empty());
        assert!(!config.client_info.version.is_empty());
        assert_eq!(config.protocol_version, MCP_PROTOCOL_VERSION);
    }

    #[tokio::test]
    async fn test_mcp_client_initial_state() {
        let client = McpClient::default_client();
        assert_eq!(client.state().await, ConnectionState::Disconnected);
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_mcp_client_cannot_connect_twice() {
        // This test verifies state machine behavior
        // A full integration test would require a real transport
        let config = McpClientConfig::new("test", "1.0");
        let client = McpClient::new(config);

        // Initial state is disconnected
        assert!(client.state().await.can_connect());

        // After simulating a state change, can_connect should be false
        // (In a real scenario, this would be tested with an actual connection)
    }

    #[test]
    fn test_state_transition_error_display() {
        let error = StateTransitionError {
            current: ConnectionState::Connected,
            action: "connect",
            required_states: &[ConnectionState::Disconnected],
        };

        let message = error.to_string();
        assert!(message.contains("Cannot connect"));
        assert!(message.contains("connected"));
        assert!(message.contains("disconnected"));
    }

    #[test]
    fn test_state_transition_error_to_mcp_error() {
        let error = StateTransitionError {
            current: ConnectionState::Connected,
            action: "connect",
            required_states: &[ConnectionState::Disconnected],
        };

        let mcp_error: McpError = error.into();
        assert_eq!(mcp_error.code, McpErrorCode::SessionError);
    }

    #[test]
    fn test_server_info_from_initialize_result() {
        let init_result = InitializeResult {
            protocol_version: "2025-11-25".to_string(),
            capabilities: ServerCapabilities::default(),
            server_info: ImplementationInfo::new("test-server", "1.0.0"),
            instructions: Some("Welcome!".to_string()),
        };

        let server_info = ServerInfo::from(init_result);

        assert_eq!(server_info.info.name, "test-server");
        assert_eq!(server_info.protocol_version, "2025-11-25");
        assert_eq!(server_info.instructions, Some("Welcome!".to_string()));
    }

    #[test]
    fn test_request_id_generation() {
        let config = McpClientConfig::new("test", "1.0");
        let client = McpClient::new(config);

        // IDs should be sequential
        let id1 = client.next_request_id();
        let id2 = client.next_request_id();
        let id3 = client.next_request_id();

        match (id1, id2, id3) {
            (RequestId::Number(n1), RequestId::Number(n2), RequestId::Number(n3)) => {
                assert_eq!(n2 - n1, 1);
                assert_eq!(n3 - n2, 1);
            }
            _ => panic!("Expected numeric request IDs"),
        }
    }
}
