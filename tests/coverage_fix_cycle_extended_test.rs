//! Extended integration tests for coverage analysis and fix cycle triggering
//!
//! This test suite provides additional coverage for edge cases,
//! performance issues, and serialization scenarios.

use ltmatrix::pipeline::coverage::{
    aggregate_findings, AggregatedFindings, CoverageReport, FileCoverage,
    IssueDetail, IssueSeverity, ModuleCoverage, PerformanceIssue, SecurityIssue, TestFailure,
};
use ltmatrix::pipeline::fix_cycle::{should_trigger_fix_cycle, FixCycleConfig};

// ==================== Performance Issue Tests ====================

#[test]
fn test_performance_issue_creation() {
    let issue = PerformanceIssue {
        title: "Slow Algorithm".to_string(),
        description: "O(n²) complexity detected".to_string(),
        severity: IssueSeverity::High,
        affected_component: "src/sort.rs".to_string(),
        metric: "Time Complexity".to_string(),
        actual_value: 10000.0,
        threshold_value: 1000.0,
        suggested_fix: Some("Use O(n log n) algorithm".to_string()),
    };

    assert_eq!(issue.title(), "Slow Algorithm");
    assert_eq!(issue.severity(), IssueSeverity::High);
    assert_eq!(issue.metric, "Time Complexity");
    assert_eq!(issue.actual_value, 10000.0);
    assert_eq!(issue.threshold_value, 1000.0);
}

#[test]
fn test_performance_issue_detail_trait() {
    let issue = PerformanceIssue {
        title: "Memory Leak".to_string(),
        description: "Memory usage grows unbounded".to_string(),
        severity: IssueSeverity::Critical,
        affected_component: "src/cache.rs".to_string(),
        metric: "Memory Growth".to_string(),
        actual_value: 1000000.0,
        threshold_value: 100000.0,
        suggested_fix: Some("Implement cache eviction".to_string()),
    };

    assert_eq!(issue.title(), "Memory Leak");
    assert_eq!(issue.description(), "Memory usage grows unbounded");
    assert_eq!(issue.severity(), IssueSeverity::Critical);
    assert_eq!(issue.affected_component(), "src/cache.rs");
    assert_eq!(
        issue.suggested_fix(),
        Some("Implement cache eviction")
    );
}

// ==================== File Coverage Tests ====================

#[test]
fn test_file_coverage_creation() {
    let coverage = FileCoverage {
        path: std::path::PathBuf::from("src/lib.rs"),
        total_lines: 100,
        covered_lines: 75,
        coverage_percent: 75.0,
        uncovered_lines: vec![10, 20, 30, 40, 50],
        meets_threshold: true,
    };

    assert_eq!(coverage.total_lines, 100);
    assert_eq!(coverage.covered_lines, 75);
    assert_eq!(coverage.coverage_percent, 75.0);
    assert_eq!(coverage.uncovered_lines.len(), 5);
    assert!(coverage.meets_threshold);
}

#[test]
fn test_file_coverage_below_threshold() {
    let coverage = FileCoverage {
        path: std::path::PathBuf::from("src/low_coverage.rs"),
        total_lines: 100,
        covered_lines: 40,
        coverage_percent: 40.0,
        uncovered_lines: vec![1, 2, 3, 4, 5],
        meets_threshold: false,
    };

    assert_eq!(coverage.coverage_percent, 40.0);
    assert!(!coverage.meets_threshold);
}

#[test]
fn test_file_coverage_perfect() {
    let coverage = FileCoverage {
        path: std::path::PathBuf::from("src/perfect.rs"),
        total_lines: 50,
        covered_lines: 50,
        coverage_percent: 100.0,
        uncovered_lines: vec![],
        meets_threshold: true,
    };

    assert_eq!(coverage.coverage_percent, 100.0);
    assert!(coverage.uncovered_lines.is_empty());
    assert!(coverage.meets_threshold);
}

// ==================== Module Coverage Tests ====================

#[test]
fn test_module_coverage_creation() {
    let module = ModuleCoverage {
        name: "pipeline".to_string(),
        files: vec![FileCoverage {
            path: std::path::PathBuf::from("src/pipeline/mod.rs"),
            total_lines: 100,
            covered_lines: 80,
            coverage_percent: 80.0,
            uncovered_lines: vec![],
            meets_threshold: true,
        }],
        total_lines: 100,
        covered_lines: 80,
        coverage_percent: 80.0,
        meets_threshold: true,
    };

    assert_eq!(module.name, "pipeline");
    assert_eq!(module.files.len(), 1);
    assert_eq!(module.coverage_percent, 80.0);
    assert!(module.meets_threshold);
}

#[test]
fn test_module_coverage_multiple_files() {
    let module = ModuleCoverage {
        name: "agent".to_string(),
        files: vec![
            FileCoverage {
                path: std::path::PathBuf::from("src/agent/mod.rs"),
                total_lines: 100,
                covered_lines: 90,
                coverage_percent: 90.0,
                uncovered_lines: vec![],
                meets_threshold: true,
            },
            FileCoverage {
                path: std::path::PathBuf::from("src/agent/pool.rs"),
                total_lines: 200,
                covered_lines: 140,
                coverage_percent: 70.0,
                uncovered_lines: vec![],
                meets_threshold: true,
            },
        ],
        total_lines: 300,
        covered_lines: 230,
        coverage_percent: 76.67,
        meets_threshold: true,
    };

    assert_eq!(module.files.len(), 2);
    assert_eq!(module.total_lines, 300);
    assert_eq!(module.coverage_percent, 76.67);
}

// ==================== Aggregated Findings Extended Tests ====================

#[test]
fn test_aggregated_findings_getters() {
    let mut findings = AggregatedFindings::new();

    // Add test failures with different severities
    findings.add_test_failure(TestFailure {
        test_name: "test_critical".to_string(),
        file_path: std::path::PathBuf::from("test.rs"),
        line_number: 1,
        message: "Critical".to_string(),
        stack_trace: None,
        severity: IssueSeverity::Critical,
        is_flaky: false,
        suggested_fix: None,
    });

    findings.add_test_failure(TestFailure {
        test_name: "test_high".to_string(),
        file_path: std::path::PathBuf::from("test.rs"),
        line_number: 2,
        message: "High".to_string(),
        stack_trace: None,
        severity: IssueSeverity::High,
        is_flaky: false,
        suggested_fix: None,
    });

    findings.add_test_failure(TestFailure {
        test_name: "test_critical_2".to_string(),
        file_path: std::path::PathBuf::from("test.rs"),
        line_number: 3,
        message: "Critical 2".to_string(),
        stack_trace: None,
        severity: IssueSeverity::Critical,
        is_flaky: false,
        suggested_fix: None,
    });

    // Test getters
    let critical_indices = findings.get_critical_test_failure_indices();
    assert_eq!(critical_indices.len(), 2);
    assert!(critical_indices.contains(&0));
    assert!(critical_indices.contains(&2));

    let high_indices = findings.get_high_priority_test_failure_indices();
    assert_eq!(high_indices.len(), 1);
    assert!(high_indices.contains(&1));
}

#[test]
fn test_aggregated_findings_performance_issues() {
    let mut findings = AggregatedFindings::new();

    findings.add_performance_issue(PerformanceIssue {
        title: "Slow Query".to_string(),
        description: "Database query too slow".to_string(),
        severity: IssueSeverity::High,
        affected_component: "src/db.rs".to_string(),
        metric: "Query Time".to_string(),
        actual_value: 5000.0,
        threshold_value: 1000.0,
        suggested_fix: Some("Add index".to_string()),
    });

    findings.add_performance_issue(PerformanceIssue {
        title: "Memory Usage".to_string(),
        description: "High memory consumption".to_string(),
        severity: IssueSeverity::Medium,
        affected_component: "src/cache.rs".to_string(),
        metric: "Memory MB".to_string(),
        actual_value: 1024.0,
        threshold_value: 512.0,
        suggested_fix: None,
    });

    assert_eq!(findings.high_count, 1);
    assert_eq!(findings.medium_count, 1);
    assert_eq!(findings.performance_issues.len(), 2);
}

#[test]
fn test_aggregated_findings_security_issue_getters() {
    let mut findings = AggregatedFindings::new();

    findings.add_security_issue(SecurityIssue {
        title: "Critical Security".to_string(),
        description: "Critical".to_string(),
        cve_id: None,
        severity: IssueSeverity::Critical,
        affected_component: "test.rs".to_string(),
        suggested_fix: None,
        references: vec![],
    });

    findings.add_security_issue(SecurityIssue {
        title: "High Security".to_string(),
        description: "High".to_string(),
        cve_id: None,
        severity: IssueSeverity::High,
        affected_component: "test.rs".to_string(),
        suggested_fix: None,
        references: vec![],
    });

    let critical_indices = findings.get_critical_security_issue_indices();
    assert_eq!(critical_indices.len(), 1);

    let high_indices = findings.get_high_priority_security_issue_indices();
    assert_eq!(high_indices.len(), 1);
}

// ==================== Aggregate Findings Function Tests ====================

#[test]
fn test_aggregate_findings_function() {
    let coverage = Some(CoverageReport {
        total_lines: 1000,
        covered_lines: 750,
        coverage_percent: 75.0,
        modules: vec![],
        low_coverage_files: vec![],
        low_coverage_modules: vec![],
        meets_threshold: true,
        analysis_duration_secs: 5,
    });

    let test_failures = vec![TestFailure {
        test_name: "test_failure".to_string(),
        file_path: std::path::PathBuf::from("test.rs"),
        line_number: 10,
        message: "Test failed".to_string(),
        stack_trace: None,
        severity: IssueSeverity::High,
        is_flaky: false,
        suggested_fix: None,
    }];

    let security_issues = vec![SecurityIssue {
        title: "Security Issue".to_string(),
        description: "Critical security".to_string(),
        cve_id: None,
        severity: IssueSeverity::Critical,
        affected_component: "src/security.rs".to_string(),
        suggested_fix: None,
        references: vec![],
    }];

    let performance_issues = vec![PerformanceIssue {
        title: "Performance Issue".to_string(),
        description: "Slow performance".to_string(),
        severity: IssueSeverity::Medium,
        affected_component: "src/perf.rs".to_string(),
        metric: "Time".to_string(),
        actual_value: 1000.0,
        threshold_value: 100.0,
        suggested_fix: None,
    }];

    let findings = aggregate_findings(coverage, test_failures, security_issues, performance_issues);

    assert!(findings.coverage.is_some());
    assert_eq!(findings.test_failures.len(), 1);
    assert_eq!(findings.security_issues.len(), 1);
    assert_eq!(findings.performance_issues.len(), 1);
    assert_eq!(findings.critical_count, 1);
    assert_eq!(findings.high_count, 1);
    assert_eq!(findings.medium_count, 1);
}

// ==================== Fix Cycle Trigger Extended Tests ====================

#[test]
fn test_should_trigger_fix_cycle_with_performance_issues() {
    let mut findings = AggregatedFindings::new();

    // Performance issues alone should not trigger fix cycle
    findings.add_performance_issue(PerformanceIssue {
        title: "Slow".to_string(),
        description: "Slow performance".to_string(),
        severity: IssueSeverity::Medium,
        affected_component: "src/perf.rs".to_string(),
        metric: "Time".to_string(),
        actual_value: 1000.0,
        threshold_value: 100.0,
        suggested_fix: None,
    });

    assert!(!should_trigger_fix_cycle(&findings));
}

#[test]
fn test_should_trigger_fix_cycle_mixed_issues() {
    let mut findings = AggregatedFindings::new();

    // Add medium and low priority issues
    findings.add_test_failure(TestFailure {
        test_name: "test_medium".to_string(),
        file_path: std::path::PathBuf::from("test.rs"),
        line_number: 10,
        message: "Medium".to_string(),
        stack_trace: None,
        severity: IssueSeverity::Medium,
        is_flaky: false,
        suggested_fix: None,
    });

    findings.add_performance_issue(PerformanceIssue {
        title: "Performance".to_string(),
        description: "Performance issue".to_string(),
        severity: IssueSeverity::Low,
        affected_component: "src/perf.rs".to_string(),
        metric: "Time".to_string(),
        actual_value: 500.0,
        threshold_value: 100.0,
        suggested_fix: None,
    });

    assert!(!should_trigger_fix_cycle(&findings));
}

#[test]
fn test_should_trigger_fix_cycle_edge_cases() {
    // Test with exactly 50% coverage (should not trigger)
    let mut findings = AggregatedFindings::new();
    findings.coverage = Some(CoverageReport {
        total_lines: 100,
        covered_lines: 50,
        coverage_percent: 50.0,
        modules: vec![],
        low_coverage_files: vec![],
        low_coverage_modules: vec![],
        meets_threshold: false,
        analysis_duration_secs: 1,
    });
    assert!(!should_trigger_fix_cycle(&findings));

    // Test with 49.99% coverage (should trigger)
    findings.coverage = Some(CoverageReport {
        total_lines: 100,
        covered_lines: 49,
        coverage_percent: 49.0,
        modules: vec![],
        low_coverage_files: vec![],
        low_coverage_modules: vec![],
        meets_threshold: false,
        analysis_duration_secs: 1,
    });
    assert!(should_trigger_fix_cycle(&findings));
}

// ==================== Fix Cycle Config Extended Tests ====================

#[test]
fn test_fix_cycle_config_cloning() {
    let config = FixCycleConfig::expert_mode();
    let cloned = config.clone();

    assert_eq!(config.enabled, cloned.enabled);
    assert_eq!(config.max_fix_attempts, cloned.max_fix_attempts);
    assert_eq!(config.fix_timeout, cloned.fix_timeout);
    assert_eq!(config.require_confirmation, cloned.require_confirmation);
    assert_eq!(config.auto_fix_threshold, cloned.auto_fix_threshold);
}

#[test]
fn test_fix_cycle_config_custom() {
    let config = FixCycleConfig {
        enabled: true,
        max_fix_attempts: 10,
        fix_timeout: 1800,
        require_confirmation: true,
        auto_fix_threshold: IssueSeverity::Low,
        work_dir: std::path::PathBuf::from("/custom"),
        fix_model: "custom-model".to_string(),
    };

    assert_eq!(config.max_fix_attempts, 10);
    assert_eq!(config.fix_timeout, 1800);
    assert!(config.require_confirmation);
    assert_eq!(config.auto_fix_threshold, IssueSeverity::Low);
    assert!(config.should_auto_fix(IssueSeverity::Low));
}

// ==================== Test Failure Extended Tests ====================

#[test]
fn test_test_failure_with_stack_trace() {
    let failure = TestFailure {
        test_name: "test_panic".to_string(),
        file_path: std::path::PathBuf::from("src/lib.rs"),
        line_number: 42,
        message: "assertion failed: `false`".to_string(),
        stack_trace: Some(
            "stack trace:
  at src/lib.rs:42
  at src/test.rs:10"
                .to_string(),
        ),
        severity: IssueSeverity::Critical,
        is_flaky: false,
        suggested_fix: Some("Fix the assertion".to_string()),
    };

    assert!(failure.stack_trace.is_some());
    assert!(failure.stack_trace.as_ref().unwrap().contains("stack trace"));
    assert_eq!(failure.severity(), IssueSeverity::Critical);
}

#[test]
fn test_test_failure_flaky() {
    let failure = TestFailure {
        test_name: "test_flaky".to_string(),
        file_path: std::path::PathBuf::from("src/lib.rs"),
        line_number: 10,
        message: "Intermittent failure".to_string(),
        stack_trace: None,
        severity: IssueSeverity::Medium,
        is_flaky: true,
        suggested_fix: None,
    };

    assert!(failure.is_flaky);
    assert_eq!(failure.severity, IssueSeverity::Medium);
}

// ==================== Security Issue Extended Tests ====================

#[test]
fn test_security_issue_with_cve() {
    let issue = SecurityIssue {
        title: "Buffer Overflow".to_string(),
        description: "Critical buffer overflow".to_string(),
        cve_id: Some("CVE-2024-1234".to_string()),
        severity: IssueSeverity::Critical,
        affected_component: "src/buffer.rs".to_string(),
        suggested_fix: Some("Add bounds checking".to_string()),
        references: vec![
            "https://cve.mitre.org/cgi-bin/cvename.cgi?name=CVE-2024-1234".to_string(),
        ],
    };

    assert!(issue.cve_id.is_some());
    assert_eq!(issue.cve_id.unwrap(), "CVE-2024-1234");
    assert_eq!(issue.references.len(), 1);
}

#[test]
fn test_security_issue_multiple_references() {
    let issue = SecurityIssue {
        title: "XSS Vulnerability".to_string(),
        description: "Cross-site scripting".to_string(),
        cve_id: None,
        severity: IssueSeverity::High,
        affected_component: "src/render.rs".to_string(),
        suggested_fix: Some("Sanitize input".to_string()),
        references: vec![
            "https://owasp.org".to_string(),
            "https://cwe.mitre.org/data/definitions/79.html".to_string(),
        ],
    };

    assert_eq!(issue.references.len(), 2);
}

// ==================== Edge Cases and Boundary Tests ====================

#[test]
fn test_empty_strings_in_issues() {
    let failure = TestFailure {
        test_name: "".to_string(),
        file_path: std::path::PathBuf::from(""),
        line_number: 0,
        message: "".to_string(),
        stack_trace: None,
        severity: IssueSeverity::Low,
        is_flaky: false,
        suggested_fix: None,
    };

    assert_eq!(failure.test_name, "");
    assert_eq!(failure.message, "");
}

#[test]
fn test_large_numbers_in_findings() {
    let mut findings = AggregatedFindings::new();

    // Add many issues
    for i in 0..100 {
        findings.add_test_failure(TestFailure {
            test_name: format!("test_{}", i),
            file_path: std::path::PathBuf::from("test.rs"),
            line_number: i,
            message: "Failure".to_string(),
            stack_trace: None,
            severity: if i % 4 == 0 {
                IssueSeverity::Critical
            } else if i % 4 == 1 {
                IssueSeverity::High
            } else if i % 4 == 2 {
                IssueSeverity::Medium
            } else {
                IssueSeverity::Low
            },
            is_flaky: false,
            suggested_fix: None,
        });
    }

    assert_eq!(findings.test_failures.len(), 100);
    assert!(findings.exceeds_threshold(50));
    assert!(!findings.exceeds_threshold(200));
}

#[test]
fn test_coverage_report_with_modules() {
    let report = CoverageReport {
        total_lines: 1000,
        covered_lines: 750,
        coverage_percent: 75.0,
        modules: vec![
            ModuleCoverage {
                name: "module1".to_string(),
                files: vec![],
                total_lines: 500,
                covered_lines: 400,
                coverage_percent: 80.0,
                meets_threshold: true,
            },
            ModuleCoverage {
                name: "module2".to_string(),
                files: vec![],
                total_lines: 500,
                covered_lines: 350,
                coverage_percent: 70.0,
                meets_threshold: true,
            },
        ],
        low_coverage_files: vec![],
        low_coverage_modules: vec![],
        meets_threshold: true,
        analysis_duration_secs: 5,
    };

    assert_eq!(report.modules.len(), 2);
    assert_eq!(report.modules[0].coverage_percent, 80.0);
    assert_eq!(report.modules[1].coverage_percent, 70.0);
}

#[test]
fn test_coverage_report_with_low_coverage_modules() {
    let report = CoverageReport {
        total_lines: 1000,
        covered_lines: 600,
        coverage_percent: 60.0,
        modules: vec![
            ModuleCoverage {
                name: "good_module".to_string(),
                files: vec![],
                total_lines: 500,
                covered_lines: 450,
                coverage_percent: 90.0,
                meets_threshold: true,
            },
            ModuleCoverage {
                name: "bad_module".to_string(),
                files: vec![],
                total_lines: 500,
                covered_lines: 150,
                coverage_percent: 30.0,
                meets_threshold: false,
            },
        ],
        low_coverage_files: vec![],
        low_coverage_modules: vec![ModuleCoverage {
            name: "bad_module".to_string(),
            files: vec![],
            total_lines: 500,
            covered_lines: 150,
            coverage_percent: 30.0,
            meets_threshold: false,
        }],
        meets_threshold: false,
        analysis_duration_secs: 5,
    };

    assert_eq!(report.low_coverage_modules.len(), 1);
    assert_eq!(report.low_coverage_modules[0].name, "bad_module");
}
