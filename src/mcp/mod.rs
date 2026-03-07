// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! MCP (Model Context Protocol) implementation
//!
//! This module provides a complete MCP client implementation for communicating
//! with MCP servers (e.g., Playwright, browser automation tools) for end-to-end testing.

pub mod protocol;

// Re-export commonly used types
pub use protocol::{JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, JsonRpcNotification};
