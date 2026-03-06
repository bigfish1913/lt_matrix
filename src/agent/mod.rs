//! Agent backend implementations
//!
//! This module provides interfaces and implementations for various AI agent backends
//! including Claude, OpenCode, KimiCode, and Codex.

pub mod backend;
pub mod claude;
pub mod pool;
pub mod session;

pub use backend::{Agent, AgentBackend, AgentError};
pub use claude::ClaudeAgent;
pub use pool::AgentPool;
pub use session::Session;
