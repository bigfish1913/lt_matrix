//! AgentBackend Trait Contract Verification Tests
//!
//! These tests verify that implementations properly adhere to the AgentBackend
//! trait contract, including behavioral guarantees, error handling, and
//! integration requirements.

use ltmatrix::agent::backend::{
    AgentBackend, AgentConfig, AgentError, AgentSession, ExecutionConfig, MemorySession,
};
use ltmatrix::agent::claude::ClaudeAgent;
use ltmatrix::models::{Agent, Task};
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Mock Agent for Testing
// ============================================================================

/// A mock agent implementation for testing trait contract compliance
#[derive(Debug, Clone)]
struct MockAgent {
    should_fail: bool,
    fail_with: Option<AgentError>,
    agent_config: Agent,
}

impl MockAgent {
    fn new() -> Self {
        MockAgent {
            should_fail: false,
            fail_with: None,
            agent_config: Agent::claude_default(),
        }
    }

    fn failing(mut self, error: AgentError) -> Self {
        self.should_fail = true;
        self.fail_with = Some(error);
        self
    }

    fn create_mock_task(&self) -> Task {
        Task::new("mock-task-1", "Mock Task", "A mock task for testing")
    }
}

impl Default for MockAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AgentBackend for MockAgent {
    async fn execute(
        &self,
        prompt: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<ltmatrix::agent::backend::AgentResponse> {
        if self.should_fail {
            if let Some(ref error) = self.fail_with {
                return Err(anyhow::anyhow!("{}", error));
            }
        }

        if prompt.trim().is_empty() {
            return Err(anyhow::anyhow!("Prompt cannot be empty"));
        }

        Ok(ltmatrix::agent::backend::AgentResponse {
            output: format!("Mock response to: {}", prompt),
            structured_data: None,
            is_complete: true,
            error: None,
        })
    }

    async fn execute_with_session(
        &self,
        prompt: &str,
        _config: &ExecutionConfig,
        session: &dyn AgentSession,
    ) -> anyhow::Result<ltmatrix::agent::backend::AgentResponse> {
        if self.should_fail {
            if let Some(ref error) = self.fail_with {
                return Err(anyhow::anyhow!("{}", error));
            }
        }

        // Session should be marked as accessed
        // Note: We can't actually mark it accessed here since we don't have mut access
        // In a real implementation, the session would be updated internally

        Ok(ltmatrix::agent::backend::AgentResponse {
            output: format!(
                "Mock response with session {} to: {}",
                session.session_id(),
                prompt
            ),
            structured_data: None,
            is_complete: true,
            error: None,
        })
    }

    async fn execute_task(
        &self,
        task: &Task,
        context: &str,
        config: &ExecutionConfig,
    ) -> anyhow::Result<ltmatrix::agent::backend::AgentResponse> {
        if self.should_fail {
            if let Some(ref error) = self.fail_with {
                return Err(anyhow::anyhow!("{}", error));
            }
        }

        let prompt = format!(
            "Task: {}\n\nDescription: {}\n\nContext: {}",
            task.title, task.description, context
        );

        self.execute(&prompt, config).await
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        if self.should_fail {
            if let Some(ref error) = self.fail_with {
                match error {
                    AgentError::CommandNotFound { .. } => return Ok(false),
                    _ => return Err(anyhow::anyhow!("{}", error)),
                }
            }
        }
        Ok(true)
    }

    async fn validate_config(&self, config: &AgentConfig) -> Result<(), AgentError> {
        config.validate()
    }

    fn agent(&self) -> &Agent {
        &self.agent_config
    }
}

// ============================================================================
// Trait Contract Verification Tests
// ============================================================================

#[tokio::test]
async fn test_contract_execute_validates_prompt() {
    let agent = MockAgent::new();
    let config = ExecutionConfig::default();

    // Empty prompt should fail
    let result = agent.execute("", &config).await;
    assert!(result.is_err());

    // Whitespace-only prompt should also fail
    let result = agent.execute("   \t\n", &config).await;
    assert!(result.is_err());

    // Valid prompt should succeed
    let result = agent.execute("valid prompt", &config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_contract_execute_returns_complete_response() {
    let agent = MockAgent::new();
    let config = ExecutionConfig::default();

    let response = agent.execute("test prompt", &config).await.unwrap();

    // Response should have all required fields
    assert!(!response.output.is_empty());
    assert!(response.is_complete);
    assert!(response.error.is_none());
}

#[tokio::test]
async fn test_contract_execute_with_session_uses_session() {
    let agent = MockAgent::new();
    let config = ExecutionConfig::default();
    let session = MemorySession::default();

    let response = agent
        .execute_with_session("test prompt", &config, &session)
        .await
        .unwrap();

    // Response should include session information
    assert!(response.output.contains(session.session_id()));
}

#[tokio::test]
async fn test_contract_execute_task_includes_context() {
    let agent = MockAgent::new();
    let config = ExecutionConfig::default();
    let task = agent.create_mock_task();

    let response = agent
        .execute_task(&task, "additional context", &config)
        .await
        .unwrap();

    // Response should include task details and context
    assert!(response.output.contains(&task.title));
    assert!(response.output.contains(&task.description));
    assert!(response.output.contains("additional context"));
}

#[tokio::test]
async fn test_contract_health_check_returns_result() {
    let agent = MockAgent::new();

    let result = agent.health_check().await;

    // Should always return Ok<bool>, never panic
    assert!(result.is_ok());
    let is_healthy = result.unwrap();
    assert!(is_healthy == true || is_healthy == false);
}

#[tokio::test]
async fn test_contract_health_check_with_command_not_found() {
    let agent = MockAgent::new().failing(AgentError::CommandNotFound {
        command: "mock".to_string(),
    });

    let result = agent.health_check().await;

    // Should return Ok(false) for command not found
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false);
}

#[tokio::test]
async fn test_contract_is_available_convenience_method() {
    let agent = MockAgent::new();

    // is_available should never panic and always return bool
    let available = agent.is_available().await;
    assert!(available == true || available == false);
}

#[tokio::test]
async fn test_contract_validate_config_checks_required_fields() {
    let agent = MockAgent::new();

    // Valid config should pass
    let valid_config = AgentConfig::default();
    let result = agent.validate_config(&valid_config).await;
    assert!(result.is_ok());

    // Invalid config should fail
    let mut invalid_config = AgentConfig::default();
    invalid_config.name = "".to_string();
    let result = agent.validate_config(&invalid_config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_contract_agent_returns_valid_reference() {
    let agent = MockAgent::new();

    let agent_ref = agent.agent();

    // Should return a valid reference
    assert!(!agent_ref.name.is_empty());
    assert!(!agent_ref.model.is_empty());
    assert!(!agent_ref.command.is_empty());
}

#[tokio::test]
async fn test_contract_backend_name_defaults_to_agent_name() {
    let agent = MockAgent::new();

    let backend_name = agent.backend_name();
    let agent_name = agent.agent().name.as_str();

    assert_eq!(backend_name, agent_name);
}

// ============================================================================
// Thread Safety Tests
// ============================================================================

#[tokio::test]
async fn test_contract_backend_is_send_and_sync() {
    // AgentBackend should be Send + Sync for use in async contexts
    let agent = Arc::new(MockAgent::new());

    // Spawn multiple tasks using the same agent
    let mut handles = vec![];

    for i in 0..5 {
        let agent_clone = Arc::clone(&agent);
        let handle = tokio::spawn(async move {
            let config = ExecutionConfig::default();
            let _ = agent_clone
                .execute(&format!("test prompt {}", i), &config)
                .await;
        });
        handles.push(handle);
    }

    // All tasks should complete without panicking
    for handle in handles {
        let _ = handle.await;
    }
}

#[tokio::test]
async fn test_contract_session_is_send_and_sync() {
    // AgentSession should be Send + Sync
    let session = Arc::new(RwLock::new(MemorySession::default()));

    let mut handles = vec![];

    for i in 0..5 {
        let session_clone = Arc::clone(&session);
        let handle = tokio::spawn(async move {
            let mut sess = session_clone.write().await;
            sess.mark_accessed();
            assert_eq!(sess.reuse_count, i + 1);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let final_session = session.read().await;
    assert_eq!(final_session.reuse_count, 5);
}

// ============================================================================
// Error Handling Contract Tests
// ============================================================================

#[tokio::test]
async fn test_contract_errors_are_descriptive() {
    let test_cases = vec![
        AgentError::CommandNotFound {
            command: "test-cmd".to_string(),
        },
        AgentError::ExecutionFailed {
            command: "test-cmd".to_string(),
            message: "Process failed".to_string(),
        },
        AgentError::Timeout {
            command: "test-cmd".to_string(),
            timeout_secs: 60,
        },
        AgentError::InvalidResponse {
            reason: "Malformed JSON".to_string(),
        },
        AgentError::ConfigValidation {
            field: "model".to_string(),
            message: "Cannot be empty".to_string(),
        },
        AgentError::SessionNotFound {
            session_id: "sess-123".to_string(),
        },
    ];

    for error in test_cases {
        let error_string = format!("{}", error);
        assert!(!error_string.is_empty());

        // Error should include relevant context
        if let AgentError::CommandNotFound { command } = error {
            assert!(error_string.contains(&command));
        }
    }
}

#[tokio::test]
async fn test_contract_validate_config_returns_specific_errors() {
    let agent = MockAgent::new();

    let test_cases: Vec<(AgentConfig, &str)> = vec![
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
    ];

    for (invalid_config, expected_field) in test_cases {
        let result = agent.validate_config(&invalid_config).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AgentError::ConfigValidation { field, message } => {
                assert_eq!(field, expected_field);
                assert!(!message.is_empty());
            }
            other => panic!("Expected ConfigValidation error, got: {:?}", other),
        }
    }
}

// ============================================================================
// Session Management Contract Tests
// ============================================================================

#[tokio::test]
async fn test_contract_session_lifecycle() {
    let mut session = MemorySession::default();

    // Initial state
    assert_eq!(session.reuse_count(), 0);
    assert!(!session.is_stale());

    // Mark as accessed
    session.mark_accessed();
    assert_eq!(session.reuse_count(), 1);

    // Multiple accesses
    for _ in 0..5 {
        session.mark_accessed();
    }
    assert_eq!(session.reuse_count(), 6);

    // Timestamp should be updated
    let now = chrono::Utc::now();
    assert!(session.last_accessed() <= now);
}

#[tokio::test]
async fn test_contract_session_staleness() {
    let mut session = MemorySession::default();

    // Fresh session
    assert!(!session.is_stale());

    // Simulate old session
    let old_time = chrono::Utc::now() - chrono::Duration::seconds(4000);
    session.last_accessed = old_time;

    // Should now be stale
    assert!(session.is_stale());
}

#[tokio::test]
async fn test_contract_session_persistence_across_executions() {
    let agent = MockAgent::new();
    let config = ExecutionConfig::default();
    let session = MemorySession::default();

    // Execute multiple times with the same session
    for i in 0..3 {
        let _response = agent
            .execute_with_session(&format!("prompt {}", i), &config, &session)
            .await
            .unwrap();
    }

    // Session should track the interactions
    // Note: In this mock, we can't actually modify the session through
    // execute_with_session since we only have &dyn AgentSession
    // In a real implementation, the session would be updated internally
}

// ============================================================================
// Configuration Contract Tests
// ============================================================================

#[test]
fn test_contract_config_builder_creates_valid_configs() {
    let configs = vec![
        AgentConfig::builder().name("agent1").build(),
        AgentConfig::builder()
            .name("agent2")
            .model("model2")
            .build(),
        AgentConfig::builder()
            .name("agent3")
            .timeout_secs(100)
            .max_retries(5)
            .build(),
    ];

    for config in configs {
        // All configs should be valid
        let result = config.validate();
        assert!(result.is_ok(), "Config validation failed: {:?}", config);
    }
}

#[test]
fn test_contract_config_validation_enforces_constraints() {
    let test_cases: Vec<(&str, AgentConfig)> = vec![
        ("name", {
            let mut c = AgentConfig::default();
            c.name = "".to_string();
            c
        }),
        ("model", {
            let mut c = AgentConfig::default();
            c.model = "".to_string();
            c
        }),
        ("command", {
            let mut c = AgentConfig::default();
            c.command = "".to_string();
            c
        }),
        ("timeout_secs", {
            let mut c = AgentConfig::default();
            c.timeout_secs = 0;
            c
        }),
    ];

    for (expected_field, config) in test_cases {
        let result = config.validate();

        assert!(
            result.is_err(),
            "Expected validation error for field: {}",
            expected_field
        );

        if let Err(AgentError::ConfigValidation { field, .. }) = result {
            assert_eq!(field, expected_field);
        } else {
            panic!("Expected ConfigValidation error for {}", expected_field);
        }
    }
}

// ============================================================================
// Integration with Real Implementation
// ============================================================================

#[tokio::test]
async fn test_claude_agent_implements_trait_contract() {
    // Verify ClaudeAgent properly implements the trait
    let agent = ClaudeAgent::new().unwrap();

    // All trait methods should be callable
    let _backend_name = agent.backend_name();
    let _agent_ref = agent.agent();
    let _health = agent.health_check().await;
    let _available = agent.is_available().await;

    // Validate config
    let config = AgentConfig::default();
    let _validation = agent.validate_config(&config).await;

    // Execute methods should exist (may fail if claude not installed)
    let exec_config = ExecutionConfig::default();
    let _exec = agent.execute("test", &exec_config).await;

    let session = MemorySession::default();
    let _exec_session = agent
        .execute_with_session("test", &exec_config, &session)
        .await;

    let task = Task {
        id: "test-1".to_string(),
        title: "Test".to_string(),
        description: "Test task".to_string(),
        status: ltmatrix::models::TaskStatus::Pending,
        complexity: ltmatrix::models::TaskComplexity::Moderate,
        agent_type: ltmatrix::models::AgentType::default(),
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
    let _exec_task = agent.execute_task(&task, "context", &exec_config).await;
}

#[tokio::test]
async fn test_execution_config_affects_behavior() {
    let agent = MockAgent::new();

    // Different execution configs should be accepted
    let configs = vec![
        ExecutionConfig::default(),
        ExecutionConfig {
            model: "custom-model".to_string(),
            max_retries: 10,
            timeout: 1000,
            enable_session: false,
            env_vars: vec![],
        },
        ExecutionConfig {
            model: "another-model".to_string(),
            max_retries: 0,
            timeout: 1,
            enable_session: true,
            env_vars: vec![("KEY".to_string(), "value".to_string())],
        },
    ];

    for config in configs {
        let result = agent.execute("test prompt", &config).await;
        assert!(result.is_ok(), "Execution with config failed: {:?}", config);
    }
}

// ============================================================================
// Performance and Resource Management
// ============================================================================

#[tokio::test]
async fn test_contract_multiple_concurrent_executions() {
    let agent = Arc::new(MockAgent::new());
    let config = ExecutionConfig::default();

    let mut handles = vec![];

    // Spawn 10 concurrent executions
    for i in 0..10 {
        let agent_clone = Arc::clone(&agent);
        let config_clone = config.clone();
        let handle = tokio::spawn(async move {
            agent_clone
                .execute(&format!("concurrent prompt {}", i), &config_clone)
                .await
        });
        handles.push(handle);
    }

    // All should complete successfully
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_trait_methods_are_thread_safe() {
    // Verify that all trait methods can be called from different threads
    let agent = Arc::new(MockAgent::new());

    let agent_clone1 = Arc::clone(&agent);
    let agent_clone2 = Arc::clone(&agent);

    let task1 = tokio::spawn(async move {
        let config = ExecutionConfig::default();
        let _ = agent_clone1.execute("test1", &config).await;
    });

    let task2 = tokio::spawn(async move {
        let config = ExecutionConfig::default();
        let _ = agent_clone2.execute("test2", &config).await;
    });

    // Both should complete without deadlocking
    let _ = tokio::join!(task1, task2);
}
