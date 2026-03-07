//! Comprehensive tests for the execute stage (public API only)
//!
//! These tests verify the execute stage using only publicly accessible APIs.
//! Private functions are tested within the module itself in src/pipeline/execute.rs

use ltmatrix::models::{ModeConfig, Task, TaskComplexity, TaskStatus};
use ltmatrix::pipeline::execute::{
    display_execution_statistics, ExecuteConfig, ExecutionStatistics,
};
use std::collections::HashSet;
use std::path::PathBuf;

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_execute_config_default() {
    let config = ExecuteConfig::default();

    assert_eq!(config.max_retries, 3);
    assert_eq!(config.timeout, 3600);
    assert!(config.enable_sessions);
    assert_eq!(config.mode_config.model_fast, "claude-sonnet-4-6");
    assert_eq!(config.mode_config.model_smart, "claude-opus-4-6");
}

#[test]
fn test_execute_config_fast_mode() {
    let config = ExecuteConfig::fast_mode();

    assert_eq!(config.max_retries, 1);
    assert_eq!(config.timeout, 1800);
    assert!(config.enable_sessions);
    assert_eq!(config.mode_config.model_fast, "claude-haiku-4-5");
    assert_eq!(config.mode_config.model_smart, "claude-sonnet-4-6");
}

#[test]
fn test_execute_config_expert_mode() {
    let config = ExecuteConfig::expert_mode();

    assert_eq!(config.max_retries, 3);
    assert_eq!(config.timeout, 7200);
    assert!(config.enable_sessions);
    assert_eq!(config.mode_config.model_fast, "claude-opus-4-6");
    assert_eq!(config.mode_config.model_smart, "claude-opus-4-6");
}

#[test]
fn test_execute_config_custom_paths() {
    let config = ExecuteConfig {
        mode_config: ModeConfig::default(),
        max_retries: 5,
        timeout: 5400,
        enable_sessions: false,
        work_dir: PathBuf::from("/custom/work"),
        memory_file: PathBuf::from("/custom/memory.md"),
        enable_workspace_persistence: false,
        project_root: None,
        agent_pool: None,
    };

    assert_eq!(config.work_dir, PathBuf::from("/custom/work"));
    assert_eq!(config.memory_file, PathBuf::from("/custom/memory.md"));
    assert!(!config.enable_sessions);
}

// ============================================================================
// Execution Statistics Tests
// ============================================================================

#[test]
fn test_execution_statistics_fields() {
    let stats = ExecutionStatistics {
        total_tasks: 100,
        completed_tasks: 85,
        failed_tasks: 10,
        total_retries: 25,
        total_time: 7200,
        simple_tasks: 40,
        moderate_tasks: 35,
        complex_tasks: 25,
        sessions_reused: 60,
    };

    assert_eq!(stats.total_tasks, 100);
    assert_eq!(stats.completed_tasks, 85);
    assert_eq!(stats.failed_tasks, 10);
    assert_eq!(stats.total_retries, 25);
    assert_eq!(stats.total_time, 7200);
    assert_eq!(stats.simple_tasks, 40);
    assert_eq!(stats.moderate_tasks, 35);
    assert_eq!(stats.complex_tasks, 25);
    assert_eq!(stats.sessions_reused, 60);
}

#[test]
fn test_execution_statistics_calculations() {
    let stats = ExecutionStatistics {
        total_tasks: 10,
        completed_tasks: 7,
        failed_tasks: 2,
        total_retries: 5,
        total_time: 1800,
        simple_tasks: 3,
        moderate_tasks: 4,
        complex_tasks: 3,
        sessions_reused: 4,
    };

    // Verify complexity breakdown sums to total
    assert_eq!(
        stats.simple_tasks + stats.moderate_tasks + stats.complex_tasks,
        10
    );

    // Verify completed + failed <= total
    assert!(stats.completed_tasks + stats.failed_tasks <= stats.total_tasks);
}

#[test]
fn test_execution_statistics_all_zeros() {
    let stats = ExecutionStatistics {
        total_tasks: 0,
        completed_tasks: 0,
        failed_tasks: 0,
        total_retries: 0,
        total_time: 0,
        simple_tasks: 0,
        moderate_tasks: 0,
        complex_tasks: 0,
        sessions_reused: 0,
    };

    assert_eq!(stats.total_tasks, 0);
    assert_eq!(stats.completed_tasks, 0);
    assert_eq!(stats.failed_tasks, 0);
}

#[test]
fn test_display_execution_statistics_no_panic() {
    let stats = ExecutionStatistics {
        total_tasks: 10,
        completed_tasks: 8,
        failed_tasks: 1,
        total_retries: 3,
        total_time: 300,
        simple_tasks: 4,
        moderate_tasks: 4,
        complex_tasks: 2,
        sessions_reused: 5,
    };

    // Should not panic
    display_execution_statistics(&stats);
}

#[test]
fn test_display_statistics_with_empty_tasks() {
    let stats = ExecutionStatistics {
        total_tasks: 0,
        completed_tasks: 0,
        failed_tasks: 0,
        total_retries: 0,
        total_time: 0,
        simple_tasks: 0,
        moderate_tasks: 0,
        complex_tasks: 0,
        sessions_reused: 0,
    };

    // Should not panic even with all zeros
    display_execution_statistics(&stats);
}

// ============================================================================
// Model Selection Tests
// ============================================================================

#[test]
fn test_model_selection_simple_complexity() {
    let config = ModeConfig::default();
    let model = config.model_for_complexity(&TaskComplexity::Simple);

    assert_eq!(model, "claude-sonnet-4-6");
}

#[test]
fn test_model_selection_moderate_complexity() {
    let config = ModeConfig::default();
    let model = config.model_for_complexity(&TaskComplexity::Moderate);

    assert_eq!(model, "claude-sonnet-4-6");
}

#[test]
fn test_model_selection_complex_complexity() {
    let config = ModeConfig::default();
    let model = config.model_for_complexity(&TaskComplexity::Complex);

    assert_eq!(model, "claude-opus-4-6");
}

#[test]
fn test_model_selection_fast_mode() {
    let config = ModeConfig::fast_mode();

    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Simple),
        "claude-haiku-4-5"
    );
    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Complex),
        "claude-sonnet-4-6"
    );
}

#[test]
fn test_model_selection_expert_mode() {
    let config = ModeConfig::expert_mode();

    // Expert mode uses Opus for everything
    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Simple),
        "claude-opus-4-6"
    );
    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Moderate),
        "claude-opus-4-6"
    );
    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Complex),
        "claude-opus-4-6"
    );
}

// ============================================================================
// Task Methods Tests
// ============================================================================

#[test]
fn test_task_can_execute_no_dependencies() {
    let task = Task::new("task-1", "Independent", "No dependencies");
    let completed = HashSet::new();

    assert!(task.can_execute(&completed));
}

#[test]
fn test_task_can_execute_with_unsatisfied_dependency() {
    let mut task = Task::new("task-1", "Dependent", "Has dependencies");
    task.depends_on = vec!["task-0".to_string()];

    let completed = HashSet::new();
    assert!(!task.can_execute(&completed));
}

#[test]
fn test_task_can_execute_with_satisfied_dependency() {
    let mut task = Task::new("task-1", "Dependent", "Has dependencies");
    task.depends_on = vec!["task-0".to_string()];

    let mut completed = HashSet::new();
    completed.insert("task-0".to_string());

    assert!(task.can_execute(&completed));
}

#[test]
fn test_task_can_execute_with_multiple_dependencies() {
    let mut task = Task::new("task-1", "Multi-dep", "Multiple dependencies");
    task.depends_on = vec!["task-0".to_string(), "task-2".to_string()];

    let mut completed = HashSet::new();

    // None satisfied
    assert!(!task.can_execute(&completed));

    // One satisfied
    completed.insert("task-0".to_string());
    assert!(!task.can_execute(&completed));

    // All satisfied
    completed.insert("task-2".to_string());
    assert!(task.can_execute(&completed));
}

#[test]
fn test_task_can_retry_not_failed() {
    let task = Task::new("task-1", "Test", "Test task");

    assert!(!task.can_retry(3));
}

#[test]
fn test_task_can_retry_failed_within_limit() {
    let mut task = Task::new("task-1", "Test", "Test task");
    task.status = TaskStatus::Failed;
    task.retry_count = 1;

    assert!(task.can_retry(3));
}

#[test]
fn test_task_can_retry_failed_at_limit() {
    let mut task = Task::new("task-1", "Test", "Test task");
    task.status = TaskStatus::Failed;
    task.retry_count = 3;

    assert!(!task.can_retry(3));
}

#[test]
fn test_task_is_failed() {
    let mut task = Task::new("task-1", "Test", "Test task");
    assert!(!task.is_failed());

    task.status = TaskStatus::Failed;
    assert!(task.is_failed());
}

#[test]
fn test_task_is_completed() {
    let mut task = Task::new("task-1", "Test", "Test task");
    assert!(!task.is_completed());

    task.status = TaskStatus::Completed;
    assert!(task.is_completed());
}

// ============================================================================
// Task Creation Tests
// ============================================================================

#[test]
fn test_task_creation_defaults() {
    let task = Task::new("test-1", "Test Task", "Test description");

    assert_eq!(task.id, "test-1");
    assert_eq!(task.title, "Test Task");
    assert_eq!(task.description, "Test description");
    assert_eq!(task.status, TaskStatus::Pending);
    assert_eq!(task.complexity, TaskComplexity::Moderate);
    assert!(task.depends_on.is_empty());
    assert!(task.subtasks.is_empty());
    assert_eq!(task.retry_count, 0);
    assert!(task.error.is_none());
    assert!(task.started_at.is_none());
    assert!(task.completed_at.is_none());
}

#[test]
fn test_task_with_dependencies() {
    let mut task = Task::new("task-1", "Test", "Test");
    task.depends_on = vec!["task-0".to_string(), "task-2".to_string()];

    assert_eq!(task.depends_on.len(), 2);
    assert!(task.depends_on.contains(&"task-0".to_string()));
    assert!(task.depends_on.contains(&"task-2".to_string()));
}

#[test]
fn test_task_with_different_complexities() {
    let mut simple = Task::new("simple", "Simple", "Simple task");
    let mut moderate = Task::new("moderate", "Moderate", "Moderate task");
    let mut complex = Task::new("complex", "Complex", "Complex task");

    simple.complexity = TaskComplexity::Simple;
    moderate.complexity = TaskComplexity::Moderate;
    complex.complexity = TaskComplexity::Complex;

    assert_eq!(simple.complexity, TaskComplexity::Simple);
    assert_eq!(moderate.complexity, TaskComplexity::Moderate);
    assert_eq!(complex.complexity, TaskComplexity::Complex);
}

#[test]
fn test_task_with_error() {
    let mut task = Task::new("task-1", "Test", "Test");

    assert!(task.error.is_none());

    task.error = Some("Failed to execute".to_string());

    assert_eq!(task.error, Some("Failed to execute".to_string()));
}

#[test]
fn test_task_status_transitions() {
    let mut task = Task::new("task-1", "Test", "Test");

    assert_eq!(task.status, TaskStatus::Pending);
    assert!(!task.is_completed());
    assert!(!task.is_failed());

    task.status = TaskStatus::InProgress;
    assert!(!task.is_completed());
    assert!(!task.is_failed());

    task.status = TaskStatus::Completed;
    assert!(task.is_completed());
    assert!(!task.is_failed());

    task.status = TaskStatus::Failed;
    assert!(!task.is_completed());
    assert!(task.is_failed());
}

// ============================================================================
// ExecutionMode Tests
// ============================================================================

#[test]
fn test_execution_mode_max_retries() {
    use ltmatrix::models::ExecutionMode;

    assert_eq!(ExecutionMode::Fast.max_retries(), 1);
    assert_eq!(ExecutionMode::Standard.max_retries(), 3);
    assert_eq!(ExecutionMode::Expert.max_retries(), 3);
}

#[test]
fn test_execution_mode_run_tests() {
    use ltmatrix::models::ExecutionMode;

    assert!(!ExecutionMode::Fast.run_tests());
    assert!(ExecutionMode::Standard.run_tests());
    assert!(ExecutionMode::Expert.run_tests());
}

#[test]
fn test_execution_mode_default_model() {
    use ltmatrix::models::ExecutionMode;

    assert_eq!(ExecutionMode::Fast.default_model(), "claude-haiku-4-5");
    assert_eq!(ExecutionMode::Standard.default_model(), "claude-sonnet-4-6");
    assert_eq!(ExecutionMode::Expert.default_model(), "claude-opus-4-6");
}

#[test]
fn test_execution_mode_max_depth() {
    use ltmatrix::models::ExecutionMode;

    assert_eq!(ExecutionMode::Fast.max_depth(), 2);
    assert_eq!(ExecutionMode::Standard.max_depth(), 3);
    assert_eq!(ExecutionMode::Expert.max_depth(), 3);
}

// ============================================================================
// PipelineStage Tests
// ============================================================================

#[test]
fn test_pipeline_stage_display_names() {
    use ltmatrix::models::PipelineStage;

    assert_eq!(PipelineStage::Generate.display_name(), "Generate");
    assert_eq!(PipelineStage::Assess.display_name(), "Assess");
    assert_eq!(PipelineStage::Execute.display_name(), "Execute");
    assert_eq!(PipelineStage::Test.display_name(), "Test");
    assert_eq!(PipelineStage::Verify.display_name(), "Verify");
    assert_eq!(PipelineStage::Commit.display_name(), "Commit");
    assert_eq!(PipelineStage::Memory.display_name(), "Memory");
}

#[test]
fn test_pipeline_stage_requires_agent() {
    use ltmatrix::models::PipelineStage;

    assert!(PipelineStage::Generate.requires_agent());
    assert!(PipelineStage::Assess.requires_agent());
    assert!(PipelineStage::Execute.requires_agent());
    assert!(PipelineStage::Test.requires_agent());
    assert!(PipelineStage::Verify.requires_agent());
    assert!(!PipelineStage::Commit.requires_agent());
    assert!(!PipelineStage::Memory.requires_agent());
}

#[test]
fn test_pipeline_stage_pipeline_for_mode() {
    use ltmatrix::models::{ExecutionMode, PipelineStage};

    // Fast mode skips Test stage
    let fast_pipeline = PipelineStage::pipeline_for_mode(ExecutionMode::Fast);
    assert!(!fast_pipeline.contains(&PipelineStage::Test));

    // Standard mode includes Test stage
    let standard_pipeline = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);
    assert!(standard_pipeline.contains(&PipelineStage::Test));

    // Expert mode includes Test stage
    let expert_pipeline = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);
    assert!(expert_pipeline.contains(&PipelineStage::Test));
}

// ============================================================================
// ModeConfig Tests
// ============================================================================

#[test]
fn test_mode_config_default() {
    let config = ModeConfig::default();

    assert_eq!(config.model_fast, "claude-sonnet-4-6");
    assert_eq!(config.model_smart, "claude-opus-4-6");
    assert!(config.run_tests);
    assert!(config.verify);
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.max_depth, 3);
    assert_eq!(config.timeout_plan, 120);
    assert_eq!(config.timeout_exec, 3600);
}

#[test]
fn test_mode_config_fast_mode() {
    let config = ModeConfig::fast_mode();

    assert_eq!(config.model_fast, "claude-haiku-4-5");
    assert_eq!(config.model_smart, "claude-sonnet-4-6");
    assert!(!config.run_tests);
    assert!(config.verify);
    assert_eq!(config.max_retries, 1);
    assert_eq!(config.max_depth, 2);
    assert_eq!(config.timeout_plan, 60);
    assert_eq!(config.timeout_exec, 1800);
}

#[test]
fn test_mode_config_expert_mode() {
    let config = ModeConfig::expert_mode();

    assert_eq!(config.model_fast, "claude-opus-4-6");
    assert_eq!(config.model_smart, "claude-opus-4-6");
    assert!(config.run_tests);
    assert!(config.verify);
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.max_depth, 3);
    assert_eq!(config.timeout_plan, 180);
    assert_eq!(config.timeout_exec, 7200);
}

// ============================================================================
// TaskStatus Tests
// ============================================================================

#[test]
fn test_task_status_is_terminal() {
    assert!(TaskStatus::Completed.is_terminal());
    assert!(TaskStatus::Failed.is_terminal());
    assert!(!TaskStatus::Pending.is_terminal());
    assert!(!TaskStatus::InProgress.is_terminal());
    assert!(!TaskStatus::Blocked.is_terminal());
}
