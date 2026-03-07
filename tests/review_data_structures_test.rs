//! Tests for review data structures and models
//!
//! These tests verify the data structures defined in review.rs including:
//! - ReviewError: error types and their behavior
//! - ReviewSeverity: severity levels and their properties
//! - ReviewCategory: categories and mode-specific behavior
//! - ReviewFinding: individual findings and builder pattern
//! - ReviewReport: comprehensive reports and metrics
//! - CodeIssue conversions between legacy and new types
//! - Serde serialization/deserialization

use ltmatrix::pipeline::review::{
    CodeIssue, IssueCategory, IssueSeverity, ReviewAssessment, ReviewCategory, ReviewConfig,
    ReviewConfigSnapshot, ReviewError, ReviewFinding, ReviewMetrics, ReviewReport,
    ReviewSeverity,
};

// =============================================================================
// ReviewError Tests
// =============================================================================

#[test]
fn test_review_error_is_recoverable_for_critical_issues() {
    let error = ReviewError::CriticalIssues("Found 3 critical issues".to_string());
    assert!(
        error.is_recoverable(),
        "CriticalIssues should be recoverable (can trigger fix cycle)"
    );
}

#[test]
fn test_review_error_not_recoverable_for_other_errors() {
    let agent_error = ReviewError::AgentError("Connection failed".to_string());
    let parse_error = ReviewError::ParseError("Invalid JSON".to_string());
    let timeout = ReviewError::Timeout(60);
    let config_error = ReviewError::ConfigError("Invalid config".to_string());
    let io_error = ReviewError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"));

    assert!(!agent_error.is_recoverable());
    assert!(!parse_error.is_recoverable());
    assert!(!timeout.is_recoverable());
    assert!(!config_error.is_recoverable());
    assert!(!io_error.is_recoverable());
}

#[test]
fn test_review_error_is_fatal_for_non_critical_issues() {
    let agent_error = ReviewError::AgentError("Connection failed".to_string());
    let parse_error = ReviewError::ParseError("Invalid JSON".to_string());
    let timeout = ReviewError::Timeout(60);
    let config_error = ReviewError::ConfigError("Invalid config".to_string());
    let io_error = ReviewError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"));

    assert!(agent_error.is_fatal());
    assert!(parse_error.is_fatal());
    assert!(timeout.is_fatal());
    assert!(config_error.is_fatal());
    assert!(!io_error.is_fatal(), "IoError should not be fatal (not in the is_fatal match)");
}

#[test]
fn test_review_error_not_fatal_for_critical_issues() {
    let critical = ReviewError::CriticalIssues("Found issues".to_string());
    assert!(
        !critical.is_fatal(),
        "CriticalIssues should not be fatal (allows fix cycle)"
    );
}

#[test]
fn test_review_error_error_codes() {
    assert_eq!(
        ReviewError::AgentError("test".to_string()).error_code(),
        "REVIEW_AGENT_ERROR"
    );
    assert_eq!(
        ReviewError::ParseError("test".to_string()).error_code(),
        "REVIEW_PARSE_ERROR"
    );
    assert_eq!(ReviewError::Timeout(60).error_code(), "REVIEW_TIMEOUT");
    assert_eq!(
        ReviewError::CriticalIssues("test".to_string()).error_code(),
        "REVIEW_CRITICAL_ISSUES"
    );
    assert_eq!(
        ReviewError::ConfigError("test".to_string()).error_code(),
        "REVIEW_CONFIG_ERROR"
    );
    assert_eq!(
        ReviewError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "file not found")).error_code(),
        "REVIEW_IO_ERROR"
    );
}

#[test]
fn test_review_error_display() {
    let error = ReviewError::AgentError("Connection refused".to_string());
    assert!(error.to_string().contains("Connection refused"));
    assert!(error.to_string().contains("Agent error"));

    let timeout = ReviewError::Timeout(120);
    assert!(timeout.to_string().contains("120"));
    assert!(timeout.to_string().contains("timed out"));
}

// =============================================================================
// ReviewSeverity Tests
// =============================================================================

#[test]
fn test_review_severity_default_is_medium() {
    assert_eq!(ReviewSeverity::default(), ReviewSeverity::Medium);
}

#[test]
fn test_review_severity_is_blocking() {
    assert!(
        ReviewSeverity::Critical.is_blocking(),
        "Critical should be blocking"
    );
    assert!(
        !ReviewSeverity::High.is_blocking(),
        "High should not be blocking"
    );
    assert!(
        !ReviewSeverity::Medium.is_blocking(),
        "Medium should not be blocking"
    );
    assert!(
        !ReviewSeverity::Low.is_blocking(),
        "Low should not be blocking"
    );
    assert!(
        !ReviewSeverity::Info.is_blocking(),
        "Info should not be blocking"
    );
}

#[test]
fn test_review_severity_triggers_fix_cycle_in_expert_mode() {
    // In expert mode
    assert!(
        ReviewSeverity::Critical.triggers_fix_cycle(true),
        "Critical should trigger fix cycle in expert mode"
    );
    assert!(
        ReviewSeverity::High.triggers_fix_cycle(true),
        "High should trigger fix cycle in expert mode"
    );
    assert!(
        ReviewSeverity::Medium.triggers_fix_cycle(true),
        "Medium should trigger fix cycle in expert mode"
    );
    assert!(
        !ReviewSeverity::Low.triggers_fix_cycle(true),
        "Low should not trigger fix cycle even in expert mode"
    );
    assert!(
        !ReviewSeverity::Info.triggers_fix_cycle(true),
        "Info should not trigger fix cycle even in expert mode"
    );
}

#[test]
fn test_review_severity_triggers_fix_cycle_in_standard_mode() {
    // In standard mode
    assert!(
        ReviewSeverity::Critical.triggers_fix_cycle(false),
        "Critical should trigger fix cycle in standard mode"
    );
    assert!(
        ReviewSeverity::High.triggers_fix_cycle(false),
        "High should trigger fix cycle in standard mode"
    );
    assert!(
        !ReviewSeverity::Medium.triggers_fix_cycle(false),
        "Medium should not trigger fix cycle in standard mode"
    );
    assert!(
        !ReviewSeverity::Low.triggers_fix_cycle(false),
        "Low should not trigger fix cycle in standard mode"
    );
    assert!(
        !ReviewSeverity::Info.triggers_fix_cycle(false),
        "Info should not trigger fix cycle in standard mode"
    );
}

#[test]
fn test_review_severity_descriptions() {
    assert!(ReviewSeverity::Critical.description().contains("Critical"));
    assert!(ReviewSeverity::High.description().contains("Serious"));
    assert!(ReviewSeverity::Medium.description().contains("Moderate"));
    assert!(ReviewSeverity::Low.description().contains("Minor"));
    assert!(ReviewSeverity::Info.description().contains("Informational"));
}

#[test]
fn test_review_severity_icons() {
    assert!(!ReviewSeverity::Critical.icon().is_empty());
    assert!(!ReviewSeverity::High.icon().is_empty());
    assert!(!ReviewSeverity::Medium.icon().is_empty());
    assert!(!ReviewSeverity::Low.icon().is_empty());
    assert!(!ReviewSeverity::Info.icon().is_empty());
}

#[test]
fn test_review_severity_color_codes() {
    // Verify color codes are valid ANSI escape sequences
    assert!(ReviewSeverity::Critical.color_code().starts_with("\x1b["));
    assert!(ReviewSeverity::High.color_code().starts_with("\x1b["));
    assert!(ReviewSeverity::Medium.color_code().starts_with("\x1b["));
    assert!(ReviewSeverity::Low.color_code().starts_with("\x1b["));
    assert!(ReviewSeverity::Info.color_code().starts_with("\x1b["));
}

#[test]
fn test_review_severity_ordering_comprehensive() {
    // Comprehensive ordering test
    assert!(ReviewSeverity::Critical > ReviewSeverity::High);
    assert!(ReviewSeverity::Critical > ReviewSeverity::Medium);
    assert!(ReviewSeverity::Critical > ReviewSeverity::Low);
    assert!(ReviewSeverity::Critical > ReviewSeverity::Info);

    assert!(ReviewSeverity::High > ReviewSeverity::Medium);
    assert!(ReviewSeverity::High > ReviewSeverity::Low);
    assert!(ReviewSeverity::High > ReviewSeverity::Info);

    assert!(ReviewSeverity::Medium > ReviewSeverity::Low);
    assert!(ReviewSeverity::Medium > ReviewSeverity::Info);

    assert!(ReviewSeverity::Low > ReviewSeverity::Info);
}

#[test]
fn test_review_severity_equality() {
    assert_eq!(ReviewSeverity::Critical, ReviewSeverity::Critical);
    assert_ne!(ReviewSeverity::Critical, ReviewSeverity::High);
}

#[test]
fn test_review_severity_serde_roundtrip() {
    let original = ReviewSeverity::High;
    let json = serde_json::to_string(&original).unwrap();
    assert_eq!(json, "\"high\"");
    let deserialized: ReviewSeverity = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

// =============================================================================
// ReviewCategory Tests
// =============================================================================

#[test]
fn test_review_category_descriptions() {
    assert!(ReviewCategory::Security.description().contains("Security"));
    assert!(ReviewCategory::Performance.description().contains("Performance"));
    assert!(ReviewCategory::Quality.description().contains("quality"));
    assert!(ReviewCategory::BestPractices.description().contains("practices"));
    assert!(ReviewCategory::Documentation.description().contains("Documentation"));
    assert!(ReviewCategory::Testing.description().contains("Test"));
    assert!(ReviewCategory::ErrorHandling.description().contains("Error"));
    assert!(ReviewCategory::Architecture.description().contains("Architectural"));
    assert!(ReviewCategory::Style.description().contains("style"));
}

#[test]
fn test_review_category_default_severities() {
    // Security issues default to High severity
    assert_eq!(
        ReviewCategory::Security.default_severity(),
        ReviewSeverity::High
    );

    // Performance, Quality, Testing, ErrorHandling default to Medium
    assert_eq!(
        ReviewCategory::Performance.default_severity(),
        ReviewSeverity::Medium
    );
    assert_eq!(
        ReviewCategory::Quality.default_severity(),
        ReviewSeverity::Medium
    );
    assert_eq!(
        ReviewCategory::Testing.default_severity(),
        ReviewSeverity::Medium
    );
    assert_eq!(
        ReviewCategory::ErrorHandling.default_severity(),
        ReviewSeverity::Medium
    );

    // BestPractices and Documentation default to Low
    assert_eq!(
        ReviewCategory::BestPractices.default_severity(),
        ReviewSeverity::Low
    );
    assert_eq!(
        ReviewCategory::Documentation.default_severity(),
        ReviewSeverity::Low
    );

    // Architecture defaults to High
    assert_eq!(
        ReviewCategory::Architecture.default_severity(),
        ReviewSeverity::High
    );

    // Style defaults to Info
    assert_eq!(ReviewCategory::Style.default_severity(), ReviewSeverity::Info);
}

#[test]
fn test_review_category_enabled_for_expert_mode() {
    // All categories should be enabled in expert mode
    assert!(
        ReviewCategory::Security.is_enabled_for_mode(true),
        "Security should always be enabled"
    );
    assert!(
        ReviewCategory::Architecture.is_enabled_for_mode(true),
        "Architecture should be enabled in expert mode"
    );
    assert!(
        ReviewCategory::Documentation.is_enabled_for_mode(true),
        "Documentation should be enabled in expert mode"
    );
    assert!(
        ReviewCategory::Style.is_enabled_for_mode(true),
        "Style should be enabled in expert mode"
    );
}

#[test]
fn test_review_category_enabled_for_standard_mode() {
    // Core categories should be enabled in standard mode
    assert!(
        ReviewCategory::Security.is_enabled_for_mode(false),
        "Security should always be enabled"
    );
    assert!(
        ReviewCategory::Quality.is_enabled_for_mode(false),
        "Quality should always be enabled"
    );
    assert!(
        ReviewCategory::ErrorHandling.is_enabled_for_mode(false),
        "ErrorHandling should always be enabled"
    );

    // Optional categories should be disabled in standard mode
    assert!(
        !ReviewCategory::Architecture.is_enabled_for_mode(false),
        "Architecture should only be enabled in expert mode"
    );
    assert!(
        !ReviewCategory::Documentation.is_enabled_for_mode(false),
        "Documentation should only be enabled in expert mode"
    );
    assert!(
        !ReviewCategory::Style.is_enabled_for_mode(false),
        "Style should only be enabled in expert mode"
    );
}

#[test]
fn test_review_category_display() {
    assert_eq!(ReviewCategory::Security.to_string(), "security");
    assert_eq!(ReviewCategory::Performance.to_string(), "performance");
    assert_eq!(ReviewCategory::Quality.to_string(), "quality");
    assert_eq!(ReviewCategory::BestPractices.to_string(), "best_practices");
    assert_eq!(ReviewCategory::Documentation.to_string(), "documentation");
    assert_eq!(ReviewCategory::Testing.to_string(), "testing");
    assert_eq!(ReviewCategory::ErrorHandling.to_string(), "error_handling");
    assert_eq!(ReviewCategory::Architecture.to_string(), "architecture");
    assert_eq!(ReviewCategory::Style.to_string(), "style");
}

#[test]
fn test_review_category_serde_roundtrip() {
    let original = ReviewCategory::BestPractices;
    let json = serde_json::to_string(&original).unwrap();
    assert_eq!(json, "\"best_practices\"");
    let deserialized: ReviewCategory = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

// =============================================================================
// ReviewFinding Tests
// =============================================================================

#[test]
fn test_review_finding_new_generates_id() {
    let finding = ReviewFinding::new(
        ReviewCategory::Security,
        ReviewSeverity::High,
        "Potential SQL injection vulnerability",
    );

    assert!(!finding.id.is_empty(), "Finding should have an auto-generated ID");
    assert!(finding.id.contains("security"));
    assert_eq!(finding.category, ReviewCategory::Security);
    assert_eq!(finding.severity, ReviewSeverity::High);
    assert_eq!(finding.title, "Potential SQL injection vulnerability");
}

#[test]
fn test_review_finding_default_blocking_based_on_severity() {
    let critical =
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::Critical, "Critical issue");
    assert!(critical.blocking, "Critical findings should be blocking by default");

    let high =
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::High, "High issue");
    assert!(!high.blocking, "High findings should not be blocking by default");
}

#[test]
fn test_review_finding_builder_pattern() {
    let finding = ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::Critical, "SQL Injection")
        .with_file("src/db.rs", 42)
        .with_description("User input is concatenated directly into SQL query")
        .with_suggestion("Use parameterized queries or prepared statements")
        .with_code_snippet("format!(\"SELECT * FROM users WHERE id = {}\", user_input)")
        .blocking(true)
        .with_confidence(0.95)
        .with_tag("security")
        .with_tag("injection")
        .with_cwe("CWE-89")
        .with_reference("https://owasp.org/www-community/attacks/SQL_Injection");

    assert_eq!(finding.file, Some("src/db.rs".to_string()));
    assert_eq!(finding.line, Some(42));
    assert_eq!(finding.category, ReviewCategory::Security);
    assert_eq!(finding.severity, ReviewSeverity::Critical);
    assert_eq!(finding.title, "SQL Injection");
    assert_eq!(
        finding.description,
        "User input is concatenated directly into SQL query"
    );
    assert_eq!(
        finding.suggestion,
        Some("Use parameterized queries or prepared statements".to_string())
    );
    assert!(finding.blocking);
    assert!((finding.confidence - 0.95).abs() < f32::EPSILON);
    assert_eq!(finding.tags, vec!["security", "injection"]);
    assert_eq!(finding.cwe_id, Some("CWE-89".to_string()));
    assert_eq!(
        finding.reference,
        Some("https://owasp.org/www-community/attacks/SQL_Injection".to_string())
    );
}

#[test]
fn test_review_finding_with_file_range() {
    let finding = ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::Medium, "Long function")
        .with_file_range("src/utils.rs", 100, 150);

    assert_eq!(finding.file, Some("src/utils.rs".to_string()));
    assert_eq!(finding.line, Some(100));
    assert_eq!(finding.end_line, Some(150));
}

#[test]
fn test_review_finding_confidence_clamped() {
    let over_confident = ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::Low, "Test")
        .with_confidence(1.5);
    assert!(
        (over_confident.confidence - 1.0).abs() < f32::EPSILON,
        "Confidence should be clamped to 1.0"
    );

    let under_confident = ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::Low, "Test")
        .with_confidence(-0.5);
    assert!(
        (under_confident.confidence - 0.0).abs() < f32::EPSILON,
        "Confidence should be clamped to 0.0"
    );
}

#[test]
fn test_review_finding_add_related() {
    let mut finding =
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::High, "Related issue 1");
    finding.add_related("finding-123");
    finding.add_related("finding-456");

    assert_eq!(finding.related, vec!["finding-123", "finding-456"]);
}

#[test]
fn test_review_finding_meets_severity_threshold() {
    let high =
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::High, "High severity issue");

    assert!(high.meets_severity_threshold(ReviewSeverity::Info));
    assert!(high.meets_severity_threshold(ReviewSeverity::Low));
    assert!(high.meets_severity_threshold(ReviewSeverity::Medium));
    assert!(high.meets_severity_threshold(ReviewSeverity::High));
    assert!(!high.meets_severity_threshold(ReviewSeverity::Critical));
}

#[test]
fn test_review_finding_format() {
    let finding = ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::High, "SQL Injection")
        .with_file("src/db.rs", 42)
        .with_description("User input not sanitized");

    let formatted = finding.format();
    assert!(formatted.contains("SQL Injection"));
    assert!(formatted.contains("src/db.rs:42"));
    assert!(formatted.contains("User input not sanitized"));
    assert!(formatted.contains("HIGH"));
}

#[test]
fn test_review_finding_format_with_range() {
    let finding = ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::Medium, "Long function")
        .with_file_range("src/utils.rs", 100, 150)
        .with_description("Function exceeds 50 lines");

    let formatted = finding.format();
    assert!(formatted.contains("src/utils.rs:100-150"));
}

#[test]
fn test_review_finding_format_without_location() {
    let finding =
        ReviewFinding::new(ReviewCategory::BestPractices, ReviewSeverity::Info, "General advice")
            .with_description("Consider using a more idiomatic approach");

    let formatted = finding.format();
    assert!(formatted.contains("unknown location"));
}

#[test]
fn test_review_finding_serde_roundtrip() {
    let original = ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::Critical, "SQL Injection")
        .with_file("src/db.rs", 42)
        .with_description("User input not sanitized")
        .with_suggestion("Use parameterized queries")
        .blocking(true)
        .with_confidence(0.9);

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ReviewFinding = serde_json::from_str(&json).unwrap();

    assert_eq!(original.category, deserialized.category);
    assert_eq!(original.severity, deserialized.severity);
    assert_eq!(original.title, deserialized.title);
    assert_eq!(original.file, deserialized.file);
    assert_eq!(original.line, deserialized.line);
    assert_eq!(original.blocking, deserialized.blocking);
}

// =============================================================================
// ReviewReport Tests
// =============================================================================

#[test]
fn test_review_report_new() {
    let report = ReviewReport::new("task-123");

    assert_eq!(report.task_id, "task-123");
    assert_eq!(report.assessment, ReviewAssessment::Pass);
    assert!(report.findings.is_empty());
    assert!(report.summary.is_empty());
    assert!(report.strengths.is_empty());
    assert!(report.recommendations.is_empty());
    assert!(!report.retry_recommended);
    assert_eq!(report.review_time_secs, 0);
}

#[test]
fn test_review_report_add_finding_updates_metrics() {
    let mut report = ReviewReport::new("task-123");

    let finding1 = ReviewFinding::new(
        ReviewCategory::Security,
        ReviewSeverity::High,
        "Security issue",
    )
    .with_file("src/main.rs", 10);

    let finding2 = ReviewFinding::new(
        ReviewCategory::Quality,
        ReviewSeverity::Medium,
        "Quality issue",
    )
    .with_file("src/lib.rs", 20);

    report.add_finding(finding1);
    report.add_finding(finding2);

    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.metrics.total_findings, 2);
    assert_eq!(report.metrics.files_affected, 2);
}

#[test]
fn test_review_report_add_findings_batch() {
    let mut report = ReviewReport::new("task-123");

    let findings = vec![
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::High, "Issue 1"),
        ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::Medium, "Issue 2"),
        ReviewFinding::new(ReviewCategory::Performance, ReviewSeverity::Low, "Issue 3"),
    ];

    report.add_findings(findings);

    assert_eq!(report.findings.len(), 3);
    assert_eq!(report.metrics.total_findings, 3);
}

#[test]
fn test_review_report_metrics_by_severity() {
    let mut report = ReviewReport::new("task-123");

    report.add_findings(vec![
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::Critical, "Critical 1"),
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::Critical, "Critical 2"),
        ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::High, "High 1"),
        ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::Medium, "Medium 1"),
    ]);

    assert_eq!(report.metrics.by_severity.get("critical"), Some(&2));
    assert_eq!(report.metrics.by_severity.get("high"), Some(&1));
    assert_eq!(report.metrics.by_severity.get("medium"), Some(&1));
}

#[test]
fn test_review_report_metrics_by_category() {
    let mut report = ReviewReport::new("task-123");

    report.add_findings(vec![
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::High, "Security 1"),
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::Medium, "Security 2"),
        ReviewFinding::new(ReviewCategory::Performance, ReviewSeverity::Low, "Performance 1"),
    ]);

    assert_eq!(report.metrics.by_category.get("security"), Some(&2));
    assert_eq!(report.metrics.by_category.get("performance"), Some(&1));
}

#[test]
fn test_review_report_metrics_blocking_count() {
    let mut report = ReviewReport::new("task-123");

    let mut critical_finding =
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::Critical, "Critical issue");
    critical_finding.blocking = true;

    let high_finding =
        ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::High, "High issue");

    report.add_findings(vec![critical_finding, high_finding]);

    assert_eq!(report.metrics.blocking_count, 1);
}

#[test]
fn test_review_report_metrics_avg_confidence() {
    let mut report = ReviewReport::new("task-123");

    report.add_findings(vec![
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::High, "Issue 1")
            .with_confidence(0.8),
        ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::Medium, "Issue 2")
            .with_confidence(0.9),
        ReviewFinding::new(ReviewCategory::Performance, ReviewSeverity::Low, "Issue 3")
            .with_confidence(0.7),
    ]);

    // Average: (0.8 + 0.9 + 0.7) / 3 = 0.8
    assert!((report.metrics.avg_confidence - 0.8).abs() < f32::EPSILON);
}

#[test]
fn test_review_report_calculate_assessment_pass() {
    let mut report = ReviewReport::new("task-123");
    report.calculate_assessment();

    assert_eq!(report.assessment, ReviewAssessment::Pass);
    assert!(!report.retry_recommended);
}

#[test]
fn test_review_report_calculate_assessment_warning() {
    let mut report = ReviewReport::new("task-123");

    report.add_finding(
        ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::Low, "Minor issue"),
    );
    report.calculate_assessment();

    assert_eq!(report.assessment, ReviewAssessment::Warning);
    assert!(!report.retry_recommended);
}

#[test]
fn test_review_report_calculate_assessment_needs_improvements() {
    let mut report = ReviewReport::new("task-123");

    report.add_finding(
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::High, "High severity issue"),
    );
    report.calculate_assessment();

    assert_eq!(report.assessment, ReviewAssessment::NeedsImprovements);
    assert!(report.retry_recommended);
}

#[test]
fn test_review_report_calculate_assessment_fail_with_critical() {
    let mut report = ReviewReport::new("task-123");

    report.add_finding(
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::Critical, "Critical issue"),
    );
    report.calculate_assessment();

    assert_eq!(report.assessment, ReviewAssessment::Fail);
    assert!(report.retry_recommended);
}

#[test]
fn test_review_report_calculate_assessment_fail_with_blocking() {
    let mut report = ReviewReport::new("task-123");

    let mut finding =
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::High, "Blocking issue");
    finding.blocking = true;

    report.add_finding(finding);
    report.calculate_assessment();

    assert_eq!(report.assessment, ReviewAssessment::Fail);
    assert!(report.retry_recommended);
}

#[test]
fn test_review_report_passes() {
    let mut report = ReviewReport::new("task-123");
    assert!(report.passes());

    report.add_finding(
        ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::Low, "Minor issue"),
    );
    report.calculate_assessment();
    assert!(report.passes()); // Warning still passes

    report.findings.clear();
    report.add_finding(
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::Critical, "Critical issue"),
    );
    report.calculate_assessment();
    assert!(!report.passes()); // Fail doesn't pass
}

#[test]
fn test_review_report_findings_by_severity() {
    let mut report = ReviewReport::new("task-123");

    report.add_findings(vec![
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::Critical, "Critical 1"),
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::Critical, "Critical 2"),
        ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::High, "High 1"),
    ]);

    let critical_findings = report.findings_by_severity(ReviewSeverity::Critical);
    assert_eq!(critical_findings.len(), 2);

    let high_findings = report.findings_by_severity(ReviewSeverity::High);
    assert_eq!(high_findings.len(), 1);

    let medium_findings = report.findings_by_severity(ReviewSeverity::Medium);
    assert!(medium_findings.is_empty());
}

#[test]
fn test_review_report_findings_by_category() {
    let mut report = ReviewReport::new("task-123");

    report.add_findings(vec![
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::High, "Security 1"),
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::Medium, "Security 2"),
        ReviewFinding::new(ReviewCategory::Performance, ReviewSeverity::Low, "Performance 1"),
    ]);

    let security_findings = report.findings_by_category(ReviewCategory::Security);
    assert_eq!(security_findings.len(), 2);

    let performance_findings = report.findings_by_category(ReviewCategory::Performance);
    assert_eq!(performance_findings.len(), 1);

    let quality_findings = report.findings_by_category(ReviewCategory::Quality);
    assert!(quality_findings.is_empty());
}

#[test]
fn test_review_report_to_markdown_basic() {
    let report = ReviewReport::new("task-456");

    let markdown = report.to_markdown();

    assert!(markdown.contains("# Review Report: task-456"));
    assert!(markdown.contains("**Assessment**: pass"));
}

#[test]
fn test_review_report_to_markdown_with_findings() {
    let mut report = ReviewReport::new("task-456");

    report.add_finding(
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::High, "SQL Injection")
            .with_file("src/db.rs", 42)
            .with_description("User input not sanitized")
            .with_suggestion("Use parameterized queries"),
    );
    report.summary = "Found security issues".to_string();
    report.calculate_assessment();

    let markdown = report.to_markdown();

    assert!(markdown.contains("SQL Injection"));
    assert!(markdown.contains("src/db.rs:42"));
    assert!(markdown.contains("User input not sanitized"));
    assert!(markdown.contains("Use parameterized queries"));
    assert!(markdown.contains("Found security issues"));
    // Severity is formatted as heading in markdown: ### HIGH
    assert!(markdown.contains("### HIGH"));
}

#[test]
fn test_review_report_to_markdown_with_strengths_and_recommendations() {
    let mut report = ReviewReport::new("task-456");
    report.strengths = vec![
        "Well-structured code".to_string(),
        "Good test coverage".to_string(),
    ];
    report.recommendations = vec![
        "Add more documentation".to_string(),
        "Consider caching".to_string(),
    ];

    let markdown = report.to_markdown();

    assert!(markdown.contains("## Strengths"));
    assert!(markdown.contains("Well-structured code"));
    assert!(markdown.contains("Good test coverage"));
    assert!(markdown.contains("## Recommendations"));
    assert!(markdown.contains("Add more documentation"));
    assert!(markdown.contains("Consider caching"));
}

// =============================================================================
// ReviewMetrics Tests
// =============================================================================

#[test]
fn test_review_metrics_default() {
    let metrics = ReviewMetrics::default();

    assert_eq!(metrics.total_findings, 0);
    assert!(metrics.by_severity.is_empty());
    assert!(metrics.by_category.is_empty());
    assert_eq!(metrics.blocking_count, 0);
    assert_eq!(metrics.files_affected, 0);
    assert!((metrics.avg_confidence - 0.0).abs() < f32::EPSILON);
    assert_eq!(metrics.lines_reviewed, 0);
}

// =============================================================================
// ReviewConfigSnapshot Tests
// =============================================================================

#[test]
fn test_review_config_snapshot_structure() {
    let snapshot = ReviewConfigSnapshot {
        severity_threshold: ReviewSeverity::Low,
        categories_checked: vec![
            ReviewCategory::Security,
            ReviewCategory::Performance,
            ReviewCategory::Quality,
        ],
        max_issues_per_category: 15,
        expert_mode: true,
    };

    assert_eq!(snapshot.severity_threshold, ReviewSeverity::Low);
    assert_eq!(snapshot.categories_checked.len(), 3);
    assert_eq!(snapshot.max_issues_per_category, 15);
    assert!(snapshot.expert_mode);
}

// =============================================================================
// CodeIssue <-> ReviewFinding Conversion Tests
// =============================================================================

#[test]
fn test_code_issue_to_finding() {
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

    let finding = issue.to_finding();

    assert_eq!(finding.category, ReviewCategory::Security);
    assert_eq!(finding.severity, ReviewSeverity::Critical);
    assert_eq!(finding.file, Some("src/main.rs".to_string()));
    assert_eq!(finding.line, Some(42));
    assert_eq!(finding.title, "SQL Injection");
    assert_eq!(finding.description, "User input not sanitized");
    assert_eq!(finding.suggestion, Some("Use parameterized queries".to_string()));
    assert!(finding.blocking);
}

#[test]
fn test_code_issue_from_finding() {
    let finding = ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::Critical, "SQL Injection")
        .with_file("src/main.rs", 42)
        .with_description("User input not sanitized")
        .with_suggestion("Use parameterized queries")
        .blocking(true);

    let issue = CodeIssue::from_finding(&finding);

    assert_eq!(issue.category, IssueCategory::Security);
    assert_eq!(issue.severity, IssueSeverity::Critical);
    assert_eq!(issue.file, Some("src/main.rs".to_string()));
    assert_eq!(issue.line, Some(42));
    assert_eq!(issue.title, "SQL Injection");
    assert_eq!(issue.description, "User input not sanitized");
    assert_eq!(issue.suggestion, Some("Use parameterized queries".to_string()));
    assert!(issue.blocking);
}

#[test]
fn test_code_issue_roundtrip_conversion() {
    let original = CodeIssue {
        category: IssueCategory::Performance,
        severity: IssueSeverity::Medium,
        file: Some("src/utils.rs".to_string()),
        line: Some(100),
        title: "Inefficient loop".to_string(),
        description: "Loop can be optimized".to_string(),
        suggestion: Some("Use iterator methods".to_string()),
        code_snippet: None,
        blocking: false,
    };

    let finding = original.to_finding();
    let converted = CodeIssue::from_finding(&finding);

    assert_eq!(original.category, converted.category);
    assert_eq!(original.severity, converted.severity);
    assert_eq!(original.file, converted.file);
    assert_eq!(original.line, converted.line);
    assert_eq!(original.title, converted.title);
    assert_eq!(original.description, converted.description);
    assert_eq!(original.suggestion, converted.suggestion);
    assert_eq!(original.blocking, converted.blocking);
}

// =============================================================================
// ReviewAssessment Tests
// =============================================================================

#[test]
fn test_review_assessment_serde_roundtrip() {
    let test_cases = vec![
        ReviewAssessment::Pass,
        ReviewAssessment::Warning,
        ReviewAssessment::NeedsImprovements,
        ReviewAssessment::Fail,
    ];

    for assessment in test_cases {
        let json = serde_json::to_string(&assessment).unwrap();
        let deserialized: ReviewAssessment = serde_json::from_str(&json).unwrap();
        assert_eq!(assessment, deserialized);
    }
}

// =============================================================================
// Integration Tests: Expert Mode vs Standard Mode Behavior
// =============================================================================

#[test]
fn test_expert_mode_finding_threshold() {
    // In expert mode, even Info severity findings should be included
    let info_finding =
        ReviewFinding::new(ReviewCategory::Style, ReviewSeverity::Info, "Style suggestion");

    // Expert mode threshold is typically Low
    assert!(
        info_finding.meets_severity_threshold(ReviewSeverity::Info),
        "Info findings should meet Info threshold in expert mode"
    );
}

#[test]
fn test_standard_mode_finding_threshold() {
    // In standard mode, only Medium+ severity findings should be included
    let low_finding =
        ReviewFinding::new(ReviewCategory::Style, ReviewSeverity::Low, "Minor style issue");

    // Standard mode threshold is typically Medium
    assert!(
        !low_finding.meets_severity_threshold(ReviewSeverity::Medium),
        "Low findings should not meet Medium threshold in standard mode"
    );
}

#[test]
fn test_expert_mode_categories_coverage() {
    // All categories should be checked in expert mode
    let all_categories = [
        ReviewCategory::Security,
        ReviewCategory::Performance,
        ReviewCategory::Quality,
        ReviewCategory::BestPractices,
        ReviewCategory::Documentation,
        ReviewCategory::Testing,
        ReviewCategory::ErrorHandling,
        ReviewCategory::Architecture,
        ReviewCategory::Style,
    ];

    let enabled_count = all_categories
        .iter()
        .filter(|c| c.is_enabled_for_mode(true))
        .count();

    assert_eq!(
        enabled_count,
        all_categories.len(),
        "All categories should be enabled in expert mode"
    );
}

#[test]
fn test_standard_mode_categories_coverage() {
    // Only core categories should be checked in standard mode
    let all_categories = [
        ReviewCategory::Security,
        ReviewCategory::Performance,
        ReviewCategory::Quality,
        ReviewCategory::BestPractices,
        ReviewCategory::Documentation,
        ReviewCategory::Testing,
        ReviewCategory::ErrorHandling,
        ReviewCategory::Architecture,
        ReviewCategory::Style,
    ];

    let enabled: Vec<_> = all_categories
        .iter()
        .filter(|c| c.is_enabled_for_mode(false))
        .copied()
        .collect();

    // These should always be enabled
    assert!(enabled.contains(&ReviewCategory::Security));
    assert!(enabled.contains(&ReviewCategory::ErrorHandling));
    assert!(enabled.contains(&ReviewCategory::Quality));

    // These should not be enabled in standard mode
    assert!(!enabled.contains(&ReviewCategory::Architecture));
    assert!(!enabled.contains(&ReviewCategory::Documentation));
    assert!(!enabled.contains(&ReviewCategory::Style));
}

#[test]
fn test_pipeline_error_handling_compatibility() {
    // Test that ReviewError integrates properly with anyhow error handling
    use anyhow::Result;

    fn simulate_review_failure() -> Result<()> {
        Err(ReviewError::CriticalIssues("Found 3 critical issues".to_string()).into())
    }

    let result = simulate_review_failure();
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.to_string().contains("critical issues"));
}

#[test]
fn test_review_config_severity_threshold_conversion() {
    let config = ReviewConfig::default();
    let threshold = config.severity_threshold_as_review();
    assert_eq!(threshold, ReviewSeverity::Medium);

    let expert_config = ReviewConfig::expert_mode();
    let expert_threshold = expert_config.severity_threshold_as_review();
    assert_eq!(expert_threshold, ReviewSeverity::Low);
}

// =============================================================================
// Additional Edge Case Tests
// =============================================================================

#[test]
fn test_review_error_io_error_from_std_io_error() {
    // Test that std::io::Error can be converted into ReviewError
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let review_err: ReviewError = io_err.into();

    assert_eq!(review_err.error_code(), "REVIEW_IO_ERROR");
    assert!(!review_err.is_recoverable(), "IoError should not be recoverable");
    assert!(!review_err.is_fatal(), "IoError should not be fatal");
}

#[test]
fn test_review_finding_with_empty_tags() {
    let finding = ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::Low, "Test")
        .with_tag("first")
        .with_tag("second");

    assert_eq!(finding.tags.len(), 2);
    assert!(finding.tags.contains(&"first".to_string()));
    assert!(finding.tags.contains(&"second".to_string()));
}

#[test]
fn test_review_report_empty_findings_by_severity() {
    let report = ReviewReport::new("task-empty");

    // Should return empty vector for any severity when no findings exist
    assert!(report.findings_by_severity(ReviewSeverity::Critical).is_empty());
    assert!(report.findings_by_severity(ReviewSeverity::High).is_empty());
    assert!(report.findings_by_severity(ReviewSeverity::Medium).is_empty());
    assert!(report.findings_by_severity(ReviewSeverity::Low).is_empty());
    assert!(report.findings_by_severity(ReviewSeverity::Info).is_empty());
}

#[test]
fn test_review_report_empty_findings_by_category() {
    let report = ReviewReport::new("task-empty");

    // Should return empty vector for any category when no findings exist
    assert!(report.findings_by_category(ReviewCategory::Security).is_empty());
    assert!(report.findings_by_category(ReviewCategory::Performance).is_empty());
    assert!(report.findings_by_category(ReviewCategory::Quality).is_empty());
}

#[test]
fn test_review_report_multiple_findings_same_file() {
    let mut report = ReviewReport::new("task-123");

    // Add multiple findings in the same file
    report.add_finding(
        ReviewFinding::new(ReviewCategory::Security, ReviewSeverity::High, "Issue 1")
            .with_file("src/lib.rs", 10),
    );
    report.add_finding(
        ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::Medium, "Issue 2")
            .with_file("src/lib.rs", 20),
    );
    report.add_finding(
        ReviewFinding::new(ReviewCategory::Performance, ReviewSeverity::Low, "Issue 3")
            .with_file("src/lib.rs", 30),
    );

    // Should count as 1 file affected, not 3
    assert_eq!(report.metrics.files_affected, 1);
    assert_eq!(report.findings.len(), 3);
}

#[test]
fn test_review_finding_confidence_boundary_values() {
    // Test exact boundary values
    let exact_one = ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::Low, "Test")
        .with_confidence(1.0);
    assert!((exact_one.confidence - 1.0).abs() < f32::EPSILON);

    let exact_zero = ReviewFinding::new(ReviewCategory::Quality, ReviewSeverity::Low, "Test")
        .with_confidence(0.0);
    assert!((exact_zero.confidence - 0.0).abs() < f32::EPSILON);
}

#[test]
fn test_review_severity_ordering_edge_cases() {
    // Test transitivity
    assert!(ReviewSeverity::Critical > ReviewSeverity::Medium);
    assert!(ReviewSeverity::Medium > ReviewSeverity::Low);
    // If Critical > Medium and Medium > Low, then Critical > Low (transitivity)
    assert!(ReviewSeverity::Critical > ReviewSeverity::Low);

    // Test self-comparison
    assert!(ReviewSeverity::Medium >= ReviewSeverity::Medium);
    assert!(ReviewSeverity::Medium <= ReviewSeverity::Medium);
}

#[test]
fn test_review_category_is_enabled_consistency() {
    // Security should always be enabled in both modes
    assert!(ReviewCategory::Security.is_enabled_for_mode(true));
    assert!(ReviewCategory::Security.is_enabled_for_mode(false));

    // Quality should always be enabled in both modes
    assert!(ReviewCategory::Quality.is_enabled_for_mode(true));
    assert!(ReviewCategory::Quality.is_enabled_for_mode(false));

    // ErrorHandling should always be enabled in both modes
    assert!(ReviewCategory::ErrorHandling.is_enabled_for_mode(true));
    assert!(ReviewCategory::ErrorHandling.is_enabled_for_mode(false));
}

#[test]
fn test_review_assessment_serde_values() {
    // Note: serde(rename_all = "lowercase") converts NeedsImprovements to "needsimprovements"
    assert_eq!(serde_json::to_string(&ReviewAssessment::Pass).unwrap(), "\"pass\"");
    assert_eq!(serde_json::to_string(&ReviewAssessment::Warning).unwrap(), "\"warning\"");
    assert_eq!(
        serde_json::to_string(&ReviewAssessment::NeedsImprovements).unwrap(),
        "\"needsimprovements\""
    );
    assert_eq!(serde_json::to_string(&ReviewAssessment::Fail).unwrap(), "\"fail\"");
}

#[test]
fn test_code_issue_with_code_snippet_conversion() {
    let issue = CodeIssue {
        category: IssueCategory::Security,
        severity: IssueSeverity::High,
        file: Some("src/auth.rs".to_string()),
        line: Some(100),
        title: "Hardcoded secret".to_string(),
        description: "API key is hardcoded in source".to_string(),
        suggestion: Some("Use environment variables".to_string()),
        code_snippet: Some("const API_KEY: &str = \"secret123\";".to_string()),
        blocking: false,
    };

    let finding = issue.to_finding();
    assert_eq!(
        finding.code_snippet,
        Some("const API_KEY: &str = \"secret123\";".to_string())
    );

    // Convert back
    let converted = CodeIssue::from_finding(&finding);
    assert_eq!(converted.code_snippet, issue.code_snippet);
}