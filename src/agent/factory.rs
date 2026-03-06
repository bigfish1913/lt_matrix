//! AgentFactory — create agent backend instances by name
//!
//! The factory is the single entry point for constructing any
//! [`AgentBackend`] implementation.  It hides the concrete types behind the
//! trait object, so the pipeline only needs to hold a `Box<dyn AgentBackend>`.
//!
//! # Supported Backends
//!
//! | Name          | Type               |
//! |---------------|--------------------|
//! | `"claude"`    | [`ClaudeAgent`]    |
//! | `"opencode"`  | [`OpenCodeAgent`]  |
//! | `"kimicode"`  | [`KimiCodeAgent`]  |
//!
//! Additional backends (Codex) will be registered as they are implemented.
//!
//! # Usage
//!
//! ```rust,no_run
//! use ltmatrix::agent::factory::AgentFactory;
//!
//! let factory = AgentFactory::new();
//!
//! // Create with default config for the named backend
//! let agent = factory.create("claude").expect("claude backend");
//!
//! // Create with a fully custom AgentConfig
//! use ltmatrix::agent::backend::AgentConfig;
//! let config = AgentConfig::builder()
//!     .name("claude")
//!     .model("claude-opus-4-6")
//!     .timeout_secs(7200)
//!     .build();
//! let agent = factory.create_with_config(config).expect("custom config");
//! ```

use anyhow::{anyhow, Result};

use crate::agent::backend::{AgentBackend, AgentConfig, AgentError};
use crate::agent::claude::ClaudeAgent;
use crate::agent::kimicode::KimiCodeAgent;
use crate::agent::opencode::OpenCodeAgent;
use crate::agent::session::SessionManager;
use crate::models::Agent;

/// Factory-level configuration (not per-backend).
#[derive(Debug, Clone)]
pub struct AgentFactoryConfig {
    /// The default backend name used by [`AgentFactory::create_default`].
    pub default_backend: String,
}

impl Default for AgentFactoryConfig {
    fn default() -> Self {
        AgentFactoryConfig {
            default_backend: "claude".to_string(),
        }
    }
}

/// Factory for creating agent backend instances.
///
/// Each backend type has built-in per-backend validation rules (in addition to
/// the generic [`AgentConfig::validate`] checks) that are applied before any
/// backend is instantiated.
#[derive(Debug, Clone)]
pub struct AgentFactory {
    factory_config: AgentFactoryConfig,
}

impl Default for AgentFactory {
    fn default() -> Self {
        AgentFactory::new()
    }
}

impl AgentFactory {
    /// Create a new factory with default settings (`claude` as the default backend).
    pub fn new() -> Self {
        AgentFactory {
            factory_config: AgentFactoryConfig::default(),
        }
    }

    /// Create a factory with a custom [`AgentFactoryConfig`].
    pub fn with_factory_config(config: AgentFactoryConfig) -> Self {
        AgentFactory {
            factory_config: config,
        }
    }

    /// Return the names of all backends this factory knows how to create.
    pub fn supported_backends(&self) -> Vec<&'static str> {
        vec!["claude", "opencode", "kimicode"]
    }

    /// Return `true` if `name` is a recognised backend.
    pub fn is_supported(&self, name: &str) -> bool {
        self.supported_backends().contains(&name)
    }

    /// Create a backend using the default [`AgentConfig`] for that backend.
    ///
    /// The `name` is case-sensitive (always lowercase, e.g. `"claude"`).
    pub fn create(&self, name: &str) -> Result<Box<dyn AgentBackend>> {
        let config = self.default_config_for(name)?;
        self.build(name, config)
    }

    /// Create the default backend as specified in [`AgentFactoryConfig`].
    pub fn create_default(&self) -> Result<Box<dyn AgentBackend>> {
        self.create(&self.factory_config.default_backend.clone())
    }

    /// Create a backend from a fully-specified [`AgentConfig`].
    ///
    /// The backend type is inferred from `config.name`.
    pub fn create_with_config(&self, config: AgentConfig) -> Result<Box<dyn AgentBackend>> {
        // Run generic validation first
        config.validate().map_err(|e| anyhow!("{}", e))?;

        let name = config.name.clone();
        self.build(&name, config)
    }

    /// Validate a configuration for a specific named backend.
    ///
    /// Runs both the generic [`AgentConfig::validate`] checks and any
    /// backend-specific rules.  Returns `Ok(())` when the config is valid.
    pub fn validate_config(&self, backend: &str, config: &AgentConfig) -> Result<(), AgentError> {
        // Generic field checks
        config.validate()?;

        // Per-backend checks
        match backend {
            "claude" => self.validate_claude_config(config),
            "opencode" => self.validate_opencode_config(config),
            "kimicode" => self.validate_kimicode_config(config),
            unknown => Err(AgentError::ConfigValidation {
                field: "name".to_string(),
                message: format!("unknown backend '{}'", unknown),
            }),
        }
    }

    // ── private helpers ──────────────────────────────────────────────────────

    /// Return the canonical default [`AgentConfig`] for the named backend.
    fn default_config_for(&self, name: &str) -> Result<AgentConfig> {
        match name {
            "claude" => Ok(AgentConfig::default()),
            "opencode" => Ok(AgentConfig::builder()
                .name("opencode")
                .command("opencode")
                .model("gpt-4")
                .timeout_secs(3600)
                .max_retries(3)
                .enable_session(true)
                .build()),
            "kimicode" => Ok(AgentConfig::builder()
                .name("kimicode")
                .command("kimi-code")
                .model("moonshot-v1-128k")
                .timeout_secs(3600)
                .max_retries(3)
                .enable_session(true)
                .build()),
            unknown => Err(anyhow!(
                "unsupported agent backend '{}'; supported: {:?}",
                unknown,
                self.supported_backends()
            )),
        }
    }

    /// Instantiate the concrete backend type for `name` from `config`.
    fn build(&self, name: &str, config: AgentConfig) -> Result<Box<dyn AgentBackend>> {
        // Per-backend validation before instantiation
        self.validate_config(name, &config)
            .map_err(|e| anyhow!("{}", e))?;

        match name {
            "claude" => {
                // Build an Agent model from the factory config
                let agent = Agent {
                    name: config.name.clone(),
                    command: config.command.clone(),
                    model: config.model.clone(),
                    timeout: config.timeout_secs,
                    is_default: true,
                };

                let session_manager = SessionManager::default_manager().unwrap_or_else(|_| {
                    // Fall back to a temp-dir-based session manager when the
                    // current directory is not writable (common in tests).
                    let tmp = std::env::temp_dir().join("ltmatrix-sessions");
                    SessionManager::new(&tmp).expect("temp-dir session manager creation")
                });

                let backend = ClaudeAgent::with_agent(agent, session_manager);
                Ok(Box::new(backend))
            }
            "opencode" => {
                let agent = Agent {
                    name: config.name.clone(),
                    command: config.command.clone(),
                    model: config.model.clone(),
                    timeout: config.timeout_secs,
                    is_default: false,
                };

                let session_manager = SessionManager::default_manager().unwrap_or_else(|_| {
                    let tmp = std::env::temp_dir().join("ltmatrix-sessions");
                    SessionManager::new(&tmp).expect("temp-dir session manager creation")
                });

                let backend = OpenCodeAgent::with_agent(agent, session_manager);
                Ok(Box::new(backend))
            }
            "kimicode" => {
                let agent = Agent {
                    name: config.name.clone(),
                    command: config.command.clone(),
                    model: config.model.clone(),
                    timeout: config.timeout_secs,
                    is_default: false,
                };

                let session_manager = SessionManager::default_manager().unwrap_or_else(|_| {
                    let tmp = std::env::temp_dir().join("ltmatrix-sessions");
                    SessionManager::new(&tmp).expect("temp-dir session manager creation")
                });

                let backend = KimiCodeAgent::with_agent(agent, session_manager);
                Ok(Box::new(backend))
            }
            unknown => Err(anyhow!("unsupported agent backend '{}'", unknown)),
        }
    }

    /// Claude-specific validation rules.
    fn validate_claude_config(&self, config: &AgentConfig) -> Result<(), AgentError> {
        if config.name != "claude" {
            return Err(AgentError::ConfigValidation {
                field: "name".to_string(),
                message: format!(
                    "claude backend requires name='claude', got '{}'",
                    config.name
                ),
            });
        }
        Ok(())
    }

    /// OpenCode-specific validation rules.
    fn validate_opencode_config(&self, config: &AgentConfig) -> Result<(), AgentError> {
        if config.name != "opencode" {
            return Err(AgentError::ConfigValidation {
                field: "name".to_string(),
                message: format!(
                    "opencode backend requires name='opencode', got '{}'",
                    config.name
                ),
            });
        }
        Ok(())
    }

    /// KimiCode-specific validation rules.
    fn validate_kimicode_config(&self, config: &AgentConfig) -> Result<(), AgentError> {
        if config.name != "kimicode" {
            return Err(AgentError::ConfigValidation {
                field: "name".to_string(),
                message: format!(
                    "kimicode backend requires name='kimicode', got '{}'",
                    config.name
                ),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_factory_creates_claude() {
        let factory = AgentFactory::new();
        assert!(factory.create("claude").is_ok());
    }

    #[test]
    fn unknown_backend_returns_error() {
        let factory = AgentFactory::new();
        let result = factory.create("ghost");
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string().to_lowercase();
        assert!(msg.contains("ghost") || msg.contains("unsupported"));
    }

    #[test]
    fn create_default_uses_claude() {
        let factory = AgentFactory::new();
        assert!(factory.create_default().is_ok());
    }

    #[test]
    fn create_with_valid_config() {
        let factory = AgentFactory::new();
        let config = AgentConfig::builder()
            .name("claude")
            .model("claude-opus-4-6")
            .command("claude")
            .timeout_secs(7200)
            .max_retries(5)
            .build();
        assert!(factory.create_with_config(config).is_ok());
    }

    #[test]
    fn create_with_empty_model_fails() {
        let factory = AgentFactory::new();
        let config = AgentConfig::builder()
            .name("claude")
            .model("")
            .command("claude")
            .timeout_secs(3600)
            .build();
        assert!(factory.create_with_config(config).is_err());
    }

    #[test]
    fn supported_backends_includes_claude() {
        let factory = AgentFactory::new();
        assert!(factory.supported_backends().contains(&"claude"));
    }

    #[test]
    fn is_supported_known_and_unknown() {
        let factory = AgentFactory::new();
        assert!(factory.is_supported("claude"));
        assert!(!factory.is_supported("xyz"));
    }

    #[test]
    fn validate_config_bad_name_for_claude_backend() {
        let factory = AgentFactory::new();
        let config = AgentConfig::builder()
            .name("opencode")
            .model("claude-sonnet-4-6")
            .command("claude")
            .timeout_secs(3600)
            .build();
        assert!(factory.validate_config("claude", &config).is_err());
    }

    #[test]
    fn validate_config_good_claude() {
        let factory = AgentFactory::new();
        assert!(factory
            .validate_config("claude", &AgentConfig::default())
            .is_ok());
    }

    #[test]
    fn supported_backends_includes_opencode() {
        let factory = AgentFactory::new();
        assert!(factory.supported_backends().contains(&"opencode"));
    }

    #[test]
    fn is_supported_opencode() {
        let factory = AgentFactory::new();
        assert!(factory.is_supported("opencode"));
    }

    #[test]
    fn create_opencode_backend() {
        let factory = AgentFactory::new();
        let result = factory.create("opencode");
        assert!(result.is_ok());
        let agent = result.unwrap();
        assert_eq!(agent.backend_name(), "opencode");
    }

    #[test]
    fn create_opencode_with_config() {
        let factory = AgentFactory::new();
        let config = AgentConfig::builder()
            .name("opencode")
            .command("opencode")
            .model("gpt-4-turbo")
            .timeout_secs(3600)
            .max_retries(3)
            .build();
        assert!(factory.create_with_config(config).is_ok());
    }

    #[test]
    fn validate_config_bad_name_for_opencode_backend() {
        let factory = AgentFactory::new();
        let config = AgentConfig::builder()
            .name("claude")
            .model("gpt-4")
            .command("opencode")
            .timeout_secs(3600)
            .build();
        assert!(factory.validate_config("opencode", &config).is_err());
    }

    #[test]
    fn validate_config_good_opencode() {
        let factory = AgentFactory::new();
        let config = AgentConfig::builder()
            .name("opencode")
            .command("opencode")
            .model("gpt-4")
            .timeout_secs(3600)
            .build();
        assert!(factory.validate_config("opencode", &config).is_ok());
    }

    #[test]
    fn custom_factory_config_sets_default() {
        let fc = AgentFactoryConfig {
            default_backend: "claude".to_string(),
        };
        let factory = AgentFactory::with_factory_config(fc);
        assert!(factory.create_default().is_ok());
    }

    #[test]
    fn supported_backends_includes_kimicode() {
        let factory = AgentFactory::new();
        assert!(factory.supported_backends().contains(&"kimicode"));
    }

    #[test]
    fn is_supported_kimicode() {
        let factory = AgentFactory::new();
        assert!(factory.is_supported("kimicode"));
    }

    #[test]
    fn create_kimicode_backend() {
        let factory = AgentFactory::new();
        let result = factory.create("kimicode");
        assert!(result.is_ok());
        let agent = result.unwrap();
        assert_eq!(agent.backend_name(), "kimicode");
    }

    #[test]
    fn create_kimicode_with_custom_config() {
        let factory = AgentFactory::new();
        let config = AgentConfig::builder()
            .name("kimicode")
            .command("kimi-code")
            .model("moonshot-v1-32k")
            .timeout_secs(7200)
            .max_retries(3)
            .build();
        assert!(factory.create_with_config(config).is_ok());
    }

    #[test]
    fn validate_config_bad_name_for_kimicode_backend() {
        let factory = AgentFactory::new();
        let config = AgentConfig::builder()
            .name("claude")
            .model("moonshot-v1-128k")
            .command("kimi-code")
            .timeout_secs(3600)
            .build();
        assert!(factory.validate_config("kimicode", &config).is_err());
    }

    #[test]
    fn validate_config_good_kimicode() {
        let factory = AgentFactory::new();
        let config = AgentConfig::builder()
            .name("kimicode")
            .command("kimi-code")
            .model("moonshot-v1-128k")
            .timeout_secs(3600)
            .build();
        assert!(factory.validate_config("kimicode", &config).is_ok());
    }
}
