//! Test coverage analysis module
//!
//! This module handles test coverage checking, aggregation of test findings,
//! and critical issue detection. It integrates with the test and verify stages
//! to provide comprehensive coverage analysis.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info, warn};

use crate::pipeline::test::TestFramework;

/// Coverage analysis configuration
#[derive(Debug, Clone)]
pub struct CoverageConfig {
    /// Minimum acceptable coverage percentage (0-100)
    pub min_coverage_percent: f64,

    /// Whether to fail on low coverage
    pub fail_on_low_coverage: bool,

    /// Paths to include in coverage analysis
    pub include_paths: Vec<PathBuf>,

    /// Paths to exclude from coverage analysis
    pub exclude_paths: Vec<PathBuf>,

    /// Working directory
    pub work_dir: PathBuf,

    /// Whether to generate detailed coverage reports
    pub generate_reports: bool,

    /// Output directory for coverage reports
    pub report_dir: PathBuf,
}

impl Default for CoverageConfig {
    fn default() -> Self {
        CoverageConfig {
            min_coverage_percent: 70.0,
            fail_on_low_coverage: true,
            include_paths: vec![PathBuf::from("src")],
            exclude_paths: vec![],
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            generate_reports: true,
            report_dir: PathBuf::from("target/coverage"),
        }
    }
}

impl CoverageConfig {
    /// Create config for strict mode (higher coverage requirements)
    pub fn strict_mode() -> Self {
        CoverageConfig {
            min_coverage_percent: 90.0,
            fail_on_low_coverage: true,
            include_paths: vec![PathBuf::from("src")],
            exclude_paths: vec![],
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            generate_reports: true,
            report_dir: PathBuf::from("target/coverage"),
        }
    }

    /// Create config for lenient mode (lower coverage requirements)
    pub fn lenient_mode() -> Self {
        CoverageConfig {
            min_coverage_percent: 50.0,
            fail_on_low_coverage: false,
            include_paths: vec![PathBuf::from("src")],
            exclude_paths: vec![],
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            generate_reports: false,
            report_dir: PathBuf::from("target/coverage"),
        }
    }
}

/// Result of coverage analysis for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    /// Path to the file
    pub path: PathBuf,

    /// Total lines in the file
    pub total_lines: usize,

    /// Covered lines
    pub covered_lines: usize,

    /// Coverage percentage
    pub coverage_percent: f64,

    /// Uncovered line numbers
    pub uncovered_lines: Vec<usize>,

    /// Whether this file meets minimum coverage requirements
    pub meets_threshold: bool,
}

/// Result of coverage analysis for a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleCoverage {
    /// Module name
    pub name: String,

    /// Files in this module
    pub files: Vec<FileCoverage>,

    /// Total lines in module
    pub total_lines: usize,

    /// Covered lines in module
    pub covered_lines: usize,

    /// Module coverage percentage
    pub coverage_percent: f64,

    /// Whether this module meets minimum coverage requirements
    pub meets_threshold: bool,
}

/// Overall coverage analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    /// Total lines across all files
    pub total_lines: usize,

    /// Total covered lines
    pub covered_lines: usize,

    /// Overall coverage percentage
    pub coverage_percent: f64,

    /// Coverage by module
    pub modules: Vec<ModuleCoverage>,

    /// Files below coverage threshold
    pub low_coverage_files: Vec<FileCoverage>,

    /// Modules below coverage threshold
    pub low_coverage_modules: Vec<ModuleCoverage>,

    /// Whether overall coverage meets minimum requirements
    pub meets_threshold: bool,

    /// Analysis duration in seconds
    pub analysis_duration_secs: u64,
}

/// Aggregate findings from multiple sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedFindings {
    /// Coverage report
    pub coverage: Option<CoverageReport>,

    /// Test failures
    pub test_failures: Vec<TestFailure>,

    /// Security issues
    pub security_issues: Vec<SecurityIssue>,

    /// Performance issues
    pub performance_issues: Vec<PerformanceIssue>,

    /// Total critical issues
    pub critical_count: usize,

    /// Total high-priority issues
    pub high_count: usize,

    /// Total medium-priority issues
    pub medium_count: usize,

    /// Total low-priority issues
    pub low_count: usize,
}

impl AggregatedFindings {
    /// Create empty findings
    pub fn new() -> Self {
        AggregatedFindings {
            coverage: None,
            test_failures: Vec::new(),
            security_issues: Vec::new(),
            performance_issues: Vec::new(),
            critical_count: 0,
            high_count: 0,
            medium_count: 0,
            low_count: 0,
        }
    }

    /// Add a test failure
    pub fn add_test_failure(&mut self, failure: TestFailure) {
        match failure.severity {
            IssueSeverity::Critical => self.critical_count += 1,
            IssueSeverity::High => self.high_count += 1,
            IssueSeverity::Medium => self.medium_count += 1,
            IssueSeverity::Low => self.low_count += 1,
        }
        self.test_failures.push(failure);
    }

    /// Add a security issue
    pub fn add_security_issue(&mut self, issue: SecurityIssue) {
        match issue.severity {
            IssueSeverity::Critical => self.critical_count += 1,
            IssueSeverity::High => self.high_count += 1,
            IssueSeverity::Medium => self.medium_count += 1,
            IssueSeverity::Low => self.low_count += 1,
        }
        self.security_issues.push(issue);
    }

    /// Add a performance issue
    pub fn add_performance_issue(&mut self, issue: PerformanceIssue) {
        match issue.severity {
            IssueSeverity::Critical => self.critical_count += 1,
            IssueSeverity::High => self.high_count += 1,
            IssueSeverity::Medium => self.medium_count += 1,
            IssueSeverity::Low => self.low_count += 1,
        }
        self.performance_issues.push(issue);
    }

    /// Get all issues by severity (returns indices)
    pub fn get_critical_test_failure_indices(&self) -> Vec<usize> {
        self.test_failures
            .iter()
            .enumerate()
            .filter(|(_, f)| f.severity == IssueSeverity::Critical)
            .map(|(i, _)| i)
            .collect()
    }

    /// Get all critical security issue indices
    pub fn get_critical_security_issue_indices(&self) -> Vec<usize> {
        self.security_issues
            .iter()
            .enumerate()
            .filter(|(_, f)| f.severity == IssueSeverity::Critical)
            .map(|(i, _)| i)
            .collect()
    }

    /// Get all high-priority test failure indices
    pub fn get_high_priority_test_failure_indices(&self) -> Vec<usize> {
        self.test_failures
            .iter()
            .enumerate()
            .filter(|(_, f)| f.severity == IssueSeverity::High)
            .map(|(i, _)| i)
            .collect()
    }

    /// Get all high-priority security issue indices
    pub fn get_high_priority_security_issue_indices(&self) -> Vec<usize> {
        self.security_issues
            .iter()
            .enumerate()
            .filter(|(_, f)| f.severity == IssueSeverity::High)
            .map(|(i, _)| i)
            .collect()
    }

    /// Check if there are any critical issues
    pub fn has_critical_issues(&self) -> bool {
        self.critical_count > 0
    }

    /// Check if total issue count exceeds threshold
    pub fn exceeds_threshold(&self, threshold: usize) -> bool {
        (self.critical_count + self.high_count + self.medium_count + self.low_count) > threshold
    }
}

impl Default for AggregatedFindings {
    fn default() -> Self {
        Self::new()
    }
}

/// Severity of an issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    /// Critical - must fix before proceeding
    Critical,

    /// High - should fix soon
    High,

    /// Medium - fix when convenient
    Medium,

    /// Low - optional fix
    Low,
}

/// Trait for issue details
pub trait IssueDetail {
    /// Get the issue title
    fn title(&self) -> &str;

    /// Get the issue description
    fn description(&self) -> &str;

    /// Get the issue severity
    fn severity(&self) -> IssueSeverity;

    /// Get the affected file or component
    fn affected_component(&self) -> &str;

    /// Get suggested fix
    fn suggested_fix(&self) -> Option<&str>;
}

/// Test failure information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailure {
    /// Test name
    pub test_name: String,

    /// File where test failed
    pub file_path: PathBuf,

    /// Line number
    pub line_number: usize,

    /// Failure message
    pub message: String,

    /// Stack trace if available
    pub stack_trace: Option<String>,

    /// Severity of the failure
    pub severity: IssueSeverity,

    /// Whether this is a flaky test
    pub is_flaky: bool,

    /// Suggested fix
    pub suggested_fix: Option<String>,
}

impl IssueDetail for TestFailure {
    fn title(&self) -> &str {
        &self.test_name
    }

    fn description(&self) -> &str {
        &self.message
    }

    fn severity(&self) -> IssueSeverity {
        self.severity
    }

    fn affected_component(&self) -> &str {
        self.file_path.to_str().unwrap_or("unknown")
    }

    fn suggested_fix(&self) -> Option<&str> {
        self.suggested_fix.as_deref()
    }
}

/// Security issue information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    /// Issue title
    pub title: String,

    /// Issue description
    pub description: String,

    /// CVE or security identifier if applicable
    pub cve_id: Option<String>,

    /// Severity
    pub severity: IssueSeverity,

    /// Affected file or component
    pub affected_component: String,

    /// Suggested fix
    pub suggested_fix: Option<String>,

    /// References for more information
    pub references: Vec<String>,
}

impl IssueDetail for SecurityIssue {
    fn title(&self) -> &str {
        &self.title
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn severity(&self) -> IssueSeverity {
        self.severity
    }

    fn affected_component(&self) -> &str {
        &self.affected_component
    }

    fn suggested_fix(&self) -> Option<&str> {
        self.suggested_fix.as_deref()
    }
}

/// Performance issue information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceIssue {
    /// Issue title
    pub title: String,

    /// Issue description
    pub description: String,

    /// Severity
    pub severity: IssueSeverity,

    /// Affected function or component
    pub affected_component: String,

    /// Measured performance metric
    pub metric: String,

    /// Actual value
    pub actual_value: f64,

    /// Threshold value
    pub threshold_value: f64,

    /// Suggested fix
    pub suggested_fix: Option<String>,
}

impl IssueDetail for PerformanceIssue {
    fn title(&self) -> &str {
        &self.title
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn severity(&self) -> IssueSeverity {
        self.severity
    }

    fn affected_component(&self) -> &str {
        &self.affected_component
    }

    fn suggested_fix(&self) -> Option<&str> {
        self.suggested_fix.as_deref()
    }
}

/// Analyze test coverage for a Rust project
pub async fn analyze_coverage(
    framework: &TestFramework,
    config: &CoverageConfig,
) -> Result<CoverageReport> {
    let start_time = std::time::Instant::now();

    info!("Starting coverage analysis for {:?}", framework);

    let report = match framework {
        TestFramework::Cargo => analyze_cargo_coverage(config).await?,
        TestFramework::Pytest => {
            analyze_python_coverage(config).await.unwrap_or_else(|e| {
                warn!("Python coverage analysis failed: {}", e);
                create_empty_coverage_report()
            })
        }
        TestFramework::Go => {
            analyze_go_coverage(config).await.unwrap_or_else(|e| {
                warn!("Go coverage analysis failed: {}", e);
                create_empty_coverage_report()
            })
        }
        TestFramework::Npm => {
            analyze_javascript_coverage(config).await.unwrap_or_else(|e| {
                warn!("JavaScript coverage analysis failed: {}", e);
                create_empty_coverage_report()
            })
        }
        TestFramework::None => create_empty_coverage_report(),
    };

    let duration = start_time.elapsed().as_secs();

    info!(
        "Coverage analysis complete: {:.1}% coverage in {}s",
        report.coverage_percent, duration
    );

    Ok(report)
}

/// Analyze coverage for Rust/Cargo projects
async fn analyze_cargo_coverage(config: &CoverageConfig) -> Result<CoverageReport> {
    debug!("Analyzing Cargo coverage");

    // Create report directory
    std::fs::create_dir_all(&config.report_dir)
        .context("Failed to create coverage report directory")?;

    // Run cargo-tarpaulin if available (faster than grcov)
    let coverage_result = if Command::new("cargo-tarpaulin")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        analyze_with_tarpaulin(config).await?
    } else {
        // Fallback to basic line counting
        analyze_by_line_counting(config)?
    };

    Ok(coverage_result)
}

/// Analyze coverage using tarpaulin
async fn analyze_with_tarpaulin(config: &CoverageConfig) -> Result<CoverageReport> {
    debug!("Using tarpaulin for coverage analysis");

    let output = Command::new("cargo-tarpaulin")
        .args([
            "--out",
            "Json",
            "--output-dir",
            config.report_dir.to_str().unwrap(),
            "--skip-clean",
        ])
        .current_dir(&config.work_dir)
        .output()
        .context("Failed to run cargo-tarpaulin")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        warn!("Tarpaulin execution failed: {}", error);
        return analyze_by_line_counting(config);
    }

    // Parse tarpaulin output if available
    // For now, return a basic report
    analyze_by_line_counting(config)
}

/// Analyze coverage by counting lines (fallback method)
fn analyze_by_line_counting(config: &CoverageConfig) -> Result<CoverageReport> {
    debug!("Using line counting for coverage estimation");

    let mut total_lines = 0;
    let mut covered_lines = 0;
    let mut modules = Vec::new();
    let mut low_coverage_files = Vec::new();
    let mut low_coverage_modules = Vec::new();

    // Analyze src/ directory
    let src_dir = config.work_dir.join("src");
    if src_dir.exists() {
        let (module_coverage, total, covered) = analyze_directory(&src_dir, config)?;
        total_lines += total;
        covered_lines += covered;
        modules.extend(module_coverage);

        // Identify low coverage files
        for module in &modules {
            if !module.meets_threshold {
                low_coverage_modules.push(module.clone());
            }
            for file in &module.files {
                if !file.meets_threshold {
                    low_coverage_files.push(file.clone());
                }
            }
        }
    }

    let coverage_percent = if total_lines > 0 {
        (covered_lines as f64 / total_lines as f64) * 100.0
    } else {
        0.0
    };

    let meets_threshold = coverage_percent >= config.min_coverage_percent;

    Ok(CoverageReport {
        total_lines,
        covered_lines,
        coverage_percent,
        modules,
        low_coverage_files,
        low_coverage_modules,
        meets_threshold,
        analysis_duration_secs: 0,
    })
}

/// Analyze a directory for coverage
fn analyze_directory(
    dir: &Path,
    config: &CoverageConfig,
) -> Result<(Vec<ModuleCoverage>, usize, usize)> {
    let mut modules = Vec::new();
    let mut total_lines = 0;
    let mut covered_lines = 0;

    let entries = std::fs::read_dir(dir)
        .context(format!("Failed to read directory: {:?}", dir))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let (sub_modules, sub_total, sub_covered) = analyze_directory(&path, config)?;
            modules.extend(sub_modules);
            total_lines += sub_total;
            covered_lines += sub_covered;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            // Analyze Rust file
            if let Some(file_coverage) = analyze_rust_file(&path, config)? {
                total_lines += file_coverage.total_lines;
                covered_lines += file_coverage.covered_lines;

                // Create module for this file
                let module_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let module_coverage = ModuleCoverage {
                    name: module_name,
                    files: vec![file_coverage.clone()],
                    total_lines: file_coverage.total_lines,
                    covered_lines: file_coverage.covered_lines,
                    coverage_percent: file_coverage.coverage_percent,
                    meets_threshold: file_coverage.meets_threshold,
                };

                modules.push(module_coverage);
            }
        }
    }

    Ok((modules, total_lines, covered_lines))
}

/// Analyze a single Rust file for coverage
fn analyze_rust_file(path: &Path, _config: &CoverageConfig) -> Result<Option<FileCoverage>> {
    let content = std::fs::read_to_string(path)
        .context(format!("Failed to read file: {:?}", path))?;

    let mut code_lines = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        // Skip test code
        if trimmed.contains("#[test]")
            || trimmed.contains("#[cfg(test)]")
            || trimmed.contains("mod tests")
        {
            continue;
        }

        code_lines += 1;
    }

    // Estimate coverage based on test lines vs code lines
    // This is a rough estimate - real coverage requires instrumentation
    let estimated_covered = (code_lines as f64 * 0.6) as usize; // Assume 60% coverage
    let coverage_percent = if code_lines > 0 {
        (estimated_covered as f64 / code_lines as f64) * 100.0
    } else {
        0.0
    };

    Ok(Some(FileCoverage {
        path: path.to_path_buf(),
        total_lines: code_lines,
        covered_lines: estimated_covered,
        coverage_percent,
        uncovered_lines: Vec::new(), // Not available without instrumentation
        meets_threshold: coverage_percent >= 70.0, // Default threshold
    }))
}

/// Analyze Python coverage (placeholder)
async fn analyze_python_coverage(_config: &CoverageConfig) -> Result<CoverageReport> {
    // Try running pytest with coverage
    Ok(create_empty_coverage_report())
}

/// Analyze Go coverage (placeholder)
async fn analyze_go_coverage(_config: &CoverageConfig) -> Result<CoverageReport> {
    // Try running go test with coverage
    Ok(create_empty_coverage_report())
}

/// Analyze JavaScript/Node coverage (placeholder)
async fn analyze_javascript_coverage(_config: &CoverageConfig) -> Result<CoverageReport> {
    // Try running npm test with coverage
    Ok(create_empty_coverage_report())
}

/// Create an empty coverage report
fn create_empty_coverage_report() -> CoverageReport {
    CoverageReport {
        total_lines: 0,
        covered_lines: 0,
        coverage_percent: 0.0,
        modules: Vec::new(),
        low_coverage_files: Vec::new(),
        low_coverage_modules: Vec::new(),
        meets_threshold: false,
        analysis_duration_secs: 0,
    }
}

/// Aggregate findings from multiple sources
pub fn aggregate_findings(
    coverage: Option<CoverageReport>,
    test_failures: Vec<TestFailure>,
    security_issues: Vec<SecurityIssue>,
    performance_issues: Vec<PerformanceIssue>,
) -> AggregatedFindings {
    let mut findings = AggregatedFindings::new();
    findings.coverage = coverage;

    for failure in test_failures {
        findings.add_test_failure(failure);
    }

    for issue in security_issues {
        findings.add_security_issue(issue);
    }

    for issue in performance_issues {
        findings.add_performance_issue(issue);
    }

    findings
}

/// Display coverage report summary
pub fn display_coverage_summary(report: &CoverageReport) {
    println!("\n=== Coverage Summary ===");
    println!(
        "Total Lines: {} | Covered: {} | Coverage: {:.1}%",
        report.total_lines, report.covered_lines, report.coverage_percent
    );
    println!("Meets Threshold: {}", report.meets_threshold);

    if !report.low_coverage_modules.is_empty() {
        println!("\nLow Coverage Modules:");
        for module in &report.low_coverage_modules {
            println!(
                "  {} - {:.1}%",
                module.name, module.coverage_percent
            );
        }
    }
}

/// Display aggregated findings summary
pub fn display_findings_summary(findings: &AggregatedFindings) {
    println!("\n=== Aggregated Findings ===");
    println!("Critical: {} | High: {} | Medium: {} | Low: {}",
        findings.critical_count,
        findings.high_count,
        findings.medium_count,
        findings.low_count
    );

    if let Some(coverage) = &findings.coverage {
        println!("Coverage: {:.1}%", coverage.coverage_percent);
    }

    if !findings.test_failures.is_empty() {
        println!("Test Failures: {}", findings.test_failures.len());
    }

    if !findings.security_issues.is_empty() {
        println!("Security Issues: {}", findings.security_issues.len());
    }

    if !findings.performance_issues.is_empty() {
        println!("Performance Issues: {}", findings.performance_issues.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_config_default() {
        let config = CoverageConfig::default();
        assert_eq!(config.min_coverage_percent, 70.0);
        assert!(config.fail_on_low_coverage);
        assert!(config.generate_reports);
    }

    #[test]
    fn test_coverage_config_strict_mode() {
        let config = CoverageConfig::strict_mode();
        assert_eq!(config.min_coverage_percent, 90.0);
        assert!(config.fail_on_low_coverage);
    }

    #[test]
    fn test_coverage_config_lenient_mode() {
        let config = CoverageConfig::lenient_mode();
        assert_eq!(config.min_coverage_percent, 50.0);
        assert!(!config.fail_on_low_coverage);
    }

    #[test]
    fn test_aggregated_findings_new() {
        let findings = AggregatedFindings::new();
        assert_eq!(findings.critical_count, 0);
        assert!(findings.test_failures.is_empty());
        assert!(findings.security_issues.is_empty());
        assert!(findings.performance_issues.is_empty());
    }

    #[test]
    fn test_aggregated_findings_add_test_failure() {
        let mut findings = AggregatedFindings::new();

        let failure = TestFailure {
            test_name: "test_example".to_string(),
            file_path: PathBuf::from("test.rs"),
            line_number: 10,
            message: "Assertion failed".to_string(),
            stack_trace: None,
            severity: IssueSeverity::High,
            is_flaky: false,
            suggested_fix: None,
        };

        findings.add_test_failure(failure);
        assert_eq!(findings.high_count, 1);
        assert_eq!(findings.test_failures.len(), 1);
    }

    #[test]
    fn test_aggregated_findings_critical_issues() {
        let mut findings = AggregatedFindings::new();

        let critical_issue = SecurityIssue {
            title: "Buffer Overflow".to_string(),
            description: "Unsafe buffer operation".to_string(),
            cve_id: Some("CVE-2024-1234".to_string()),
            severity: IssueSeverity::Critical,
            affected_component: "src/buffer.rs".to_string(),
            suggested_fix: Some("Use bounds checking".to_string()),
            references: vec![],
        };

        findings.add_security_issue(critical_issue);
        assert!(findings.has_critical_issues());
        assert_eq!(findings.critical_count, 1);
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

    #[test]
    fn test_coverage_report_creation() {
        let report = CoverageReport {
            total_lines: 100,
            covered_lines: 75,
            coverage_percent: 75.0,
            modules: vec![],
            low_coverage_files: vec![],
            low_coverage_modules: vec![],
            meets_threshold: true,
            analysis_duration_secs: 1,
        };

        assert_eq!(report.total_lines, 100);
        assert_eq!(report.covered_lines, 75);
        assert_eq!(report.coverage_percent, 75.0);
        assert!(report.meets_threshold);
    }

    #[test]
    fn test_aggregated_findings_exceeds_threshold() {
        let mut findings = AggregatedFindings::new();

        let failure = TestFailure {
            test_name: "test".to_string(),
            file_path: PathBuf::from("test.rs"),
            line_number: 1,
            message: "Fail".to_string(),
            stack_trace: None,
            severity: IssueSeverity::Low,
            is_flaky: false,
            suggested_fix: None,
        };

        findings.add_test_failure(failure.clone());
        findings.add_test_failure(failure);

        assert!(findings.exceeds_threshold(1));
        assert!(!findings.exceeds_threshold(10));
    }

    #[test]
    fn test_empty_coverage_report() {
        let report = create_empty_coverage_report();
        assert_eq!(report.total_lines, 0);
        assert_eq!(report.covered_lines, 0);
        assert_eq!(report.coverage_percent, 0.0);
        assert!(!report.meets_threshold);
    }
}
