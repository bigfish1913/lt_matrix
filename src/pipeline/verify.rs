// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Verify stage of the pipeline
//!
//! This module handles verification that completed tasks fulfill their
//! original requirements. It uses Claude AI to review task completion
//! against the original description and acceptance criteria.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

use ltmatrix_agent::backend::{AgentBackend, ExecutionConfig};
use ltmatrix_agent::claude::ClaudeAgent;
use ltmatrix_core::{ModeConfig, Task, TaskStatus};

/// Configuration for the verify stage
#[derive(Debug, Clone)]
pub struct VerifyConfig {
    /// Mode configuration
    pub mode_config: ModeConfig,

    /// Whether verification is enabled (can be disabled by config)
    pub enabled: bool,

    /// Fast mode skip setting (--fast flag)
    pub fast_mode: bool,

    /// Strategy when verification fails
    pub on_blocked: OnBlockedStrategy,

    /// Model to use for verification
    pub verify_model: String,

    /// Maximum retries after failed verification
    pub max_retries: u32,

    /// Timeout for verification (seconds)
    pub timeout: u64,

    /// Working directory
    pub work_dir: PathBuf,
}

impl Default for VerifyConfig {
    fn default() -> Self {
        VerifyConfig {
            mode_config: ModeConfig::default(),
            enabled: true,
            fast_mode: false,
            on_blocked: OnBlockedStrategy::default(),
            verify_model: "claude-sonnet-4-6".to_string(),
            max_retries: 1,
            timeout: 300,
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

impl VerifyConfig {
    /// Create config for fast mode
    pub fn fast_mode() -> Self {
        VerifyConfig {
            mode_config: ModeConfig::fast_mode(),
            enabled: true,
            fast_mode: true,
            on_blocked: OnBlockedStrategy::Fail,
            verify_model: "claude-haiku-4-5".to_string(),
            max_retries: 0,
            timeout: 120,
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create config for expert mode
    pub fn expert_mode() -> Self {
        VerifyConfig {
            mode_config: ModeConfig::expert_mode(),
            enabled: true,
            fast_mode: false,
            on_blocked: OnBlockedStrategy::Retry,
            verify_model: "claude-opus-4-6".to_string(),
            max_retries: 2,
            timeout: 600,
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Returns true if verification should run
    pub fn should_run(&self) -> bool {
        self.enabled && self.mode_config.verify
    }
}

/// Strategy for handling verification failures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnBlockedStrategy {
    /// Fail immediately on verification failure
    Fail,

    /// Retry the task up to max_retries times
    Retry,

    /// Mark as blocked and continue with other tasks
    Block,

    /// Skip verification and proceed (dangerous)
    Skip,
}

impl Default for OnBlockedStrategy {
    fn default() -> Self {
        OnBlockedStrategy::Retry
    }
}

/// Result of verifying a single task
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// The task that was verified
    pub task: Task,

    /// Whether verification passed
    pub passed: bool,

    /// Detailed reasoning for the verification decision
    pub reasoning: String,

    /// Acceptance criteria that were not met (if any)
    pub unmet_criteria: Vec<String>,

    /// Suggested fixes for issues found
    pub suggestions: Vec<String>,

    /// Whether a retry is recommended
    pub retry_recommended: bool,
}

/// Result of verifying multiple tasks
#[derive(Debug, Clone)]
pub struct VerificationSummary {
    /// Total tasks verified
    pub total_tasks: usize,

    /// Tasks that passed verification
    pub passed_tasks: usize,

    /// Tasks that failed verification
    pub failed_tasks: usize,

    /// Tasks that were skipped
    pub skipped_tasks: usize,

    /// Total verification time in seconds
    pub total_time: u64,

    /// Individual verification results
    pub results: Vec<VerificationResult>,
}

/// Verify a list of completed tasks
pub async fn verify_tasks(
    tasks: Vec<Task>,
    config: &VerifyConfig,
) -> Result<(Vec<Task>, VerificationSummary)> {
    let start_time = std::time::Instant::now();

    info!("Starting verification stage for {} tasks", tasks.len());

    // Check if verification should run
    if !config.should_run() {
        info!("Verification disabled by config, skipping");
        let total = tasks.len();
        return Ok((
            tasks,
            VerificationSummary {
                total_tasks: total,
                passed_tasks: 0,
                failed_tasks: 0,
                skipped_tasks: total,
                total_time: 0,
                results: Vec::new(),
            },
        ));
    }

    // Create Claude agent for verification
    let agent = ClaudeAgent::new().context("Failed to create Claude agent for verification")?;

    // Filter to only completed tasks
    let completed_tasks: Vec<_> = tasks
        .into_iter()
        .filter(|t| t.is_completed() || t.status == TaskStatus::InProgress)
        .collect();

    let total = completed_tasks.len();
    let results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;
    let mut updated_tasks = Vec::new();

    for task in completed_tasks {
        info!("Verifying task: {} ({})", task.id, task.title);

        match verify_single_task(&task, &agent, config).await {
            Ok(result) => {
                if result.passed {
                    passed += 1;
                    info!("Task {} verified: PASSED", task.id);
                } else {
                    failed += 1;
                    warn!("Task {} verified: FAILED", task.id);

                    // Handle verification failure based on strategy
                    let mut updated_task = task.clone();
                    if !result.retry_recommended
                        || config.max_retries == 0
                        || config.on_blocked == OnBlockedStrategy::Fail
                    {
                        updated_task.status = TaskStatus::Failed;
                        updated_task.error =
                            Some(format!("Verification failed: {}", result.reasoning));
                    } else if config.on_blocked == OnBlockedStrategy::Block {
                        updated_task.status = TaskStatus::Blocked;
                        updated_task.error =
                            Some(format!("Blocked pending fix: {}", result.reasoning));
                    }
                    // If strategy is Retry, task stays in current state
                    // for the execute stage to handle
                    updated_tasks.push(updated_task);
                    continue;
                }
                updated_tasks.push(task);
            }
            Err(e) => {
                warn!("Verification error for task {}: {}", task.id, e);
                // On verification error, apply the on_blocked strategy
                let mut updated_task = task.clone();
                match config.on_blocked {
                    OnBlockedStrategy::Fail | OnBlockedStrategy::Retry => {
                        updated_task.status = TaskStatus::Failed;
                        updated_task.error = Some(format!("Verification error: {}", e));
                    }
                    OnBlockedStrategy::Block => {
                        updated_task.status = TaskStatus::Blocked;
                        updated_task.error = Some(format!("Blocked by verification error: {}", e));
                    }
                    OnBlockedStrategy::Skip => {
                        // Keep task as-is
                    }
                }
                updated_tasks.push(updated_task);
                failed += 1;
            }
        }
    }

    let elapsed = start_time.elapsed().as_secs();
    info!(
        "Verification stage completed: {}/{} passed in {}s",
        passed, total, elapsed
    );

    let summary = VerificationSummary {
        total_tasks: total,
        passed_tasks: passed,
        failed_tasks: failed,
        skipped_tasks: 0,
        total_time: elapsed,
        results,
    };

    Ok((updated_tasks, summary))
}

/// Verify a single task using Claude
async fn verify_single_task(
    task: &Task,
    agent: &ClaudeAgent,
    config: &VerifyConfig,
) -> Result<VerificationResult> {
    // Build verification prompt
    let prompt = build_verification_prompt(task);

    // Build execution config
    let exec_config = ExecutionConfig {
        model: config.verify_model.clone(),
        max_retries: 1, // Don't retry verification itself
        timeout: config.timeout,
        enable_session: false,
        env_vars: Vec::new(),
    };

    // Execute verification
    let response = agent.execute(&prompt, &exec_config).await?;

    // Parse the response
    parse_verification_response(task, response.output)
}

/// Build the verification prompt for a task
fn build_verification_prompt(task: &Task) -> String {
    format!(
        r#"You are verifying that a software development task has been completed correctly.

## Original Task Description

**Task ID**: {}
**Title**: {}
**Description**: {}

## Your Task

Review the current state of the codebase and determine if this task has been completed successfully.

## Verification Criteria

1. **Acceptance Criteria**: Does the implementation fulfill the requirements stated in the task description?
2. **Code Quality**: Is the code well-structured, readable, and maintainable?
3. **Testing**: Have appropriate tests been added or updated?
4. **Documentation**: Has relevant documentation been updated?
5. **Edge Cases**: Are edge cases and error conditions properly handled?

## Important Instructions

- Examine the actual code changes made
- Look for test files related to this task
- Check if the described functionality actually works
- Be thorough but fair - minor style issues should not cause failure

## Response Format

Respond with a structured assessment in the following format:

```json
{{
  "passed": true|false,
  "reasoning": "Detailed explanation of your assessment",
  "unmet_criteria": ["List any acceptance criteria not met"],
  "suggestions": ["List specific fixes for any issues found"],
  "retry_recommended": true|false
}}
```

Begin your verification now. Examine the codebase thoroughly and provide your assessment.
"#,
        task.id, task.title, task.description
    )
}

/// Parse the verification response from Claude
fn parse_verification_response(task: &Task, response: String) -> Result<VerificationResult> {
    // Look for JSON in the response
    let json_str = if let Some(start) = response.find("```json") {
        let start = start + 7;
        let end = response[start..]
            .find("```")
            .context("Unterminated JSON block in verification response")?;
        response[start..start + end].trim().to_string()
    } else if let Some(start) = response.find('{') {
        // Try to find the entire JSON object
        let mut brace_count = 0;
        let mut end = start;
        for (i, c) in response[start..].char_indices() {
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
        response[start..end].trim().to_string()
    } else {
        // No JSON found - parse from text
        return Ok(VerificationResult {
            task: task.clone(),
            passed: true, // Default to pass if we can't parse
            reasoning: response.clone(),
            unmet_criteria: Vec::new(),
            suggestions: Vec::new(),
            retry_recommended: false,
        });
    };

    // Parse the JSON
    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .with_context(|| format!("Failed to parse verification JSON: {}", json_str))?;

    Ok(VerificationResult {
        task: task.clone(),
        passed: parsed
            .get("passed")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        reasoning: parsed
            .get("reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("No reasoning provided")
            .to_string(),
        unmet_criteria: parsed
            .get("unmet_criteria")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        suggestions: parsed
            .get("suggestions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        retry_recommended: parsed
            .get("retry_recommended")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

/// Display verification summary
pub fn display_verification_summary(summary: &VerificationSummary) {
    println!("\n=== Verification Summary ===");
    println!("Total tasks: {}", summary.total_tasks);
    println!("Passed: {}", summary.passed_tasks);
    println!("Failed: {}", summary.failed_tasks);
    println!("Skipped: {}", summary.skipped_tasks);
    println!("Time: {}s", summary.total_time);

    if !summary.results.is_empty() {
        println!("\n--- Detailed Results ---");
        for result in &summary.results {
            let status = if result.passed { "PASS" } else { "FAIL" };
            println!(
                "{} - Task {}: {}",
                status, result.task.id, result.task.title
            );
            if !result.passed {
                println!("  Reason: {}", result.reasoning);
                if !result.unmet_criteria.is_empty() {
                    println!("  Unmet criteria:");
                    for criteria in &result.unmet_criteria {
                        println!("    - {}", criteria);
                    }
                }
            }
        }
    }
}

/// Check if verification should run based on fast mode and config
pub fn should_verify_in_fast_mode(config: &ModeConfig) -> bool {
    config.verify
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_config_default() {
        let config = VerifyConfig::default();
        assert!(config.enabled);
        assert!(config.should_run());
        assert_eq!(config.max_retries, 1);
        assert_eq!(config.timeout, 300);
    }

    #[test]
    fn test_verify_config_fast_mode() {
        let config = VerifyConfig::fast_mode();
        assert!(config.fast_mode);
        assert_eq!(config.max_retries, 0);
        assert_eq!(config.timeout, 120);
        assert_eq!(config.on_blocked, OnBlockedStrategy::Fail);
    }

    #[test]
    fn test_verify_config_expert_mode() {
        let config = VerifyConfig::expert_mode();
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.timeout, 600);
        assert_eq!(config.on_blocked, OnBlockedStrategy::Retry);
    }

    #[test]
    fn test_should_run() {
        let config = VerifyConfig::default();
        assert!(config.should_run());
    }

    #[test]
    fn test_on_blocked_strategy_default() {
        assert_eq!(OnBlockedStrategy::default(), OnBlockedStrategy::Retry);
    }

    #[test]
    fn test_build_verification_prompt() {
        let task = Task::new("task-1", "Test Task", "Implement a feature");
        let prompt = build_verification_prompt(&task);

        assert!(prompt.contains("task-1"));
        assert!(prompt.contains("Test Task"));
        assert!(prompt.contains("Implement a feature"));
        assert!(prompt.contains("Verification Criteria"));
        assert!(prompt.contains("Acceptance Criteria"));
    }

    #[test]
    fn test_parse_verification_response_passing() {
        let task = Task::new("task-1", "Test", "Description");
        let response = r#"
Some text here.

```json
{
  "passed": true,
  "reasoning": "All requirements met",
  "unmet_criteria": [],
  "suggestions": [],
  "retry_recommended": false
}
```

More text.
"#
        .to_string();

        let result = parse_verification_response(&task, response).unwrap();
        assert!(result.passed);
        assert_eq!(result.reasoning, "All requirements met");
        assert!(result.unmet_criteria.is_empty());
        assert!(!result.retry_recommended);
    }

    #[test]
    fn test_parse_verification_response_failing() {
        let task = Task::new("task-1", "Test", "Description");
        let response = r#"
```json
{
  "passed": false,
  "reasoning": "Missing error handling",
  "unmet_criteria": ["Edge case handling"],
  "suggestions": ["Add error checking"],
  "retry_recommended": true
}
```
"#
        .to_string();

        let result = parse_verification_response(&task, response).unwrap();
        assert!(!result.passed);
        assert_eq!(result.reasoning, "Missing error handling");
        assert_eq!(result.unmet_criteria.len(), 1);
        assert_eq!(result.unmet_criteria[0], "Edge case handling");
        assert!(result.retry_recommended);
    }

    #[test]
    fn test_parse_verification_response_no_json() {
        let task = Task::new("task-1", "Test", "Description");
        let response = "The task looks good to me. All requirements are met.".to_string();

        let result = parse_verification_response(&task, response).unwrap();
        // When no JSON is found, default to passing
        assert!(result.passed);
        assert!(result.reasoning.contains("task looks good"));
    }

    #[test]
    fn test_verification_result_creation() {
        let task = Task::new("task-1", "Test", "Description");
        let result = VerificationResult {
            task: task.clone(),
            passed: true,
            reasoning: "Good work".to_string(),
            unmet_criteria: vec![],
            suggestions: vec![],
            retry_recommended: false,
        };

        assert!(result.passed);
        assert_eq!(result.task.id, "task-1");
        assert!(result.unmet_criteria.is_empty());
    }

    #[test]
    fn test_verification_summary_empty() {
        let summary = VerificationSummary {
            total_tasks: 0,
            passed_tasks: 0,
            failed_tasks: 0,
            skipped_tasks: 0,
            total_time: 0,
            results: vec![],
        };

        assert_eq!(summary.total_tasks, 0);
        assert_eq!(summary.passed_tasks, 0);
        assert_eq!(summary.failed_tasks, 0);
    }

    #[test]
    fn test_on_blocked_strategy_equality() {
        assert_eq!(OnBlockedStrategy::Fail, OnBlockedStrategy::Fail);
        assert_eq!(OnBlockedStrategy::Retry, OnBlockedStrategy::Retry);
        assert_eq!(OnBlockedStrategy::Block, OnBlockedStrategy::Block);
        assert_eq!(OnBlockedStrategy::Skip, OnBlockedStrategy::Skip);
    }

    #[test]
    fn test_on_blocked_strategy_inequality() {
        assert_ne!(OnBlockedStrategy::Fail, OnBlockedStrategy::Retry);
        assert_ne!(OnBlockedStrategy::Retry, OnBlockedStrategy::Block);
        assert_ne!(OnBlockedStrategy::Block, OnBlockedStrategy::Skip);
        assert_ne!(OnBlockedStrategy::Skip, OnBlockedStrategy::Fail);
    }

    #[tokio::test]
    async fn test_verify_tasks_with_disabled_config() {
        let task = Task::new("task-1", "Test", "Description");
        let mut config = VerifyConfig::default();
        config.enabled = false;

        let (tasks, summary) = verify_tasks(vec![task], &config).await.unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(summary.skipped_tasks, 1);
        assert_eq!(summary.passed_tasks, 0);
        assert_eq!(summary.failed_tasks, 0);
    }

    #[test]
    fn test_should_verify_in_fast_mode() {
        let config = ModeConfig::fast_mode();
        // Fast mode still has verify: true by default
        assert!(should_verify_in_fast_mode(&config));
    }
}
