// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Agent backend implementations
//!
//! This module provides interfaces and implementations for various AI agent backends
//! including Claude, OpenCode, KimiCode, and Codex.

pub mod agent_pool;
pub mod backend;
pub mod claude;
pub mod codex;
pub mod factory;
pub mod kimicode;
pub mod opencode;
pub mod pool;
pub mod session;
pub mod warmup;

pub use agent_pool::{AgentPool, PoolStats};
pub use backend::{
    AgentBackend, AgentConfig, AgentConfigBuilder, AgentError, AgentResponse, AgentSession,
    ExecutionConfig, MemorySession,
};
pub use claude::ClaudeAgent;
pub use codex::CodexAgent;
pub use factory::{AgentFactory, AgentFactoryConfig};
pub use kimicode::KimiCodeAgent;
pub use opencode::OpenCodeAgent;
pub use pool::SessionPool;
pub use session::{SessionData, SessionManager};
pub use warmup::{WarmupExecutor, WarmupResult};
