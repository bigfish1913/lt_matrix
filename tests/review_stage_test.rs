//! Tests for the review stage

use ltmatrix::models::Task;
use ltmatrix::pipeline::review::{
    build_review_prompt, CodeIssue, IssueCategory, IssueSeverity, ReviewAssessment, ReviewConfig, ReviewSummary,
};
use std::path::PathBuf;

#[test]
fn test_review_config_default_values() {
    let config = ReviewConfig::default();

    assert!(!config.enabled, "Review should be disabled by default");
    assert_eq!(config.review_model, "claude-opus-4-6");
    assert_eq!(config.max_issues_per_category, 10);
    assert_eq!(config.severity_threshold, IssueSeverity::Medium);
    assert!(config.check_security);
    assert!(config.check_performance);
    assert!(config.check_quality);
    assert!(config.check_best_practices);
    assert_eq!(config.timeout, 600);
}

#[test]
fn test_review_config_expert_mode_enables_review() {
    let config = ReviewConfig::expert_mode();

    assert!(config.enabled, "Review should be enabled in expert mode");
    assert!(config.should_run(), "should_run should return true in expert mode");
    assert!(config.is_expert_mode(), "is_expert_mode should return true");
    assert_eq!(config.review_model, "claude-opus-4-6");
    assert_eq!(config.max_issues_per_category, 15);
    assert_eq!(config.severity_threshold, IssueSeverity::Low);
    assert_eq!(config.timeout, 900);
}

#[test]
fn test_review_config_expert_with_review_all_severities() {
    let config = ReviewConfig::expert_with_review();

    assert!(config.enabled);
    assert!(config.should_run());
    assert!(config.is_expert_mode());
    assert_eq!(config.max_issues_per_category, 20);
    assert_eq!(config.severity_threshold, IssueSeverity::Info);
    assert_eq!(config.timeout, 10800); // 3 hours
}

#[test]
fn test_review_config_should_run_with_verify_disabled() {
    let mut config = ReviewConfig::expert_mode();
    config.mode_config.verify = false;

    assert!(!config.should_run(), "should_run should return false when verify is disabled");
}

#[test]
fn test_issue_severity_ordering() {
    assert!(IssueSeverity::Critical > IssueSeverity::High);
    assert!(IssueSeverity::High > IssueSeverity::Medium);
    assert!(IssueSeverity::Medium > IssueSeverity::Low);
    assert!(IssueSeverity::Low > IssueSeverity::Info);
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

#[test]
fn test_review_assessment_display() {
    assert_eq!(ReviewAssessment::Pass.to_string(), "pass");
    assert_eq!(ReviewAssessment::Warning.to_string(), "warning");
    assert_eq!(ReviewAssessment::NeedsImprovements.to_string(), "needs_improvements");
    assert_eq!(ReviewAssessment::Fail.to_string(), "fail");
}

#[test]
fn test_code_issue_all_fields() {
    let issue = CodeIssue {
        category: IssueCategory::Security,
        severity: IssueSeverity::Critical,
        file: Some("src/main.rs".to_string()),
        line: Some(42),
        title: "SQL Injection".to_string(),
        description: "User input not sanitized".to_string(),
        suggestion: Some("Use parameterized queries".to_string()),
        code_snippet: Some("format!(\"SELECT * FROM users WHERE id = {}\", user_input)".to_string()),
        blocking: true,
    };

    assert_eq!(issue.category, IssueCategory::Security);
    assert_eq!(issue.severity, IssueSeverity::Critical);
    assert_eq!(issue.file, Some("src/main.rs".to_string()));
    assert_eq!(issue.line, Some(42));
    assert_eq!(issue.title, "SQL Injection");
    assert_eq!(issue.blocking, true);
}

#[test]
fn test_review_summary_counts() {
    let summary = ReviewSummary {
        total_tasks: 10,
        passed_tasks: 5,
        warning_tasks: 2,
        needs_improvements: 2,
        failed_tasks: 1,
        skipped_tasks: 0,
        total_time: 300,
        all_issues: vec![],
        issues_by_category: vec![],
        issues_by_severity: vec![],
        results: vec![],
    };

    assert_eq!(summary.total_tasks, 10);
    assert_eq!(summary.passed_tasks, 5);
    assert_eq!(summary.warning_tasks, 2);
    assert_eq!(summary.needs_improvements, 2);
    assert_eq!(summary.failed_tasks, 1);
    assert_eq!(summary.skipped_tasks, 0);
    assert_eq!(summary.total_time, 300);
}

#[test]
fn test_build_review_prompt_includes_security_check() {
    let task = Task::new("task-1", "Add login", "Implement user authentication");
    let config = ReviewConfig {
        check_security: true,
        check_performance: false,
        check_quality: false,
        check_best_practices: false,
        ..Default::default()
    };

    let prompt = build_review_prompt(&task, &config);

    assert!(prompt.contains("Security vulnerabilities"));
    assert!(prompt.contains("Task: Add login"));
    assert!(prompt.contains("Implement user authentication"));
    assert!(prompt.contains("injection attacks"));
}

#[test]
fn test_build_review_prompt_includes_performance_check() {
    let task = Task::new("task-1", "Optimize DB", "Database optimization");
    let config = ReviewConfig {
        check_security: false,
        check_performance: true,
        check_quality: false,
        check_best_practices: false,
        ..Default::default()
    };

    let prompt = build_review_prompt(&task, &config);

    assert!(prompt.contains("Performance issues"));
    assert!(prompt.contains("inefficient algorithms"));
    assert!(prompt.contains("unnecessary allocations"));
}

#[test]
fn test_build_review_prompt_includes_quality_check() {
    let task = Task::new("task-1", "Refactor code", "Clean up implementation");
    let config = ReviewConfig {
        check_security: false,
        check_performance: false,
        check_quality: true,
        check_best_practices: false,
        ..Default::default()
    };

    let prompt = build_review_prompt(&task, &config);

    assert!(prompt.contains("Code quality"));
    assert!(prompt.contains("readability"));
    assert!(prompt.contains("maintainability"));
}

#[test]
fn test_build_review_prompt_includes_best_practices_check() {
    let task = Task::new("task-1", "Add feature", "New feature implementation");
    let config = ReviewConfig {
        check_security: false,
        check_performance: false,
        check_quality: false,
        check_best_practices: true,
        ..Default::default()
    };

    let prompt = build_review_prompt(&task, &config);

    assert!(prompt.contains("Best practices"));
    assert!(prompt.contains("language idioms"));
    assert!(prompt.contains("design patterns"));
}

#[test]
fn test_build_review_prompt_includes_all_checks() {
    let task = Task::new("task-1", "Complete task", "Full implementation");
    let config = ReviewConfig {
        check_security: true,
        check_performance: true,
        check_quality: true,
        check_best_practices: true,
        ..Default::default()
    };

    let prompt = build_review_prompt(&task, &config);

    assert!(prompt.contains("Security vulnerabilities"));
    assert!(prompt.contains("Performance issues"));
    assert!(prompt.contains("Code quality"));
    assert!(prompt.contains("Best practices"));
}

#[test]
fn test_build_review_prompt_includes_severity_levels() {
    let task = Task::new("task-1", "Test", "Test task");
    let config = ReviewConfig::default();

    let prompt = build_review_prompt(&task, &config);

    assert!(prompt.contains("Critical: Security vulnerabilities"));
    assert!(prompt.contains("High: Serious problems"));
    assert!(prompt.contains("Medium: Moderate issues"));
    assert!(prompt.contains("Low: Minor issues"));
    assert!(prompt.contains("Info: Minor suggestions"));
}

#[test]
fn test_build_review_prompt_requests_json_format() {
    let task = Task::new("task-1", "Test", "Test task");
    let config = ReviewConfig::default();

    let prompt = build_review_prompt(&task, &config);

    assert!(prompt.contains("assessment"));
    assert!(prompt.contains("summary"));
    assert!(prompt.contains("strengths"));
    assert!(prompt.contains("issues"));
    assert!(prompt.contains("category"));
    assert!(prompt.contains("severity"));
    assert!(prompt.contains("blocking"));
}

#[test]
fn test_review_config_work_dir_default() {
    let config = ReviewConfig::default();

    // Work dir should default to current directory or "."
    assert!(!config.work_dir.as_os_str().is_empty());
}

#[test]
fn test_review_config_custom_work_dir() {
    let custom_path = PathBuf::from("/custom/path");
    let config = ReviewConfig {
        work_dir: custom_path.clone(),
        ..Default::default()
    };

    assert_eq!(config.work_dir, custom_path);
}

#[test]
fn test_review_summary_with_issues_by_category() {
    let summary = ReviewSummary {
        total_tasks: 1,
        passed_tasks: 0,
        warning_tasks: 0,
        needs_improvements: 1,
        failed_tasks: 0,
        skipped_tasks: 0,
        total_time: 60,
        all_issues: vec![],
        issues_by_category: vec![
            (IssueCategory::Security, 3),
            (IssueCategory::Performance, 2),
            (IssueCategory::Quality, 5),
        ],
        issues_by_severity: vec![
            (IssueSeverity::Critical, 1),
            (IssueSeverity::High, 2),
            (IssueSeverity::Medium, 4),
        ],
        results: vec![],
    };

    assert_eq!(summary.issues_by_category.len(), 3);
    assert_eq!(summary.issues_by_severity.len(), 3);

    // Check category counts
    let security_count = summary
        .issues_by_category
        .iter()
        .find(|(cat, _)| *cat == IssueCategory::Security)
        .map(|(_, count)| *count)
        .unwrap_or(0);
    assert_eq!(security_count, 3);

    // Check severity counts
    let critical_count = summary
        .issues_by_severity
        .iter()
        .find(|(sev, _)| *sev == IssueSeverity::Critical)
        .map(|(_, count)| *count)
        .unwrap_or(0);
    assert_eq!(critical_count, 1);
}
