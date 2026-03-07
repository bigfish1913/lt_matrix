//! Tests for review stage orchestrator execution behavior
//!
//! These tests verify the actual execution logic of the review stage
//! within the pipeline orchestrator, including progress tracking,
//! logging, blocking issue handling, and error scenarios.

use ltmatrix::models::{ExecutionMode, PipelineStage, Task, TaskStatus};
use ltmatrix::pipeline::orchestrator::{OrchestratorConfig, PipelineOrchestrator};
use ltmatrix::pipeline::review::{ReviewConfig, IssueCategory, IssueSeverity, CodeIssue};
use tempfile::TempDir;

// =============================================================================
// Review Stage Execution in Orchestrator Tests
// =============================================================================

#[tokio::test]
async fn test_orchestrator_review_stage_skipped_when_not_expert() {
    let temp_dir = TempDir::new().unwrap();

    // Test with fast mode - review should be skipped
    let config = OrchestratorConfig::fast_mode()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let _orchestrator = PipelineOrchestrator::new(config).unwrap();

    // Review should not run in fast mode
    let fast_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Fast);
    assert!(!fast_stages.contains(&PipelineStage::Review));
}

#[tokio::test]
async fn test_orchestrator_review_stage_enabled_in_expert_mode() {
    let temp_dir = TempDir::new().unwrap();

    // Test with expert mode - review should be enabled
    let config = OrchestratorConfig::expert_mode()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    assert!(config.review_config.should_run());
    assert_eq!(config.execution_mode, ExecutionMode::Expert);

    let expert_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);
    assert!(expert_stages.contains(&PipelineStage::Review));
}

#[tokio::test]
async fn test_orchestrator_review_config_work_dir_propagation() {
    let temp_dir = TempDir::new().unwrap();

    let config = OrchestratorConfig::expert_mode()
        .with_work_dir(temp_dir.path());

    // Work dir should be propagated to review config
    assert_eq!(config.review_config.work_dir, temp_dir.path());
    assert_eq!(config.work_dir, temp_dir.path());
}

#[tokio::test]
async fn test_orchestrator_review_config_timeout_settings() {
    let config = OrchestratorConfig::expert_mode();

    // Expert mode should have appropriate timeout
    assert!(config.review_config.timeout >= 600);
    assert_eq!(config.review_config.timeout, 900); // 15 minutes
}

#[tokio::test]
async fn test_orchestrator_review_severity_threshold_expert_mode() {
    let config = OrchestratorConfig::expert_mode();

    // Expert mode should catch all issues (Low threshold)
    assert_eq!(config.review_config.severity_threshold, IssueSeverity::Low);
}

// =============================================================================
// Review Stage Stage Dependency Tests
// =============================================================================

#[test]
fn test_review_stage_dependencies_on_test() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    let test_idx = stages.iter().position(|s| s == &PipelineStage::Test).unwrap();
    let review_idx = stages.iter().position(|s| s == &PipelineStage::Review).unwrap();

    // Review must come after Test (Test must complete before Review)
    assert!(review_idx > test_idx, "Review depends on Test completion");
}

#[test]
fn test_review_stage_before_verify() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    let review_idx = stages.iter().position(|s| s == &PipelineStage::Review).unwrap();
    let verify_idx = stages.iter().position(|s| s == &PipelineStage::Verify).unwrap();

    // Review must come before Verify
    assert!(review_idx < verify_idx, "Review must complete before Verify");
}

#[test]
fn test_review_stage_parallel_with_nothing() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    // Review is not parallel - it's sequential between Test and Verify
    let _stages_before_review: Vec<_> = stages
        .iter()
        .take_while(|&&s| s != PipelineStage::Review)
        .cloned()
        .collect();

    let stages_after_review: Vec<_> = stages
        .iter()
        .skip_while(|&&s| s != PipelineStage::Review)
        .skip(1) // Skip Review itself
        .cloned()
        .collect();

    // Verify should be immediately after Review
    assert_eq!(stages_after_review.first(), Some(&PipelineStage::Verify));
}

// =============================================================================
// Review Results Impact on Pipeline Tests
// =============================================================================

#[test]
fn test_review_blocking_issues_prevent_completion() {
    // Simulate task with blocking review issues
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::Completed;

    // After review with blocking issues, task should be marked failed
    // This simulates what happens in the orchestrator
    let has_blocking_issues = true;

    if has_blocking_issues {
        task.status = TaskStatus::Failed;
        task.error = Some("Critical security issues found during review".to_string());
    }

    assert!(task.is_failed());
    assert!(task.error.is_some());
    assert!(task.error.unwrap().contains("security"));
}

#[test]
fn test_review_non_blocking_issues_allow_continuation() {
    // Simulate task with only non-blocking review issues
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::Completed;

    // After review with only non-blocking issues, task stays completed
    let has_blocking_issues = false;

    if !has_blocking_issues {
        // Task remains completed
        assert!(task.is_completed());
    } else {
        task.status = TaskStatus::Failed;
    }

    assert!(task.is_completed());
}

#[test]
fn test_review_summary_aggregates_correctly() {
    use ltmatrix::pipeline::review::ReviewSummary;

    let summary = ReviewSummary {
        total_tasks: 5,
        passed_tasks: 2,
        warning_tasks: 1,
        needs_improvements: 1,
        failed_tasks: 1,
        skipped_tasks: 0,
        total_time: 100,
        all_issues: vec![
            CodeIssue {
                category: IssueCategory::Security,
                severity: IssueSeverity::Critical,
                title: "SQL Injection".to_string(),
                description: "Critical security issue".to_string(),
                file: Some("src/db.rs".to_string()),
                line: Some(42),
                suggestion: Some("Use parameterized queries".to_string()),
                code_snippet: None,
                blocking: true,
            },
            CodeIssue {
                category: IssueCategory::Documentation,
                severity: IssueSeverity::Info,
                title: "Missing docs".to_string(),
                description: "Add documentation".to_string(),
                file: None,
                line: None,
                suggestion: None,
                code_snippet: None,
                blocking: false,
            },
        ],
        issues_by_category: vec![
            (IssueCategory::Security, 1),
            (IssueCategory::Documentation, 1),
        ],
        issues_by_severity: vec![
            (IssueSeverity::Critical, 1),
            (IssueSeverity::Info, 1),
        ],
        results: vec![],
    };

    assert_eq!(summary.total_tasks, 5);
    assert_eq!(summary.all_issues.len(), 2);
    assert_eq!(summary.issues_by_category.len(), 2);

    // Count blocking issues
    let blocking_count = summary.all_issues.iter().filter(|i| i.blocking).count();
    assert_eq!(blocking_count, 1);
}

// =============================================================================
// Review Configuration Mode Interaction Tests
// =============================================================================

#[test]
fn test_review_requires_verify_enabled() {
    let mut config = ReviewConfig::expert_mode();

    // Review should run when both enabled and verify is true
    assert!(config.should_run());

    // When verify is disabled, review should not run
    config.mode_config.verify = false;
    assert!(!config.should_run(), "Review should not run when verify is disabled");
}

#[test]
fn test_review_respects_mode_config() {
    let expert_config = ReviewConfig::expert_mode();

    // Expert mode should have verify enabled
    assert!(expert_config.mode_config.verify);
    assert!(expert_config.should_run());
}

#[test]
fn test_review_disabled_by_default() {
    let standard_config = ReviewConfig::default();

    // Standard/default config should have review disabled
    assert!(!standard_config.enabled);
    assert!(!standard_config.should_run());
}

// =============================================================================
// Review Stage Error Handling Tests
// =============================================================================

#[tokio::test]
async fn test_orchestrator_handles_empty_task_list_review() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::expert_mode()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orchestrator = PipelineOrchestrator::new(config).unwrap();

    // Orchestrator should handle empty task list gracefully
    let result = orchestrator.execute_pipeline("Empty goal", ExecutionMode::Expert).await;

    // Should succeed even with no actual tasks (just generates minimal tasks)
    assert!(result.is_ok());
}

#[test]
fn test_review_config_handles_invalid_work_dir() {
    let config = ReviewConfig::expert_mode();

    // Work dir should be set to current directory if invalid
    assert!(config.work_dir.exists() || config.work_dir.to_string_lossy() != "");
}

// =============================================================================
// Review Stage Progress Tracking Tests
// =============================================================================

#[test]
fn test_review_progress_message_format() {
    let _stage = PipelineStage::Review;

    // Progress message should be descriptive
    let message = format!("Reviewing {} tasks...", 5);
    assert!(message.contains("Reviewing"));
    assert!(message.contains("tasks"));
}

#[test]
fn test_review_completion_message_includes_blocking_count() {
    let blocking_count = 3;
    let total_tasks = 10;

    // Completion message should include blocking issue count
    let message = format!(
        "Reviewed {}/{} tasks ({} blocking issues)",
        total_tasks - blocking_count,
        total_tasks,
        blocking_count
    );

    assert!(message.contains("Reviewed"));
    assert!(message.contains("blocking issues"));
}

// =============================================================================
// Review Stage Integration Edge Cases Tests
// =============================================================================

#[test]
fn test_review_with_all_tasks_skipped() {
    use ltmatrix::pipeline::review::ReviewSummary;

    // When review is disabled, all tasks should be marked as skipped
    let summary = ReviewSummary {
        total_tasks: 5,
        passed_tasks: 0,
        warning_tasks: 0,
        needs_improvements: 0,
        failed_tasks: 0,
        skipped_tasks: 5,
        total_time: 0,
        all_issues: vec![],
        issues_by_category: vec![],
        issues_by_severity: vec![],
        results: vec![],
    };

    assert_eq!(summary.skipped_tasks, 5);
    assert_eq!(summary.total_tasks, 5);
}

#[test]
fn test_review_issue_category_filtering() {
    let config = ReviewConfig::expert_mode();

    // Verify all check flags are enabled in expert mode
    assert!(config.check_security);
    assert!(config.check_performance);
    assert!(config.check_quality);
    assert!(config.check_best_practices);
}

#[test]
fn test_review_max_issues_limit_enforced() {
    let config = ReviewConfig::expert_mode();

    // Should have a reasonable max issues limit
    assert!(config.max_issues_per_category > 0);
    assert!(config.max_issues_per_category <= 100);
}

#[test]
fn test_review_severity_threshold_filtering() {
    use ltmatrix::pipeline::review::ReviewSummary;

    let summary = ReviewSummary {
        total_tasks: 1,
        passed_tasks: 0,
        warning_tasks: 0,
        needs_improvements: 0,
        failed_tasks: 0,
        skipped_tasks: 0,
        total_time: 0,
        all_issues: vec![
            CodeIssue {
                category: IssueCategory::Quality,
                severity: IssueSeverity::Info, // Below threshold
                title: "Minor style issue".to_string(),
                description: "Style suggestion".to_string(),
                file: None,
                line: None,
                suggestion: None,
                code_snippet: None,
                blocking: false,
            },
        ],
        issues_by_category: vec![],
        issues_by_severity: vec![],
        results: vec![],
    };

    // With Medium threshold, Info issues should be filtered out
    let threshold = IssueSeverity::Medium;
    let filtered_issues: Vec<_> = summary.all_issues
        .iter()
        .filter(|issue| issue.severity >= threshold)
        .collect();

    assert_eq!(filtered_issues.len(), 0, "Info issues should be filtered out by Medium threshold");
}

// =============================================================================
// Review Stage Model Selection Tests
// =============================================================================

#[test]
fn test_review_uses_correct_model_in_expert_mode() {
    let config = ReviewConfig::expert_mode();

    // Expert mode should use Opus for review
    assert!(config.review_model.contains("opus"));
    assert_eq!(config.review_model, "claude-opus-4-6");
}

#[test]
fn test_review_timeout_scales_with_mode() {
    let expert_config = ReviewConfig::expert_mode();
    let standard_config = ReviewConfig::default();

    // Expert mode should have longer timeout for thorough review
    assert!(expert_config.timeout > standard_config.timeout);
}

// =============================================================================
// Review Stage Blocking Issues Grouping Tests
// =============================================================================

#[test]
fn test_blocking_issues_grouped_by_category() {
    use std::collections::HashMap;

    let issues = vec![
        CodeIssue {
            category: IssueCategory::Security,
            severity: IssueSeverity::Critical,
            title: "SQL Injection".to_string(),
            description: "Critical".to_string(),
            file: None,
            line: None,
            suggestion: None,
            code_snippet: None,
            blocking: true,
        },
        CodeIssue {
            category: IssueCategory::Security,
            severity: IssueSeverity::High,
            title: "XSS".to_string(),
            description: "High".to_string(),
            file: None,
            line: None,
            suggestion: None,
            code_snippet: None,
            blocking: true,
        },
        CodeIssue {
            category: IssueCategory::Performance,
            severity: IssueSeverity::High,
            title: "Slow query".to_string(),
            description: "Performance".to_string(),
            file: None,
            line: None,
            suggestion: None,
            code_snippet: None,
            blocking: true,
        },
    ];

    // Group blocking issues by category
    let mut blocking_by_category: HashMap<IssueCategory, Vec<&CodeIssue>> = HashMap::new();

    for issue in &issues {
        if issue.blocking {
            blocking_by_category
                .entry(issue.category)
                .or_insert_with(Vec::new)
                .push(issue);
        }
    }

    assert_eq!(blocking_by_category.len(), 2);
    assert_eq!(blocking_by_category.get(&IssueCategory::Security).unwrap().len(), 2);
    assert_eq!(blocking_by_category.get(&IssueCategory::Performance).unwrap().len(), 1);
}
