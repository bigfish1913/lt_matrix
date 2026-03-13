// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! MCP (Model Context Protocol) implementation
//!
//! This module provides a complete MCP client implementation for communicating
//! with MCP servers (e.g., Playwright, browser automation tools) for end-to-end testing.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        MCP Module                                │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
//! │  │   Client     │  │   Protocol   │  │      Transport       │  │
//! │  │  (McpClient) │  │  (Messages)  │  │  (Stdio/WebSocket)   │  │
//! │  └──────────────┘  └──────────────┘  └──────────────────────┘  │
//! │         │                 │                    │                │
//! │         └─────────────────┼────────────────────┘                │
//! │                           ▼                                     │
//! │              ┌──────────────────────┐                           │
//! │              │    MCP Server        │                           │
//! │              │  (Playwright, etc.)  │                           │
//! │              └──────────────────────┘                           │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ```no_run
//! use ltmatrix::mcp::client::{McpClient, McpClientConfig, ConnectionState};
//! use ltmatrix::mcp::transport::TransportConfig;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client configuration
//!     let transport_config = TransportConfig::stdio_command("playwright-mcp-server");
//!     let config = McpClientConfig::new("my-app", "1.0.0")
//!         .with_transport(transport_config);
//!
//!     // Create and connect client
//!     let mut client = McpClient::new(config);
//!     client.connect().await?;
//!
//!     // Check connection state
//!     println!("Connected: {}", client.is_connected().await);
//!
//!     // Disconnect
//!     client.disconnect().await?;
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod correlation;
pub mod heartbeat;
pub mod notification;
pub mod protocol;
pub mod reconnect;
pub mod router;
pub mod transport;

// Re-export JSON-RPC message types
pub use protocol::{
    JsonRpcError, JsonRpcErrorCode, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse, RequestId,
};

// Re-export MCP method types
pub use protocol::{
    ClientCapabilities, ImplementationInfo, InitializeParams, InitializeResult, LogLevel,
    LoggingSetLevelParams, Prompt, PromptArgument, PromptContent, PromptMessage, PromptsCapability,
    PromptsGetParams, PromptsGetResult, PromptsListParams, PromptsListResult, Resource,
    ResourceContents, ResourceReadParams, ResourceReadResult, ResourcesCapability,
    ResourcesListParams, ResourcesListResult, Root, RootsCapability, RootsListParams,
    RootsListResult, ServerCapabilities, Tool, ToolCallParams, ToolCallResult, ToolContent,
    ToolsCapability, ToolsListParams, ToolsListResult, MCP_PROTOCOL_VERSION,
};

// Re-export type-safe wrappers
pub use protocol::{
    CompletionArgument,
    // Completion methods
    CompletionComplete,
    CompletionCompleteParams,
    CompletionCompleteResult,
    CompletionInfo,
    CompletionReference,
    // Lifecycle methods
    Initialize,
    LogMessageParams,
    // Logging methods
    LoggingSetLevel,
    LoggingSetLevelResult,
    // Core traits
    McpMethod,
    // Method registry
    McpMethodKind,
    McpNotification,
    ModelHint,
    ModelPreferences,
    // Notifications
    NotificationsInitialized,
    NotificationsMessage,
    NotificationsProgress,
    NotificationsPromptsListChanged,
    NotificationsResourcesListChanged,
    NotificationsRootsListChanged,
    NotificationsToolsListChanged,
    PaginatedMethod,
    Ping,
    PingParams,
    PingResult,
    ProgressParams,
    PromptsGet,
    // Prompts methods
    PromptsList,
    // Resources methods
    ResourcesList,
    ResourcesRead,
    ResourcesSubscribe,
    ResourcesSubscribeParams,
    ResourcesSubscribeResult,
    ResourcesUnsubscribe,
    ResourcesUnsubscribeParams,
    ResourcesUnsubscribeResult,
    // Roots methods
    RootsList,
    SamplingContent,
    // Sampling methods
    SamplingCreateMessage,
    SamplingCreateMessageParams,
    SamplingCreateMessageResult,
    SamplingMessage,
    ToolsCall,
    // Tools methods
    ToolsList,
};

// Re-export transport types
pub use transport::{
    ChildProcess,
    ContentLengthFramer,
    FramingError,
    LineDelimitedFramer,
    // Framing
    MessageFramer,
    OutgoingMessage,
    StdioConfig,
    // Stdio transport
    StdioTransport,
    // Core types
    Transport,
    TransportConfig,
    TransportError,
    TransportMessage,
    TransportStats,
    TransportType,
};

// Re-export client types
pub use client::{
    ConnectionState,
    // Core types
    McpClient,
    McpClientConfig,
    ServerInfo,
    StateTransitionError,
};

// Re-export correlation types
pub use correlation::{
    CorrelationError,
    PendingRequest,
    PendingRequestHandle,
    PendingRequestInfo,
    // Core types
    RequestTracker,
    TrackerStats,
};

// Re-export heartbeat types
pub use heartbeat::{
    ActivityTracker,
    ConnectionHealth,
    HeartbeatConfig,
    HeartbeatHandle,
    // Core types
    HeartbeatManager,
    HeartbeatStats,
    PingSender,
};

// Re-export reconnect types
pub use reconnect::{
    BackoffStrategy,
    DegradationLevel,
    ReconnectConfig,
    ReconnectHandle,
    ReconnectStats,
    // Core types
    ReconnectionManager,
    Reconnector,
    RecoveryConfig,
    RecoveryStrategy,
};

// Re-export router types
pub use router::{
    MessageClassifier,
    MessageKind,
    NotificationHandler,
    RequestBuilder,
    RequestHandler,
    // Core types
    RequestRouter,
    ResponseCorrelator,
    ResponseParser,
    RouterStats,
    TypedResponse,
};

// Re-export notification types
pub use notification::{
    NotificationBuilder, NotificationDispatcher, NotificationEvent, NotificationStats,
};
