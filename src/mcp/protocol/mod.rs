// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! MCP protocol message types
//!
//! This module implements the JSON-RPC 2.0 based message types used by MCP.

pub mod errors;
pub mod messages;
pub mod methods;

// Re-export JSON-RPC message types from messages.rs
pub use messages::{
    JsonRpcMessage,
    JsonRpcRequest,
    JsonRpcResponse,
    JsonRpcNotification,
    RequestId,
};

// Re-export error types from errors.rs
pub use errors::{JsonRpcError, JsonRpcErrorCode};

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
