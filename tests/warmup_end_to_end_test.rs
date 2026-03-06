//! End-to-end tests for warmup executor
//!
//! These tests verify complete warmup workflows:
//! - Full warmup lifecycle from configuration to session reuse
//! - Realistic multi-agent scenarios
//! - Integration with configuration system
//! - Production-like usage patterns
//! - Performance characteristics

use ltmatrix::agent::warmup::{WarmupExecutor, WarmupResult};
use ltmatrix::agent::{AgentSession, pool::SessionPool};
use ltmatrix::agent::backend::{AgentBackend, AgentConfig, AgentResponse, ExecutionConfig};
use ltmatrix::config::settings::{Config, WarmupConfig};
use ltmatrix::models::Agent;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

// ============================================================================
// Mock Agent for E2E Testing
// ============================================================================

struct E2ETestAgent {
    agent: Agent,
    warmup_count: Arc<AtomicU32>,
    execute_count: Arc<AtomicU32>,
    response_delay_ms: u64,
}

impl E2ETestAgent {
    fn new(name: &str, response_delay_ms: u64) -> Self {
        Self {
            agent: Agent::new(name, name, "test-model", 3600),
            warmup_count: Arc::new(AtomicU32::new(0)),
            execute_count: Arc::new(AtomicU32::new(0)),
            response_delay_ms,
        }
    }

    fn warmup_count(&self) -> u32 {
        self.warmup_count.load(Ordering::SeqCst)
    }

    fn execute_count(&self) -> u32 {
        self.execute_count.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl AgentBackend for E2ETestAgent {
    async fn execute(&self, _prompt: &str, _config: &ExecutionConfig) -> anyhow::Result<AgentResponse> {
        self.execute_count.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(self.response_delay_ms)).await;
        Ok(AgentResponse {
            output: "Response".to_string(),
            ..Default::default()
        })
    }

    async fn execute_with_session(
        &self,
        prompt: &str,
        _config: &ExecutionConfig,
        _session: &dyn AgentSession,
    ) -> anyhow::Result<AgentResponse> {
        // Check if this is a warmup query (case-insensitive)
        let prompt_lower = prompt.to_lowercase();
        if prompt_lower.contains("ready") || prompt_lower.contains("hello") || prompt_lower.contains("warmup") {
            self.warmup_count.fetch_add(1, Ordering::SeqCst);
        } else {
            self.execute_count.fetch_add(1, Ordering::SeqCst);
        }

        tokio::time::sleep(std::time::Duration::from_millis(self.response_delay_ms)).await;
        Ok(AgentResponse {
            output: "Response".to_string(),
            ..Default::default()
        })
    }

    async fn execute_task(
        &self,
        _task: &ltmatrix::models::Task,
        _context: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        self.execute_count.fetch_add(1, Ordering::SeqCst);
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
// End-to-End Warmup Workflow Tests
// ============================================================================

#[tokio::test]
async fn e2e_warmup_workflow_with_config() {
    // Simulate loading configuration
    let config = Config {
        warmup: WarmupConfig {
            enabled: true,
            max_queries: 1,
            timeout_seconds: 30,
            retry_on_failure: false,
            prompt_template: Some("Hello! Custom warmup prompt for testing.".to_string()),
        },
        ..Default::default()
    };

    let executor = WarmupExecutor::new(config.warmup);
    let mut pool = SessionPool::new();
    let agent = E2ETestAgent::new("e2e-agent", 100);

    // Execute warmup
    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    // Verify workflow completed successfully
    assert!(result.is_success());
    assert_eq!(agent.warmup_count(), 1, "Agent should have received 1 warmup query");
    assert_eq!(agent.execute_count(), 0, "Agent should not have received real queries yet");
    assert!(pool.len() >= 1, "Session pool should have a session");

    // Verify warmup prepared session for reuse
    let session = pool.get_or_create("e2e-agent", "test-model");
    assert!(!session.session_id().to_string().is_empty(), "Session should have a valid ID");
}

#[tokio::test]
async fn e2e_multi_agent_warmup_scenario() {
    // Simulate production scenario with multiple agents
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 30,
        retry_on_failure: true,
        prompt_template: None,
    };

    let executor = WarmupExecutor::new(config);
    let mut pool = SessionPool::new();

    // Create multiple agents (simulating Claude, GPT-4, etc.)
    let agents: Vec<E2ETestAgent> = vec![
        E2ETestAgent::new("claude", 150),
        E2ETestAgent::new("gpt-4", 200),
        E2ETestAgent::new("gemini", 180),
    ];

    let agent_refs: Vec<&E2ETestAgent> = agents.iter().collect();

    // Warm up all agents
    let results = executor.warmup_agents(&agent_refs, &mut pool).await;

    // Verify all agents warmed up successfully
    assert_eq!(results.len(), 3);
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_success(), "Agent {} should have warmed up successfully", i);
    }

    // Verify each agent received exactly one warmup query
    for agent in &agents {
        assert_eq!(agent.warmup_count(), 1, "Each agent should receive 1 warmup query");
    }

    // Verify pool has sessions for all agents
    assert_eq!(pool.len(), 3, "Pool should have 3 sessions");
}

#[tokio::test]
async fn e2e_warmup_to_task_execution_workflow() {
    // Test full workflow: warmup -> real task execution
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = E2ETestAgent::new("workflow-agent", 100);

    // Step 1: Warm up the agent
    let warmup_result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    assert!(warmup_result.is_success());
    assert_eq!(agent.warmup_count(), 1);
    assert_eq!(agent.execute_count(), 0);

    // Step 2: Execute real task using the warmed session
    let session = pool.get_or_create("workflow-agent", "test-model");
    let exec_config = ExecutionConfig::default();

    let task_result = agent
        .execute_with_session("Real task prompt", &exec_config, session)
        .await
        .unwrap();

    assert!(!task_result.output.is_empty());
    assert_eq!(agent.execute_count(), 1, "Real task should have been executed");

    // Verify counts
    assert_eq!(agent.warmup_count(), 1, "Warmup count should still be 1");
}

#[tokio::test]
async fn e2e_configuration_driven_warmup_behavior() {
    // Test different configurations driving different behavior

    // Disabled warmup
    let disabled_executor = WarmupExecutor::new(WarmupConfig {
        enabled: false,
        ..Default::default()
    });
    let mut pool1 = SessionPool::new();
    let agent1 = E2ETestAgent::new("disabled-agent", 100);

    let result1 = disabled_executor.warmup_agent(&agent1, &mut pool1).await.unwrap();
    assert!(result1.is_skipped());
    assert_eq!(agent1.warmup_count(), 0, "No warmup queries when disabled");
    assert!(pool1.is_empty(), "No sessions created when warmup disabled");

    // Enabled warmup with retry
    let retry_executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 30,
        retry_on_failure: true,
        prompt_template: None,
    });
    let mut pool2 = SessionPool::new();
    let agent2 = E2ETestAgent::new("retry-agent", 100);

    let result2 = retry_executor.warmup_agent(&agent2, &mut pool2).await.unwrap();
    assert!(result2.is_success());
    assert_eq!(agent2.warmup_count(), 1);
}

#[tokio::test]
async fn e2e_warmup_performance_characteristics() {
    // Test that warmup provides performance benefits

    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = E2ETestAgent::new("perf-agent", 50); // Fast response

    // Measure warmup time
    let warmup_start = std::time::Instant::now();
    let _warmup_result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    let warmup_duration = warmup_start.elapsed();

    // Warmup should be fast
    assert!(warmup_duration < std::time::Duration::from_secs(1),
            "Warmup should complete quickly, took {:?}", warmup_duration);

    // Measure task execution time after warmup
    let session = pool.get_or_create("perf-agent", "test-model");
    let exec_config = ExecutionConfig::default();

    let task_start = std::time::Instant::now();
    let _task_result = agent
        .execute_with_session("Task prompt", &exec_config, session)
        .await
        .unwrap();
    let task_duration = task_start.elapsed();

    // Task should also be fast (using warmed session)
    assert!(task_duration < std::time::Duration::from_millis(200),
            "Task after warmup should be fast, took {:?}", task_duration);
}

#[tokio::test]
async fn e2e_warmup_with_custom_prompt_template() {
    // Test that custom prompt templates are used
    let custom_prompt = "Are you ready for code generation tasks?";

    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: Some(custom_prompt.to_string()),
    });

    let mut pool = SessionPool::new();
    let agent = E2ETestAgent::new("custom-prompt-agent", 100);

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_success());
    assert_eq!(agent.warmup_count(), 1, "Should have received custom warmup prompt");
}

#[tokio::test]
async fn e2e_warmup_session_reuse_across_operations() {
    // Test that warmed sessions are properly reused

    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = E2ETestAgent::new("reuse-agent", 100);

    // Initial warmup
    let warmup_result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    assert!(warmup_result.is_success());

    let session_id1 = pool.get_or_create("reuse-agent", "test-model").session_id().to_string();

    // Warmup again (should reuse session)
    let warmup_result2 = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    assert!(warmup_result2.is_success());

    let session_id2 = pool.get_or_create("reuse-agent", "test-model").session_id().to_string();

    assert_eq!(session_id1, session_id2, "Session ID should remain the same");
    assert_eq!(pool.len(), 1, "Should still have only 1 session");

    // Execute tasks using the session
    let session = pool.get_or_create("reuse-agent", "test-model");
    let exec_config = ExecutionConfig::default();

    for _ in 0..3 {
        agent
            .execute_with_session("Task", &exec_config, session)
            .await
            .unwrap();
    }

    // Should still have the same session
    let session_id3 = pool.get_or_create("reuse-agent", "test-model").session_id().to_string();
    assert_eq!(session_id1, session_id3, "Session ID should still be the same after tasks");
}

#[tokio::test]
async fn e2e_warmup_error_recovery_workflow() {
    // Test workflow when warmup encounters errors

    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 30,
        retry_on_failure: true, // Enable retry for recovery
        prompt_template: None,
    });

    let mut pool = SessionPool::new();

    // Create an agent that will succeed
    let agent = E2ETestAgent::new("recovery-agent", 100);

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    // Should succeed
    assert!(result.is_success());
    assert_eq!(agent.warmup_count(), 1);
    assert!(pool.len() >= 1);
}

#[tokio::test]
async fn e2e_warmup_result_tracking_and_reporting() {
    // Test that warmup results provide useful information

    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = E2ETestAgent::new("tracking-agent", 100);

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    // Verify result provides useful information
    assert!(result.is_success());
    assert_eq!(result.queries_executed(), Some(1));

    if let WarmupResult::Success { queries_executed, duration_ms } = result {
        assert_eq!(queries_executed, 1);
        assert!(duration_ms > 0, "Duration should be recorded");
        assert!(duration_ms < 10000, "Duration should be reasonable (< 10s)");
    } else {
        panic!("Expected Success result with timing info");
    }
}

#[tokio::test]
async fn e2e_warmup_multiple_queries_configuration() {
    // Test behavior with multiple warmup queries configured

    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 3, // Multiple queries
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = E2ETestAgent::new("multi-query-agent", 50);

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    // Implementation breaks after first successful query
    assert!(result.is_success());
    assert_eq!(result.queries_executed(), Some(1), "Should only execute 1 query (breaks after success)");
    assert_eq!(agent.warmup_count(), 1);
}

#[tokio::test]
async fn e2e_warmup_integration_with_config_loading() {
    // Test integration with configuration loading (simulated)

    // Simulate TOML config
    let toml_config = r#"
        [warmup]
        enabled = true
        max_queries = 2
        timeout_seconds = 45
        retry_on_failure = false
        prompt_template = "Integration test prompt"
    "#;

    let parsed_config: Config = toml::from_str(toml_config).unwrap();

    assert_eq!(parsed_config.warmup.enabled, true);
    assert_eq!(parsed_config.warmup.max_queries, 2);
    assert_eq!(parsed_config.warmup.timeout_seconds, 45);
    assert_eq!(parsed_config.warmup.retry_on_failure, false);
    assert_eq!(
        parsed_config.warmup.prompt_template,
        Some("Integration test prompt".to_string())
    );

    // Use the parsed config for warmup
    let executor = WarmupExecutor::new(parsed_config.warmup);
    let mut pool = SessionPool::new();
    let agent = E2ETestAgent::new("config-integration-agent", 100);

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(result.is_success());
}
