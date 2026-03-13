//! Integration tests for the Pipeline Orchestrator
//!
//! These tests verify the complete pipeline orchestration workflow including:
//! - Stage execution in correct order
//! - Stage transitions and state management
//! - Error propagation through pipeline stages
//! - Mode-based stage skipping (Fast skips Test, Expert adds Review)
//! - Parallel task execution respecting dependencies
//! - Progress reporting and result aggregation

use ltmatrix::models::{
    ExecutionMode, ModeConfig, PipelineStage, Task, TaskComplexity, TaskStatus,
};
use ltmatrix::pipeline::orchestrator::{OrchestratorConfig, PipelineOrchestrator};
use std::time::Duration;
use tempfile::TempDir;

/// Test default orchestrator configuration
#[tokio::test]
async fn test_orchestrator_config_default() {
    let config = OrchestratorConfig::default();

    assert_eq!(config.execution_mode, ExecutionMode::Standard);
    assert_eq!(config.max_parallel_tasks, 4);
    assert!(config.show_progress);
    assert!(config.agent_pool.is_none());
}

/// Test fast mode configuration
#[tokio::test]
async fn test_orchestrator_config_fast_mode() {
    let config = OrchestratorConfig::fast_mode();

    assert_eq!(config.execution_mode, ExecutionMode::Fast);
    // Fast mode should not run tests
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Fast);
    assert!(!stages.contains(&PipelineStage::Test));
}

/// Test expert mode configuration
#[tokio::test]
async fn test_orchestrator_config_expert_mode() {
    let config = OrchestratorConfig::expert_mode();

    assert_eq!(config.execution_mode, ExecutionMode::Expert);
    // Expert mode runs all stages
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);
    assert!(stages.contains(&PipelineStage::Test));
}

/// Test configuration builder pattern
#[tokio::test]
async fn test_orchestrator_config_builder() {
    let temp_dir = TempDir::new().unwrap();

    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_max_parallel(8)
        .with_progress(false);

    assert_eq!(config.work_dir, temp_dir.path());
    assert_eq!(config.max_parallel_tasks, 8);
    assert!(!config.show_progress);

    // Verify work_dir is propagated to sub-configs
    assert_eq!(config.execute_config.work_dir, temp_dir.path());
    assert_eq!(config.test_config.work_dir, temp_dir.path());
    assert_eq!(config.verify_config.work_dir, temp_dir.path());
    assert_eq!(config.commit_config.work_dir, temp_dir.path());
    assert_eq!(
        config.memory_config.project_root,
        Some(temp_dir.path().to_path_buf())
    );
}

/// Test orchestrator creation with valid work directory
#[tokio::test]
async fn test_orchestrator_creation_valid() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::default().with_work_dir(temp_dir.path());

    let result = PipelineOrchestrator::new(config);
    assert!(result.is_ok());
}

/// Test orchestrator creation fails with invalid work directory
#[tokio::test]
async fn test_orchestrator_creation_invalid_dir() {
    let config = OrchestratorConfig::default().with_work_dir("/nonexistent/path/12345");

    let result = PipelineOrchestrator::new(config);
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(e.to_string().contains("Working directory does not exist"));
    }
}

/// Test pipeline result success rate calculation
#[test]
fn test_pipeline_result_success_rate() {
    // Create a result via execution to get a valid instance
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orchestrator = PipelineOrchestrator::new(config).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();

    let result = rt.block_on(async {
        orchestrator
            .execute_pipeline("Test", ExecutionMode::Fast)
            .await
            .unwrap()
    });

    // Test success_rate method
    assert!(result.success_rate() >= 0.0);
    assert!(result.success_rate() <= 100.0);
}

/// Test that PipelineStage::pipeline_for_mode returns correct stages for Fast mode
#[test]
fn test_pipeline_stages_fast_mode() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Fast);

    // Fast mode should skip Test stage
    assert_eq!(stages.len(), 6);
    assert!(stages.contains(&PipelineStage::Generate));
    assert!(stages.contains(&PipelineStage::Assess));
    assert!(stages.contains(&PipelineStage::Execute));
    assert!(stages.contains(&PipelineStage::Verify));
    assert!(stages.contains(&PipelineStage::Commit));
    assert!(stages.contains(&PipelineStage::Memory));
    assert!(!stages.contains(&PipelineStage::Test));
}

/// Test that PipelineStage::pipeline_for_mode returns correct stages for Standard mode
#[test]
fn test_pipeline_stages_standard_mode() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);

    // Standard mode should include all stages
    assert_eq!(stages.len(), 7);
    assert!(stages.contains(&PipelineStage::Generate));
    assert!(stages.contains(&PipelineStage::Assess));
    assert!(stages.contains(&PipelineStage::Execute));
    assert!(stages.contains(&PipelineStage::Test));
    assert!(stages.contains(&PipelineStage::Verify));
    assert!(stages.contains(&PipelineStage::Commit));
    assert!(stages.contains(&PipelineStage::Memory));
}

/// Test that PipelineStage::pipeline_for_mode returns correct stages for Expert mode
#[test]
fn test_pipeline_stages_expert_mode() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    // Expert mode should include all stages plus Review
    assert_eq!(stages.len(), 8);
    assert!(stages.contains(&PipelineStage::Generate));
    assert!(stages.contains(&PipelineStage::Assess));
    assert!(stages.contains(&PipelineStage::Execute));
    assert!(stages.contains(&PipelineStage::Test));
    assert!(stages.contains(&PipelineStage::Review));
    assert!(stages.contains(&PipelineStage::Verify));
    assert!(stages.contains(&PipelineStage::Commit));
    assert!(stages.contains(&PipelineStage::Memory));

    // Verify Review is positioned between Test and Verify
    let test_idx = stages
        .iter()
        .position(|s| s == &PipelineStage::Test)
        .unwrap();
    let review_idx = stages
        .iter()
        .position(|s| s == &PipelineStage::Review)
        .unwrap();
    let verify_idx = stages
        .iter()
        .position(|s| s == &PipelineStage::Verify)
        .unwrap();
    assert!(review_idx > test_idx, "Review should come after Test");
    assert!(review_idx < verify_idx, "Review should come before Verify");
}

/// Test stage display names
#[test]
fn test_pipeline_stage_display_names() {
    assert_eq!(PipelineStage::Generate.display_name(), "Generate");
    assert_eq!(PipelineStage::Assess.display_name(), "Assess");
    assert_eq!(PipelineStage::Execute.display_name(), "Execute");
    assert_eq!(PipelineStage::Test.display_name(), "Test");
    assert_eq!(PipelineStage::Review.display_name(), "Code Review");
    assert_eq!(PipelineStage::Verify.display_name(), "Verify");
    assert_eq!(PipelineStage::Commit.display_name(), "Commit");
    assert_eq!(PipelineStage::Memory.display_name(), "Memory");
}

/// Test that stage requires agent detection works correctly
#[test]
fn test_pipeline_stage_requires_agent() {
    assert!(PipelineStage::Generate.requires_agent());
    assert!(PipelineStage::Assess.requires_agent());
    assert!(PipelineStage::Execute.requires_agent());
    assert!(PipelineStage::Test.requires_agent());
    assert!(PipelineStage::Review.requires_agent());
    assert!(PipelineStage::Verify.requires_agent());
    assert!(!PipelineStage::Commit.requires_agent());
    assert!(!PipelineStage::Memory.requires_agent());
}

/// Test execution mode test flag
#[test]
fn test_execution_mode_run_tests() {
    assert!(!ExecutionMode::Fast.run_tests());
    assert!(ExecutionMode::Standard.run_tests());
    assert!(ExecutionMode::Expert.run_tests());
}

/// Test execution mode defaults
#[test]
fn test_execution_mode_default() {
    assert_eq!(ExecutionMode::default(), ExecutionMode::Standard);
}

/// Test orchestrator execution with empty goal (should handle gracefully)
#[tokio::test]
async fn test_orchestrator_execution_empty_goal() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orchestrator = PipelineOrchestrator::new(config).unwrap();

    // Execute with empty goal - should not crash
    let result = orchestrator.execute_pipeline("", ExecutionMode::Fast).await;

    // The result should be valid even if stages fail
    assert!(result.is_ok());
}

/// Test orchestrator state initialization
#[tokio::test]
async fn test_orchestrator_state_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orchestrator = PipelineOrchestrator::new(config).unwrap();

    // Execute a simple pipeline
    let result = orchestrator
        .execute_pipeline("Test goal", ExecutionMode::Fast)
        .await;

    // Pipeline should complete without errors
    assert!(result.is_ok());

    // Verify that execution took some time (state was initialized)
    if let Ok(pipeline_result) = result {
        assert!(pipeline_result.total_time >= Duration::ZERO);
    }
}

/// Test orchestrator with different execution modes
#[tokio::test]
async fn test_orchestrator_different_modes() {
    let temp_dir = TempDir::new().unwrap();

    for mode in [
        ExecutionMode::Fast,
        ExecutionMode::Standard,
        ExecutionMode::Expert,
    ] {
        let config = OrchestratorConfig::default()
            .with_work_dir(temp_dir.path())
            .with_progress(false);

        let orchestrator = PipelineOrchestrator::new(config);
        assert!(
            orchestrator.is_ok(),
            "Failed to create orchestrator for {:?}",
            mode
        );

        if let Ok(orch) = orchestrator {
            let result = orch.execute_pipeline("Test goal", mode).await;
            assert!(result.is_ok(), "Failed to execute pipeline in {:?}", mode);
        }
    }
}

/// Test that orchestrator handles progress bar configuration
#[tokio::test]
async fn test_orchestrator_progress_configuration() {
    let temp_dir = TempDir::new().unwrap();

    // Test with progress enabled
    let config_with_progress = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(true);

    let orch_with = PipelineOrchestrator::new(config_with_progress);
    assert!(orch_with.is_ok());

    // Test with progress disabled
    let config_without_progress = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orch_without = PipelineOrchestrator::new(config_without_progress);
    assert!(orch_without.is_ok());
}

/// Test orchestrator with maximum parallel tasks configuration
#[tokio::test]
async fn test_orchestrator_max_parallel_configuration() {
    let temp_dir = TempDir::new().unwrap();

    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_max_parallel(16);

    assert_eq!(config.max_parallel_tasks, 16);

    let orchestrator = PipelineOrchestrator::new(config);
    assert!(orchestrator.is_ok());
}

/// Test execution mode model selection
#[test]
fn test_execution_mode_default_model() {
    assert_eq!(ExecutionMode::Fast.default_model(), "claude-haiku-4-5");
    assert_eq!(ExecutionMode::Standard.default_model(), "claude-sonnet-4-6");
    assert_eq!(ExecutionMode::Expert.default_model(), "claude-opus-4-6");
}

/// Test execution mode max depth
#[test]
fn test_execution_mode_max_depth() {
    assert_eq!(ExecutionMode::Fast.max_depth(), 2);
    assert_eq!(ExecutionMode::Standard.max_depth(), 3);
    assert_eq!(ExecutionMode::Expert.max_depth(), 3);
}

/// Test execution mode max retries
#[test]
fn test_execution_mode_max_retries() {
    assert_eq!(ExecutionMode::Fast.max_retries(), 1);
    assert_eq!(ExecutionMode::Standard.max_retries(), 3);
    assert_eq!(ExecutionMode::Expert.max_retries(), 3);
}

/// Test task status terminal states
#[test]
fn test_task_status_terminal() {
    assert!(TaskStatus::Completed.is_terminal());
    assert!(TaskStatus::Failed.is_terminal());
    assert!(!TaskStatus::Pending.is_terminal());
    assert!(!TaskStatus::InProgress.is_terminal());
    assert!(!TaskStatus::Blocked.is_terminal());
}

/// Test task helper methods
#[test]
fn test_task_helper_methods() {
    let mut task = Task::new("task-1", "Test", "Description");

    // Initial state
    assert!(!task.is_failed());
    assert!(!task.is_completed());
    // can_retry only returns true for failed tasks
    assert!(!task.can_retry(3));
    assert!(!task.has_session());

    // Set session
    task.set_session_id("session-123");
    assert!(task.has_session());
    assert_eq!(task.get_session_id(), Some("session-123"));

    // Set parent session
    task.set_parent_session_id("parent-456");
    assert_eq!(task.get_parent_session_id(), Some("parent-456"));

    // Clear session
    task.clear_session_id();
    assert!(!task.has_session());

    // Mark as failed
    task.status = TaskStatus::Failed;
    assert!(task.is_failed());
    // Now can_retry should return true
    assert!(task.can_retry(3));
    assert!(!task.can_retry(0));

    // Prepare retry
    task.prepare_retry();
    assert_eq!(task.retry_count, 1);
    assert_eq!(task.status, TaskStatus::Pending);

    // Mark as completed
    task.status = TaskStatus::Completed;
    assert!(task.is_completed());
    // Completed tasks cannot be retried
    assert!(!task.can_retry(3));
}

/// Test task dependency checking
#[test]
fn test_task_can_execute_with_dependencies() {
    let mut task = Task::new("task-1", "Test", "Description");
    task.depends_on = vec!["task-0".to_string(), "task-2".to_string()];

    let mut completed = std::collections::HashSet::new();

    // No dependencies completed
    assert!(!task.can_execute(&completed));

    // One dependency completed
    completed.insert("task-0".to_string());
    assert!(!task.can_execute(&completed));

    // All dependencies completed
    completed.insert("task-2".to_string());
    assert!(task.can_execute(&completed));
}

/// Test task elapsed time calculation
#[test]
fn test_task_elapsed_time() {
    let mut task = Task::new("task-1", "Test", "Description");

    // Pending task should have zero elapsed time
    assert_eq!(task.elapsed_time(), Duration::ZERO);

    // Set start time
    task.started_at = Some(chrono::Utc::now());
    std::thread::sleep(std::time::Duration::from_millis(10));

    // In-progress task should have positive elapsed time
    let elapsed = task.elapsed_time();
    assert!(elapsed > Duration::ZERO);

    // Complete the task
    task.completed_at = Some(chrono::Utc::now());
    let elapsed_completed = task.elapsed_time();
    assert!(elapsed_completed >= elapsed);
}

/// Test orchestrator error handling with invalid working directory
#[tokio::test]
async fn test_orchestrator_error_handling_invalid_dir() {
    let config = OrchestratorConfig::default().with_work_dir("/this/path/does/not/exist/12345");

    let result = PipelineOrchestrator::new(config);
    assert!(result.is_err());

    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(error_msg.contains("Working directory does not exist"));
        assert!(error_msg.contains("/this/path/does/not/exist/12345"));
    }
}

/// Test multiple orchestrator instances can be created
#[tokio::test]
async fn test_multiple_orchestrator_instances() {
    let temp_dir = TempDir::new().unwrap();

    let config1 = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let config2 = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_max_parallel(8)
        .with_progress(false);

    let orch1 = PipelineOrchestrator::new(config1);
    let orch2 = PipelineOrchestrator::new(config2);

    assert!(orch1.is_ok());
    assert!(orch2.is_ok());
}

/// Test orchestrator configuration cloning
#[test]
fn test_orchestrator_config_clone() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_max_parallel(8);

    let cloned = config.clone();

    assert_eq!(config.execution_mode, cloned.execution_mode);
    assert_eq!(config.work_dir, cloned.work_dir);
    assert_eq!(config.max_parallel_tasks, cloned.max_parallel_tasks);
    assert_eq!(config.show_progress, cloned.show_progress);
}

/// Test pipeline stage equality and ordering
#[test]
fn test_pipeline_stage_equality() {
    assert_eq!(PipelineStage::Generate, PipelineStage::Generate);
    assert_ne!(PipelineStage::Generate, PipelineStage::Execute);
}

/// Test execution mode equality
#[test]
fn test_execution_mode_equality() {
    assert_eq!(ExecutionMode::Fast, ExecutionMode::Fast);
    assert_ne!(ExecutionMode::Fast, ExecutionMode::Standard);
}

/// Test task complexity hash implementation (used for HashMap keys)
#[test]
fn test_task_complexity_hash() {
    use std::collections::HashMap;

    let mut map = HashMap::new();
    map.insert(TaskComplexity::Simple, "Fast");
    map.insert(TaskComplexity::Moderate, "Standard");
    map.insert(TaskComplexity::Complex, "Smart");

    assert_eq!(map.get(&TaskComplexity::Simple), Some(&"Fast"));
    assert_eq!(map.get(&TaskComplexity::Moderate), Some(&"Standard"));
    assert_eq!(map.get(&TaskComplexity::Complex), Some(&"Smart"));
}

/// Test task serialization/deserialization
#[test]
fn test_task_serialization() {
    let task = Task::new("task-1", "Test Task", "Test description");

    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: Task = serde_json::from_str(&serialized).unwrap();

    assert_eq!(task.id, deserialized.id);
    assert_eq!(task.title, deserialized.title);
    assert_eq!(task.description, deserialized.description);
    assert_eq!(task.status, deserialized.status);
    assert_eq!(task.complexity, deserialized.complexity);
}

/// Test execution mode serialization
#[test]
fn test_execution_mode_serialization() {
    let modes = vec![
        ExecutionMode::Fast,
        ExecutionMode::Standard,
        ExecutionMode::Expert,
    ];

    for mode in modes {
        let serialized = serde_json::to_string(&mode).unwrap();
        let deserialized: ExecutionMode = serde_json::from_str(&serialized).unwrap();
        assert_eq!(mode, deserialized);
    }
}

/// Test pipeline stage serialization
#[test]
fn test_pipeline_stage_serialization() {
    let stages = vec![
        PipelineStage::Generate,
        PipelineStage::Assess,
        PipelineStage::Execute,
        PipelineStage::Test,
        PipelineStage::Verify,
        PipelineStage::Commit,
        PipelineStage::Memory,
    ];

    for stage in stages {
        let serialized = serde_json::to_string(&stage).unwrap();
        let deserialized: PipelineStage = serde_json::from_str(&serialized).unwrap();
        assert_eq!(stage, deserialized);
    }
}

/// Test ModeConfig serialization
#[test]
fn test_mode_config_serialization() {
    let config = ModeConfig::default();

    let serialized = serde_json::to_string(&config).unwrap();
    let deserialized: ModeConfig = serde_json::from_str(&serialized).unwrap();

    assert_eq!(config.model_fast, deserialized.model_fast);
    assert_eq!(config.model_smart, deserialized.model_smart);
    assert_eq!(config.run_tests, deserialized.run_tests);
    assert_eq!(config.max_retries, deserialized.max_retries);
}

/// Test ModeConfig model selection
#[test]
fn test_mode_config_model_selection() {
    let config = ModeConfig::default();

    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Simple),
        "claude-sonnet-4-6"
    );
    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Moderate),
        "claude-sonnet-4-6"
    );
    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Complex),
        "claude-opus-4-6"
    );
}

/// Test orchestrator with very long goal
#[tokio::test]
async fn test_orchestrator_long_goal() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orchestrator = PipelineOrchestrator::new(config).unwrap();

    // Create a very long goal (1000 characters)
    let long_goal = "Build a comprehensive system ".repeat(50);

    let result = orchestrator
        .execute_pipeline(&long_goal, ExecutionMode::Fast)
        .await;
    assert!(result.is_ok());
}

/// Test orchestrator with custom working directory
#[tokio::test]
async fn test_orchestrator_custom_work_dir() {
    let temp_dir = TempDir::new().unwrap();
    let custom_dir = temp_dir.path().join("custom");

    std::fs::create_dir(&custom_dir).unwrap();

    let config = OrchestratorConfig::default()
        .with_work_dir(&custom_dir)
        .with_progress(false);

    // Verify the config was created with the custom directory before moving
    assert_eq!(config.work_dir, custom_dir);

    let orchestrator = PipelineOrchestrator::new(config).unwrap();

    let result = orchestrator
        .execute_pipeline("Test custom dir", ExecutionMode::Fast)
        .await;
    assert!(result.is_ok());
}

/// Test pipeline execution time tracking
#[tokio::test]
async fn test_orchestrator_execution_time_tracking() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orchestrator = PipelineOrchestrator::new(config).unwrap();

    let start = std::time::Instant::now();
    let result = orchestrator
        .execute_pipeline("Time tracking test", ExecutionMode::Fast)
        .await;

    assert!(result.is_ok());

    let pipeline_result = result.unwrap();
    let elapsed = start.elapsed();

    // Pipeline result should have timing info
    assert!(pipeline_result.total_time <= elapsed);
}

/// Test pipeline handles goal with special characters
#[tokio::test]
async fn test_orchestrator_special_characters_in_goal() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orchestrator = PipelineOrchestrator::new(config).unwrap();

    let goals = vec![
        "Build API with <html> tags",
        "Create \"quoted\" and 'single-quoted' strings",
        "Use &lt;entity&gt; encoding",
        "Test\nmultiline\ngoals",
        "Use emoji 🚀 and unicode™",
    ];

    for goal in goals {
        let result = orchestrator
            .execute_pipeline(goal, ExecutionMode::Fast)
            .await;
        assert!(result.is_ok(), "Failed for goal: {}", goal);
    }
}

/// Test pipeline sequential execution (concurrent test removed due to git2::Sync constraints)
#[tokio::test]
async fn test_orchestrator_sequential_execution() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orchestrator = PipelineOrchestrator::new(config).unwrap();

    // Execute multiple pipelines sequentially
    for i in 0..3 {
        let result = orchestrator
            .execute_pipeline(&format!("Sequential test {}", i), ExecutionMode::Fast)
            .await;
        assert!(result.is_ok(), "Failed on iteration {}", i);
    }
}

/// Test pipeline with zero parallel tasks
#[tokio::test]
async fn test_orchestrator_zero_parallel_tasks() {
    let temp_dir = TempDir::new().unwrap();

    // Note: max_parallel_tasks = 0 is technically valid but may cause issues
    // This test verifies the orchestrator accepts the configuration
    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_max_parallel(0)
        .with_progress(false);

    // Verify config was created correctly before moving
    assert_eq!(config.max_parallel_tasks, 0);

    let orchestrator = PipelineOrchestrator::new(config);
    assert!(orchestrator.is_ok());
}

/// Test pipeline with very large parallel task count
#[tokio::test]
async fn test_orchestrator_large_parallel_tasks() {
    let temp_dir = TempDir::new().unwrap();

    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_max_parallel(1000)
        .with_progress(false);

    // Verify config was created correctly before moving
    assert_eq!(config.max_parallel_tasks, 1000);

    let orchestrator = PipelineOrchestrator::new(config);
    assert!(orchestrator.is_ok());
}
