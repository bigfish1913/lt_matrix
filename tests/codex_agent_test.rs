//! Integration tests for the Codex agent backend
//!
//! Verifies that [`CodexAgent`] correctly implements [`AgentBackend`],
//! validates configuration, and integrates properly with [`AgentFactory`].

use ltmatrix::agent::{
    AgentBackend, AgentConfig, AgentError, AgentFactory, CodexAgent, ExecutionConfig, MemorySession,
};

// ── Construction ─────────────────────────────────────────────────────────────

#[test]
fn codex_agent_can_be_created() {
    let agent = CodexAgent::new();
    assert!(agent.is_ok());
}

#[test]
fn codex_agent_default_creation() {
    let agent = CodexAgent::default();
    assert_eq!(agent.agent().name, "codex");
    assert_eq!(agent.agent().command, "codex");
    assert_eq!(agent.agent().model, "o4-mini");
    assert_eq!(agent.agent().timeout, 3600);
}

#[test]
fn codex_agent_backend_name() {
    let agent = CodexAgent::default();
    assert_eq!(agent.backend_name(), "codex");
}

// ── Configuration validation ─────────────────────────────────────────────────

#[tokio::test]
async fn validate_config_correct_name() {
    let agent = CodexAgent::default();
    let config = AgentConfig::builder()
        .name("codex")
        .command("codex")
        .model("o4-mini")
        .timeout_secs(3600)
        .build();

    let result = agent.validate_config(&config).await;
    // Either OK (codex installed) or CommandNotFound (not installed)
    // Must NOT be a ConfigValidation error on the name field.
    if let Err(e) = result {
        assert!(
            !matches!(e, AgentError::ConfigValidation { ref field, .. } if field == "name"),
            "Name validation should pass for 'codex'"
        );
    }
}

#[tokio::test]
async fn validate_config_wrong_name_fails() {
    let agent = CodexAgent::default();
    let config = AgentConfig::builder()
        .name("claude")
        .command("codex")
        .model("o4-mini")
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
    let agent = CodexAgent::default();
    let config = AgentConfig::builder()
        .name("codex")
        .command("codex")
        .model("")
        .timeout_secs(3600)
        .build();

    let result = agent.validate_config(&config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn validate_config_empty_command_fails() {
    let agent = CodexAgent::default();
    let config = AgentConfig::builder()
        .name("codex")
        .command("")
        .model("o4-mini")
        .timeout_secs(3600)
        .build();

    let result = agent.validate_config(&config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn validate_config_zero_timeout_fails() {
    let agent = CodexAgent::default();
    let config = AgentConfig::builder()
        .name("codex")
        .command("codex")
        .model("o4-mini")
        .timeout_secs(0)
        .build();

    let result = agent.validate_config(&config).await;
    assert!(result.is_err());
}

// ── Execution ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn execute_rejects_empty_prompt() {
    let agent = CodexAgent::default().without_verification();
    let config = ExecutionConfig::default();
    let result = agent.execute("", &config).await;
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(msg.contains("empty") || msg.contains("prompt"));
}

#[tokio::test]
async fn execute_rejects_whitespace_only_prompt() {
    let agent = CodexAgent::default().without_verification();
    let config = ExecutionConfig::default();
    let result = agent.execute("   \t\n  ", &config).await;
    assert!(result.is_err());
}

// ── Health check ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_check_returns_ok_variant() {
    // health_check must return Ok(bool), never Err
    let agent = CodexAgent::default();
    let result = agent.health_check().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn is_available_returns_bool() {
    let agent = CodexAgent::default();
    let _available: bool = agent.is_available().await;
}

// ── Session handling ──────────────────────────────────────────────────────────

#[tokio::test]
async fn execute_with_session_rejects_empty_prompt() {
    let agent = CodexAgent::default().without_verification();
    let config = ExecutionConfig::default();
    let session = MemorySession::default();

    let result = agent.execute_with_session("", &config, &session).await;
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(msg.contains("empty") || msg.contains("prompt"));
}

#[tokio::test]
async fn execute_with_session_rejects_whitespace_prompt() {
    let agent = CodexAgent::default().without_verification();
    let config = ExecutionConfig::default();
    let session = MemorySession::default();

    let result = agent.execute_with_session("   ", &config, &session).await;
    assert!(result.is_err());
}

// ── Completion detection ─────────────────────────────────────────────────────

#[test]
fn check_completion_positive_indicators() {
    assert!(CodexAgent::check_completion("Task completed successfully"));
    assert!(CodexAgent::check_completion("Implementation complete."));
    assert!(CodexAgent::check_completion("Done! All steps finished."));
    assert!(CodexAgent::check_completion("Success! The build passed."));
    assert!(CodexAgent::check_completion("finished"));
    assert!(CodexAgent::check_completion("completed"));
}

#[test]
fn check_completion_negative_takes_precedence() {
    // "not done" beats "done"
    assert!(!CodexAgent::check_completion("Not done yet"));
    assert!(!CodexAgent::check_completion("Task not completed"));
    assert!(!CodexAgent::check_completion("Error: something went wrong"));
    assert!(!CodexAgent::check_completion("Build failed!"));
    assert!(!CodexAgent::check_completion("incomplete implementation"));
}

#[test]
fn check_completion_neutral_output_is_false() {
    assert!(!CodexAgent::check_completion("Still working on it"));
    assert!(!CodexAgent::check_completion(""));
}

#[test]
fn check_completion_is_case_insensitive() {
    assert!(CodexAgent::check_completion("DONE"));
    assert!(CodexAgent::check_completion("TASK COMPLETED"));
    assert!(!CodexAgent::check_completion("ERROR"));
    assert!(!CodexAgent::check_completion("FAILED"));
}

// ── Structured data parsing ───────────────────────────────────────────────────

#[test]
fn parse_structured_data_valid_json_block() {
    let output = r#"
Here is the result:

```json
{"tasks": [{"id": "1", "title": "Init"}]}
```

Done.
"#;
    let data = CodexAgent::parse_structured_data(output);
    assert!(data.is_some());
    assert!(data.unwrap().get("tasks").is_some());
}

#[test]
fn parse_structured_data_no_json_block() {
    assert!(CodexAgent::parse_structured_data("No JSON here").is_none());
}

#[test]
fn parse_structured_data_malformed_json() {
    let bad = "```json\n{ not valid json\n```";
    assert!(CodexAgent::parse_structured_data(bad).is_none());
}

#[test]
fn parse_structured_data_nested_json() {
    let output = "```json\n{\"a\":{\"b\":[1,2,3]}}\n```";
    let data = CodexAgent::parse_structured_data(output);
    assert!(data.is_some());
}

// ── Factory integration ───────────────────────────────────────────────────────

#[test]
fn factory_creates_codex_backend() {
    let factory = AgentFactory::new();
    let result = factory.create("codex");
    assert!(result.is_ok());
    let agent = result.unwrap();
    assert_eq!(agent.backend_name(), "codex");
}

#[test]
fn factory_supports_codex() {
    let factory = AgentFactory::new();
    assert!(factory.is_supported("codex"));
    assert!(factory.supported_backends().contains(&"codex"));
}

#[test]
fn factory_create_codex_with_custom_config() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("codex")
        .command("codex")
        .model("o3")
        .timeout_secs(7200)
        .max_retries(5)
        .build();
    let result = factory.create_with_config(config);
    assert!(result.is_ok());
}

#[test]
fn factory_create_codex_with_o3_mini_model() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("codex")
        .command("codex")
        .model("o3-mini")
        .timeout_secs(3600)
        .build();
    let result = factory.create_with_config(config);
    assert!(result.is_ok());
}

#[test]
fn factory_codex_rejects_wrong_name() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("claude")
        .model("o4-mini")
        .command("codex")
        .timeout_secs(3600)
        .build();
    assert!(factory.validate_config("codex", &config).is_err());
}

#[test]
fn factory_codex_validation_passes_with_correct_config() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("codex")
        .command("codex")
        .model("o4-mini")
        .timeout_secs(3600)
        .build();
    assert!(factory.validate_config("codex", &config).is_ok());
}

// ── Agent configuration ───────────────────────────────────────────────────────

#[test]
fn agent_returns_correct_configuration() {
    let agent = CodexAgent::default();
    let agent_info = agent.agent();
    assert_eq!(agent_info.name, "codex");
    assert_eq!(agent_info.command, "codex");
    assert_eq!(agent_info.model, "o4-mini");
    assert_eq!(agent_info.timeout, 3600);
}

#[test]
fn factory_all_four_backends_supported() {
    let factory = AgentFactory::new();
    let backends = factory.supported_backends();
    assert!(backends.contains(&"claude"));
    assert!(backends.contains(&"opencode"));
    assert!(backends.contains(&"kimicode"));
    assert!(backends.contains(&"codex"));
}
