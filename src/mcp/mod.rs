// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! MCP (Model Context Protocol) implementation
//!
//! This module provides a complete MCP client implementation for communicating
//! with MCP servers (e.g., Playwright, browser automation tools) for end-to-end testing.

pub mod protocol;

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

