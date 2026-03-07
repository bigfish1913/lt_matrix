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
pub mod protocol;
pub mod transport;

// Re-export JSON-RPC message types
pub use protocol::{
    JsonRpcMessage,
    JsonRpcRequest,
    JsonRpcResponse,
    JsonRpcNotification,
    RequestId,
    JsonRpcError,
    JsonRpcErrorCode,
};

// Re-export MCP method types
pub use protocol::{
    MCP_PROTOCOL_VERSION,
    ImplementationInfo,
    ClientCapabilities,
    ServerCapabilities,
    RootsCapability,
    PromptsCapability,
    ResourcesCapability,
    ToolsCapability,
    InitializeParams,
    InitializeResult,
    Tool,
    ToolsListParams,
    ToolsListResult,
    ToolCallParams,
    ToolCallResult,
    ToolContent,
    Resource,
    ResourcesListParams,
    ResourcesListResult,
    ResourceReadParams,
    ResourceReadResult,
    ResourceContents,
    Prompt,
    PromptArgument,
    PromptsListParams,
    PromptsListResult,
    PromptsGetParams,
    PromptsGetResult,
    PromptContent,
    PromptMessage,
    Root,
    RootsListParams,
    RootsListResult,
    LogLevel,
    LoggingSetLevelParams,
};

// Re-export type-safe wrappers
pub use protocol::{
    // Core traits
    McpMethod,
    PaginatedMethod,
    McpNotification,
    // Lifecycle methods
    Initialize,
    Ping,
    PingParams,
    PingResult,
    // Tools methods
    ToolsList,
    ToolsCall,
    // Resources methods
    ResourcesList,
    ResourcesRead,
    ResourcesSubscribe,
    ResourcesSubscribeParams,
    ResourcesSubscribeResult,
    ResourcesUnsubscribe,
    ResourcesUnsubscribeParams,
    ResourcesUnsubscribeResult,
    // Prompts methods
    PromptsList,
    PromptsGet,
    // Roots methods
    RootsList,
    // Logging methods
    LoggingSetLevel,
    LoggingSetLevelResult,
    // Completion methods
    CompletionComplete,
    CompletionReference,
    CompletionArgument,
    CompletionCompleteParams,
    CompletionCompleteResult,
    CompletionInfo,
    // Sampling methods
    SamplingCreateMessage,
    SamplingMessage,
    SamplingContent,
    SamplingCreateMessageParams,
    SamplingCreateMessageResult,
    ModelPreferences,
    ModelHint,
    // Notifications
    NotificationsInitialized,
    NotificationsToolsListChanged,
    NotificationsResourcesListChanged,
    NotificationsPromptsListChanged,
    NotificationsRootsListChanged,
    NotificationsProgress,
    ProgressParams,
    NotificationsMessage,
    LogMessageParams,
    // Method registry
    McpMethodKind,
};

// Re-export transport types
pub use transport::{
    // Core types
    Transport,
    TransportConfig,
    TransportType,
    TransportMessage,
    OutgoingMessage,
    TransportError,
    TransportStats,
    // Stdio transport
    StdioTransport,
    StdioConfig,
    ChildProcess,
    // Framing
    MessageFramer,
    FramingError,
    LineDelimitedFramer,
    ContentLengthFramer,
};

// Re-export client types
pub use client::{
    // Core types
    McpClient,
    McpClientConfig,
    ConnectionState,
    ServerInfo,
    StateTransitionError,
};

