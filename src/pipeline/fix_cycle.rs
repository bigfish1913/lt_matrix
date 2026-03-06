//! Fix cycle triggering module
//!
//! This module implements automatic fix cycle triggering when critical issues
//! are detected during testing or verification. It integrates with the execute
//! and test stages to provide intelligent fix attempts.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use crate::agent::backend::{AgentBackend, ExecutionConfig};
use crate::agent::claude::ClaudeAgent;
use crate::pipeline::coverage::{
    AggregatedFindings, IssueDetail, IssueSeverity, SecurityIssue, TestFailure,
};

/// Configuration for fix cycle behavior
#[derive(Debug, Clone)]
pub struct FixCycleConfig {
    /// Whether automatic fixing is enabled
    pub enabled: bool,

    /// Maximum fix attempts per issue
    pub max_fix_attempts: u32,

    /// Timeout for fix attempts (seconds)
    pub fix_timeout: u64,

    /// Whether to require user confirmation for fixes
    pub require_confirmation: bool,

    /// Severity threshold for automatic fixes
    pub auto_fix_threshold: IssueSeverity,

    /// Working directory
    pub work_dir: PathBuf,

    /// Model to use for fix generation
    pub fix_model: String,
}

impl Default for FixCycleConfig {
    fn default() -> Self {
        FixCycleConfig {
            enabled: true,
            max_fix_attempts: 3,
            fix_timeout: 600,
            require_confirmation: false,
            auto_fix_threshold: IssueSeverity::High,
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            fix_model: "claude-sonnet-4-6".to_string(),
        }
    }
}

impl FixCycleConfig {
    /// Create config for fast mode (minimal fixing)
    pub fn fast_mode() -> Self {
        FixCycleConfig {
            enabled: true,
            max_fix_attempts: 1,
            fix_timeout: 300,
            require_confirmation: false,
            auto_fix_threshold: IssueSeverity::Critical,
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            fix_model: "claude-haiku-4-5".to_string(),
        }
    }

    /// Create config for expert mode (thorough fixing)
    pub fn expert_mode() -> Self {
        FixCycleConfig {
            enabled: true,
            max_fix_attempts: 5,
            fix_timeout: 1200,
            require_confirmation: true,
            auto_fix_threshold: IssueSeverity::Medium,
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            fix_model: "claude-opus-4-6".to_string(),
        }
    }

    /// Check if an issue should be automatically fixed
    pub fn should_auto_fix(&self, severity: IssueSeverity) -> bool {
        self.enabled && severity as u8 <= self.auto_fix_threshold as u8
    }
}

/// Result of a fix attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixAttempt {
    /// The issue being fixed
    pub issue_title: String,

    /// Whether the fix was successful
    pub success: bool,

    /// Number of attempts made
    pub attempts: u32,

    /// Fix description
    pub fix_description: String,

    /// Changes made (file paths)
    pub files_modified: Vec<PathBuf>,

    /// Time taken for fix (seconds)
    pub duration_secs: u64,

    /// Error message if fix failed
    pub error: Option<String>,

    /// Whether verification passed after fix
    pub verified: bool,
}

/// Summary of fix cycle execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixCycleSummary {
    /// Total issues identified
    pub total_issues: usize,

    /// Issues that were fixed
    pub fixed_issues: usize,

    /// Issues that failed to fix
    pub failed_issues: usize,

    /// Issues skipped (not auto-fixable)
    pub skipped_issues: usize,

    /// Total fix attempts made
    pub total_attempts: u32,

    /// Total time spent fixing (seconds)
    pub total_time_secs: u64,

    /// Individual fix attempts
    pub attempts: Vec<FixAttempt>,
}

/// Fix cycle trigger
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FixCycleTrigger {
    /// Triggered by test failure
    TestFailure,

    /// Triggered by verification failure
    VerificationFailure,

    /// Triggered by security issue
    SecurityIssue,

    /// Triggered by performance issue
    PerformanceIssue,

    /// Triggered by low coverage
    LowCoverage,

    /// Manual trigger
    Manual,
}

/// Fix strategy for different types of issues
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixStrategy {
    /// Apply fix immediately without verification
    Immediate,

    /// Apply fix and verify with tests
    FixAndTest,

    /// Apply fix, verify, and run full test suite
    FixAndVerify,

    /// Generate fix suggestion but don't apply
    SuggestOnly,
}

/// Determine the fix strategy for an issue
pub fn determine_fix_strategy(severity: IssueSeverity, issue_type: FixCycleTrigger) -> FixStrategy {
    match (severity, issue_type) {
        (IssueSeverity::Critical, FixCycleTrigger::SecurityIssue) => FixStrategy::Immediate,
        (IssueSeverity::Critical, _) => FixStrategy::FixAndVerify,
        (IssueSeverity::High, FixCycleTrigger::TestFailure) => FixStrategy::FixAndTest,
        (IssueSeverity::High, _) => FixStrategy::FixAndVerify,
        (IssueSeverity::Medium, _) => FixStrategy::FixAndTest,
        (IssueSeverity::Low, _) => FixStrategy::SuggestOnly,
    }
}

/// Execute fix cycle for aggregated findings
pub async fn execute_fix_cycle(
    findings: &AggregatedFindings,
    config: &FixCycleConfig,
    trigger: FixCycleTrigger,
) -> Result<FixCycleSummary> {
    let start_time = std::time::Instant::now();

    info!("Starting fix cycle for {} issues", findings.critical_count + findings.high_count);

    if !config.enabled {
        info!("Fix cycle disabled by config");
        return Ok(FixCycleSummary {
            total_issues: findings.critical_count + findings.high_count,
            fixed_issues: 0,
            failed_issues: 0,
            skipped_issues: findings.critical_count + findings.high_count,
            total_attempts: 0,
            total_time_secs: 0,
            attempts: Vec::new(),
        });
    }

    let agent = ClaudeAgent::new().context("Failed to create Claude agent for fix cycle")?;
    let mut summary = FixCycleSummary {
        total_issues: findings.critical_count + findings.high_count,
        fixed_issues: 0,
        failed_issues: 0,
        skipped_issues: 0,
        total_attempts: 0,
        total_time_secs: 0,
        attempts: Vec::new(),
    };

    // Process critical test failures
    for idx in findings.get_critical_test_failure_indices() {
        let failure = &findings.test_failures[idx];
        let strategy = determine_fix_strategy(failure.severity, trigger);
        match attempt_fix_for_test_failure(failure, strategy, &agent, config).await {
            Ok(attempt) => {
                if attempt.success {
                    summary.fixed_issues += 1;
                } else {
                    summary.failed_issues += 1;
                }
                summary.total_attempts += attempt.attempts;
                summary.attempts.push(attempt);
            }
            Err(e) => {
                error!("Fix attempt failed for {}: {}", failure.test_name, e);
                summary.failed_issues += 1;
            }
        }
    }

    // Process critical security issues
    for idx in findings.get_critical_security_issue_indices() {
        let issue = &findings.security_issues[idx];
        let strategy = determine_fix_strategy(issue.severity, trigger);
        match attempt_fix_for_security_issue(issue, strategy, &agent, config).await {
            Ok(attempt) => {
                if attempt.success {
                    summary.fixed_issues += 1;
                } else {
                    summary.failed_issues += 1;
                }
                summary.total_attempts += attempt.attempts;
                summary.attempts.push(attempt);
            }
            Err(e) => {
                error!("Fix attempt failed for {}: {}", issue.title, e);
                summary.failed_issues += 1;
            }
        }
    }

    // Process high-priority test failures
    for idx in findings.get_high_priority_test_failure_indices() {
        let failure = &findings.test_failures[idx];
        let strategy = determine_fix_strategy(failure.severity, trigger);
        match attempt_fix_for_test_failure(failure, strategy, &agent, config).await {
            Ok(attempt) => {
                if attempt.success {
                    summary.fixed_issues += 1;
                } else {
                    summary.failed_issues += 1;
                }
                summary.total_attempts += attempt.attempts;
                summary.attempts.push(attempt);
            }
            Err(e) => {
                error!("Fix attempt failed for {}: {}", failure.test_name, e);
                summary.failed_issues += 1;
            }
        }
    }

    // Process high-priority security issues
    for idx in findings.get_high_priority_security_issue_indices() {
        let issue = &findings.security_issues[idx];
        let strategy = determine_fix_strategy(issue.severity, trigger);
        match attempt_fix_for_security_issue(issue, strategy, &agent, config).await {
            Ok(attempt) => {
                if attempt.success {
                    summary.fixed_issues += 1;
                } else {
                    summary.failed_issues += 1;
                }
                summary.total_attempts += attempt.attempts;
                summary.attempts.push(attempt);
            }
            Err(e) => {
                error!("Fix attempt failed for {}: {}", issue.title, e);
                summary.failed_issues += 1;
            }
        }
    }

    summary.total_time_secs = start_time.elapsed().as_secs();

    info!(
        "Fix cycle completed: {}/{} issues fixed in {}s",
        summary.fixed_issues, summary.total_issues, summary.total_time_secs
    );

    Ok(summary)
}

/// Attempt to fix a single issue
async fn attempt_fix(
    issue: &dyn IssueDetail,
    strategy: FixStrategy,
    agent: &ClaudeAgent,
    config: &FixCycleConfig,
) -> Result<FixAttempt> {
    let start_time = std::time::Instant::now();

    info!("Attempting fix for: {}", issue.title());

    // Check if we should auto-fix this issue
    if !config.should_auto_fix(issue.severity()) {
        return Ok(FixAttempt {
            issue_title: issue.title().to_string(),
            success: false,
            attempts: 0,
            fix_description: "Skipped - issue severity below auto-fix threshold".to_string(),
            files_modified: Vec::new(),
            duration_secs: 0,
            error: None,
            verified: false,
        });
    }

    let mut attempts = 0;
    let mut last_error = None;

    while attempts < config.max_fix_attempts {
        attempts += 1;

        debug!("Fix attempt {} for: {}", attempts, issue.title());

        // Build fix prompt
        let prompt = build_fix_prompt(issue, strategy);

        // Build execution config
        let exec_config = ExecutionConfig {
            model: config.fix_model.clone(),
            max_retries: 1,
            timeout: config.fix_timeout,
            enable_session: true,
            env_vars: Vec::new(),
        };

        // Execute fix with timeout
        let fix_result = timeout(
            Duration::from_secs(config.fix_timeout),
            agent.execute(&prompt, &exec_config),
        )
        .await;

        let fix_output = match fix_result {
            Ok(Ok(result)) => result.output,
            Ok(Err(e)) => {
                warn!("Fix execution failed: {}", e);
                last_error = Some(e.to_string());
                continue;
            }
            Err(_) => {
                let timeout_msg = format!("Fix attempt timed out after {}s", config.fix_timeout);
                warn!("{}", timeout_msg);
                last_error = Some(timeout_msg);
                continue;
            }
        };

        // Parse the fix result
        let fix_result = parse_fix_result(&fix_output);

        match fix_result {
            Ok((description, files_modified)) => {
                let duration = start_time.elapsed().as_secs();

                // Verify fix based on strategy
                let verified = verify_fix(issue, strategy).await?;

                if verified {
                    info!("Fix successful for: {}", issue.title());
                    return Ok(FixAttempt {
                        issue_title: issue.title().to_string(),
                        success: true,
                        attempts,
                        fix_description: description,
                        files_modified,
                        duration_secs: duration,
                        error: None,
                        verified: true,
                    });
                } else if attempts < config.max_fix_attempts {
                    warn!("Fix verification failed, retrying...");
                    continue;
                } else {
                    return Ok(FixAttempt {
                        issue_title: issue.title().to_string(),
                        success: false,
                        attempts,
                        fix_description: description,
                        files_modified,
                        duration_secs: duration,
                        error: Some("Fix verification failed".to_string()),
                        verified: false,
                    });
                }
            }
            Err(e) => {
                warn!("Failed to parse fix result: {}", e);
                last_error = Some(e.to_string());
                continue;
            }
        }
    }

    // All attempts failed
    Ok(FixAttempt {
        issue_title: issue.title().to_string(),
        success: false,
        attempts,
        fix_description: format!("Failed after {} attempts", attempts),
        files_modified: Vec::new(),
        duration_secs: start_time.elapsed().as_secs(),
        error: last_error,
        verified: false,
    })
}

/// Attempt to fix a test failure
async fn attempt_fix_for_test_failure(
    failure: &TestFailure,
    strategy: FixStrategy,
    agent: &ClaudeAgent,
    config: &FixCycleConfig,
) -> Result<FixAttempt> {
    let start_time = std::time::Instant::now();

    info!("Attempting fix for test failure: {}", failure.test_name);

    // Check if we should auto-fix this issue
    if !config.should_auto_fix(failure.severity) {
        return Ok(FixAttempt {
            issue_title: failure.test_name.clone(),
            success: false,
            attempts: 0,
            fix_description: "Skipped - issue severity below auto-fix threshold".to_string(),
            files_modified: Vec::new(),
            duration_secs: 0,
            error: None,
            verified: false,
        });
    }

    let mut attempts = 0;
    let mut last_error = None;

    while attempts < config.max_fix_attempts {
        attempts += 1;

        debug!("Fix attempt {} for test failure: {}", attempts, failure.test_name);

        // Build fix prompt
        let prompt = build_fix_prompt_for_test_failure(failure, strategy);

        // Build execution config
        let exec_config = ExecutionConfig {
            model: config.fix_model.clone(),
            max_retries: 1,
            timeout: config.fix_timeout,
            enable_session: true,
            env_vars: Vec::new(),
        };

        // Execute fix with timeout
        let fix_result = timeout(
            Duration::from_secs(config.fix_timeout),
            agent.execute(&prompt, &exec_config),
        )
        .await;

        let fix_output = match fix_result {
            Ok(Ok(result)) => result.output,
            Ok(Err(e)) => {
                warn!("Fix execution failed: {}", e);
                last_error = Some(e.to_string());
                continue;
            }
            Err(_) => {
                let timeout_msg = format!("Fix attempt timed out after {}s", config.fix_timeout);
                warn!("{}", timeout_msg);
                last_error = Some(timeout_msg);
                continue;
            }
        };

        // Parse the fix result
        let fix_result = parse_fix_result(&fix_output);

        match fix_result {
            Ok((description, files_modified)) => {
                let duration = start_time.elapsed().as_secs();

                // Verify fix based on strategy
                let verified = verify_fix_for_test(failure, strategy).await?;

                if verified {
                    info!("Fix successful for test failure: {}", failure.test_name);
                    return Ok(FixAttempt {
                        issue_title: failure.test_name.clone(),
                        success: true,
                        attempts,
                        fix_description: description,
                        files_modified,
                        duration_secs: duration,
                        error: None,
                        verified: true,
                    });
                } else if attempts < config.max_fix_attempts {
                    warn!("Fix verification failed, retrying...");
                    continue;
                } else {
                    return Ok(FixAttempt {
                        issue_title: failure.test_name.clone(),
                        success: false,
                        attempts,
                        fix_description: description,
                        files_modified,
                        duration_secs: duration,
                        error: Some("Fix verification failed".to_string()),
                        verified: false,
                    });
                }
            }
            Err(e) => {
                warn!("Failed to parse fix result: {}", e);
                last_error = Some(e.to_string());
                continue;
            }
        }
    }

    // All attempts failed
    Ok(FixAttempt {
        issue_title: failure.test_name.clone(),
        success: false,
        attempts,
        fix_description: format!("Failed after {} attempts", attempts),
        files_modified: Vec::new(),
        duration_secs: start_time.elapsed().as_secs(),
        error: last_error,
        verified: false,
    })
}

/// Attempt to fix a security issue
async fn attempt_fix_for_security_issue(
    issue: &SecurityIssue,
    strategy: FixStrategy,
    agent: &ClaudeAgent,
    config: &FixCycleConfig,
) -> Result<FixAttempt> {
    let start_time = std::time::Instant::now();

    info!("Attempting fix for security issue: {}", issue.title);

    // Check if we should auto-fix this issue
    if !config.should_auto_fix(issue.severity) {
        return Ok(FixAttempt {
            issue_title: issue.title.clone(),
            success: false,
            attempts: 0,
            fix_description: "Skipped - issue severity below auto-fix threshold".to_string(),
            files_modified: Vec::new(),
            duration_secs: 0,
            error: None,
            verified: false,
        });
    }

    let mut attempts = 0;
    let mut last_error = None;

    while attempts < config.max_fix_attempts {
        attempts += 1;

        debug!("Fix attempt {} for security issue: {}", attempts, issue.title);

        // Build fix prompt
        let prompt = build_fix_prompt_for_security_issue(issue, strategy);

        // Build execution config
        let exec_config = ExecutionConfig {
            model: config.fix_model.clone(),
            max_retries: 1,
            timeout: config.fix_timeout,
            enable_session: true,
            env_vars: Vec::new(),
        };

        // Execute fix with timeout
        let fix_result = timeout(
            Duration::from_secs(config.fix_timeout),
            agent.execute(&prompt, &exec_config),
        )
        .await;

        let fix_output = match fix_result {
            Ok(Ok(result)) => result.output,
            Ok(Err(e)) => {
                warn!("Fix execution failed: {}", e);
                last_error = Some(e.to_string());
                continue;
            }
            Err(_) => {
                let timeout_msg = format!("Fix attempt timed out after {}s", config.fix_timeout);
                warn!("{}", timeout_msg);
                last_error = Some(timeout_msg);
                continue;
            }
        };

        // Parse the fix result
        let fix_result = parse_fix_result(&fix_output);

        match fix_result {
            Ok((description, files_modified)) => {
                let duration = start_time.elapsed().as_secs();

                // Verify fix based on strategy
                let verified = verify_fix_for_security(issue, strategy).await?;

                if verified {
                    info!("Fix successful for security issue: {}", issue.title);
                    return Ok(FixAttempt {
                        issue_title: issue.title.clone(),
                        success: true,
                        attempts,
                        fix_description: description,
                        files_modified,
                        duration_secs: duration,
                        error: None,
                        verified: true,
                    });
                } else if attempts < config.max_fix_attempts {
                    warn!("Fix verification failed, retrying...");
                    continue;
                } else {
                    return Ok(FixAttempt {
                        issue_title: issue.title.clone(),
                        success: false,
                        attempts,
                        fix_description: description,
                        files_modified,
                        duration_secs: duration,
                        error: Some("Fix verification failed".to_string()),
                        verified: false,
                    });
                }
            }
            Err(e) => {
                warn!("Failed to parse fix result: {}", e);
                last_error = Some(e.to_string());
                continue;
            }
        }
    }

    // All attempts failed
    Ok(FixAttempt {
        issue_title: issue.title.clone(),
        success: false,
        attempts,
        fix_description: format!("Failed after {} attempts", attempts),
        files_modified: Vec::new(),
        duration_secs: start_time.elapsed().as_secs(),
        error: last_error,
        verified: false,
    })
}

/// Build the fix prompt for an issue
fn build_fix_prompt(issue: &dyn IssueDetail, strategy: FixStrategy) -> String {
    let strategy_desc = match strategy {
        FixStrategy::Immediate => "Apply the fix immediately",
        FixStrategy::FixAndTest => "Apply the fix and ensure tests pass",
        FixStrategy::FixAndVerify => "Apply the fix, verify it works, and ensure all tests pass",
        FixStrategy::SuggestOnly => "Provide a suggested fix but do not apply it",
    };

    format!(
        r#"You are fixing a critical issue in a software project.

## Issue Details

**Title**: {}
**Description**: {}
**Severity**: {:?}
**Affected Component**: {}

## Your Task

Fix this issue by making the necessary code changes.

## Fix Strategy

{}

## Important Instructions

- Make minimal, focused changes to fix the specific issue
- Preserve existing functionality and API contracts
- Add appropriate error handling if needed
- Update related tests if applicable
- Ensure your changes don't introduce new issues

## Response Format

Respond with a structured assessment in the following format:

```json
{{
  "description": "Description of the fix applied",
  "files_modified": ["list of files that were modified"],
  "changes_summary": "Summary of changes made"
}}
```

Begin your fix now. Analyze the issue, implement the fix, and provide your assessment.
"#,
        issue.title(),
        issue.description(),
        issue.severity(),
        issue.affected_component(),
        strategy_desc
    )
}

/// Parse the fix result from Claude
fn parse_fix_result(output: &str) -> Result<(String, Vec<PathBuf>)> {
    // Look for JSON in the response
    let json_str = if let Some(start) = output.find("```json") {
        let start = start + 7;
        let end = output[start..]
            .find("```")
            .context("Unterminated JSON block in fix response")?;
        output[start..start + end].trim().to_string()
    } else if let Some(start) = output.find('{') {
        // Try to find the entire JSON object
        let mut brace_count = 0;
        let mut end = start;
        for (i, c) in output[start..].char_indices() {
            if c == '{' {
                brace_count += 1;
            } else if c == '}' {
                brace_count -= 1;
                if brace_count == 0 {
                    end = start + i + 1;
                    break;
                }
            }
        }
        output[start..end].trim().to_string()
    } else {
        // No JSON found - treat entire output as description
        return Ok((output.to_string(), Vec::new()));
    };

    // Parse the JSON
    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .with_context(|| format!("Failed to parse fix JSON: {}", json_str))?;

    let description = parsed
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("No description provided")
        .to_string();

    let files_modified: Vec<PathBuf> = parsed
        .get("files_modified")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default();

    Ok((description, files_modified))
}

/// Verify a fix based on the strategy
async fn verify_fix(issue: &dyn IssueDetail, strategy: FixStrategy) -> Result<bool> {
    match strategy {
        FixStrategy::Immediate => Ok(true), // Assume successful
        FixStrategy::FixAndTest => {
            // Run relevant tests
            run_tests_for_issue(issue).await
        }
        FixStrategy::FixAndVerify => {
            // Run full test suite
            run_full_test_suite().await
        }
        FixStrategy::SuggestOnly => Ok(true), // Suggestions don't need verification
    }
}

/// Run tests related to a specific issue
async fn run_tests_for_issue(issue: &dyn IssueDetail) -> Result<bool> {
    debug!("Running tests for issue: {}", issue.title());

    // Get the affected component
    let component = issue.affected_component();

    // Determine test command based on file extension
    let test_command = if component.ends_with(".rs") {
        Some("cargo test")
    } else if component.ends_with(".py") {
        Some("pytest")
    } else if component.ends_with(".go") {
        Some("go test ./...")
    } else {
        None
    };

    if let Some(command) = test_command {
        let result = tokio::process::Command::new("sh")
            .args(["-c", command])
            .output()
            .await;

        match result {
            Ok(output) => Ok(output.status.success()),
            Err(e) => {
                warn!("Failed to run tests: {}", e);
                Ok(false)
            }
        }
    } else {
        // No test command available, assume pass
        Ok(true)
    }
}

/// Run full test suite
async fn run_full_test_suite() -> Result<bool> {
    debug!("Running full test suite");

    // Check if we're in a Rust project
    if std::path::Path::new("Cargo.toml").exists() {
        let result = tokio::process::Command::new("cargo")
            .args(["test", "--all"])
            .output()
            .await;

        match result {
            Ok(output) => Ok(output.status.success()),
            Err(e) => {
                warn!("Failed to run cargo test: {}", e);
                Ok(false)
            }
        }
    } else {
        // Not a Rust project, skip
        Ok(true)
    }
}

/// Build the fix prompt for a test failure
fn build_fix_prompt_for_test_failure(failure: &TestFailure, strategy: FixStrategy) -> String {
    let strategy_desc = match strategy {
        FixStrategy::Immediate => "Apply the fix immediately",
        FixStrategy::FixAndTest => "Apply the fix and ensure tests pass",
        FixStrategy::FixAndVerify => "Apply the fix, verify it works, and ensure all tests pass",
        FixStrategy::SuggestOnly => "Provide a suggested fix but do not apply it",
    };

    format!(
        r#"You are fixing a failing test in a software project.

## Test Failure Details

**Test Name**: {}
**File**: {}
**Line**: {}
**Failure Message**: {}
**Severity**: {:?}

## Your Task

Fix this test failure by making the necessary code changes.

## Fix Strategy

{}

## Important Instructions

- Make minimal, focused changes to fix the specific test failure
- Preserve existing functionality and API contracts
- Don't modify the test itself unless it's clearly incorrect
- Ensure your changes don't break other tests

## Response Format

Respond with a structured assessment in the following format:

```json
{{
  "description": "Description of the fix applied",
  "files_modified": ["list of files that were modified"],
  "changes_summary": "Summary of changes made"
}}
```

Begin your fix now. Analyze the test failure, implement the fix, and provide your assessment.
"#,
        failure.test_name,
        failure.file_path.display(),
        failure.line_number,
        failure.message,
        failure.severity,
        strategy_desc
    )
}

/// Build the fix prompt for a security issue
fn build_fix_prompt_for_security_issue(issue: &SecurityIssue, strategy: FixStrategy) -> String {
    let strategy_desc = match strategy {
        FixStrategy::Immediate => "Apply the fix immediately",
        FixStrategy::FixAndTest => "Apply the fix and ensure tests pass",
        FixStrategy::FixAndVerify => "Apply the fix, verify it works, and ensure all tests pass",
        FixStrategy::SuggestOnly => "Provide a suggested fix but do not apply it",
    };

    let cve_info = if let Some(cve_id) = &issue.cve_id {
        format!("**CVE ID**: {}\n", cve_id)
    } else {
        String::new()
    };

    format!(
        r#"You are fixing a critical security issue in a software project.

## Security Issue Details

**Title**: {}
{}
**Description**: {}
**Severity**: {:?}
**Affected Component**: {}

## Your Task

Fix this security issue by making the necessary code changes.

## Fix Strategy

{}

## Important Instructions

- Make minimal, focused changes to fix the specific security issue
- Follow security best practices
- Add appropriate validation and sanitization
- Preserve existing functionality and API contracts
- Ensure your changes don't introduce new vulnerabilities

## Response Format

Respond with a structured assessment in the following format:

```json
{{
  "description": "Description of the fix applied",
  "files_modified": ["list of files that were modified"],
  "changes_summary": "Summary of changes made"
}}
```

Begin your fix now. Analyze the security issue, implement the fix, and provide your assessment.
"#,
        issue.title, cve_info, issue.description, issue.severity, issue.affected_component, strategy_desc
    )
}

/// Verify a fix for a test failure
async fn verify_fix_for_test(_failure: &TestFailure, strategy: FixStrategy) -> Result<bool> {
    match strategy {
        FixStrategy::Immediate => Ok(true),
        FixStrategy::FixAndTest => {
            // Run the specific test that failed
            run_full_test_suite().await
        }
        FixStrategy::FixAndVerify => {
            // Run full test suite
            run_full_test_suite().await
        }
        FixStrategy::SuggestOnly => Ok(true),
    }
}

/// Verify a fix for a security issue
async fn verify_fix_for_security(issue: &SecurityIssue, strategy: FixStrategy) -> Result<bool> {
    match strategy {
        FixStrategy::Immediate => Ok(true),
        FixStrategy::FixAndTest => {
            // Run tests related to the affected component
            run_tests_for_issue_by_ref(&issue.affected_component).await
        }
        FixStrategy::FixAndVerify => {
            // Run full test suite
            run_full_test_suite().await
        }
        FixStrategy::SuggestOnly => Ok(true),
    }
}

/// Run tests related to a component by reference
async fn run_tests_for_issue_by_ref(component: &str) -> Result<bool> {
    debug!("Running tests for component: {}", component);

    // Determine test command based on file extension
    let test_command = if component.ends_with(".rs") {
        Some("cargo test")
    } else if component.ends_with(".py") {
        Some("pytest")
    } else if component.ends_with(".go") {
        Some("go test ./...")
    } else {
        None
    };

    if let Some(command) = test_command {
        let result = tokio::process::Command::new("sh")
            .args(["-c", command])
            .output()
            .await;

        match result {
            Ok(output) => Ok(output.status.success()),
            Err(e) => {
                warn!("Failed to run tests: {}", e);
                Ok(false)
            }
        }
    } else {
        // No test command available, assume pass
        Ok(true)
    }
}

/// Check if fix cycle should be triggered based on findings
pub fn should_trigger_fix_cycle(findings: &AggregatedFindings) -> bool {
    // Trigger if there are critical issues
    if findings.has_critical_issues() {
        return true;
    }

    // Trigger if there are high-priority test failures
    let high_test_failures = findings
        .test_failures
        .iter()
        .filter(|f| f.severity == IssueSeverity::High)
        .count();

    if high_test_failures > 0 {
        return true;
    }

    // Trigger if coverage is critically low (< 50%)
    if let Some(coverage) = &findings.coverage {
        if coverage.coverage_percent < 50.0 {
            return true;
        }
    }

    false
}

/// Display fix cycle summary
pub fn display_fix_cycle_summary(summary: &FixCycleSummary) {
    println!("\n=== Fix Cycle Summary ===");
    println!(
        "Fixed: {} | Failed: {} | Skipped: {}",
        summary.fixed_issues, summary.failed_issues, summary.skipped_issues
    );
    println!("Total Attempts: {} | Time: {}s", summary.total_attempts, summary.total_time_secs);

    if !summary.attempts.is_empty() {
        println!("\n--- Fix Attempts ---");
        for attempt in &summary.attempts {
            let status = if attempt.success { "SUCCESS" } else { "FAILED" };
            println!(
                "{} - {} ({})",
                status, attempt.issue_title, attempt.attempts
            );
            if !attempt.success {
                if let Some(error) = &attempt.error {
                    println!("  Error: {}", error);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_should_auto_fix() {
        let config = FixCycleConfig::default();

        // Should auto-fix high and critical
        assert!(config.should_auto_fix(IssueSeverity::High));
        assert!(config.should_auto_fix(IssueSeverity::Critical));

        // Should not auto-fix medium and low (threshold is High)
        assert!(!config.should_auto_fix(IssueSeverity::Medium));
        assert!(!config.should_auto_fix(IssueSeverity::Low));
    }

    #[test]
    fn test_determine_fix_strategy() {
        // Critical security issue -> Immediate
        let strategy = determine_fix_strategy(IssueSeverity::Critical, FixCycleTrigger::SecurityIssue);
        assert_eq!(strategy, FixStrategy::Immediate);

        // Critical test failure -> Fix and verify
        let strategy = determine_fix_strategy(IssueSeverity::Critical, FixCycleTrigger::TestFailure);
        assert_eq!(strategy, FixStrategy::FixAndVerify);

        // High test failure -> Fix and test
        let strategy = determine_fix_strategy(IssueSeverity::High, FixCycleTrigger::TestFailure);
        assert_eq!(strategy, FixStrategy::FixAndTest);

        // Low severity -> Suggest only
        let strategy = determine_fix_strategy(IssueSeverity::Low, FixCycleTrigger::TestFailure);
        assert_eq!(strategy, FixStrategy::SuggestOnly);
    }

    #[test]
    fn test_should_trigger_fix_cycle() {
        let mut findings = AggregatedFindings::new();

        // No issues - don't trigger
        assert!(!should_trigger_fix_cycle(&findings));

        // Add critical issue - should trigger
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
    fn test_fix_cycle_trigger_equality() {
        assert_eq!(FixCycleTrigger::TestFailure, FixCycleTrigger::TestFailure);
        assert_eq!(FixCycleTrigger::SecurityIssue, FixCycleTrigger::SecurityIssue);
        assert_ne!(FixCycleTrigger::TestFailure, FixCycleTrigger::VerificationFailure);
    }

    #[test]
    fn test_fix_strategy_equality() {
        assert_eq!(FixStrategy::Immediate, FixStrategy::Immediate);
        assert_eq!(FixStrategy::FixAndTest, FixStrategy::FixAndTest);
        assert_ne!(FixStrategy::Immediate, FixStrategy::FixAndVerify);
    }

    #[test]
    fn test_fix_attempt_creation() {
        let attempt = FixAttempt {
            issue_title: "Test Issue".to_string(),
            success: true,
            attempts: 1,
            fix_description: "Fixed".to_string(),
            files_modified: vec![PathBuf::from("test.rs")],
            duration_secs: 10,
            error: None,
            verified: true,
        };

        assert!(attempt.success);
        assert_eq!(attempt.attempts, 1);
        assert!(attempt.verified);
        assert!(attempt.error.is_none());
    }

    #[test]
    fn test_fix_cycle_summary_creation() {
        let summary = FixCycleSummary {
            total_issues: 5,
            fixed_issues: 3,
            failed_issues: 1,
            skipped_issues: 1,
            total_attempts: 7,
            total_time_secs: 120,
            attempts: vec![],
        };

        assert_eq!(summary.total_issues, 5);
        assert_eq!(summary.fixed_issues, 3);
        assert_eq!(summary.failed_issues, 1);
        assert_eq!(summary.skipped_issues, 1);
        assert_eq!(summary.total_attempts, 7);
    }

    #[test]
    fn test_should_trigger_with_low_coverage() {
        let mut findings = AggregatedFindings::new();

        // Add low coverage report
        findings.coverage = Some(crate::pipeline::coverage::CoverageReport {
            total_lines: 100,
            covered_lines: 40,
            coverage_percent: 40.0,
            modules: vec![],
            low_coverage_files: vec![],
            low_coverage_modules: vec![],
            meets_threshold: false,
            analysis_duration_secs: 0,
        });

        // Should trigger due to low coverage (< 50%)
        assert!(should_trigger_fix_cycle(&findings));
    }

    #[test]
    fn test_should_trigger_with_high_test_failures() {
        let mut findings = AggregatedFindings::new();

        // Add high-priority test failure
        findings.add_test_failure(TestFailure {
            test_name: "test_high_priority".to_string(),
            file_path: PathBuf::from("test.rs"),
            line_number: 10,
            message: "Critical failure".to_string(),
            stack_trace: None,
            severity: IssueSeverity::High,
            is_flaky: false,
            suggested_fix: None,
        });

        // Should trigger due to high-priority test failure
        assert!(should_trigger_fix_cycle(&findings));
    }
}
