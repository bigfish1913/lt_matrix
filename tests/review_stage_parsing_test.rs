//! Tests for review stage parsing functions

use ltmatrix::pipeline::review::{
    build_review_prompt, IssueCategory, IssueSeverity, ReviewAssessment, ReviewConfig,
};
use ltmatrix::models::Task;
use serde_json::json;

#[test]
fn test_build_review_prompt_with_no_checks_enabled() {
    let task = Task::new("task-1", "Test", "Test task");
    let config = ReviewConfig {
        check_security: false,
        check_performance: false,
        check_quality: false,
        check_best_practices: false,
        ..Default::default()
    };

    let prompt = build_review_prompt(&task, &config);

    // Should still contain basic structure
    assert!(prompt.contains("Task: Test"));
    assert!(prompt.contains("You are an expert code reviewer"));
    assert!(prompt.contains("assessment"));
    assert!(prompt.contains("summary"));
}

#[test]
fn test_build_review_prompt_includes_task_context() {
    let task = Task::new("task-123", "Auth Module", "Implement JWT authentication with refresh tokens");
    let config = ReviewConfig::default();

    let prompt = build_review_prompt(&task, &config);

    // Note: task.id is not included in the prompt, only title and description
    assert!(prompt.contains("Auth Module"));
    assert!(prompt.contains("JWT authentication"));
    assert!(prompt.contains("refresh tokens"));
}

#[test]
fn test_build_review_prompt_custom_model() {
    let task = Task::new("task-1", "Test", "Test task");
    let config = ReviewConfig {
        review_model: "claude-sonnet-4-6".to_string(),
        ..Default::default()
    };

    let _prompt = build_review_prompt(&task, &config);

    // Model doesn't appear in prompt, but config should hold it
    assert_eq!(config.review_model, "claude-sonnet-4-6");
}

#[test]
fn test_build_review_prompt_expert_mode_settings() {
    let task = Task::new("task-1", "Complex Task", "Complex implementation");
    let config = ReviewConfig::expert_mode();

    let prompt = build_review_prompt(&task, &config);

    // Expert mode should have all checks enabled
    assert!(prompt.contains("Security vulnerabilities"));
    assert!(prompt.contains("Performance issues"));
    assert!(prompt.contains("Code quality"));
    assert!(prompt.contains("Best practices"));
}

#[test]
fn test_review_config_severity_threshold_filtering() {
    let config_critical = ReviewConfig {
        severity_threshold: IssueSeverity::Critical,
        ..Default::default()
    };
    let config_info = ReviewConfig {
        severity_threshold: IssueSeverity::Info,
        ..Default::default()
    };

    // Critical threshold should only show critical issues
    assert!(config_critical.severity_threshold == IssueSeverity::Critical);

    // Info threshold should show all issues
    assert!(config_info.severity_threshold == IssueSeverity::Info);
}

#[test]
fn test_review_config_max_issues_limit() {
    let config = ReviewConfig {
        max_issues_per_category: 5,
        ..Default::default()
    };

    assert_eq!(config.max_issues_per_category, 5);
}

#[test]
fn test_review_config_timeout_settings() {
    let config_default = ReviewConfig::default();
    let config_expert = ReviewConfig::expert_mode();
    let config_extended = ReviewConfig::expert_with_review();

    assert_eq!(config_default.timeout, 600);
    assert_eq!(config_expert.timeout, 900);
    assert_eq!(config_extended.timeout, 10800);
}

#[test]
fn test_review_config_should_run_logic() {
    // Default: disabled
    let config_default = ReviewConfig::default();
    assert!(!config_default.should_run());

    // Expert mode: enabled
    let config_expert = ReviewConfig::expert_mode();
    assert!(config_expert.should_run());

    // Enabled but verify disabled
    let mut config = ReviewConfig::expert_mode();
    config.mode_config.verify = false;
    assert!(!config.should_run());
}

#[test]
fn test_review_category_coverage() {
    // All categories should be representable
    let categories = vec![
        IssueCategory::Security,
        IssueCategory::Performance,
        IssueCategory::Quality,
        IssueCategory::BestPractices,
        IssueCategory::Documentation,
        IssueCategory::Testing,
        IssueCategory::ErrorHandling,
    ];

    for category in categories {
        let display = category.to_string();
        assert!(!display.is_empty());
    }
}

#[test]
fn test_review_severity_coverage() {
    // All severity levels should be representable
    let severities = vec![
        IssueSeverity::Info,
        IssueSeverity::Low,
        IssueSeverity::Medium,
        IssueSeverity::High,
        IssueSeverity::Critical,
    ];

    for severity in severities {
        let display = severity.to_string();
        assert!(!display.is_empty());
    }
}

#[test]
fn test_review_assessment_coverage() {
    // All assessment levels should be representable
    let assessments = vec![
        ReviewAssessment::Pass,
        ReviewAssessment::Warning,
        ReviewAssessment::NeedsImprovements,
        ReviewAssessment::Fail,
    ];

    for assessment in assessments {
        let display = assessment.to_string();
        assert!(!display.is_empty());
    }
}

#[test]
fn test_build_review_prompt_json_structure_requirements() {
    let task = Task::new("task-1", "JSON Test", "Test JSON structure");
    let config = ReviewConfig::default();

    let prompt = build_review_prompt(&task, &config);

    // Verify JSON structure is requested
    assert!(prompt.contains("\"assessment\":"));
    assert!(prompt.contains("\"summary\":"));
    assert!(prompt.contains("\"strengths\":"));
    assert!(prompt.contains("\"issues\":"));
    assert!(prompt.contains("\"category\":"));
    assert!(prompt.contains("\"severity\":"));
    assert!(prompt.contains("\"title\":"));
    assert!(prompt.contains("\"description\":"));
    assert!(prompt.contains("\"blocking\":"));
}

#[test]
fn test_build_review_prompt_optional_fields() {
    let task = Task::new("task-1", "Optional Fields", "Test optional fields");
    let config = ReviewConfig::default();

    let prompt = build_review_prompt(&task, &config);

    // Optional fields should be mentioned
    assert!(prompt.contains("\"file\":"));
    assert!(prompt.contains("\"line\":"));
    assert!(prompt.contains("\"suggestion\":"));
    assert!(prompt.contains("\"code_snippet\":"));
    assert!(prompt.contains("(optional)"));
}

#[test]
fn test_review_config_all_combinations_of_checks() {
    let task = Task::new("task-1", "Test", "Test");

    // Test all 16 combinations of the 4 boolean flags
    for check_security in [false, true] {
        for check_performance in [false, true] {
            for check_quality in [false, true] {
                for check_best_practices in [false, true] {
                    let config = ReviewConfig {
                        check_security,
                        check_performance,
                        check_quality,
                        check_best_practices,
                        ..Default::default()
                    };

                    let prompt = build_review_prompt(&task, &config);

                    // Verify that enabled checks are mentioned
                    if check_security {
                        assert!(prompt.contains("Security vulnerabilities"));
                    }
                    if check_performance {
                        assert!(prompt.contains("Performance issues"));
                    }
                    if check_quality {
                        assert!(prompt.contains("Code quality"));
                    }
                    if check_best_practices {
                        assert!(prompt.contains("Best practices"));
                    }

                    // At least the basic structure should be present
                    assert!(prompt.contains("You are an expert code reviewer"));
                }
            }
        }
    }
}

#[test]
fn test_review_config_cloning() {
    let config1 = ReviewConfig::expert_mode();
    let config2 = config1.clone();

    assert_eq!(config1.enabled, config2.enabled);
    assert_eq!(config1.review_model, config2.review_model);
    assert_eq!(config1.severity_threshold, config2.severity_threshold);
}

#[test]
fn test_review_config_equality() {
    let config1 = ReviewConfig::expert_mode();
    let config2 = ReviewConfig::expert_mode();
    let config3 = ReviewConfig::default();

    // Same values should be equal
    assert_eq!(config1.enabled, config2.enabled);
    assert_eq!(config1.severity_threshold, config2.severity_threshold);

    // Different values should not be equal
    assert_ne!(config1.enabled, config3.enabled);
}

#[test]
fn test_issue_severity_total_ordering() {
    // Verify total ordering (all combinations)
    let severities = vec![
        IssueSeverity::Info,
        IssueSeverity::Low,
        IssueSeverity::Medium,
        IssueSeverity::High,
        IssueSeverity::Critical,
    ];

    for i in 0..severities.len() {
        for j in 0..severities.len() {
            if i < j {
                assert!(severities[i] < severities[j]);
            } else if i > j {
                assert!(severities[i] > severities[j]);
            } else {
                assert_eq!(severities[i], severities[j]);
            }
        }
    }
}

#[test]
fn test_review_config_with_custom_timeout() {
    let mut config = ReviewConfig::expert_mode();
    config.timeout = 1800; // 30 minutes

    assert_eq!(config.timeout, 1800);
    assert!(config.is_expert_mode());
}

#[test]
fn test_review_config_with_custom_threshold() {
    let mut config = ReviewConfig::expert_mode();
    config.severity_threshold = IssueSeverity::High;

    assert_eq!(config.severity_threshold, IssueSeverity::High);
}

#[test]
fn test_build_review_prompt_includes_strengths_request() {
    let task = Task::new("task-1", "Strengths Test", "Test strengths extraction");
    let config = ReviewConfig::default();

    let prompt = build_review_prompt(&task, &config);

    assert!(prompt.contains("strengths"));
    assert!(prompt.contains("Strengths in the code"));
}
