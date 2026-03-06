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

// ── Constructor variants ──────────────────────────────────────────────────────

#[test]
fn codex_with_agent_constructor() {
    use ltmatrix::agent::SessionManager;
    use ltmatrix::models::Agent;

    let agent_model = Agent::codex_default();
    let session_manager = SessionManager::default_manager().unwrap_or_else(|_| {
        let tmp = std::env::temp_dir().join("ltmatrix-test-sessions-codex");
        SessionManager::new(&tmp).expect("temp session manager")
    });
    let codex = ltmatrix::agent::CodexAgent::with_agent(agent_model, session_manager);
    assert_eq!(codex.backend_name(), "codex");
    assert_eq!(codex.agent().name, "codex");
    assert_eq!(codex.agent().command, "codex");
    assert_eq!(codex.agent().model, "o4-mini");
}

#[test]
fn codex_without_verification_builder() {
    // without_verification() is a builder method — should not panic
    let _agent = ltmatrix::agent::CodexAgent::default().without_verification();
}

// ── Agent metadata ────────────────────────────────────────────────────────────

#[test]
fn codex_agent_is_not_default_backend() {
    // The default backend is claude, not codex
    let agent = ltmatrix::agent::CodexAgent::default();
    assert!(!agent.agent().is_default);
}

#[test]
fn codex_default_model_is_o4_mini() {
    let agent = ltmatrix::agent::CodexAgent::default();
    assert_eq!(agent.agent().model, "o4-mini");
}

// ── Additional structured-data parsing ───────────────────────────────────────

#[test]
fn parse_structured_data_uses_first_block_only() {
    let output =
        "```json\n{\"first\": true}\n```\n```json\n{\"second\": true}\n```";
    let data = ltmatrix::agent::CodexAgent::parse_structured_data(output);
    assert!(data.is_some());
    assert_eq!(
        data.unwrap().get("first").and_then(|v| v.as_bool()),
        Some(true)
    );
}

#[test]
fn parse_structured_data_empty_json_object() {
    let output = "```json\n{}\n```";
    let data = ltmatrix::agent::CodexAgent::parse_structured_data(output);
    assert!(data.is_some());
}

#[test]
fn parse_structured_data_json_array() {
    let output = "```json\n[1, 2, 3]\n```";
    let data = ltmatrix::agent::CodexAgent::parse_structured_data(output);
    assert!(data.is_some());
    assert!(data.unwrap().is_array());
}

#[test]
fn parse_structured_data_returns_none_for_empty_string() {
    assert!(ltmatrix::agent::CodexAgent::parse_structured_data("").is_none());
}

// ── execute_task signature test ───────────────────────────────────────────────

#[tokio::test]
async fn execute_task_is_callable_and_returns_result() {
    use ltmatrix::models::Task;

    let agent = ltmatrix::agent::CodexAgent::default().without_verification();
    let config = ExecutionConfig::default();
    let task = Task::new("t-1", "Implement feature X", "Add X to the codebase");

    // Without codex installed the call will fail, but it must not panic and
    // must return a Result (not diverge or abort).
    let result = agent.execute_task(&task, "some context", &config).await;
    let _ = result;
}

#[tokio::test]
async fn execute_task_with_empty_context_does_not_panic() {
    use ltmatrix::models::Task;

    let agent = ltmatrix::agent::CodexAgent::default().without_verification();
    let config = ExecutionConfig::default();
    let task = Task::new("t-2", "Write tests", "Write unit tests for module A");

    let result = agent.execute_task(&task, "", &config).await;
    let _ = result;
}

// ── Additional factory tests ──────────────────────────────────────────────────

#[test]
fn factory_codex_is_not_default_backend() {
    let factory = AgentFactory::new();
    let default_agent = factory.create_default().unwrap();
    assert_ne!(default_agent.backend_name(), "codex");
}

#[test]
fn factory_codex_rejects_empty_model() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("codex")
        .command("codex")
        .model("")
        .timeout_secs(3600)
        .build();
    assert!(factory.create_with_config(config).is_err());
}

#[test]
fn factory_codex_rejects_zero_timeout() {
    let factory = AgentFactory::new();
    let config = AgentConfig {
        name: "codex".to_string(),
        command: "codex".to_string(),
        model: "o4-mini".to_string(),
        timeout_secs: 0,
        max_retries: 3,
        enable_session: true,
    };
    assert!(factory.create_with_config(config).is_err());
}

#[test]
fn factory_codex_rejects_opencode_name() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("opencode")
        .command("codex")
        .model("o4-mini")
        .timeout_secs(3600)
        .build();
    assert!(factory.validate_config("codex", &config).is_err());
}

#[test]
fn factory_codex_model_has_correct_default() {
    let factory = AgentFactory::new();
    let agent = factory.create("codex").unwrap();
    assert_eq!(agent.agent().model, "o4-mini");
}

#[test]
fn factory_codex_command_has_correct_default() {
    let factory = AgentFactory::new();
    let agent = factory.create("codex").unwrap();
    assert_eq!(agent.agent().command, "codex");
}

// ── Model variants ────────────────────────────────────────────────────────────

#[test]
fn codex_all_model_variants_accepted_by_factory() {
    let factory = AgentFactory::new();
    let models = ["o4-mini", "o3", "o3-mini"];

    for model in models {
        let config = AgentConfig::builder()
            .name("codex")
            .command("codex")
            .model(model)
            .timeout_secs(3600)
            .build();
        assert!(
            factory.create_with_config(config).is_ok(),
            "model '{}' should be accepted",
            model
        );
    }
}

// ── Additional validate_config tests ─────────────────────────────────────────

#[tokio::test]
async fn validate_config_kimicode_name_rejected() {
    let agent = ltmatrix::agent::CodexAgent::default();
    let config = AgentConfig::builder()
        .name("kimicode")
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
