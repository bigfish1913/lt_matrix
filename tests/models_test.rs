//! Comprehensive tests for core data models
//!
//! This test suite verifies:
//! - All struct and enum serialization/deserialization
//! - Method behavior and edge cases
//! - Default implementations
//! - Field validation

use ltmatrix::models::*;
use serde_json;
use std::collections::HashSet;

#[test]
fn test_task_new_creates_task_with_defaults() {
    let task = Task::new("task-1", "Test Task", "A test description");

    assert_eq!(task.id, "task-1");
    assert_eq!(task.title, "Test Task");
    assert_eq!(task.description, "A test description");
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
    let mut task = Task::new("task-2", "Dependent Task", "Has dependencies");
    task.depends_on = vec!["task-1".to_string(), "task-0".to_string()];

    let mut completed = HashSet::new();
    assert!(
        !task.can_execute(&completed),
        "Should not execute with no completed deps"
    );

    completed.insert("task-1".to_string());
    assert!(
        !task.can_execute(&completed),
        "Should not execute with partial deps"
    );

    completed.insert("task-0".to_string());
    assert!(
        task.can_execute(&completed),
        "Should execute with all deps completed"
    );
}

#[test]
fn test_task_with_no_dependencies() {
    let task = Task::new("task-3", "Independent Task", "No dependencies");
    let completed = HashSet::new();
    assert!(
        task.can_execute(&completed),
        "Task with no deps should always be executable"
    );
}

#[test]
fn test_task_status_checks() {
    let mut task = Task::new("task-4", "Status Test", "Testing status methods");

    // Initial state
    assert!(!task.is_failed(), "New task should not be failed");
    assert!(!task.is_completed(), "New task should not be completed");

    // Failed state
    task.status = TaskStatus::Failed;
    assert!(task.is_failed(), "Failed task should return true");
    assert!(!task.is_completed(), "Failed task should not be completed");

    // Completed state
    task.status = TaskStatus::Completed;
    assert!(!task.is_failed(), "Completed task should not be failed");
    assert!(task.is_completed(), "Completed task should return true");
}

#[test]
fn test_task_retry_logic() {
    let mut task = Task::new("task-5", "Retry Test", "Testing retry logic");

    // Not failed, can't retry
    assert!(!task.can_retry(3), "Non-failed task cannot retry");

    // Failed, no retries
    task.status = TaskStatus::Failed;
    task.retry_count = 0;
    assert!(
        task.can_retry(3),
        "Failed task with no retries should be retryable"
    );
    assert!(
        task.can_retry(1),
        "Failed task within retry limit should be retryable"
    );

    task.retry_count = 3;
    assert!(
        !task.can_retry(3),
        "Failed task at retry limit should not be retryable"
    );
    assert!(
        !task.can_retry(2),
        "Failed task over retry limit should not be retryable"
    );
}

#[test]
fn test_task_serialization() {
    let task = Task::new("task-6", "Serialize Test", "Testing serialization");

    let json = serde_json::to_string(&task).expect("Failed to serialize task");
    assert!(json.contains("\"id\":\"task-6\""));
    assert!(json.contains("\"title\":\"Serialize Test\""));
    assert!(json.contains("\"status\":\"pending\""));
    assert!(json.contains("\"complexity\":\"moderate\""));

    let deserialized: Task = serde_json::from_str(&json).expect("Failed to deserialize task");
    assert_eq!(deserialized.id, task.id);
    assert_eq!(deserialized.title, task.title);
    assert_eq!(deserialized.status, task.status);
}

#[test]
fn test_task_with_subtasks_serialization() {
    let mut parent = Task::new("parent", "Parent Task", "Has subtasks");
    parent.subtasks = vec![
        Task::new("child-1", "Child 1", "First subtask"),
        Task::new("child-2", "Child 2", "Second subtask"),
    ];

    let json = serde_json::to_string(&parent).expect("Failed to serialize task with subtasks");
    assert!(json.contains("\"subtasks\""));
    assert!(json.contains("\"child-1\""));
    assert!(json.contains("\"child-2\""));

    let deserialized: Task = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.subtasks.len(), 2);
    assert_eq!(deserialized.subtasks[0].id, "child-1");
    assert_eq!(deserialized.subtasks[1].id, "child-2");
}

#[test]
fn test_task_with_error_message() {
    let mut task = Task::new("task-7", "Error Task", "Has error");
    task.status = TaskStatus::Failed;
    task.error = Some("Something went wrong".to_string());

    let json = serde_json::to_string(&task).expect("Failed to serialize task with error");
    assert!(json.contains("\"error\":\"Something went wrong\""));

    let deserialized: Task = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.error, Some("Something went wrong".to_string()));
}

#[test]
fn test_task_timestamps_serialization() {
    let mut task = Task::new("task-8", "Timestamp Test", "Testing timestamps");

    // started_at and completed_at should be None initially
    let json = serde_json::to_string(&task).expect("Failed to serialize");
    assert!(!json.contains("\"started_at\""));
    assert!(!json.contains("\"completed_at\""));

    // Set timestamps
    task.started_at = Some(chrono::Utc::now());
    task.completed_at = Some(chrono::Utc::now());

    let json = serde_json::to_string(&task).expect("Failed to serialize with timestamps");
    assert!(json.contains("\"started_at\""));
    assert!(json.contains("\"completed_at\""));
}

#[test]
fn test_task_status_serialization() {
    let statuses = vec![
        TaskStatus::Pending,
        TaskStatus::InProgress,
        TaskStatus::Completed,
        TaskStatus::Failed,
        TaskStatus::Blocked,
    ];

    for status in statuses {
        let json = serde_json::to_string(&status).expect("Failed to serialize status");
        let deserialized: TaskStatus =
            serde_json::from_str(&json).expect("Failed to deserialize status");

        assert_eq!(deserialized, status, "Status should round-trip correctly");
    }
}

#[test]
fn test_task_status_terminal() {
    assert!(!TaskStatus::Pending.is_terminal());
    assert!(!TaskStatus::InProgress.is_terminal());
    assert!(TaskStatus::Completed.is_terminal());
    assert!(TaskStatus::Failed.is_terminal());
    assert!(!TaskStatus::Blocked.is_terminal());
}

#[test]
fn test_task_complexity_serialization() {
    let complexities = vec![
        TaskComplexity::Simple,
        TaskComplexity::Moderate,
        TaskComplexity::Complex,
    ];

    for complexity in complexities {
        let json = serde_json::to_string(&complexity).expect("Failed to serialize complexity");
        let deserialized: TaskComplexity =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(
            deserialized, complexity,
            "Complexity should round-trip correctly"
        );
    }
}

#[test]
fn test_agent_creation() {
    let agent = Agent::new("test-agent", "test-command", "test-model", 300);

    assert_eq!(agent.name, "test-agent");
    assert_eq!(agent.command, "test-command");
    assert_eq!(agent.model, "test-model");
    assert_eq!(agent.timeout, 300);
    assert!(!agent.is_default, "New agent should not be default");
}

#[test]
fn test_agent_default_marker() {
    let agent = Agent::new("default-agent", "default-command", "default-model", 600).with_default();

    assert!(
        agent.is_default,
        "with_default() should set is_default to true"
    );
}

#[test]
fn test_agent_claude_default() {
    let agent = Agent::claude_default();

    assert_eq!(agent.name, "claude");
    assert_eq!(agent.command, "claude");
    assert_eq!(agent.model, "claude-sonnet-4-6");
    assert_eq!(agent.timeout, 3600);
    assert!(
        agent.is_default,
        "Claude default should be marked as default"
    );
}

#[test]
fn test_agent_serialization() {
    let agent =
        Agent::new("serialize-agent", "serialize-cmd", "serialize-model", 120).with_default();

    let json = serde_json::to_string(&agent).expect("Failed to serialize agent");
    assert!(json.contains("\"name\":\"serialize-agent\""));
    assert!(json.contains("\"is_default\":true"));

    let deserialized: Agent = serde_json::from_str(&json).expect("Failed to deserialize agent");
    assert_eq!(deserialized.name, agent.name);
    assert_eq!(deserialized.is_default, agent.is_default);
}

#[test]
fn test_execution_mode_serialization() {
    let modes = vec![
        ExecutionMode::Fast,
        ExecutionMode::Standard,
        ExecutionMode::Expert,
    ];

    for mode in modes {
        let json = serde_json::to_string(&mode).expect("Failed to serialize mode");
        let deserialized: ExecutionMode =
            serde_json::from_str(&json).expect("Failed to deserialize mode");

        assert_eq!(deserialized, mode, "Mode should round-trip correctly");
    }
}

#[test]
fn test_execution_mode_default() {
    assert_eq!(ExecutionMode::default(), ExecutionMode::Standard);
}

#[test]
fn test_execution_mode_run_tests() {
    assert!(
        !ExecutionMode::Fast.run_tests(),
        "Fast mode should not run tests"
    );
    assert!(
        ExecutionMode::Standard.run_tests(),
        "Standard mode should run tests"
    );
    assert!(
        ExecutionMode::Expert.run_tests(),
        "Expert mode should run tests"
    );
}

#[test]
fn test_execution_mode_default_model() {
    assert_eq!(ExecutionMode::Fast.default_model(), "claude-haiku-4-5");
    assert_eq!(ExecutionMode::Standard.default_model(), "claude-sonnet-4-6");
    assert_eq!(ExecutionMode::Expert.default_model(), "claude-opus-4-6");
}

#[test]
fn test_execution_mode_max_depth() {
    assert_eq!(ExecutionMode::Fast.max_depth(), 2);
    assert_eq!(ExecutionMode::Standard.max_depth(), 3);
    assert_eq!(ExecutionMode::Expert.max_depth(), 3);
}

#[test]
fn test_execution_mode_max_retries() {
    assert_eq!(ExecutionMode::Fast.max_retries(), 1);
    assert_eq!(ExecutionMode::Standard.max_retries(), 3);
    assert_eq!(ExecutionMode::Expert.max_retries(), 3);
}

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
        let json = serde_json::to_string(&stage).expect("Failed to serialize stage");
        let deserialized: PipelineStage =
            serde_json::from_str(&json).expect("Failed to deserialize stage");

        assert_eq!(deserialized, stage, "Stage should round-trip correctly");
    }
}

#[test]
fn test_pipeline_stage_display_names() {
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
    assert!(PipelineStage::Generate.requires_agent());
    assert!(PipelineStage::Assess.requires_agent());
    assert!(PipelineStage::Execute.requires_agent());
    assert!(PipelineStage::Test.requires_agent());
    assert!(PipelineStage::Verify.requires_agent());
    assert!(
        !PipelineStage::Commit.requires_agent(),
        "Commit stage should not require agent"
    );
    assert!(
        !PipelineStage::Memory.requires_agent(),
        "Memory stage should not require agent"
    );
}

#[test]
fn test_pipeline_stage_fast_mode() {
    let pipeline = PipelineStage::pipeline_for_mode(ExecutionMode::Fast);

    assert!(
        !pipeline.contains(&PipelineStage::Test),
        "Fast mode should skip Test stage"
    );
    assert!(pipeline.contains(&PipelineStage::Generate));
    assert!(pipeline.contains(&PipelineStage::Execute));
    assert!(pipeline.contains(&PipelineStage::Commit));

    // Verify order
    assert_eq!(pipeline[0], PipelineStage::Generate);
    assert_eq!(pipeline[1], PipelineStage::Assess);
    assert_eq!(pipeline[2], PipelineStage::Execute);
}

#[test]
fn test_pipeline_stage_standard_mode() {
    let pipeline = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);

    assert!(
        pipeline.contains(&PipelineStage::Test),
        "Standard mode should include Test stage"
    );
    assert!(pipeline.contains(&PipelineStage::Verify));
    assert!(pipeline.contains(&PipelineStage::Memory));

    // Verify all stages present
    assert_eq!(pipeline.len(), 7, "Standard mode should have all 7 stages");
}

#[test]
fn test_pipeline_stage_expert_mode() {
    let pipeline = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    assert!(
        pipeline.contains(&PipelineStage::Test),
        "Expert mode should include Test stage"
    );
    assert!(
        pipeline.contains(&PipelineStage::Review),
        "Expert mode should include Review stage"
    );
    assert_eq!(pipeline.len(), 8, "Expert mode should have all 8 stages (Generate, Assess, Execute, Test, Review, Verify, Commit, Memory)");

    // Expert mode has an extra Review stage compared to Standard
    let standard_pipeline = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);
    assert_eq!(
        standard_pipeline.len(),
        7,
        "Standard mode should have 7 stages"
    );
}

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
    assert!(!config.run_tests, "Fast mode should not run tests");
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

#[test]
fn test_mode_config_model_for_complexity() {
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

#[test]
fn test_mode_config_serialization() {
    let config = ModeConfig {
        model_fast: "fast-model".to_string(),
        model_smart: "smart-model".to_string(),
        run_tests: true,
        verify: false,
        max_retries: 5,
        max_depth: 4,
        timeout_plan: 90,
        timeout_exec: 2400,
    };

    let json = serde_json::to_string(&config).expect("Failed to serialize config");
    assert!(json.contains("\"model_fast\":\"fast-model\""));
    assert!(json.contains("\"run_tests\":true"));

    let deserialized: ModeConfig =
        serde_json::from_str(&json).expect("Failed to deserialize config");
    assert_eq!(deserialized.model_fast, config.model_fast);
    assert_eq!(deserialized.run_tests, config.run_tests);
}

#[test]
fn test_complex_task_with_nested_subtasks() {
    let level2 = Task::new("level-2", "Level 2", "Nested subtask");
    let mut level1 = Task::new("level-1", "Level 1", "Subtask");
    level1.subtasks = vec![level2];

    let mut root = Task::new("root", "Root Task", "Complex task");
    root.subtasks = vec![level1];
    root.complexity = TaskComplexity::Complex;

    assert_eq!(root.subtasks.len(), 1);
    assert_eq!(root.subtasks[0].subtasks.len(), 1);
    assert_eq!(root.complexity, TaskComplexity::Complex);

    // Test serialization preserves nesting
    let json = serde_json::to_string(&root).expect("Failed to serialize nested tasks");
    let deserialized: Task =
        serde_json::from_str(&json).expect("Failed to deserialize nested tasks");

    assert_eq!(deserialized.subtasks.len(), 1);
    assert_eq!(deserialized.subtasks[0].subtasks.len(), 1);
    assert_eq!(deserialized.subtasks[0].subtasks[0].id, "level-2");
}

#[test]
fn test_task_with_multiple_dependencies() {
    let mut task = Task::new("multi-dep", "Multi Dependency", "Multiple dependencies");
    task.depends_on = vec![
        "dep-1".to_string(),
        "dep-2".to_string(),
        "dep-3".to_string(),
        "dep-4".to_string(),
    ];

    let mut completed = HashSet::new();
    assert!(!task.can_execute(&completed));

    completed.insert("dep-1".to_string());
    assert!(!task.can_execute(&completed));

    completed.insert("dep-3".to_string());
    completed.insert("dep-4".to_string());
    assert!(
        !task.can_execute(&completed),
        "Should need all dependencies"
    );

    completed.insert("dep-2".to_string());
    assert!(
        task.can_execute(&completed),
        "Should execute with all dependencies"
    );
}

#[test]
fn test_task_retry_count_increment() {
    let mut task = Task::new("retry-count", "Retry Count", "Testing retry count");
    task.status = TaskStatus::Failed;
    task.retry_count = 0;

    assert!(task.can_retry(3));

    task.retry_count = 1;
    assert!(task.can_retry(3));

    task.retry_count = 2;
    assert!(task.can_retry(3));

    task.retry_count = 3;
    assert!(!task.can_retry(3));
}

#[test]
fn test_empty_task_serialization() {
    let task = Task::new("empty", "Empty Task", "No fields set");

    let json = serde_json::to_string(&task).expect("Failed to serialize");
    let deserialized: Task = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.id, task.id);
    assert!(deserialized.depends_on.is_empty());
    assert!(deserialized.subtasks.is_empty());
}

#[test]
fn test_pipeline_stage_ordering_consistency() {
    // Test that pipeline order is consistent across calls
    let pipeline1 = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);
    let pipeline2 = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);

    assert_eq!(pipeline1, pipeline2, "Pipeline should be consistent");

    // Verify the expected order
    let expected_order = vec![
        PipelineStage::Generate,
        PipelineStage::Assess,
        PipelineStage::Execute,
        PipelineStage::Test,
        PipelineStage::Verify,
        PipelineStage::Commit,
        PipelineStage::Memory,
    ];

    assert_eq!(
        pipeline1, expected_order,
        "Pipeline should be in expected order"
    );
}

#[test]
fn test_agent_with_timeout_variations() {
    let quick_agent = Agent::new("quick", "quick-cmd", "quick-model", 30);
    let normal_agent = Agent::new("normal", "normal-cmd", "normal-model", 300);
    let long_agent = Agent::new("long", "long-cmd", "long-model", 7200);

    assert_eq!(quick_agent.timeout, 30);
    assert_eq!(normal_agent.timeout, 300);
    assert_eq!(long_agent.timeout, 7200);
}

#[test]
fn test_all_task_statuses_serializable_roundtrip() {
    let statuses = vec![
        ("pending", TaskStatus::Pending),
        ("in_progress", TaskStatus::InProgress),
        ("completed", TaskStatus::Completed),
        ("failed", TaskStatus::Failed),
        ("blocked", TaskStatus::Blocked),
    ];

    for (expected_str, status) in statuses {
        let json =
            serde_json::to_string(&status).expect(&format!("Failed to serialize {:?}", status));
        assert!(
            json.contains(expected_str),
            "JSON should contain '{}'",
            expected_str
        );

        let deserialized: TaskStatus = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized, status, "Status should round-trip correctly");
    }
}
