//! Agent backend plugin trait
//!
//! This module defines the trait for custom agent backends.

use anyhow::Result;
use async_trait::async_trait;

use super::Plugin;
use crate::models::Task;

/// Trait for agent backend plugins
#[async_trait]
pub trait AgentBackendPlugin: Plugin {
    /// Execute a prompt with the agent
    async fn execute(&self, prompt: &str, task: Option<&Task>) -> Result<String>;

    /// Check if the agent is available
    fn is_available(&self) -> bool;

    /// Get the agent's model identifier
    fn model(&self) -> &str;
}

#[cfg(test)]
mod tests {
    // Tests would go here
}