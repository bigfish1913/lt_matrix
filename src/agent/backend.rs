// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Agent backend abstraction
//!
//! This module defines the core abstraction contract for all agent backends,
//! providing a unified interface for interacting with different AI agents
//! (Claude, OpenCode, KimiCode, Codex, etc.).
//!
//! # Architecture Overview
//!
//! The agent backend system consists of several key components:
//!
//! - **[`AgentBackend`]**: Core trait that all agent implementations must implement
//! - **[`AgentConfig`]**: Configuration structure for agent backends
//! - **[`AgentError`]**: Comprehensive error types for agent operations
//! - **[`AgentSession`]**: Trait for managing reusable agent sessions
//! - **[`ExecutionConfig`]**: Runtime configuration for individual agent executions
//! - **[`AgentResponse`]**: Standardized response structure from agent executions
//!
//! # Agent Backend Contract
//!
//! All agent backends must implement the [`AgentBackend`] trait, which requires:
//!
//! ## Core Methods
//!
//! - **[`execute()`]**: Execute a prompt with the agent
//! - **[`execute_with_session()`]**: Execute with session reuse support
//! - **[`execute_task()`]**: Execute a task with full context
//! - **[`health_check()`]**: Check if the agent is available
//! - **[`is_available()`]**: Convenience method that returns boolean directly
//! - **[`validate_config()`]**: Validate agent-specific configuration
//! - **[`agent()`]**: Get the agent's configuration
//! - **[`backend_name()`]**: Get the backend name (has default implementation)
//!
//! ## Error Handling
//!
//! Agent backends must use [`AgentError`] for all error conditions:
//!
//! - [`CommandNotFound`]: Agent CLI command not available
//! - [`ExecutionFailed`]: Agent process failed during execution
//! - [`Timeout`]: Agent execution exceeded timeout limit
//! - [`InvalidResponse`]: Agent response couldn't be parsed
//! - [`ConfigValidation`]: Configuration validation failed
//! - [`SessionNotFound`]: Requested session doesn't exist
//!
//! ## Session Management
//!
//! Sessions allow agent processes and context to be reused across multiple
//! executions, improving performance and maintaining conversational context.
//!
//! ### Session Lifecycle
//!
//! 1. **Creation**: Backend creates a session via [`AgentSession::default()`]
//! 2. **Usage**: Session passed to [`execute_with_session()`]
//! 3. **Access Tracking**: Session marked as accessed via [`mark_accessed()`]
//! 4. **Stale Detection**: Sessions older than 1 hour are considered stale
//!
//! ### MemorySession
//!
//! The [`MemorySession`] type provides a simple in-memory implementation
//! of [`AgentSession`] for testing and simple use cases.
//!
//! # Example Implementation
//!
//! ```rust
//! # use async_trait::async_trait;
//! # use ltmatrix::agent::backend::{
//! #     AgentBackend, AgentConfig, AgentError, AgentResponse, ExecutionConfig, AgentSession
//! # };
//! # use ltmatrix::models::Agent;
//! #
//! struct MyAgent {
//!     agent_config: Agent,
//! }
//!
//! #[async_trait]
//! impl AgentBackend for MyAgent {
//!     async fn execute(&self, prompt: &str, config: &ExecutionConfig) -> anyhow::Result<AgentResponse> {
//!         // Implementation here
//! #       Ok(AgentResponse {
//! #           output: "Response".to_string(),
//! #           structured_data: None,
//! #           is_complete: true,
//! #           error: None,
//! #       })
//!     }
//!
//!     async fn execute_with_session(
//!         &self,
//!         prompt: &str,
//!         config: &ExecutionConfig,
//!         session: &dyn AgentSession,
//!     ) -> anyhow::Result<AgentResponse> {
//!         // Use session for context reuse
//! #       Ok(AgentResponse {
//! #           output: format!("Session: {}", session.session_id()),
//! #           structured_data: None,
//! #           is_complete: true,
//! #           error: None,
//! #       })
//!     }
//!
//!     async fn execute_task(
//!         &self,
//!         task: &ltmatrix::models::Task,
//!         context: &str,
//!         config: &ExecutionConfig,
//!     ) -> anyhow::Result<AgentResponse> {
//!         let prompt = format!("Task: {}\nContext: {}", task.title, context);
//! #       Ok(AgentResponse::default())
//!     }
//!
//!     async fn health_check(&self) -> anyhow::Result<bool> {
//!         // Check if agent command is available
//! #       Ok(true)
//!     }
//!
//!     async fn validate_config(&self, config: &AgentConfig) -> Result<(), AgentError> {
//!         config.validate()
//!     }
//!
//!     fn agent(&self) -> &Agent {
//!         // Return agent configuration
//! #       &self.agent_config
//!     }
//! }
//! ```
//!
//! # Configuration
//!
//! ## AgentConfig
//!
//! [`AgentConfig`] defines the static configuration for an agent backend:
//!
//! ```rust
//! # use ltmatrix::agent::backend::AgentConfig;
//! let config = AgentConfig::builder()
//!     .name("claude")
//!     .model("claude-sonnet-4-6")
//!     .command("claude")
//!     .timeout_secs(3600)
//!     .max_retries(3)
//!     .enable_session(true)
//!     .build();
//!
//! // Validate configuration
//! let _ = config.validate();
//! ```
//!
//! ## ExecutionConfig
//!
//! [`ExecutionConfig`] defines runtime configuration for individual executions:
//!
//! ```rust
//! # use ltmatrix::agent::backend::ExecutionConfig;
//! let exec_config = ExecutionConfig {
//!     model: "claude-opus-4-6".to_string(), // Override default model
//!     max_retries: 5,                        // More retries for complex tasks
//!     timeout: 7200,                         // Longer timeout
//!     enable_session: true,
//!     env_vars: vec![],
//! };
//! # assert_eq!(exec_config.model, "claude-opus-4-6");
//! ```
//!
//! # Error Handling Best Practices
//!
//! 1. **Use Specific Errors**: Return the most specific [`AgentError`] variant
//! 2. **Include Context**: Error messages should include relevant details
//! 3. **Propagate Appropriately**: Convert backend-specific errors to [`AgentError`]
//! 4. **Document Failures**: Clearly document what causes each error variant
//!
//! # Testing Agent Backends
//!
//! When testing agent backends:
//!
//! 1. **Test Error Cases**: Verify all error variants are returned correctly
//! 2. **Test Validation**: Ensure config validation catches invalid inputs
//! 3. **Test Session Behavior**: Verify session reuse and staleness detection
//! 4. **Mock When Needed**: Use mock agents for testing pipeline integration
//!
//! [`CommandNotFound`]: AgentError::CommandNotFound
//! [`ExecutionFailed`]: AgentError::ExecutionFailed
//! [`Timeout`]: AgentError::Timeout
//! [`InvalidResponse`]: AgentError::InvalidResponse
//! [`ConfigValidation`]: AgentError::ConfigValidation
//! [`SessionNotFound`]: AgentError::SessionNotFound

use anyhow::Result;
use async_trait::async_trait;
use std::fmt;

use crate::models::{Agent, Task};

/// Errors that can occur during agent backend operations
#[derive(Debug, Clone)]
pub enum AgentError {
    /// Agent command not found in PATH
    CommandNotFound { command: String },

    /// Agent execution failed
    ExecutionFailed { command: String, message: String },

    /// Agent execution timed out
    Timeout { command: String, timeout_secs: u64 },

    /// Agent response was invalid or couldn't be parsed
    InvalidResponse { reason: String },

    /// Configuration validation failed
    ConfigValidation { field: String, message: String },

    /// Session not found
    SessionNotFound { session_id: String },
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::CommandNotFound { command } => {
                write!(f, "Agent command '{}' not found", command)
            }
            AgentError::ExecutionFailed { command, message } => {
                write!(f, "Agent '{}' execution failed: {}", command, message)
            }
            AgentError::Timeout {
                command,
                timeout_secs,
            } => {
                write!(
                    f,
                    "Agent '{}' timed out after {} seconds",
                    command, timeout_secs
                )
            }
            AgentError::InvalidResponse { reason } => {
                write!(f, "Invalid agent response: {}", reason)
            }
            AgentError::ConfigValidation { field, message } => {
                write!(f, "Config validation failed for '{}': {}", field, message)
            }
            AgentError::SessionNotFound { session_id } => {
                write!(f, "Session '{}' not found", session_id)
            }
        }
    }
}

impl std::error::Error for AgentError {}

/// Configuration for an agent backend
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Agent name (e.g., "claude", "opencode")
    pub name: String,

    /// Model to use (e.g., "claude-sonnet-4-6", "gpt-4")
    pub model: String,

    /// Command to execute (e.g., "claude", "opencode")
    pub command: String,

    /// Timeout in seconds
    pub timeout_secs: u64,

    /// Maximum number of retries
    pub max_retries: u32,

    /// Whether to enable session reuse
    pub enable_session: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        AgentConfig {
            name: "claude".to_string(),
            model: "claude-sonnet-4-6".to_string(),
            command: "claude".to_string(),
            timeout_secs: 3600,
            max_retries: 3,
            enable_session: true,
        }
    }
}

impl AgentConfig {
    /// Create a builder for AgentConfig
    pub fn builder() -> AgentConfigBuilder {
        AgentConfigBuilder::default()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), AgentError> {
        if self.name.trim().is_empty() {
            return Err(AgentError::ConfigValidation {
                field: "name".to_string(),
                message: "Agent name cannot be empty".to_string(),
            });
        }

        if self.model.trim().is_empty() {
            return Err(AgentError::ConfigValidation {
                field: "model".to_string(),
                message: "Model name cannot be empty".to_string(),
            });
        }

        if self.command.trim().is_empty() {
            return Err(AgentError::ConfigValidation {
                field: "command".to_string(),
                message: "Command cannot be empty".to_string(),
            });
        }

        if self.timeout_secs == 0 {
            return Err(AgentError::ConfigValidation {
                field: "timeout_secs".to_string(),
                message: "Timeout must be greater than 0".to_string(),
            });
        }

        Ok(())
    }
}

/// Builder for AgentConfig
#[derive(Debug, Default)]
pub struct AgentConfigBuilder {
    config: AgentConfig,
}

impl AgentConfigBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
    }

    pub fn command(mut self, command: impl Into<String>) -> Self {
        self.config.command = command.into();
        self
    }

    pub fn timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.config.timeout_secs = timeout_secs;
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.config.max_retries = max_retries;
        self
    }

    pub fn enable_session(mut self, enable_session: bool) -> Self {
        self.config.enable_session = enable_session;
        self
    }

    pub fn build(self) -> AgentConfig {
        self.config
    }
}

/// Session trait for agent session management
///
/// This trait defines the interface for managing agent sessions across different backends.
/// Sessions allow reuse of agent processes and context across multiple executions.
pub trait AgentSession: Send + Sync {
    /// Get the unique session identifier
    fn session_id(&self) -> &str;

    /// Get the agent name
    fn agent_name(&self) -> &str;

    /// Get the model being used
    fn model(&self) -> &str;

    /// Get the creation timestamp
    fn created_at(&self) -> chrono::DateTime<chrono::Utc>;

    /// Get the last accessed timestamp
    fn last_accessed(&self) -> chrono::DateTime<chrono::Utc>;

    /// Get the reuse count
    fn reuse_count(&self) -> u32;

    /// Mark the session as accessed
    fn mark_accessed(&mut self);

    /// Check if the session is stale (older than 1 hour)
    fn is_stale(&self) -> bool;
}

/// Simple in-memory session implementation
#[derive(Debug, Clone)]
pub struct MemorySession {
    /// Unique session identifier
    pub session_id: String,

    /// Agent backend name
    pub agent_name: String,

    /// Model being used
    pub model: String,

    /// Session creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last access timestamp
    pub last_accessed: chrono::DateTime<chrono::Utc>,

    /// Number of times this session has been reused
    pub reuse_count: u32,
}

impl Default for MemorySession {
    fn default() -> Self {
        let now = chrono::Utc::now();
        MemorySession {
            session_id: uuid::Uuid::new_v4().to_string(),
            agent_name: "claude".to_string(),
            model: "claude-sonnet-4-6".to_string(),
            created_at: now,
            last_accessed: now,
            reuse_count: 0,
        }
    }
}

impl AgentSession for MemorySession {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn agent_name(&self) -> &str {
        &self.agent_name
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.created_at
    }

    fn last_accessed(&self) -> chrono::DateTime<chrono::Utc> {
        self.last_accessed
    }

    fn reuse_count(&self) -> u32 {
        self.reuse_count
    }

    fn mark_accessed(&mut self) {
        self.last_accessed = chrono::Utc::now();
        self.reuse_count += 1;
    }

    fn is_stale(&self) -> bool {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(self.last_accessed);
        duration.num_seconds() > 3600 // 1 hour
    }
}

/// Response from an agent execution
#[derive(Debug, Clone, Default)]
pub struct AgentResponse {
    /// The raw output from the agent
    pub output: String,

    /// Any structured data extracted from the response
    pub structured_data: Option<serde_json::Value>,

    /// Whether the agent considers the task complete
    pub is_complete: bool,

    /// Error message if the agent failed
    pub error: Option<String>,
}

/// Configuration for agent execution
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    /// Model to use for this execution
    pub model: String,

    /// Maximum number of retries
    pub max_retries: u32,

    /// Timeout in seconds
    pub timeout: u64,

    /// Whether to enable session reuse
    pub enable_session: bool,

    /// Additional environment variables
    pub env_vars: Vec<(String, String)>,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        ExecutionConfig {
            model: "claude-sonnet-4-6".to_string(),
            max_retries: 3,
            timeout: 3600,
            enable_session: true,
            env_vars: Vec::new(),
        }
    }
}

/// Trait that all agent backends must implement
#[async_trait]
pub trait AgentBackend: Send + Sync {
    /// Execute a prompt with the given configuration
    async fn execute(&self, prompt: &str, config: &ExecutionConfig) -> Result<AgentResponse>;

    /// Execute a prompt with session support
    async fn execute_with_session(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
        session: &dyn AgentSession,
    ) -> Result<AgentResponse>;

    /// Execute a task with full context
    async fn execute_task(
        &self,
        task: &Task,
        context: &str,
        config: &ExecutionConfig,
    ) -> Result<AgentResponse>;

    /// Check if the agent is available and properly configured
    async fn health_check(&self) -> Result<bool>;

    /// Check if the agent is available (convenience method that returns bool directly)
    async fn is_available(&self) -> bool {
        self.health_check().await.unwrap_or(false)
    }

    /// Validate agent configuration
    async fn validate_config(&self, config: &AgentConfig) -> Result<(), AgentError>;

    /// Get the agent's configuration
    fn agent(&self) -> &Agent;

    /// Get the name of this backend
    fn backend_name(&self) -> &str {
        self.agent().name.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // AgentError Tests
    // =========================================================================

    #[test]
    fn test_agent_error_command_not_found() {
        let error = AgentError::CommandNotFound {
            command: "claude".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Agent command 'claude' not found"
        );
    }

    #[test]
    fn test_agent_error_execution_failed() {
        let error = AgentError::ExecutionFailed {
            command: "opencode".to_string(),
            message: "Process exited with code 1".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Agent 'opencode' execution failed: Process exited with code 1"
        );
    }

    #[test]
    fn test_agent_error_timeout() {
        let error = AgentError::Timeout {
            command: "kimi-code".to_string(),
            timeout_secs: 3600,
        };
        assert_eq!(
            error.to_string(),
            "Agent 'kimi-code' timed out after 3600 seconds"
        );
    }

    #[test]
    fn test_agent_error_invalid_response() {
        let error = AgentError::InvalidResponse {
            reason: "JSON parse error".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Invalid agent response: JSON parse error"
        );
    }

    #[test]
    fn test_agent_error_config_validation() {
        let error = AgentError::ConfigValidation {
            field: "timeout_secs".to_string(),
            message: "Timeout must be greater than 0".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Config validation failed for 'timeout_secs': Timeout must be greater than 0"
        );
    }

    #[test]
    fn test_agent_error_session_not_found() {
        let error = AgentError::SessionNotFound {
            session_id: "abc-123".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Session 'abc-123' not found"
        );
    }

    #[test]
    fn test_agent_error_is_std_error() {
        let error = AgentError::CommandNotFound {
            command: "test".to_string(),
        };
        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &error;
    }

    // =========================================================================
    // AgentConfig Tests
    // =========================================================================

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.name, "claude");
        assert_eq!(config.model, "claude-sonnet-4-6");
        assert_eq!(config.command, "claude");
        assert_eq!(config.timeout_secs, 3600);
        assert_eq!(config.max_retries, 3);
        assert!(config.enable_session);
    }

    #[test]
    fn test_agent_config_validate_valid() {
        let config = AgentConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_agent_config_validate_empty_name() {
        let config = AgentConfig {
            name: "".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        if let Err(AgentError::ConfigValidation { field, .. }) = result {
            assert_eq!(field, "name");
        } else {
            panic!("Expected ConfigValidation error");
        }
    }

    #[test]
    fn test_agent_config_validate_whitespace_name() {
        let config = AgentConfig {
            name: "   ".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_config_validate_empty_model() {
        let config = AgentConfig {
            model: "".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        if let Err(AgentError::ConfigValidation { field, .. }) = result {
            assert_eq!(field, "model");
        } else {
            panic!("Expected ConfigValidation error");
        }
    }

    #[test]
    fn test_agent_config_validate_empty_command() {
        let config = AgentConfig {
            command: "".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        if let Err(AgentError::ConfigValidation { field, .. }) = result {
            assert_eq!(field, "command");
        } else {
            panic!("Expected ConfigValidation error");
        }
    }

    #[test]
    fn test_agent_config_validate_zero_timeout() {
        let config = AgentConfig {
            timeout_secs: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        if let Err(AgentError::ConfigValidation { field, .. }) = result {
            assert_eq!(field, "timeout_secs");
        } else {
            panic!("Expected ConfigValidation error");
        }
    }

    // =========================================================================
    // AgentConfigBuilder Tests
    // =========================================================================

    #[test]
    fn test_agent_config_builder_default() {
        let config = AgentConfig::builder().build();
        assert_eq!(config.name, "claude");
    }

    #[test]
    fn test_agent_config_builder_custom() {
        let config = AgentConfig::builder()
            .name("opencode")
            .model("gpt-4")
            .command("opencode")
            .timeout_secs(7200)
            .max_retries(5)
            .enable_session(false)
            .build();

        assert_eq!(config.name, "opencode");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.command, "opencode");
        assert_eq!(config.timeout_secs, 7200);
        assert_eq!(config.max_retries, 5);
        assert!(!config.enable_session);
    }

    #[test]
    fn test_agent_config_builder_chaining() {
        let config = AgentConfig::builder()
            .name("a")
            .model("b")
            .command("c")
            .timeout_secs(1)
            .max_retries(0)
            .enable_session(true)
            .build();

        assert_eq!(config.name, "a");
        assert_eq!(config.model, "b");
        assert_eq!(config.command, "c");
        assert_eq!(config.timeout_secs, 1);
        assert_eq!(config.max_retries, 0);
        assert!(config.enable_session);
    }

    // =========================================================================
    // MemorySession Tests
    // =========================================================================

    #[test]
    fn test_memory_session_default() {
        let session = MemorySession::default();
        assert!(!session.session_id.is_empty());
        assert_eq!(session.agent_name, "claude");
        assert_eq!(session.model, "claude-sonnet-4-6");
        assert_eq!(session.reuse_count, 0);
    }

    #[test]
    fn test_memory_session_mark_accessed() {
        let mut session = MemorySession::default();
        assert_eq!(session.reuse_count, 0);

        session.mark_accessed();
        assert_eq!(session.reuse_count, 1);

        session.mark_accessed();
        assert_eq!(session.reuse_count, 2);
    }

    #[test]
    fn test_memory_session_not_stale_initially() {
        let session = MemorySession::default();
        assert!(!session.is_stale());
    }

    #[test]
    fn test_memory_session_is_stale_after_time() {
        let session = MemorySession {
            session_id: "test".to_string(),
            agent_name: "claude".to_string(),
            model: "claude-sonnet-4-6".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::hours(2),
            last_accessed: chrono::Utc::now() - chrono::Duration::hours(2),
            reuse_count: 0,
        };
        assert!(session.is_stale());
    }

    #[test]
    fn test_memory_session_trait_implementation() {
        let mut session = MemorySession::default();

        // Test trait methods
        let _ = session.session_id();
        let _ = session.agent_name();
        let _ = session.model();
        let _ = session.created_at();
        let _ = session.last_accessed();
        let _ = session.reuse_count();
        session.mark_accessed();
        let _ = session.is_stale();
    }

    // =========================================================================
    // AgentResponse Tests
    // =========================================================================

    #[test]
    fn test_agent_response_default() {
        let response = AgentResponse::default();
        assert!(response.output.is_empty());
        assert!(response.structured_data.is_none());
        assert!(!response.is_complete);
        assert!(response.error.is_none());
    }

    #[test]
    fn test_agent_response_with_output() {
        let response = AgentResponse {
            output: "Task completed".to_string(),
            structured_data: Some(serde_json::json!({"key": "value"})),
            is_complete: true,
            error: None,
        };
        assert_eq!(response.output, "Task completed");
        assert!(response.structured_data.is_some());
        assert!(response.is_complete);
    }

    #[test]
    fn test_agent_response_with_error() {
        let response = AgentResponse {
            output: String::new(),
            structured_data: None,
            is_complete: false,
            error: Some("Execution failed".to_string()),
        };
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap(), "Execution failed");
    }

    // =========================================================================
    // ExecutionConfig Tests
    // =========================================================================

    #[test]
    fn test_execution_config_default() {
        let config = ExecutionConfig::default();
        assert_eq!(config.model, "claude-sonnet-4-6");
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout, 3600);
        assert!(config.enable_session);
        assert!(config.env_vars.is_empty());
    }

    #[test]
    fn test_execution_config_custom() {
        let config = ExecutionConfig {
            model: "claude-opus-4-6".to_string(),
            max_retries: 5,
            timeout: 7200,
            enable_session: false,
            env_vars: vec![("KEY".to_string(), "VALUE".to_string())],
        };
        assert_eq!(config.model, "claude-opus-4-6");
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.timeout, 7200);
        assert!(!config.enable_session);
        assert_eq!(config.env_vars.len(), 1);
    }

    #[test]
    fn test_execution_config_clone() {
        let config = ExecutionConfig::default();
        let cloned = config.clone();
        assert_eq!(config.model, cloned.model);
        assert_eq!(config.timeout, cloned.timeout);
    }
}
