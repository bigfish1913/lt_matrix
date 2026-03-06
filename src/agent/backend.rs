//! Agent backend abstraction
//!
//! This module defines the trait that all agent backends must implement,
//! providing a unified interface for interacting with different AI agents.

use anyhow::Result;
use async_trait::async_trait;

use crate::models::{Agent, Task};

/// Response from an agent execution
#[derive(Debug, Clone)]
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

    /// Execute a task with full context
    async fn execute_task(
        &self,
        task: &Task,
        context: &str,
        config: &ExecutionConfig,
    ) -> Result<AgentResponse>;

    /// Check if the agent is available and properly configured
    async fn health_check(&self) -> Result<bool>;

    /// Get the agent's configuration
    fn agent(&self) -> &Agent;

    /// Get the name of this backend
    fn backend_name(&self) -> &str {
        self.agent().name.as_str()
    }
}
