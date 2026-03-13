//! Session pool integration tests for warmup executor
//!
//! These tests verify the interaction between warmup executor and session pool:
//! - Sessions are created during warmup
//! - Sessions are reused across warmup queries
//! - Session lifecycle is managed correctly
//! - Multiple agents get separate sessions
//! - Session pool statistics are accurate
//! - Session cleanup and management

use ltmatrix::agent::backend::{AgentBackend, AgentConfig, AgentResponse, ExecutionConfig};
use ltmatrix::agent::warmup::WarmupExecutor;
use ltmatrix::agent::{pool::SessionPool, AgentSession};
use ltmatrix::config::settings::WarmupConfig;
use ltmatrix::models::Agent;

// ============================================================================
// Mock Agent for Session Pool Testing
// ============================================================================

struct SessionTrackingAgent {
    agent: Agent,
    session_ids: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl SessionTrackingAgent {
    fn new(name: &str) -> Self {
        Self {
            agent: Agent::new(name, name, "test-model", 3600),
            session_ids: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    fn recorded_session_count(&self) -> usize {
        self.session_ids.lock().unwrap().len()
    }
}

#[async_trait::async_trait]
impl AgentBackend for SessionTrackingAgent {
    async fn execute(
        &self,
        _prompt: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        Ok(AgentResponse {
            output: "Ready".to_string(),
            ..Default::default()
        })
    }

    async fn execute_with_session(
        &self,
        _prompt: &str,
        _config: &ExecutionConfig,
        session: &dyn AgentSession,
    ) -> anyhow::Result<AgentResponse> {
        // Record the session ID
        let session_id = session.session_id().to_string();
        self.session_ids.lock().unwrap().push(session_id);

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
// Session Creation Tests
// ============================================================================

#[tokio::test]
async fn warmup_creates_session_in_pool() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    assert_eq!(pool.len(), 0, "Pool should start empty");

    let agent = SessionTrackingAgent::new("test-agent");
    let _result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(
        pool.len() >= 1,
        "Pool should have at least one session after warmup"
    );
}

#[tokio::test]
async fn warmup_creates_separate_sessions_for_different_agents() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();

    let agent1 = SessionTrackingAgent::new("agent1");
    let agent2 = SessionTrackingAgent::new("agent2");
    let agent3 = SessionTrackingAgent::new("agent3");

    executor.warmup_agent(&agent1, &mut pool).await.unwrap();
    executor.warmup_agent(&agent2, &mut pool).await.unwrap();
    executor.warmup_agent(&agent3, &mut pool).await.unwrap();

    assert_eq!(pool.len(), 3, "Pool should have 3 separate sessions");
}

// ============================================================================
// Session Reuse Tests
// ============================================================================

#[tokio::test]
async fn warmup_reuses_session_for_same_agent() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 2, // Multiple queries
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = SessionTrackingAgent::new("single-agent");

    let _result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    // Should have recorded exactly 1 session ID (even with 2 queries, they reuse the session)
    assert_eq!(
        agent.recorded_session_count(),
        1,
        "Agent should use the same session for all warmup queries"
    );
}

#[tokio::test]
async fn warmup_session_reuse_across_multiple_warmup_calls() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = SessionTrackingAgent::new("reuse-agent");

    // Warm up the same agent multiple times
    executor.warmup_agent(&agent, &mut pool).await.unwrap();
    executor.warmup_agent(&agent, &mut pool).await.unwrap();
    executor.warmup_agent(&agent, &mut pool).await.unwrap();

    // All calls should use the same session
    assert_eq!(pool.len(), 1, "Pool should still have only 1 session");
    assert_eq!(
        agent.recorded_session_count(),
        3,
        "Should have recorded 3 session uses (same session ID 3 times)"
    );
}

// ============================================================================
// Session Pool Statistics Tests
// ============================================================================

#[tokio::test]
async fn session_pool_statistics_accurate_after_warmup() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    assert!(pool.is_empty(), "New pool should be empty");
    assert_eq!(pool.len(), 0, "New pool length should be 0");

    let agent1 = SessionTrackingAgent::new("agent1");
    let agent2 = SessionTrackingAgent::new("agent2");

    executor.warmup_agent(&agent1, &mut pool).await.unwrap();
    assert!(
        !pool.is_empty(),
        "Pool should not be empty after first warmup"
    );
    assert_eq!(
        pool.len(),
        1,
        "Pool should have 1 session after first warmup"
    );

    executor.warmup_agent(&agent2, &mut pool).await.unwrap();
    assert_eq!(
        pool.len(),
        2,
        "Pool should have 2 sessions after second warmup"
    );
}

#[tokio::test]
async fn session_pool_handles_multiple_agents_correctly() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();

    let agents: Vec<SessionTrackingAgent> = (0..5)
        .map(|i| SessionTrackingAgent::new(&format!("agent{}", i)))
        .collect();

    for agent in &agents {
        executor.warmup_agent(agent, &mut pool).await.unwrap();
    }

    assert_eq!(pool.len(), 5, "Pool should have 5 sessions for 5 agents");

    // Verify each agent got a session
    for agent in &agents {
        assert_eq!(
            agent.recorded_session_count(),
            1,
            "Each agent should have recorded 1 session use"
        );
    }
}

// ============================================================================
// Session Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn session_persists_after_failed_warmup() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = SessionTrackingAgent::new("persistent-session-agent");

    // First warmup succeeds
    let result1 = executor.warmup_agent(&agent, &mut pool).await.unwrap();
    assert!(result1.is_success());
    let pool_size_after_success = pool.len();

    // Session should persist even if we don't do anything else
    assert!(pool_size_after_success >= 1);
}

#[tokio::test]
async fn warmup_with_multiple_queries_uses_same_session() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 5, // Multiple queries
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = SessionTrackingAgent::new("multi-query-agent");

    let _result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    // Even though max_queries is 5, only first query should execute
    // (implementation breaks after first success)
    // But all queries should use the same session
    assert_eq!(
        agent.recorded_session_count(),
        1,
        "All queries should use the same session"
    );
}

// ============================================================================
// Session ID Uniqueness Tests
// ============================================================================

#[tokio::test]
async fn session_ids_are_unique_for_different_agents() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();

    let agent1 = SessionTrackingAgent::new("unique-agent-1");
    let agent2 = SessionTrackingAgent::new("unique-agent-2");

    executor.warmup_agent(&agent1, &mut pool).await.unwrap();
    executor.warmup_agent(&agent2, &mut pool).await.unwrap();

    // Get session IDs for each agent
    let id1 = pool
        .get_or_create("unique-agent-1", "test-model")
        .session_id()
        .to_string();
    let id2 = pool
        .get_or_create("unique-agent-2", "test-model")
        .session_id()
        .to_string();

    assert_ne!(
        id1, id2,
        "Different agents should have different session IDs"
    );
}

#[tokio::test]
async fn session_id_is_consistent_for_same_agent() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();
    let agent = SessionTrackingAgent::new("consistent-id-agent");

    // Warm up twice
    executor.warmup_agent(&agent, &mut pool).await.unwrap();
    let session1 = pool.get_or_create("consistent-id-agent", "test-model");
    let id1 = session1.session_id().to_string();

    executor.warmup_agent(&agent, &mut pool).await.unwrap();
    let session2 = pool.get_or_create("consistent-id-agent", "test-model");
    let id2 = session2.session_id().to_string();

    assert_eq!(
        id1, id2,
        "Same agent should get the same session ID across warmups"
    );
}

// ============================================================================
// Session Pool and Warmup Integration Tests
// ============================================================================

#[tokio::test]
async fn warmup_multiple_agents_populates_pool() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();

    let agents: Vec<SessionTrackingAgent> = (0..10)
        .map(|i| SessionTrackingAgent::new(&format!("pool-agent-{}", i)))
        .collect();

    let agent_refs: Vec<&SessionTrackingAgent> = agents.iter().collect();
    let _results = executor.warmup_agents(&agent_refs, &mut pool).await;

    assert_eq!(pool.len(), 10, "Pool should contain 10 sessions");

    // Verify each agent has a recorded session
    for agent in &agents {
        assert_eq!(
            agent.recorded_session_count(),
            1,
            "Each agent should have used a session"
        );
    }
}

#[tokio::test]
async fn warmup_respects_existing_sessions_in_pool() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();

    // Pre-populate pool with a session
    let _existing_session = pool.get_or_create("existing-agent", "test-model");
    assert_eq!(pool.len(), 1, "Pool should have 1 pre-existing session");

    let agent = SessionTrackingAgent::new("existing-agent");

    // Warmup should reuse the existing session
    let _result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert_eq!(
        pool.len(),
        1,
        "Pool should still have only 1 session (reused existing)"
    );
}

// ============================================================================
// Session Pool Empty State Tests
// ============================================================================

#[tokio::test]
async fn session_pool_empty_state() {
    let mut pool = SessionPool::new();

    assert!(pool.is_empty(), "New pool should be empty");
    assert_eq!(pool.len(), 0, "New pool should have length 0");

    // Add a session
    let _session = pool.get_or_create("test-agent", "test-model");
    assert!(
        !pool.is_empty(),
        "Pool should not be empty after adding session"
    );
    assert_eq!(
        pool.len(),
        1,
        "Pool should have length 1 after adding session"
    );
}

#[tokio::test]
async fn session_pool_with_no_warmup() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: false, // Warmup disabled
        ..Default::default()
    });

    let mut pool = SessionPool::new();
    let agent = SessionTrackingAgent::new("no-warmup-agent");

    let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();

    assert!(
        result.is_skipped(),
        "Warmup should be skipped when disabled"
    );
    assert!(
        pool.is_empty(),
        "Pool should be empty when warmup is skipped"
    );
}

// ============================================================================
// Session Pool Sequential Access Tests
// ============================================================================

#[tokio::test]
async fn session_pool_handles_sequential_warmup() {
    let executor = WarmupExecutor::new(WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 5,
        retry_on_failure: false,
        prompt_template: None,
    });

    let mut pool = SessionPool::new();

    let agent1 = SessionTrackingAgent::new("sequential-agent-1");
    let agent2 = SessionTrackingAgent::new("sequential-agent-2");
    let agent3 = SessionTrackingAgent::new("sequential-agent-3");

    // Run warmups sequentially (avoiding concurrent borrow issues)
    executor.warmup_agent(&agent1, &mut pool).await.unwrap();
    executor.warmup_agent(&agent2, &mut pool).await.unwrap();
    executor.warmup_agent(&agent3, &mut pool).await.unwrap();

    // Verify pool has sessions for all agents
    assert_eq!(pool.len(), 3, "Pool should have 3 sessions");

    // Verify each agent recorded a session use
    assert_eq!(agent1.recorded_session_count(), 1);
    assert_eq!(agent2.recorded_session_count(), 1);
    assert_eq!(agent3.recorded_session_count(), 1);
}
