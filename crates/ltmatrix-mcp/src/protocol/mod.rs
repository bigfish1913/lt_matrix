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
    JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, RequestId,
};

// Re-export error types from errors.rs
pub use errors::{
    ErrorBuilder, ErrorCategory, JsonRpcError, JsonRpcErrorCode, JsonRpcResult, McpError,
    McpErrorCode, McpResult,
};

// Re-export MCP method types from methods.rs
pub use methods::{
    ClientCapabilities, ImplementationInfo, InitializeParams, InitializeResult, LogLevel,
    LoggingSetLevelParams, Prompt, PromptArgument, PromptContent, PromptMessage, PromptsCapability,
    PromptsGetParams, PromptsGetResult, PromptsListParams, PromptsListResult, Resource,
    ResourceContents, ResourceReadParams, ResourceReadResult, ResourcesCapability,
    ResourcesListParams, ResourcesListResult, Root, RootsCapability, RootsListParams,
    RootsListResult, ServerCapabilities, Tool, ToolCallParams, ToolCallResult, ToolContent,
    ToolsCapability, ToolsListParams, ToolsListResult, MCP_PROTOCOL_VERSION,
};

// Re-export type-safe wrappers from wrappers.rs
pub use wrappers::{
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
