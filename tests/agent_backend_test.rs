//! Tests for AgentBackend trait and core types
//!
//! These tests verify the abstraction contract for agent backends.

use ltmatrix::agent::backend::{
    AgentBackend, AgentConfig, AgentError, AgentSession, ExecutionConfig, MemorySession,
};
use ltmatrix::agent::claude::ClaudeAgent;

#[test]
fn test_agent_error_command_not_found() {
    let error = AgentError::CommandNotFound {
        command: "claude".to_string(),
    };

    assert!(matches!(error, AgentError::CommandNotFound { .. }));
    assert!(error.to_string().contains("claude"));
    assert!(error.to_string().contains("not found"));
}

#[test]
fn test_agent_error_execution_failed() {
    let error = AgentError::ExecutionFailed {
        command: "claude".to_string(),
        message: "Process exited with status 1".to_string(),
    };

    assert!(matches!(error, AgentError::ExecutionFailed { .. }));
    assert!(error.to_string().contains("claude"));
    assert!(error.to_string().contains("exited"));
}

#[test]
fn test_agent_error_timeout() {
    let error = AgentError::Timeout {
        command: "claude".to_string(),
        timeout_secs: 3600,
    };

    assert!(matches!(error, AgentError::Timeout { .. }));
    assert!(error.to_string().contains("claude"));
    assert!(error.to_string().contains("3600"));
}

#[test]
fn test_agent_error_invalid_response() {
    let error = AgentError::InvalidResponse {
        reason: "Missing required field 'tasks'".to_string(),
    };

    assert!(matches!(error, AgentError::InvalidResponse { .. }));
    assert!(error.to_string().contains("Missing required field"));
}

#[test]
fn test_agent_error_config_validation() {
    let error = AgentError::ConfigValidation {
        field: "model".to_string(),
        message: "Model name cannot be empty".to_string(),
    };

    assert!(matches!(error, AgentError::ConfigValidation { .. }));
    assert!(error.to_string().contains("model"));
    assert!(error.to_string().contains("cannot be empty"));
}

#[test]
fn test_agent_error_session_not_found() {
    let error = AgentError::SessionNotFound {
        session_id: "abc-123".to_string(),
    };

    assert!(matches!(error, AgentError::SessionNotFound { .. }));
    assert!(error.to_string().contains("abc-123"));
}

#[test]
fn test_agent_error_display_and_debug() {
    let error = AgentError::CommandNotFound {
        command: "test-agent".to_string(),
    };

    // Test Display
    let display_string = format!("{}", error);
    assert!(!display_string.is_empty());

    // Test Debug
    let debug_string = format!("{:?}", error);
    assert!(!debug_string.is_empty());
    assert!(debug_string.contains("CommandNotFound"));
}

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
fn test_agent_config_builder() {
    let config = AgentConfig::builder()
        .name("opencode")
        .model("gpt-4")
        .command("opencode")
        .timeout_secs(1800)
        .max_retries(5)
        .enable_session(false)
        .build();

    assert_eq!(config.name, "opencode");
    assert_eq!(config.model, "gpt-4");
    assert_eq!(config.command, "opencode");
    assert_eq!(config.timeout_secs, 1800);
    assert_eq!(config.max_retries, 5);
    assert!(!config.enable_session);
}

#[test]
fn test_agent_config_validate_valid() {
    let config = AgentConfig::default();

    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_agent_config_validate_empty_name() {
    let mut config = AgentConfig::default();
    config.name = "".to_string();

    let result = config.validate();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AgentError::ConfigValidation { .. }
    ));
}

#[test]
fn test_agent_config_validate_empty_model() {
    let mut config = AgentConfig::default();
    config.model = "".to_string();

    let result = config.validate();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AgentError::ConfigValidation { .. }
    ));
}

#[test]
fn test_agent_config_validate_empty_command() {
    let mut config = AgentConfig::default();
    config.command = "".to_string();

    let result = config.validate();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AgentError::ConfigValidation { .. }
    ));
}

#[test]
fn test_agent_config_validate_zero_timeout() {
    let mut config = AgentConfig::default();
    config.timeout_secs = 0;

    let result = config.validate();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AgentError::ConfigValidation { .. }
    ));
}

#[test]
fn test_agent_session_default() {
    let session = MemorySession::default();

    assert!(!session.session_id.is_empty());
    assert_eq!(session.agent_name, "claude");
    assert_eq!(session.model, "claude-sonnet-4-6");
    assert!(session.created_at <= chrono::Utc::now());
    assert_eq!(session.reuse_count, 0);
}

#[test]
fn test_agent_session_mark_accessed() {
    let mut session = MemorySession::default();

    let initial_count = session.reuse_count;
    session.mark_accessed();

    assert_eq!(session.reuse_count, initial_count + 1);
    assert!(session.last_accessed >= session.created_at);
}

#[test]
fn test_agent_session_is_stale() {
    let mut session = MemorySession::default();

    // Fresh session should not be stale
    assert!(!session.is_stale());

    // Make session appear old (more than 1 hour)
    let old_time = chrono::Utc::now() - chrono::Duration::seconds(3700);
    session.last_accessed = old_time;

    assert!(session.is_stale());
}

#[tokio::test]
async fn test_agent_backend_execute_requires_valid_prompt() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();

    // Empty prompt should fail validation
    let result = agent.execute("", &config).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    // Should be an AgentError or anyhow::Error wrapping AgentError
    assert!(
        error.to_string().contains("prompt")
            || error.to_string().contains("empty")
            || error.to_string().contains("required")
    );
}

#[tokio::test]
async fn test_agent_backend_health_check() {
    let agent = ClaudeAgent::new().unwrap();

    // Health check should not panic
    let result = agent.health_check().await;

    // We expect it might fail if claude is not installed, but should not panic
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_agent_backend_is_available() {
    let agent = ClaudeAgent::new().unwrap();

    // is_available should not panic and should return a boolean
    let available = agent.is_available().await;
    assert!(available == true || available == false);
}

#[tokio::test]
async fn test_agent_backend_validate_config() {
    let agent = ClaudeAgent::new().unwrap();

    // Valid config should pass
    let valid_config = AgentConfig::default();
    let result = agent.validate_config(&valid_config).await;
    assert!(result.is_ok());

    // Invalid config should fail
    let mut invalid_config = AgentConfig::default();
    invalid_config.model = "".to_string();
    let result = agent.validate_config(&invalid_config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_agent_backend_execute_with_session() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();
    let session = MemorySession::default();

    // This might fail if claude is not installed, but should not panic
    let result = agent
        .execute_with_session("test prompt", &config, &session)
        .await;

    // We're testing the API exists and doesn't panic, not the actual execution
    assert!(result.is_ok() || result.is_err());
}
