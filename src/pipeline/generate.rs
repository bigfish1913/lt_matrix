// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


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

use ltmatrix_agent::backend::{AgentBackend, ExecutionConfig};
use ltmatrix_agent::ClaudeAgent;
use ltmatrix_core::{AgentType, Task, TaskComplexity, TaskStatus};

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

    /// Generation log with detailed analysis
    pub generation_log: Option<GenerationLog>,
}

/// Log of task generation with detailed analysis
#[derive(Debug, Clone)]
pub struct GenerationLog {
    /// Timestamp when generation started
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Original goal
    pub goal: String,

    /// Number of tasks generated
    pub tasks_generated: usize,

    /// Breakdown by complexity
    pub complexity_breakdown: HashMap<String, usize>,

    /// Breakdown by agent type
    pub agent_type_breakdown: HashMap<String, usize>,

    /// Dependency analysis
    pub dependency_analysis: DependencyAnalysis,

    /// Split strategy used
    pub split_strategy: SplitStrategy,
}

/// Dependency analysis results
#[derive(Debug, Clone)]
pub struct DependencyAnalysis {
    /// Maximum depth of the dependency tree
    pub max_depth: usize,

    /// Number of root tasks (no dependencies)
    pub root_count: usize,

    /// Number of leaf tasks (no dependents)
    pub leaf_count: usize,

    /// Total number of dependency edges
    pub total_edges: usize,

    /// Critical path length
    pub critical_path_length: usize,

    /// Number of parallelizable tasks
    pub parallelizable_count: usize,
}

/// Strategy used for task splitting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitStrategy {
    /// Granular: Small tasks, 1-2 hours each
    Granular,
    /// Moderate: Medium tasks, 2-4 hours each
    Moderate,
    /// Coarse: Large tasks, 4-8 hours each
    Coarse,
}

impl GenerationLog {
    /// Creates a new generation log
    pub fn new(goal: &str, tasks: &[Task]) -> Self {
        let mut complexity_breakdown = HashMap::new();
        let mut agent_type_breakdown = HashMap::new();

        for task in tasks {
            let complexity_key = match task.complexity {
                TaskComplexity::Simple => "simple",
                TaskComplexity::Moderate => "moderate",
                TaskComplexity::Complex => "complex",
            };
            *complexity_breakdown.entry(complexity_key.to_string()).or_insert(0) += 1;

            let agent_type_key = match task.agent_type {
                AgentType::Plan => "plan",
                AgentType::Dev => "dev",
                AgentType::Test => "test",
                AgentType::Review => "review",
            };
            *agent_type_breakdown.entry(agent_type_key.to_string()).or_insert(0) += 1;
        }

        let dependency_analysis = analyze_dependencies(tasks);

        let split_strategy = determine_split_strategy(tasks);

        GenerationLog {
            timestamp: chrono::Utc::now(),
            goal: goal.to_string(),
            tasks_generated: tasks.len(),
            complexity_breakdown,
            agent_type_breakdown,
            dependency_analysis,
            split_strategy,
        }
    }

    /// Formats the log as a human-readable string
    pub fn to_readable_string(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("=== Task Generation Log ===\n"));
        output.push_str(&format!("Timestamp: {}\n", self.timestamp));
        output.push_str(&format!("Goal: {}\n", self.goal));
        output.push_str(&format!("Tasks Generated: {}\n\n", self.tasks_generated));

        output.push_str("Complexity Breakdown:\n");
        for (complexity, count) in &self.complexity_breakdown {
            output.push_str(&format!("  - {}: {}\n", complexity, count));
        }

        output.push_str("\nAgent Type Breakdown:\n");
        for (agent_type, count) in &self.agent_type_breakdown {
            output.push_str(&format!("  - {}: {}\n", agent_type, count));
        }

        output.push_str("\nDependency Analysis:\n");
        output.push_str(&format!("  - Max Depth: {}\n", self.dependency_analysis.max_depth));
        output.push_str(&format!("  - Root Tasks: {}\n", self.dependency_analysis.root_count));
        output.push_str(&format!("  - Leaf Tasks: {}\n", self.dependency_analysis.leaf_count));
        output.push_str(&format!("  - Total Edges: {}\n", self.dependency_analysis.total_edges));
        output.push_str(&format!("  - Critical Path: {}\n", self.dependency_analysis.critical_path_length));
        output.push_str(&format!("  - Parallelizable: {}\n", self.dependency_analysis.parallelizable_count));

        output.push_str(&format!("\nSplit Strategy: {:?}\n", self.split_strategy));

        output
    }
}

/// Analyzes task dependencies
fn analyze_dependencies(tasks: &[Task]) -> DependencyAnalysis {
    use std::collections::{HashSet, HashMap};

    let task_ids: HashSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();

    // Count edges and roots
    let mut total_edges = 0;
    let mut root_count = 0;
    let mut dependent_count: HashMap<&str, usize> = HashMap::new();

    for task in tasks {
        total_edges += task.depends_on.len();
        if task.depends_on.is_empty() {
            root_count += 1;
        }

        // Track how many tasks depend on each task
        for dep in &task.depends_on {
            *dependent_count.entry(dep.as_str()).or_insert(0) += 1;
        }
    }

    // Leaf tasks are those that have no dependents
    let leaf_count = tasks.iter()
        .filter(|t| dependent_count.get(t.id.as_str()).unwrap_or(&0) == &0)
        .count();

    // Calculate max depth using BFS
    let max_depth = calculate_dependency_depth(tasks);

    // Estimate critical path (simplified)
    let critical_path_length = max_depth;

    // Count parallelizable tasks (tasks at the same level)
    let parallelizable_count = root_count; // Simplified: root tasks can run in parallel

    DependencyAnalysis {
        max_depth,
        root_count,
        leaf_count,
        total_edges,
        critical_path_length,
        parallelizable_count,
    }
}

/// Determines the split strategy based on task characteristics
fn determine_split_strategy(tasks: &[Task]) -> SplitStrategy {
    if tasks.is_empty() {
        return SplitStrategy::Moderate;
    }

    // Calculate average task description length as a heuristic
    let avg_desc_len: f64 = tasks.iter()
        .map(|t| t.description.len())
        .sum::<usize>() as f64 / tasks.len() as f64;

    // Shorter descriptions typically indicate smaller, more granular tasks
    if avg_desc_len < 100.0 {
        SplitStrategy::Granular
    } else if avg_desc_len < 300.0 {
        SplitStrategy::Moderate
    } else {
        SplitStrategy::Coarse
    }
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

/// Result of dependency validation with detailed metrics
#[derive(Debug, Clone)]
pub struct DependencyValidationResult {
    /// Whether all dependencies are valid (no missing or circular dependencies)
    pub is_valid: bool,

    /// Dependency validation errors found
    pub errors: Vec<DependencyError>,

    /// Statistics about the dependency graph
    pub stats: DependencyGraphStats,
}

/// Detailed statistics about the task dependency graph
#[derive(Debug, Clone)]
pub struct DependencyGraphStats {
    /// Total number of tasks in the graph
    pub total_tasks: usize,

    /// Number of tasks that have dependencies
    pub tasks_with_dependencies: usize,

    /// Total number of dependency edges in the graph
    pub total_dependencies: usize,

    /// Maximum depth of the dependency graph
    pub max_depth: usize,

    /// Number of tasks with no dependencies (roots)
    pub root_tasks: usize,

    /// Number of tasks with no dependents (leaves)
    pub leaf_tasks: usize,

    /// Number of missing dependencies detected
    pub missing_dependencies: usize,

    /// Number of circular dependency chains detected
    pub circular_dependencies: usize,

    /// Whether the graph is a Directed Acyclic Graph (DAG)
    pub is_dag: bool,
}

/// Dependency-specific validation errors
#[derive(Debug, Clone)]
pub enum DependencyError {
    /// Reference to a task that doesn't exist
    MissingReference { task_id: String, missing_ref: String },

    /// Circular dependency in the task graph
    CircularChain { chain: Vec<String> },
}

impl From<DependencyError> for ValidationError {
    fn from(error: DependencyError) -> Self {
        match error {
            DependencyError::MissingReference { task_id, missing_ref } => {
                ValidationError::MissingDependency {
                    task: task_id,
                    dependency: missing_ref,
                }
            }
            DependencyError::CircularChain { chain } => {
                ValidationError::CircularDependency { chain }
            }
        }
    }
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

    // Create generation log with detailed analysis
    let generation_log = GenerationLog::new(goal, &tasks);

    // Log detailed generation information
    info!(
        "Generation completed: {} tasks created (depth: {})",
        task_count,
        dependency_depth
    );
    debug!("Generation log:\n{}", generation_log.to_readable_string());

    // Log complexity and agent type breakdown
    for (complexity, count) in &generation_log.complexity_breakdown {
        debug!("  Complexity {}: {}", complexity, count);
    }
    for (agent_type, count) in &generation_log.agent_type_breakdown {
        debug!("  Agent type {}: {}", agent_type, count);
    }

    Ok(GenerationResult {
        tasks,
        task_count,
        dependency_depth,
        validation_errors,
        generation_log: Some(generation_log),
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

        // Auto-assign agent_type based on task content
        // Combine title and description for better keyword matching
        let combined_text = format!("{} {}", title, description);
        task.agent_type = AgentType::from_keywords(&combined_text);

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

    // Check for duplicate IDs
    let mut seen_ids = HashSet::new();
    for task in tasks {
        if !seen_ids.insert(&task.id) {
            errors.push(ValidationError::DuplicateTaskId {
                id: task.id.clone(),
            });
        }
    }

    // Validate dependencies using the dedicated dependency validation function
    let dep_errors = validate_dependencies(tasks);
    errors.extend(dep_errors);

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

/// Validates task dependencies for missing references and circular dependencies
///
/// This function performs focused validation of the task dependency graph using
/// graph traversal algorithms to detect two critical issues:
///
/// 1. **Missing Dependencies**: References to tasks that don't exist in the task list
/// 2. **Circular Dependencies**: Cycles in the dependency graph that would prevent
///    topological execution
///
/// # Arguments
/// * `tasks` - Slice of tasks to validate
///
/// # Returns
/// List of dependency validation errors found. Empty list indicates valid dependencies.
///
/// # Algorithm
/// - Missing dependency detection: O(n + d) where n=number of tasks, d=total dependencies
/// - Circular dependency detection: O(n + d) using DFS with cycle detection
///
/// # Examples
/// ```no_run
/// use ltmatrix::pipeline::generate::validate_dependencies;
/// use ltmatrix::models::Task;
///
/// let mut tasks = vec![
///     Task::new("task-1", "First", "No dependencies"),
///     Task::new("task-2", "Second", "Depends on first"),
/// ];
/// tasks[1].depends_on = vec!["task-1".to_string()];
///
/// let errors = validate_dependencies(&tasks);
/// assert!(errors.is_empty());
/// ```
pub fn validate_dependencies(tasks: &[Task]) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Build set of valid task IDs for O(1) lookup
    let task_ids: HashSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();

    // Check for missing dependencies
    let missing_deps = detect_missing_dependencies(tasks, &task_ids);
    errors.extend(missing_deps);

    // Check for circular dependencies
    let circular_chains = detect_circular_dependencies(tasks);
    errors.extend(circular_chains.into_iter().map(|chain| {
        ValidationError::CircularDependency { chain }
    }));

    errors
}

/// Enhanced dependency validation that returns detailed statistics
///
/// This is an extended version of `validate_dependencies()` that provides comprehensive
/// metrics about the dependency graph structure in addition to validation errors.
///
/// # Returns
/// A `DependencyValidationResult` containing validation status, errors, and detailed statistics
pub fn validate_dependencies_with_stats(tasks: &[Task]) -> DependencyValidationResult {
    // Build set of valid task IDs
    let task_ids: HashSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();

    // Check for missing dependencies
    let missing_deps = detect_missing_dependencies(tasks, &task_ids);
    let missing_count = missing_deps.len();

    // Convert to DependencyError
    let dep_errors: Vec<DependencyError> = missing_deps
        .into_iter()
        .map(|e| match e {
            ValidationError::MissingDependency { task, dependency } => {
                DependencyError::MissingReference {
                    task_id: task,
                    missing_ref: dependency,
                }
            }
            _ => unreachable!("detect_missing_dependencies only returns MissingDependency"),
        })
        .collect();

    // Check for circular dependencies
    let circular_chains = detect_circular_dependencies(tasks);
    let circular_count = circular_chains.len();

    let mut errors = dep_errors;
    errors.extend(circular_chains.iter().cloned().map(|chain| {
        DependencyError::CircularChain { chain }
    }));

    // Calculate graph statistics
    let stats = calculate_dependency_graph_stats(tasks, missing_count, circular_count);

    DependencyValidationResult {
        is_valid: errors.is_empty(),
        errors,
        stats,
    }
}

/// Calculates comprehensive statistics about the task dependency graph
fn calculate_dependency_graph_stats(
    tasks: &[Task],
    missing_count: usize,
    circular_count: usize,
) -> DependencyGraphStats {
    let total_tasks = tasks.len();
    let mut tasks_with_dependencies = 0;
    let mut total_dependencies = 0;

    // Build dependency adjacency map for depth calculation
    let mut dep_map: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut reverse_dep_map: HashMap<&str, Vec<&str>> = HashMap::new();

    for task in tasks {
        if !task.depends_on.is_empty() {
            tasks_with_dependencies += 1;
            total_dependencies += task.depends_on.len();

            // Build forward dependency map (what this task depends on)
            let deps: Vec<&str> = task.depends_on.iter().map(|s| s.as_str()).collect();
            dep_map.insert(task.id.as_str(), deps);

            // Build reverse dependency map (what depends on this task)
            for dep in &task.depends_on {
                reverse_dep_map
                    .entry(dep.as_str())
                    .or_insert_with(Vec::new)
                    .push(task.id.as_str());
            }
        } else {
            dep_map.insert(task.id.as_str(), Vec::new());
        }
    }

    // Calculate max depth
    let max_depth = calculate_dependency_depth(tasks);

    // Count root tasks (no dependencies) and leaf tasks (nothing depends on them)
    let root_tasks = tasks.iter().filter(|t| t.depends_on.is_empty()).count();
    let leaf_tasks = tasks
        .iter()
        .filter(|t| reverse_dep_map.get(t.id.as_str()).map_or(true, |v| v.is_empty()))
        .count();

    DependencyGraphStats {
        total_tasks,
        tasks_with_dependencies,
        total_dependencies,
        max_depth,
        root_tasks,
        leaf_tasks,
        missing_dependencies: missing_count,
        circular_dependencies: circular_count,
        is_dag: circular_count == 0 && missing_count == 0,
    }
}

/// Detects missing dependencies by checking if all referenced task IDs exist
///
/// # Algorithm
/// - Time Complexity: O(n * d) where n=number of tasks, d=avg dependencies per task
/// - Space Complexity: O(n) for the task_ids HashSet
///
/// # Returns
/// List of MissingDependency errors for each invalid reference
fn detect_missing_dependencies(
    tasks: &[Task],
    task_ids: &HashSet<&str>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

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
            generation_log: None,
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

    // Comprehensive tests for validate_dependencies()

    #[test]
    fn test_validate_dependencies_no_errors() {
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "First", "No dependencies");
                t.depends_on = vec![];
                t
            },
            {
                let mut t = Task::new("task-2", "Second", "Depends on first");
                t.depends_on = vec!["task-1".to_string()];
                t
            },
        ];

        let errors = validate_dependencies(&tasks);
        assert!(errors.is_empty(), "Should have no errors for valid dependencies");
    }

    #[test]
    fn test_validate_dependencies_single_missing() {
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "Task 1", "Has missing dependency");
                t.depends_on = vec!["non-existent".to_string()];
                t
            },
        ];

        let errors = validate_dependencies(&tasks);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            ValidationError::MissingDependency { task, .. } if task == "task-1"
        ));
    }

    #[test]
    fn test_validate_dependencies_multiple_missing() {
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "Task 1", "Missing deps");
                t.depends_on = vec!["missing-1".to_string(), "missing-2".to_string()];
                t
            },
        ];

        let errors = validate_dependencies(&tasks);
        assert_eq!(errors.len(), 2);
        assert!(errors.iter().all(|e| matches!(e, ValidationError::MissingDependency { .. })));
    }

    #[test]
    fn test_validate_dependencies_simple_cycle() {
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

        let errors = validate_dependencies(&tasks);
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            ValidationError::CircularDependency { chain } => {
                assert_eq!(chain.len(), 2);
                assert!(chain.contains(&"a".to_string()));
                assert!(chain.contains(&"b".to_string()));
            }
            _ => panic!("Expected CircularDependency error"),
        }
    }

    #[test]
    fn test_validate_dependencies_complex_cycle() {
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

        let errors = validate_dependencies(&tasks);
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            ValidationError::CircularDependency { chain } => {
                assert_eq!(chain.len(), 3);
            }
            _ => panic!("Expected CircularDependency error"),
        }
    }

    #[test]
    fn test_validate_dependencies_mixed_errors() {
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "Task 1", "Has missing dep");
                t.depends_on = vec!["missing".to_string()];
                t
            },
            {
                let mut t = Task::new("task-2", "Task 2", "Part of cycle");
                t.depends_on = vec!["task-3".to_string()];
                t
            },
            {
                let mut t = Task::new("task-3", "Task 3", "Part of cycle");
                t.depends_on = vec!["task-2".to_string()];
                t
            },
        ];

        let errors = validate_dependencies(&tasks);
        assert_eq!(errors.len(), 2); // 1 missing + 1 circular
        assert!(errors.iter().any(|e| matches!(e, ValidationError::MissingDependency { .. })));
        assert!(errors.iter().any(|e| matches!(e, ValidationError::CircularDependency { .. })));
    }

    #[test]
    fn test_validate_dependencies_empty_list() {
        let tasks: Vec<Task> = vec![];
        let errors = validate_dependencies(&tasks);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_dependencies_diamond_structure() {
        // Diamond dependency (should be valid - not a cycle)
        let tasks = vec![
            {
                let mut t = Task::new("a", "A", "Root");
                t.depends_on = vec![];
                t
            },
            {
                let mut t = Task::new("b", "B", "Left");
                t.depends_on = vec!["a".to_string()];
                t
            },
            {
                let mut t = Task::new("c", "C", "Right");
                t.depends_on = vec!["a".to_string()];
                t
            },
            {
                let mut t = Task::new("d", "D", "Bottom");
                t.depends_on = vec!["b".to_string(), "c".to_string()];
                t
            },
        ];

        let errors = validate_dependencies(&tasks);
        assert!(errors.is_empty(), "Diamond structure should be valid");
    }

    // Tests for validate_dependencies_with_stats()

    #[test]
    fn test_validate_dependencies_with_stats_valid_graph() {
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "First", "Root task");
                t.depends_on = vec![];
                t
            },
            {
                let mut t = Task::new("task-2", "Second", "Depends on first");
                t.depends_on = vec!["task-1".to_string()];
                t
            },
        ];

        let result = validate_dependencies_with_stats(&tasks);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
        assert_eq!(result.stats.total_tasks, 2);
        assert_eq!(result.stats.tasks_with_dependencies, 1);
        assert_eq!(result.stats.total_dependencies, 1);
        assert_eq!(result.stats.max_depth, 1);
        assert_eq!(result.stats.root_tasks, 1);
        assert_eq!(result.stats.leaf_tasks, 1);
        assert!(result.stats.is_dag);
    }

    #[test]
    fn test_validate_dependencies_with_stats_complex_graph() {
        let tasks = vec![
            {
                let mut t = Task::new("setup", "Setup", "Setup");
                t.depends_on = vec![];
                t
            },
            {
                let mut t = Task::new("implement", "Implement", "Implement");
                t.depends_on = vec!["setup".to_string()];
                t
            },
            {
                let mut t = Task::new("test", "Test", "Test");
                t.depends_on = vec!["implement".to_string()];
                t
            },
        ];

        let result = validate_dependencies_with_stats(&tasks);
        assert!(result.is_valid);
        assert_eq!(result.stats.total_tasks, 3);
        assert_eq!(result.stats.tasks_with_dependencies, 2);
        assert_eq!(result.stats.total_dependencies, 2);
        assert_eq!(result.stats.max_depth, 2);
        assert_eq!(result.stats.root_tasks, 1);
        assert_eq!(result.stats.leaf_tasks, 1);
    }

    #[test]
    fn test_validate_dependencies_with_stats_with_missing() {
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "Task 1", "Has missing dep");
                t.depends_on = vec!["missing".to_string()];
                t
            },
        ];

        let result = validate_dependencies_with_stats(&tasks);
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.stats.missing_dependencies, 1);
        assert!(!result.stats.is_dag);
    }

    #[test]
    fn test_validate_dependencies_with_stats_with_cycle() {
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

        let result = validate_dependencies_with_stats(&tasks);
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.stats.circular_dependencies, 1);
        assert!(!result.stats.is_dag);
    }

    #[test]
    fn test_validate_dependencies_with_stats_empty_tasks() {
        let tasks: Vec<Task> = vec![];
        let result = validate_dependencies_with_stats(&tasks);

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
        assert_eq!(result.stats.total_tasks, 0);
        assert_eq!(result.stats.tasks_with_dependencies, 0);
        assert_eq!(result.stats.total_dependencies, 0);
        assert_eq!(result.stats.max_depth, 0);
        assert!(result.stats.is_dag);
    }

    #[test]
    fn test_validate_dependencies_with_stats_independent_tasks() {
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "Task 1", "Independent");
                t.depends_on = vec![];
                t
            },
            {
                let mut t = Task::new("task-2", "Task 2", "Independent");
                t.depends_on = vec![];
                t
            },
            {
                let mut t = Task::new("task-3", "Task 3", "Independent");
                t.depends_on = vec![];
                t
            },
        ];

        let result = validate_dependencies_with_stats(&tasks);
        assert!(result.is_valid);
        assert_eq!(result.stats.root_tasks, 3); // All are roots
        assert_eq!(result.stats.leaf_tasks, 3); // All are leaves
        assert_eq!(result.stats.max_depth, 0);
        assert_eq!(result.stats.tasks_with_dependencies, 0);
    }

    #[test]
    fn test_validate_dependencies_with_stats_multi_parent() {
        // Task with multiple parents
        let tasks = vec![
            {
                let mut t = Task::new("a", "A", "Root");
                t.depends_on = vec![];
                t
            },
            {
                let mut t = Task::new("b", "B", "Branch");
                t.depends_on = vec!["a".to_string()];
                t
            },
            {
                let mut t = Task::new("c", "C", "Branch");
                t.depends_on = vec!["a".to_string()];
                t
            },
            {
                let mut t = Task::new("d", "D", "Multi-parent");
                t.depends_on = vec!["b".to_string(), "c".to_string()];
                t
            },
        ];

        let result = validate_dependencies_with_stats(&tasks);
        assert!(result.is_valid);
        assert_eq!(result.stats.total_dependencies, 4); // 0 + 1 + 1 + 2 = 4
        assert_eq!(result.stats.root_tasks, 1);
        assert_eq!(result.stats.leaf_tasks, 1);
    }
}
