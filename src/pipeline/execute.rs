//! Task execution stage
//!
//! This module implements the Execute stage of the pipeline, which:
//! - Executes tasks using the agent backend with task context
//! - Uses session management for efficient retry and dependency chain handling
//! - Passes memory.md content for context awareness
//! - Handles different models based on task complexity
//! - Captures agent output and updates task status
//! - Implements retry with max_retries limit from config

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, error, info, warn};

use crate::agent::backend::{AgentBackend, ExecutionConfig};
use crate::agent::claude::ClaudeAgent;
use crate::agent::session::SessionManager;
use crate::models::{ModeConfig, Task, TaskComplexity, TaskStatus};
use crate::workspace::WorkspaceState;

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

    // Create agent and session manager
    let agent = ClaudeAgent::new().context("Failed to create Claude agent")?;
    let session_manager =
        SessionManager::new(&config.work_dir).context("Failed to create session manager")?;

    // Clean up stale sessions
    session_manager
        .cleanup_stale_sessions()
        .await
        .context("Failed to cleanup stale sessions")?;

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

        // Execute task with retry logic
        let execution_result = execute_task_with_retry(
            &task,
            &context,
            model,
            session_id,
            &agent,
            &session_manager,
            config,
        )
        .await?;

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

/// Execute a task with retry logic
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
    let mut retries = 0;

    for attempt in 0..=config.max_retries {
        if attempt > 0 {
            info!(
                "Retrying task {} (attempt {}/{})",
                task.id, attempt, config.max_retries
            );
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
        session_id: current_session,
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
    println!("Total retries: {}", stats.total_retries);
    println!("Total time: {}s", stats.total_time);
    println!("\nComplexity breakdown:");
    println!("  Simple: {}", stats.simple_tasks);
    println!("  Moderate: {}", stats.moderate_tasks);
    println!("  Complex: {}", stats.complex_tasks);
    println!("Sessions reused: {}", stats.sessions_reused);
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
    use crate::models::TaskComplexity;

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
}
