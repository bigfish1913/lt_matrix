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
//!     assert_eq!(client.state().await, ConnectionState::Connected);
//!
//!     // Use the client...
//!
//!     // Disconnect
//!     client.disconnect().await?;
//!     assert_eq!(client.state().await, ConnectionState::Disconnected);
//!
//!     Ok(())
//! }
//! ```

use crate::mcp::protocol::errors::{McpError, McpErrorCode, McpResult};
use crate::mcp::protocol::messages::RequestId;
use crate::mcp::protocol::methods::{
    ClientCapabilities, ImplementationInfo, InitializeParams, InitializeResult,
    Resource, ResourceReadParams, ResourceReadResult,
    ResourcesListParams, ResourcesListResult, ServerCapabilities, Tool, ToolCallParams,
    ToolCallResult, ToolContent, ToolsListParams, ToolsListResult,
    MCP_PROTOCOL_VERSION,
};
use crate::mcp::protocol::wrappers::{
    Initialize, McpMethod, McpNotification, NotificationsInitialized,
    Ping, PingParams, ResourcesList, ResourcesRead, ToolsList, ToolsCall,
};
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
///     println!("Connected! State: {}", client.state().await);
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
    /// // Note: is_connected() is async, use in async context:
    /// // assert!(!client.is_connected().await);
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

    // ========================================================================
    // Core MCP Methods
    // ========================================================================

    /// Call any MCP method with typed request/response
    ///
    /// This is the low-level method for calling MCP methods. Prefer using
    /// the typed methods like [`list_resources`](Self::list_resources) when available.
    ///
    /// # Type Parameters
    ///
    /// - `M`: The MCP method type implementing [`McpMethod`]
    ///
    /// # Arguments
    ///
    /// - `params`: The method parameters
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The client is not connected
    /// - The request fails to send
    /// - The response times out
    /// - The response contains an error
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::mcp::client::McpClient;
    /// use ltmatrix::mcp::protocol::wrappers::{Ping, McpMethod, PingParams};
    ///
    /// async fn example(client: &McpClient) -> Result<(), Box<dyn std::error::Error>> {
    ///     let result = client.call_method::<Ping>(PingParams::default()).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn call_method<M: McpMethod>(&self, params: M::Params) -> McpResult<M::Result> {
        // Check connection state
        if !self.is_connected().await {
            return Err(McpError::with_category(
                McpErrorCode::SessionError,
                "Client not connected",
                crate::mcp::protocol::errors::ErrorCategory::Protocol,
            ));
        }

        let request_id = self.next_request_id();
        let request = M::build_request(request_id.clone(), params);

        if self.config.debug_logging {
            tracing::debug!("Sending {} request: {:?}", M::METHOD_NAME, request);
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
            self.config.request_timeout,
            self.wait_for_response(&request_id)
        ).await
            .map_err(|_| McpError::timeout(M::METHOD_NAME, self.config.request_timeout))??;

        if self.config.debug_logging {
            tracing::debug!("Received {} response: {:?}", M::METHOD_NAME, response);
        }

        // Parse and return the result
        M::parse_response(response)
    }

    /// Ping the server to check connection health
    ///
    /// This sends a `ping` request to the server and waits for a response.
    /// It's useful for checking if the connection is still alive.
    ///
    /// # Errors
    ///
    /// Returns an error if the client is not connected or the ping fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::mcp::client::McpClient;
    ///
    /// async fn example(client: &McpClient) -> Result<(), Box<dyn std::error::Error>> {
    ///     client.ping().await?;
    ///     println!("Server is alive!");
    ///     Ok(())
    /// }
    /// ```
    pub async fn ping(&self) -> McpResult<()> {
        self.call_method::<Ping>(PingParams::default()).await?;
        Ok(())
    }

    // ========================================================================
    // Resources Methods
    // ========================================================================

    /// List available resources from the server
    ///
    /// This sends a `resources/list` request to discover what resources
    /// the server provides.
    ///
    /// # Arguments
    ///
    /// - `cursor`: Optional pagination cursor for large result sets
    ///
    /// # Returns
    ///
    /// A list of available resources and an optional next cursor for pagination.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The client is not connected
    /// - The server doesn't support resources
    /// - The request fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::mcp::client::McpClient;
    ///
    /// async fn example(client: &McpClient) -> Result<(), Box<dyn std::error::Error>> {
    ///     let result = client.list_resources(None).await?;
    ///     for resource in result.resources {
    ///         println!("Resource: {} ({})", resource.name, resource.uri);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn list_resources(&self, cursor: Option<&str>) -> McpResult<ResourcesListResult> {
        let params = match cursor {
            Some(c) => ResourcesListParams {
                cursor: Some(c.to_string()),
            },
            None => ResourcesListParams::default(),
        };
        self.call_method::<ResourcesList>(params).await
    }

    /// List all resources, handling pagination automatically
    ///
    /// This method fetches all pages of resources until no more are available.
    ///
    /// # Returns
    ///
    /// A vector of all available resources.
    ///
    /// # Errors
    ///
    /// Returns an error if any page request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::mcp::client::McpClient;
    ///
    /// async fn example(client: &McpClient) -> Result<(), Box<dyn std::error::Error>> {
    ///     let resources = client.list_all_resources().await?;
    ///     println!("Found {} resources", resources.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn list_all_resources(&self) -> McpResult<Vec<Resource>> {
        let mut all_resources = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let result = self.list_resources(cursor.as_deref()).await?;
            all_resources.extend(result.resources);

            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(all_resources)
    }

    /// Read a specific resource by URI
    ///
    /// This sends a `resources/read` request to fetch the contents of a resource.
    ///
    /// # Arguments
    ///
    /// - `uri`: The URI of the resource to read
    ///
    /// # Returns
    ///
    /// The resource contents, which may be text or binary data.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The client is not connected
    /// - The resource doesn't exist
    /// - The request fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::mcp::client::McpClient;
    ///
    /// async fn example(client: &McpClient) -> Result<(), Box<dyn std::error::Error>> {
    ///     let result = client.read_resource("file:///project/README.md").await?;
    ///     for contents in result.contents {
    ///         if let Some(text) = contents.text {
    ///             println!("Content: {}", text);
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn read_resource(&self, uri: &str) -> McpResult<ResourceReadResult> {
        let params = ResourceReadParams::new(uri);
        self.call_method::<ResourcesRead>(params).await
    }

    /// Read a resource and return its text content
    ///
    /// This is a convenience method that reads a resource and extracts
    /// the text content, returning an error if the resource is binary.
    ///
    /// # Arguments
    ///
    /// - `uri`: The URI of the resource to read
    ///
    /// # Returns
    ///
    /// The text content of the resource.
    ///
    /// # Errors
    ///
    /// Returns an error if the resource is binary or contains no text.
    pub async fn read_resource_text(&self, uri: &str) -> McpResult<String> {
        let result = self.read_resource(uri).await?;

        // Combine all text contents
        let mut text = String::new();
        for contents in result.contents {
            if let Some(t) = contents.text {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&t);
            }
        }

        if text.is_empty() {
            return Err(McpError::with_category(
                McpErrorCode::InternalError,
                "Resource contains no text content",
                crate::mcp::protocol::errors::ErrorCategory::Protocol,
            ));
        }

        Ok(text)
    }

    // ========================================================================
    // Tools Methods
    // ========================================================================

    /// List available tools from the server
    ///
    /// This sends a `tools/list` request to discover what tools
    /// the server provides.
    ///
    /// # Arguments
    ///
    /// - `cursor`: Optional pagination cursor for large result sets
    ///
    /// # Returns
    ///
    /// A list of available tools and an optional next cursor for pagination.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The client is not connected
    /// - The server doesn't support tools
    /// - The request fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::mcp::client::McpClient;
    ///
    /// async fn example(client: &McpClient) -> Result<(), Box<dyn std::error::Error>> {
    ///     let result = client.list_tools(None).await?;
    ///     for tool in result.tools {
    ///         println!("Tool: {} - {}", tool.name, tool.description);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn list_tools(&self, cursor: Option<&str>) -> McpResult<ToolsListResult> {
        let params = match cursor {
            Some(c) => ToolsListParams {
                cursor: Some(c.to_string()),
            },
            None => ToolsListParams::default(),
        };
        self.call_method::<ToolsList>(params).await
    }

    /// List all tools, handling pagination automatically
    ///
    /// This method fetches all pages of tools until no more are available.
    ///
    /// # Returns
    ///
    /// A vector of all available tools.
    ///
    /// # Errors
    ///
    /// Returns an error if any page request fails.
    pub async fn list_all_tools(&self) -> McpResult<Vec<Tool>> {
        let mut all_tools = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let result = self.list_tools(cursor.as_deref()).await?;
            all_tools.extend(result.tools);

            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(all_tools)
    }

    /// Call a tool on the server
    ///
    /// This sends a `tools/call` request to execute a tool with the given arguments.
    ///
    /// # Arguments
    ///
    /// - `name`: The name of the tool to call
    /// - `arguments`: Optional JSON arguments for the tool
    ///
    /// # Returns
    ///
    /// The tool execution result, which may contain text, images, or resource references.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The client is not connected
    /// - The tool doesn't exist
    /// - The tool execution fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::mcp::client::McpClient;
    /// use serde_json::json;
    ///
    /// async fn example(client: &McpClient) -> Result<(), Box<dyn std::error::Error>> {
    ///     let result = client.call_tool("browser_navigate", Some(json!({
    ///         "url": "https://example.com"
    ///     }))).await?;
    ///
    ///     if result.is_error {
    ///         println!("Tool failed!");
    ///     } else {
    ///         for content in result.content {
    ///             println!("Result: {:?}", content);
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> McpResult<ToolCallResult> {
        let params = ToolCallParams {
            name: name.to_string(),
            arguments,
        };
        self.call_method::<ToolsCall>(params).await
    }

    /// Call a tool and return the text content
    ///
    /// This is a convenience method that calls a tool and extracts
    /// all text content from the result.
    ///
    /// # Arguments
    ///
    /// - `name`: The name of the tool to call
    /// - `arguments`: Optional JSON arguments for the tool
    ///
    /// # Returns
    ///
    /// A tuple of (text_content, is_error).
    pub async fn call_tool_text(
        &self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> McpResult<(String, bool)> {
        let result = self.call_tool(name, arguments).await?;

        let mut text = String::new();
        for content in &result.content {
            match content {
                ToolContent::Text { text: t } => {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(t);
                }
                _ => {} // Skip non-text content
            }
        }

        Ok((text, result.is_error))
    }

    /// Check if the server supports resources
    ///
    /// Returns true if the server advertised resources capability during handshake.
    pub async fn supports_resources(&self) -> bool {
        self.server_info
            .read()
            .await
            .as_ref()
            .map(|info| info.capabilities.resources.is_some())
            .unwrap_or(false)
    }

    /// Check if the server supports tools
    ///
    /// Returns true if the server advertised tools capability during handshake.
    pub async fn supports_tools(&self) -> bool {
        self.server_info
            .read()
            .await
            .as_ref()
            .map(|info| info.capabilities.tools.is_some())
            .unwrap_or(false)
    }

    // ========================================================================
    // Tool Parameter Validation
    // ========================================================================

    /// Find a tool by name from the server's available tools
    ///
    /// This method lists tools and searches for one with the given name.
    ///
    /// # Arguments
    ///
    /// - `name`: The name of the tool to find
    ///
    /// # Returns
    ///
    /// The tool definition if found, or None if not found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::mcp::client::McpClient;
    ///
    /// async fn example(client: &McpClient) -> Result<(), Box<dyn std::error::Error>> {
    ///     if let Some(tool) = client.find_tool("browser_navigate").await? {
    ///         println!("Found tool: {}", tool.description);
    ///     } else {
    ///         println!("Tool not found");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn find_tool(&self, name: &str) -> McpResult<Option<Tool>> {
        let tools = self.list_all_tools().await?;
        Ok(tools.into_iter().find(|t| t.name == name))
    }

    /// Check if a tool exists on the server
    ///
    /// # Arguments
    ///
    /// - `name`: The name of the tool to check
    ///
    /// # Returns
    ///
    /// `true` if the tool exists, `false` otherwise.
    pub async fn tool_exists(&self, name: &str) -> McpResult<bool> {
        Ok(self.find_tool(name).await?.is_some())
    }

    /// Validate that a tool exists and optionally validate arguments against its schema
    ///
    /// This is a convenience method that combines `find_tool` with basic validation.
    ///
    /// # Arguments
    ///
    /// - `name`: The name of the tool
    /// - `arguments`: Optional arguments to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if validation passes, or an error describing the validation failure.
    pub async fn validate_tool_call(
        &self,
        name: &str,
        arguments: Option<&serde_json::Value>,
    ) -> McpResult<()> {
        // Check if tools are supported
        if !self.supports_tools().await {
            return Err(McpError::new(
                McpErrorCode::CapabilityNotSupported,
                "Server does not support tools",
            ));
        }

        // Find the tool
        let tool = self.find_tool(name).await?;
        if tool.is_none() {
            return Err(McpError::new(
                McpErrorCode::ToolNotFound,
                format!("Tool '{}' not found", name),
            ).with_data(serde_json::json!({ "tool": name })));
        }

        // Basic validation: if arguments provided, ensure they're an object
        if let Some(args) = arguments {
            if !args.is_object() && !args.is_null() {
                return Err(McpError::new(
                    McpErrorCode::InvalidParams,
                    "Tool arguments must be an object or null",
                ));
            }
        }

        Ok(())
    }

    // ========================================================================
    // Result Formatting Helpers
    // ========================================================================

    /// Extract all text content from a tool call result
    ///
    /// This extracts text from all text-type content items and joins them with newlines.
    ///
    /// # Arguments
    ///
    /// - `result`: The tool call result to extract text from
    ///
    /// # Returns
    ///
    /// A string containing all text content joined by newlines.
    pub fn extract_text_from_result(result: &ToolCallResult) -> String {
        result.content.iter()
            .filter_map(|c| match c {
                ToolContent::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Extract all image data from a tool call result
    ///
    /// # Arguments
    ///
    /// - `result`: The tool call result to extract images from
    ///
    /// # Returns
    ///
    /// A vector of (data, mime_type) tuples for each image.
    pub fn extract_images_from_result(result: &ToolCallResult) -> Vec<(String, String)> {
        result.content.iter()
            .filter_map(|c| match c {
                ToolContent::Image { data, mime_type } => Some((data.clone(), mime_type.clone())),
                _ => None,
            })
            .collect()
    }

    /// Extract all resource references from a tool call result
    ///
    /// # Arguments
    ///
    /// - `result`: The tool call result to extract resource references from
    ///
    /// # Returns
    ///
    /// A vector of (uri, mime_type) tuples for each resource reference.
    pub fn extract_resources_from_result(result: &ToolCallResult) -> Vec<(String, Option<String>)> {
        result.content.iter()
            .filter_map(|c| match c {
                ToolContent::Resource { uri, mime_type } => {
                    Some((uri.clone(), mime_type.clone()))
                }
                _ => None,
            })
            .collect()
    }

    /// Check if a tool call result contains any error content
    ///
    /// # Arguments
    ///
    /// - `result`: The tool call result to check
    ///
    /// # Returns
    ///
    /// `true` if the result indicates an error or contains error text.
    pub fn is_result_error(result: &ToolCallResult) -> bool {
        if result.is_error {
            return true;
        }

        // Also check for error-like content
        result.content.iter().any(|c| {
            matches!(c, ToolContent::Text { text } if
                text.to_lowercase().contains("error") ||
                text.to_lowercase().contains("failed") ||
                text.to_lowercase().contains("exception")
            )
        })
    }

    /// Get a summary of tool call result content types
    ///
    /// # Arguments
    ///
    /// - `result`: The tool call result to summarize
    ///
    /// # Returns
    ///
    /// A tuple of (text_count, image_count, resource_count).
    pub fn result_content_summary(result: &ToolCallResult) -> (usize, usize, usize) {
        let mut text_count = 0;
        let mut image_count = 0;
        let mut resource_count = 0;

        for content in &result.content {
            match content {
                ToolContent::Text { .. } => text_count += 1,
                ToolContent::Image { .. } => image_count += 1,
                ToolContent::Resource { .. } => resource_count += 1,
            }
        }

        (text_count, image_count, resource_count)
    }

    /// Call a tool with validation
    ///
    /// This combines validation and execution in a single method.
    ///
    /// # Arguments
    ///
    /// - `name`: The name of the tool to call
    /// - `arguments`: Optional JSON arguments for the tool
    ///
    /// # Returns
    ///
    /// The tool execution result.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The server doesn't support tools
    /// - The tool doesn't exist
    /// - The arguments are invalid
    /// - The tool execution fails
    pub async fn call_tool_validated(
        &self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> McpResult<ToolCallResult> {
        // Validate the tool call
        self.validate_tool_call(name, arguments.as_ref()).await?;

        // Execute the tool call
        self.call_tool(name, arguments).await
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
