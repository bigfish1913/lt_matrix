// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Task execution stage
//!
//! This module implements the Execute stage of the pipeline, which:
//! - Executes tasks using the agent backend with task context
//! - Uses session management for efficient retry and dependency chain handling
//! - Passes memory.md content for context awareness
//! - Handles different models based on task complexity
//! - Captures agent output and updates task status
//! - Implements retry with max_retries limit from config
//! - Uses exponential backoff for retry delays
//! - Classifies errors as retryable or non-retryable

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use ltmatrix_agent::backend::{AgentBackend, ExecutionConfig};
use ltmatrix_agent::claude::ClaudeAgent;
use ltmatrix_agent::session::SessionManager;
use ltmatrix_agent::AgentPool;
use ltmatrix_core::{AgentType, Mode, ModeConfig, Task, TaskComplexity, TaskStatus};
use crate::workspace::WorkspaceState;

/// Classification of execution errors for retry decisions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorClass {
    /// Transient errors that can be retried (timeout, rate limit, network)
    Retryable,
    /// Permanent errors that should not be retried (syntax, permission, not found)
    NonRetryable,
    /// Unknown errors - default to retryable with caution
    Unknown,
}

/// Report of a failed task and its impact
#[derive(Debug, Clone)]
pub struct FailureReport {
    /// ID of the failed task
    pub task_id: String,

    /// Error message from the failure
    pub error_message: String,

    /// Number of retry attempts made
    pub retry_count: u32,

    /// Tasks that depend on this failed task (directly or transitively)
    pub blocked_downstream: Vec<String>,

    /// Suggested action to take
    pub suggested_action: FailureAction,
}

/// Suggested action for handling a failure
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureAction {
    /// Retry the task (for transient errors)
    Retry,
    /// Skip the task and continue with others
    Skip,
    /// Abort the entire execution
    Abort,
}

/// Calculate exponential backoff delay for retry attempts
///
/// Uses formula: base_delay * 2^attempt with optional jitter
/// Maximum delay is capped to prevent excessive waits
pub fn calculate_backoff_delay(attempt: u32, base_delay_secs: u64, max_delay_secs: u64) -> Duration {
    // Exponential backoff: 1s, 2s, 4s, 8s, 16s, etc.
    let multiplier = 2u64.pow(attempt);
    let delay = std::cmp::min(base_delay_secs * multiplier, max_delay_secs);

    // Add small jitter (±10%) to avoid thundering herd
    let (jitter_min, jitter_max) = if delay > 0 {
        let jitter_range = delay / 10;
        (delay.saturating_sub(jitter_range), delay.saturating_add(jitter_range))
    } else {
        (0, 0)
    };

    Duration::from_secs(rand::random_in_range(jitter_min..=jitter_max))
}

/// Internal helper for random jitter (simple implementation)
mod rand {
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn random_in_range(range: std::ops::RangeInclusive<u64>) -> u64 {
        // Simple LCG random for jitter (doesn't need to be cryptographically secure)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let range_size = range.end() - range.start() + 1;
        if range_size == 0 {
            return *range.start();
        }
        (now.wrapping_mul(1103515245).wrapping_add(12345) % range_size) + *range.start()
    }
}

/// Classify an error message to determine if it's retryable
pub fn classify_error(error_message: &str) -> ErrorClass {
    let lower = error_message.to_lowercase();

    // Retryable errors - transient conditions
    let retryable_patterns = [
        "timeout",
        "timed out",
        "rate limit",
        "too many requests",
        "service unavailable",
        "temporarily unavailable",
        "connection reset",
        "connection refused",
        "network error",
        "dns error",
        "socket error",
        "internal server error",
        "503",
        "502",
        "504",
        "429",
    ];

    // Non-retryable errors - permanent conditions
    let non_retryable_patterns = [
        "syntax error",
        "parse error",
        "invalid syntax",
        "permission denied",
        "access denied",
        "unauthorized",
        "forbidden",
        "not found",
        "does not exist",
        "no such file",
        "invalid argument",
        "invalid parameter",
        "out of memory",
        "stack overflow",
        "unsupported",
        "not implemented",
    ];

    // Check for retryable patterns
    for pattern in &retryable_patterns {
        if lower.contains(pattern) {
            return ErrorClass::Retryable;
        }
    }

    // Check for non-retryable patterns
    for pattern in &non_retryable_patterns {
        if lower.contains(pattern) {
            return ErrorClass::NonRetryable;
        }
    }

    // Default to unknown (treated as retryable with caution)
    ErrorClass::Unknown
}

/// Generate a failure report for a failed task
pub fn generate_failure_report(
    task: &Task,
    task_map: &HashMap<String, Task>,
) -> FailureReport {
    let error_message = task.error.clone().unwrap_or_else(|| "Unknown error".to_string());
    let error_class = classify_error(&error_message);

    // Find all downstream tasks that depend on this task
    let blocked_downstream = find_downstream_tasks(&task.id, task_map);

    // Determine suggested action based on error class and retry count
    let suggested_action = match error_class {
        ErrorClass::Retryable if task.retry_count < 3 => FailureAction::Retry,
        ErrorClass::NonRetryable => {
            if blocked_downstream.is_empty() {
                FailureAction::Skip
            } else {
                FailureAction::Abort
            }
        }
        ErrorClass::Retryable | ErrorClass::Unknown => {
            if blocked_downstream.len() > 3 {
                FailureAction::Abort // Too many downstream tasks affected
            } else {
                FailureAction::Retry
            }
        }
    };

    FailureReport {
        task_id: task.id.clone(),
        error_message,
        retry_count: task.retry_count,
        blocked_downstream,
        suggested_action,
    }
}

/// Find all tasks that transitively depend on a given task
fn find_downstream_tasks(task_id: &str, task_map: &HashMap<String, Task>) -> Vec<String> {
    let mut downstream = Vec::new();
    let mut visited = HashSet::new();

    fn find_recursive(
        id: &str,
        task_map: &HashMap<String, Task>,
        downstream: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) {
        if visited.contains(id) {
            return;
        }
        visited.insert(id.to_string());

        for (other_id, task) in task_map {
            if task.depends_on.iter().any(|dep| dep == id) {
                if !downstream.contains(other_id) {
                    downstream.push(other_id.clone());
                }
                find_recursive(other_id, task_map, downstream, visited);
            }
        }
    }

    find_recursive(task_id, task_map, &mut downstream, &mut visited);
    downstream
}

/// Configuration for the execution stage
#[derive(Debug, Clone)]
pub struct ExecuteConfig {
    /// Mode configuration for model selection
    pub mode_config: ModeConfig,

    /// Maximum retries per task
    pub max_retries: u32,

    /// Timeout for task execution (seconds)
    pub timeout: u64,

    /// Whether to enable session reuse
    pub enable_sessions: bool,

    /// Working directory for execution
    pub work_dir: PathBuf,

    /// Project memory file path
    pub memory_file: PathBuf,

    /// Whether to enable workspace state persistence
    pub enable_workspace_persistence: bool,

    /// Project root directory for workspace state
    pub project_root: Option<PathBuf>,

    /// Optional AgentPool for session management
    ///
    /// When provided, this takes precedence over SessionManager
    /// for all session operations.
    pub agent_pool: Option<AgentPool>,
}

impl Default for ExecuteConfig {
    fn default() -> Self {
        ExecuteConfig {
            mode_config: ModeConfig::default(),
            max_retries: 3,
            timeout: 3600,
            enable_sessions: true,
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            memory_file: PathBuf::from(".claude/memory.md"),
            enable_workspace_persistence: false,
            project_root: None,
            agent_pool: None,
        }
    }
}

impl ExecuteConfig {
    /// Create config for fast mode
    pub fn fast_mode() -> Self {
        ExecuteConfig {
            mode_config: ModeConfig::fast_mode(),
            max_retries: 1,
            timeout: 1800,
            enable_sessions: true,
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            memory_file: PathBuf::from(".claude/memory.md"),
            enable_workspace_persistence: false,
            project_root: None,
            agent_pool: None,
        }
    }

    /// Create config for expert mode
    pub fn expert_mode() -> Self {
        ExecuteConfig {
            mode_config: ModeConfig::expert_mode(),
            max_retries: 3,
            timeout: 7200,
            enable_sessions: true,
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            memory_file: PathBuf::from(".claude/memory.md"),
            enable_workspace_persistence: false,
            project_root: None,
            agent_pool: None,
        }
    }
}

/// Result of executing a single task
#[derive(Debug, Clone)]
pub struct TaskExecutionResult {
    /// The executed task (with updated status)
    pub task: Task,

    /// Output from the agent
    pub output: String,

    /// Number of retries attempted
    pub retries: u32,

    /// Session ID if session was reused
    pub session_id: Option<String>,

    /// Execution time in seconds
    pub execution_time: u64,
}

/// Statistics about task execution
#[derive(Debug, Clone)]
pub struct ExecutionStatistics {
    /// Total tasks executed
    pub total_tasks: usize,

    /// Successfully completed tasks
    pub completed_tasks: usize,

    /// Failed tasks
    pub failed_tasks: usize,

    /// Tasks skipped due to mode disabled
    pub skipped_tasks: usize,

    /// Total retries attempted
    pub total_retries: u32,

    /// Total execution time in seconds
    pub total_time: u64,

    /// Tasks by complexity
    pub simple_tasks: usize,
    pub moderate_tasks: usize,
    pub complex_tasks: usize,

    /// Sessions reused
    pub sessions_reused: usize,
}

/// Execute a list of tasks with dependency resolution
pub async fn execute_tasks(
    tasks: Vec<Task>,
    config: &ExecuteConfig,
) -> Result<(Vec<Task>, ExecutionStatistics)> {
    info!("Starting execution stage for {} tasks", tasks.len());

    let start_time = std::time::Instant::now();

    // Create agent and session manager (fallback when agent_pool not provided)
    let agent = ClaudeAgent::new().context("Failed to create Claude agent")?;
    let session_manager = if config.agent_pool.is_none() {
        Some(SessionManager::new(&config.work_dir).context("Failed to create session manager")?)
    } else {
        None
    };

    // Clean up stale sessions
    if let Some(pool) = &config.agent_pool {
        pool.cleanup_stale_sessions().await;
    } else if let Some(sm) = &session_manager {
        sm.cleanup_stale_sessions()
            .await
            .context("Failed to cleanup stale sessions")?;
    }

    // Load project memory for context
    let project_memory = load_project_memory(&config.memory_file).await?;

    // Build task map for dependency lookup
    let task_map: HashMap<String, Task> = tasks
        .into_iter()
        .map(|task| (task.id.clone(), task))
        .collect();

    // Track completed tasks and session propagation
    let mut completed_tasks: HashSet<String> = HashSet::new();
    let mut task_sessions: HashMap<String, String> = HashMap::new();
    let mut results = Vec::new();
    let mut stats = ExecutionStatistics {
        total_tasks: task_map.len(),
        completed_tasks: 0,
        failed_tasks: 0,
        total_retries: 0,
        total_time: 0,
        simple_tasks: 0,
        moderate_tasks: 0,
        complex_tasks: 0,
        sessions_reused: 0,
        skipped_tasks: 0,
    };

    // Get mode from agent pool if available
    let mode = if let Some(pool) = &config.agent_pool {
        pool.get_mode().await
    } else {
        None
    };

    // Execute tasks in dependency order
    for task_id in get_execution_order(&task_map)? {
        let mut task = task_map
            .get(&task_id)
            .cloned()
            .context(format!("Task {} not found in task map", task_id))?;

        // Update complexity stats
        match task.complexity {
            TaskComplexity::Simple => stats.simple_tasks += 1,
            TaskComplexity::Moderate => stats.moderate_tasks += 1,
            TaskComplexity::Complex => stats.complex_tasks += 1,
        }

        // Check if task's agent type is enabled in current mode
        if let Some(m) = mode {
            if !m.is_agent_enabled(task.agent_type) {
                info!(
                    "Skipping task {} - agent type {:?} not enabled in {:?} mode",
                    task.id, task.agent_type, m
                );
                task.status = TaskStatus::SkippedModeDisabled;
                task.error = Some(format!(
                    "Agent type {:?} not enabled in {:?} mode",
                    task.agent_type, m
                ));
                stats.skipped_tasks += 1;
                results.push(task);
                continue;
            }
        }

        // Check if dependencies are satisfied
        if !task.can_execute(&completed_tasks) {
            warn!("Task {} dependencies not satisfied, skipping", task.id);
            continue;
        }

        info!("Executing task: {} ({})", task.id, task.title);

        // Select model based on complexity
        let model = config.mode_config.model_for_complexity(&task.complexity);

        // Propagate session from dependencies
        let session_id = task
            .depends_on
            .iter()
            .filter_map(|dep_id| task_sessions.get(dep_id))
            .next()
            .cloned();

        // Build task context
        let context = build_task_context(&task, &task_map, &completed_tasks, &project_memory)?;

        // Execute task with retry logic - use AgentPool if available
        let execution_result = if let Some(pool) = &config.agent_pool {
            execute_task_with_agent_pool(
                &mut task,
                &context,
                model,
                session_id,
                pool,
                &agent,
                config,
            )
            .await?
        } else {
            execute_task_with_retry(
                &task,
                &context,
                model,
                session_id,
                &agent,
                session_manager.as_ref().unwrap(),
                config,
            )
            .await?
        };

        stats.total_retries += execution_result.retries;
        stats.total_time += execution_result.execution_time;

        // Update task status
        if task.error.is_none() {
            task.status = TaskStatus::Completed;
            task.completed_at = Some(chrono::Utc::now());
            completed_tasks.insert(task.id.clone());
            stats.completed_tasks += 1;

            // Store session for dependent tasks
            if let Some(sid) = &execution_result.session_id {
                task_sessions.insert(task.id.clone(), sid.clone());
                stats.sessions_reused += 1;
            }

            info!("Task {} completed successfully", task.id);
        } else {
            task.status = TaskStatus::Failed;
            stats.failed_tasks += 1;
            error!("Task {} failed: {:?}", task.id, task.error);
        }

        // Save workspace state after each task completion if enabled
        if config.enable_workspace_persistence {
            if let Some(project_root) = &config.project_root {
                if let Err(e) = save_workspace_state(project_root, &task_map) {
                    warn!("Failed to save workspace state after task {}: {}", task.id, e);
                }
            }
        }

        results.push(task);
    }

    let elapsed = start_time.elapsed().as_secs();
    info!("Execution stage completed in {}s", elapsed);

    Ok((results, stats))
}

/// Load project memory from memory.md file
async fn load_project_memory(memory_path: &Path) -> Result<String> {
    if memory_path.exists() {
        let content = fs::read_to_string(memory_path)
            .await
            .context("Failed to read project memory file")?;

        debug!("Loaded project memory from {:?}", memory_path);
        Ok(content)
    } else {
        debug!("No project memory file found at {:?}", memory_path);
        Ok(String::new())
    }
}

/// Get execution order respecting dependencies
pub fn get_execution_order(task_map: &HashMap<String, Task>) -> Result<Vec<String>> {
    let mut order = Vec::new();
    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();

    for task_id in task_map.keys() {
        if !visited.contains(task_id) {
            visit_task(task_id, task_map, &mut visited, &mut visiting, &mut order)?;
        }
    }

    Ok(order)
}

/// Visit task for topological sort with cycle detection
fn visit_task(
    task_id: &str,
    task_map: &HashMap<String, Task>,
    visited: &mut HashSet<String>,
    visiting: &mut HashSet<String>,
    order: &mut Vec<String>,
) -> Result<()> {
    // If already fully processed, skip
    if visited.contains(task_id) {
        return Ok(());
    }

    // If currently on the recursion stack, we have a cycle
    if visiting.contains(task_id) {
        anyhow::bail!("Circular dependency detected involving task '{}'", task_id);
    }

    let task = task_map
        .get(task_id)
        .context(format!("Task {} not found", task_id))?;

    // Mark as currently being visited
    visiting.insert(task_id.to_string());

    // Visit dependencies first
    for dep_id in &task.depends_on {
        visit_task(dep_id, task_map, visited, visiting, order)?;
    }

    // Done visiting - mark as fully processed
    visiting.remove(task_id);
    visited.insert(task_id.to_string());
    order.push(task_id.to_string());
    Ok(())
}

/// Build context string for task execution
pub fn build_task_context(
    task: &Task,
    task_map: &HashMap<String, Task>,
    completed_tasks: &HashSet<String>,
    project_memory: &str,
) -> Result<String> {
    let mut context = String::new();

    // Add project memory if available
    if !project_memory.is_empty() {
        context.push_str("## Project Memory\n\n");
        context.push_str(project_memory);
        context.push_str("\n\n");
    }

    // Add goal context
    context.push_str("## Task Context\n\n");
    context.push_str(&format!("Task: {}\n", task.title));
    context.push_str(&format!("Description: {}\n\n", task.description));

    // Add dependency information
    if !task.depends_on.is_empty() {
        context.push_str("### Dependencies\n\n");
        for dep_id in &task.depends_on {
            if completed_tasks.contains(dep_id) {
                if let Some(dep_task) = task_map.get(dep_id) {
                    context.push_str(&format!("- {} (completed)\n", dep_task.title));
                }
            }
        }
        context.push('\n');
    }

    // Add complexity information
    context.push_str(&format!("Complexity: {:?}\n\n", task.complexity));

    Ok(context)
}

/// Execute a task with retry logic and exponential backoff
async fn execute_task_with_retry(
    task: &Task,
    context: &str,
    model: &str,
    session_id: Option<String>,
    agent: &ClaudeAgent,
    session_manager: &SessionManager,
    config: &ExecuteConfig,
) -> Result<TaskExecutionResult> {
    let start_time = std::time::Instant::now();
    let current_session = session_id;
    let mut last_output = String::new();
    let mut last_error: Option<String> = None;
    let mut retries = 0;

    for attempt in 0..=config.max_retries {
        if attempt > 0 {
            // Apply exponential backoff before retry
            let backoff = calculate_backoff_delay(attempt - 1, 1, 60);
            info!(
                "Retrying task {} (attempt {}/{}) after {:?} backoff",
                task.id, attempt, config.max_retries, backoff
            );
            sleep(backoff).await;
            retries += 1;
        }

        // Build execution config
        let exec_config = ExecutionConfig {
            model: model.to_string(),
            max_retries: 0, // We handle retries at the task level
            timeout: config.timeout,
            enable_session: config.enable_sessions,
            env_vars: Vec::new(),
        };

        // Build prompt with context
        let prompt = build_execution_prompt(task, context);

        // Execute with or without session
        let result = if config.enable_sessions {
            execute_with_session(
                task,
                &prompt,
                &exec_config,
                current_session.clone(),
                agent,
                session_manager,
            )
            .await?
        } else {
            let response = agent.execute(&prompt, &exec_config).await?;
            TaskExecutionResult {
                task: task.clone(),
                output: response.output.clone(),
                retries: 0,
                session_id: None,
                execution_time: start_time.elapsed().as_secs(),
            }
        };

        last_output = result.output.clone();

        // Check if execution was successful
        if task.error.is_none() {
            debug!("Task {} completed successfully", task.id);
            return Ok(TaskExecutionResult {
                task: task.clone(),
                output: last_output,
                retries,
                session_id: current_session,
                execution_time: start_time.elapsed().as_secs(),
            });
        }

        // Store the error for classification
        last_error = task.error.clone();

        // Classify the error to decide if we should retry
        if let Some(ref error_msg) = task.error {
            let error_class = classify_error(error_msg);

            match error_class {
                ErrorClass::NonRetryable => {
                    warn!(
                        "Task {} failed with non-retryable error: {}",
                        task.id, error_msg
                    );
                    break; // Don't retry non-retryable errors
                }
                ErrorClass::Retryable | ErrorClass::Unknown => {
                    // Check if we should retry
                    if attempt < config.max_retries && task.can_retry(config.max_retries) {
                        warn!(
                            "Task {} failed with retryable error, will retry: {}",
                            task.id, error_msg
                        );
                        continue;
                    } else {
                        break;
                    }
                }
            }
        } else {
            break;
        }
    }

    // All retries exhausted or non-retryable error
    let final_error = last_error.unwrap_or_else(|| "Unknown error".to_string());
    warn!(
        "Task {} failed after {} retries: {}",
        task.id, retries, final_error
    );

    Ok(TaskExecutionResult {
        task: task.clone(),
        output: last_output,
        retries,
        session_id: current_session,
        execution_time: start_time.elapsed().as_secs(),
    })
}

/// Execute a task with AgentPool integration
///
/// This function uses AgentPool for session management, providing:
/// - Session reuse for retry scenarios
/// - Session inheritance for dependency chains
/// - Cross-task session sharing
async fn execute_task_with_agent_pool(
    task: &mut Task,
    context: &str,
    model: &str,
    session_id: Option<String>,
    pool: &AgentPool,
    agent: &ClaudeAgent,
    config: &ExecuteConfig,
) -> Result<TaskExecutionResult> {
    let start_time = std::time::Instant::now();
    let mut last_output = String::new();
    let mut retries = 0;

    // Set parent session ID if provided (dependency chain)
    if let Some(parent_sid) = session_id {
        task.set_parent_session_id(&parent_sid);
    }

    for attempt in 0..=config.max_retries {
        if attempt > 0 {
            info!(
                "Retrying task {} (attempt {}/{})",
                task.id, attempt, config.max_retries
            );
            retries += 1;
            task.prepare_retry();
        }

        // Build execution config
        let exec_config = ExecutionConfig {
            model: model.to_string(),
            max_retries: 0, // We handle retries at the task level
            timeout: config.timeout,
            enable_session: config.enable_sessions,
            env_vars: Vec::new(),
        };

        // Build prompt with context
        let prompt = build_execution_prompt(task, context);

        // Get or create session using AgentPool
        // This handles retry reuse, dependency inheritance, and cross-task sharing
        let _session_id = pool
            .get_or_create_session_for_task(task, agent.backend_name(), model)
            .await;

        // Execute using the agent pool
        let result = pool
            .execute_with_session(task, agent, &prompt, &exec_config)
            .await?;

        last_output = result.output.clone();

        // Check if execution was successful
        if task.error.is_none() {
            debug!("Task {} completed successfully with AgentPool", task.id);
            return Ok(TaskExecutionResult {
                task: task.clone(),
                output: last_output,
                retries,
                session_id: task.get_session_id().map(|s| s.to_string()),
                execution_time: start_time.elapsed().as_secs(),
            });
        }

        // Check if we should retry
        if attempt < config.max_retries && task.can_retry(config.max_retries) {
            warn!("Task {} failed, will retry: {:?}", task.id, task.error);
            continue;
        } else {
            break;
        }
    }

    // All retries exhausted
    Ok(TaskExecutionResult {
        task: task.clone(),
        output: last_output,
        retries,
        session_id: task.get_session_id().map(|s| s.to_string()),
        execution_time: start_time.elapsed().as_secs(),
    })
}

/// Execute task with session management
async fn execute_with_session(
    task: &Task,
    prompt: &str,
    config: &ExecutionConfig,
    session_id: Option<String>,
    agent: &ClaudeAgent,
    session_manager: &SessionManager,
) -> Result<TaskExecutionResult> {
    let start_time = std::time::Instant::now();

    // Load or create session
    let session = if let Some(sid) = session_id {
        session_manager
            .load_session(&sid)
            .await?
            .context(format!("Session {} not found", sid))?
    } else {
        session_manager
            .create_session(agent.backend_name(), &config.model)
            .await?
    };

    // Execute with session context
    let response = agent.execute(prompt, config).await?;

    // Update session access
    let mut updated_session = session.clone();
    updated_session.mark_accessed();
    session_manager.save_session(&updated_session).await?;

    Ok(TaskExecutionResult {
        task: task.clone(),
        output: response.output,
        retries: 0,
        session_id: Some(session.session_id),
        execution_time: start_time.elapsed().as_secs(),
    })
}

/// Build execution prompt for the agent
pub fn build_execution_prompt(task: &Task, context: &str) -> String {
    format!(
        r#"You are implementing a task for a software development project.

{}

## Your Task

**{}**

{}

## Instructions

Please implement this task following these requirements:

1. **Complete the task**: Implement all necessary code, tests, and documentation
2. **Follow best practices**: Write clean, maintainable code following project conventions
3. **Add tests**: Include appropriate unit tests or integration tests
4. **Document changes**: Add or update relevant documentation

When you're done, provide a summary of:
- What you implemented
- Files created or modified
- Tests added
- Any issues encountered

Begin your implementation now.
"#,
        context, task.title, task.description
    )
}

/// Display execution statistics
pub fn display_execution_statistics(stats: &ExecutionStatistics) {
    println!("\n=== Execution Statistics ===");
    println!("Total tasks: {}", stats.total_tasks);
    println!("Completed: {}", stats.completed_tasks);
    println!("Failed: {}", stats.failed_tasks);
    if stats.skipped_tasks > 0 {
        println!("Skipped (mode disabled): {}", stats.skipped_tasks);
    }
    println!("Total retries: {}", stats.total_retries);
    println!("Total time: {}s", stats.total_time);
    println!("\nComplexity breakdown:");
    println!("  Simple: {}", stats.simple_tasks);
    println!("  Moderate: {}", stats.moderate_tasks);
    println!("  Complex: {}", stats.complex_tasks);
    println!("Sessions reused: {}", stats.sessions_reused);
}

/// Select model for task based on agent type and mode
///
/// Returns the appropriate model for the task's agent type,
/// considering the execution mode.
pub fn select_model_for_task(task: &Task, mode: Option<Mode>, config: &ExecuteConfig) -> String {
    // If mode is available, use mode-specific model selection
    if let Some(m) = mode {
        let model = match task.agent_type {
            AgentType::Plan => m.plan_model(),
            AgentType::Dev => m.exec_model(),
            AgentType::Test => m.exec_model(),
            AgentType::Review => m.review_model(),
        };
        return model.to_string();
    }

    // Fallback to complexity-based model selection
    config.mode_config.model_for_complexity(&task.complexity).to_string()
}

/// Check if a task should be executed based on its agent type and the current mode
///
/// Returns true if the task's agent type is enabled in the given mode.
pub fn should_execute_task(task: &Task, mode: Option<Mode>) -> bool {
    match mode {
        Some(m) => m.is_agent_enabled(task.agent_type),
        None => true, // No mode specified, execute all tasks
    }
}

/// Save workspace state for all tasks in the task map
///
/// This function loads the current workspace state, updates all tasks
/// from the task map, and saves it back to disk.
///
/// # Arguments
///
/// * `project_root` - Path to the project root directory
/// * `task_map` - HashMap containing the current state of all tasks
///
/// # Returns
///
/// Returns `Ok(())` if the state was saved successfully, or an error otherwise.
fn save_workspace_state(
    project_root: &Path,
    task_map: &HashMap<String, Task>,
) -> Result<()> {
    // Load or create workspace state
    let state = if let Ok(loaded) = WorkspaceState::load(project_root.to_path_buf()) {
        loaded
    } else {
        // If load fails, create new state
        let tasks: Vec<Task> = task_map.values().cloned().collect();
        WorkspaceState::new(project_root.to_path_buf(), tasks)
    };

    // Update tasks from task map
    let mut updated_state = state;
    updated_state.tasks = task_map.values().cloned().collect();

    // Save the updated state
    updated_state.save()?;

    debug!("Workspace state saved to {:?}", project_root);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ltmatrix_core::TaskComplexity;

    #[test]
    fn test_execute_config_default() {
        let config = ExecuteConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout, 3600);
        assert!(config.enable_sessions);
    }

    #[test]
    fn test_execute_config_fast_mode() {
        let config = ExecuteConfig::fast_mode();
        assert_eq!(config.max_retries, 1);
        assert_eq!(config.timeout, 1800);
    }

    #[test]
    fn test_execute_config_expert_mode() {
        let config = ExecuteConfig::expert_mode();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout, 7200);
    }

    #[test]
    fn test_build_execution_prompt() {
        let task = Task::new("task-1", "Test Task", "Implement a feature");
        let context = "Project context here";

        let prompt = build_execution_prompt(&task, context);

        assert!(prompt.contains("Implement a feature"));
        assert!(prompt.contains("Project context here"));
        assert!(prompt.contains("Begin your implementation now"));
    }

    #[test]
    fn test_get_execution_order_no_deps() {
        let mut task1 = Task::new("task-1", "First", "First task");
        let mut task2 = Task::new("task-2", "Second", "Second task");

        task1.complexity = TaskComplexity::Simple;
        task2.complexity = TaskComplexity::Moderate;

        let task_map: HashMap<String, Task> =
            [(task1.id.clone(), task1), (task2.id.clone(), task2)]
                .into_iter()
                .collect();

        let order = get_execution_order(&task_map).unwrap();

        assert_eq!(order.len(), 2);
        assert!(order.contains(&"task-1".to_string()));
        assert!(order.contains(&"task-2".to_string()));
    }

    #[test]
    fn test_get_execution_order_with_deps() {
        let task1 = Task::new("task-1", "First", "First task");
        let mut task2 = Task::new("task-2", "Second", "Second task");
        task2.depends_on = vec!["task-1".to_string()];

        let task_map: HashMap<String, Task> =
            [(task1.id.clone(), task1), (task2.id.clone(), task2)]
                .into_iter()
                .collect();

        let order = get_execution_order(&task_map).unwrap();

        assert_eq!(order.len(), 2);
        assert_eq!(order[0], "task-1");
        assert_eq!(order[1], "task-2");
    }

    #[tokio::test]
    async fn test_load_project_memory_not_found() {
        let temp_dir = tempfile::tempdir().unwrap();
        let memory_path = temp_dir.path().join("nonexistent.md");

        let memory = load_project_memory(&memory_path).await.unwrap();

        assert!(memory.is_empty());
    }

    #[test]
    fn test_build_task_context() {
        let task = Task::new("task-1", "Test Task", "Implement something");
        let task_map: HashMap<String, Task> =
            [(task.id.clone(), task.clone())].into_iter().collect();
        let completed_tasks: HashSet<String> = HashSet::new();
        let project_memory = "Previous decisions";

        let context =
            build_task_context(&task, &task_map, &completed_tasks, project_memory).unwrap();

        assert!(context.contains("Project Memory"));
        assert!(context.contains("Previous decisions"));
        assert!(context.contains("Task: Test Task"));
    }

    #[test]
    fn test_classify_error_retryable() {
        // Test retryable error patterns
        assert_eq!(
            classify_error("Connection timeout while fetching data"),
            ErrorClass::Retryable
        );
        assert_eq!(
            classify_error("Rate limit exceeded, please retry later"),
            ErrorClass::Retryable
        );
        assert_eq!(
            classify_error("Service temporarily unavailable (503)"),
            ErrorClass::Retryable
        );
        assert_eq!(
            classify_error("Network error: connection reset"),
            ErrorClass::Retryable
        );
        assert_eq!(
            classify_error("429 Too Many Requests"),
            ErrorClass::Retryable
        );
    }

    #[test]
    fn test_classify_error_non_retryable() {
        // Test non-retryable error patterns
        assert_eq!(
            classify_error("Syntax error in configuration file"),
            ErrorClass::NonRetryable
        );
        assert_eq!(
            classify_error("Permission denied: cannot access file"),
            ErrorClass::NonRetryable
        );
        assert_eq!(
            classify_error("File not found: config.toml"),
            ErrorClass::NonRetryable
        );
        assert_eq!(
            classify_error("Invalid argument provided"),
            ErrorClass::NonRetryable
        );
        assert_eq!(
            classify_error("Feature not implemented"),
            ErrorClass::NonRetryable
        );
    }

    #[test]
    fn test_classify_error_unknown() {
        // Test unknown error patterns
        assert_eq!(
            classify_error("Something went wrong"),
            ErrorClass::Unknown
        );
        assert_eq!(
            classify_error("An unexpected error occurred"),
            ErrorClass::Unknown
        );
    }

    #[test]
    fn test_calculate_backoff_delay() {
        // Test exponential backoff progression
        let base = 1u64;
        let max = 60u64;

        // Attempt 0: 1s base
        let d0 = calculate_backoff_delay(0, base, max);
        assert!(d0.as_secs() >= 1 && d0.as_secs() <= 2); // With jitter

        // Attempt 1: 2s base
        let d1 = calculate_backoff_delay(1, base, max);
        assert!(d1.as_secs() >= 1 && d1.as_secs() <= 3); // With jitter

        // Attempt 2: 4s base
        let d2 = calculate_backoff_delay(2, base, max);
        assert!(d2.as_secs() >= 3 && d2.as_secs() <= 5); // With jitter

        // Test max cap
        let d10 = calculate_backoff_delay(10, base, max);
        assert!(d10.as_secs() <= max);
    }

    #[test]
    fn test_generate_failure_report() {
        let mut task = Task::new("task-1", "Test", "Test task");
        task.error = Some("Connection timeout".to_string());
        task.retry_count = 2;

        let mut task2 = Task::new("task-2", "Dependent", "Dependent task");
        task2.depends_on = vec!["task-1".to_string()];

        let mut task3 = Task::new("task-3", "Indirect", "Indirect dependent");
        task3.depends_on = vec!["task-2".to_string()];

        let task_map: HashMap<String, Task> = [
            (task.id.clone(), task.clone()),
            (task2.id.clone(), task2),
            (task3.id.clone(), task3),
        ]
        .into_iter()
        .collect();

        let report = generate_failure_report(&task, &task_map);

        assert_eq!(report.task_id, "task-1");
        assert!(report.error_message.contains("timeout"));
        assert_eq!(report.retry_count, 2);
        assert!(report.blocked_downstream.contains(&"task-2".to_string()));
        assert!(report.blocked_downstream.contains(&"task-3".to_string()));
        // Should suggest retry for retryable errors
        assert_eq!(report.suggested_action, FailureAction::Retry);
    }

    #[test]
    fn test_generate_failure_report_non_retryable() {
        let mut task = Task::new("task-1", "Test", "Test task");
        task.error = Some("Syntax error in code".to_string());

        let task_map: HashMap<String, Task> = [(task.id.clone(), task.clone())]
            .into_iter()
            .collect();

        let report = generate_failure_report(&task, &task_map);

        // Non-retryable with no downstream should suggest skip
        assert_eq!(report.suggested_action, FailureAction::Skip);
    }

    #[test]
    fn test_find_downstream_tasks() {
        let task1 = Task::new("task-1", "Root", "Root task");
        let mut task2 = Task::new("task-2", "Child1", "Child task 1");
        task2.depends_on = vec!["task-1".to_string()];
        let mut task3 = Task::new("task-3", "Child2", "Child task 2");
        task3.depends_on = vec!["task-1".to_string()];
        let mut task4 = Task::new("task-4", "Grandchild", "Grandchild task");
        task4.depends_on = vec!["task-2".to_string()];

        let task_map: HashMap<String, Task> = [
            (task1.id.clone(), task1),
            (task2.id.clone(), task2),
            (task3.id.clone(), task3),
            (task4.id.clone(), task4),
        ]
        .into_iter()
        .collect();

        let downstream = find_downstream_tasks("task-1", &task_map);

        assert_eq!(downstream.len(), 3);
        assert!(downstream.contains(&"task-2".to_string()));
        assert!(downstream.contains(&"task-3".to_string()));
        assert!(downstream.contains(&"task-4".to_string()));
    }
}
