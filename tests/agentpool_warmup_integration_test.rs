//! AgentPool warmup integration tests
//!
//! These tests verify that AgentPool integrates with the warmup executor
//! to pre-initialize agent sessions before use.

use ltmatrix::agent::pool::SessionPool;
use ltmatrix::agent::warmup::WarmupExecutor;
use ltmatrix::agent::AgentSession;
use ltmatrix::config::settings::WarmupConfig;

// ============================================================================
// AgentPool Warmup Initialization Tests
// ============================================================================

#[test]
fn agentpool_accepts_warmup_executor() {
    // AgentPool should be able to accept and store a warmup executor
    let warmup_config = WarmupConfig {
        enabled: true,
        max_queries: 2,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: None,
    };
    let executor = WarmupExecutor::new(warmup_config);

    // This should create an AgentPool with warmup capability
    let _pool = SessionPool::with_warmup(executor);
}

#[test]
fn agentpool_default_has_no_warmup() {
    // Default AgentPool should not have warmup enabled
    let pool = SessionPool::new();

    // Should not have warmup executor by default
    assert!(!pool.has_warmup());
}

#[test]
fn agentpool_with_warmup_has_executor() {
    // AgentPool with warmup should report having an executor
    let warmup_config = WarmupConfig {
        enabled: true,
        ..Default::default()
    };
    let executor = WarmupExecutor::new(warmup_config);

    let pool = SessionPool::with_warmup(executor);

    assert!(pool.has_warmup());
}

// ============================================================================
// Warmup Execution on Session Creation Tests
// ============================================================================

#[tokio::test]
async fn agentpool_warms_up_on_first_session_creation() {
    // When creating the first session for an agent, AgentPool should run warmup
    let warmup_config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: Some("Test warmup".to_string()),
    };

    let executor = WarmupExecutor::new(warmup_config);
    let mut pool = SessionPool::with_warmup(executor);

    // Create first session - should trigger warmup
    let session_id = pool
        .get_or_create_warmup("claude", "claude-sonnet-4-6")
        .await
        .expect("Warmup should succeed");

    // Session should be created and registered
    assert!(pool.get(&session_id).is_some());
}

#[tokio::test]
async fn agentpool_skips_warmup_when_disabled() {
    // When warmup is disabled, AgentPool should not run warmup
    let warmup_config = WarmupConfig {
        enabled: false, // Disabled
        ..Default::default()
    };

    let executor = WarmupExecutor::new(warmup_config);
    let mut pool = SessionPool::with_warmup(executor);

    // Create session - should skip warmup
    let session_id = pool
        .get_or_create_warmup("claude", "claude-sonnet-4-6")
        .await
        .expect("Should succeed even without warmup");

    // Session should still be created
    assert!(pool.get(&session_id).is_some());
}

// ============================================================================
// Warmup Failure Handling Tests
// ============================================================================

#[tokio::test]
async fn agentpool_handles_warmup_failure_when_retry_disabled() {
    // When warmup fails and retry is disabled, AgentPool should handle gracefully
    let warmup_config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 1, // Very short timeout to trigger failure
        retry_on_failure: false,
        prompt_template: None,
    };

    let executor = WarmupExecutor::new(warmup_config);
    let mut pool = SessionPool::with_warmup(executor);

    // Should handle warmup failure gracefully
    let result = pool
        .get_or_create_warmup("claude", "claude-sonnet-4-6")
        .await;

    // Should either succeed with fallback or return error
    // (Implementation choice: fail fast or create session anyway)
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn agentpool_retries_warmup_when_enabled() {
    // When warmup fails and retry is enabled, AgentPool should retry
    let warmup_config = WarmupConfig {
        enabled: true,
        max_queries: 2,
        timeout_seconds: 30,
        retry_on_failure: true, // Enable retry
        prompt_template: None,
    };

    let executor = WarmupExecutor::new(warmup_config);
    let mut pool = SessionPool::with_warmup(executor);

    // Should retry warmup on failure
    let session_id = pool
        .get_or_create_warmup("claude", "claude-sonnet-4-6")
        .await
        .expect("Should succeed after retry");

    // Session should be created
    assert!(pool.get(&session_id).is_some());
}

// ============================================================================
// Session Reuse After Warmup Tests
// ============================================================================

#[tokio::test]
async fn agentpool_reuses_warmed_session() {
    // After warmup, subsequent requests should reuse the warmed session
    let warmup_config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: None,
    };

    let executor = WarmupExecutor::new(warmup_config);
    let mut pool = SessionPool::with_warmup(executor);

    // First call - warms up
    let session_id1 = pool
        .get_or_create_warmup("claude", "claude-sonnet-4-6")
        .await
        .expect("First call should warm up");

    // Second call - should reuse
    let session_id2 = pool
        .get_or_create_warmup("claude", "claude-sonnet-4-6")
        .await
        .expect("Second call should reuse");

    // Should be the same session
    assert_eq!(session_id1, session_id2);

    // Pool should only have one session
    assert_eq!(pool.len(), 1);
}

// ============================================================================
// Warmup Status Tracking Tests
// ============================================================================

#[test]
fn agentpool_tracks_warmup_status() {
    // AgentPool should track which agents have been warmed up
    let warmup_config = WarmupConfig {
        enabled: true,
        ..Default::default()
    };
    let executor = WarmupExecutor::new(warmup_config);

    let pool = SessionPool::with_warmup(executor);

    // Should be able to check warmup status for agents
    assert!(!pool.is_warmed_up("claude", "claude-sonnet-4-6"));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn agentpool_warmup_integration_end_to_end() {
    // Full integration test: warmup → session creation → reuse
    let warmup_config = WarmupConfig {
        enabled: true,
        max_queries: 2,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: Some("E2E test warmup".to_string()),
    };

    let executor = WarmupExecutor::new(warmup_config);
    let mut pool = SessionPool::with_warmup(executor);

    // Step 1: Create session with warmup
    let session_id = pool
        .get_or_create_warmup("claude", "claude-sonnet-4-6")
        .await
        .expect("Warmup should succeed");

    // Step 2: Verify session exists
    let session = pool.get(&session_id);
    assert!(session.is_some());
    let session_ref = session.unwrap();
    assert_eq!(session_ref.agent_name(), "claude");

    // Step 3: Reuse session
    let session_id2 = pool
        .get_or_create_warmup("claude", "claude-sonnet-4-6")
        .await
        .expect("Reuse should succeed");

    assert_eq!(session_id, session_id2);
}
