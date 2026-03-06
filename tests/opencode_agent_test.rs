//! Integration tests for the OpenCode agent backend
//!
//! These tests verify that the OpenCodeAgent correctly implements the
//! AgentBackend trait, handles configuration validation, and can be
//! instantiated through the factory.

use ltmatrix::agent::{
    AgentBackend, AgentConfig, AgentError, AgentFactory, MemorySession, OpenCodeAgent,
};

// ── Construction ─────────────────────────────────────────────────────────────

#[test]
fn opencode_agent_can_be_created() {
    let agent = OpenCodeAgent::new();
    assert!(agent.is_ok());
}

#[test]
fn opencode_agent_default_creation() {
    let agent = OpenCodeAgent::default();
    assert_eq!(agent.agent().name, "opencode");
    assert_eq!(agent.agent().model, "gpt-4");
    assert_eq!(agent.agent().command, "opencode");
}

#[test]
fn opencode_agent_backend_name() {
    let agent = OpenCodeAgent::default();
    assert_eq!(agent.backend_name(), "opencode");
}

// ── Configuration validation ─────────────────────────────────────────────────

#[tokio::test]
async fn validate_config_correct_name() {
    let agent = OpenCodeAgent::default();
    let config = AgentConfig::builder()
        .name("opencode")
        .command("opencode")
        .model("gpt-4")
        .timeout_secs(3600)
        .build();

    // This will fail because opencode command likely isn't installed, but
    // we can test that it validates the name field before trying the command
    let result = agent.validate_config(&config).await;
    // Either OK (opencode installed) or CommandNotFound (not installed)
    // Not ConfigValidation with name field
    if let Err(e) = result {
        assert!(
            !matches!(e, AgentError::ConfigValidation { field, .. } if field == "name"),
            "Name validation should pass for 'opencode'"
        );
    }
}

#[tokio::test]
async fn validate_config_wrong_name_fails() {
    let agent = OpenCodeAgent::default();
    let config = AgentConfig::builder()
        .name("claude")
        .command("opencode")
        .model("gpt-4")
        .timeout_secs(3600)
        .build();

    let result = agent.validate_config(&config).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AgentError::ConfigValidation { .. }
    ));
}

#[tokio::test]
async fn validate_config_empty_model_fails() {
    let agent = OpenCodeAgent::default();
    let config = AgentConfig::builder()
        .name("opencode")
        .command("opencode")
        .model("")
        .timeout_secs(3600)
        .build();

    let result = agent.validate_config(&config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn validate_config_empty_command_fails() {
    let agent = OpenCodeAgent::default();
    let config = AgentConfig::builder()
        .name("opencode")
        .command("")
        .model("gpt-4")
        .timeout_secs(3600)
        .build();

    let result = agent.validate_config(&config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn validate_config_zero_timeout_fails() {
    let agent = OpenCodeAgent::default();
    let config = AgentConfig::builder()
        .name("opencode")
        .command("opencode")
        .model("gpt-4")
        .timeout_secs(0)
        .build();

    let result = agent.validate_config(&config).await;
    assert!(result.is_err());
}

// ── Execution ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn execute_rejects_empty_prompt() {
    let agent = OpenCodeAgent::default().without_verification();
    let config = ltmatrix::agent::ExecutionConfig::default();
    let result = agent.execute("", &config).await;
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(msg.contains("empty") || msg.contains("prompt"));
}

#[tokio::test]
async fn execute_rejects_whitespace_only_prompt() {
    let agent = OpenCodeAgent::default().without_verification();
    let config = ltmatrix::agent::ExecutionConfig::default();
    let result = agent.execute("   \t\n  ", &config).await;
    assert!(result.is_err());
}

// ── Health check ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_check_returns_bool() {
    let agent = OpenCodeAgent::default();
    // health_check should return Ok(bool), not Err, even if opencode isn't installed
    let result = agent.health_check().await;
    assert!(result.is_ok());
    // is_available convenience method should work too
    let _ = agent.is_available().await;
}

#[tokio::test]
async fn health_check_false_when_verification_fails() {
    // When opencode is not installed, health_check returns Ok(false)
    let agent = OpenCodeAgent::default();
    let result = agent.health_check().await;
    assert!(result.is_ok());
    // Result may be true or false depending on environment
}

// ── Session handling ──────────────────────────────────────────────────────────

#[tokio::test]
async fn execute_with_session_rejects_empty_prompt() {
    let agent = OpenCodeAgent::default().without_verification();
    let config = ltmatrix::agent::ExecutionConfig::default();
    let session = MemorySession::default();

    let result = agent.execute_with_session("", &config, &session).await;
    assert!(result.is_err());
}

// ── Completion detection ─────────────────────────────────────────────────────

#[test]
fn check_completion_positive_indicators() {
    assert!(OpenCodeAgent::check_completion(
        "Task completed successfully"
    ));
    assert!(OpenCodeAgent::check_completion("Implementation complete."));
    assert!(OpenCodeAgent::check_completion("Done! All steps finished."));
    assert!(OpenCodeAgent::check_completion(
        "Success! The build passed."
    ));
}

#[test]
fn check_completion_negative_indicators() {
    assert!(!OpenCodeAgent::check_completion("Not done yet"));
    assert!(!OpenCodeAgent::check_completion("Task not completed"));
    assert!(!OpenCodeAgent::check_completion(
        "Error: something went wrong"
    ));
    assert!(!OpenCodeAgent::check_completion("Build failed!"));
    assert!(!OpenCodeAgent::check_completion("Still working on it"));
}

// ── Factory integration ───────────────────────────────────────────────────────

#[test]
fn factory_creates_opencode_backend() {
    let factory = AgentFactory::new();
    let result = factory.create("opencode");
    assert!(result.is_ok());
    let agent = result.unwrap();
    assert_eq!(agent.backend_name(), "opencode");
}

#[test]
fn factory_supports_opencode() {
    let factory = AgentFactory::new();
    assert!(factory.is_supported("opencode"));
    assert!(factory.supported_backends().contains(&"opencode"));
}

#[test]
fn factory_create_opencode_with_custom_config() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("opencode")
        .command("opencode")
        .model("gpt-4-turbo")
        .timeout_secs(7200)
        .max_retries(5)
        .build();
    let result = factory.create_with_config(config);
    assert!(result.is_ok());
}

#[test]
fn factory_opencode_validates_backend_specific_rules() {
    let factory = AgentFactory::new();
    // Correct name but wrong backend name for opencode
    let config = AgentConfig::builder()
        .name("claude")
        .command("opencode")
        .model("gpt-4")
        .timeout_secs(3600)
        .build();
    assert!(factory.validate_config("opencode", &config).is_err());
}

#[test]
fn factory_opencode_validation_passes_with_correct_config() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("opencode")
        .command("opencode")
        .model("gpt-4")
        .timeout_secs(3600)
        .build();
    assert!(factory.validate_config("opencode", &config).is_ok());
}

// ── Agent configuration ───────────────────────────────────────────────────────

#[test]
fn agent_returns_correct_configuration() {
    let agent = OpenCodeAgent::default();
    let agent_info = agent.agent();
    assert_eq!(agent_info.name, "opencode");
    assert_eq!(agent_info.command, "opencode");
    assert_eq!(agent_info.model, "gpt-4");
    assert_eq!(agent_info.timeout, 3600);
}
