//! Integration tests for coverage analysis and fix cycle triggering
//!
//! This test suite verifies that the coverage analysis and fix cycle
//! modules integrate correctly with the existing execute/test/verify stages.

use ltmatrix::pipeline::coverage::{
    AggregatedFindings, CoverageConfig, CoverageReport, IssueDetail, IssueSeverity, SecurityIssue,
    TestFailure,
};
use ltmatrix::pipeline::fix_cycle::{
    determine_fix_strategy, execute_fix_cycle, should_trigger_fix_cycle, FixCycleConfig,
    FixCycleTrigger, FixStrategy,
};

// ==================== Coverage Configuration Tests ====================

#[test]
fn test_coverage_config_default_values() {
    let config = CoverageConfig::default();
    assert_eq!(config.min_coverage_percent, 70.0);
    assert!(config.fail_on_low_coverage);
    assert!(config.generate_reports);
    assert_eq!(config.include_paths.len(), 1);
    assert_eq!(config.include_paths[0], std::path::PathBuf::from("src"));
}

#[test]
fn test_coverage_config_strict_mode() {
    let config = CoverageConfig::strict_mode();
    assert_eq!(config.min_coverage_percent, 90.0);
    assert!(config.fail_on_low_coverage);
    assert!(config.generate_reports);
}

#[test]
fn test_coverage_config_lenient_mode() {
    let config = CoverageConfig::lenient_mode();
    assert_eq!(config.min_coverage_percent, 50.0);
    assert!(!config.fail_on_low_coverage);
    assert!(!config.generate_reports);
}

#[test]
fn test_coverage_config_cloning() {
    let config = CoverageConfig::strict_mode();
    let cloned = config.clone();
    assert_eq!(config.min_coverage_percent, cloned.min_coverage_percent);
    assert_eq!(config.fail_on_low_coverage, cloned.fail_on_low_coverage);
}

// ==================== Coverage Report Tests ====================

#[test]
fn test_coverage_report_creation() {
    let report = CoverageReport {
        total_lines: 1000,
        covered_lines: 750,
        coverage_percent: 75.0,
        modules: vec![],
        low_coverage_files: vec![],
        low_coverage_modules: vec![],
        meets_threshold: true,
        analysis_duration_secs: 5,
    };

    assert_eq!(report.total_lines, 1000);
    assert_eq!(report.covered_lines, 750);
    assert_eq!(report.coverage_percent, 75.0);
    assert!(report.meets_threshold);
    assert_eq!(report.analysis_duration_secs, 5);
}

#[test]
fn test_coverage_report_threshold_calculation() {
    let mut report = CoverageReport {
        total_lines: 1000,
        covered_lines: 650,
        coverage_percent: 65.0,
        modules: vec![],
        low_coverage_files: vec![],
        low_coverage_modules: vec![],
        meets_threshold: false,
        analysis_duration_secs: 3,
    };

    // Test with 70% threshold
    assert_eq!(report.coverage_percent, 65.0);
    assert!(!report.meets_threshold);

    // Update to meet threshold
    report.covered_lines = 750;
    report.coverage_percent = 75.0;
    report.meets_threshold = true;
    assert!(report.meets_threshold);
}

#[test]
fn test_coverage_report_zero_coverage() {
    let report = CoverageReport {
        total_lines: 1000,
        covered_lines: 0,
        coverage_percent: 0.0,
        modules: vec![],
        low_coverage_files: vec![],
        low_coverage_modules: vec![],
        meets_threshold: false,
        analysis_duration_secs: 0,
    };

    assert_eq!(report.coverage_percent, 0.0);
    assert!(!report.meets_threshold);
}

#[test]
fn test_coverage_report_perfect_coverage() {
    let report = CoverageReport {
        total_lines: 1000,
        covered_lines: 1000,
        coverage_percent: 100.0,
        modules: vec![],
        low_coverage_files: vec![],
        low_coverage_modules: vec![],
        meets_threshold: true,
        analysis_duration_secs: 1,
    };

    assert_eq!(report.coverage_percent, 100.0);
    assert!(report.meets_threshold);
}

// ==================== Aggregated Findings Tests ====================

#[test]
fn test_aggregated_findings_empty() {
    let findings = AggregatedFindings::new();
    assert_eq!(findings.critical_count, 0);
    assert_eq!(findings.high_count, 0);
    assert_eq!(findings.medium_count, 0);
    assert_eq!(findings.low_count, 0);
    assert!(findings.test_failures.is_empty());
    assert!(findings.security_issues.is_empty());
    assert!(findings.performance_issues.is_empty());
    assert!(!findings.has_critical_issues());
}

#[test]
fn test_aggregated_findings_add_critical_issue() {
    let mut findings = AggregatedFindings::new();

    let critical_issue = SecurityIssue {
        title: "Buffer Overflow".to_string(),
        description: "Critical security issue".to_string(),
        cve_id: Some("CVE-2024-1234".to_string()),
        severity: IssueSeverity::Critical,
        affected_component: "src/buffer.rs".to_string(),
        suggested_fix: Some("Add bounds checking".to_string()),
        references: vec![],
    };

    findings.add_security_issue(critical_issue);

    assert_eq!(findings.critical_count, 1);
    assert!(findings.has_critical_issues());
    assert_eq!(findings.security_issues.len(), 1);
}

#[test]
fn test_aggregated_findings_add_multiple_severities() {
    let mut findings = AggregatedFindings::new();

    // Add critical issue
    findings.add_security_issue(SecurityIssue {
        title: "Critical".to_string(),
        description: "Test".to_string(),
        cve_id: None,
        severity: IssueSeverity::Critical,
        affected_component: "test.rs".to_string(),
        suggested_fix: None,
        references: vec![],
    });

    // Add high priority issue
    findings.add_test_failure(TestFailure {
        test_name: "test_high".to_string(),
        file_path: std::path::PathBuf::from("test.rs"),
        line_number: 10,
        message: "High priority failure".to_string(),
        stack_trace: None,
        severity: IssueSeverity::High,
        is_flaky: false,
        suggested_fix: None,
    });

    // Add medium priority issue
    findings.add_test_failure(TestFailure {
        test_name: "test_medium".to_string(),
        file_path: std::path::PathBuf::from("test.rs"),
        line_number: 20,
        message: "Medium priority failure".to_string(),
        stack_trace: None,
        severity: IssueSeverity::Medium,
        is_flaky: false,
        suggested_fix: None,
    });

    assert_eq!(findings.critical_count, 1);
    assert_eq!(findings.high_count, 1);
    assert_eq!(findings.medium_count, 1);
    assert_eq!(findings.low_count, 0);
}

#[test]
fn test_aggregated_findings_exceeds_threshold() {
    let mut findings = AggregatedFindings::new();

    // Add 2 low-severity issues
    for i in 0..2 {
        findings.add_test_failure(TestFailure {
            test_name: format!("test_{}", i),
            file_path: std::path::PathBuf::from("test.rs"),
            line_number: i,
            message: "Low priority failure".to_string(),
            stack_trace: None,
            severity: IssueSeverity::Low,
            is_flaky: false,
            suggested_fix: None,
        });
    }

    // Should not exceed threshold of 5
    assert!(!findings.exceeds_threshold(5));

    // Add more issues
    for i in 2..6 {
        findings.add_test_failure(TestFailure {
            test_name: format!("test_{}", i),
            file_path: std::path::PathBuf::from("test.rs"),
            line_number: i,
            message: "Low priority failure".to_string(),
            stack_trace: None,
            severity: IssueSeverity::Low,
            is_flaky: false,
            suggested_fix: None,
        });
    }

    // Should exceed threshold of 5
    assert!(findings.exceeds_threshold(5));
}

#[test]
fn test_aggregated_findings_with_coverage() {
    let mut findings = AggregatedFindings::new();

    findings.coverage = Some(CoverageReport {
        total_lines: 1000,
        covered_lines: 400,
        coverage_percent: 40.0,
        modules: vec![],
        low_coverage_files: vec![],
        low_coverage_modules: vec![],
        meets_threshold: false,
        analysis_duration_secs: 2,
    });

    assert!(findings.coverage.is_some());
    let coverage = findings.coverage.as_ref().unwrap();
    assert_eq!(coverage.coverage_percent, 40.0);
    assert!(!coverage.meets_threshold);
}

// ==================== Fix Cycle Configuration Tests ====================

#[test]
fn test_fix_cycle_config_default() {
    let config = FixCycleConfig::default();
    assert!(config.enabled);
    assert_eq!(config.max_fix_attempts, 3);
    assert_eq!(config.fix_timeout, 600);
    assert!(!config.require_confirmation);
    assert_eq!(config.auto_fix_threshold, IssueSeverity::High);
}

#[test]
fn test_fix_cycle_config_fast_mode() {
    let config = FixCycleConfig::fast_mode();
    assert!(config.enabled);
    assert_eq!(config.max_fix_attempts, 1);
    assert_eq!(config.fix_timeout, 300);
    assert_eq!(config.auto_fix_threshold, IssueSeverity::Critical);
}

#[test]
fn test_fix_cycle_config_expert_mode() {
    let config = FixCycleConfig::expert_mode();
    assert!(config.enabled);
    assert_eq!(config.max_fix_attempts, 5);
    assert_eq!(config.fix_timeout, 1200);
    assert!(config.require_confirmation);
    assert_eq!(config.auto_fix_threshold, IssueSeverity::Medium);
}

#[test]
fn test_should_auto_fix_default_config() {
    let config = FixCycleConfig::default();

    // Should auto-fix critical and high (threshold is High)
    assert!(config.should_auto_fix(IssueSeverity::Critical));
    assert!(config.should_auto_fix(IssueSeverity::High));

    // Should not auto-fix medium and low
    assert!(!config.should_auto_fix(IssueSeverity::Medium));
    assert!(!config.should_auto_fix(IssueSeverity::Low));
}

#[test]
fn test_should_auto_fix_strict_threshold() {
    let config = FixCycleConfig {
        auto_fix_threshold: IssueSeverity::Critical,
        ..Default::default()
    };

    // Should only auto-fix critical
    assert!(config.should_auto_fix(IssueSeverity::Critical));
    assert!(!config.should_auto_fix(IssueSeverity::High));
    assert!(!config.should_auto_fix(IssueSeverity::Medium));
    assert!(!config.should_auto_fix(IssueSeverity::Low));
}

// ==================== Fix Strategy Tests ====================

#[test]
fn test_determine_fix_strategy_critical_security() {
    let strategy = determine_fix_strategy(IssueSeverity::Critical, FixCycleTrigger::SecurityIssue);
    assert_eq!(strategy, FixStrategy::Immediate);
}

#[test]
fn test_determine_fix_strategy_critical_test_failure() {
    let strategy = determine_fix_strategy(IssueSeverity::Critical, FixCycleTrigger::TestFailure);
    assert_eq!(strategy, FixStrategy::FixAndVerify);
}

#[test]
fn test_determine_fix_strategy_high_test_failure() {
    let strategy = determine_fix_strategy(IssueSeverity::High, FixCycleTrigger::TestFailure);
    assert_eq!(strategy, FixStrategy::FixAndTest);
}

#[test]
fn test_determine_fix_strategy_medium_issues() {
    let strategy = determine_fix_strategy(IssueSeverity::Medium, FixCycleTrigger::TestFailure);
    assert_eq!(strategy, FixStrategy::FixAndTest);
}

#[test]
fn test_determine_fix_strategy_low_issues() {
    let strategy = determine_fix_strategy(IssueSeverity::Low, FixCycleTrigger::TestFailure);
    assert_eq!(strategy, FixStrategy::SuggestOnly);
}

#[test]
fn test_determine_fix_strategy_verification_failure() {
    let strategy = determine_fix_strategy(
        IssueSeverity::Critical,
        FixCycleTrigger::VerificationFailure,
    );
    assert_eq!(strategy, FixStrategy::FixAndVerify);
}

// ==================== Fix Cycle Trigger Tests ====================

#[test]
fn test_should_trigger_fix_cycle_no_issues() {
    let findings = AggregatedFindings::new();
    assert!(!should_trigger_fix_cycle(&findings));
}

#[test]
fn test_should_trigger_fix_cycle_critical_issue() {
    let mut findings = AggregatedFindings::new();

    findings.add_security_issue(SecurityIssue {
        title: "Critical".to_string(),
        description: "Test".to_string(),
        cve_id: None,
        severity: IssueSeverity::Critical,
        affected_component: "test.rs".to_string(),
        suggested_fix: None,
        references: vec![],
    });

    assert!(should_trigger_fix_cycle(&findings));
}

#[test]
fn test_should_trigger_fix_cycle_high_test_failure() {
    let mut findings = AggregatedFindings::new();

    findings.add_test_failure(TestFailure {
        test_name: "test_high".to_string(),
        file_path: std::path::PathBuf::from("test.rs"),
        line_number: 10,
        message: "High priority failure".to_string(),
        stack_trace: None,
        severity: IssueSeverity::High,
        is_flaky: false,
        suggested_fix: None,
    });

    assert!(should_trigger_fix_cycle(&findings));
}

#[test]
fn test_should_trigger_fix_cycle_low_coverage() {
    let mut findings = AggregatedFindings::new();

    findings.coverage = Some(CoverageReport {
        total_lines: 1000,
        covered_lines: 400,
        coverage_percent: 40.0,
        modules: vec![],
        low_coverage_files: vec![],
        low_coverage_modules: vec![],
        meets_threshold: false,
        analysis_duration_secs: 2,
    });

    // Should trigger due to < 50% coverage
    assert!(should_trigger_fix_cycle(&findings));
}

#[test]
fn test_should_trigger_fix_cycle_medium_coverage() {
    let mut findings = AggregatedFindings::new();

    findings.coverage = Some(CoverageReport {
        total_lines: 1000,
        covered_lines: 600,
        coverage_percent: 60.0,
        modules: vec![],
        low_coverage_files: vec![],
        low_coverage_modules: vec![],
        meets_threshold: false,
        analysis_duration_secs: 2,
    });

    // Should not trigger (coverage >= 50%)
    assert!(!should_trigger_fix_cycle(&findings));
}

#[test]
fn test_should_trigger_fix_cycle_medium_and_low_issues() {
    let mut findings = AggregatedFindings::new();

    // Add only medium and low priority issues
    findings.add_test_failure(TestFailure {
        test_name: "test_medium".to_string(),
        file_path: std::path::PathBuf::from("test.rs"),
        line_number: 10,
        message: "Medium priority failure".to_string(),
        stack_trace: None,
        severity: IssueSeverity::Medium,
        is_flaky: false,
        suggested_fix: None,
    });

    findings.add_test_failure(TestFailure {
        test_name: "test_low".to_string(),
        file_path: std::path::PathBuf::from("test.rs"),
        line_number: 20,
        message: "Low priority failure".to_string(),
        stack_trace: None,
        severity: IssueSeverity::Low,
        is_flaky: false,
        suggested_fix: None,
    });

    // Should not trigger (no critical or high issues)
    assert!(!should_trigger_fix_cycle(&findings));
}

// ==================== Fix Cycle Trigger Enum Tests ====================

#[test]
fn test_fix_cycle_trigger_equality() {
    assert_eq!(FixCycleTrigger::TestFailure, FixCycleTrigger::TestFailure);
    assert_eq!(
        FixCycleTrigger::VerificationFailure,
        FixCycleTrigger::VerificationFailure
    );
    assert_eq!(
        FixCycleTrigger::SecurityIssue,
        FixCycleTrigger::SecurityIssue
    );
    assert_eq!(
        FixCycleTrigger::PerformanceIssue,
        FixCycleTrigger::PerformanceIssue
    );
    assert_eq!(FixCycleTrigger::LowCoverage, FixCycleTrigger::LowCoverage);
    assert_eq!(FixCycleTrigger::Manual, FixCycleTrigger::Manual);
}

#[test]
fn test_fix_cycle_trigger_inequality() {
    assert_ne!(
        FixCycleTrigger::TestFailure,
        FixCycleTrigger::VerificationFailure
    );
    assert_ne!(
        FixCycleTrigger::SecurityIssue,
        FixCycleTrigger::PerformanceIssue
    );
    assert_ne!(FixCycleTrigger::LowCoverage, FixCycleTrigger::Manual);
}

// ==================== Fix Strategy Enum Tests ====================

#[test]
fn test_fix_strategy_equality() {
    assert_eq!(FixStrategy::Immediate, FixStrategy::Immediate);
    assert_eq!(FixStrategy::FixAndTest, FixStrategy::FixAndTest);
    assert_eq!(FixStrategy::FixAndVerify, FixStrategy::FixAndVerify);
    assert_eq!(FixStrategy::SuggestOnly, FixStrategy::SuggestOnly);
}

#[test]
fn test_fix_strategy_inequality() {
    assert_ne!(FixStrategy::Immediate, FixStrategy::FixAndTest);
    assert_ne!(FixStrategy::FixAndTest, FixStrategy::FixAndVerify);
    assert_ne!(FixStrategy::FixAndVerify, FixStrategy::SuggestOnly);
}

// ==================== Issue Severity Tests ====================

#[test]
fn test_issue_severity_ordering() {
    // The enum is defined as: Critical, High, Medium, Low
    // So the discriminant values are: Critical = 0, High = 1, Medium = 2, Low = 3
    assert_eq!(IssueSeverity::Critical as u8, 0);
    assert_eq!(IssueSeverity::High as u8, 1);
    assert_eq!(IssueSeverity::Medium as u8, 2);
    assert_eq!(IssueSeverity::Low as u8, 3);

    // Lower numeric value = higher severity
    assert!((IssueSeverity::Critical as u8) < (IssueSeverity::High as u8));
    assert!((IssueSeverity::High as u8) < (IssueSeverity::Medium as u8));
    assert!((IssueSeverity::Medium as u8) < (IssueSeverity::Low as u8));
}

#[test]
fn test_issue_severity_equality() {
    assert_eq!(IssueSeverity::Critical, IssueSeverity::Critical);
    assert_eq!(IssueSeverity::High, IssueSeverity::High);
    assert_eq!(IssueSeverity::Medium, IssueSeverity::Medium);
    assert_eq!(IssueSeverity::Low, IssueSeverity::Low);
}

#[test]
fn test_issue_severity_inequality() {
    assert_ne!(IssueSeverity::Critical, IssueSeverity::High);
    assert_ne!(IssueSeverity::High, IssueSeverity::Medium);
    assert_ne!(IssueSeverity::Medium, IssueSeverity::Low);
}

// ==================== Integration with Existing Stages ====================

#[tokio::test]
async fn test_fix_cycle_integration_with_disabled_config() {
    let findings = AggregatedFindings::new();

    let mut config = FixCycleConfig::default();
    config.enabled = false;

    let summary = execute_fix_cycle(&findings, &config, FixCycleTrigger::TestFailure)
        .await
        .unwrap();

    assert_eq!(summary.total_issues, 0);
    assert_eq!(summary.fixed_issues, 0);
    assert_eq!(summary.failed_issues, 0);
    assert_eq!(summary.skipped_issues, 0);
    assert_eq!(summary.total_attempts, 0);
}

#[tokio::test]
async fn test_fix_cycle_integration_with_critical_issue() {
    let mut findings = AggregatedFindings::new();

    findings.add_security_issue(SecurityIssue {
        title: "Buffer Overflow".to_string(),
        description: "Unsafe buffer operation".to_string(),
        cve_id: None,
        severity: IssueSeverity::Critical,
        affected_component: "src/buffer.rs".to_string(),
        suggested_fix: Some("Add bounds checking".to_string()),
        references: vec![],
    });

    let config = FixCycleConfig {
        enabled: false, // Disable to avoid actual agent calls in tests
        ..Default::default()
    };

    let summary = execute_fix_cycle(&findings, &config, FixCycleTrigger::SecurityIssue)
        .await
        .unwrap();

    // With disabled config, all issues should be skipped
    assert_eq!(summary.total_issues, 1);
    assert_eq!(summary.skipped_issues, 1);
    assert_eq!(summary.fixed_issues, 0);
    assert_eq!(summary.failed_issues, 0);
}

// ==================== Complex Integration Scenarios ====================

#[test]
fn test_complex_findings_aggregation() {
    let mut findings = AggregatedFindings::new();

    // Add multiple issues of different types and severities
    findings.add_test_failure(TestFailure {
        test_name: "test_critical".to_string(),
        file_path: std::path::PathBuf::from("test.rs"),
        line_number: 1,
        message: "Critical test failure".to_string(),
        stack_trace: None,
        severity: IssueSeverity::Critical,
        is_flaky: false,
        suggested_fix: None,
    });

    findings.add_security_issue(SecurityIssue {
        title: "SQL Injection".to_string(),
        description: "User input not sanitized".to_string(),
        cve_id: None,
        severity: IssueSeverity::High,
        affected_component: "src/database.rs".to_string(),
        suggested_fix: Some("Use parameterized queries".to_string()),
        references: vec![],
    });

    findings.add_test_failure(TestFailure {
        test_name: "test_medium".to_string(),
        file_path: std::path::PathBuf::from("test.rs"),
        line_number: 10,
        message: "Medium priority failure".to_string(),
        stack_trace: None,
        severity: IssueSeverity::Medium,
        is_flaky: false,
        suggested_fix: None,
    });

    findings.coverage = Some(CoverageReport {
        total_lines: 1000,
        covered_lines: 750,
        coverage_percent: 75.0,
        modules: vec![],
        low_coverage_files: vec![],
        low_coverage_modules: vec![],
        meets_threshold: true,
        analysis_duration_secs: 5,
    });

    // Verify counts
    assert_eq!(findings.critical_count, 1);
    assert_eq!(findings.high_count, 1);
    assert_eq!(findings.medium_count, 1);
    assert_eq!(findings.low_count, 0);
    assert_eq!(findings.test_failures.len(), 2);
    assert_eq!(findings.security_issues.len(), 1);
    assert!(findings.coverage.is_some());

    // Should trigger fix cycle
    assert!(should_trigger_fix_cycle(&findings));
}

#[test]
fn test_coverage_threshold_boundary() {
    let mut findings = AggregatedFindings::new();

    // Exactly 50% coverage - should not trigger
    findings.coverage = Some(CoverageReport {
        total_lines: 1000,
        covered_lines: 500,
        coverage_percent: 50.0,
        modules: vec![],
        low_coverage_files: vec![],
        low_coverage_modules: vec![],
        meets_threshold: false,
        analysis_duration_secs: 2,
    });

    assert!(!should_trigger_fix_cycle(&findings));

    // Just below 50% - should trigger
    findings.coverage = Some(CoverageReport {
        total_lines: 1000,
        covered_lines: 499,
        coverage_percent: 49.9,
        modules: vec![],
        low_coverage_files: vec![],
        low_coverage_modules: vec![],
        meets_threshold: false,
        analysis_duration_secs: 2,
    });

    assert!(should_trigger_fix_cycle(&findings));
}

#[test]
fn test_issue_detail_trait_implementation() {
    // Test that TestFailure implements IssueDetail
    let failure = TestFailure {
        test_name: "test_example".to_string(),
        file_path: std::path::PathBuf::from("test.rs"),
        line_number: 42,
        message: "Test failed".to_string(),
        stack_trace: None,
        severity: IssueSeverity::High,
        is_flaky: false,
        suggested_fix: Some("Fix the test".to_string()),
    };

    assert_eq!(failure.title(), "test_example");
    assert_eq!(failure.description(), "Test failed");
    assert_eq!(failure.severity(), IssueSeverity::High);
    assert_eq!(failure.affected_component(), "test.rs");
    assert_eq!(failure.suggested_fix(), Some("Fix the test"));
}

#[test]
fn test_security_issue_detail_implementation() {
    let issue = SecurityIssue {
        title: "XSS Vulnerability".to_string(),
        description: "Cross-site scripting vulnerability".to_string(),
        cve_id: Some("CVE-2024-5678".to_string()),
        severity: IssueSeverity::Critical,
        affected_component: "src/render.rs".to_string(),
        suggested_fix: Some("Sanitize user input".to_string()),
        references: vec!["https://owasp.org".to_string()],
    };

    assert_eq!(issue.title(), "XSS Vulnerability");
    assert_eq!(issue.description(), "Cross-site scripting vulnerability");
    assert_eq!(issue.severity(), IssueSeverity::Critical);
    assert_eq!(issue.affected_component(), "src/render.rs");
    assert_eq!(issue.suggested_fix(), Some("Sanitize user input"));
}
