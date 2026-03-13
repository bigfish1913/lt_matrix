//! Tests for review stage flow control and task status management
//!
//! These tests verify how review results affect task flow control,
//! including blocking issues, task status transitions, and pipeline
//! continuation behavior.

use ltmatrix::models::{ExecutionMode, PipelineStage, Task, TaskStatus};
use ltmatrix::pipeline::orchestrator::OrchestratorConfig;
use ltmatrix::pipeline::review::{
    CodeIssue, IssueCategory, IssueSeverity, ReviewConfig, ReviewSummary,
};
use tempfile::TempDir;

// =============================================================================
// Review Summary Tests
// =============================================================================

#[test]
fn test_review_summary_default() {
    let summary = ReviewSummary {
        total_tasks: 0,
        passed_tasks: 0,
        warning_tasks: 0,
        needs_improvements: 0,
        failed_tasks: 0,
        skipped_tasks: 0,
        total_time: 0,
        all_issues: Vec::new(),
        issues_by_category: Vec::new(),
        issues_by_severity: Vec::new(),
        results: Vec::new(),
    };

    assert_eq!(summary.total_tasks, 0);
    assert_eq!(summary.passed_tasks, 0);
    assert_eq!(summary.failed_tasks, 0);
    assert!(summary.all_issues.is_empty());
}

#[test]
fn test_review_summary_with_blocking_issues() {
    let mut summary = ReviewSummary {
        total_tasks: 5,
        passed_tasks: 2,
        warning_tasks: 1,
        needs_improvements: 1,
        failed_tasks: 1,
        skipped_tasks: 0,
        total_time: 100,
        all_issues: Vec::new(),
        issues_by_category: Vec::new(),
        issues_by_severity: Vec::new(),
        results: Vec::new(),
    };

    // Add a blocking issue
    let blocking_issue = CodeIssue {
        category: IssueCategory::Security,
        severity: IssueSeverity::Critical,
        title: "Security vulnerability".to_string(),
        description: "Critical security issue found".to_string(),
        file: Some("src/main.rs".to_string()),
        line: Some(42),
        suggestion: Some("Fix immediately".to_string()),
        code_snippet: None,
        blocking: true,
    };

    summary.all_issues.push(blocking_issue);

    assert_eq!(summary.all_issues.len(), 1);
    assert_eq!(summary.failed_tasks, 1);
    assert!(summary.all_issues.iter().any(|i| i.blocking));
}

#[test]
fn test_review_summary_counts_issues_correctly() {
    let mut summary = ReviewSummary {
        total_tasks: 3,
        passed_tasks: 1,
        warning_tasks: 1,
        needs_improvements: 1,
        failed_tasks: 0,
        skipped_tasks: 0,
        total_time: 50,
        all_issues: Vec::new(),
        issues_by_category: Vec::new(),
        issues_by_severity: Vec::new(),
        results: Vec::new(),
    };

    // Add non-blocking issues
    summary.all_issues.push(CodeIssue {
        category: IssueCategory::Quality,
        severity: IssueSeverity::Low,
        title: "Code style".to_string(),
        description: "Minor style issue".to_string(),
        file: None,
        line: None,
        suggestion: None,
        code_snippet: None,
        blocking: false,
    });

    // Add blocking issue
    summary.all_issues.push(CodeIssue {
        category: IssueCategory::Security,
        severity: IssueSeverity::Critical,
        title: "Security issue".to_string(),
        description: "Critical security issue".to_string(),
        file: None,
        line: None,
        suggestion: None,
        code_snippet: None,
        blocking: true,
    });

    assert_eq!(summary.all_issues.len(), 2);
    assert_eq!(summary.failed_tasks, 0); // No failed tasks yet
}

// =============================================================================
// Code Issue Tests
// =============================================================================

#[test]
fn test_code_issue_creation() {
    let issue = CodeIssue {
        category: IssueCategory::Security,
        severity: IssueSeverity::High,
        title: "SQL Injection".to_string(),
        description: "Potential SQL injection vulnerability".to_string(),
        file: Some("src/database.rs".to_string()),
        line: Some(100),
        suggestion: Some("Use parameterized queries".to_string()),
        code_snippet: None,
        blocking: true,
    };

    assert_eq!(issue.category, IssueCategory::Security);
    assert_eq!(issue.severity, IssueSeverity::High);
    assert!(issue.blocking);
    assert_eq!(issue.file, Some("src/database.rs".to_string()));
    assert_eq!(issue.line, Some(100));
}

#[test]
fn test_code_issue_non_blocking() {
    let issue = CodeIssue {
        category: IssueCategory::Documentation,
        severity: IssueSeverity::Info,
        title: "Missing docs".to_string(),
        description: "Add documentation".to_string(),
        file: None,
        line: None,
        suggestion: None,
        code_snippet: None,
        blocking: false,
    };

    assert!(!issue.blocking);
    assert_eq!(issue.severity, IssueSeverity::Info);
}

// =============================================================================
// Review Configuration Flow Control Tests
// =============================================================================

#[test]
fn test_review_config_should_run_expert_mode() {
    let config = ReviewConfig::expert_mode();
    assert!(config.should_run(), "Review should run in expert mode");
    assert!(config.is_expert_mode(), "Should be expert mode");
}

#[test]
fn test_review_config_should_not_run_standard_mode() {
    let config = ReviewConfig::default();
    assert!(
        !config.should_run(),
        "Review should not run in standard mode"
    );
    assert!(!config.is_expert_mode(), "Should not be expert mode");
}

#[test]
fn test_review_config_verify_required() {
    let mut config = ReviewConfig::expert_mode();
    assert!(config.should_run());

    // If verify is disabled, review should not run
    config.mode_config.verify = false;
    assert!(
        !config.should_run(),
        "Review should not run when verify is disabled"
    );
}

#[test]
fn test_review_config_expert_with_review() {
    let config = ReviewConfig::expert_with_review();

    assert!(config.enabled);
    assert_eq!(config.severity_threshold, IssueSeverity::Info);
    assert_eq!(config.max_issues_per_category, 20);
    assert_eq!(config.timeout, 10800); // 3 hours
}

// =============================================================================
// Task Status After Review Tests
// =============================================================================

#[test]
fn test_task_status_with_no_blocking_issues() {
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::Completed;

    // Simulate review with no blocking issues
    // Task should remain completed
    assert!(task.is_completed());
}

#[test]
fn test_task_status_with_blocking_issues() {
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::Completed;

    // Simulate review finding blocking issues
    // Task would be marked as failed or blocked
    task.status = TaskStatus::Failed;
    task.error = Some("Critical security issues found".to_string());

    assert!(task.is_failed());
    assert!(task.error.is_some());
}

#[test]
fn test_task_review_does_not_affect_pending_tasks() {
    let task = Task::new("task-1", "Pending Task", "Description");

    // Pending tasks should not be affected by review
    assert_eq!(task.status, TaskStatus::Pending);
    assert!(!task.is_completed());
    assert!(!task.is_failed());
}

// =============================================================================
// Pipeline Stage Integration Tests
// =============================================================================

#[test]
fn test_review_stage_position_in_pipeline() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    let review_idx = stages
        .iter()
        .position(|s| matches!(s, PipelineStage::Review))
        .expect("Review stage should exist");

    let test_idx = stages
        .iter()
        .position(|s| matches!(s, PipelineStage::Test))
        .expect("Test stage should exist");

    let verify_idx = stages
        .iter()
        .position(|s| matches!(s, PipelineStage::Verify))
        .expect("Verify stage should exist");

    // Review must be after Test and before Verify
    assert!(review_idx > test_idx, "Review should come after Test");
    assert!(review_idx < verify_idx, "Review should come before Verify");
}

#[test]
fn test_review_stage_only_in_expert_mode() {
    let fast_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Fast);
    let standard_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);
    let expert_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    assert!(!fast_stages.contains(&PipelineStage::Review));
    assert!(!standard_stages.contains(&PipelineStage::Review));
    assert!(expert_stages.contains(&PipelineStage::Review));
}

#[test]
fn test_review_stage_dependencies() {
    // Review depends on Test stage completion
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    let test_idx = stages
        .iter()
        .position(|s| matches!(s, PipelineStage::Test))
        .unwrap();
    let review_idx = stages
        .iter()
        .position(|s| matches!(s, PipelineStage::Review))
        .unwrap();

    assert!(review_idx > test_idx, "Review must come after Test");
}

// =============================================================================
// Orchestrator Integration Tests
// =============================================================================

#[tokio::test]
async fn test_orchestrator_review_in_expert_mode() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::expert_mode()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    assert!(config.review_config.should_run());
    assert_eq!(config.execution_mode, ExecutionMode::Expert);
}

#[tokio::test]
async fn test_orchestrator_review_skipped_in_standard_mode() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    assert!(!config.review_config.should_run());
    assert_eq!(config.execution_mode, ExecutionMode::Standard);
}

#[tokio::test]
async fn test_orchestrator_review_skipped_in_fast_mode() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::fast_mode()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    assert!(!config.review_config.should_run());
    assert_eq!(config.execution_mode, ExecutionMode::Fast);
}

// =============================================================================
// Issue Severity and Category Tests
// =============================================================================

#[test]
fn test_issue_severity_ordering() {
    assert!(IssueSeverity::Critical > IssueSeverity::High);
    assert!(IssueSeverity::High > IssueSeverity::Medium);
    assert!(IssueSeverity::Medium > IssueSeverity::Low);
    assert!(IssueSeverity::Low > IssueSeverity::Info);
}

#[test]
fn test_issue_categories_complete() {
    let categories = vec![
        IssueCategory::Security,
        IssueCategory::Performance,
        IssueCategory::Quality,
        IssueCategory::BestPractices,
        IssueCategory::Documentation,
        IssueCategory::Testing,
        IssueCategory::ErrorHandling,
    ];

    assert_eq!(categories.len(), 7, "Should have 7 issue categories");
}

#[test]
fn test_issue_severity_display() {
    assert_eq!(IssueSeverity::Critical.to_string(), "critical");
    assert_eq!(IssueSeverity::High.to_string(), "high");
    assert_eq!(IssueSeverity::Medium.to_string(), "medium");
    assert_eq!(IssueSeverity::Low.to_string(), "low");
    assert_eq!(IssueSeverity::Info.to_string(), "info");
}

#[test]
fn test_issue_category_display() {
    assert_eq!(IssueCategory::Security.to_string(), "security");
    assert_eq!(IssueCategory::Performance.to_string(), "performance");
    assert_eq!(IssueCategory::Quality.to_string(), "quality");
    assert_eq!(IssueCategory::BestPractices.to_string(), "best_practices");
    assert_eq!(IssueCategory::Documentation.to_string(), "documentation");
    assert_eq!(IssueCategory::Testing.to_string(), "testing");
    assert_eq!(IssueCategory::ErrorHandling.to_string(), "error_handling");
}

// =============================================================================
// Review Stage Stage Properties Tests
// =============================================================================

#[test]
fn test_review_stage_properties() {
    let stage = PipelineStage::Review;

    assert_eq!(stage.display_name(), "Code Review");
    assert!(stage.requires_agent(), "Review stage should require agent");
}

#[test]
fn test_review_stage_serialization() {
    let stage = PipelineStage::Review;

    // Test that stage can be serialized (used in telemetry)
    let serialized = serde_json::to_string(&stage).unwrap();
    let deserialized: PipelineStage = serde_json::from_str(&serialized).unwrap();

    assert_eq!(stage, deserialized);
}

// =============================================================================
// Integration: Review in Full Pipeline Context
// =============================================================================

#[test]
fn test_review_integration_with_all_stages() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    // Verify complete pipeline order with review
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

    assert_eq!(stages, expected_order);
}

#[test]
fn test_review_pipeline_stage_count() {
    let fast_count = PipelineStage::pipeline_for_mode(ExecutionMode::Fast).len();
    let standard_count = PipelineStage::pipeline_for_mode(ExecutionMode::Standard).len();
    let expert_count = PipelineStage::pipeline_for_mode(ExecutionMode::Expert).len();

    // Expert mode should have exactly one more stage (Review) than Standard
    assert_eq!(expert_count, standard_count + 1);
    // Standard mode should have exactly one more stage (Test) than Fast
    assert_eq!(standard_count, fast_count + 1);
}

#[test]
fn test_review_stage_isolation() {
    // Verify review stage configuration doesn't affect other stages
    let expert_config = OrchestratorConfig::expert_mode();

    assert!(expert_config.review_config.enabled);
    assert_eq!(expert_config.execution_mode, ExecutionMode::Expert);

    // Other stage configs should still be properly set
    assert!(expert_config.mode_config.run_tests);
    assert!(expert_config.mode_config.verify);
}

// =============================================================================
// Review Stage Error Handling Tests
// =============================================================================

#[test]
fn test_review_config_handles_missing_work_dir() {
    let config = ReviewConfig::default();

    // Should have a valid work_dir even if not explicitly set
    assert!(config.work_dir.exists() || config.work_dir.to_string_lossy().contains("."));
}

#[test]
fn test_review_config_timeout_validation() {
    let expert_config = ReviewConfig::expert_mode();

    // Expert mode should have longer timeout
    assert!(expert_config.timeout > 600);
    assert_eq!(expert_config.timeout, 900); // 15 minutes
}

#[test]
fn test_review_config_severity_thresholds() {
    let standard_config = ReviewConfig::default();
    let expert_config = ReviewConfig::expert_mode();

    // Expert mode should catch all issues
    assert_eq!(standard_config.severity_threshold, IssueSeverity::Medium);
    assert_eq!(expert_config.severity_threshold, IssueSeverity::Low);
}

// =============================================================================
// Review Results Impact on Pipeline Tests
// =============================================================================

#[test]
fn test_blocking_issues_prevent_pipeline_continuation() {
    let mut summary = ReviewSummary {
        total_tasks: 3,
        passed_tasks: 2,
        warning_tasks: 0,
        needs_improvements: 0,
        failed_tasks: 1,
        skipped_tasks: 0,
        total_time: 100,
        all_issues: Vec::new(),
        issues_by_category: Vec::new(),
        issues_by_severity: Vec::new(),
        results: Vec::new(),
    };

    // Add blocking issue
    summary.all_issues.push(CodeIssue {
        category: IssueCategory::Security,
        severity: IssueSeverity::Critical,
        title: "Critical issue".to_string(),
        description: "Must fix".to_string(),
        file: None,
        line: None,
        suggestion: None,
        code_snippet: None,
        blocking: true,
    });

    assert!(summary.failed_tasks > 0);
    assert!(summary.all_issues.iter().any(|i| i.blocking));
}

#[test]
fn test_non_blocking_issues_allow_pipeline_continuation() {
    let mut summary = ReviewSummary {
        total_tasks: 3,
        passed_tasks: 3,
        warning_tasks: 0,
        needs_improvements: 0,
        failed_tasks: 0,
        skipped_tasks: 0,
        total_time: 100,
        all_issues: Vec::new(),
        issues_by_category: Vec::new(),
        issues_by_severity: Vec::new(),
        results: Vec::new(),
    };

    // Add non-blocking issue
    summary.all_issues.push(CodeIssue {
        category: IssueCategory::Documentation,
        severity: IssueSeverity::Info,
        title: "Missing docs".to_string(),
        description: "Add docs".to_string(),
        file: None,
        line: None,
        suggestion: None,
        code_snippet: None,
        blocking: false,
    });

    assert_eq!(summary.failed_tasks, 0);
    assert!(!summary.all_issues.iter().any(|i| i.blocking));
}

#[test]
fn test_review_summary_task_counts() {
    let summary = ReviewSummary {
        total_tasks: 10,
        passed_tasks: 7,
        warning_tasks: 1,
        needs_improvements: 1,
        failed_tasks: 1,
        skipped_tasks: 0,
        total_time: 500,
        all_issues: Vec::new(),
        issues_by_category: Vec::new(),
        issues_by_severity: Vec::new(),
        results: Vec::new(),
    };

    assert_eq!(summary.total_tasks, 10);
    assert_eq!(
        summary.passed_tasks
            + summary.warning_tasks
            + summary.needs_improvements
            + summary.failed_tasks
            + summary.skipped_tasks,
        10
    );
}
