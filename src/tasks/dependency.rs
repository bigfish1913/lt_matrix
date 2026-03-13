// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Dependency validation module
//!
//! This module provides comprehensive dependency validation for task graphs,
//! detecting cycles, unreachable tasks, and orphaned tasks.
//!
//! # Features
//!
//! - **Cycle Detection**: Identifies circular dependencies using DFS
//! - **Unreachable Task Detection**: Finds tasks that depend on non-existent tasks
//! - **Orphaned Task Detection**: Finds tasks with no dependencies and no dependents
//! - **Detailed Validation Reports**: Provides actionable error messages

use ltmatrix_core::Task;
use std::collections::{HashMap, HashSet};

/// Result of validating a task dependency graph
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the dependency graph is valid
    pub is_valid: bool,

    /// List of issues found during validation
    pub issues: Vec<ValidationIssue>,

    /// Statistics about the dependency graph
    pub stats: ValidationStats,
}

/// Individual validation issue
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationIssue {
    /// A cycle was detected in the dependency graph
    CycleDetected {
        /// The path of task IDs forming the cycle
        path: Vec<String>,
    },

    /// A task depends on a non-existent task
    UnreachableTask {
        /// The task ID with the invalid dependency
        task_id: String,
        /// The non-existent dependency ID
        missing_dependency: String,
    },

    /// A task has no dependencies and no dependents (not a root or leaf)
    OrphanedTask {
        /// The orphaned task ID
        task_id: String,
    },

    /// A task depends on itself
    SelfDependency {
        /// The task ID that depends on itself
        task_id: String,
    },
}

impl std::fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationIssue::CycleDetected { path } => {
                write!(f, "Cycle detected: {}", path.join(" -> "))
            }
            ValidationIssue::UnreachableTask {
                task_id,
                missing_dependency,
            } => {
                write!(
                    f,
                    "Task '{}' depends on non-existent task '{}'",
                    task_id, missing_dependency
                )
            }
            ValidationIssue::OrphanedTask { task_id } => {
                write!(
                    f,
                    "Task '{}' is orphaned (no dependencies and no dependents)",
                    task_id
                )
            }
            ValidationIssue::SelfDependency { task_id } => {
                write!(f, "Task '{}' depends on itself", task_id)
            }
        }
    }
}

/// Statistics about the validated task graph
#[derive(Debug, Clone, Default)]
pub struct ValidationStats {
    /// Total number of tasks
    pub total_tasks: usize,

    /// Number of root tasks (no dependencies)
    pub root_tasks: usize,

    /// Number of leaf tasks (no dependents)
    pub leaf_tasks: usize,

    /// Maximum depth of the dependency graph
    pub max_depth: usize,

    /// Total number of dependency edges
    pub total_edges: usize,

    /// Number of tasks with dependencies
    pub tasks_with_deps: usize,
}

impl ValidationResult {
    /// Creates a new validation result
    pub fn new() -> Self {
        Self {
            is_valid: true,
            issues: Vec::new(),
            stats: ValidationStats::default(),
        }
    }

    /// Returns true if there are any cycle issues
    pub fn has_cycles(&self) -> bool {
        self.issues
            .iter()
            .any(|i| matches!(i, ValidationIssue::CycleDetected { .. }))
    }

    /// Returns true if there are any unreachable task issues
    pub fn has_unreachable(&self) -> bool {
        self.issues
            .iter()
            .any(|i| matches!(i, ValidationIssue::UnreachableTask { .. }))
    }

    /// Returns true if there are any orphaned task issues
    pub fn has_orphans(&self) -> bool {
        self.issues
            .iter()
            .any(|i| matches!(i, ValidationIssue::OrphanedTask { .. }))
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Validates a task dependency graph
///
/// # Arguments
///
/// * `tasks` - The list of tasks to validate
///
/// # Returns
///
/// Returns a `ValidationResult` containing the validation status,
/// any issues found, and statistics about the graph.
///
/// # Example
///
/// ```no_run
/// use ltmatrix::tasks::dependency::validate_task_graph;
/// use ltmatrix::models::Task;
///
/// let tasks = vec![
///     Task::new("task-1", "First", "No dependencies"),
///     Task::new("task-2", "Second", "Depends on first"),
/// ];
///
/// let result = validate_task_graph(&tasks);
/// if !result.is_valid {
///     for issue in &result.issues {
///         println!("Issue: {}", issue);
///     }
/// }
/// ```
pub fn validate_task_graph(tasks: &[Task]) -> ValidationResult {
    let mut result = ValidationResult::new();

    if tasks.is_empty() {
        return result;
    }

    // Build task ID set for quick lookup
    let task_ids: HashSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();

    // Build dependency graph
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut total_edges = 0;
    let mut tasks_with_deps = 0;

    for task in tasks {
        if !task.depends_on.is_empty() {
            tasks_with_deps += 1;
        }

        for dep in &task.depends_on {
            total_edges += 1;
            dependents
                .entry(dep.as_str())
                .or_default()
                .push(task.id.as_str());
        }
    }

    // Check for self-dependencies
    for task in tasks {
        if task.depends_on.contains(&task.id) {
            result.issues.push(ValidationIssue::SelfDependency {
                task_id: task.id.clone(),
            });
            result.is_valid = false;
        }
    }

    // Check for unreachable tasks (dependencies that don't exist)
    for task in tasks {
        for dep in &task.depends_on {
            if !task_ids.contains(dep.as_str()) {
                result.issues.push(ValidationIssue::UnreachableTask {
                    task_id: task.id.clone(),
                    missing_dependency: dep.clone(),
                });
                result.is_valid = false;
            }
        }
    }

    // Check for cycles (only if no unreachable tasks, as they could cause false positives)
    if !result.has_unreachable() {
        let cycles = detect_cycles(tasks, &task_ids);
        for cycle in cycles {
            result
                .issues
                .push(ValidationIssue::CycleDetected { path: cycle });
            result.is_valid = false;
        }
    }

    // Check for orphaned tasks (only if graph is otherwise valid)
    if result.is_valid {
        let root_ids: HashSet<&str> = tasks
            .iter()
            .filter(|t| t.depends_on.is_empty())
            .map(|t| t.id.as_str())
            .collect();

        let leaf_ids: HashSet<&str> = tasks
            .iter()
            .filter(|t| dependents.get(t.id.as_str()).map_or(true, |v| v.is_empty()))
            .map(|t| t.id.as_str())
            .collect();

        // An orphan is a task that is neither a root nor a leaf
        // AND is not part of a connected component with other tasks
        // For simplicity, we flag tasks that have no deps AND no dependents
        for task in tasks {
            if task.depends_on.is_empty()
                && dependents
                    .get(task.id.as_str())
                    .map_or(true, |v| v.is_empty())
            {
                // This is both a root and a leaf - could be orphaned if there are other tasks
                if tasks.len() > 1 {
                    result.issues.push(ValidationIssue::OrphanedTask {
                        task_id: task.id.clone(),
                    });
                }
            }
        }
    }

    // Calculate statistics
    let root_tasks = tasks.iter().filter(|t| t.depends_on.is_empty()).count();
    let leaf_tasks = tasks
        .iter()
        .filter(|t| dependents.get(t.id.as_str()).map_or(true, |v| v.is_empty()))
        .count();

    let max_depth = calculate_max_depth(tasks);

    result.stats = ValidationStats {
        total_tasks: tasks.len(),
        root_tasks,
        leaf_tasks,
        max_depth,
        total_edges,
        tasks_with_deps,
    };

    result
}

/// Detects cycles in the task dependency graph
fn detect_cycles(tasks: &[Task], task_ids: &HashSet<&str>) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();

    // Build adjacency list
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for task in tasks {
        let deps: Vec<&str> = task
            .depends_on
            .iter()
            .filter(|dep| task_ids.contains(dep.as_str()))
            .map(|s| s.as_str())
            .collect();
        adj.insert(task.id.as_str(), deps);
    }

    // DFS cycle detection
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    for task_id in task_ids {
        if !visited.contains(task_id) {
            if let Some(cycle) = dfs_cycle(*task_id, &adj, &mut visited, &mut rec_stack, &mut path)
            {
                cycles.push(cycle);
            }
        }
    }

    cycles
}

/// DFS helper for cycle detection
fn dfs_cycle<'a>(
    node: &'a str,
    adj: &HashMap<&'a str, Vec<&'a str>>,
    visited: &mut HashSet<&'a str>,
    rec_stack: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<String>> {
    visited.insert(node);
    rec_stack.insert(node);
    path.push(node);

    if let Some(neighbors) = adj.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                if let Some(cycle) = dfs_cycle(*neighbor, adj, visited, rec_stack, path) {
                    return Some(cycle);
                }
            } else if rec_stack.contains(neighbor) {
                // Found a cycle - extract it from path
                let cycle_start = path.iter().position(|&n| n == *neighbor).unwrap_or(0);
                let cycle: Vec<String> =
                    path[cycle_start..].iter().map(|s| s.to_string()).collect();
                return Some(cycle);
            }
        }
    }

    path.pop();
    rec_stack.remove(node);
    None
}

/// Calculates the maximum depth of the dependency graph
fn calculate_max_depth(tasks: &[Task]) -> usize {
    let task_ids: HashSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();

    let mut depth_map: HashMap<&str, usize> = HashMap::new();
    for task in tasks {
        depth_map.insert(task.id.as_str(), 0);
    }

    // Iteratively calculate depths
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

            let max_dep_depth = task
                .depends_on
                .iter()
                .filter(|dep| task_ids.contains(dep.as_str()))
                .filter_map(|dep| depth_map.get(dep.as_str()).copied())
                .max()
                .unwrap_or(0);

            let new_depth = max_dep_depth + 1;
            if new_depth > *depth_map.get(task.id.as_str()).unwrap_or(&0) {
                depth_map.insert(task.id.as_str(), new_depth);
                changed = true;
            }
        }
    }

    depth_map.values().copied().max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: &str, depends_on: Vec<&str>) -> Task {
        let mut task = Task::new(id, id, id);
        task.depends_on = depends_on.iter().map(|s| s.to_string()).collect();
        task
    }

    #[test]
    fn test_validate_empty_graph() {
        let tasks: Vec<Task> = vec![];
        let result = validate_task_graph(&tasks);
        assert!(result.is_valid);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn test_validate_single_task() {
        let tasks = vec![make_task("task-1", vec![])];
        let result = validate_task_graph(&tasks);
        assert!(result.is_valid);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn test_validate_valid_chain() {
        let tasks = vec![
            make_task("task-1", vec![]),
            make_task("task-2", vec!["task-1"]),
            make_task("task-3", vec!["task-2"]),
        ];
        let result = validate_task_graph(&tasks);
        assert!(result.is_valid);
        assert!(result.issues.is_empty());
        assert_eq!(result.stats.max_depth, 2);
    }

    #[test]
    fn test_validate_simple_cycle() {
        let tasks = vec![
            make_task("task-1", vec!["task-2"]),
            make_task("task-2", vec!["task-1"]),
        ];
        let result = validate_task_graph(&tasks);
        assert!(!result.is_valid);
        assert!(result.has_cycles());
    }

    #[test]
    fn test_validate_three_node_cycle() {
        let tasks = vec![
            make_task("task-1", vec!["task-2"]),
            make_task("task-2", vec!["task-3"]),
            make_task("task-3", vec!["task-1"]),
        ];
        let result = validate_task_graph(&tasks);
        assert!(!result.is_valid);
        assert!(result.has_cycles());
    }

    #[test]
    fn test_validate_unreachable_dependency() {
        let tasks = vec![
            make_task("task-1", vec![]),
            make_task("task-2", vec!["nonexistent"]),
        ];
        let result = validate_task_graph(&tasks);
        assert!(!result.is_valid);
        assert!(result.has_unreachable());
    }

    #[test]
    fn test_validate_self_dependency() {
        let tasks = vec![make_task("task-1", vec!["task-1"])];
        let result = validate_task_graph(&tasks);
        assert!(!result.is_valid);
        assert!(result
            .issues
            .iter()
            .any(|i| matches!(i, ValidationIssue::SelfDependency { .. })));
    }

    #[test]
    fn test_validate_diamond_structure() {
        // Diamond: A -> B, A -> C, B -> D, C -> D
        let tasks = vec![
            make_task("A", vec![]),
            make_task("B", vec!["A"]),
            make_task("C", vec!["A"]),
            make_task("D", vec!["B", "C"]),
        ];
        let result = validate_task_graph(&tasks);
        assert!(result.is_valid);
        assert_eq!(result.stats.root_tasks, 1);
        assert_eq!(result.stats.leaf_tasks, 1);
    }

    #[test]
    fn test_validate_orphaned_task() {
        let tasks = vec![
            make_task("task-1", vec![]),
            make_task("task-2", vec![]), // orphaned
        ];
        let result = validate_task_graph(&tasks);
        assert!(result.has_orphans());
    }

    #[test]
    fn test_validation_stats() {
        let tasks = vec![
            make_task("task-1", vec![]),
            make_task("task-2", vec!["task-1"]),
            make_task("task-3", vec!["task-1"]),
            make_task("task-4", vec!["task-2", "task-3"]),
        ];
        let result = validate_task_graph(&tasks);

        assert!(result.is_valid);
        assert_eq!(result.stats.total_tasks, 4);
        assert_eq!(result.stats.root_tasks, 1);
        assert_eq!(result.stats.leaf_tasks, 1);
        assert_eq!(result.stats.max_depth, 2);
        assert_eq!(result.stats.total_edges, 4);
        assert_eq!(result.stats.tasks_with_deps, 3);
    }

    #[test]
    fn test_validation_issue_display() {
        let issue = ValidationIssue::CycleDetected {
            path: vec!["A".to_string(), "B".to_string(), "A".to_string()],
        };
        assert_eq!(format!("{}", issue), "Cycle detected: A -> B -> A");

        let issue = ValidationIssue::UnreachableTask {
            task_id: "task-1".to_string(),
            missing_dependency: "missing".to_string(),
        };
        assert!(format!("{}", issue).contains("task-1"));
        assert!(format!("{}", issue).contains("missing"));
    }
}
