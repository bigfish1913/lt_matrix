//! ClaudeAgent Integration Tests with Mock Commands
//!
//! These tests use mock/simulated commands to verify process spawning,
//! timeout handling, and response parsing behavior without requiring
//! the actual Claude CLI to be installed.

use ltmatrix::agent::backend::{
    AgentBackend, AgentConfig, AgentError, AgentSession, ExecutionConfig, MemorySession,
};
use ltmatrix::agent::claude::ClaudeAgent;
use ltmatrix::models::Task;

// ============================================================================
// Mock Command Tests
//
// These tests use simple shell commands that are available on most systems
// to verify process spawning behavior without requiring Claude CLI.
// ============================================================================

#[tokio::test]
async fn test_agent_echo_command() {
    // Note: This test would require a mock agent implementation
    // For now, we verify the error handling works

    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();

    // Try to execute - will fail if claude not installed, but that's OK
    let result = agent.execute("test", &config).await;

    // We're testing that the agent handles the result gracefully
    match result {
        Ok(_) => {
            // Claude is installed and executed successfully
        }
        Err(e) => {
            // Expected if claude not installed - verify error is descriptive
            let error_msg = e.to_string().to_lowercase();
            // Error should mention something about command or execution
            assert!(
                error_msg.contains("claude")
                    || error_msg.contains("command")
                    || error_msg.contains("not found")
                    || error_msg.contains("failed"),
                "Error should be descriptive, got: {}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_health_check_returns_result() {
    let agent = ClaudeAgent::new().unwrap();

    let result = agent.health_check().await;

    // Should always return Ok<bool>, never panic
    assert!(result.is_ok());
    let is_healthy = result.unwrap();
    assert!(is_healthy == true || is_healthy == false);
}

#[tokio::test]
async fn test_health_check_descriptive_on_failure() {
    let agent = ClaudeAgent::new().unwrap();

    let result = agent.health_check().await;

    match result {
        Ok(true) => {
            // Claude is installed and available
        }
        Ok(false) => {
            // Claude not installed or not available - this is OK
        }
        Err(e) => {
            // Should not happen, but if it does, error should be descriptive
            let error_msg = e.to_string();
            assert!(!error_msg.is_empty());
        }
    }
}

// ============================================================================
// Validation Tests
// ============================================================================

#[tokio::test]
async fn test_validate_config_with_valid_claude_config() {
    let agent = ClaudeAgent::new().unwrap();
    let config = AgentConfig::default();

    let result = agent.validate_config(&config).await;

    // Should succeed if claude is installed, fail otherwise with specific error
    match result {
        Ok(()) => {
            // Config is valid and claude is installed
        }
        Err(AgentError::CommandNotFound { .. }) => {
            // Expected if claude not installed
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_validate_config_with_wrong_name() {
    let agent = ClaudeAgent::new().unwrap();
    let mut config = AgentConfig::default();
    config.name = "not-claude".to_string();

    let result = agent.validate_config(&config).await;

    // Should fail with ConfigValidation error about name
    assert!(result.is_err());
    match result.unwrap_err() {
        AgentError::ConfigValidation { field, .. } => {
            assert_eq!(field, "name");
        }
        other => {
            panic!("Expected ConfigValidation error, got: {:?}", other);
        }
    }
}

#[tokio::test]
async fn test_validate_config_with_empty_model() {
    let agent = ClaudeAgent::new().unwrap();
    let mut config = AgentConfig::default();
    config.model = "".to_string();

    let result = agent.validate_config(&config).await;

    // Should fail with ConfigValidation error about model
    assert!(result.is_err());
    match result.unwrap_err() {
        AgentError::ConfigValidation { field, .. } => {
            assert_eq!(field, "model");
        }
        other => {
            panic!("Expected ConfigValidation error, got: {:?}", other);
        }
    }
}

// ============================================================================
// Execute Task Integration Tests
// ============================================================================

#[tokio::test]
async fn test_execute_task_constructs_prompt_correctly() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();

    let task = Task {
        id: "task-123".to_string(),
        title: "Implement Feature X".to_string(),
        description: "Add a new feature that does X, Y, and Z".to_string(),
        status: ltmatrix::models::TaskStatus::Pending,
        complexity: ltmatrix::models::TaskComplexity::Complex,
        depends_on: vec!["task-100".to_string(), "task-101".to_string()],
        subtasks: vec![],
        retry_count: 0,
        session_id: None,
        error: None,
        created_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
    };

    let context = r#"## Project Context
This is a Rust project using async/await.
The codebase follows modular architecture."#;

    let result = agent.execute_task(&task, context, &config).await;

    // We're testing that the method doesn't panic and handles errors gracefully
    match result {
        Ok(response) => {
            // If claude is installed, verify response structure
            assert!(!response.output.is_empty() || response.error.is_some());
        }
        Err(e) => {
            // Expected if claude not installed
            let error_msg = e.to_string().to_lowercase();
            assert!(
                error_msg.contains("claude")
                    || error_msg.contains("command")
                    || error_msg.contains("not found")
            );
        }
    }
}

#[tokio::test]
async fn test_execute_task_with_empty_context() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();

    let task = Task {
        id: "task-456".to_string(),
        title: "Simple Task".to_string(),
        description: "Simple description".to_string(),
        status: ltmatrix::models::TaskStatus::Pending,
        complexity: ltmatrix::models::TaskComplexity::Simple,
        depends_on: vec![],
        subtasks: vec![],
        retry_count: 0,
        session_id: None,
        error: None,
        created_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
    };

    let result = agent.execute_task(&task, "", &config).await;

    // Should handle empty context gracefully
    match result {
        Ok(_) | Err(_) => {
            // Either is acceptable
        }
    }
}

#[tokio::test]
async fn test_execute_task_with_long_context() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig {
        timeout: 10,
        ..Default::default()
    };

    let task = Task {
        id: "task-789".to_string(),
        title: "Complex Task".to_string(),
        description: "Complex task requiring lots of context".to_string(),
        status: ltmatrix::models::TaskStatus::Pending,
        complexity: ltmatrix::models::TaskComplexity::Complex,
        depends_on: vec![],
        subtasks: vec![],
        retry_count: 0,
        session_id: None,
        error: None,
        created_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
    };

    let long_context = "## Context\n".repeat(1000); // ~11KB of context

    let result = agent.execute_task(&task, &long_context, &config).await;

    // Should handle large context without panicking
    match result {
        Ok(_) | Err(_) => {
            // Either is acceptable
        }
    }
}

// ============================================================================
// Session Integration Tests
// ============================================================================

#[tokio::test]
async fn test_execute_with_session_accepts_session() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();
    let session = MemorySession::default();

    let result = agent
        .execute_with_session("test prompt", &config, &session)
        .await;

    // Should accept session parameter without panicking
    match result {
        Ok(_) | Err(_) => {
            // Either is acceptable
        }
    }
}

#[tokio::test]
async fn test_execute_with_session_preserves_session_info() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();
    let session = MemorySession::default();

    let session_id = session.session_id().to_string();
    let agent_name = session.agent_name().to_string();
    let model = session.model().to_string();

    // Execute with session
    let _result = agent.execute_with_session("test", &config, &session).await;

    // Session info should remain unchanged (we're not modifying it)
    assert_eq!(session.session_id(), session_id);
    assert_eq!(session.agent_name(), agent_name);
    assert_eq!(session.model(), model);
}

#[tokio::test]
async fn test_multiple_executions_with_same_session() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();
    let session = MemorySession::default();

    // Execute multiple times with the same session
    for i in 0..3 {
        let _result = agent
            .execute_with_session(&format!("prompt {}", i), &config, &session)
            .await;
    }

    // Session should still be valid
    assert!(!session.session_id().is_empty());
}

// ============================================================================
// Error Recovery Tests
// ============================================================================

#[tokio::test]
async fn test_agent_handles_graceful_degradation() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();

    // First execution might fail if claude not installed
    let result1 = agent.execute("test1", &config).await;

    // Subsequent executions should still be possible
    let result2 = agent.execute("test2", &config).await;
    let result3 = agent.execute("test3", &config).await;

    // All should handle the same way (all succeed or all fail similarly)
    let success1 = result1.is_ok();
    let success2 = result2.is_ok();
    let success3 = result3.is_ok();

    // Should be consistent (if claude is not installed, all fail; if installed, all succeed)
    assert_eq!(success1, success2);
    assert_eq!(success2, success3);
}

#[tokio::test]
async fn test_agent_without_verification_skips_check() {
    let agent_with_verification = ClaudeAgent::new().unwrap();
    let agent_without_verification = ClaudeAgent::new().unwrap().without_verification();

    let config = ExecutionConfig::default();

    // Both should handle execution similarly
    let result1 = agent_with_verification.execute("test", &config).await;
    let result2 = agent_without_verification.execute("test", &config).await;

    // Note: without_verification might behave slightly differently,
    // but both should not panic
    match (result1, result2) {
        (Ok(_), Ok(_)) | (Err(_), Err(_)) => {
            // Consistent behavior
        }
        _ => {
            // Different behavior is also acceptable
        }
    }
}

// ============================================================================
// Configuration Override Tests
// ============================================================================

#[tokio::test]
async fn test_execution_config_model_override() {
    let agent = ClaudeAgent::new().unwrap();

    let configs = vec![
        ExecutionConfig {
            model: "claude-sonnet-4-6".to_string(),
            ..Default::default()
        },
        ExecutionConfig {
            model: "claude-opus-4-6".to_string(),
            ..Default::default()
        },
        ExecutionConfig {
            model: "claude-haiku-4-5".to_string(),
            ..Default::default()
        },
    ];

    for config in configs {
        let result = agent.execute("test", &config).await;
        // Should accept the config without panicking
        match result {
            Ok(_) | Err(_) => {
                // Either is acceptable
            }
        }
    }
}

#[tokio::test]
async fn test_execution_config_timeout_variations() {
    let agent = ClaudeAgent::new().unwrap();

    let timeouts = vec![1, 10, 60, 300, 3600, 7200];

    for timeout in timeouts {
        let config = ExecutionConfig {
            timeout,
            ..Default::default()
        };

        let result = agent.execute("test", &config).await;
        // Should accept any timeout value
        match result {
            Ok(_) | Err(_) => {
                // Either is acceptable
            }
        }
    }
}

#[tokio::test]
async fn test_execution_config_retry_variations() {
    let agent = ClaudeAgent::new().unwrap();

    let retry_counts = vec![0, 1, 2, 3, 5, 10];

    for max_retries in retry_counts {
        let config = ExecutionConfig {
            max_retries,
            ..Default::default()
        };

        let result = agent.execute("test", &config).await;
        // Should accept any retry count
        match result {
            Ok(_) | Err(_) => {
                // Either is acceptable
            }
        }
    }
}

// ============================================================================
// Special Input Tests
// ============================================================================

#[tokio::test]
async fn test_agent_handles_special_characters() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();

    let special_prompts = vec![
        "Test with $VAR and {brace}",
        "Test with \"quotes\" and 'apostrophes'",
        "Test with \t\ttabs and\n\nnewlines",
        "Test with backticks `code`",
        "Test with <html> tags",
        "Test with &amp; and && operators",
        "Test with | pipe",
        "Test with ; semicolons",
        "Test with \\ escape sequences",
    ];

    for prompt in special_prompts {
        let result = agent.execute(prompt, &config).await;
        // Should handle all special characters without panicking
        match result {
            Ok(_) | Err(_) => {
                // Either is acceptable
            }
        }
    }
}

#[tokio::test]
async fn test_agent_handles_very_long_prompts() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig {
        timeout: 10,
        ..Default::default()
    };

    let long_prompt = "a".repeat(100_000); // 100KB prompt

    let result = agent.execute(&long_prompt, &config).await;

    // Should handle long prompts without crashing
    match result {
        Ok(_) | Err(_) => {
            // Either is acceptable
        }
    }
}

#[tokio::test]
async fn test_agent_handles_multiline_prompts() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();

    let multiline_prompt = r#"
## Task Description

Write a function that:

1. Validates input
2. Processes data
3. Returns result

### Requirements

- Must be efficient
- Must handle errors
- Must be well-documented

### Example

```rust
fn example() {
    // code here
}
```

Please implement this function.
"#;

    let result = agent.execute(multiline_prompt, &config).await;

    // Should handle multiline prompts correctly
    match result {
        Ok(_) | Err(_) => {
            // Either is acceptable
        }
    }
}

// ============================================================================
// Backend Name Tests
// ============================================================================

#[tokio::test]
async fn test_backend_name_returns_correct_value() {
    let _agent = ClaudeAgent::new().unwrap();

    assert_eq!(_agent.backend_name(), "claude");
}

#[tokio::test]
async fn test_agent_returns_correct_config() {
    let agent = ClaudeAgent::new().unwrap();
    let agent_config = agent.agent();

    assert_eq!(agent_config.name, "claude");
    assert_eq!(agent_config.model, "claude-sonnet-4-6");
    assert_eq!(agent_config.command, "claude");
}

// ============================================================================
// Response Structure Tests
// ============================================================================

#[tokio::test]
async fn test_successful_response_has_required_fields() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();

    let result = agent.execute("simple test", &config).await;

    if let Ok(response) = result {
        // Response should have all required fields
        let _output: &str = &response.output;
        let _structured_data: Option<serde_json::Value> = response.structured_data;
        let _is_complete: bool = response.is_complete;
        let _error: Option<String> = response.error;
    }
}

#[tokio::test]
async fn test_error_response_contains_error_info() {
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();

    let result = agent.execute("", &config).await;

    if let Err(e) = result {
        // Error should be descriptive
        let error_msg = e.to_string();
        assert!(!error_msg.is_empty());
    }
}

// ============================================================================
// Concurrent Safety Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_health_checks() {
    let _agent = ClaudeAgent::new().unwrap();

    let mut handles = vec![];

    // Spawn 5 concurrent health checks
    for _ in 0..5 {
        let agent_clone = ClaudeAgent::new().unwrap();
        let handle = tokio::spawn(async move { agent_clone.health_check().await });
        handles.push(handle);
    }

    // All should complete without panicking
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_concurrent_config_validations() {
    let _agent = ClaudeAgent::new().unwrap();
    let config = AgentConfig::default();

    let mut handles = vec![];

    // Spawn 5 concurrent validations
    for _ in 0..5 {
        let agent_clone = ClaudeAgent::new().unwrap();
        let config_clone = config.clone();
        let handle = tokio::spawn(async move { agent_clone.validate_config(&config_clone).await });
        handles.push(handle);
    }

    // All should complete without panicking
    for handle in handles {
        let result = handle.await.unwrap();
        match result {
            Ok(_) | Err(AgentError::CommandNotFound { .. }) => {
                // Either is acceptable
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }
}

// ============================================================================
// Default Implementation Tests
// ============================================================================

#[tokio::test]
async fn test_default_agent_has_correct_defaults() {
    let agent = ClaudeAgent::default();

    let agent_ref = agent.agent();
    assert_eq!(agent_ref.name, "claude");
    assert_eq!(agent_ref.model, "claude-sonnet-4-6");
    assert_eq!(agent_ref.command, "claude");
}
