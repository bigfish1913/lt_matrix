// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! MCP protocol message types
//!
//! This module implements the JSON-RPC 2.0 based message types used by MCP.

pub mod errors;
pub mod messages;

// Re-export message types
pub use messages::{
    JsonRpcMessage,
    JsonRpcRequest,
    JsonRpcResponse,
    JsonRpcNotification,
    RequestId,
};
pub use errors::{JsonRpcError, JsonRpcErrorCode};
