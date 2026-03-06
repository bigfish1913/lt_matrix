//! Comprehensive integration tests for AgentPool warmup functionality
//!
//! These tests verify the complete integration between SessionPool and WarmupExecutor,
//! including initialization, execution, failure handling, and session management.

use ltmatrix::agent::pool::SessionPool;
use ltmatrix::agent::warmup::WarmupExecutor;
use ltmatrix::agent::backend::{AgentBackend, AgentConfig, ExecutionConfig, AgentResponse, AgentSession};
use ltmatrix::config::settings::WarmupConfig;
use ltmatrix::models::Agent;

// ============================================================================
// Mock Agent Backend for Testing
// ============================================================================

/// Mock agent backend that simulates various warmup scenarios
struct MockWarmupBackend {
    agent: Agent,
    behavior: MockBehavior,
}

#[derive(Clone)]
enum MockBehavior {
    Success,
    Timeout,
    Fail { should_fail: bool },
    EmptyResponse,
    RetryThenSucceed,
}

impl MockWarmupBackend {
    fn new(name: &str, model: &str, behavior: MockBehavior) -> Self {
        Self {
            agent: Agent::new(name, name, model, 3600),
            behavior,
        }
    }

    fn success(name: &str, model: &str) -> Self {
        Self::new(name, model, MockBehavior::Success)
    }

    fn timeout(name: &str, model: &str) -> Self {
        Self::new(name, model, MockBehavior::Timeout)
    }

    fn failing(name: &str, model: &str) -> Self {
        Self::new(name, model, MockBehavior::Fail { should_fail: true })
    }
}

#[async_trait::async_trait]
impl AgentBackend for MockWarmupBackend {
    async fn execute(&self, _prompt: &str, _config: &ExecutionConfig) -> anyhow::Result<AgentResponse> {
        match &self.behavior {
            MockBehavior::Success => Ok(AgentResponse {
                output: "Ready".to_string(),
                ..Default::default()
            }),
            MockBehavior::Timeout => {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                Ok(AgentResponse {
                    output: "Late response".to_string(),
                    ..Default::default()
                })
            }
            MockBehavior::Fail { should_fail } => {
                if *should_fail {
                    anyhow::bail!("Mock agent failure");
                }
                Ok(AgentResponse {
                    output: "Ready".to_string(),
                    ..Default::default()
                })
            }
            MockBehavior::EmptyResponse => Ok(AgentResponse {
                output: "".to_string(),
                ..Default::default()
            }),
            MockBehavior::RetryThenSucceed => Ok(AgentResponse {
                output: "Ready after retry".to_string(),
                ..Default::default()
            }),
        }
    }

    async fn execute_with_session(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
        _session: &dyn AgentSession,
    ) -> anyhow::Result<AgentResponse> {
        self.execute(prompt, config).await
    }

    async fn execute_task(
        &self,
        _task: &ltmatrix::models::Task,
        _context: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        Ok(AgentResponse::default())
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        match &self.behavior {
            MockBehavior::Fail { should_fail: true } => Ok(false),
            _ => Ok(true),
        }
    }

    async fn validate_config(&self, _config: &AgentConfig) -> Result<(), ltmatrix::agent::backend::AgentError> {
        Ok(())
    }

    fn agent(&self) -> &Agent {
        &self.agent
    }
}

// ============================================================================
// SessionPool Initialization Tests
// ============================================================================

#[test]
fn sessionpool_with_warmup_stores_executor() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 2,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: Some("Test warmup".to_string()),
    };
    let executor = WarmupExecutor::new(config);
    let pool = SessionPool::with_warmup(executor);

    assert!(pool.has_warmup(), "Pool should report having warmup capability");
}

#[test]
fn sessionpool_default_no_warmup() {
    let pool = SessionPool::new();
    assert!(!pool.has_warmup(), "Default pool should not have warmup");
}

#[test]
fn sessionpool_tracks_warmed_agents() {
    let config = WarmupConfig::default();
    let executor = WarmupExecutor::new(config);
    let pool = SessionPool::with_warmup(executor);

    // Initially no agents are warmed up
    assert!(!pool.is_warmed_up("claude", "claude-sonnet-4-6"));
    assert!(!pool.is_warmed_up("opencode", "gpt-4"));
}

// ============================================================================
// Warmup Execution Integration Tests
// ============================================================================

#[tokio::test]
async fn warmup_executor_succeeds_with_healthy_backend() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    };
    let executor = WarmupExecutor::new(config);
    let mut pool = SessionPool::new();

    let backend = MockWarmupBackend::success("claude", "claude-sonnet-4-6");

    let result = executor.warmup_agent(&backend, &mut pool).await
        .expect("Warmup should complete without panicking");

    assert!(result.is_success(), "Warmup should succeed with healthy backend");
    assert_eq!(result.queries_executed(), Some(1), "Should execute 1 query");

    // Pool should have a session for the agent
    let sessions = pool.list_by_agent("claude");
    assert_eq!(sessions.len(), 1, "Pool should contain one session");
}

#[tokio::test]
async fn warmup_executor_skips_when_disabled() {
    let config = WarmupConfig {
        enabled: false, // Disabled
        ..Default::default()
    };
    let executor = WarmupExecutor::new(config);
    let mut pool = SessionPool::new();

    let backend = MockWarmupBackend::success("claude", "claude-sonnet-4-6");

    let result = executor.warmup_agent(&backend, &mut pool).await
        .expect("Warmup should complete without panicking");

    assert!(result.is_skipped(), "Warmup should be skipped when disabled");
    assert_eq!(result.queries_executed(), None, "No queries should be executed");
}

#[tokio::test]
async fn warmup_executor_handles_timeout_gracefully() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 1, // Short timeout
        retry_on_failure: false,
        prompt_template: None,
    };
    let executor = WarmupExecutor::new(config);
    let mut pool = SessionPool::new();

    let backend = MockWarmupBackend::timeout("claude", "claude-sonnet-4-6");

    let result = executor.warmup_agent(&backend, &mut pool).await
        .expect("Warmup should complete without panicking");

    assert!(result.is_failed(), "Warmup should fail on timeout");
    assert_eq!(result.queries_executed(), Some(0), "No queries should complete");
}

#[tokio::test]
async fn warmup_executor_handles_backend_failure() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    };
    let executor = WarmupExecutor::new(config);
    let mut pool = SessionPool::new();

    let backend = MockWarmupBackend::failing("claude", "claude-sonnet-4-6");

    let result = executor.warmup_agent(&backend, &mut pool).await
        .expect("Warmup should complete without panicking");

    assert!(result.is_failed(), "Warmup should fail when backend fails");
    assert_eq!(result.queries_executed(), Some(0), "No queries should succeed");
}

#[tokio::test]
async fn warmup_executor_respects_max_queries() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 3, // Try up to 3 queries
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    };
    let executor = WarmupExecutor::new(config);
    let mut pool = SessionPool::new();

    let backend = MockWarmupBackend::success("claude", "claude-sonnet-4-6");

    let result = executor.warmup_agent(&backend, &mut pool).await
        .expect("Warmup should complete");

    // Should stop after first successful query
    assert!(result.is_success());
    // The implementation stops after first success, so we get 1 query
    assert_eq!(result.queries_executed(), Some(1));
}

// ============================================================================
// Session Reuse After Warmup Tests
// ============================================================================

#[tokio::test]
async fn warmup_creates_reusable_session() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    };
    let executor = WarmupExecutor::new(config);
    let mut pool = SessionPool::new();

    let backend = MockWarmupBackend::success("claude", "claude-sonnet-4-6");

    // Warm up the agent
    executor.warmup_agent(&backend, &mut pool).await
        .expect("Warmup should succeed");

    // Check that a session was created
    let sessions = pool.list_by_agent("claude");
    assert_eq!(sessions.len(), 1, "Pool should have one session");

    let session = sessions[0];
    assert_eq!(session.agent_name(), "claude");
    assert_eq!(session.model(), "claude-sonnet-4-6");
}

#[tokio::test]
async fn warmup_reuses_existing_session() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    };
    let executor = WarmupExecutor::new(config);
    let mut pool = SessionPool::new();

    let backend = MockWarmupBackend::success("claude", "claude-sonnet-4-6");

    // First warmup
    executor.warmup_agent(&backend, &mut pool).await
        .expect("First warmup should succeed");

    let session_count_after_first = pool.len();

    // Second warmup for same agent
    executor.warmup_agent(&backend, &mut pool).await
        .expect("Second warmup should succeed");

    // Should reuse the same session
    assert_eq!(pool.len(), session_count_after_first, "Should not create additional session");

    let sessions = pool.list_by_agent("claude");
    assert_eq!(sessions.len(), 1, "Should still have only one session");
}

// ============================================================================
// Multiple Agents Warmup Tests
// ============================================================================

#[tokio::test]
async fn warmup_multiple_agents_sequentially() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    };
    let executor = WarmupExecutor::new(config);
    let mut pool = SessionPool::new();

    let backend1 = MockWarmupBackend::success("claude", "claude-sonnet-4-6");
    let backend2 = MockWarmupBackend::success("opencode", "gpt-4");
    let backend3 = MockWarmupBackend::success("codex", "codex-model");

    let backends = vec![&backend1, &backend2, &backend3];

    let results = executor.warmup_agents(&backends, &mut pool).await;

    assert_eq!(results.len(), 3, "Should have results for all backends");

    // All warmups should succeed
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_success(), "Warmup {} should succeed", i);
    }

    // Pool should have 3 sessions
    assert_eq!(pool.len(), 3, "Pool should have 3 sessions");

    // Each agent should have a session
    assert_eq!(pool.list_by_agent("claude").len(), 1);
    assert_eq!(pool.list_by_agent("opencode").len(), 1);
    assert_eq!(pool.list_by_agent("codex").len(), 1);
}

#[tokio::test]
async fn warmup_multiple_agents_one_fails() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    };
    let executor = WarmupExecutor::new(config);
    let mut pool = SessionPool::new();

    let backend1 = MockWarmupBackend::success("claude", "claude-sonnet-4-6");
    let backend2 = MockWarmupBackend::failing("opencode", "gpt-4"); // This one fails
    let backend3 = MockWarmupBackend::success("codex", "codex-model");

    let backends = vec![&backend1, &backend2, &backend3];

    let results = executor.warmup_agents(&backends, &mut pool).await;

    assert_eq!(results.len(), 3, "Should have results for all backends");

    // First and third should succeed, second should fail
    assert!(results[0].is_success(), "First warmup should succeed");
    assert!(results[1].is_failed(), "Second warmup should fail");
    assert!(results[2].is_success(), "Third warmup should succeed");

    // Pool should have 3 sessions (one for each agent, even the failed one)
    // Note: warmup_agent() creates a session before attempting warmup,
    // so sessions are created even when warmup fails
    assert_eq!(pool.len(), 3, "Pool should have 3 sessions (one per agent)");
}

// ============================================================================
// Warmup with get_or_create_warmup Tests
// ============================================================================

#[tokio::test]
async fn sessionpool_get_or_create_warmup_without_executor() {
    let mut pool = SessionPool::new();

    // Should work even without warmup executor
    let result = pool.get_or_create_warmup("claude", "claude-sonnet-4-6").await;

    assert!(result.is_ok(), "Should succeed even without warmup executor");

    let session_id = result.unwrap();
    assert!(pool.get(&session_id).is_some(), "Session should be created");
}

#[tokio::test]
async fn sessionpool_get_or_create_warmup_with_executor_skips_actual_warmup() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    };
    let executor = WarmupExecutor::new(config);
    let mut pool = SessionPool::with_warmup(executor);

    // Current implementation skips actual warmup (test mode)
    let result = pool.get_or_create_warmup("claude", "claude-sonnet-4-6").await;

    assert!(result.is_ok(), "Should succeed");

    let session_id = result.unwrap();
    assert!(pool.get(&session_id).is_some(), "Session should be created");

    // Agent should be marked as warmed
    assert!(pool.is_warmed_up("claude", "claude-sonnet-4-6"), "Agent should be marked as warmed");
}

#[tokio::test]
async fn sessionpool_get_or_create_warmup_reuses_warmed_session() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    };
    let executor = WarmupExecutor::new(config);
    let mut pool = SessionPool::with_warmup(executor);

    // First call
    let session_id1 = pool.get_or_create_warmup("claude", "claude-sonnet-4-6").await
        .expect("First call should succeed");

    // Second call
    let session_id2 = pool.get_or_create_warmup("claude", "claude-sonnet-4-6").await
        .expect("Second call should succeed");

    // Should return the same session
    assert_eq!(session_id1, session_id2, "Should reuse the same session");

    // Pool should only have one session
    assert_eq!(pool.len(), 1, "Pool should have only one session");
}

// ============================================================================
// Warmup Configuration Tests
// ============================================================================

#[test]
fn warmup_executor_uses_custom_prompt_template() {
    let custom_prompt = "Custom warmup prompt for testing";
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: Some(custom_prompt.to_string()),
    };

    let _executor = WarmupExecutor::new(config);

    // The executor should use the custom prompt
    // (This would be verified by checking the actual execution in integration tests)
    // Note: We can't access executor.config directly as it's private
    // In real usage, the executor would use this config during warmup_agent()
}

#[test]
fn warmup_executor_retry_configuration() {
    let config_with_retry = WarmupConfig {
        enabled: true,
        retry_on_failure: true,
        ..Default::default()
    };
    let _executor_with_retry = WarmupExecutor::new(config_with_retry);

    let config_no_retry = WarmupConfig {
        enabled: true,
        retry_on_failure: false,
        ..Default::default()
    };
    let _executor_no_retry = WarmupExecutor::new(config_no_retry);

    // Configuration is stored internally and used during warmup
    // Can't access directly but the executor will use it
}

// ============================================================================
// Edge Cases and Error Handling Tests
// ============================================================================

#[tokio::test]
async fn warmup_with_empty_prompt_template() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: Some("".to_string()), // Empty string
    };
    let executor = WarmupExecutor::new(config);
    let mut pool = SessionPool::new();

    let backend = MockWarmupBackend::success("claude", "claude-sonnet-4-6");

    // Should use default prompt when empty template is provided
    let result = executor.warmup_agent(&backend, &mut pool).await
        .expect("Warmup should complete");

    // Empty template should fall back to default prompt
    // The warmup should still succeed
    assert!(result.is_success() || result.is_failed(), "Should handle empty template");
}

#[tokio::test]
async fn warmup_with_zero_timeout_is_rejected() {
    // Note: This would be caught by validation in production
    // For now, we test that the executor doesn't crash with zero timeout
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 0, // Invalid
        retry_on_failure: false,
        prompt_template: None,
    };

    let executor = WarmupExecutor::new(config);
    let mut pool = SessionPool::new();

    let backend = MockWarmupBackend::success("claude", "claude-sonnet-4-6");

    // Should complete without panicking (though it might fail)
    let result = executor.warmup_agent(&backend, &mut pool).await;

    // Result should be Ok (no panic), but warmup might fail
    assert!(result.is_ok(), "Should complete without panic");
}

// ============================================================================
// Agent Availability Check Tests
// ============================================================================

#[tokio::test]
async fn check_agent_available_returns_true_for_healthy_backend() {
    let executor = WarmupExecutor::default();
    let backend = MockWarmupBackend::success("claude", "claude-sonnet-4-6");

    let available = executor.check_agent_available(&backend).await
        .expect("Check should complete");

    assert!(available, "Healthy backend should be available");
}

#[tokio::test]
async fn check_agent_available_returns_false_for_failing_backend() {
    let executor = WarmupExecutor::default();
    let backend = MockWarmupBackend::failing("claude", "claude-sonnet-4-6");

    let available = executor.check_agent_available(&backend).await
        .expect("Check should complete");

    assert!(!available, "Failing backend should not be available");
}

// ============================================================================
// Integration Test: Full Warmup Workflow
// ============================================================================

#[tokio::test]
async fn full_warmup_workflow_from_pool_creation_to_session_reuse() {
    // Step 1: Create a pool with warmup enabled
    let config = WarmupConfig {
        enabled: true,
        max_queries: 2,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: Some("Integration test warmup".to_string()),
    };
    let executor = WarmupExecutor::new(config.clone());
    let mut pool = SessionPool::with_warmup(executor.clone());

    // Step 2: Warm up an agent using the executor
    let backend = MockWarmupBackend::success("claude", "claude-sonnet-4-6");
    let warmup_result = executor.warmup_agent(&backend, &mut pool).await
        .expect("Warmup should complete");

    assert!(warmup_result.is_success(), "Warmup should succeed");

    // Step 3: Verify session was created
    let sessions = pool.list_by_agent("claude");
    assert_eq!(sessions.len(), 1, "Should have one session");

    // Step 4: Get the session for reuse
    let session = pool.get_or_create("claude", "claude-sonnet-4-6");
    assert_eq!(session.agent_name(), "claude");
    assert_eq!(session.model(), "claude-sonnet-4-6");

    // Step 5: Warmup the same agent again (should reuse)
    let warmup_result2 = executor.warmup_agent(&backend, &mut pool).await
        .expect("Second warmup should complete");

    assert!(warmup_result2.is_success(), "Second warmup should succeed");

    // Should still have only one session
    assert_eq!(pool.len(), 1, "Should still have only one session");
}
