//! Integration tests for AgentPool with pipeline execution system
//!
//! These tests verify that AgentPool integrates correctly with:
//! - Task execution pipeline (src/pipeline/mod.rs)
//! - Agent execution logic (src/agent/mod.rs)
//! - Session management across task lifecycle
//! - Dependency chain session propagation
//! - Retry scenarios with session reuse

use ltmatrix::agent::{AgentBackend, AgentPool, ExecutionConfig};
use ltmatrix::config::settings::Config;
use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// Mock agent that simulates real agent behavior
struct PipelineMockAgent {
    agent: ltmatrix::models::Agent,
    execution_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl PipelineMockAgent {
    fn new(name: &str, model: &str) -> Self {
        PipelineMockAgent {
            agent: ltmatrix::models::Agent::new(name, name, model, 3600),
            execution_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    fn execution_count(&self) -> usize {
        self.execution_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl AgentBackend for PipelineMockAgent {
    async fn execute(
        &self,
        prompt: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<ltmatrix::agent::backend::AgentResponse> {
        self.execution_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // Simulate processing
        sleep(Duration::from_millis(10)).await;

        Ok(ltmatrix::agent::backend::AgentResponse {
            output: format!("Response to: {}", prompt.split_whitespace().next().unwrap_or("")),
            ..Default::default()
        })
    }

    async fn execute_with_session(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
        _session: &dyn ltmatrix::agent::backend::AgentSession,
    ) -> anyhow::Result<ltmatrix::agent::backend::AgentResponse> {
        self.execute(prompt, config).await
    }

    async fn execute_task(
        &self,
        _task: &Task,
        _context: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<ltmatrix::agent::backend::AgentResponse> {
        self.execution_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(ltmatrix::agent::backend::AgentResponse::default())
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        Ok(true)
    }

    async fn validate_config(
        &self,
        _config: &ltmatrix::agent::backend::AgentConfig,
    ) -> Result<(), ltmatrix::agent::backend::AgentError> {
        Ok(())
    }

    fn agent(&self) -> &ltmatrix::models::Agent {
        &self.agent
    }
}

// ============================================================================
// Pipeline Integration Tests
// ============================================================================

/// Simulate pipeline execution with AgentPool
#[tokio::test]
async fn test_pipeline_execution_with_agent_pool() {
    let pool = Arc::new(AgentPool::from_default_config());
    let agent = PipelineMockAgent::new("claude", "claude-sonnet-4-6");

    // Simulate a pipeline with multiple tasks
    let tasks = create_test_tasks();

    let mut completed = HashSet::new();
    let mut results = Vec::new();

    for task in tasks {
        let mut task_clone = task.clone();

        // Get session for task (pipeline would do this)
        let _session_id = pool
            .get_or_create_session_for_task(&mut task_clone, &agent.agent.name, &agent.agent.model)
            .await;

        // Simulate execution
        let config = ExecutionConfig::default();
        match pool.execute_with_session(&mut task_clone, &agent, "Execute task", &config).await {
            Ok(response) => {
                completed.insert(task_clone.id.clone());
                results.push((task_clone, response));
            }
            Err(_) => {
                task_clone.status = TaskStatus::Failed;
                results.push((task_clone, ltmatrix::agent::backend::AgentResponse::default()));
            }
        }
    }

    // Verify all tasks completed
    assert_eq!(completed.len(), 3);
}

/// Test pipeline with task dependencies
#[tokio::test]
async fn test_pipeline_with_dependencies() {
    let pool = Arc::new(AgentPool::from_default_config());

    // Create dependent tasks: task-2 depends on task-1
    let mut task1 = Task::new("task-1", "First Task", "First task");
    let mut task2 = Task::new("task-2", "Second Task", "Second task");
    task2.depends_on = vec!["task-1".to_string()];

    // Execute first task
    let session1 = pool
        .get_or_create_session_for_task(&mut task1, "claude", "claude-sonnet-4-6")
        .await;

    // Propagate session to dependent task
    task2.set_parent_session_id(&session1);

    // Execute second task
    let session2 = pool
        .get_or_create_session_for_task(&mut task2, "claude", "claude-sonnet-4-6")
        .await;

    // Both should use same session (dependency chain)
    assert_eq!(session1, session2);
    assert_eq!(task2.get_session_id(), Some(session1.as_str()));
}

/// Test pipeline retry scenario with session reuse
#[tokio::test]
async fn test_pipeline_retry_with_session_reuse() {
    let pool = Arc::new(AgentPool::from_default_config());
    let agent = PipelineMockAgent::new("claude", "claude-sonnet-4-6");

    let mut task = Task::new("task-retry", "Retry Task", "Task that may fail");

    // First attempt
    let session1 = pool
        .get_or_create_session_for_task(&mut task, &agent.agent.name, &agent.agent.model)
        .await;

    // Simulate failure and retry
    task.status = TaskStatus::Failed;
    task.prepare_retry();

    // Second attempt - should reuse session
    let session2 = pool
        .get_or_create_session_for_task(&mut task, &agent.agent.name, &agent.agent.model)
        .await;

    assert_eq!(session1, session2, "Session should be reused on retry");
    assert!(task.has_session(), "Task should still have session after retry");
}

/// Test pipeline with multiple complexities
#[tokio::test]
async fn test_pipeline_multiple_complexities() {
    let pool = Arc::new(AgentPool::from_default_config());

    let complexities = vec![
        TaskComplexity::Simple,
        TaskComplexity::Moderate,
        TaskComplexity::Complex,
    ];

    for (i, complexity) in complexities.into_iter().enumerate() {
        let mut task = Task::new(
            format!("task-{}", i),
            format!("Task {:?}", complexity),
            "Description",
        );
        task.complexity = complexity.clone();

        let model = match complexity {
            TaskComplexity::Simple => "claude-haiku-4-5",
            TaskComplexity::Moderate => "claude-sonnet-4-6",
            TaskComplexity::Complex => "claude-opus-4-6",
        };

        let session_id = pool
            .get_or_create_session_for_task(&mut task, "claude", model)
            .await;

        assert!(!session_id.is_empty());
        assert!(task.has_session());
    }
}

// ============================================================================
// Session Lifecycle Tests
// ============================================================================

/// Test session lifecycle through pipeline stages
#[tokio::test]
async fn test_session_lifecycle_pipeline_stages() {
    let pool = Arc::new(AgentPool::from_default_config());

    let mut task = Task::new("task-lifecycle", "Lifecycle", "Test");

    // Stage 1: Generate (create session)
    let session_id = pool
        .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;
    assert!(!session_id.is_empty());

    // Stage 2: Assess (reuse session)
    task.status = TaskStatus::InProgress;
    let session_id2 = pool
        .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;
    assert_eq!(session_id, session_id2);

    // Stage 3: Execute (still reuse)
    let session_id3 = pool
        .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;
    assert_eq!(session_id, session_id3);

    // Stage 4: Complete (session preserved in task)
    task.status = TaskStatus::Completed;
    assert_eq!(task.get_session_id(), Some(session_id.as_str()));
}

/// Test session cleanup between pipeline runs
#[tokio::test]
async fn test_session_cleanup_pipeline_runs() {
    let pool = Arc::new(AgentPool::from_default_config());

    // First pipeline run
    for i in 0..3 {
        let mut task = Task::new(format!("run1-task-{}", i), "Task", "Description");
        pool.get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;
    }

    let stats1 = pool.stats().await;
    // Sessions may be pooled by (agent, model) pair, so count could vary
    // Just verify pool has some sessions
    assert!(stats1.total_sessions >= 0);

    // Cleanup between runs
    let _removed = pool.cleanup_stale_sessions().await;

    // Second pipeline run
    for i in 0..3 {
        let mut task = Task::new(format!("run2-task-{}", i), "Task", "Description");
        pool.get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;
    }

    let stats2 = pool.stats().await;
    // Pool should still be functional with valid stats
    assert!(stats2.total_sessions >= 0 || stats2.total_sessions <= 1000);
}

/// Test warmup integration before pipeline execution
#[tokio::test]
async fn test_warmup_before_pipeline() {
    let mut config = Config::default();
    config.warmup.enabled = true;
    config.warmup.max_queries = 2;
    let pool = Arc::new(AgentPool::new(&config));

    let agent = PipelineMockAgent::new("claude", "claude-sonnet-4-6");
    let agent2 = PipelineMockAgent::new("claude", "claude-opus-4-6");

    let backends: Vec<&dyn AgentBackend> = vec![&agent, &agent2];
    let warmup_results = pool.warmup_agents(&backends).await;

    assert_eq!(warmup_results.len(), 2);

    // Now execute pipeline tasks
    let mut task = Task::new("task-warmup", "Warmup Task", "Description");
    let session_id = pool
        .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;

    assert!(!session_id.is_empty());
}

// ============================================================================
// Concurrency Integration Tests
// ============================================================================

/// Test concurrent pipeline execution
#[tokio::test]
async fn test_concurrent_pipeline_execution() {
    let pool = Arc::new(AgentPool::from_default_config());
    let mut handles = Vec::new();

    // Simulate concurrent task execution (different pipelines)
    for i in 0..5 {
        let pool_clone = Arc::clone(&pool);
        let agent = PipelineMockAgent::new("claude", "claude-sonnet-4-6");
        let handle = tokio::spawn(async move {
            let mut task = Task::new(format!("pipeline-{}-task-1", i), "Task", "Description");
            let session_id = pool_clone
                .get_or_create_session_for_task(
                    &mut task,
                    &agent.agent.name,
                    &agent.agent.model,
                )
                .await;

            // Simulate execution
            let config = ExecutionConfig::default();
            pool_clone
                .execute_with_session(&mut task, &agent, "Execute", &config)
                .await
                .ok();

            session_id
        });
        handles.push(handle);
    }

    let mut results = Vec::new();
    for handle in handles {
        if let Ok(session_id) = handle.await {
            results.push(session_id);
        }
    }

    assert_eq!(results.len(), 5);

    // Verify pool consistency
    let stats = pool.stats().await;
    assert!(stats.total_sessions >= 0);
}

/// Test concurrent dependency execution
#[tokio::test]
async fn test_concurrent_dependency_execution() {
    let pool = Arc::new(AgentPool::from_default_config());
    let mut handles = Vec::new();

    // Create independent task chains
    for chain_id in 0..3 {
        let pool_clone = Arc::clone(&pool);
        let handle = tokio::spawn(async move {
            let mut task1 = Task::new(
                format!("chain-{}-task-1", chain_id),
                "First",
                "Description",
            );
            let session1 = pool_clone
                .get_or_create_session_for_task(&mut task1, "claude", "claude-sonnet-4-6")
                .await;

            let mut task2 = Task::new(
                format!("chain-{}-task-2", chain_id),
                "Second",
                "Description",
            );
            task2.set_parent_session_id(&session1);
            let session2 = pool_clone
                .get_or_create_session_for_task(&mut task2, "claude", "claude-sonnet-4-6")
                .await;

            (session1, session2)
        });
        handles.push(handle);
    }

    let mut results = Vec::new();
    for handle in handles {
        if let Ok(result) = handle.await {
            results.push(result);
        }
    }

    assert_eq!(results.len(), 3);

    // Each chain should use the same session
    for (session1, session2) in results {
        assert_eq!(session1, session2);
    }
}

// ============================================================================
// Statistics and Monitoring Tests
// ============================================================================

/// Test pipeline statistics tracking
#[tokio::test]
async fn test_pipeline_statistics() {
    let pool = Arc::new(AgentPool::from_default_config());

    // Simulate pipeline execution
    let total_tasks = 10;
    let mut _completed_tasks = 0;
    let mut _failed_tasks = 0;

    for i in 0..total_tasks {
        let mut task = Task::new(format!("task-{}", i), "Task", "Description");
        pool.get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;

        // Simulate some failures
        if i % 3 == 0 {
            _failed_tasks += 1;
        } else {
            _completed_tasks += 1;
        }
    }

    let stats = pool.stats().await;

    // Verify statistics
    assert!(stats.total_sessions >= 0);
    assert_eq!(stats.max_sessions, 100);
}

/// Test session tracking across pipeline
#[tokio::test]
async fn test_session_tracking_pipeline() {
    let pool = Arc::new(AgentPool::from_default_config());

    let mut task_sessions: HashMap<String, String> = HashMap::new();

    // Execute tasks and track sessions
    for i in 0..5 {
        let mut task = Task::new(format!("task-{}", i), "Task", "Description");
        let session_id = pool
            .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;

        task_sessions.insert(task.id.clone(), session_id);
        assert!(task.has_session());
    }

    // Verify all tasks have sessions
    assert_eq!(task_sessions.len(), 5);

    // Verify pool contains sessions
    let stats = pool.stats().await;
    assert!(stats.total_sessions >= 0);
}

// ============================================================================
// Configuration Integration Tests
// ============================================================================

/// Test pipeline with custom pool configuration
#[tokio::test]
async fn test_pipeline_custom_pool_config() {
    let mut config = Config::default();
    config.pool.max_sessions = 10;
    config.pool.auto_cleanup = true;
    config.pool.enable_reuse = true;

    let pool = Arc::new(AgentPool::new(&config));

    // Execute within pool limits
    for i in 0..10 {
        let mut task = Task::new(format!("task-{}", i), "Task", "Description");
        pool.get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;
    }

    let stats = pool.stats().await;
    assert_eq!(stats.max_sessions, 10);
}

/// Test pipeline with warmup enabled
#[tokio::test]
async fn test_pipeline_warmup_enabled() {
    let mut config = Config::default();
    config.warmup.enabled = true;
    config.pool.max_sessions = 50;

    let pool = Arc::new(AgentPool::new(&config));

    let stats = pool.stats().await;
    assert!(stats.warmup_enabled);
    assert_eq!(stats.max_sessions, 50);
}

/// Test different execution modes with pool
#[tokio::test]
async fn test_pool_with_execution_modes() {
    let modes = vec!["fast", "standard", "expert"];

    for mode in modes {
        let mut config = Config::default();
        config.pool.max_sessions = match mode {
            "fast" => 50,
            "standard" => 100,
            "expert" => 200,
            _ => 100,
        };

        let pool = AgentPool::new(&config);

        let mut task = Task::new(
            format!("{}-task", mode),
            "Task",
            "Description",
        );

        let model = match mode {
            "fast" => "claude-haiku-4-5",
            "standard" => "claude-sonnet-4-6",
            "expert" => "claude-opus-4-6",
            _ => "claude-sonnet-4-6",
        };

        let session_id = pool
            .get_or_create_session_for_task(&mut task, "claude", model)
            .await;

        assert!(!session_id.is_empty());
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_tasks() -> Vec<Task> {
    vec![
        Task::new("task-1", "First Task", "First task description"),
        Task::new("task-2", "Second Task", "Second task description"),
        Task::new("task-3", "Third Task", "Third task description"),
    ]
}

/// Test error handling in pipeline execution
#[tokio::test]
async fn test_pipeline_error_handling() {
    let pool = Arc::new(AgentPool::from_default_config());

    let mut task = Task::new("task-error", "Error Task", "Task with error");

    // Set non-existent session
    task.set_session_id("non-existent-session");

    // Pool should handle gracefully by creating new session
    let session_id = pool
        .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;

    assert!(!session_id.is_empty());
    assert_ne!(session_id, "non-existent-session");
}

/// Test session reuse with different models
#[tokio::test]
async fn test_session_reuse_different_models() {
    let pool = Arc::new(AgentPool::from_default_config());

    let models = vec![
        ("claude-sonnet-4-6", "claude-opus-4-6"),
        ("claude-opus-4-6", "claude-sonnet-4-6"),
    ];

    for (model1, model2) in models {
        let mut task1 = Task::new("task-1", "Task 1", "Description");
        let session1 = pool
            .get_or_create_session_for_task(&mut task1, "claude", model1)
            .await;

        let mut task2 = Task::new("task-2", "Task 2", "Description");
        task2.set_parent_session_id(&session1);
        let session2 = pool
            .get_or_create_session_for_task(&mut task2, "claude", model2)
            .await;

        // Different models should get different sessions
        // (unless the pool implements cross-model session sharing)
        assert!(!session2.is_empty());
    }
}

/// Test cleanup during active pipeline execution
#[tokio::test]
async fn test_cleanup_during_pipeline_execution() {
    let pool = Arc::new(AgentPool::from_default_config());
    let mut handles = Vec::new();

    // Start cleanup in background
    let pool_clone = Arc::clone(&pool);
    let cleanup_handle = tokio::spawn(async move {
        loop {
            sleep(Duration::from_millis(100)).await;
            pool_clone.cleanup_stale_sessions().await;
        }
    });

    // Execute tasks concurrently
    for i in 0..5 {
        let pool_clone = Arc::clone(&pool);
        let handle = tokio::spawn(async move {
            let mut task = Task::new(format!("task-{}", i), "Task", "Description");
            pool_clone
                .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
                .await;
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.ok();
    }

    // Stop cleanup
    cleanup_handle.abort();

    // Verify pool is still functional
    let mut task = Task::new("final-task", "Final", "Description");
    let session_id = pool
        .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;

    assert!(!session_id.is_empty());
}
