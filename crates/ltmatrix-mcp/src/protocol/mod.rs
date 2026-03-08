// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! MCP protocol message types
//!
//! This module implements the JSON-RPC 2.0 based message types used by MCP.

pub mod errors;
pub mod messages;
pub mod methods;
pub mod wrappers;

// Re-export JSON-RPC message types from messages.rs
pub use messages::{
    JsonRpcMessage,
    JsonRpcRequest,
    JsonRpcResponse,
    JsonRpcNotification,
    RequestId,
};

// Re-export error types from errors.rs
pub use errors::{
    JsonRpcError,
    JsonRpcErrorCode,
    McpError,
    McpErrorCode,
    ErrorCategory,
    ErrorBuilder,
    McpResult,
    JsonRpcResult,
};

// Re-export MCP method types from methods.rs
pub use methods::{
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

// Re-export type-safe wrappers from wrappers.rs
pub use wrappers::{
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
