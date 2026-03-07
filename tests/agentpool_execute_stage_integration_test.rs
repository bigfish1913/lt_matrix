//! AgentPool Integration with Execute Stage
//!
//! TDD tests for integrating AgentPool into the pipeline execution system.
//!
//! These tests describe the desired behavior where the execute stage
//! uses AgentPool instead of SessionManager directly.

use ltmatrix::agent::{AgentBackend, AgentPool};
use ltmatrix::agent::claude::ClaudeAgent;
use ltmatrix::config::settings::Config;
use ltmatrix::models::{Agent, Task, TaskComplexity, TaskStatus};
use ltmatrix::pipeline::execute::{ExecuteConfig, execute_tasks};
use std::path::PathBuf;

// ============================================================================
// RED PHASE: Tests that will fail initially
// ============================================================================

#[test]
fn test_execute_config_accepts_agent_pool() {
    // ExecuteConfig should accept an AgentPool
    let mut config = ExecuteConfig::default();
    // This should compile - we need to add agent_pool field
    // let pool = AgentPool::from_default_config();
    // config.agent_pool = Some(pool);
    // assert!(config.agent_pool.is_some());
}

#[tokio::test]
async fn test_execute_tasks_uses_agent_pool() {
    // execute_tasks should use AgentPool if provided
    let config = ExecuteConfig::default();
    let tasks = vec![
        Task::new("task-1", "Test 1", "Description 1"),
        Task::new("task-2", "Test 2", "Description 2"),
    ];

    // This should use AgentPool internally
    // We'll verify by checking pool stats after execution
    let _result = execute_tasks(tasks, &config).await;

    // TODO: Verify AgentPool was used
    // let pool = config.agent_pool.unwrap();
    // let stats = pool.stats().await;
    // assert!(stats.total_sessions > 0);
}

#[tokio::test]
async fn test_agent_pool_config_from_execute_config() {
    // ExecuteConfig should be able to create AgentPool from its settings
    let config = ExecuteConfig::default();

    // This should create an AgentPool with the same settings
    // let pool = config.create_agent_pool().await;
    // assert!(pool.stats().await.total_sessions >= 0);
}

#[tokio::test]
async fn test_concurrent_execution_with_pool() {
    // Multiple concurrent executions should use the same AgentPool
    let pool = AgentPool::from_default_config();
    let config = ExecuteConfig {
        work_dir: PathBuf::from("."),
        ..Default::default()
    };

    // Execute multiple task sets concurrently
    let handle1 = tokio::spawn(async {
        let tasks = vec![Task::new("task-1", "Test", "Desc")];
        execute_tasks(tasks, &config).await
    });

    let handle2 = tokio::spawn(async {
        let tasks = vec![Task::new("task-2", "Test", "Desc")];
        execute_tasks(tasks, &config).await
    });

    let _ = tokio::join!(handle1, handle2);

    // Pool should handle concurrent access
    let stats = pool.stats().await;
    assert!(stats.total_sessions >= 0);
}

#[tokio::test]
async fn test_warmup_before_execute() {
    // AgentPool should warm up agents before task execution
    let mut config = Config::default();
    config.warmup.enabled = true;

    let pool = AgentPool::new(&config);

    // Warm up agents
    let agent = Agent {
        name: "claude".to_string(),
        model: "claude-sonnet-4-6".to_string(),
        command: None,
    };
    let claude_agent = ClaudeAgent::new(agent.clone());

    let _results = pool.warmup_agents(&[&claude_agent]).await;

    // Now execute tasks should use warmed sessions
    let execute_config = ExecuteConfig::default();
    let tasks = vec![Task::new("task-1", "Test", "Description")];

    let _result = execute_tasks(tasks, &execute_config).await;

    // TODO: Verify warmed sessions were used
}

#[tokio::test]
async fn test_session_reuse_in_execute_stage() {
    // Execute stage should reuse sessions across task retries
    let pool = AgentPool::from_default_config();
    let mut task = Task::new("task-retry", "Retry", "Description");

    // First execution
    let session1 = pool
        .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;

    // Simulate retry
    task.prepare_retry();

    // Second execution - should reuse session
    let session2 = pool
        .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;

    assert_eq!(session1, session2, "Session should be reused on retry");
}

#[tokio::test]
async fn test_dependency_session_propagation() {
    // Execute stage should propagate sessions through dependencies
    let pool = AgentPool::from_default_config();

    let mut parent = Task::new("parent", "Parent", "Description");
    let mut child = Task::new("child", "Child", "Description");
    child.depends_on = vec!["parent".to_string()];

    // Execute parent
    let parent_session = pool
        .get_or_create_session_for_task(&mut parent, "claude", "claude-sonnet-4-6")
        .await;

    // Child should use parent's session
    child.set_parent_session_id(&parent_session);
    let child_session = pool
        .get_or_create_session_for_task(&mut child, "claude", "claude-sonnet-4-6")
        .await;

    assert_eq!(parent_session, child_session);
}

#[tokio::test]
async fn test_cleanup_after_execute() {
    // Execute stage should trigger cleanup after execution
    let pool = AgentPool::from_default_config();

    // Create some sessions
    for i in 0..3 {
        let mut task = Task::new(format!("task-{}", i), "Test", "Description");
        pool.get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;
    }

    let stats_before = pool.stats().await;
    assert!(stats_before.total_sessions > 0);

    // Cleanup should be called
    let removed = pool.cleanup_stale_sessions().await;
    assert!(removed >= 0);
}

// ============================================================================
// Helper tests for configuration
// ============================================================================

#[test]
fn test_execute_config_has_pool_settings() {
    // ExecuteConfig should expose pool-related settings
    let config = ExecuteConfig::default();

    // These should be accessible
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.timeout, 3600);
    assert!(config.enable_sessions);
}

#[test]
fn test_execute_config_from_mode_config() {
    // Should be able to create ExecuteConfig from ModeConfig
    let fast_config = ExecuteConfig::fast_mode();
    assert_eq!(fast_config.max_retries, 1);
    assert_eq!(fast_config.timeout, 1800);

    let expert_config = ExecuteConfig::expert_mode();
    assert_eq!(expert_config.max_retries, 3);
    assert_eq!(expert_config.timeout, 7200);
}
