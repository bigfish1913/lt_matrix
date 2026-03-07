//! Tests for AgentPool integration with execute stage
//!
//! This test module verifies the integration of AgentPool with the task execution system,
//! ensuring proper session management, reuse strategies, and cleanup behavior.

use ltmatrix::agent::AgentPool;
use ltmatrix::config::settings::{Config, PoolConfig, WarmupConfig};
use ltmatrix::models::{ModeConfig, Task, TaskComplexity};
use ltmatrix::pipeline::execute::{ExecuteConfig, execute_tasks, ExecutionStatistics};
use std::path::PathBuf;
use std::sync::Arc;

/// Test that AgentPool is used when provided in ExecuteConfig
#[tokio::test]
async fn test_agent_pool_integration_in_execute_config() {
    let pool = AgentPool::from_default_config();
    let config = ExecuteConfig {
        mode_config: ModeConfig::default(),
        max_retries: 3,
        timeout: 3600,
        enable_sessions: true,
        work_dir: std::env::current_dir().unwrap(),
        memory_file: PathBuf::from(".claude/memory.md"),
        enable_workspace_persistence: false,
        project_root: None,
        agent_pool: Some(pool),
    };

    // Verify agent_pool field is set
    assert!(config.agent_pool.is_some());
}

/// Test session reuse across tasks using AgentPool
#[tokio::test]
async fn test_session_reuse_across_tasks() {
    let pool = Arc::new(AgentPool::from_default_config());
    let mut task1 = Task::new("task-1", "First Task", "Description");
    let mut task2 = Task::new("task-2", "Second Task", "Description");

    // Get sessions for both tasks
    let session1 = pool
        .get_or_create_session_for_task(&mut task1, "claude", "claude-sonnet-4-6")
        .await;

    // Task 2 should reuse the session from task 1 (same agent/model)
    let session2 = pool
        .get_or_create_session_for_task(&mut task2, "claude", "claude-sonnet-4-6")
        .await;

    // Sessions should be the same for same agent/model
    assert_eq!(session1, session2, "Sessions should be reused for same agent/model");
}

/// Test session inheritance in dependency chains
#[tokio::test]
async fn test_session_inheritance_dependencies() {
    let pool = Arc::new(AgentPool::from_default_config());
    let mut parent_task = Task::new("task-1", "Parent", "Parent task");
    let mut child_task = Task::new("task-2", "Child", "Child task");

    child_task.depends_on = vec!["task-1".to_string()];

    // Get session for parent
    let parent_session = pool
        .get_or_create_session_for_task(&mut parent_task, "claude", "claude-sonnet-4-6")
        .await;

    // Set parent session ID
    child_task.set_parent_session_id(&parent_session);

    // Child should inherit parent's session
    let child_session = pool
        .get_or_create_session_for_task(&mut child_task, "claude", "claude-sonnet-4-6")
        .await;

    assert_eq!(
        parent_session, child_session,
        "Child task should inherit parent session"
    );
}

/// Test session reuse on retry
#[tokio::test]
async fn test_session_reuse_on_retry() {
    let pool = Arc::new(AgentPool::from_default_config());
    let mut task = Task::new("task-1", "Test", "Test task");

    // Get initial session
    let session1 = pool
        .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;

    // Prepare retry (should keep session)
    task.prepare_retry();

    // Get session again (should reuse)
    let session2 = pool
        .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;

    assert_eq!(session1, session2, "Session should be reused on retry");
}

/// Test concurrent access to AgentPool
#[tokio::test]
async fn test_concurrent_agent_pool_access() {
    use tokio::task::JoinSet;

    let pool = Arc::new(AgentPool::from_default_config());
    let mut tasks = JoinSet::new();

    // Spawn 10 concurrent tasks getting sessions
    for i in 0..10 {
        let pool_clone = Arc::clone(&pool);
        tasks.spawn(async move {
            let mut task = Task::new(&format!("task-{}", i), "Concurrent", "Test");
            pool_clone
                .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
                .await
        });
    }

    // Collect results
    let mut sessions = Vec::new();
    while let Some(result) = tasks.join_next().await {
        sessions.push(result.unwrap());
    }

    // All sessions should be the same (reused)
    assert!(
        sessions.iter().all(|s| s == &sessions[0]),
        "All concurrent sessions should be the same"
    );
}

/// Test AgentPool cleanup of stale sessions
#[tokio::test]
async fn test_agent_pool_cleanup() {
    let pool = AgentPool::from_default_config();
    let mut task = Task::new("task-1", "Test", "Test task");

    // Create a session
    let _session = pool
        .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;

    // Get stats before cleanup
    let stats_before = pool.stats().await;

    // Run cleanup (may not remove anything if not stale)
    let removed = pool.cleanup_stale_sessions().await;

    // Get stats after cleanup
    let stats_after = pool.stats().await;

    // Verify cleanup ran without error
    assert!(removed <= stats_before.total_sessions);
    assert!(stats_after.total_sessions <= stats_before.total_sessions);
}

/// Test AgentPool with custom configuration
#[tokio::test]
async fn test_agent_pool_custom_config() {
    let config = Config {
        pool: PoolConfig {
            max_sessions: 10,
            auto_cleanup: true,
            cleanup_interval_seconds: 60,
            stale_threshold_seconds: 300,
            enable_reuse: true,
        },
        warmup: WarmupConfig {
            enabled: false,
            max_queries: 0,
            timeout_seconds: 60,
            retry_on_failure: false,
            prompt_template: Some(String::new()),
        },
        ..Config::default()
    };

    let pool = AgentPool::new(&config);
    let stats = pool.stats().await;

    assert_eq!(stats.max_sessions, 10);
    assert!(!stats.warmup_enabled);
}

/// Test ExecuteConfig backward compatibility (without AgentPool)
#[tokio::test]
async fn test_execute_config_backward_compatibility() {
    // Config without agent_pool should work
    let config = ExecuteConfig {
        mode_config: ModeConfig::default(),
        max_retries: 3,
        timeout: 3600,
        enable_sessions: true,
        work_dir: std::env::current_dir().unwrap(),
        memory_file: PathBuf::from(".claude/memory.md"),
        enable_workspace_persistence: false,
        project_root: None,
        agent_pool: None, // No pool
    };

    // Verify it's None
    assert!(config.agent_pool.is_none());
}

/// Test that AgentPool is properly integrated with execute_tasks
#[tokio::test]
async fn test_execute_tasks_uses_agent_pool() {
    let pool = Arc::new(AgentPool::from_default_config());

    // Get initial stats
    let stats_before = pool.stats().await;

    // Create a simple task
    let task = Task::new("test-task", "Test Task", "Test description");

    let config = ExecuteConfig {
        mode_config: ModeConfig::default(),
        max_retries: 0,
        timeout: 60,
        enable_sessions: true,
        work_dir: std::env::current_dir().unwrap(),
        memory_file: PathBuf::from(".claude/memory.md"),
        enable_workspace_persistence: false,
        project_root: None,
        agent_pool: Some((*pool).clone()),
    };

    // Note: We can't actually execute the task without a real agent backend,
    // but we can verify the configuration is set up correctly
    assert!(config.agent_pool.is_some());

    // The pool should be ready for use
    let stats_after = pool.stats().await;
    assert_eq!(stats_after.total_sessions, stats_before.total_sessions);
}

/// Test session lifecycle management
#[tokio::test]
async fn test_session_lifecycle() {
    let pool = Arc::new(AgentPool::from_default_config());
    let mut task = Task::new("task-1", "Test", "Test task");

    // 1. Create session
    let session_id = pool
        .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;
    assert!(task.get_session_id().is_some());

    // 2. Verify session is stored in task
    assert_eq!(task.get_session_id(), Some(session_id.as_str()));

    // 3. Verify session can be reused
    let session_id_2 = pool
        .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;
    assert_eq!(session_id, session_id_2);
}

/// Test AgentPool statistics tracking
#[tokio::test]
async fn test_pool_statistics() {
    let pool = AgentPool::from_default_config();

    // Initial state
    let stats = pool.stats().await;
    assert_eq!(stats.total_sessions, 0);

    // After creating sessions
    let mut task1 = Task::new("task-1", "Test 1", "Description");
    let mut task2 = Task::new("task-2", "Test 2", "Description");

    pool.get_or_create_session_for_task(&mut task1, "claude", "claude-sonnet-4-6")
        .await;
    pool.get_or_create_session_for_task(&mut task2, "claude", "claude-sonnet-4-6")
        .await;

    let stats = pool.stats().await;
    // Should be 1 because sessions are reused for same agent/model
    assert_eq!(stats.total_sessions, 1);
}
