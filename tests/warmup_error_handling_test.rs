//! Error handling tests for warmup executor
//!
//! These tests verify error scenarios and recovery behavior:
//! - Agent unavailable errors
//! - Timeout errors
//! - Invalid response errors
//! - Network errors
//! - Error recovery and retry
//! - Error reporting and logging

use ltmatrix::agent::warmup::{WarmupExecutor, WarmupResult};
use ltmatrix::agent::{AgentSession, pool::SessionPool};
use ltmatrix::agent::backend::{AgentBackend, AgentConfig, AgentResponse, ExecutionConfig};
use ltmatrix::config::settings::WarmupConfig;
use ltmatrix::models::Agent;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

// ============================================================================
// Mock Agents for Error Testing
// ============================================================================

/// Mock agent that simulates being unavailable
struct UnavailableAgent {
    agent: Agent,
    is_available: Arc<AtomicBool>,
}

impl UnavailableAgent {
    fn new(name: &str, available: bool) -> Self {
        Self {
            agent: Agent::new(name, name, "test-model", 3600),
            is_available: Arc::new(AtomicBool::new(available)),
        }
    }
}

#[async_trait::async_trait]
impl AgentBackend for UnavailableAgent {
    async fn execute(&self, _prompt: &str, _config: &ExecutionConfig) -> anyhow::Result<AgentResponse> {
        if !self.is_available.load(Ordering::SeqCst) {
            anyhow::bail!("Agent is currently unavailable");
        }
        Ok(AgentResponse {
            output: "Ready".to_string(),
            ..Default::default()
        })
    }

    async fn execute_with_session(
        &self,
        _prompt: &str,
        _config: &ExecutionConfig,
        _session: &dyn AgentSession,
    ) -> anyhow::Result<AgentResponse> {
        if !self.is_available.load(Ordering::SeqCst) {
            anyhow::bail!("Agent is currently unavailable");
        }
        Ok(AgentResponse {
            output: "Ready".to_string(),
            ..Default::default()
        })
    }

    async fn execute_task(
        &self,
        _task: &ltmatrix::models::Task,
        _context: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        Ok(AgentResponse::default())
    }

    async fn is_available(&self) -> bool {
        self.is_available.load(Ordering::SeqCst)
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        Ok(self.is_available.load(Ordering::SeqCst))
    }

    async fn validate_config(&self, _config: &AgentConfig) -> Result<(), ltmatrix::agent::backend::AgentError> {
        Ok(())
    }

    fn agent(&self) -> &Agent {
        &self.agent
    }
}

/// Mock agent that returns errors in responses
struct ErrorReturningAgent {
    agent: Agent,
    return_error: bool,
}

impl ErrorReturningAgent {
    fn new(name: &str, return_error: bool) -> Self {
        Self {
            agent: Agent::new(name, name, "test-model", 3600),
            return_error,
        }
    }
}

#[async_trait::async_trait]
impl AgentBackend for ErrorReturningAgent {
    async fn execute(&self, _prompt: &str, _config: &ExecutionConfig) -> anyhow::Result<AgentResponse> {
        Ok(AgentResponse {
            output: if self.return_error {
                String::new()
            } else {
                "Ready".to_string()
            },
            error: if self.return_error {
                Some("Simulated agent error".to_string())
            } else {
                None
            },
            ..Default::default()
        })
    }

    async fn execute_with_session(
        &self,
        _prompt: &str,
        _config: &ExecutionConfig,
        _session: &dyn AgentSession,
    ) -> anyhow::Result<AgentResponse> {
        Ok(AgentResponse {
            output: if self.return_error {
                String::new()
            } else {
                "Ready".to_string()
            },
            error: if self.return_error {
                Some("Simulated agent error".to_string())
            } else {
                None
            },
            ..Default::default()
        })
    }

    async fn execute_task(
        &self,
        _task: &ltmatrix::models::Task,
        _context: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        Ok(AgentResponse::default())
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        Ok(true)
    }

    async fn validate_config(&self, _config: &AgentConfig) -> Result<(), ltmatrix::agent::backend::AgentError> {
        Ok(())
    }

    fn agent(&self) -> &Agent {
        &self.agent
    }
}

/// Mock agent that returns empty responses
struct EmptyResponseAgent {
    agent: Agent,
    return_empty: bool,
}

impl EmptyResponseAgent {
    fn new(name: &str, return_empty: bool) -> Self {
        Self {
            agent: Agent::new(name, name, "test-model", 3600),
            return_empty,
        }
    }
}

#[async_trait::async_trait]
impl AgentBackend for EmptyResponseAgent {
    async fn execute(&self, _prompt: &str, _config: &ExecutionConfig) -> anyhow::Result<AgentResponse> {
        Ok(AgentResponse {
            output: if self.return_empty {
                String::new()
            } else {
                "Ready".to_string()
            },
            ..Default::default()
        })
    }

    async fn execute_with_session(
        &self,
        _prompt: &str,
        _config: &ExecutionConfig,
        _session: &dyn AgentSession,
    ) -> anyhow::Result<AgentResponse> {
        Ok(AgentResponse {
            output: if self.return_empty {
                String::new()
            } else {
                "Ready".to_string()
            },
            ..Default::default()
        })
    }

    async fn execute_task(
        &self,
        _task: &ltmatrix::models::Task,
        _context: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        Ok(AgentResponse::default())
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        Ok(true)
    }

    async fn validate_config(&self, _config: &AgentConfig) -> Result<(), ltmatrix::agent::backend::AgentError> {
        Ok(())
    }

    fn agent(&self) -> &Agent {
        &self.agent
    }
}

/// Mock agent that fails intermittently
struct IntermittentFailureAgent {
    agent: Agent,
    failure_count: Arc<AtomicU32>,
    total_failures: u32,
}

impl IntermittentFailureAgent {
    fn new(name: &str, total_failures: u32) -> Self {
        Self {
            agent: Agent::new(name, name, "test-model", 3600),
            failure_count: Arc::new(AtomicU32::new(0)),
            total_failures,
        }
    }
}

#[async_trait::async_trait]
impl AgentBackend for IntermittentFailureAgent {
    async fn execute(&self, _prompt: &str, _config: &ExecutionConfig) -> anyhow::Result<AgentResponse> {
        let current_failures = self.failure_count.fetch_add(1, Ordering::SeqCst);
        if current_failures < self.total_failures {
            anyhow::bail!("Simulated intermittent failure (attempt {})", current_failures + 1);
        }
        Ok(AgentResponse {
            output: "Ready".to_string(),
            ..Default::default()
        })
    }

    async fn execute_with_session(
        &self,
        _prompt: &str,
        _config: &ExecutionConfig,
        _session: &dyn AgentSession,
    ) -> anyhow::Result<AgentResponse> {
        let current_failures = self.failure_count.fetch_add(1, Ordering::SeqCst);
        if current_failures < self.total_failures {
            anyhow::bail!("Simulated intermittent failure (attempt {})", current_failures + 1);
        }
        Ok(AgentResponse {
            output: "Ready".to_string(),
            ..Default::default()
        })
    }

    async fn execute_task(
        &self,
        _task: &ltmatrix::models::Task,
        _context: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        Ok(AgentResponse::default())
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        Ok(true)
    }

    async fn validate_config(&self, _config: &AgentConfig) -> Result<(), ltmatrix::agent::backend::AgentError> {
        Ok(())
    }

    fn agent(&self) -> &Agent {
        &self.agent
    }
}

// ============================================================================
// Agent Unavailable Error Tests
// ============================================================================

#[tokio::test]
async fn warmup_fails_when_agent_unavailable() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = UnavailableAgent::new("unavailable-agent", false);

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_failed());
    assert_eq!(result.queries_executed(), Some(0));

    if let WarmupResult::Failed { error, .. } = result {
        assert!(error.contains("unavailable") || error.contains("failed"),
                "Error message should indicate unavailability: {}", error);
    }
}

#[tokio::test]
async fn warmup_succeeds_when_agent_becomes_available() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = UnavailableAgent::new("becoming-available", true);

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_success());
    assert_eq!(result.queries_executed(), Some(1));
}

// ============================================================================
// Error Response Tests
// ============================================================================

#[tokio::test]
async fn warmup_fails_on_error_response() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = ErrorReturningAgent::new("error-agent", true);

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_failed());
    assert_eq!(result.queries_executed(), Some(0));

    if let WarmupResult::Failed { error, .. } = result {
        assert!(error.contains("error") || error.contains("failed"),
                "Error should mention response error: {}", error);
    }
}

#[tokio::test]
async fn warmup_fails_on_empty_response() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = EmptyResponseAgent::new("empty-response-agent", true);

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_failed());
    assert_eq!(result.queries_executed(), Some(0));

    if let WarmupResult::Failed { error, .. } = result {
        assert!(error.contains("empty") || error.contains("response"),
                "Error should mention empty response: {}", error);
    }
}

// ============================================================================
// Intermittent Failure Tests
// ============================================================================

#[tokio::test]
async fn warmup_handles_intermittent_failures_with_retry() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: true, // Enable retry
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    // Agent will fail 2 times, then succeed on 3rd try
    let agent = IntermittentFailureAgent::new("intermittent-agent", 2);

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_success());
    assert_eq!(result.queries_executed(), Some(1));
}

#[tokio::test]
async fn warmup_fails_after_max_retries_exhausted() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: true,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    // Agent will fail 5 times (more than MAX_WARMUP_RETRIES=2)
    let agent = IntermittentFailureAgent::new("persistent-failure-agent", 5);

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_failed());
    assert_eq!(result.queries_executed(), Some(0));

    if let WarmupResult::Failed { error, .. } = result {
        assert!(error.contains("retry") || error.contains("failed"),
                "Error should mention retry exhaustion: {}", error);
    }
}

// ============================================================================
// Multiple Agent Error Handling Tests
// ============================================================================

#[tokio::test]
async fn warmup_multiple_agents_with_mixed_results() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();

    let agent1 = UnavailableAgent::new("working-agent", true);
    let agent2 = UnavailableAgent::new("failing-agent", false);
    let agent3 = EmptyResponseAgent::new("empty-agent", true);

    let backends: Vec<&dyn AgentBackend> = vec![&agent1, &agent2, &agent3];
    let results = executor.warmup_agents(&backends, &mut pool).await;

    assert_eq!(results.len(), 3);
    assert!(results[0].is_success(), "First agent should succeed");
    assert!(results[1].is_failed(), "Second agent should fail");
    assert!(results[2].is_failed(), "Third agent should fail");
}

// ============================================================================
// Error Message Quality Tests
// ============================================================================

#[tokio::test]
async fn warmup_error_messages_are_descriptive() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = UnavailableAgent::new("test-agent", false);

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    if let WarmupResult::Failed { error, queries_executed } = result {
        assert!(!error.is_empty(), "Error message should not be empty");
        assert!(error.len() > 10, "Error message should be descriptive");
        assert_eq!(queries_executed, 0, "No queries should have executed");
    } else {
        panic!("Expected Failed result");
    }
}

#[tokio::test]
async fn warmup_error_includes_queries_executed_count() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 3,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    // Agent that fails first query (no retry configured)
    let agent = IntermittentFailureAgent::new("partial-failure-agent", 1);

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    // Since retry is disabled and agent fails first query, should have 0 executed queries
    if let WarmupResult::Failed { queries_executed, .. } = result {
        assert_eq!(queries_executed, 0, "Should have 0 executed queries when first attempt fails and retry disabled");
    }
}

// ============================================================================
// Graceful Degradation Tests
// ============================================================================

#[tokio::test]
async fn warmup_continues_after_single_agent_failure() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();

    let agent1 = UnavailableAgent::new("working-agent", true);
    let agent2 = UnavailableAgent::new("failing-agent", false);
    let agent3 = UnavailableAgent::new("another-working-agent", true);

    let backends: Vec<&dyn AgentBackend> = vec![&agent1, &agent2, &agent3];
    let results = executor.warmup_agents(&backends, &mut pool).await;

    // Should have results for all agents, even if some failed
    assert_eq!(results.len(), 3);
    assert!(results[0].is_success());
    assert!(results[1].is_failed());
    assert!(results[2].is_success());

    // Pool should have sessions for the successful agents
    assert!(pool.len() >= 1, "Pool should have at least one session");
}

// ============================================================================
// Check Agent Available Error Tests
// ============================================================================

#[tokio::test]
async fn check_agent_available_returns_false_for_unavailable() {
    let executor = WarmupExecutor::default();
    let agent = UnavailableAgent::new("unavailable", false);

    let available = executor.check_agent_available(&agent).await.unwrap();
    assert!(!available);
}

#[tokio::test]
async fn check_agent_available_returns_true_for_available() {
    let executor = WarmupExecutor::default();
    let agent = UnavailableAgent::new("available", true);

    let available = executor.check_agent_available(&agent).await.unwrap();
    assert!(available);
}
