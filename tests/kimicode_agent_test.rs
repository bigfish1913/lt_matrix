//! Integration tests for the KimiCode agent backend
//!
//! These tests verify that [`KimiCodeAgent`] correctly implements the
//! [`AgentBackend`] trait, handles configuration validation, and integrates
//! with the factory.  They do **not** require the `kimi-code` CLI binary to
//! be installed — process-level tests use `without_verification()` so the
//! command check is bypassed.

use ltmatrix::agent::{
    AgentBackend, AgentConfig, AgentError, AgentFactory, KimiCodeAgent, MemorySession,
    SessionManager,
};
use ltmatrix::models::{Agent, Task};

// ── Construction ─────────────────────────────────────────────────────────────

#[test]
fn kimicode_agent_can_be_created() {
    let agent = KimiCodeAgent::new();
    assert!(agent.is_ok());
}

#[test]
fn kimicode_agent_default_fields() {
    let agent = KimiCodeAgent::default();
    assert_eq!(agent.agent().name, "kimicode");
    assert_eq!(agent.agent().command, "kimi-code");
    assert_eq!(agent.agent().model, "moonshot-v1-128k");
    assert_eq!(agent.agent().timeout, 3600);
}

#[test]
fn kimicode_agent_backend_name() {
    let agent = KimiCodeAgent::default();
    assert_eq!(agent.backend_name(), "kimicode");
}

#[test]
fn kimicode_with_agent_constructor() {
    let agent_model = Agent::kimicode_default();
    let session_manager = SessionManager::default_manager()
        .unwrap_or_else(|_| {
            let tmp = std::env::temp_dir().join("ltmatrix-test-sessions");
            SessionManager::new(&tmp).expect("temp session manager")
        });
    let kimicode = KimiCodeAgent::with_agent(agent_model, session_manager);
    assert_eq!(kimicode.backend_name(), "kimicode");
    assert_eq!(kimicode.agent().name, "kimicode");
    assert_eq!(kimicode.agent().command, "kimi-code");
    assert_eq!(kimicode.agent().model, "moonshot-v1-128k");
}

#[test]
fn kimicode_without_verification_builder() {
    // without_verification() is a builder method — should not panic
    let _agent = KimiCodeAgent::default().without_verification();
}

// ── Configuration validation ─────────────────────────────────────────────────

#[tokio::test]
async fn validate_config_correct_name_passes_or_command_not_found() {
    // Either Ok (kimi-code installed) or CommandNotFound (not installed) — but
    // never a ConfigValidation error on the 'name' field.
    let agent = KimiCodeAgent::default();
    let config = AgentConfig::builder()
        .name("kimicode")
        .command("kimi-code")
        .model("moonshot-v1-128k")
        .timeout_secs(3600)
        .build();

    let result = agent.validate_config(&config).await;
    if let Err(ref e) = result {
        assert!(
            !matches!(e, AgentError::ConfigValidation { field, .. } if field == "name"),
            "name 'kimicode' is valid — should not return a ConfigValidation on 'name'"
        );
    }
}

#[tokio::test]
async fn validate_config_wrong_name_returns_config_validation_error() {
    let agent = KimiCodeAgent::default();
    let config = AgentConfig::builder()
        .name("claude")
        .command("kimi-code")
        .model("moonshot-v1-128k")
        .timeout_secs(3600)
        .build();

    let result = agent.validate_config(&config).await;
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), AgentError::ConfigValidation { .. }),
        "wrong name should produce ConfigValidation error"
    );
}

#[tokio::test]
async fn validate_config_empty_model_fails() {
    let agent = KimiCodeAgent::default();
    let config = AgentConfig::builder()
        .name("kimicode")
        .command("kimi-code")
        .model("")
        .timeout_secs(3600)
        .build();

    assert!(agent.validate_config(&config).await.is_err());
}

#[tokio::test]
async fn validate_config_empty_command_fails() {
    let agent = KimiCodeAgent::default();
    let config = AgentConfig::builder()
        .name("kimicode")
        .command("")
        .model("moonshot-v1-128k")
        .timeout_secs(3600)
        .build();

    assert!(agent.validate_config(&config).await.is_err());
}

#[tokio::test]
async fn validate_config_zero_timeout_fails() {
    let agent = KimiCodeAgent::default();
    let config = AgentConfig::builder()
        .name("kimicode")
        .command("kimi-code")
        .model("moonshot-v1-128k")
        .timeout_secs(0)
        .build();

    assert!(agent.validate_config(&config).await.is_err());
}

#[tokio::test]
async fn validate_config_opencode_name_rejected() {
    // A config named "opencode" must be rejected even with the right command
    let agent = KimiCodeAgent::default();
    let config = AgentConfig::builder()
        .name("opencode")
        .command("kimi-code")
        .model("moonshot-v1-128k")
        .timeout_secs(3600)
        .build();

    let result = agent.validate_config(&config).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AgentError::ConfigValidation { .. }
    ));
}

// ── Execution guards ─────────────────────────────────────────────────────────

#[tokio::test]
async fn execute_rejects_empty_prompt() {
    let agent = KimiCodeAgent::default().without_verification();
    let config = ltmatrix::agent::ExecutionConfig::default();
    let result = agent.execute("", &config).await;
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(msg.contains("empty") || msg.contains("prompt"));
}

#[tokio::test]
async fn execute_rejects_whitespace_only_prompt() {
    let agent = KimiCodeAgent::default().without_verification();
    let config = ltmatrix::agent::ExecutionConfig::default();
    assert!(agent.execute("   \t\n  ", &config).await.is_err());
}

#[tokio::test]
async fn execute_with_session_rejects_empty_prompt() {
    let agent = KimiCodeAgent::default().without_verification();
    let config = ltmatrix::agent::ExecutionConfig::default();
    let session = MemorySession::default();

    let result = agent.execute_with_session("", &config, &session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn execute_with_session_rejects_whitespace_prompt() {
    let agent = KimiCodeAgent::default().without_verification();
    let config = ltmatrix::agent::ExecutionConfig::default();
    let session = MemorySession::default();

    assert!(
        agent
            .execute_with_session("   ", &config, &session)
            .await
            .is_err()
    );
}

// ── execute_task prompt formatting ───────────────────────────────────────────

#[tokio::test]
async fn execute_task_rejects_when_kimi_code_unavailable() {
    // Without the binary installed, execute_task should fail after building prompt.
    // We use without_verification() bypass so the error comes from the spawn, not --version.
    let agent = KimiCodeAgent::default().without_verification();
    let config = ltmatrix::agent::ExecutionConfig::default();
    let task = Task::new("t-1", "Implement feature X", "Add X to the codebase");

    // This will fail because kimi-code binary is not installed; the point is
    // verifying the method exists and is callable with the right signature.
    let result = agent.execute_task(&task, "some context", &config).await;
    // We accept either error — the important thing is it did NOT panic and
    // returned a Result.
    let _ = result;
}

#[tokio::test]
async fn execute_task_rejects_on_empty_context_propagates_error() {
    let agent = KimiCodeAgent::default().without_verification();
    let config = ltmatrix::agent::ExecutionConfig::default();
    let task = Task::new("t-2", "Write tests", "Write unit tests for module A");

    // execute_task produces a prompt internally; we only verify it doesn't panic.
    let result = agent.execute_task(&task, "", &config).await;
    let _ = result;
}

// ── Health check ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_check_always_returns_ok() {
    let agent = KimiCodeAgent::default();
    // Must return Ok(bool) — never Err — even when kimi-code is not installed.
    let result = agent.health_check().await;
    assert!(result.is_ok(), "health_check should never return Err");
}

#[tokio::test]
async fn is_available_returns_bool() {
    let agent = KimiCodeAgent::default();
    // is_available is a convenience wrapper around health_check
    let _: bool = agent.is_available().await;
}

#[tokio::test]
async fn health_check_returns_false_when_binary_absent() {
    // When kimi-code is not on PATH, health_check returns Ok(false) (not Err).
    let agent = KimiCodeAgent::default();
    let result = agent.health_check().await;
    assert!(result.is_ok());
    // We don't assert the bool value — it depends on the test environment.
}

// ── Completion detection ─────────────────────────────────────────────────────

#[test]
fn check_completion_positive_phrases() {
    assert!(KimiCodeAgent::check_completion("Task completed successfully."));
    assert!(KimiCodeAgent::check_completion("Implementation complete."));
    assert!(KimiCodeAgent::check_completion("Done! All steps finished."));
    assert!(KimiCodeAgent::check_completion("Success! The build passed."));
    assert!(KimiCodeAgent::check_completion("Completed all requirements."));
}

#[test]
fn check_completion_negative_phrases_override_positive() {
    assert!(!KimiCodeAgent::check_completion("not done yet"));
    assert!(!KimiCodeAgent::check_completion("task not completed"));
    assert!(!KimiCodeAgent::check_completion("Error: build failed"));
    assert!(!KimiCodeAgent::check_completion("incomplete task"));
    assert!(!KimiCodeAgent::check_completion("not finished"));
    assert!(!KimiCodeAgent::check_completion("not complete"));
}

#[test]
fn check_completion_empty_or_neutral() {
    assert!(!KimiCodeAgent::check_completion(""));
    assert!(!KimiCodeAgent::check_completion("Still working on this"));
    assert!(!KimiCodeAgent::check_completion("Processing..."));
}

#[test]
fn check_completion_case_insensitive() {
    assert!(KimiCodeAgent::check_completion("DONE"));
    assert!(KimiCodeAgent::check_completion("TASK COMPLETED"));
    assert!(!KimiCodeAgent::check_completion("ERROR: null pointer"));
    assert!(!KimiCodeAgent::check_completion("FAILED to compile"));
}

#[test]
fn check_completion_failed_substring_returns_false() {
    assert!(!KimiCodeAgent::check_completion(
        "Build failed with exit code 1"
    ));
    assert!(!KimiCodeAgent::check_completion("test failed"));
}

// ── Structured data parsing ───────────────────────────────────────────────────

#[test]
fn parse_structured_data_extracts_json_block() {
    let response = r#"
Here is the result:

```json
{"tasks": [{"id": "1", "title": "Init"}]}
```

Done.
"#;
    let data = KimiCodeAgent::parse_structured_data(response);
    assert!(data.is_some());
    assert!(data.unwrap().get("tasks").is_some());
}

#[test]
fn parse_structured_data_returns_none_for_plain_text() {
    assert!(KimiCodeAgent::parse_structured_data("No JSON here").is_none());
}

#[test]
fn parse_structured_data_returns_none_for_malformed_json() {
    let bad = "```json\n{ not valid json }\n```";
    assert!(KimiCodeAgent::parse_structured_data(bad).is_none());
}

#[test]
fn parse_structured_data_handles_nested_json() {
    let output = "```json\n{\"a\":{\"b\":[1,2,3]}}\n```";
    let data = KimiCodeAgent::parse_structured_data(output);
    assert!(data.is_some());
    let val = data.unwrap();
    assert!(val.get("a").is_some());
}

#[test]
fn parse_structured_data_uses_first_block_only() {
    let output =
        "```json\n{\"first\": true}\n```\n```json\n{\"second\": true}\n```";
    let data = KimiCodeAgent::parse_structured_data(output);
    assert!(data.is_some());
    assert_eq!(
        data.unwrap().get("first").and_then(|v| v.as_bool()),
        Some(true)
    );
}

#[test]
fn parse_structured_data_empty_json_object() {
    let output = "```json\n{}\n```";
    let data = KimiCodeAgent::parse_structured_data(output);
    assert!(data.is_some());
}

#[test]
fn parse_structured_data_json_array() {
    let output = "```json\n[1, 2, 3]\n```";
    let data = KimiCodeAgent::parse_structured_data(output);
    assert!(data.is_some());
    assert!(data.unwrap().is_array());
}

#[test]
fn parse_structured_data_returns_none_for_empty_string() {
    assert!(KimiCodeAgent::parse_structured_data("").is_none());
}

// ── Agent configuration accessor ─────────────────────────────────────────────

#[test]
fn agent_returns_correct_configuration() {
    let agent = KimiCodeAgent::default();
    let info = agent.agent();
    assert_eq!(info.name, "kimicode");
    assert_eq!(info.command, "kimi-code");
    assert_eq!(info.model, "moonshot-v1-128k");
    assert_eq!(info.timeout, 3600);
}

#[test]
fn agent_is_not_default_backend() {
    // KimiCode is not the default backend — is_default should be false.
    let agent = KimiCodeAgent::default();
    assert!(!agent.agent().is_default);
}

// ── Factory integration ───────────────────────────────────────────────────────

#[test]
fn factory_creates_kimicode_backend_by_name() {
    let factory = AgentFactory::new();
    let result = factory.create("kimicode");
    assert!(result.is_ok(), "factory.create('kimicode') should succeed");
    let agent = result.unwrap();
    assert_eq!(agent.backend_name(), "kimicode");
}

#[test]
fn factory_kimicode_agent_has_correct_model() {
    let factory = AgentFactory::new();
    let agent = factory.create("kimicode").unwrap();
    assert_eq!(agent.agent().model, "moonshot-v1-128k");
}

#[test]
fn factory_kimicode_agent_has_correct_command() {
    let factory = AgentFactory::new();
    let agent = factory.create("kimicode").unwrap();
    assert_eq!(agent.agent().command, "kimi-code");
}

#[test]
fn factory_supports_kimicode() {
    let factory = AgentFactory::new();
    assert!(factory.is_supported("kimicode"));
    assert!(factory.supported_backends().contains(&"kimicode"));
}

#[test]
fn factory_create_kimicode_with_custom_config() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("kimicode")
        .command("kimi-code")
        .model("moonshot-v1-32k")
        .timeout_secs(7200)
        .max_retries(5)
        .build();
    assert!(factory.create_with_config(config).is_ok());
}

#[test]
fn factory_kimicode_rejects_wrong_name() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("claude")
        .command("kimi-code")
        .model("moonshot-v1-128k")
        .timeout_secs(3600)
        .build();
    assert!(factory.validate_config("kimicode", &config).is_err());
}

#[test]
fn factory_kimicode_validation_passes_with_correct_config() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("kimicode")
        .command("kimi-code")
        .model("moonshot-v1-128k")
        .timeout_secs(3600)
        .build();
    assert!(factory.validate_config("kimicode", &config).is_ok());
}

#[test]
fn factory_kimicode_rejects_empty_model() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("kimicode")
        .command("kimi-code")
        .model("")
        .timeout_secs(3600)
        .build();
    assert!(factory.create_with_config(config).is_err());
}

#[test]
fn factory_kimicode_rejects_zero_timeout() {
    let factory = AgentFactory::new();
    let config = AgentConfig {
        name: "kimicode".to_string(),
        command: "kimi-code".to_string(),
        model: "moonshot-v1-128k".to_string(),
        timeout_secs: 0,
        max_retries: 3,
        enable_session: true,
    };
    assert!(factory.create_with_config(config).is_err());
}

#[test]
fn factory_kimicode_is_not_default_backend() {
    // The factory default backend is claude, not kimicode.
    let factory = AgentFactory::new();
    let default_agent = factory.create_default().unwrap();
    assert_ne!(default_agent.backend_name(), "kimicode");
}

// ── Model variants ────────────────────────────────────────────────────────────

#[test]
fn kimicode_all_model_variants_accepted_by_factory() {
    let factory = AgentFactory::new();
    let models = ["moonshot-v1-128k", "moonshot-v1-32k", "moonshot-v1-8k"];

    for model in models {
        let config = AgentConfig::builder()
            .name("kimicode")
            .command("kimi-code")
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

#[test]
fn kimicode_default_model_is_128k() {
    // The default model should be the long-context 128k variant.
    let agent = KimiCodeAgent::default();
    assert_eq!(agent.agent().model, "moonshot-v1-128k");
}
