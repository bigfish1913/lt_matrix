//! Review stage of the pipeline
//!
//! This module handles code review for expert mode execution.
//! It performs comprehensive code quality checks including:
//! - Code quality assessment
//! - Security vulnerability detection
//! - Performance analysis
//! - Best practices verification
//!
//! The review stage is only active in expert mode and uses a separate
//! review agent to provide thorough code analysis.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

use crate::agent::backend::{AgentBackend, ExecutionConfig};
use crate::agent::claude::ClaudeAgent;
use crate::models::{ModeConfig, Task, TaskStatus};

/// Configuration for the review stage
#[derive(Debug, Clone)]
pub struct ReviewConfig {
    /// Mode configuration
    pub mode_config: ModeConfig,

    /// Whether review is enabled (only in expert mode)
    pub enabled: bool,

    /// Model to use for review
    pub review_model: String,

    /// Maximum number of issues to report per category
    pub max_issues_per_category: usize,

    /// Severity threshold for reporting issues
    pub severity_threshold: IssueSeverity,

    /// Whether to check for security issues
    pub check_security: bool,

    /// Whether to check for performance issues
    pub check_performance: bool,

    /// Whether to check for code quality issues
    pub check_quality: bool,

    /// Whether to check for best practices violations
    pub check_best_practices: bool,

    /// Timeout for review (seconds)
    pub timeout: u64,

    /// Working directory
    pub work_dir: PathBuf,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        ReviewConfig {
            mode_config: ModeConfig::default(),
            enabled: false, // Disabled by default, only for expert mode
            review_model: "claude-opus-4-6".to_string(),
            max_issues_per_category: 10,
            severity_threshold: IssueSeverity::Medium,
            check_security: true,
            check_performance: true,
            check_quality: true,
            check_best_practices: true,
            timeout: 600,
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

impl ReviewConfig {
    /// Create config for expert mode
    pub fn expert_mode() -> Self {
        ReviewConfig {
            mode_config: ModeConfig::expert_mode(),
            enabled: true,
            review_model: "claude-opus-4-6".to_string(),
            max_issues_per_category: 15,
            severity_threshold: IssueSeverity::Low, // Report all issues in expert mode
            check_security: true,
            check_performance: true,
            check_quality: true,
            check_best_practices: true,
            timeout: 900, // 15 minutes for thorough review
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create config for expert mode with code review
    pub fn expert_with_review() -> Self {
        ReviewConfig {
            mode_config: ModeConfig::expert_mode(),
            enabled: true,
            review_model: "claude-opus-4-6".to_string(),
            max_issues_per_category: 20,
            severity_threshold: IssueSeverity::Info, // Report everything
            check_security: true,
            check_performance: true,
            check_quality: true,
            check_best_practices: true,
            timeout: 10800, // 3 hours for very thorough review
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Returns true if review should run
    pub fn should_run(&self) -> bool {
        self.enabled && self.mode_config.verify
    }

    /// Returns true if this is expert mode
    pub fn is_expert_mode(&self) -> bool {
        self.enabled
    }
}

/// Severity levels for code review issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    /// Informational - minor suggestions
    Info,

    /// Low - minor issues that should be addressed
    Low,

    /// Medium - moderate issues that should be fixed
    Medium,

    /// High - serious issues that must be fixed
    High,

    /// Critical - security vulnerabilities or blocking issues
    Critical,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueSeverity::Info => write!(f, "info"),
            IssueSeverity::Low => write!(f, "low"),
            IssueSeverity::Medium => write!(f, "medium"),
            IssueSeverity::High => write!(f, "high"),
            IssueSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// Categories of code review issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCategory {
    /// Security vulnerabilities
    Security,

    /// Performance problems
    Performance,

    /// Code quality issues
    Quality,

    /// Best practices violations
    BestPractices,

    /// Documentation issues
    Documentation,

    /// Testing issues
    Testing,

    /// Error handling
    ErrorHandling,
}

impl std::fmt::Display for IssueCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueCategory::Security => write!(f, "security"),
            IssueCategory::Performance => write!(f, "performance"),
            IssueCategory::Quality => write!(f, "quality"),
            IssueCategory::BestPractices => write!(f, "best_practices"),
            IssueCategory::Documentation => write!(f, "documentation"),
            IssueCategory::Testing => write!(f, "testing"),
            IssueCategory::ErrorHandling => write!(f, "error_handling"),
        }
    }
}

/// A single code review issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIssue {
    /// Category of the issue
    pub category: IssueCategory,

    /// Severity level
    pub severity: IssueSeverity,

    /// File where the issue was found (if applicable)
    pub file: Option<String>,

    /// Line number (if applicable)
    pub line: Option<usize>,

    /// Issue title
    pub title: String,

    /// Detailed description
    pub description: String,

    /// Suggested fix or improvement
    pub suggestion: Option<String>,

    /// Code snippet showing the issue (if applicable)
    pub code_snippet: Option<String>,

    /// Whether this issue blocks the task from being considered complete
    pub blocking: bool,
}

/// Result of reviewing a single task
#[derive(Debug, Clone)]
pub struct ReviewResult {
    /// The task that was reviewed
    pub task: Task,

    /// Overall assessment: pass, warning, or fail
    pub assessment: ReviewAssessment,

    /// Issues found during review
    pub issues: Vec<CodeIssue>,

    /// Summary of the review
    pub summary: String,

    /// Strengths identified in the code
    pub strengths: Vec<String>,

    /// Whether retry is recommended to fix issues
    pub retry_recommended: bool,

    /// Time taken for review (seconds)
    pub review_time: u64,
}

/// Overall assessment after code review
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewAssessment {
    /// No issues found - code is excellent
    Pass,

    /// Minor issues found - not blocking
    Warning,

    /// Significant issues found - should be fixed
    NeedsImprovements,

    /// Critical issues found - must be fixed
    Fail,
}

impl std::fmt::Display for ReviewAssessment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewAssessment::Pass => write!(f, "pass"),
            ReviewAssessment::Warning => write!(f, "warning"),
            ReviewAssessment::NeedsImprovements => write!(f, "needs_improvements"),
            ReviewAssessment::Fail => write!(f, "fail"),
        }
    }
}

/// Result of reviewing multiple tasks
#[derive(Debug, Clone)]
pub struct ReviewSummary {
    /// Total tasks reviewed
    pub total_tasks: usize,

    /// Tasks that passed review
    pub passed_tasks: usize,

    /// Tasks with warnings
    pub warning_tasks: usize,

    /// Tasks that need improvements
    pub needs_improvements: usize,

    /// Tasks that failed review
    pub failed_tasks: usize,

    /// Tasks that were skipped
    pub skipped_tasks: usize,

    /// Total review time in seconds
    pub total_time: u64,

    /// All issues found across all tasks
    pub all_issues: Vec<CodeIssue>,

    /// Issues by category
    pub issues_by_category: Vec<(IssueCategory, usize)>,

    /// Issues by severity
    pub issues_by_severity: Vec<(IssueSeverity, usize)>,

    /// Individual review results
    pub results: Vec<ReviewResult>,
}

/// Review a list of completed tasks
///
/// This function performs comprehensive code review on all completed tasks,
/// checking for security issues, performance problems, code quality issues,
/// and best practices violations.
///
/// # Arguments
///
/// * `tasks` - Tasks to review (typically completed tasks)
/// * `config` - Review configuration
///
/// # Returns
///
/// A tuple of (updated tasks, review summary)
pub async fn review_tasks(
    tasks: Vec<Task>,
    config: &ReviewConfig,
) -> Result<(Vec<Task>, ReviewSummary)> {
    let start_time = std::time::Instant::now();

    info!("Starting review stage for {} tasks", tasks.len());

    // Check if review should run
    if !config.should_run() {
        info!("Review disabled by config or not in expert mode, skipping");
        let total = tasks.len();
        return Ok((
            tasks,
            ReviewSummary {
                total_tasks: total,
                passed_tasks: 0,
                warning_tasks: 0,
                needs_improvements: 0,
                failed_tasks: 0,
                skipped_tasks: total,
                total_time: 0,
                all_issues: Vec::new(),
                issues_by_category: Vec::new(),
                issues_by_severity: Vec::new(),
                results: Vec::new(),
            },
        ));
    }

    // Log expert mode activation
    if config.is_expert_mode() {
        info!("Running in expert mode with comprehensive code review");
    }

    // Create Claude agent for review
    let agent = ClaudeAgent::new().context("Failed to create Claude agent for review")?;

    // Filter to only completed tasks
    let completed_tasks: Vec<_> = tasks
        .into_iter()
        .filter(|t| t.is_completed() || t.status == TaskStatus::InProgress)
        .collect();

    let total = completed_tasks.len();
    let mut results = Vec::new();
    let mut all_issues = Vec::new();
    let mut passed = 0;
    let mut warnings = 0;
    let mut needs_improvements = 0;
    let mut failed = 0;

    for task in completed_tasks {
        info!("Reviewing task: {}", task.id);

        // Perform the review
        let result = review_single_task(&task, &agent, config).await?;

        // Count issues by category and severity
        for issue in &result.issues {
            if issue.severity >= config.severity_threshold {
                all_issues.push(issue.clone());
            }
        }

        // Update assessment counts
        match result.assessment {
            ReviewAssessment::Pass => passed += 1,
            ReviewAssessment::Warning => warnings += 1,
            ReviewAssessment::NeedsImprovements => needs_improvements += 1,
            ReviewAssessment::Fail => failed += 1,
        }

        results.push(result);
    }

    let elapsed = start_time.elapsed().as_secs();

    // Aggregate issues by category
    let mut issues_by_category_map = std::collections::HashMap::new();
    for issue in &all_issues {
        *issues_by_category_map.entry(issue.category).or_insert(0) += 1;
    }
    let issues_by_category: Vec<_> = issues_by_category_map
        .into_iter()
        .collect();

    // Aggregate issues by severity
    let mut issues_by_severity_map = std::collections::HashMap::new();
    for issue in &all_issues {
        *issues_by_severity_map.entry(issue.severity).or_insert(0) += 1;
    }
    let issues_by_severity: Vec<_> = issues_by_severity_map
        .into_iter()
        .collect();

    info!(
        "Review stage completed in {}s: {} passed, {} warnings, {} needs improvements, {} failed",
        elapsed, passed, warnings, needs_improvements, failed
    );

    // Collect updated tasks
    let updated_tasks: Vec<_> = results
        .iter()
        .map(|r| r.task.clone())
        .collect();

    let summary = ReviewSummary {
        total_tasks: total,
        passed_tasks: passed,
        warning_tasks: warnings,
        needs_improvements: needs_improvements,
        failed_tasks: failed,
        skipped_tasks: 0,
        total_time: elapsed,
        all_issues,
        issues_by_category,
        issues_by_severity,
        results,
    };

    Ok((updated_tasks, summary))
}

/// Review a single task
///
/// Performs comprehensive code review on a single task including
/// security, performance, quality, and best practices checks.
async fn review_single_task(
    task: &Task,
    agent: &ClaudeAgent,
    config: &ReviewConfig,
) -> Result<ReviewResult> {
    let start_time = std::time::Instant::now();

    // Build review prompt based on configuration
    let prompt = build_review_prompt(task, config);

    // Create execution config
    let exec_config = ExecutionConfig {
        model: config.review_model.clone(),
        max_retries: 1,
        timeout: config.timeout,
        enable_session: true,
        env_vars: Vec::new(),
    };

    // Execute review
    let response = agent
        .execute(&prompt, &exec_config)
        .await
        .context("Failed to execute code review")?;

    let elapsed = start_time.elapsed().as_secs();

    // Parse the review response
    let (issues, assessment, summary, strengths) =
        parse_review_response(&response.output, config)?;

    // Determine if retry is recommended
    let retry_recommended = assessment == ReviewAssessment::NeedsImprovements
        || assessment == ReviewAssessment::Fail;

    // Check for blocking issues
    let has_blocking_issues = issues.iter().any(|i| i.blocking);

    let final_assessment = if has_blocking_issues {
        ReviewAssessment::Fail
    } else {
        assessment
    };

    Ok(ReviewResult {
        task: task.clone(),
        assessment: final_assessment,
        issues,
        summary,
        strengths,
        retry_recommended,
        review_time: elapsed,
    })
}

/// Build the review prompt for a task
pub fn build_review_prompt(task: &Task, config: &ReviewConfig) -> String {
    let mut checks = Vec::new();

    if config.check_security {
        checks.push("Security vulnerabilities (injection attacks, authentication issues, data exposure)");
    }
    if config.check_performance {
        checks.push("Performance issues (inefficient algorithms, unnecessary allocations, I/O problems)");
    }
    if config.check_quality {
        checks.push("Code quality (readability, maintainability, naming, structure)");
    }
    if config.check_best_practices {
        checks.push("Best practices (language idioms, design patterns, error handling)");
    }

    let checks_list = checks
        .iter()
        .enumerate()
        .map(|(i, check)| format!("{}. {}", i + 1, check))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "You are an expert code reviewer. Review the following completed task for code quality issues.\n\n\
         Task: {}\n\
         Description: {}\n\n\
         Check the following aspects:\n\
         {}\n\n\
         Severity levels:\n\
         - Critical: Security vulnerabilities or blocking issues\n\
         - High: Serious problems that must be fixed\n\
         - Medium: Moderate issues that should be fixed\n\
         - Low: Minor issues that should be addressed\n\
         - Info: Minor suggestions\n\n\
         For each issue found, provide:\n\
         1. Category (security, performance, quality, best_practices, documentation, testing, error_handling)\n\
         2. Severity (critical, high, medium, low, info)\n\
         3. File and line number (if applicable)\n\
         4. Brief title\n\
         5. Detailed description\n\
         6. Suggested fix\n\
         7. Whether this issue blocks the task from being considered complete\n\n\
         After reviewing, provide:\n\
         1. Overall assessment (pass/warning/needs_improvements/fail)\n\
         2. Summary of findings\n\
         3. Strengths in the code\n\
         4. Total issue count\n\n\
         Please format your response as JSON with this structure:\n\
         {{\n\
           \"assessment\": \"pass|warning|needs_improvements|fail\",\n\
           \"summary\": \"Overall summary\",\n\
           \"strengths\": [\"strength1\", \"strength2\"],\n\
           \"issues\": [\n\
             {{\n\
               \"category\": \"security|performance|quality|best_practices|documentation|testing|error_handling\",\n\
               \"severity\": \"critical|high|medium|low|info\",\n\
               \"file\": \"path/to/file\" (optional),\n\
               \"line\": number (optional),\n\
               \"title\": \"Brief title\",\n\
               \"description\": \"Detailed description\",\n\
               \"suggestion\": \"Suggested fix\",\n\
               \"code_snippet\": \"Code example\" (optional),\n\
               \"blocking\": true|false\n\
             }}\n\
           ]\n\
         }}",
        task.title,
        task.description,
        checks_list
    )
}

/// Parse the review response from the agent
fn parse_review_response(
    response: &str,
    config: &ReviewConfig,
) -> Result<(Vec<CodeIssue>, ReviewAssessment, String, Vec<String>)> {
    // Try to parse as JSON first
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(response) {
        return parse_json_review(&json, config);
    }

    // Fallback: parse text response
    parse_text_review(response, config)
}

/// Parse JSON-formatted review response
fn parse_json_review(
    json: &serde_json::Value,
    config: &ReviewConfig,
) -> Result<(Vec<CodeIssue>, ReviewAssessment, String, Vec<String>)> {
    let assessment_str = json["assessment"]
        .as_str()
        .unwrap_or("warning");
    let assessment = match assessment_str {
        "pass" => ReviewAssessment::Pass,
        "warning" => ReviewAssessment::Warning,
        "needs_improvements" => ReviewAssessment::NeedsImprovements,
        "fail" => ReviewAssessment::Fail,
        _ => ReviewAssessment::Warning,
    };

    let summary = json["summary"]
        .as_str()
        .unwrap_or("Review completed")
        .to_string();

    let strengths: Vec<String> = json["strengths"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    let issues: Vec<CodeIssue> = json["issues"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| parse_issue_json(v))
                .filter(|issue| issue.severity >= config.severity_threshold)
                .take(config.max_issues_per_category)
                .collect()
        })
        .unwrap_or_default();

    Ok((issues, assessment, summary, strengths))
}

/// Parse a single issue from JSON
fn parse_issue_json(json: &serde_json::Value) -> Option<CodeIssue> {
    let category_str = json["category"].as_str()?;
    let category = match category_str {
        "security" => IssueCategory::Security,
        "performance" => IssueCategory::Performance,
        "quality" => IssueCategory::Quality,
        "best_practices" => IssueCategory::BestPractices,
        "documentation" => IssueCategory::Documentation,
        "testing" => IssueCategory::Testing,
        "error_handling" => IssueCategory::ErrorHandling,
        _ => return None,
    };

    let severity_str = json["severity"].as_str()?;
    let severity = match severity_str {
        "critical" => IssueSeverity::Critical,
        "high" => IssueSeverity::High,
        "medium" => IssueSeverity::Medium,
        "low" => IssueSeverity::Low,
        "info" => IssueSeverity::Info,
        _ => IssueSeverity::Medium,
    };

    Some(CodeIssue {
        category,
        severity,
        file: json["file"].as_str().map(|s| s.to_string()),
        line: json["line"].as_u64().map(|v| v as usize),
        title: json["title"].as_str().unwrap_or("Issue").to_string(),
        description: json["description"].as_str().unwrap_or("").to_string(),
        suggestion: json["suggestion"].as_str().map(|s| s.to_string()),
        code_snippet: json["code_snippet"].as_str().map(|s| s.to_string()),
        blocking: json["blocking"].as_bool().unwrap_or(false),
    })
}

/// Parse text-formatted review response
fn parse_text_review(
    response: &str,
    _config: &ReviewConfig,
) -> Result<(Vec<CodeIssue>, ReviewAssessment, String, Vec<String>)> {
    // Simple text parsing - look for keywords
    let response_lower = response.to_lowercase();

    let assessment = if response_lower.contains("critical")
        || response_lower.contains("failed")
        || response_lower.contains("fail")
    {
        ReviewAssessment::Fail
    } else if response_lower.contains("needs improvement")
        || response_lower.contains("should be fixed")
    {
        ReviewAssessment::NeedsImprovements
    } else if response_lower.contains("warning")
        || response_lower.contains("minor issues")
    {
        ReviewAssessment::Warning
    } else {
        ReviewAssessment::Pass
    };

    let summary = if response.len() > 200 {
        format!("{}...", &response[..200])
    } else {
        response.to_string()
    };

    // Try to extract strengths (lines with "good", "excellent", "well")
    let strengths: Vec<String> = response
        .lines()
        .filter(|line| {
            let line_lower = line.to_lowercase();
            line_lower.contains("good")
                || line_lower.contains("excellent")
                || line_lower.contains("well")
                || line_lower.contains("strong")
                || line_lower.contains("proper")
        })
        .map(|s| s.trim().to_string())
        .take(5)
        .collect();

    Ok((Vec::new(), assessment, summary, strengths))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_config_default() {
        let config = ReviewConfig::default();
        assert!(!config.enabled); // Disabled by default
        assert_eq!(config.review_model, "claude-opus-4-6");
        assert_eq!(config.max_issues_per_category, 10);
        assert!(config.check_security);
        assert!(config.check_performance);
        assert!(config.check_quality);
        assert!(config.check_best_practices);
    }

    #[test]
    fn test_review_config_expert_mode() {
        let config = ReviewConfig::expert_mode();
        assert!(config.enabled); // Enabled in expert mode
        assert!(config.should_run());
        assert!(config.is_expert_mode());
        assert_eq!(config.review_model, "claude-opus-4-6");
        assert_eq!(config.max_issues_per_category, 15);
        assert_eq!(config.severity_threshold, IssueSeverity::Low);
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
    }

    #[test]
    fn test_review_assessment_display() {
        assert_eq!(ReviewAssessment::Pass.to_string(), "pass");
        assert_eq!(ReviewAssessment::Warning.to_string(), "warning");
        assert_eq!(ReviewAssessment::NeedsImprovements.to_string(), "needs_improvements");
        assert_eq!(ReviewAssessment::Fail.to_string(), "fail");
    }

    #[test]
    fn test_issue_severity_ordering() {
        assert!(IssueSeverity::Critical > IssueSeverity::High);
        assert!(IssueSeverity::High > IssueSeverity::Medium);
        assert!(IssueSeverity::Medium > IssueSeverity::Low);
        assert!(IssueSeverity::Low > IssueSeverity::Info);
    }

    #[test]
    fn test_code_issue_blocking() {
        let issue = CodeIssue {
            category: IssueCategory::Security,
            severity: IssueSeverity::Critical,
            file: Some("test.rs".to_string()),
            line: Some(42),
            title: "SQL Injection".to_string(),
            description: "Critical security issue".to_string(),
            suggestion: Some("Use parameterized queries".to_string()),
            code_snippet: None,
            blocking: true,
        };

        assert!(issue.blocking);
        assert_eq!(issue.category, IssueCategory::Security);
        assert_eq!(issue.severity, IssueSeverity::Critical);
    }

    #[test]
    fn test_review_result_structure() {
        let task = Task::new("test-1", "Test Task", "Description");
        let result = ReviewResult {
            task: task.clone(),
            assessment: ReviewAssessment::Pass,
            issues: Vec::new(),
            summary: "Review passed".to_string(),
            strengths: vec!["Good code".to_string()],
            retry_recommended: false,
            review_time: 10,
        };

        assert_eq!(result.assessment, ReviewAssessment::Pass);
        assert!(!result.retry_recommended);
        assert_eq!(result.issues.len(), 0);
        assert_eq!(result.strengths.len(), 1);
    }

    #[test]
    fn test_review_summary_structure() {
        let summary = ReviewSummary {
            total_tasks: 10,
            passed_tasks: 7,
            warning_tasks: 2,
            needs_improvements: 1,
            failed_tasks: 0,
            skipped_tasks: 0,
            total_time: 100,
            all_issues: Vec::new(),
            issues_by_category: Vec::new(),
            issues_by_severity: Vec::new(),
            results: Vec::new(),
        };

        assert_eq!(summary.total_tasks, 10);
        assert_eq!(summary.passed_tasks, 7);
        assert_eq!(summary.warning_tasks, 2);
        assert_eq!(summary.needs_improvements, 1);
        assert_eq!(summary.failed_tasks, 0);
    }

    #[tokio::test]
    async fn test_review_tasks_when_disabled() {
        let config = ReviewConfig {
            enabled: false,
            ..Default::default()
        };

        let tasks = vec![Task::new("test-1", "Test", "Description")];
        let (updated_tasks, summary) = review_tasks(tasks, &config)
            .await
            .unwrap();

        assert_eq!(updated_tasks.len(), 1);
        assert_eq!(summary.total_tasks, 1);
        assert_eq!(summary.skipped_tasks, 1);
        assert_eq!(summary.passed_tasks, 0);
    }

    #[test]
    fn test_parse_issue_json_complete() {
        let json = serde_json::json!({
            "category": "security",
            "severity": "critical",
            "file": "src/main.rs",
            "line": 42,
            "title": "SQL Injection",
            "description": "User input not sanitized",
            "suggestion": "Use parameterized queries",
            "code_snippet": "format!(\"SELECT * FROM users WHERE id = {}\", input)",
            "blocking": true
        });

        let issue = parse_issue_json(&json).expect("Should parse valid issue");

        assert_eq!(issue.category, IssueCategory::Security);
        assert_eq!(issue.severity, IssueSeverity::Critical);
        assert_eq!(issue.file, Some("src/main.rs".to_string()));
        assert_eq!(issue.line, Some(42));
        assert_eq!(issue.title, "SQL Injection");
        assert_eq!(issue.description, "User input not sanitized");
        assert_eq!(issue.suggestion, Some("Use parameterized queries".to_string()));
        assert_eq!(issue.code_snippet, Some("format!(\"SELECT * FROM users WHERE id = {}\", input)".to_string()));
        assert!(issue.blocking);
    }

    #[test]
    fn test_parse_issue_json_minimal() {
        let json = serde_json::json!({
            "category": "performance",
            "severity": "medium",
            "title": "Inefficient loop",
            "description": "Using O(n²) algorithm"
        });

        let issue = parse_issue_json(&json).expect("Should parse minimal issue");

        assert_eq!(issue.category, IssueCategory::Performance);
        assert_eq!(issue.severity, IssueSeverity::Medium);
        assert_eq!(issue.file, None);
        assert_eq!(issue.line, None);
        assert_eq!(issue.title, "Inefficient loop");
        assert_eq!(issue.suggestion, None);
        assert_eq!(issue.code_snippet, None);
        assert!(!issue.blocking);
    }

    #[test]
    fn test_parse_issue_json_all_categories() {
        let categories = [
            ("security", IssueCategory::Security),
            ("performance", IssueCategory::Performance),
            ("quality", IssueCategory::Quality),
            ("best_practices", IssueCategory::BestPractices),
            ("documentation", IssueCategory::Documentation),
            ("testing", IssueCategory::Testing),
            ("error_handling", IssueCategory::ErrorHandling),
        ];

        for (category_str, expected_category) in categories {
            let json = serde_json::json!({
                "category": category_str,
                "severity": "low",
                "title": "Test issue",
                "description": "Test description"
            });

            let issue = parse_issue_json(&json).expect(&format!("Should parse category: {}", category_str));
            assert_eq!(issue.category, expected_category, "Category mismatch for {}", category_str);
        }
    }

    #[test]
    fn test_parse_issue_json_all_severities() {
        let severities = [
            ("critical", IssueSeverity::Critical),
            ("high", IssueSeverity::High),
            ("medium", IssueSeverity::Medium),
            ("low", IssueSeverity::Low),
            ("info", IssueSeverity::Info),
        ];

        for (severity_str, expected_severity) in severities {
            let json = serde_json::json!({
                "category": "quality",
                "severity": severity_str,
                "title": "Test issue",
                "description": "Test description"
            });

            let issue = parse_issue_json(&json).expect(&format!("Should parse severity: {}", severity_str));
            assert_eq!(issue.severity, expected_severity, "Severity mismatch for {}", severity_str);
        }
    }

    #[test]
    fn test_parse_issue_json_invalid_category() {
        let json = serde_json::json!({
            "category": "invalid_category",
            "severity": "high",
            "title": "Test",
            "description": "Test"
        });

        assert!(parse_issue_json(&json).is_none(), "Should return None for invalid category");
    }

    #[test]
    fn test_parse_issue_json_missing_required_fields() {
        // Missing category
        let json1 = serde_json::json!({
            "severity": "high",
            "title": "Test",
            "description": "Test"
        });
        assert!(parse_issue_json(&json1).is_none());

        // Missing severity
        let json2 = serde_json::json!({
            "category": "security",
            "title": "Test",
            "description": "Test"
        });
        assert!(parse_issue_json(&json2).is_none());
    }

    #[test]
    fn test_parse_json_review_pass() {
        let json = serde_json::json!({
            "assessment": "pass",
            "summary": "Code is excellent, no issues found",
            "strengths": ["Good error handling", "Well documented"],
            "issues": []
        });

        let config = ReviewConfig::default();
        let (issues, assessment, summary, strengths) = parse_json_review(&json, &config)
            .expect("Should parse valid JSON review");

        assert_eq!(assessment, ReviewAssessment::Pass);
        assert_eq!(summary, "Code is excellent, no issues found");
        assert_eq!(strengths.len(), 2);
        assert!(strengths.contains(&"Good error handling".to_string()));
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_parse_json_review_with_issues() {
        let json = serde_json::json!({
            "assessment": "needs_improvements",
            "summary": "Several issues found that need fixing",
            "strengths": ["Good structure"],
            "issues": [
                {
                    "category": "security",
                    "severity": "high",
                    "title": "SQL injection",
                    "description": "Unsanitized input",
                    "blocking": false
                },
                {
                    "category": "performance",
                    "severity": "medium",
                    "title": "Slow query",
                    "description": "Missing index",
                    "blocking": false
                }
            ]
        });

        let config = ReviewConfig {
            severity_threshold: IssueSeverity::Low,
            max_issues_per_category: 10,
            ..Default::default()
        };

        let (issues, assessment, summary, strengths) = parse_json_review(&json, &config)
            .expect("Should parse JSON review with issues");

        assert_eq!(assessment, ReviewAssessment::NeedsImprovements);
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].category, IssueCategory::Security);
        assert_eq!(issues[1].category, IssueCategory::Performance);
        assert_eq!(strengths.len(), 1);
    }

    #[test]
    fn test_parse_json_review_filters_by_severity() {
        let json = serde_json::json!({
            "assessment": "warning",
            "summary": "Some minor issues",
            "strengths": [],
            "issues": [
                {
                    "category": "quality",
                    "severity": "low",
                    "title": "Minor style issue",
                    "description": "Inconsistent naming",
                    "blocking": false
                },
                {
                    "category": "quality",
                    "severity": "info",
                    "title": "Suggestion",
                    "description": "Could be more idiomatic",
                    "blocking": false
                }
            ]
        });

        let config = ReviewConfig {
            severity_threshold: IssueSeverity::Low,
            max_issues_per_category: 10,
            ..Default::default()
        };

        let (issues, _, _, _) = parse_json_review(&json, &config)
            .expect("Should parse and filter by severity");

        // Only include issues with severity >= Low (Low and above, not Info)
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, IssueSeverity::Low);
    }

    #[test]
    fn test_parse_json_review_limits_issues_per_category() {
        // Create 15 issues in the same category
        let issues: Vec<serde_json::Value> = (0..15)
            .map(|i| serde_json::json!({
                "category": "quality",
                "severity": "medium",
                "title": format!("Issue {}", i),
                "description": format!("Description {}", i),
                "blocking": false
            }))
            .collect();

        let json = serde_json::json!({
            "assessment": "warning",
            "summary": "Many issues",
            "strengths": [],
            "issues": issues
        });

        let config = ReviewConfig {
            severity_threshold: IssueSeverity::Info,
            max_issues_per_category: 10,
            ..Default::default()
        };

        let (issues, _, _, _) = parse_json_review(&json, &config)
            .expect("Should parse and limit issues");

        assert_eq!(issues.len(), 10, "Should limit to max_issues_per_category");
    }

    #[test]
    fn test_parse_json_review_handles_invalid_assessment() {
        let json = serde_json::json!({
            "assessment": "invalid_assessment",
            "summary": "Test",
            "strengths": [],
            "issues": []
        });

        let config = ReviewConfig::default();
        let (_, assessment, _, _) = parse_json_review(&json, &config)
            .expect("Should handle invalid assessment gracefully");

        // Should default to Warning for unknown assessment
        assert_eq!(assessment, ReviewAssessment::Warning);
    }

    #[test]
    fn test_parse_text_review_fail() {
        let response = "CRITICAL issues found in the code. Security vulnerabilities detected.";

        let config = ReviewConfig::default();
        let (issues, assessment, summary, strengths) = parse_text_review(response, &config)
            .expect("Should parse text review");

        assert_eq!(assessment, ReviewAssessment::Fail);
        assert!(summary.contains("CRITICAL"));
        assert_eq!(issues.len(), 0); // Text parsing doesn't extract structured issues
    }

    #[test]
    fn test_parse_text_review_needs_improvements() {
        let response = "The code has some issues that should be fixed. Performance is lacking and needs improvement.";

        let config = ReviewConfig::default();
        let (_, assessment, _, _) = parse_text_review(response, &config)
            .expect("Should parse text review");

        assert_eq!(assessment, ReviewAssessment::NeedsImprovements);
    }

    #[test]
    fn test_parse_text_review_warning() {
        let response = "Code looks good but there are some minor issues and warnings to consider.";

        let config = ReviewConfig::default();
        let (_, assessment, _, _) = parse_text_review(response, &config)
            .expect("Should parse text review");

        assert_eq!(assessment, ReviewAssessment::Warning);
    }

    #[test]
    fn test_parse_text_review_pass() {
        let response = "Excellent implementation! Everything works correctly.";

        let config = ReviewConfig::default();
        let (_, assessment, _, _) = parse_text_review(response, &config)
            .expect("Should parse text review");

        assert_eq!(assessment, ReviewAssessment::Pass);
    }

    #[test]
    fn test_parse_text_review_extracts_strengths() {
        let response = "Good error handling throughout. The code is well structured and properly documented. Excellent performance characteristics.";

        let config = ReviewConfig::default();
        let (_, _, _, strengths) = parse_text_review(response, &config)
            .expect("Should parse text review");

        assert!(!strengths.is_empty());
        assert!(strengths.iter().any(|s| s.contains("Good") || s.contains("well") || s.contains("Excellent")));
    }

    #[test]
    fn test_parse_text_review_truncates_long_summary() {
        let long_response = "A".repeat(300);
        let config = ReviewConfig::default();

        let (_, _, summary, _) = parse_text_review(&long_response, &config)
            .expect("Should parse and truncate long summary");

        assert!(summary.len() <= 203, "Summary should be truncated to ~200 chars plus ellipsis");
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_build_review_prompt_formatting() {
        let task = Task::new("task-1", "Implement auth", "Add JWT authentication");
        let config = ReviewConfig::expert_mode();

        let prompt = build_review_prompt(&task, &config);

        // Check key elements are present
        assert!(prompt.contains("expert code reviewer"));
        assert!(prompt.contains("Task: Implement auth"));
        assert!(prompt.contains("Add JWT authentication"));
        assert!(prompt.contains("Overall assessment"));
        assert!(prompt.contains("Severity levels"));
        assert!(prompt.contains("blocking"));
    }

    #[test]
    fn test_review_assessment_retry_recommendation() {
        // Pass should not recommend retry
        let result1 = ReviewResult {
            task: Task::new("test-1", "Test", "Description"),
            assessment: ReviewAssessment::Pass,
            issues: vec![],
            summary: "Good".to_string(),
            strengths: vec![],
            retry_recommended: false,
            review_time: 10,
        };

        // Check manually calculated retry recommendation
        let needs_retry = matches!(
            result1.assessment,
            ReviewAssessment::NeedsImprovements | ReviewAssessment::Fail
        );
        assert!(!needs_retry);

        // NeedsImprovements should recommend retry
        let result2 = ReviewResult {
            assessment: ReviewAssessment::NeedsImprovements,
            ..result1.clone()
        };
        let needs_retry = matches!(
            result2.assessment,
            ReviewAssessment::NeedsImprovements | ReviewAssessment::Fail
        );
        assert!(needs_retry);

        // Fail should recommend retry
        let result3 = ReviewResult {
            assessment: ReviewAssessment::Fail,
            ..result1
        };
        let needs_retry = matches!(
            result3.assessment,
            ReviewAssessment::NeedsImprovements | ReviewAssessment::Fail
        );
        assert!(needs_retry);
    }
}
