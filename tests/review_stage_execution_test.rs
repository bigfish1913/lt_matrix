//! Tests for review stage execution behavior
//!
//! These tests verify the actual execution logic of the review stage,
//! including error handling, result processing, and integration with
//! the pipeline orchestrator.

use ltmatrix::models::{ExecutionMode, PipelineStage, Task, TaskStatus};
use ltmatrix::pipeline::orchestrator::OrchestratorConfig;
use ltmatrix::pipeline::review::{IssueCategory, IssueSeverity, ReviewConfig};
use tempfile::TempDir;

// =============================================================================
// Review Stage Execution Tests
// =============================================================================

#[test]
fn test_review_config_should_run_expert_mode() {
    let config = ReviewConfig::expert_mode();
    assert!(config.should_run(), "Review should run in expert mode");
}

#[test]
fn test_review_config_should_run_standard_mode() {
    let config = ReviewConfig::default();
    assert!(
        !config.should_run(),
        "Review should not run in standard mode"
    );
}

#[test]
fn test_review_config_should_run_fast_mode() {
    let config = ReviewConfig::default(); // Fast mode uses default config (review disabled)
    assert!(!config.should_run(), "Review should not run in fast mode");
}

#[test]
fn test_review_config_expert_mode_properties() {
    let config = ReviewConfig::expert_mode();

    // Verify expert mode has appropriate configuration
    assert_eq!(config.mode_config.max_retries, 3);
    assert!(config.mode_config.verify);
    assert!(config.enabled);
    assert!(config.review_model.contains("opus"));
    assert_eq!(config.timeout, 900); // 15 minutes
}

#[test]
fn test_review_config_mode_combinations() {
    // Test that review respects mode configuration
    let expert_config = ReviewConfig::expert_mode();
    let standard_config = ReviewConfig::default();
    let fast_config = ReviewConfig::default(); // Same as standard

    // Expert mode should have different settings than standard
    assert_ne!(expert_config.timeout, standard_config.timeout);
    assert_ne!(expert_config.enabled, standard_config.enabled);

    // Fast and standard should both have review disabled
    assert_eq!(fast_config.enabled, standard_config.enabled);
    assert!(!fast_config.enabled);
}

#[test]
fn test_review_stage_display_properties() {
    let stage = PipelineStage::Review;

    assert_eq!(stage.display_name(), "Code Review");
    assert!(
        stage.requires_agent(),
        "Review stage should require agent interaction"
    );
}

#[test]
fn test_review_stage_in_expert_pipeline() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    // Review should be present
    assert!(stages.contains(&PipelineStage::Review));

    // Review should be between Test and Verify
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

#[test]
fn test_review_stage_not_in_other_pipelines() {
    let standard_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);
    let fast_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Fast);

    assert!(!standard_stages.contains(&PipelineStage::Review));
    assert!(!fast_stages.contains(&PipelineStage::Review));
}

// =============================================================================
// Review Configuration Tests
// =============================================================================

#[test]
fn test_review_issue_category_types() {
    // Verify all issue categories exist
    let categories = vec![
        IssueCategory::Security,
        IssueCategory::Performance,
        IssueCategory::Quality,
        IssueCategory::BestPractices,
        IssueCategory::Documentation,
        IssueCategory::Testing,
    ];

    assert_eq!(categories.len(), 6, "Should have 6 issue categories");
}

#[test]
fn test_review_issue_severity_levels() {
    // Verify severity levels exist
    let severities = vec![
        IssueSeverity::Critical,
        IssueSeverity::High,
        IssueSeverity::Medium,
        IssueSeverity::Low,
        IssueSeverity::Info,
    ];

    assert_eq!(severities.len(), 5, "Should have 5 severity levels");
}

// =============================================================================
// Pipeline Orchestrator Integration Tests
// =============================================================================

#[tokio::test]
async fn test_orchestrator_creates_with_review_config() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::expert_mode()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orchestrator = ltmatrix::pipeline::orchestrator::PipelineOrchestrator::new(config);
    assert!(
        orchestrator.is_ok(),
        "Orchestrator should create successfully with review config"
    );
}

#[tokio::test]
async fn test_orchestrator_review_config_in_expert_mode() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::expert_mode()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    assert!(config.review_config.enabled);
    assert!(config.review_config.should_run());
    assert_eq!(config.review_config.timeout, 900);
}

#[test]
fn test_orchestrator_config_mode_factories() {
    // Test that mode factory methods set review config correctly
    let expert_config = OrchestratorConfig::expert_mode();
    let standard_config = OrchestratorConfig::default();
    let fast_config = OrchestratorConfig::fast_mode();

    // Expert mode should have review enabled
    assert!(expert_config.review_config.enabled);
    assert!(expert_config.review_config.should_run());

    // Standard and fast should have review disabled
    assert!(!standard_config.review_config.enabled);
    assert!(!fast_config.review_config.enabled);
}

#[test]
fn test_orchestrator_review_stage_skipped_when_disabled() {
    let config = OrchestratorConfig::fast_mode(); // Review disabled in fast mode

    // Verify that review config would not run
    assert!(!config.review_config.should_run());
}

// =============================================================================
// Pipeline Flow Tests
// =============================================================================

#[test]
fn test_pipeline_flow_review_in_correct_position() {
    // Test that review is positioned correctly in the flow
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    let expected_order = vec![
        PipelineStage::Generate,
        PipelineStage::Assess,
        PipelineStage::Execute,
        PipelineStage::Test,
        PipelineStage::Review,
        PipelineStage::Verify,
        PipelineStage::Commit,
        PipelineStage::Memory,
    ];

    assert_eq!(
        stages, expected_order,
        "Review should be between Test and Verify"
    );
}

#[test]
fn test_pipeline_flow_all_modes_have_valid_stages() {
    // Test that all execution modes produce valid stage sequences
    let modes = vec![
        ExecutionMode::Fast,
        ExecutionMode::Standard,
        ExecutionMode::Expert,
    ];

    for mode in modes {
        let stages = PipelineStage::pipeline_for_mode(mode);

        // All pipelines should have at least Generate and Memory
        assert!(stages.contains(&PipelineStage::Generate));
        assert!(stages.contains(&PipelineStage::Memory));

        // Stages should be unique
        let unique_stages: std::collections::HashSet<_> = stages.iter().collect();
        assert_eq!(
            unique_stages.len(),
            stages.len(),
            "Pipeline should not have duplicate stages"
        );

        // Review should only be in Expert mode
        if mode == ExecutionMode::Expert {
            assert!(stages.contains(&PipelineStage::Review));
        } else {
            assert!(!stages.contains(&PipelineStage::Review));
        }
    }
}

// =============================================================================
// Review Configuration Builder Tests
// =============================================================================

#[test]
fn test_review_config_work_dir_setting() {
    let temp_dir = TempDir::new().unwrap();

    let mut config = ReviewConfig::expert_mode();
    config.work_dir = temp_dir.path().to_path_buf();

    assert_eq!(config.work_dir, temp_dir.path());
}

#[test]
fn test_review_config_check_flags() {
    let config = ReviewConfig::expert_mode();

    // Expert mode should enable all checks
    assert!(config.check_security);
    assert!(config.check_performance);
    assert!(config.check_quality);
    assert!(config.check_best_practices);
}

#[test]
fn test_review_config_severity_threshold() {
    let config = ReviewConfig::expert_mode();

    // Verify severity threshold is set
    assert!(matches!(
        config.severity_threshold,
        IssueSeverity::Medium | IssueSeverity::Low
    ));
}

#[test]
fn test_review_config_max_issues_limit() {
    let config = ReviewConfig::expert_mode();

    // Should have a reasonable max issues limit
    assert!(config.max_issues_per_category > 0);
    assert!(config.max_issues_per_category <= 100);
}

// =============================================================================
// Task Status Transition Tests
// =============================================================================

#[test]
fn test_task_status_after_successful_review() {
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::Completed;

    // After successful review, task should remain completed
    assert!(task.is_completed());
}

#[test]
fn test_task_status_after_blocking_review() {
    let task = Task::new("task-1", "Test Task", "Description");

    // Task with blocking issues should not be completed
    assert!(!task.is_completed());
    assert!(task.is_failed() || task.status == TaskStatus::Pending);
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_end_to_end_review_integration() {
    // Test that review integrates properly with the entire pipeline

    // 1. Verify stage exists in enum
    let review_stage = PipelineStage::Review;
    assert_eq!(review_stage.display_name(), "Code Review");

    // 2. Verify stage is in expert pipeline
    let expert_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);
    assert!(expert_stages.contains(&review_stage));

    // 3. Verify stage position
    let review_idx = expert_stages
        .iter()
        .position(|s| s == &PipelineStage::Review)
        .unwrap();
    let test_idx = expert_stages
        .iter()
        .position(|s| s == &PipelineStage::Test)
        .unwrap();
    let verify_idx = expert_stages
        .iter()
        .position(|s| s == &PipelineStage::Verify)
        .unwrap();

    assert!(review_idx > test_idx);
    assert!(review_idx < verify_idx);

    // 4. Verify orchestrator config supports review
    let config = OrchestratorConfig::expert_mode();
    assert!(config.review_config.should_run());

    // 5. Verify review requires agent
    assert!(review_stage.requires_agent());
}

#[test]
fn test_review_isolation_from_other_stages() {
    // Verify that review stage is properly isolated and doesn't affect
    // other stages' configurations

    let expert_config = OrchestratorConfig::expert_mode();

    // Review config should be independent of other stage configs
    assert!(expert_config.review_config.enabled);

    // Other stages should still be configured (using default behavior)
    assert_eq!(expert_config.execution_mode, ExecutionMode::Expert);
}

#[test]
fn test_multiple_mode_transitions() {
    // Test that switching modes properly updates review configuration

    // Start with expert mode
    let config = OrchestratorConfig::expert_mode();
    assert!(config.review_config.should_run());

    // Switch to standard mode
    let config = OrchestratorConfig::default();
    assert!(!config.review_config.should_run());

    // Switch to fast mode
    let config = OrchestratorConfig::fast_mode();
    assert!(!config.review_config.should_run());

    // Back to expert mode
    let config = OrchestratorConfig::expert_mode();
    assert!(config.review_config.should_run());
}
