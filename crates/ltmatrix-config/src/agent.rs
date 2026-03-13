// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Agent backend configuration
//!
//! Defines configuration for different agent backends (Claude, OpenCode, KimiCode, Codex)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported agent backends
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentBackend {
    Claude,
    OpenCode,
    KimiCode,
    Codex,
}

impl Default for AgentBackend {
    fn default() -> Self {
        AgentBackend::Claude
    }
}

impl std::fmt::Display for AgentBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentBackend::Claude => write!(f, "claude"),
            AgentBackend::OpenCode => write!(f, "opencode"),
            AgentBackend::KimiCode => write!(f, "kimi-code"),
            AgentBackend::Codex => write!(f, "codex"),
        }
    }
}

/// Configuration for an agent backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Model identifier (e.g., "claude-sonnet-4-6")
    #[serde(default)]
    pub model: String,

    /// Command to invoke the agent (if different from default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Timeout in seconds for agent operations
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Additional arguments to pass to the agent
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,

    /// Environment variables to set for the agent
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,

    /// Maximum retries for failed operations
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Enable verbose logging for this agent
    #[serde(default)]
    pub verbose: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        AgentConfig {
            model: String::new(),
            command: None,
            timeout: default_timeout(),
            args: Vec::new(),
            env: HashMap::new(),
            max_retries: default_max_retries(),
            verbose: false,
        }
    }
}

impl AgentConfig {
    /// Create a new agent configuration with the specified model
    pub fn new(model: impl Into<String>) -> Self {
        AgentConfig {
            model: model.into(),
            ..Default::default()
        }
    }

    /// Set the command for this agent
    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    /// Set the timeout in seconds
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }

    /// Add an argument
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Set an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set max retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Enable verbose logging
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Get the effective command for this agent
    pub fn get_command(&self, backend: &AgentBackend) -> String {
        if let Some(ref cmd) = self.command {
            cmd.clone()
        } else {
            match backend {
                AgentBackend::Claude => "claude".to_string(),
                AgentBackend::OpenCode => "opencode".to_string(),
                AgentBackend::KimiCode => "kimi-code".to_string(),
                AgentBackend::Codex => "codex".to_string(),
            }
        }
    }
}

/// Claude-specific configuration defaults
impl AgentConfig {
    /// Create default Claude agent configuration
    pub fn claude_default() -> Self {
        AgentConfig {
            model: "claude-sonnet-4-6".to_string(),
            timeout: 3600,
            max_retries: 3,
            ..Default::default()
        }
    }

    /// Create Claude Sonnet configuration
    pub fn claude_sonnet() -> Self {
        Self::claude_default()
    }

    /// Create Claude Opus configuration
    pub fn claude_opus() -> Self {
        AgentConfig {
            model: "claude-opus-4-6".to_string(),
            timeout: 7200, // Longer timeout for complex tasks
            max_retries: 3,
            ..Default::default()
        }
    }

    /// Create Claude Haiku configuration
    pub fn claude_haiku() -> Self {
        AgentConfig {
            model: "claude-haiku-4-5".to_string(),
            timeout: 1800, // Shorter timeout for fast operations
            max_retries: 2,
            ..Default::default()
        }
    }
}

/// OpenCode configuration
impl AgentConfig {
    /// Create default OpenCode configuration
    pub fn opencode_default() -> Self {
        AgentConfig {
            model: "gpt-4".to_string(),
            command: Some("opencode".to_string()),
            timeout: 3600,
            max_retries: 3,
            ..Default::default()
        }
    }
}

/// KimiCode configuration
impl AgentConfig {
    /// Create default KimiCode configuration
    pub fn kimicode_default() -> Self {
        AgentConfig {
            model: "moonshot-v1".to_string(),
            command: Some("kimi-code".to_string()),
            timeout: 3600,
            max_retries: 3,
            ..Default::default()
        }
    }
}

/// Codex configuration
impl AgentConfig {
    /// Create default Codex configuration
    pub fn codex_default() -> Self {
        AgentConfig {
            model: "code-davinci-002".to_string(),
            command: Some("codex".to_string()),
            timeout: 3600,
            max_retries: 3,
            ..Default::default()
        }
    }
}

// Helper functions for defaults
fn default_timeout() -> u64 {
    3600 // 1 hour
}

fn default_max_retries() -> u32 {
    3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_backend_default() {
        assert_eq!(AgentBackend::default(), AgentBackend::Claude);
    }

    #[test]
    fn test_agent_backend_display() {
        assert_eq!(AgentBackend::Claude.to_string(), "claude");
        assert_eq!(AgentBackend::OpenCode.to_string(), "opencode");
    }

    #[test]
    fn test_agent_backend_serialization() {
        let backend = AgentBackend::Claude;
        let json = serde_json::to_string(&backend).unwrap();
        assert_eq!(json, "\"claude\"");

        let deserialized: AgentBackend = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, backend);
    }

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert!(config.model.is_empty());
        assert!(config.command.is_none());
        assert_eq!(config.timeout, 3600);
        assert_eq!(config.max_retries, 3);
        assert!(config.args.is_empty());
        assert!(config.env.is_empty());
    }

    #[test]
    fn test_agent_config_builder() {
        let config = AgentConfig::new("claude-sonnet-4-6")
            .with_timeout(1800)
            .with_arg("--verbose")
            .with_env("API_KEY", "test")
            .with_max_retries(5)
            .with_verbose(true);

        assert_eq!(config.model, "claude-sonnet-4-6");
        assert_eq!(config.timeout, 1800);
        assert_eq!(config.args, vec!["--verbose"]);
        assert_eq!(config.env.get("API_KEY"), Some(&"test".to_string()));
        assert_eq!(config.max_retries, 5);
        assert!(config.verbose);
    }

    #[test]
    fn test_agent_config_claude_default() {
        let config = AgentConfig::claude_default();
        assert_eq!(config.model, "claude-sonnet-4-6");
        assert_eq!(config.timeout, 3600);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_agent_config_get_command() {
        let config = AgentConfig::claude_default();
        assert_eq!(config.get_command(&AgentBackend::Claude), "claude");

        let config_with_cmd = AgentConfig::new("test").with_command("my-agent");
        assert_eq!(
            config_with_cmd.get_command(&AgentBackend::Claude),
            "my-agent"
        );
    }

    #[test]
    fn test_agent_config_serialization() {
        let config = AgentConfig::claude_sonnet();
        let json = serde_json::to_string(&config).unwrap();

        let deserialized: AgentConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.model, config.model);
        assert_eq!(deserialized.timeout, config.timeout);
    }
}
