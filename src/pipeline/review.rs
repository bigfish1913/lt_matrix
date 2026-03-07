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
//!
//! # Data Structures
//!
//! - [`ReviewSeverity`]: Severity levels for review findings (Info to Critical)
//! - [`ReviewCategory`]: Categories for classifying review findings
//! - [`ReviewFinding`]: A single finding from code review
//! - [`ReviewReport`]: Comprehensive report of all findings for a task
//! - [`ReviewSummary`]: Aggregated summary across multiple tasks
//!
//! # Expert Mode vs Standard Mode
//!
//! In **expert mode**:
//! - All categories are checked (security, performance, quality, best practices, etc.)
//! - Lower severity threshold (reports Info and above)
//! - More issues per category allowed
//! - Blocking issues prevent task completion
//!
//! In **standard mode**:
//! - Review stage is skipped (runs only verify stage)
//! - When manually enabled, uses higher severity threshold

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use tracing::info;

use crate::agent::backend::{AgentBackend, ExecutionConfig};
use crate::agent::claude::ClaudeAgent;
use crate::models::{ModeConfig, Task, TaskStatus};

// =============================================================================
// Error Types for Pipeline Integration
// =============================================================================

/// Errors that can occur during the review stage
#[derive(Debug, Error)]
pub enum ReviewError {
    /// Failed to create or communicate with the review agent
    #[error("Agent error during review: {0}")]
    AgentError(String),

    /// Failed to parse the review response
    #[error("Failed to parse review response: {0}")]
    ParseError(String),

    /// Review timed out
    #[error("Review timed out after {0} seconds")]
    Timeout(u64),

    /// Critical issues found that block completion
    #[error("Critical issues found: {0}")]
    CriticalIssues(String),

    /// Configuration error
    #[error("Invalid review configuration: {0}")]
    ConfigError(String),

    /// I/O error during review
    #[error("I/O error during review: {0}")]
    IoError(#[from] std::io::Error),
}

impl ReviewError {
    /// Check if this error is recoverable (can trigger fix cycle)
    pub fn is_recoverable(&self) -> bool {
        matches!(self, ReviewError::CriticalIssues(_))
    }

    /// Check if this error should cause pipeline to stop
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            ReviewError::AgentError(_)
                | ReviewError::ParseError(_)
                | ReviewError::Timeout(_)
                | ReviewError::ConfigError(_)
        )
    }

    /// Get error code for logging/telemetry
    pub fn error_code(&self) -> &'static str {
        match self {
            ReviewError::AgentError(_) => "REVIEW_AGENT_ERROR",
            ReviewError::ParseError(_) => "REVIEW_PARSE_ERROR",
            ReviewError::Timeout(_) => "REVIEW_TIMEOUT",
            ReviewError::CriticalIssues(_) => "REVIEW_CRITICAL_ISSUES",
            ReviewError::ConfigError(_) => "REVIEW_CONFIG_ERROR",
            ReviewError::IoError(_) => "REVIEW_IO_ERROR",
        }
    }
}

// =============================================================================
// Review Severity - Primary Severity Type for Code Review
// =============================================================================

/// Severity levels for review findings
///
/// This enum defines the severity of issues found during code review.
/// Severity affects both the filtering of findings and the overall
/// assessment of the review.
///
/// # Ordering
///
/// Severity levels are ordered from least to most severe:
/// `Info < Low < Medium < High < Critical`
///
/// # Usage in Pipeline
///
/// - `Critical`: Blocks task completion, must be fixed before commit
/// - `High`: Should be fixed before merge, triggers fix cycle
/// - `Medium`: Should be addressed, may trigger fix cycle in expert mode
/// - `Low`: Minor issues, informational
/// - `Info`: Suggestions and recommendations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewSeverity {
    /// Informational - suggestions and recommendations
    /// Not actionable but helpful feedback
    Info,

    /// Low - minor issues that could be improved
    /// Non-blocking, cosmetic or minor style issues
    Low,

    /// Medium - moderate issues that should be addressed
    /// May affect maintainability or code quality
    Medium,

    /// High - serious issues that should be fixed
    /// Could lead to bugs or performance problems
    /// Triggers fix cycle in expert mode
    High,

    /// Critical - blocking issues that must be fixed
    /// Security vulnerabilities or blocking defects
    /// Prevents task from being marked complete
    Critical,
}

impl Default for ReviewSeverity {
    fn default() -> Self {
        ReviewSeverity::Medium
    }
}

impl std::fmt::Display for ReviewSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewSeverity::Info => write!(f, "info"),
            ReviewSeverity::Low => write!(f, "low"),
            ReviewSeverity::Medium => write!(f, "medium"),
            ReviewSeverity::High => write!(f, "high"),
            ReviewSeverity::Critical => write!(f, "critical"),
        }
    }
}

impl ReviewSeverity {
    /// Get a human-readable description of the severity
    pub fn description(&self) -> &'static str {
        match self {
            ReviewSeverity::Info => "Informational suggestion",
            ReviewSeverity::Low => "Minor issue",
            ReviewSeverity::Medium => "Moderate issue",
            ReviewSeverity::High => "Serious issue",
            ReviewSeverity::Critical => "Critical issue - blocks completion",
        }
    }

    /// Check if this severity should block task completion
    pub fn is_blocking(&self) -> bool {
        matches!(self, ReviewSeverity::Critical)
    }

    /// Check if this severity should trigger a fix cycle
    pub fn triggers_fix_cycle(&self, expert_mode: bool) -> bool {
        match self {
            ReviewSeverity::Critical | ReviewSeverity::High => true,
            ReviewSeverity::Medium => expert_mode,
            ReviewSeverity::Low | ReviewSeverity::Info => false,
        }
    }

    /// Get the ANSI color code for terminal display
    pub fn color_code(&self) -> &'static str {
        match self {
            ReviewSeverity::Info => "\x1b[34m",      // Blue
            ReviewSeverity::Low => "\x1b[36m",       // Cyan
            ReviewSeverity::Medium => "\x1b[33m",    // Yellow
            ReviewSeverity::High => "\x1b[31m",      // Red
            ReviewSeverity::Critical => "\x1b[35m",  // Magenta
        }
    }

    /// Get the icon/emoji for visual display
    pub fn icon(&self) -> &'static str {
        match self {
            ReviewSeverity::Info => "ℹ",
            ReviewSeverity::Low => "○",
            ReviewSeverity::Medium => "◐",
            ReviewSeverity::High => "●",
            ReviewSeverity::Critical => "⚠",
        }
    }
}

// =============================================================================
// Review Category - Categories for Classifying Findings
// =============================================================================

/// Categories of review findings
///
/// Each finding is categorized to help organize the review report
/// and enable filtering by category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewCategory {
    /// Security vulnerabilities (injection, auth issues, data exposure)
    Security,

    /// Performance problems (inefficient algorithms, memory issues)
    Performance,

    /// Code quality issues (readability, maintainability, structure)
    Quality,

    /// Best practices violations (language idioms, design patterns)
    BestPractices,

    /// Documentation issues (missing or outdated docs)
    Documentation,

    /// Testing issues (missing tests, poor test coverage)
    Testing,

    /// Error handling issues (unhandled errors, poor error messages)
    ErrorHandling,

    /// Architectural issues (module structure, dependencies)
    Architecture,

    /// Style and formatting issues
    Style,
}

impl std::fmt::Display for ReviewCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewCategory::Security => write!(f, "security"),
            ReviewCategory::Performance => write!(f, "performance"),
            ReviewCategory::Quality => write!(f, "quality"),
            ReviewCategory::BestPractices => write!(f, "best_practices"),
            ReviewCategory::Documentation => write!(f, "documentation"),
            ReviewCategory::Testing => write!(f, "testing"),
            ReviewCategory::ErrorHandling => write!(f, "error_handling"),
            ReviewCategory::Architecture => write!(f, "architecture"),
            ReviewCategory::Style => write!(f, "style"),
        }
    }
}

impl ReviewCategory {
    /// Get a description of what this category covers
    pub fn description(&self) -> &'static str {
        match self {
            ReviewCategory::Security => "Security vulnerabilities and risks",
            ReviewCategory::Performance => "Performance and efficiency issues",
            ReviewCategory::Quality => "Code quality and maintainability",
            ReviewCategory::BestPractices => "Best practices and idioms",
            ReviewCategory::Documentation => "Documentation completeness",
            ReviewCategory::Testing => "Test coverage and quality",
            ReviewCategory::ErrorHandling => "Error handling and recovery",
            ReviewCategory::Architecture => "Architectural and design issues",
            ReviewCategory::Style => "Code style and formatting",
        }
    }

    /// Get the default severity for this category
    pub fn default_severity(&self) -> ReviewSeverity {
        match self {
            ReviewCategory::Security => ReviewSeverity::High,
            ReviewCategory::Performance => ReviewSeverity::Medium,
            ReviewCategory::Quality => ReviewSeverity::Medium,
            ReviewCategory::BestPractices => ReviewSeverity::Low,
            ReviewCategory::Documentation => ReviewSeverity::Low,
            ReviewCategory::Testing => ReviewSeverity::Medium,
            ReviewCategory::ErrorHandling => ReviewSeverity::Medium,
            ReviewCategory::Architecture => ReviewSeverity::High,
            ReviewCategory::Style => ReviewSeverity::Info,
        }
    }

    /// Check if this category should be checked in the given mode
    pub fn is_enabled_for_mode(&self, expert_mode: bool) -> bool {
        match self {
            // Always check these categories
            ReviewCategory::Security
            | ReviewCategory::ErrorHandling
            | ReviewCategory::Quality => true,

            // Only check in expert mode
            ReviewCategory::Architecture | ReviewCategory::Documentation | ReviewCategory::Style => {
                expert_mode
            }

            // Check in both modes but with different thresholds
            ReviewCategory::Performance
            | ReviewCategory::BestPractices
            | ReviewCategory::Testing => true,
        }
    }
}

// =============================================================================
// Review Finding - A Single Finding from Code Review
// =============================================================================

/// A single finding from code review
///
/// Represents a specific issue, suggestion, or observation discovered
/// during the code review process.
///
/// # Example
///
/// ```rust
/// use ltmatrix::pipeline::review::{ReviewFinding, ReviewCategory, ReviewSeverity};
///
/// let finding = ReviewFinding::new(
///     ReviewCategory::Security,
///     ReviewSeverity::High,
///     "Potential SQL injection",
/// )
/// .with_file("src/db.rs", 42)
/// .with_description("User input is concatenated directly into SQL query")
/// .with_suggestion("Use parameterized queries or prepared statements")
/// .blocking(true);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewFinding {
    /// Unique identifier for this finding
    pub id: String,

    /// Category of the finding
    pub category: ReviewCategory,

    /// Severity level
    pub severity: ReviewSeverity,

    /// Brief title/summary of the finding
    pub title: String,

    /// Detailed description of the finding
    pub description: String,

    /// File path where the issue was found (if applicable)
    pub file: Option<String>,

    /// Line number in the file (if applicable)
    pub line: Option<usize>,

    /// End line number for multi-line findings
    pub end_line: Option<usize>,

    /// Column number for precise positioning
    pub column: Option<usize>,

    /// Suggested fix or improvement
    pub suggestion: Option<String>,

    /// Code snippet showing the issue
    pub code_snippet: Option<String>,

    /// Whether this finding blocks task completion
    pub blocking: bool,

    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,

    /// Related findings (IDs of related issues)
    pub related: Vec<String>,

    /// Tags for filtering and categorization
    pub tags: Vec<String>,

    /// CWE (Common Weakness Enumeration) ID for security issues
    pub cwe_id: Option<String>,

    /// Reference URL for more information
    pub reference: Option<String>,

    /// When this finding was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl ReviewFinding {
    /// Create a new finding with required fields
    pub fn new(
        category: ReviewCategory,
        severity: ReviewSeverity,
        title: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now();
        let title_str = title.into();
        let id = format!(
            "{}-{}-{}",
            category,
            now.timestamp(),
            title_str.chars().take(20).collect::<String>()
        );

        ReviewFinding {
            id,
            category,
            severity,
            title: title_str,
            description: String::new(),
            file: None,
            line: None,
            end_line: None,
            column: None,
            suggestion: None,
            code_snippet: None,
            blocking: severity.is_blocking(),
            confidence: 0.8,
            related: Vec::new(),
            tags: Vec::new(),
            cwe_id: None,
            reference: None,
            created_at: now,
        }
    }

    /// Set the file location
    pub fn with_file(mut self, file: impl Into<String>, line: usize) -> Self {
        self.file = Some(file.into());
        self.line = Some(line);
        self
    }

    /// Set the file location with end line
    pub fn with_file_range(
        mut self,
        file: impl Into<String>,
        start_line: usize,
        end_line: usize,
    ) -> Self {
        self.file = Some(file.into());
        self.line = Some(start_line);
        self.end_line = Some(end_line);
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Set the code snippet
    pub fn with_code_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.code_snippet = Some(snippet.into());
        self
    }

    /// Set whether this finding is blocking
    pub fn blocking(mut self, blocking: bool) -> Self {
        self.blocking = blocking;
        self
    }

    /// Set the confidence level
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set the CWE ID for security issues
    pub fn with_cwe(mut self, cwe_id: impl Into<String>) -> Self {
        self.cwe_id = Some(cwe_id.into());
        self
    }

    /// Set a reference URL
    pub fn with_reference(mut self, url: impl Into<String>) -> Self {
        self.reference = Some(url.into());
        self
    }

    /// Add a related finding ID
    pub fn add_related(&mut self, finding_id: impl Into<String>) {
        self.related.push(finding_id.into());
    }

    /// Check if this finding should be included based on severity threshold
    pub fn meets_severity_threshold(&self, threshold: ReviewSeverity) -> bool {
        self.severity >= threshold
    }

    /// Format the finding for display
    pub fn format(&self) -> String {
        let location = if let (Some(file), Some(line)) = (&self.file, self.line) {
            if let Some(end_line) = self.end_line {
                format!("{}:{}-{}", file, line, end_line)
            } else {
                format!("{}:{}", file, line)
            }
        } else {
            "unknown location".to_string()
        };

        format!(
            "{} [{}] {} ({}): {}",
            self.severity.icon(),
            self.severity.to_string().to_uppercase(),
            self.title,
            location,
            self.description
        )
    }
}

// =============================================================================
// Review Report - Comprehensive Report for a Task
// =============================================================================

/// Comprehensive review report for a single task
///
/// Contains all findings, overall assessment, and metadata from
/// reviewing a single task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewReport {
    /// ID of the task that was reviewed
    pub task_id: String,

    /// Overall assessment of the review
    pub assessment: ReviewAssessment,

    /// All findings from the review
    pub findings: Vec<ReviewFinding>,

    /// Summary text of the review
    pub summary: String,

    /// Strengths identified in the code
    pub strengths: Vec<String>,

    /// Overall recommendations
    pub recommendations: Vec<String>,

    /// Metrics about the findings
    pub metrics: ReviewMetrics,

    /// Whether retry is recommended
    pub retry_recommended: bool,

    /// Time taken for review (seconds)
    pub review_time_secs: u64,

    /// Model used for review
    pub review_model: String,

    /// Configuration used for review
    pub config_snapshot: ReviewConfigSnapshot,

    /// When the review was performed
    pub reviewed_at: chrono::DateTime<chrono::Utc>,
}

/// Metrics about review findings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReviewMetrics {
    /// Total number of findings
    pub total_findings: usize,

    /// Findings by severity
    pub by_severity: std::collections::HashMap<String, usize>,

    /// Findings by category
    pub by_category: std::collections::HashMap<String, usize>,

    /// Number of blocking findings
    pub blocking_count: usize,

    /// Number of files with findings
    pub files_affected: usize,

    /// Average confidence
    pub avg_confidence: f32,

    /// Lines of code reviewed
    pub lines_reviewed: usize,
}

/// Snapshot of review configuration for reproducibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewConfigSnapshot {
    /// Severity threshold used
    pub severity_threshold: ReviewSeverity,

    /// Categories checked
    pub categories_checked: Vec<ReviewCategory>,

    /// Maximum issues per category
    pub max_issues_per_category: usize,

    /// Expert mode was enabled
    pub expert_mode: bool,
}

impl ReviewReport {
    /// Create a new empty report for a task
    pub fn new(task_id: impl Into<String>) -> Self {
        ReviewReport {
            task_id: task_id.into(),
            assessment: ReviewAssessment::Pass,
            findings: Vec::new(),
            summary: String::new(),
            strengths: Vec::new(),
            recommendations: Vec::new(),
            metrics: ReviewMetrics::default(),
            retry_recommended: false,
            review_time_secs: 0,
            review_model: String::new(),
            config_snapshot: ReviewConfigSnapshot {
                severity_threshold: ReviewSeverity::Medium,
                categories_checked: Vec::new(),
                max_issues_per_category: 10,
                expert_mode: false,
            },
            reviewed_at: chrono::Utc::now(),
        }
    }

    /// Add a finding to the report
    pub fn add_finding(&mut self, finding: ReviewFinding) {
        self.findings.push(finding);
        self.update_metrics();
    }

    /// Add multiple findings
    pub fn add_findings(&mut self, findings: Vec<ReviewFinding>) {
        self.findings.extend(findings);
        self.update_metrics();
    }

    /// Update metrics based on current findings
    pub fn update_metrics(&mut self) {
        let mut by_severity = std::collections::HashMap::new();
        let mut by_category = std::collections::HashMap::new();
        let mut files = std::collections::HashSet::new();
        let mut blocking_count = 0;
        let mut total_confidence = 0.0f32;

        for finding in &self.findings {
            *by_severity.entry(finding.severity.to_string()).or_insert(0) += 1;
            *by_category.entry(finding.category.to_string()).or_insert(0) += 1;

            if finding.blocking {
                blocking_count += 1;
            }

            if let Some(ref file) = finding.file {
                files.insert(file.clone());
            }

            total_confidence += finding.confidence;
        }

        self.metrics = ReviewMetrics {
            total_findings: self.findings.len(),
            by_severity,
            by_category,
            blocking_count,
            files_affected: files.len(),
            avg_confidence: if self.findings.is_empty() {
                0.0
            } else {
                total_confidence / self.findings.len() as f32
            },
            lines_reviewed: 0, // Would be set during actual review
        };
    }

    /// Calculate the overall assessment based on findings
    pub fn calculate_assessment(&mut self) {
        let has_blocking = self.findings.iter().any(|f| f.blocking);
        let critical_count = self
            .metrics
            .by_severity
            .get("critical")
            .copied()
            .unwrap_or(0);
        let high_count = self
            .metrics
            .by_severity
            .get("high")
            .copied()
            .unwrap_or(0);
        let medium_count = self
            .metrics
            .by_severity
            .get("medium")
            .copied()
            .unwrap_or(0);

        self.assessment = if has_blocking || critical_count > 0 {
            ReviewAssessment::Fail
        } else if high_count > 0 {
            ReviewAssessment::NeedsImprovements
        } else if medium_count > 0 {
            ReviewAssessment::Warning
        } else if !self.findings.is_empty() {
            ReviewAssessment::Warning
        } else {
            ReviewAssessment::Pass
        };

        self.retry_recommended = matches!(
            self.assessment,
            ReviewAssessment::NeedsImprovements | ReviewAssessment::Fail
        );
    }

    /// Check if the report passes (no blocking issues)
    pub fn passes(&self) -> bool {
        !matches!(self.assessment, ReviewAssessment::Fail)
    }

    /// Get findings by severity
    pub fn findings_by_severity(&self, severity: ReviewSeverity) -> Vec<&ReviewFinding> {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .collect()
    }

    /// Get findings by category
    pub fn findings_by_category(&self, category: ReviewCategory) -> Vec<&ReviewFinding> {
        self.findings
            .iter()
            .filter(|f| f.category == category)
            .collect()
    }

    /// Format the report as markdown
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str(&format!("# Review Report: {}\n\n", self.task_id));
        md.push_str(&format!("**Assessment**: {}\n\n", self.assessment));
        md.push_str(&format!("**Summary**: {}\n\n", self.summary));

        if !self.strengths.is_empty() {
            md.push_str("## Strengths\n\n");
            for strength in &self.strengths {
                md.push_str(&format!("- {}\n", strength));
            }
            md.push('\n');
        }

        if !self.findings.is_empty() {
            md.push_str("## Findings\n\n");

            // Group by severity
            for severity in [
                ReviewSeverity::Critical,
                ReviewSeverity::High,
                ReviewSeverity::Medium,
                ReviewSeverity::Low,
                ReviewSeverity::Info,
            ] {
                let findings = self.findings_by_severity(severity);
                if !findings.is_empty() {
                    md.push_str(&format!(
                        "### {} ({} findings)\n\n",
                        severity.to_string().to_uppercase(),
                        findings.len()
                    ));

                    for finding in findings {
                        md.push_str(&format!(
                            "- **{}** [{}]: {}\n",
                            finding.title, finding.category, finding.description
                        ));
                        if let Some(ref file) = finding.file {
                            if let Some(line) = finding.line {
                                md.push_str(&format!("  - Location: `{}:{}`\n", file, line));
                            }
                        }
                        if let Some(ref suggestion) = finding.suggestion {
                            md.push_str(&format!("  - Suggestion: {}\n", suggestion));
                        }
                    }
                    md.push('\n');
                }
            }
        }

        if !self.recommendations.is_empty() {
            md.push_str("## Recommendations\n\n");
            for rec in &self.recommendations {
                md.push_str(&format!("- {}\n", rec));
            }
        }

        md
    }
}

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

    /// Get the severity threshold as the new ReviewSeverity type
    pub fn severity_threshold_as_review(&self) -> ReviewSeverity {
        self.severity_threshold.into()
    }
}

// =============================================================================
// Backward Compatibility - Type Aliases and Conversions
// =============================================================================

/// Alias for backward compatibility with existing code
pub type IssueSeverity = ReviewSeverity;

/// Alias for backward compatibility with existing code
pub type IssueCategory = ReviewCategory;

// =============================================================================
// Legacy Types for Backward Compatibility
// =============================================================================

/// A single code review issue (legacy type, use ReviewFinding for new code)
///
/// This type is maintained for backward compatibility. New code should use
/// [`ReviewFinding`] which provides additional fields and functionality.
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

impl CodeIssue {
    /// Convert to the new ReviewFinding type
    pub fn to_finding(&self) -> ReviewFinding {
        let mut finding = ReviewFinding::new(self.category, self.severity, &self.title)
            .with_description(&self.description);

        if let Some(ref file) = self.file {
            if let Some(line) = self.line {
                finding = finding.with_file(file, line);
            }
        }

        if let Some(ref suggestion) = self.suggestion {
            finding = finding.with_suggestion(suggestion);
        }

        if let Some(ref snippet) = self.code_snippet {
            finding = finding.with_code_snippet(snippet);
        }

        finding.blocking(self.blocking)
    }

    /// Create from a ReviewFinding
    pub fn from_finding(finding: &ReviewFinding) -> Self {
        CodeIssue {
            category: finding.category,
            severity: finding.severity,
            file: finding.file.clone(),
            line: finding.line,
            title: finding.title.clone(),
            description: finding.description.clone(),
            suggestion: finding.suggestion.clone(),
            code_snippet: finding.code_snippet.clone(),
            blocking: finding.blocking,
        }
    }
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

    // =========================================================================
    // Tests for New Data Structures (ReviewSeverity, ReviewCategory, etc.)
    // =========================================================================

    #[test]
    fn test_review_severity_ordering() {
        // Test ordering from least to most severe
        assert!(ReviewSeverity::Critical > ReviewSeverity::High);
        assert!(ReviewSeverity::High > ReviewSeverity::Medium);
        assert!(ReviewSeverity::Medium > ReviewSeverity::Low);
        assert!(ReviewSeverity::Low > ReviewSeverity::Info);

        // Test transitivity
        assert!(ReviewSeverity::Critical > ReviewSeverity::Medium);
        assert!(ReviewSeverity::High > ReviewSeverity::Info);
    }

    #[test]
    fn test_review_severity_is_blocking() {
        assert!(ReviewSeverity::Critical.is_blocking());
        assert!(!ReviewSeverity::High.is_blocking());
        assert!(!ReviewSeverity::Medium.is_blocking());
        assert!(!ReviewSeverity::Low.is_blocking());
        assert!(!ReviewSeverity::Info.is_blocking());
    }

    #[test]
    fn test_review_severity_triggers_fix_cycle() {
        // In expert mode
        assert!(ReviewSeverity::Critical.triggers_fix_cycle(true));
        assert!(ReviewSeverity::High.triggers_fix_cycle(true));
        assert!(ReviewSeverity::Medium.triggers_fix_cycle(true));
        assert!(!ReviewSeverity::Low.triggers_fix_cycle(true));
        assert!(!ReviewSeverity::Info.triggers_fix_cycle(true));

        // In standard mode
        assert!(ReviewSeverity::Critical.triggers_fix_cycle(false));
        assert!(ReviewSeverity::High.triggers_fix_cycle(false));
        assert!(!ReviewSeverity::Medium.triggers_fix_cycle(false));
        assert!(!ReviewSeverity::Low.triggers_fix_cycle(false));
        assert!(!ReviewSeverity::Info.triggers_fix_cycle(false));
    }

    #[test]
    fn test_review_severity_display() {
        assert_eq!(ReviewSeverity::Critical.to_string(), "critical");
        assert_eq!(ReviewSeverity::High.to_string(), "high");
        assert_eq!(ReviewSeverity::Medium.to_string(), "medium");
        assert_eq!(ReviewSeverity::Low.to_string(), "low");
        assert_eq!(ReviewSeverity::Info.to_string(), "info");
    }

    #[test]
    fn test_review_severity_description() {
        assert!(ReviewSeverity::Critical.description().contains("blocks"));
        assert!(ReviewSeverity::High.description().contains("Serious"));
        assert!(ReviewSeverity::Medium.description().contains("Moderate"));
        assert!(ReviewSeverity::Low.description().contains("Minor"));
        assert!(ReviewSeverity::Info.description().contains("Informational"));
    }

    #[test]
    fn test_review_category_default_severity() {
        assert_eq!(ReviewCategory::Security.default_severity(), ReviewSeverity::High);
        assert_eq!(ReviewCategory::Architecture.default_severity(), ReviewSeverity::High);
        assert_eq!(ReviewCategory::Performance.default_severity(), ReviewSeverity::Medium);
        assert_eq!(ReviewCategory::Quality.default_severity(), ReviewSeverity::Medium);
        assert_eq!(ReviewCategory::BestPractices.default_severity(), ReviewSeverity::Low);
        assert_eq!(ReviewCategory::Documentation.default_severity(), ReviewSeverity::Low);
        assert_eq!(ReviewCategory::Style.default_severity(), ReviewSeverity::Info);
    }

    #[test]
    fn test_review_category_mode_visibility() {
        // Security, ErrorHandling, Quality should always be checked
        assert!(ReviewCategory::Security.is_enabled_for_mode(false));
        assert!(ReviewCategory::ErrorHandling.is_enabled_for_mode(false));
        assert!(ReviewCategory::Quality.is_enabled_for_mode(false));

        // Architecture, Documentation, Style only in expert mode
        assert!(!ReviewCategory::Architecture.is_enabled_for_mode(false));
        assert!(ReviewCategory::Architecture.is_enabled_for_mode(true));
        assert!(!ReviewCategory::Documentation.is_enabled_for_mode(false));
        assert!(ReviewCategory::Documentation.is_enabled_for_mode(true));
        assert!(!ReviewCategory::Style.is_enabled_for_mode(false));
        assert!(ReviewCategory::Style.is_enabled_for_mode(true));
    }

    #[test]
    fn test_review_category_display() {
        assert_eq!(ReviewCategory::Security.to_string(), "security");
        assert_eq!(ReviewCategory::BestPractices.to_string(), "best_practices");
        assert_eq!(ReviewCategory::ErrorHandling.to_string(), "error_handling");
        assert_eq!(ReviewCategory::Architecture.to_string(), "architecture");
    }

    #[test]
    fn test_review_finding_creation() {
        let finding = ReviewFinding::new(
            ReviewCategory::Security,
            ReviewSeverity::High,
            "SQL Injection",
        );

        assert_eq!(finding.category, ReviewCategory::Security);
        assert_eq!(finding.severity, ReviewSeverity::High);
        assert_eq!(finding.title, "SQL Injection");
        assert!(!finding.blocking); // High is not blocking
        assert_eq!(finding.confidence, 0.8); // Default confidence
        assert!(finding.file.is_none());
        assert!(finding.line.is_none());
    }

    #[test]
    fn test_review_finding_builder_pattern() {
        let finding = ReviewFinding::new(
            ReviewCategory::Security,
            ReviewSeverity::Critical,
            "Buffer Overflow",
        )
        .with_file("src/buffer.rs", 42)
        .with_description("Potential buffer overflow in unsafe block")
        .with_suggestion("Use safe abstractions or bounds checking")
        .with_code_snippet("let ptr = buffer.as_ptr();")
        .blocking(true)
        .with_confidence(0.95)
        .with_tag("security-critical")
        .with_cwe("CWE-120");

        assert_eq!(finding.file, Some("src/buffer.rs".to_string()));
        assert_eq!(finding.line, Some(42));
        assert_eq!(finding.description, "Potential buffer overflow in unsafe block");
        assert_eq!(
            finding.suggestion,
            Some("Use safe abstractions or bounds checking".to_string())
        );
        assert!(finding.blocking);
        assert_eq!(finding.confidence, 0.95);
        assert!(finding.tags.contains(&"security-critical".to_string()));
        assert_eq!(finding.cwe_id, Some("CWE-120".to_string()));
    }

    #[test]
    fn test_review_finding_file_range() {
        let finding = ReviewFinding::new(
            ReviewCategory::Quality,
            ReviewSeverity::Medium,
            "Long function",
        )
        .with_file_range("src/main.rs", 100, 250);

        assert_eq!(finding.file, Some("src/main.rs".to_string()));
        assert_eq!(finding.line, Some(100));
        assert_eq!(finding.end_line, Some(250));
    }

    #[test]
    fn test_review_finding_severity_threshold() {
        let critical = ReviewFinding::new(
            ReviewCategory::Security,
            ReviewSeverity::Critical,
            "Critical",
        );
        let high = ReviewFinding::new(
            ReviewCategory::Security,
            ReviewSeverity::High,
            "High",
        );
        let low = ReviewFinding::new(
            ReviewCategory::Security,
            ReviewSeverity::Low,
            "Low",
        );

        // Critical meets threshold of High
        assert!(critical.meets_severity_threshold(ReviewSeverity::High));
        // High meets threshold of High
        assert!(high.meets_severity_threshold(ReviewSeverity::High));
        // Low does not meet threshold of High
        assert!(!low.meets_severity_threshold(ReviewSeverity::High));
        // Low meets threshold of Low
        assert!(low.meets_severity_threshold(ReviewSeverity::Low));
        // Low meets threshold of Info
        assert!(low.meets_severity_threshold(ReviewSeverity::Info));
    }

    #[test]
    fn test_review_finding_format() {
        let finding = ReviewFinding::new(
            ReviewCategory::Security,
            ReviewSeverity::Critical,
            "SQL Injection",
        )
        .with_file("src/db.rs", 42)
        .with_description("User input not sanitized");

        let formatted = finding.format();
        assert!(formatted.contains("SQL Injection"));
        assert!(formatted.contains("src/db.rs:42"));
        assert!(formatted.contains("User input not sanitized"));
        // Severity is displayed in uppercase per format() implementation
        assert!(formatted.contains("CRITICAL"));
    }

    #[test]
    fn test_review_report_creation() {
        let report = ReviewReport::new("task-123");

        assert_eq!(report.task_id, "task-123");
        assert_eq!(report.assessment, ReviewAssessment::Pass);
        assert!(report.findings.is_empty());
        assert!(report.strengths.is_empty());
        assert!(!report.retry_recommended);
    }

    #[test]
    fn test_review_report_add_findings() {
        let mut report = ReviewReport::new("task-123");

        let finding1 = ReviewFinding::new(
            ReviewCategory::Security,
            ReviewSeverity::High,
            "Security Issue",
        );
        let finding2 = ReviewFinding::new(
            ReviewCategory::Quality,
            ReviewSeverity::Medium,
            "Quality Issue",
        );

        report.add_finding(finding1);
        report.add_finding(finding2);

        assert_eq!(report.findings.len(), 2);
        assert_eq!(report.metrics.total_findings, 2);
        assert_eq!(report.metrics.by_severity.get("high"), Some(&1));
        assert_eq!(report.metrics.by_severity.get("medium"), Some(&1));
        assert_eq!(report.metrics.by_category.get("security"), Some(&1));
        assert_eq!(report.metrics.by_category.get("quality"), Some(&1));
    }

    #[test]
    fn test_review_report_calculate_assessment() {
        let mut report = ReviewReport::new("task-123");

        // No findings = Pass
        report.calculate_assessment();
        assert_eq!(report.assessment, ReviewAssessment::Pass);
        assert!(!report.retry_recommended);

        // Add critical finding = Fail
        report.add_finding(ReviewFinding::new(
            ReviewCategory::Security,
            ReviewSeverity::Critical,
            "Critical",
        ).blocking(true));
        report.calculate_assessment();
        assert_eq!(report.assessment, ReviewAssessment::Fail);
        assert!(report.retry_recommended);

        // Reset with high severity
        report.findings.clear();
        report.update_metrics();
        report.add_finding(ReviewFinding::new(
            ReviewCategory::Security,
            ReviewSeverity::High,
            "High",
        ));
        report.calculate_assessment();
        assert_eq!(report.assessment, ReviewAssessment::NeedsImprovements);
        assert!(report.retry_recommended);

        // Reset with medium severity
        report.findings.clear();
        report.update_metrics();
        report.add_finding(ReviewFinding::new(
            ReviewCategory::Quality,
            ReviewSeverity::Medium,
            "Medium",
        ));
        report.calculate_assessment();
        assert_eq!(report.assessment, ReviewAssessment::Warning);
        assert!(!report.retry_recommended);
    }

    #[test]
    fn test_review_report_findings_by_severity() {
        let mut report = ReviewReport::new("task-123");

        report.add_finding(ReviewFinding::new(
            ReviewCategory::Security,
            ReviewSeverity::Critical,
            "Critical 1",
        ));
        report.add_finding(ReviewFinding::new(
            ReviewCategory::Security,
            ReviewSeverity::Critical,
            "Critical 2",
        ));
        report.add_finding(ReviewFinding::new(
            ReviewCategory::Quality,
            ReviewSeverity::High,
            "High 1",
        ));
        report.add_finding(ReviewFinding::new(
            ReviewCategory::Quality,
            ReviewSeverity::Medium,
            "Medium 1",
        ));

        let critical = report.findings_by_severity(ReviewSeverity::Critical);
        let high = report.findings_by_severity(ReviewSeverity::High);
        let medium = report.findings_by_severity(ReviewSeverity::Medium);

        assert_eq!(critical.len(), 2);
        assert_eq!(high.len(), 1);
        assert_eq!(medium.len(), 1);
    }

    #[test]
    fn test_review_report_findings_by_category() {
        let mut report = ReviewReport::new("task-123");

        report.add_finding(ReviewFinding::new(
            ReviewCategory::Security,
            ReviewSeverity::High,
            "Security 1",
        ));
        report.add_finding(ReviewFinding::new(
            ReviewCategory::Security,
            ReviewSeverity::Medium,
            "Security 2",
        ));
        report.add_finding(ReviewFinding::new(
            ReviewCategory::Performance,
            ReviewSeverity::Medium,
            "Performance 1",
        ));

        let security = report.findings_by_category(ReviewCategory::Security);
        let performance = report.findings_by_category(ReviewCategory::Performance);
        let quality = report.findings_by_category(ReviewCategory::Quality);

        assert_eq!(security.len(), 2);
        assert_eq!(performance.len(), 1);
        assert_eq!(quality.len(), 0);
    }

    #[test]
    fn test_review_report_to_markdown() {
        let mut report = ReviewReport::new("task-123");
        report.summary = "Code review summary".to_string();
        report.strengths.push("Good error handling".to_string());
        report.add_finding(ReviewFinding::new(
            ReviewCategory::Security,
            ReviewSeverity::High,
            "Security Issue",
        )
        .with_file("src/main.rs", 42)
        .with_description("User input not sanitized"));

        let md = report.to_markdown();

        assert!(md.contains("# Review Report: task-123"));
        assert!(md.contains("Security Issue"));
        assert!(md.contains("src/main.rs:42"));
        assert!(md.contains("Good error handling"));
        assert!(md.contains("## Findings"));
    }

    #[test]
    fn test_review_error_is_recoverable() {
        let critical = ReviewError::CriticalIssues("Security vulnerabilities found".to_string());
        let agent_err = ReviewError::AgentError("Connection failed".to_string());
        let timeout = ReviewError::Timeout(300);
        let config_err = ReviewError::ConfigError("Invalid setting".to_string());

        assert!(critical.is_recoverable());
        assert!(!agent_err.is_recoverable());
        assert!(!timeout.is_recoverable());
        assert!(!config_err.is_recoverable());
    }

    #[test]
    fn test_review_error_is_fatal() {
        let critical = ReviewError::CriticalIssues("Issues found".to_string());
        let agent_err = ReviewError::AgentError("Connection failed".to_string());
        let timeout = ReviewError::Timeout(300);
        let parse_err = ReviewError::ParseError("Invalid JSON".to_string());

        assert!(!critical.is_fatal());
        assert!(agent_err.is_fatal());
        assert!(timeout.is_fatal());
        assert!(parse_err.is_fatal());
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
        assert_eq!(ReviewError::Timeout(100).error_code(), "REVIEW_TIMEOUT");
        assert_eq!(
            ReviewError::CriticalIssues("test".to_string()).error_code(),
            "REVIEW_CRITICAL_ISSUES"
        );
        assert_eq!(
            ReviewError::ConfigError("test".to_string()).error_code(),
            "REVIEW_CONFIG_ERROR"
        );
    }

    #[test]
    fn test_code_issue_to_finding_conversion() {
        let issue = CodeIssue {
            category: IssueCategory::Security,
            severity: IssueSeverity::High,
            file: Some("src/db.rs".to_string()),
            line: Some(42),
            title: "SQL Injection".to_string(),
            description: "Unsanitized input".to_string(),
            suggestion: Some("Use parameterized queries".to_string()),
            code_snippet: Some("query(input)".to_string()),
            blocking: true,
        };

        let finding = issue.to_finding();

        assert_eq!(finding.category, ReviewCategory::Security);
        assert_eq!(finding.severity, ReviewSeverity::High);
        assert_eq!(finding.file, Some("src/db.rs".to_string()));
        assert_eq!(finding.line, Some(42));
        assert_eq!(finding.title, "SQL Injection");
        assert_eq!(finding.description, "Unsanitized input");
        assert_eq!(finding.suggestion, Some("Use parameterized queries".to_string()));
        assert!(finding.blocking);
    }

    #[test]
    fn test_code_issue_from_finding_conversion() {
        let finding = ReviewFinding::new(
            ReviewCategory::Performance,
            ReviewSeverity::Medium,
            "Slow query",
        )
        .with_file("src/db.rs", 100)
        .with_description("Missing index")
        .blocking(false);

        let issue = CodeIssue::from_finding(&finding);

        assert_eq!(issue.category, IssueCategory::Performance);
        assert_eq!(issue.severity, IssueSeverity::Medium);
        assert_eq!(issue.file, Some("src/db.rs".to_string()));
        assert_eq!(issue.line, Some(100));
        assert_eq!(issue.title, "Slow query");
        assert_eq!(issue.description, "Missing index");
        assert!(!issue.blocking);
    }

    #[test]
    fn test_review_config_severity_threshold_conversion() {
        let config = ReviewConfig {
            severity_threshold: IssueSeverity::High,
            ..Default::default()
        };

        let threshold = config.severity_threshold_as_review();
        assert_eq!(threshold, ReviewSeverity::High);
    }

    #[test]
    fn test_type_aliases_compatibility() {
        // Test that type aliases work correctly
        let severity: IssueSeverity = ReviewSeverity::High;
        assert_eq!(severity, ReviewSeverity::High);

        let category: IssueCategory = ReviewCategory::Security;
        assert_eq!(category, ReviewCategory::Security);

        // Test that comparison works
        let s1: IssueSeverity = ReviewSeverity::Critical;
        let s2: ReviewSeverity = IssueSeverity::High;
        assert!(s1 > s2);
    }

    #[test]
    fn test_review_metrics_default() {
        let metrics = ReviewMetrics::default();

        assert_eq!(metrics.total_findings, 0);
        assert!(metrics.by_severity.is_empty());
        assert!(metrics.by_category.is_empty());
        assert_eq!(metrics.blocking_count, 0);
        assert_eq!(metrics.files_affected, 0);
        assert_eq!(metrics.avg_confidence, 0.0);
    }

    #[test]
    fn test_review_config_snapshot() {
        let snapshot = ReviewConfigSnapshot {
            severity_threshold: ReviewSeverity::Low,
            categories_checked: vec![
                ReviewCategory::Security,
                ReviewCategory::Performance,
            ],
            max_issues_per_category: 15,
            expert_mode: true,
        };

        assert_eq!(snapshot.severity_threshold, ReviewSeverity::Low);
        assert_eq!(snapshot.categories_checked.len(), 2);
        assert_eq!(snapshot.max_issues_per_category, 15);
        assert!(snapshot.expert_mode);
    }
}
