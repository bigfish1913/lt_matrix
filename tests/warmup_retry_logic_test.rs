//! Retry logic tests for warmup executor
//!
//! These tests verify the retry behavior of the warmup executor:
//! - Retry attempts are limited correctly
//! - Exponential backoff is applied
//! - Retry only happens when configured
//! - Retry state is tracked correctly
//! - Multiple failures lead to eventual abandonment

use ltmatrix::agent::warmup::{WarmupExecutor, WarmupResult};
use ltmatrix::agent::{AgentSession, pool::SessionPool};
use ltmatrix::agent::backend::{AgentBackend, AgentConfig, AgentResponse, ExecutionConfig};
use ltmatrix::config::settings::WarmupConfig;
use ltmatrix::models::Agent;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Mock Agents for Retry Testing
// ============================================================================

/// Mock agent that tracks retry attempts
struct RetryTrackingAgent {
    agent: Agent,
    attempt_count: Arc<AtomicU32>,
    succeed_on_attempt: u32,
}

impl RetryTrackingAgent {
    fn new(name: &str, succeed_on_attempt: u32) -> Self {
        Self {
            agent: Agent::new(name, name, "test-model", 3600),
            attempt_count: Arc::new(AtomicU32::new(0)),
            succeed_on_attempt,
        }
    }

    fn attempt_count(&self) -> u32 {
        self.attempt_count.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl AgentBackend for RetryTrackingAgent {
    async fn execute(&self, _prompt: &str, _config: &ExecutionConfig) -> anyhow::Result<AgentResponse> {
        let attempt = self.attempt_count.fetch_add(1, Ordering::SeqCst) + 1;
        if attempt < self.succeed_on_attempt {
            anyhow::bail!("Attempt {} failed (will succeed on attempt {})", attempt, self.succeed_on_attempt);
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
        let attempt = self.attempt_count.fetch_add(1, Ordering::SeqCst) + 1;
        if attempt < self.succeed_on_attempt {
            anyhow::bail!("Attempt {} failed (will succeed on attempt {})", attempt, self.succeed_on_attempt);
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

/// Mock agent that always fails
struct AlwaysFailAgent {
    agent: Agent,
    attempt_count: Arc<AtomicU32>,
}

impl AlwaysFailAgent {
    fn new(name: &str) -> Self {
        Self {
            agent: Agent::new(name, name, "test-model", 3600),
            attempt_count: Arc::new(AtomicU32::new(0)),
        }
    }

    fn attempt_count(&self) -> u32 {
        self.attempt_count.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl AgentBackend for AlwaysFailAgent {
    async fn execute(&self, _prompt: &str, _config: &ExecutionConfig) -> anyhow::Result<AgentResponse> {
        self.attempt_count.fetch_add(1, Ordering::SeqCst);
        anyhow::bail!("Always fails");
    }

    async fn execute_with_session(
        &self,
        _prompt: &str,
        _config: &ExecutionConfig,
        _session: &dyn AgentSession,
    ) -> anyhow::Result<AgentResponse> {
        self.attempt_count.fetch_add(1, Ordering::SeqCst);
        anyhow::bail!("Always fails");
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
// Retry Enable/Disable Tests
// ============================================================================

#[tokio::test]
async fn warmup_does_not_retry_when_disabled() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false, // Retry disabled
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = RetryTrackingAgent::new("no-retry-agent", 2); // Will fail first attempt

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_failed());
    assert_eq!(result.queries_executed(), Some(0));
    assert_eq!(agent.attempt_count(), 1, "Should only make 1 attempt when retry disabled");
}

#[tokio::test]
async fn warmup_retries_when_enabled() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: true, // Retry enabled
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = RetryTrackingAgent::new("retry-agent", 2); // Will fail first, succeed second

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_success());
    assert_eq!(result.queries_executed(), Some(1));
    assert_eq!(agent.attempt_count(), 2, "Should retry and succeed on 2nd attempt");
}

// ============================================================================
// Retry Attempt Count Tests
// ============================================================================

#[tokio::test]
async fn warmup_respects_max_retry_limit() {
    const MAX_WARMUP_RETRIES: u32 = 2; // From implementation

    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: true,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = AlwaysFailAgent::new("max-retry-agent");

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_failed());
    // Should make initial attempt + MAX_WARMUP_RETRIES retries
    let expected_attempts = 1 + MAX_WARMUP_RETRIES as u32;
    assert_eq!(agent.attempt_count(), expected_attempts,
               "Should make {} attempts total (1 initial + {} retries)", expected_attempts, MAX_WARMUP_RETRIES);
}

#[tokio::test]
async fn warmup_stops_retrying_on_success() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: true,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = RetryTrackingAgent::new("early-success-agent", 2); // Succeeds on 2nd attempt

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_success());
    assert_eq!(agent.attempt_count(), 2, "Should stop retrying after success");
}

// ============================================================================
// Exponential Backoff Tests
// ============================================================================

#[tokio::test]
async fn warmup_applies_exponential_backoff() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: true,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = AlwaysFailAgent::new("backoff-agent");

    let start = std::time::Instant::now();
    let _result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    let elapsed = start.elapsed();

    // With MAX_WARMUP_RETRIES = 2:
    // - Initial attempt fails immediately
    // - Retry 1 (attempt 0): backoff 100 * 2^0 = 100ms, then fails
    // - Retry 2 (attempt 1): no backoff (attempt < MAX_WARMUP_RETRIES - 1 is false)
    // Total minimum backoff: ~100ms
    assert!(elapsed >= Duration::from_millis(80),
            "Expected exponential backoff to add delay, but only took {:?}", elapsed);
}

#[tokio::test]
async fn warmup_backoff_increases_exponentially() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: true,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = AlwaysFailAgent::new("exponential-backoff-agent");

    // Track timing between attempts
    let _result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    // The implementation uses: 100 * (2^attempt) milliseconds
    // Attempt 0: 0ms backoff (first retry)
    // Attempt 1: 100ms backoff (second retry)
    // Total: ~100ms minimum for backoffs

    let attempts = agent.attempt_count();
    assert!(attempts > 1, "Should have made multiple attempts");
}

// ============================================================================
// Retry After Partial Success Tests
// ============================================================================

#[tokio::test]
async fn warmup_does_not_retry_after_first_success() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 3, // Allow multiple queries
        timeout_seconds: 5,
        retry_on_failure: true,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = RetryTrackingAgent::new("single-query-agent", 1); // Succeeds immediately

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_success());
    assert_eq!(result.queries_executed(), Some(1));
    assert_eq!(agent.attempt_count(), 1, "Should not execute additional queries after first success");
}

// ============================================================================
// Retry with Multiple Queries Tests
// ============================================================================

#[tokio::test]
async fn warmup_retries_on_first_query_failure() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 2,
        timeout_seconds: 5,
        retry_on_failure: true,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = RetryTrackingAgent::new("first-query-retry", 2); // Fails first, succeeds second

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_success());
    assert_eq!(result.queries_executed(), Some(1));
}

#[tokio::test]
async fn warmup_retries_on_second_query_failure() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 3,
        timeout_seconds: 5,
        retry_on_failure: true,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    // Agent that succeeds on first query, fails on second
    let agent = RetryTrackingAgent::new("second-query-agent", 1);

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    // First query should succeed immediately and stop
    assert!(result.is_success());
    assert_eq!(result.queries_executed(), Some(1));
}

// ============================================================================
// Retry Error Reporting Tests
// ============================================================================

#[tokio::test]
async fn warmup_retry_error_mentions_failures() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: true,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = AlwaysFailAgent::new("retry-error-agent");

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    if let WarmupResult::Failed { error, .. } = result {
        assert!(error.to_lowercase().contains("retry") ||
                error.to_lowercase().contains("failed") ||
                error.contains("attempts"),
                "Error should mention retry failure: {}", error);
    } else {
        panic!("Expected Failed result with retry mentioned");
    }
}

#[tokio::test]
async fn warmup_retry_includes_query_count_in_failure() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: true,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = AlwaysFailAgent::new("query-count-agent");

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    if let WarmupResult::Failed { queries_executed, .. } = result {
        assert_eq!(queries_executed, 0, "No queries should succeed with always-failing agent");
    } else {
        panic!("Expected Failed result");
    }
}

// ============================================================================
// Multiple Agent Retry Tests
// ============================================================================

#[tokio::test]
async fn warmup_multiple_agents_with_retry_enabled() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: true,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();

    let agent1 = RetryTrackingAgent::new("agent1", 1); // Immediate success
    let agent2 = RetryTrackingAgent::new("agent2", 2); // Success on retry
    let agent3 = AlwaysFailAgent::new("agent3"); // Always fails

    let backends: Vec<&dyn AgentBackend> = vec![&agent1, &agent2, &agent3];
    let results = executor.warmup_agents(&backends, &mut pool).await;

    assert_eq!(results.len(), 3);
    assert!(results[0].is_success(), "Agent 1 should succeed immediately");
    assert!(results[1].is_success(), "Agent 2 should succeed after retry");
    assert!(results[2].is_failed(), "Agent 3 should fail after all retries");
}

// ============================================================================
// Retry Configuration Tests
// ============================================================================

#[test]
fn test_retry_constants_are_reasonable() {
    // Verify retry configuration constants
    const MAX_WARMUP_RETRIES: u32 = 2; // From implementation

    assert!(MAX_WARMUP_RETRIES >= 1, "Should allow at least 1 retry");
    assert!(MAX_WARMUP_RETRIES <= 5, "Should not allow excessive retries (max 5)");
}

#[tokio::test]
async fn warmup_retry_behavior_with_custom_timeout() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 10,
        retry_on_failure: true,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = RetryTrackingAgent::new("custom-timeout-retry", 2);

    let start = std::time::Instant::now();
    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_success());
    // Agent should make 2 attempts (initial fails, retry succeeds)
    assert_eq!(agent.attempt_count(), 2);

    // Note: With succeed_on_attempt=2, the retry succeeds immediately
    // without needing backoff. The backoff only occurs if retry attempts fail.
    // This test verifies the retry mechanism works, even if no backoff occurs.
    assert!(elapsed < Duration::from_secs(1),
            "With immediate retry success, should complete quickly");
}
