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
