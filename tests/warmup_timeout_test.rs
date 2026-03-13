//! Timeout tests for warmup executor
//!
//! These tests verify timeout behavior:
//! - Warmup queries respect configured timeout
//! - Timeout errors are handled correctly
//! - Timeout doesn't affect other agents
//! - Custom timeout values are respected
//! - Timeout behavior with retry enabled
//! - Timeout aborts long-running queries

use ltmatrix::agent::backend::{AgentBackend, AgentConfig, AgentResponse, ExecutionConfig};
use ltmatrix::agent::warmup::{WarmupExecutor, WarmupResult};
use ltmatrix::agent::{pool::SessionPool, AgentSession};
use ltmatrix::config::settings::WarmupConfig;
use ltmatrix::models::Agent;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Mock Agents for Timeout Testing
// ============================================================================

/// Mock agent that simulates slow responses
struct SlowAgent {
    agent: Agent,
    delay_ms: u64,
}

impl SlowAgent {
    fn new(name: &str, delay_ms: u64) -> Self {
        Self {
            agent: Agent::new(name, name, "test-model", 3600),
            delay_ms,
        }
    }
}

#[async_trait::async_trait]
impl AgentBackend for SlowAgent {
    async fn execute(
        &self,
        _prompt: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
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
        tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
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

    async fn validate_config(
        &self,
        _config: &AgentConfig,
    ) -> Result<(), ltmatrix::agent::backend::AgentError> {
        Ok(())
    }

    fn agent(&self) -> &Agent {
        &self.agent
    }
}

/// Mock agent that can be configured to timeout
struct TimeoutAgent {
    agent: Agent,
    delay_ms: u64,
    was_cancelled: Arc<AtomicBool>,
}

impl TimeoutAgent {
    fn new(name: &str, delay_ms: u64) -> Self {
        Self {
            agent: Agent::new(name, name, "test-model", 3600),
            delay_ms,
            was_cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    fn was_cancelled(&self) -> bool {
        self.was_cancelled.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl AgentBackend for TimeoutAgent {
    async fn execute(
        &self,
        _prompt: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
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
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(self.delay_ms)) => {
                Ok(AgentResponse {
                    output: "Ready".to_string(),
                    ..Default::default()
                })
            }
            _ = tokio::time::sleep(Duration::from_secs(10)) => {
                // This simulates a very long operation that should be cancelled
                self.was_cancelled.store(true, Ordering::SeqCst);
                anyhow::bail!("Operation took too long and was cancelled");
            }
        }
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

    async fn validate_config(
        &self,
        _config: &AgentConfig,
    ) -> Result<(), ltmatrix::agent::backend::AgentError> {
        Ok(())
    }

    fn agent(&self) -> &Agent {
        &self.agent
    }
}

/// Mock agent that always times out
struct AlwaysTimeoutAgent {
    agent: Agent,
}

impl AlwaysTimeoutAgent {
    fn new(name: &str) -> Self {
        Self {
            agent: Agent::new(name, name, "test-model", 3600),
        }
    }
}

#[async_trait::async_trait]
impl AgentBackend for AlwaysTimeoutAgent {
    async fn execute(
        &self,
        _prompt: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        tokio::time::sleep(Duration::from_secs(100)).await;
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
        tokio::time::sleep(Duration::from_secs(100)).await;
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

    async fn validate_config(
        &self,
        _config: &AgentConfig,
    ) -> Result<(), ltmatrix::agent::backend::AgentError> {
        Ok(())
    }

    fn agent(&self) -> &Agent {
        &self.agent
    }
}

// ============================================================================
// Basic Timeout Tests
// ============================================================================

#[tokio::test]
async fn warmup_succeeds_within_timeout() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    // Agent responds in 100ms, well within 5 second timeout
    let agent = SlowAgent::new("fast-agent", 100);

    let start = std::time::Instant::now();
    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_success());
    assert!(
        elapsed < Duration::from_secs(1),
        "Should complete quickly (< 1s)"
    );
}

#[tokio::test]
async fn warmup_times_out_on_slow_agent() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 1, // 1 second timeout
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    // Agent takes 5 seconds, will timeout
    let agent = AlwaysTimeoutAgent::new("slow-agent");

    let start = std::time::Instant::now();
    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_failed());
    // Should timeout in approximately 1 second (with some tolerance)
    assert!(
        elapsed >= Duration::from_millis(900),
        "Should take at least 900ms"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "Should timeout before 2 seconds"
    );

    if let WarmupResult::Failed { error, .. } = result {
        assert!(
            error.to_lowercase().contains("timeout") || error.to_lowercase().contains("timed out"),
            "Error should mention timeout: {}",
            error
        );
    }
}

// ============================================================================
// Custom Timeout Configuration Tests
// ============================================================================

#[tokio::test]
async fn warmup_respects_custom_timeout() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 2, // Custom 2 second timeout
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = AlwaysTimeoutAgent::new("custom-timeout-agent");

    let start = std::time::Instant::now();
    let _result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    let elapsed = start.elapsed();

    // Should timeout in approximately 2 seconds
    assert!(
        elapsed >= Duration::from_millis(1800),
        "Should take at least 1.8s with 2s timeout"
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "Should timeout before 3 seconds"
    );
}

#[tokio::test]
async fn warmup_very_short_timeout() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 1, // Very short timeout
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = SlowAgent::new("slower-than-timeout", 2000); // 2 second delay

    let start = std::time::Instant::now();
    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_failed());
    assert!(
        elapsed < Duration::from_secs(2),
        "Should timeout before 2 seconds"
    );
}

// ============================================================================
// Timeout with Retry Tests
// ============================================================================

#[tokio::test]
async fn warmup_timeout_with_retry_enabled() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 1,
        retry_on_failure: true, // Enable retry
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = AlwaysTimeoutAgent::new("timeout-retry-agent");

    let start = std::time::Instant::now();
    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_failed());

    // With retry, should take longer (timeout + retries)
    // Initial attempt + 2 retries with exponential backoff
    assert!(
        elapsed >= Duration::from_secs(1),
        "Should take at least 1 second"
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "Should complete within 5 seconds even with retries"
    );
}

#[tokio::test]
async fn warmup_retry_after_timeout() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 2,
        retry_on_failure: true,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    // Agent will timeout
    let agent = AlwaysTimeoutAgent::new("retry-after-timeout");

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_failed());
    // Should have attempted retries after timeout
}

// ============================================================================
// Timeout Error Message Tests
// ============================================================================

#[tokio::test]
async fn timeout_error_message_is_descriptive() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 1,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = AlwaysTimeoutAgent::new("timeout-error-msg-agent");

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    if let WarmupResult::Failed { error, .. } = result {
        assert!(!error.is_empty(), "Error message should not be empty");
        assert!(
            error.to_lowercase().contains("timeout") || error.to_lowercase().contains("timed out"),
            "Error should mention timeout: {}",
            error
        );
    } else {
        panic!("Expected Failed result");
    }
}

// ============================================================================
// Multiple Agent Timeout Tests
// ============================================================================

#[tokio::test]
async fn timeout_affects_only_slow_agent() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 1,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();

    let agent1 = SlowAgent::new("fast-agent-1", 100); // Fast
    let agent2 = AlwaysTimeoutAgent::new("slow-agent-2"); // Slow
    let agent3 = SlowAgent::new("fast-agent-3", 100); // Fast

    let backends: Vec<&dyn AgentBackend> = vec![&agent1, &agent2, &agent3];
    let results = executor.warmup_agents(&backends, &mut pool).await;

    assert_eq!(results.len(), 3);
    assert!(results[0].is_success(), "Fast agent 1 should succeed");
    assert!(results[1].is_failed(), "Slow agent 2 should timeout");
    assert!(results[2].is_success(), "Fast agent 3 should succeed");
}

// ============================================================================
// Timeout Configuration Validation Tests
// ============================================================================

#[test]
fn test_timeout_configuration_validation() {
    let config1 = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 0, // Invalid
        retry_on_failure: false,
        prompt_template: None,
    };

    assert!(
        config1.validate().is_err(),
        "Timeout of 0 should fail validation"
    );

    let config2 = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 1, // Valid minimum
        retry_on_failure: false,
        prompt_template: None,
    };

    assert!(
        config2.validate().is_ok(),
        "Timeout of 1 second should be valid"
    );
}

#[tokio::test]
async fn warmup_timeout_boundary_values() {
    // Test with minimum valid timeout
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 1, // Minimum valid timeout
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = SlowAgent::new("boundary-agent", 50); // Completes in 50ms

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    assert!(result.is_success());
}

// ============================================================================
// Timeout Duration Tracking Tests
// ============================================================================

#[tokio::test]
async fn warmup_duration_includes_timeout_time() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 1,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = AlwaysTimeoutAgent::new("duration-tracking-agent");

    let start = std::time::Instant::now();
    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    let elapsed = start.elapsed();

    if let WarmupResult::Failed { .. } = result {
        // The warmup should have taken approximately the timeout duration
        assert!(
            elapsed >= Duration::from_millis(900),
            "Duration should reflect timeout wait time"
        );
    } else {
        panic!("Expected timeout failure");
    }
}

// ============================================================================
// Timeout with Slow but Successful Agents
// ============================================================================

#[tokio::test]
async fn warmup_succeeds_for_slow_but_within_timeout_agent() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 2,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    // Agent takes 1.5 seconds, within 2 second timeout
    let agent = SlowAgent::new("slow-but-ok", 1500);

    let start = std::time::Instant::now();
    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_success());
    assert!(
        elapsed >= Duration::from_millis(1400),
        "Should take at least 1.4s"
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "Should complete within 3 seconds"
    );
}

#[tokio::test]
async fn warmup_with_varied_response_times() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();

    // Test agents with different response times
    let fast_agent = SlowAgent::new("fast", 50);
    let medium_agent = SlowAgent::new("medium", 500);
    let slow_agent = SlowAgent::new("slow", 1500);

    let results1 = executor.warmup_agent(&fast_agent, &mut pool).await.unwrap();
    let results2 = executor
        .warmup_agent(&medium_agent, &mut pool)
        .await
        .unwrap();
    let results3 = executor.warmup_agent(&slow_agent, &mut pool).await.unwrap();

    assert!(results1.is_success());
    assert!(results2.is_success());
    assert!(results3.is_success());
}
