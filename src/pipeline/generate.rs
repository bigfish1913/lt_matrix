//! Task generation stage
//!
//! This module implements the Generate stage of the pipeline, which:
//! - Takes a user goal as input
//! - Uses Claude to break down the goal into structured tasks
//! - Generates task dependencies and execution order
//! - Validates tasks for missing dependencies and circular references
//! - Returns a validated task list ready for the assess stage

use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};

use crate::agent::backend::{AgentBackend, ExecutionConfig};
use crate::agent::ClaudeAgent;
use crate::models::{Task, TaskComplexity, TaskStatus};

/// Configuration for the generation stage
#[derive(Debug, Clone)]
pub struct GenerateConfig {
    /// Model to use for task generation
    pub generation_model: String,

    /// Timeout for generation requests (seconds)
    pub timeout: u64,

    /// Maximum retries for generation
    pub max_retries: u32,

    /// Maximum number of tasks to generate
    pub max_tasks: usize,

    /// Whether to enable validation
    pub enable_validation: bool,

    /// Execution mode (affects task granularity)
    pub execution_mode: ExecutionMode,
}

impl Default for GenerateConfig {
    fn default() -> Self {
        GenerateConfig {
            generation_model: "claude-sonnet-4-6".to_string(),
            timeout: 180,
            max_retries: 3,
            max_tasks: 50,
            enable_validation: true,
            execution_mode: ExecutionMode::Standard,
        }
    }
}

impl GenerateConfig {
    /// Create config for fast mode (fewer, larger tasks)
    pub fn fast_mode() -> Self {
        GenerateConfig {
            generation_model: "claude-haiku-4-5".to_string(),
            timeout: 120,
            max_retries: 1,
            max_tasks: 20,
            enable_validation: true,
            execution_mode: ExecutionMode::Fast,
        }
    }

    /// Create config for expert mode (more granular tasks)
    pub fn expert_mode() -> Self {
        GenerateConfig {
            generation_model: "claude-opus-4-6".to_string(),
            timeout: 300,
            max_retries: 3,
            max_tasks: 100,
            enable_validation: true,
            execution_mode: ExecutionMode::Expert,
        }
    }
}

/// Execution mode for task generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Fast mode: fewer, larger tasks
    Fast,
    /// Standard mode: balanced task granularity
    Standard,
    /// Expert mode: more granular tasks
    Expert,
}

/// Result of task generation
#[derive(Debug, Clone)]
pub struct GenerationResult {
    /// The generated tasks
    pub tasks: Vec<Task>,

    /// Number of tasks created
    pub task_count: usize,

    /// Maximum depth of dependencies
    pub dependency_depth: usize,

    /// Validation errors (if any)
    pub validation_errors: Vec<ValidationError>,
}

/// Validation errors that can occur during generation
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// Task depends on non-existent task
    MissingDependency { task: String, dependency: String },

    /// Circular dependency detected
    CircularDependency { chain: Vec<String> },

    /// Task ID is duplicated
    DuplicateTaskId { id: String },

    /// Task has invalid structure
    InvalidStructure { task: String, reason: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::MissingDependency { task, dependency } => {
                write!(
                    f,
                    "Task '{}' depends on non-existent task '{}'",
                    task, dependency
                )
            }
            ValidationError::CircularDependency { chain } => {
                write!(f, "Circular dependency detected: {}", chain.join(" -> "))
            }
            ValidationError::DuplicateTaskId { id } => {
                write!(f, "Duplicate task ID: '{}'", id)
            }
            ValidationError::InvalidStructure { task, reason } => {
                write!(f, "Task '{}' has invalid structure: {}", task, reason)
            }
        }
    }
}

/// Generates a task list from a user goal
pub async fn generate_tasks(goal: &str, config: &GenerateConfig) -> Result<GenerationResult> {
    info!("Starting generation stage for goal: {}", goal);

    let agent = ClaudeAgent::new().context("Failed to create Claude agent for generation")?;

    // Build generation prompt
    let prompt = build_generation_prompt(goal, config);

    // Create execution config
    let exec_config = ExecutionConfig {
        model: config.generation_model.clone(),
        timeout: config.timeout,
        max_retries: config.max_retries,
        enable_session: false,
        env_vars: Vec::new(),
    };

    // Execute generation
    let response = agent
        .execute(&prompt, &exec_config)
        .await
        .context("Failed to execute generation prompt")?;

    // Parse response into tasks
    let mut tasks = parse_generation_response(&response.output)
        .context("Failed to parse generation response")?;

    // Enforce max tasks limit
    if tasks.len() > config.max_tasks {
        warn!(
            "Generated {} tasks exceeds limit of {}, truncating",
            tasks.len(),
            config.max_tasks
        );
        tasks.truncate(config.max_tasks);
    }

    // Calculate task count before moving
    let task_count = tasks.len();

    // Validate tasks if enabled
    let validation_errors = if config.enable_validation {
        validate_tasks(&tasks)
    } else {
        Vec::new()
    };

    // Calculate dependency depth
    let dependency_depth = calculate_dependency_depth(&tasks);

    info!(
        "Generation completed: {} tasks created (depth: {})",
        task_count,
        dependency_depth
    );

    Ok(GenerationResult {
        tasks,
        task_count,
        dependency_depth,
        validation_errors,
    })
}

/// Builds the generation prompt for Claude
fn build_generation_prompt(goal: &str, config: &GenerateConfig) -> String {
    let (task_hint, granularity_hint) = match config.execution_mode {
        ExecutionMode::Fast => (
            "5-15 high-level tasks",
            "Focus on major milestones. Combine related changes into larger tasks.",
        ),
        ExecutionMode::Standard => (
            "10-30 detailed tasks",
            "Break down into logical implementation steps. Balance granularity with clarity.",
        ),
        ExecutionMode::Expert => (
            "20-50 granular tasks",
            "Break down into very small, individually testable steps. Each task should be independently verifiable.",
        ),
    };

    format!(
        r#"You are an expert software architect and project manager. Your task is to break down the following user goal into a structured task list.

## User Goal
{}

## Your Instructions

1. **Create {}** that fully implement the user goal.
2. Each task must be:
   - Specific and actionable
   - Independently testable (where possible)
   - Have a clear, concise title
   - Include a detailed description

3. **Establish dependencies** between tasks:
   - Tasks can only depend on tasks that appear earlier in the list
   - Use `depends_on` to specify task IDs that must complete first
   - Start with foundational tasks (setup, configuration)
   - End with integration and testing tasks

4. **Suggested task types** (in order):
   - Setup/initialization tasks
   - Data model/schema tasks
   - Core implementation tasks
   - API/interface tasks
   - Testing tasks
   - Documentation tasks
   - Integration/validation tasks

5. {}

## Response Format

Respond ONLY with valid JSON in this exact format:

```json
{{
  "summary": "<Brief overview of the implementation plan>",
  "estimated_tasks": <number of tasks>,
  "tasks": [
    {{
      "id": "task-1",
      "title": "<Task title>",
      "description": "<Detailed description of what this task should accomplish>",
      "depends_on": [],
      "complexity": "Simple|Moderate|Complex"
    }},
    {{
      "id": "task-2",
      "title": "<Task title>",
      "description": "<Detailed description>",
      "depends_on": ["task-1"],
      "complexity": "Simple|Moderate|Complex"
    }}
  ]
}}
```

## Important Notes

- Task IDs must be unique and follow the pattern: `task-1`, `task-2`, etc.
- The first task should have `depends_on: []` (no dependencies)
- Avoid circular dependencies
- Be realistic about complexity
- Focus on actionable implementation steps
"#,
        goal, task_hint, granularity_hint
    )
}

/// Parses Claude's response and extracts tasks
fn parse_generation_response(response: &str) -> Result<Vec<Task>> {
    // Extract JSON from response
    let json_str =
        extract_json_block(response).context("No JSON block found in generation response")?;

    // Parse JSON
    let json: Value = serde_json::from_str(json_str).context("Failed to parse generation JSON")?;

    // Extract tasks array
    let tasks_array = json["tasks"]
        .as_array()
        .context("Missing or invalid 'tasks' array in response")?;

    let mut tasks = Vec::new();
    for (index, task_json) in tasks_array.iter().enumerate() {
        let id = task_json["id"]
            .as_str()
            .unwrap_or(&format!("task-{}", index + 1))
            .to_string();

        let title = task_json["title"]
            .as_str()
            .context(format!("Task {} missing 'title'", index))?
            .to_string();

        let description = task_json["description"]
            .as_str()
            .context(format!("Task {} missing 'description'", index))?
            .to_string();

        let depends_on: Vec<String> = task_json["depends_on"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        // Parse complexity if provided
        let complexity = task_json["complexity"]
            .as_str()
            .and_then(|s| match s {
                "Simple" => Some(TaskComplexity::Simple),
                "Moderate" => Some(TaskComplexity::Moderate),
                "Complex" => Some(TaskComplexity::Complex),
                _ => None,
            })
            .unwrap_or(TaskComplexity::Moderate);

        let mut task = Task::new(&id, &title, &description);
        task.depends_on = depends_on;
        task.complexity = complexity;
        task.status = TaskStatus::Pending;

        tasks.push(task);
    }

    debug!("Parsed {} tasks from generation response", tasks.len());
    Ok(tasks)
}

/// Extracts JSON block from markdown response
fn extract_json_block(text: &str) -> Option<&str> {
    // Look for ```json block
    let json_start = text.find("```json")? + 7; // Skip past ```json
    let json_end = text[json_start..].find("```")?;
    Some(text[json_start..json_start + json_end].trim())
}

/// Validates a list of tasks for common issues
fn validate_tasks(tasks: &[Task]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let task_ids: HashSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();

    // Check for duplicate IDs
    let mut seen_ids = HashSet::new();
    for task in tasks {
        if !seen_ids.insert(&task.id) {
            errors.push(ValidationError::DuplicateTaskId {
                id: task.id.clone(),
            });
        }
    }

    // Check for missing dependencies
    for task in tasks {
        for dep in &task.depends_on {
            if !task_ids.contains(dep.as_str()) {
                errors.push(ValidationError::MissingDependency {
                    task: task.id.clone(),
                    dependency: dep.clone(),
                });
            }
        }
    }

    // Check for circular dependencies
    let circular_chains = detect_circular_dependencies(tasks);
    errors.extend(circular_chains.into_iter().map(|chain| {
        ValidationError::CircularDependency { chain }
    }));

    // Check for invalid task structures
    for task in tasks {
        if task.title.trim().is_empty() {
            errors.push(ValidationError::InvalidStructure {
                task: task.id.clone(),
                reason: "title is empty".to_string(),
            });
        }
        if task.description.trim().is_empty() {
            errors.push(ValidationError::InvalidStructure {
                task: task.id.clone(),
                reason: "description is empty".to_string(),
            });
        }
    }

    errors
}

/// Detects circular dependencies in the task graph
fn detect_circular_dependencies(tasks: &[Task]) -> Vec<Vec<String>> {
    let mut circular_chains = Vec::new();

    // Build adjacency map
    let mut adj_map: HashMap<&str, Vec<&str>> = HashMap::new();
    for task in tasks {
        let deps: Vec<&str> = task.depends_on.iter().map(|s| s.as_str()).collect();
        adj_map.insert(task.id.as_str(), deps);
    }

    // Detect cycles using DFS
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    for task_id in adj_map.keys() {
        if !visited.contains(task_id) {
            if let Some(cycle) = dfs_detect_cycle(
                task_id,
                &adj_map,
                &mut visited,
                &mut rec_stack,
                &mut path,
            ) {
                circular_chains.push(cycle);
            }
        }
    }

    circular_chains
}

/// DFS helper to detect cycles
fn dfs_detect_cycle<'a>(
    node: &'a str,
    adj_map: &HashMap<&'a str, Vec<&'a str>>,
    visited: &mut HashSet<&'a str>,
    rec_stack: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<String>> {
    visited.insert(node);
    rec_stack.insert(node);
    path.push(node);

    if let Some(neighbors) = adj_map.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                if let Some(cycle) =
                    dfs_detect_cycle(neighbor, adj_map, visited, rec_stack, path)
                {
                    return Some(cycle);
                }
            } else if rec_stack.contains(neighbor) {
                // Found a cycle - extract it from the path
                let cycle_start = path.iter().position(|&n| n == *neighbor).unwrap();
                let cycle = path[cycle_start..].iter().map(|s| s.to_string()).collect();
                return Some(cycle);
            }
        }
    }

    path.pop();
    rec_stack.remove(node);
    None
}

/// Calculates the maximum depth of dependencies
fn calculate_dependency_depth(tasks: &[Task]) -> usize {
    let mut depth_map: HashMap<&str, usize> = HashMap::new();

    // Initialize all tasks with depth 0
    for task in tasks {
        depth_map.insert(task.id.as_str(), 0);
    }

    // Calculate depth using topological order
    let mut changed = true;
    let mut iterations = 0;
    let max_iterations = tasks.len() + 1;

    while changed && iterations < max_iterations {
        changed = false;
        iterations += 1;

        for task in tasks {
            if task.depends_on.is_empty() {
                continue;
            }

            // Calculate depth based on dependencies
            let max_dep_depth = task
                .depends_on
                .iter()
                .filter_map(|dep| depth_map.get(dep.as_str()).copied())
                .max()
                .unwrap_or(0);

            let new_depth = max_dep_depth + 1;
            let current_depth = *depth_map.get(task.id.as_str()).unwrap_or(&0);

            if new_depth > current_depth {
                depth_map.insert(task.id.as_str(), new_depth);
                changed = true;
            }
        }
    }

    depth_map.values().copied().max().unwrap_or(0)
}

/// Calculates statistics about generated tasks
pub fn calculate_generation_stats(result: &GenerationResult) -> GenerationStats {
    let mut simple = 0;
    let mut moderate = 0;
    let mut complex = 0;
    let mut with_dependencies = 0;
    let mut total_dependencies = 0;

    for task in &result.tasks {
        match task.complexity {
            TaskComplexity::Simple => simple += 1,
            TaskComplexity::Moderate => moderate += 1,
            TaskComplexity::Complex => complex += 1,
        }

        if !task.depends_on.is_empty() {
            with_dependencies += 1;
            total_dependencies += task.depends_on.len();
        }
    }

    GenerationStats {
        total_tasks: result.task_count,
        simple,
        moderate,
        complex,
        tasks_with_dependencies: with_dependencies,
        total_dependencies,
        dependency_depth: result.dependency_depth,
        validation_errors: result.validation_errors.len(),
    }
}

/// Statistics about task generation
#[derive(Debug, Clone)]
pub struct GenerationStats {
    /// Total number of tasks generated
    pub total_tasks: usize,

    /// Number of simple tasks
    pub simple: usize,

    /// Number of moderate tasks
    pub moderate: usize,

    /// Number of complex tasks
    pub complex: usize,

    /// Number of tasks with dependencies
    pub tasks_with_dependencies: usize,

    /// Total number of dependencies
    pub total_dependencies: usize,

    /// Maximum dependency depth
    pub dependency_depth: usize,

    /// Number of validation errors
    pub validation_errors: usize,
}

impl std::fmt::Display for GenerationStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Generation Summary:\n\
             - Total tasks: {}\n\
             - Simple: {}\n\
             - Moderate: {}\n\
             - Complex: {}\n\
             - Tasks with dependencies: {}\n\
             - Total dependencies: {}\n\
             - Max depth: {}\n\
             - Validation errors: {}",
            self.total_tasks,
            self.simple,
            self.moderate,
            self.complex,
            self.tasks_with_dependencies,
            self.total_dependencies,
            self.dependency_depth,
            self.validation_errors
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_block() {
        let response = r#"Some text before.

```json
{
  "tasks": []
}
```

Some text after."#;

        let json = extract_json_block(response).unwrap();
        assert!(json.contains("tasks"));
    }

    #[test]
    fn test_extract_json_block_no_json() {
        let response = "This response has no JSON block in it.";
        let result = extract_json_block(response);
        assert!(
            result.is_none(),
            "Should return None when no JSON block found"
        );
    }

    #[test]
    fn test_parse_generation_response_simple() {
        let response = r#"```json
{
  "summary": "Simple plan",
  "tasks": [
    {
      "id": "task-1",
      "title": "First Task",
      "description": "A simple task",
      "depends_on": [],
      "complexity": "Simple"
    }
  ]
}
```"#;

        let tasks = parse_generation_response(response).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "task-1");
        assert_eq!(tasks[0].title, "First Task");
        assert_eq!(tasks[0].complexity, TaskComplexity::Simple);
        assert!(tasks[0].depends_on.is_empty());
    }

    #[test]
    fn test_parse_generation_response_with_dependencies() {
        let response = r#"```json
{
  "summary": "Tasks with dependencies",
  "tasks": [
    {
      "id": "task-1",
      "title": "Setup",
      "description": "Initial setup",
      "depends_on": []
    },
    {
      "id": "task-2",
      "title": "Implement",
      "description": "Main implementation",
      "depends_on": ["task-1"]
    }
  ]
}
```"#;

        let tasks = parse_generation_response(response).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[1].depends_on, vec!["task-1".to_string()]);
    }

    #[test]
    fn test_parse_generation_response_auto_ids() {
        let response = r#"```json
{
  "tasks": [
    {
      "title": "Auto ID Task",
      "description": "Task without explicit ID"
    }
  ]
}
```"#;

        let tasks = parse_generation_response(response).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "task-1");
    }

    #[test]
    fn test_validate_tasks_missing_dependency() {
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "Task 1", "First task");
                t.depends_on = vec!["non-existent".to_string()];
                t
            },
        ];

        let errors = validate_tasks(&tasks);
        assert!(
            errors.iter().any(|e| matches!(
                e,
                ValidationError::MissingDependency { .. }
            )),
            "Should detect missing dependency"
        );
    }

    #[test]
    fn test_validate_tasks_duplicate_id() {
        let tasks = vec![
            Task::new("task-1", "Task 1", "First"),
            Task::new("task-1", "Task 2", "Second"),
        ];

        let errors = validate_tasks(&tasks);
        assert!(
            errors.iter().any(|e| matches!(e, ValidationError::DuplicateTaskId { .. })),
            "Should detect duplicate ID"
        );
    }

    #[test]
    fn test_validate_tasks_circular_dependency() {
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "Task 1", "First");
                t.depends_on = vec!["task-2".to_string()];
                t
            },
            {
                let mut t = Task::new("task-2", "Task 2", "Second");
                t.depends_on = vec!["task-1".to_string()];
                t
            },
        ];

        let errors = validate_tasks(&tasks);
        assert!(
            errors.iter().any(|e| matches!(
                e,
                ValidationError::CircularDependency { .. }
            )),
            "Should detect circular dependency"
        );
    }

    #[test]
    fn test_validate_tasks_invalid_structure() {
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "", "Description");
                t.depends_on = vec![];
                t
            },
        ];

        let errors = validate_tasks(&tasks);
        assert!(
            errors.iter().any(|e| matches!(
                e,
                ValidationError::InvalidStructure { .. }
            )),
            "Should detect invalid structure (empty title)"
        );
    }

    #[test]
    fn test_validate_tasks_valid() {
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "Valid Task", "Valid description");
                t.depends_on = vec![];
                t
            },
            {
                let mut t = Task::new("task-2", "Another Valid Task", "Another valid description");
                t.depends_on = vec!["task-1".to_string()];
                t
            },
        ];

        let errors = validate_tasks(&tasks);
        assert!(
            errors.is_empty(),
            "Valid tasks should have no validation errors"
        );
    }

    #[test]
    fn test_calculate_dependency_depth() {
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "Task 1", "No deps");
                t.depends_on = vec![];
                t
            },
            {
                let mut t = Task::new("task-2", "Task 2", "Depends on task-1");
                t.depends_on = vec!["task-1".to_string()];
                t
            },
            {
                let mut t = Task::new("task-3", "Task 3", "Depends on task-2");
                t.depends_on = vec!["task-2".to_string()];
                t
            },
        ];

        let depth = calculate_dependency_depth(&tasks);
        assert_eq!(depth, 2, "Should have max depth of 2");
    }

    #[test]
    fn test_calculate_dependency_depth_no_deps() {
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "Task 1", "No deps");
                t.depends_on = vec![];
                t
            },
            {
                let mut t = Task::new("task-2", "Task 2", "Also no deps");
                t.depends_on = vec![];
                t
            },
        ];

        let depth = calculate_dependency_depth(&tasks);
        assert_eq!(depth, 0, "Should have depth of 0 when no dependencies");
    }

    #[test]
    fn test_calculate_generation_stats() {
        let result = GenerationResult {
            tasks: vec![
                {
                    let mut t = Task::new("task-1", "Simple", "Simple task");
                    t.complexity = TaskComplexity::Simple;
                    t.depends_on = vec![];
                    t
                },
                {
                    let mut t = Task::new("task-2", "Moderate", "Moderate task");
                    t.complexity = TaskComplexity::Moderate;
                    t.depends_on = vec!["task-1".to_string()];
                    t
                },
                {
                    let mut t = Task::new("task-3", "Complex", "Complex task");
                    t.complexity = TaskComplexity::Complex;
                    t.depends_on = vec!["task-2".to_string()];
                    t
                },
            ],
            task_count: 3,
            dependency_depth: 2,
            validation_errors: vec![],
        };

        let stats = calculate_generation_stats(&result);
        assert_eq!(stats.total_tasks, 3);
        assert_eq!(stats.simple, 1);
        assert_eq!(stats.moderate, 1);
        assert_eq!(stats.complex, 1);
        assert_eq!(stats.tasks_with_dependencies, 2);
        assert_eq!(stats.total_dependencies, 2);
        assert_eq!(stats.dependency_depth, 2);
        assert_eq!(stats.validation_errors, 0);
    }

    #[test]
    fn test_generation_config_defaults() {
        let config = GenerateConfig::default();
        assert_eq!(config.generation_model, "claude-sonnet-4-6");
        assert_eq!(config.timeout, 180);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.max_tasks, 50);
        assert!(config.enable_validation);
    }

    #[test]
    fn test_generation_config_fast_mode() {
        let config = GenerateConfig::fast_mode();
        assert_eq!(config.generation_model, "claude-haiku-4-5");
        assert_eq!(config.timeout, 120);
        assert_eq!(config.max_retries, 1);
        assert_eq!(config.max_tasks, 20);
        assert_eq!(config.execution_mode, ExecutionMode::Fast);
    }

    #[test]
    fn test_generation_config_expert_mode() {
        let config = GenerateConfig::expert_mode();
        assert_eq!(config.generation_model, "claude-opus-4-6");
        assert_eq!(config.timeout, 300);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.max_tasks, 100);
        assert_eq!(config.execution_mode, ExecutionMode::Expert);
    }

    #[test]
    fn test_build_generation_prompt_contains_goal() {
        let goal = "Implement a REST API";
        let config = GenerateConfig::default();
        let prompt = build_generation_prompt(goal, &config);

        assert!(prompt.contains(goal));
        assert!(prompt.contains("10-30 detailed tasks"));
        assert!(prompt.contains("tasks"));
        assert!(prompt.contains("depends_on"));
    }

    #[test]
    fn test_build_generation_prompt_fast_mode() {
        let goal = "Add feature X";
        let config = GenerateConfig::fast_mode();
        let prompt = build_generation_prompt(goal, &config);

        assert!(prompt.contains("5-15 high-level tasks"));
        assert!(prompt.contains("Combine related changes"));
    }

    #[test]
    fn test_build_generation_prompt_expert_mode() {
        let goal = "Build complex system";
        let config = GenerateConfig::expert_mode();
        let prompt = build_generation_prompt(goal, &config);

        assert!(prompt.contains("20-50 granular tasks"));
        assert!(prompt.contains("individually testable"));
    }

    #[test]
    fn test_detect_circular_dependencies_simple_cycle() {
        let tasks = vec![
            {
                let mut t = Task::new("a", "A", "Task A");
                t.depends_on = vec!["b".to_string()];
                t
            },
            {
                let mut t = Task::new("b", "B", "Task B");
                t.depends_on = vec!["a".to_string()];
                t
            },
        ];

        let cycles = detect_circular_dependencies(&tasks);
        assert_eq!(cycles.len(), 1);
    }

    #[test]
    fn test_detect_circular_dependencies_complex_cycle() {
        let tasks = vec![
            {
                let mut t = Task::new("a", "A", "Task A");
                t.depends_on = vec!["b".to_string()];
                t
            },
            {
                let mut t = Task::new("b", "B", "Task B");
                t.depends_on = vec!["c".to_string()];
                t
            },
            {
                let mut t = Task::new("c", "C", "Task C");
                t.depends_on = vec!["a".to_string()];
                t
            },
        ];

        let cycles = detect_circular_dependencies(&tasks);
        assert_eq!(cycles.len(), 1);
    }

    #[test]
    fn test_detect_circular_dependencies_no_cycle() {
        let tasks = vec![
            {
                let mut t = Task::new("a", "A", "Task A");
                t.depends_on = vec![];
                t
            },
            {
                let mut t = Task::new("b", "B", "Task B");
                t.depends_on = vec!["a".to_string()];
                t
            },
            {
                let mut t = Task::new("c", "C", "Task C");
                t.depends_on = vec!["b".to_string()];
                t
            },
        ];

        let cycles = detect_circular_dependencies(&tasks);
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError::MissingDependency {
            task: "task-1".to_string(),
            dependency: "task-2".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("task-1"));
        assert!(display.contains("task-2"));
        assert!(display.contains("non-existent"));
    }

    #[test]
    fn test_generation_stats_display() {
        let stats = GenerationStats {
            total_tasks: 10,
            simple: 3,
            moderate: 5,
            complex: 2,
            tasks_with_dependencies: 7,
            total_dependencies: 12,
            dependency_depth: 3,
            validation_errors: 0,
        };

        let display = format!("{}", stats);
        assert!(display.contains("Total tasks: 10"));
        assert!(display.contains("Simple: 3"));
        assert!(display.contains("Moderate: 5"));
        assert!(display.contains("Complex: 2"));
        assert!(display.contains("Max depth: 3"));
    }

    #[tokio::test]
    async fn test_parse_generation_response_invalid_json() {
        let response = r#"```json
{ invalid json }
```"#;

        let result = parse_generation_response(response);
        assert!(result.is_err(), "Should return error for invalid JSON");
    }

    #[tokio::test]
    async fn test_parse_generation_response_no_json_block() {
        let response = "This response has no JSON block at all.";

        let result = parse_generation_response(response);
        assert!(
            result.is_err(),
            "Should return error when no JSON block found"
        );
    }

    #[test]
    fn test_parse_generation_response_missing_tasks_array() {
        let response = r#"```json
{
  "summary": "No tasks array"
}
```"#;

        let result = parse_generation_response(response);
        assert!(result.is_err(), "Should fail when tasks array is missing");
    }

    #[test]
    fn test_validate_tasks_empty_list() {
        let tasks: Vec<Task> = vec![];
        let errors = validate_tasks(&tasks);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_calculate_dependency_depth_empty_list() {
        let tasks: Vec<Task> = vec![];
        let depth = calculate_dependency_depth(&tasks);
        assert_eq!(depth, 0);
    }

    #[test]
    fn test_parse_generation_response_complex_dependencies() {
        let response = r#"```json
{
  "tasks": [
    {
      "id": "setup",
      "title": "Setup",
      "description": "Initial setup",
      "depends_on": [],
      "complexity": "Simple"
    },
    {
      "id": "implement",
      "title": "Implement",
      "description": "Implementation",
      "depends_on": ["setup"],
      "complexity": "Moderate"
    },
    {
      "id": "test",
      "title": "Test",
      "description": "Testing",
      "depends_on": ["implement", "setup"],
      "complexity": "Complex"
    }
  ]
}
```"#;

        let tasks = parse_generation_response(response).unwrap();
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[2].depends_on.len(), 2);
        assert!(tasks[2].depends_on.contains(&"implement".to_string()));
        assert!(tasks[2].depends_on.contains(&"setup".to_string()));
    }
}
