//! Comprehensive tests for AgentBackend trait and core types
//!
//! These tests provide complete coverage of the abstraction contract,
//! including edge cases, error handling, and trait contract verification.

use ltmatrix::agent::backend::{
    AgentBackend, AgentConfig, AgentError, AgentResponse, AgentSession, ExecutionConfig,
    MemorySession,
};
use ltmatrix::agent::claude::ClaudeAgent;
use ltmatrix::models::Task;
use std::time::Duration;

// ============================================================================
// AgentError Comprehensive Tests
// ============================================================================

#[test]
fn test_agent_error_all_variants_match() {
    // Test all error variants can be created and matched
    let errors = vec![
        AgentError::CommandNotFound {
            command: "test-cmd".to_string(),
        },
        AgentError::ExecutionFailed {
            command: "test-cmd".to_string(),
            message: "Failed to execute".to_string(),
        },
        AgentError::Timeout {
            command: "test-cmd".to_string(),
            timeout_secs: 100,
        },
        AgentError::InvalidResponse {
            reason: "Malformed JSON".to_string(),
        },
        AgentError::ConfigValidation {
            field: "timeout".to_string(),
            message: "Must be positive".to_string(),
        },
        AgentError::SessionNotFound {
            session_id: "sess-123".to_string(),
        },
    ];

    for error in errors {
        // Verify all errors implement Display
        let display_str = format!("{}", error);
        assert!(!display_str.is_empty());

        // Verify all errors implement Debug
        let debug_str = format!("{:?}", error);
        assert!(!debug_str.is_empty());
    }
}

#[test]
fn test_agent_error_std_error_trait() {
    let error = AgentError::CommandNotFound {
        command: "claude".to_string(),
    };

    // Verify it implements std::error::Error
    // Note: AgentError doesn't have a source(), so we just verify Display
    assert!(!error.to_string().is_empty());
}

#[test]
fn test_agent_error_clone() {
    let error1 = AgentError::Timeout {
        command: "test".to_string(),
        timeout_secs: 60,
    };
    let error2 = error1.clone();

    assert!(matches!(error1, AgentError::Timeout { .. }));
    assert!(matches!(error2, AgentError::Timeout { .. }));

    if let AgentError::Timeout {
        command: cmd1,
        timeout_secs: secs1,
    } = error1
    {
        if let AgentError::Timeout {
            command: cmd2,
            timeout_secs: secs2,
        } = error2
        {
            assert_eq!(cmd1, cmd2);
            assert_eq!(secs1, secs2);
        }
    }
}

// ============================================================================
// AgentConfig Comprehensive Tests
// ============================================================================

#[test]
fn test_agent_config_whitespace_validation() {
    let mut config = AgentConfig::default();

    // Test whitespace-only name
    config.name = "   ".to_string();
    let result = config.validate();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AgentError::ConfigValidation { .. }
    ));

    // Test whitespace-only model
    config.name = "test".to_string();
    config.model = "\t\n".to_string();
    let result = config.validate();
    assert!(result.is_err());

    // Test whitespace-only command
    config.model = "test-model".to_string();
    config.command = "  ".to_string();
    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_agent_config_builder_fluent_interface() {
    let config = AgentConfig::builder()
        .name("agent1")
        .model("model1")
        .command("cmd1")
        .timeout_secs(100)
        .max_retries(2)
        .enable_session(true)
        .build();

    assert_eq!(config.name, "agent1");
    assert_eq!(config.model, "model1");
    assert_eq!(config.command, "cmd1");
    assert_eq!(config.timeout_secs, 100);
    assert_eq!(config.max_retries, 2);
    assert!(config.enable_session);
}

#[test]
fn test_agent_config_builder_partial_usage() {
    // Test that builder provides sensible defaults for unspecified fields
    let config = AgentConfig::builder().name("test-agent").build();

    assert_eq!(config.name, "test-agent");
    assert!(!config.model.is_empty()); // Should have default
    assert!(!config.command.is_empty()); // Should have default
    assert!(config.timeout_secs > 0); // Should have default
}

#[test]
fn test_agent_config_clone() {
    let config1 = AgentConfig::builder()
        .name("original")
        .model("model-1")
        .build();
    let config2 = config1.clone();

    assert_eq!(config1.name, config2.name);
    assert_eq!(config1.model, config2.model);
    assert_eq!(config1.command, config2.command);
}

// ============================================================================
// AgentSession Trait Tests
// ============================================================================

#[test]
fn test_agent_session_trait_methods() {
    let session = MemorySession::default();

    // Test all trait methods are accessible
    let _id = session.session_id();
    let _agent = session.agent_name();
    let _model = session.model();
    let _created = session.created_at();
    let _accessed = session.last_accessed();
    let _count = session.reuse_count();
    let _stale = session.is_stale();
}

#[test]
fn test_agent_session_mark_accessed_multiple_times() {
    let mut session = MemorySession::default();
    let initial_time = session.last_accessed;

    // Mark as accessed multiple times
    for i in 1..=5 {
        session.mark_accessed();
        assert_eq!(session.reuse_count, i);
        assert!(session.last_accessed >= initial_time);
    }
}

#[test]
fn test_agent_session_stale_threshold() {
    let mut session = MemorySession::default();

    // Fresh session - not stale
    assert!(!session.is_stale());

    // Exactly 1 hour old - should not be stale (strictly greater than)
    let one_hour_ago = chrono::Utc::now() - chrono::Duration::seconds(3600);
    session.last_accessed = one_hour_ago;
    assert!(!session.is_stale());

    // Just over 1 hour old - should be stale
    let one_hour_one_second_ago = chrono::Utc::now() - chrono::Duration::seconds(3601);
    session.last_accessed = one_hour_one_second_ago;
    assert!(session.is_stale());
}

#[test]
fn test_agent_session_unique_ids() {
    let session1 = MemorySession::default();
    let session2 = MemorySession::default();

    // Each session should have a unique ID
    assert_ne!(session1.session_id, session2.session_id);
}

// ============================================================================
// ExecutionConfig Tests
// ============================================================================

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
fn test_execution_config_clone() {
    let config1 = ExecutionConfig {
        model: "custom-model".to_string(),
        max_retries: 5,
        timeout: 7200,
        enable_session: false,
        env_vars: vec![("KEY1".to_string(), "value1".to_string())],
    };

    let config2 = config1.clone();

    assert_eq!(config1.model, config2.model);
    assert_eq!(config1.max_retries, config2.max_retries);
    assert_eq!(config1.timeout, config2.timeout);
    assert_eq!(config1.enable_session, config2.enable_session);
    assert_eq!(config1.env_vars.len(), config2.env_vars.len());
}

#[test]
fn test_execution_config_with_env_vars() {
    let config = ExecutionConfig {
        model: "test-model".to_string(),
        max_retries: 1,
        timeout: 60,
        enable_session: true,
        env_vars: vec![
            ("API_KEY".to_string(), "secret".to_string()),
            ("DEBUG".to_string(), "true".to_string()),
        ],
    };

    assert_eq!(config.env_vars.len(), 2);
    assert_eq!(config.env_vars[0].0, "API_KEY");
    assert_eq!(config.env_vars[1].0, "DEBUG");
}

// ============================================================================
// AgentResponse Tests
// ============================================================================

#[test]
fn test_agent_response_complete() {
    let response = AgentResponse {
        output: "Task completed successfully".to_string(),
        structured_data: None,
        is_complete: true,
        error: None,
    };

    assert!(response.is_complete);
    assert!(response.error.is_none());
    assert!(!response.output.is_empty());
}

#[test]
fn test_agent_response_with_error() {
    let response = AgentResponse {
        output: "Partial output".to_string(),
        structured_data: None,
        is_complete: false,
        error: Some("Execution failed: timeout".to_string()),
    };

    assert!(!response.is_complete);
    assert!(response.error.is_some());
    assert!(response.error.unwrap().contains("timeout"));
}

#[test]
fn test_agent_response_with_structured_data() {
    let json_data = serde_json::json!({
        "tasks": [
            {"id": "1", "title": "Task 1"},
            {"id": "2", "title": "Task 2"}
        ]
    });

    let response = AgentResponse {
        output: "Here are the tasks:".to_string(),
        structured_data: Some(json_data.clone()),
        is_complete: true,
        error: None,
    };

    assert!(response.structured_data.is_some());
    let data = response.structured_data.unwrap();
    assert!(data.get("tasks").is_some());
}

#[test]
fn test_agent_response_clone() {
    let response1 = AgentResponse {
        output: "test output".to_string(),
        structured_data: Some(serde_json::json!({"key": "value"})),
        is_complete: true,
        error: None,
    };

    let response2 = response1.clone();

    assert_eq!(response1.output, response2.output);
    assert_eq!(response1.is_complete, response2.is_complete);
    assert!(response1.structured_data.is_some());
    assert!(response2.structured_data.is_some());
}

// ============================================================================
// AgentBackend Trait Contract Tests
// ============================================================================

#[tokio::test]
async fn test_agent_backend_trait_methods_exist() {
    let agent = ClaudeAgent::new().unwrap();

    // Verify all trait methods are callable
    let _name = agent.backend_name();
    let _agent_config = agent.agent();

    // These should not panic even if they fail
    let _health = agent.health_check().await;
    let _available = agent.is_available().await;

    let config = ExecutionConfig::default();
    let session = MemorySession::default();

    // Execute methods exist (may fail if claude not installed)
    let _exec_result = agent.execute("test", &config).await;
    let _session_result = agent.execute_with_session("test", &config, &session).await;

    let task = Task {
        id: "test-1".to_string(),
        title: "Test Task".to_string(),
        description: "A test task".to_string(),
        status: ltmatrix::models::TaskStatus::Pending,
        complexity: ltmatrix::models::TaskComplexity::Moderate,
        agent_type: Default::default(),
        priority: 5,
        related_tasks: vec![],
        resources: None,
        depends_on: vec![],
        subtasks: vec![],
        retry_count: 0,
        session_id: None,
        parent_session_id: None,
        error: None,
        created_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
    };
    let _task_result = agent.execute_task(&task, "test context", &config).await;
}

#[tokio::test]
async fn test_agent_backend_backend_name() {
    let agent = ClaudeAgent::new().unwrap();

    // backend_name should default to agent name
    let backend_name = agent.backend_name();
    let agent_name = agent.agent().name.as_str();

    assert_eq!(backend_name, agent_name);
    assert_eq!(backend_name, "claude");
}

#[tokio::test]
async fn test_agent_backend_agent_method() {
    let agent = ClaudeAgent::new().unwrap();

    // agent() should return a reference to Agent
    let agent_ref = agent.agent();
    assert_eq!(agent_ref.name, "claude");
    assert_eq!(agent_ref.model, "claude-sonnet-4-6");
}

#[tokio::test]
async fn test_agent_backend_is_available_convenience() {
    let agent = ClaudeAgent::new().unwrap();

    // is_available is a convenience method that returns bool directly
    let available = agent.is_available().await;

    // Should always return a bool, never panic
    match available {
        true | false => {} // Valid
    }
}

#[tokio::test]
async fn test_agent_backend_validate_config_integration() {
    let agent = ClaudeAgent::new().unwrap();

    // Test with valid config
    let valid_config = AgentConfig::default();
    let result = agent.validate_config(&valid_config).await;
    assert!(result.is_ok());

    // Test with invalid config (empty model)
    let mut invalid_config = AgentConfig::default();
    invalid_config.model = "".to_string();
    let result = agent.validate_config(&invalid_config).await;
    assert!(result.is_err());

    if let Err(AgentError::ConfigValidation { field, .. }) = result {
        assert_eq!(field, "model");
    } else {
        panic!("Expected ConfigValidation error");
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_agent_backend_full_execution_flow() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();

    // Test the full flow: validate -> check health -> execute
    let agent_config = AgentConfig::default();
    let validation_result = agent.validate_config(&agent_config).await;
    assert!(validation_result.is_ok());

    let health_result = agent.health_check().await;
    // Health check may fail if claude not installed, but should not panic
    let _ = health_result;

    // Execute with empty prompt should fail validation
    let exec_result = agent.execute("", &config).await;
    assert!(exec_result.is_err());
}

#[tokio::test]
async fn test_agent_backend_with_custom_execution_config() {
    let agent = ClaudeAgent::new().unwrap();

    // Test with custom execution config
    let custom_config = ExecutionConfig {
        model: "claude-opus-4-6".to_string(),
        max_retries: 5,
        timeout: 7200,
        enable_session: false,
        env_vars: vec![],
    };

    // This might fail if claude is not installed, but config should be accepted
    let _result = agent.execute("test prompt", &custom_config).await;
}

#[tokio::test]
async fn test_agent_session_with_backend() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();
    let session = MemorySession::default();

    // Test execute_with_session accepts the session
    // May fail if claude not installed, but should accept the session parameter
    let _result = agent.execute_with_session("test", &config, &session).await;
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_agent_config_validation_edge_cases() {
    let mut config = AgentConfig::default();

    // Test very long values
    config.name = "a".repeat(10000);
    assert!(config.validate().is_ok());

    // Test zero max_retries (should be valid)
    config.name = "test".to_string();
    config.max_retries = 0;
    assert!(config.validate().is_ok());

    // Test very large timeout
    config.timeout_secs = u64::MAX;
    assert!(config.validate().is_ok());
}

#[test]
fn test_agent_session_timing_edge_cases() {
    let mut session = MemorySession::default();

    // Test marking accessed at time boundary
    let before_mark = session.last_accessed;
    std::thread::sleep(Duration::from_millis(10));
    session.mark_accessed();
    let after_mark = session.last_accessed;

    assert!(after_mark > before_mark);
    assert_eq!(session.reuse_count, 1);
}

#[tokio::test]
async fn test_agent_backend_timeout_behavior() {
    let agent = ClaudeAgent::new().unwrap();

    // Create a config with very short timeout
    let short_timeout_config = ExecutionConfig {
        model: "claude-sonnet-4-6".to_string(),
        max_retries: 0,
        timeout: 1, // 1 second
        enable_session: false,
        env_vars: vec![],
    };

    // This should timeout quickly if claude is installed and slow
    // Or fail fast if claude is not installed
    let _result = agent
        .execute("generate a very long response", &short_timeout_config)
        .await;
}

#[tokio::test]
async fn test_agent_backend_retry_configuration() {
    let agent = ClaudeAgent::new().unwrap();

    // Test different retry configurations
    for retries in [0, 1, 3, 5, 10] {
        let config = ExecutionConfig {
            model: "claude-sonnet-4-6".to_string(),
            max_retries: retries,
            timeout: 60,
            enable_session: false,
            env_vars: vec![],
        };

        // Config should be accepted even if execution fails
        let _result = agent.execute("test", &config).await;
    }
}

// ============================================================================
// Error Propagation Tests
// ============================================================================

#[tokio::test]
async fn test_agent_backend_error_propagation() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();

    // Empty prompt should produce an error
    let result = agent.execute("", &config).await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string().to_lowercase();

    // Error should mention prompt or empty
    assert!(
        error_msg.contains("prompt") || error_msg.contains("empty"),
        "Error message should mention prompt or empty, got: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_agent_backend_validation_error_messages() {
    let agent = ClaudeAgent::new().unwrap();

    let test_cases = vec![
        (
            AgentConfig {
                name: "".to_string(),
                ..Default::default()
            },
            "name",
        ),
        (
            AgentConfig {
                name: "test".to_string(),
                model: "".to_string(),
                ..Default::default()
            },
            "model",
        ),
        (
            AgentConfig {
                name: "test".to_string(),
                model: "test-model".to_string(),
                command: "".to_string(),
                ..Default::default()
            },
            "command",
        ),
        (
            AgentConfig {
                name: "test".to_string(),
                model: "test-model".to_string(),
                command: "test".to_string(),
                timeout_secs: 0,
                ..Default::default()
            },
            "timeout_secs",
        ),
    ];

    for (invalid_config, expected_field) in test_cases {
        let result = agent.validate_config(&invalid_config).await;

        assert!(result.is_err());
        if let Err(AgentError::ConfigValidation { field, .. }) = result {
            assert_eq!(field, expected_field);
        } else {
            panic!(
                "Expected ConfigValidation error for field: {}",
                expected_field
            );
        }
    }
}

// ============================================================================
// Memory-Specific Tests
// ============================================================================

#[test]
fn test_memory_session_all_fields_initialized() {
    let session = MemorySession::default();

    // Verify all fields are initialized
    assert!(!session.session_id.is_empty());
    assert!(!session.agent_name.is_empty());
    assert!(!session.model.is_empty());
    assert!(session.created_at <= chrono::Utc::now());
    assert!(session.last_accessed <= chrono::Utc::now());
    assert_eq!(session.reuse_count, 0);
}

#[test]
fn test_memory_session_clone_preserves_state() {
    let mut session1 = MemorySession::default();
    session1.mark_accessed();
    session1.mark_accessed();

    let session2 = session1.clone();

    assert_eq!(session1.session_id, session2.session_id);
    assert_eq!(session1.agent_name, session2.agent_name);
    assert_eq!(session1.model, session2.model);
    assert_eq!(session1.reuse_count, session2.reuse_count);
    // Timestamps should be equal too
    assert_eq!(session1.created_at, session2.created_at);
    assert_eq!(session1.last_accessed, session2.last_accessed);
}

#[test]
fn test_memory_session_implements_send_sync() {
    // Verify MemorySession implements Send + Sync (required for AgentSession)
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<MemorySession>();
}
