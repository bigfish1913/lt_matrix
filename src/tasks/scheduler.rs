// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Task dependency scheduler with topological sorting
//!
//! This module implements a sophisticated task scheduling system that:
//! - Builds dependency graphs from task.depends_on arrays
//! - Performs topological sorting to determine execution order
//! - Detects and reports circular dependencies
//! - Calculates critical path for the task graph
//! - Identifies parallelizable tasks to maximize throughput
//! - Returns execution order that preserves dependencies while maximizing parallelism

use anyhow::{bail, Result};
use std::collections::{HashMap, HashSet, VecDeque};
use tracing::{debug, info, warn};

use crate::models::Task;

/// Execution plan with optimized task ordering
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// Tasks grouped by execution level (can be executed in parallel within each level)
    pub execution_levels: Vec<Vec<Task>>,

    /// Complete flattened execution order (respecting dependencies)
    pub execution_order: Vec<String>,

    /// Critical path through the task graph (longest dependency chain)
    pub critical_path: Vec<String>,

    /// Tasks that can be parallelized (not on critical path)
    pub parallelizable_tasks: HashSet<String>,

    /// Maximum depth of the dependency graph
    pub max_depth: usize,

    /// Total number of tasks
    pub total_tasks: usize,
}

/// Result of cycle detection
#[derive(Debug, Clone)]
pub struct CycleDetectionResult {
    /// Whether a cycle was detected
    pub has_cycle: bool,

    /// The cycle path if detected (e.g., ["task-1", "task-2", "task-1"])
    pub cycle_path: Vec<String>,
}

/// Statistics about the task graph
#[derive(Debug, Clone)]
pub struct GraphStatistics {
    /// Total number of tasks
    pub total_tasks: usize,

    /// Number of dependency edges
    pub total_edges: usize,

    /// Number of tasks with no dependencies
    pub root_tasks: usize,

    /// Number of tasks with no dependents
    pub leaf_tasks: usize,

    /// Maximum depth of the dependency graph
    pub max_depth: usize,

    /// Number of tasks on the critical path
    pub critical_path_length: usize,

    /// Estimated parallelism (how many tasks can run in parallel on average)
    pub parallelism_factor: f64,
}

/// Builds a dependency graph and performs topological sorting
pub fn schedule_tasks(tasks: Vec<Task>) -> Result<ExecutionPlan> {
    info!("Building execution plan for {} tasks", tasks.len());

    // Build task map for quick lookup
    let task_map: HashMap<String, Task> = tasks
        .into_iter()
        .map(|task| (task.id.clone(), task))
        .collect();

    // Validate dependencies exist
    validate_dependencies(&task_map)?;

    // Detect cycles
    let cycle_result = detect_cycles(&task_map)?;
    if cycle_result.has_cycle {
        bail!(
            "Circular dependency detected: {}",
            cycle_result.cycle_path.join(" -> ")
        );
    }

    // Build adjacency list for dependency graph
    let graph = build_dependency_graph(&task_map);

    // Perform topological sort
    let execution_order = topological_sort(&task_map, &graph)?;

    // Calculate execution levels (tasks that can run in parallel)
    let execution_levels = calculate_execution_levels(&task_map, &graph, &execution_order);

    // Calculate critical path
    let critical_path = calculate_critical_path(&task_map, &graph);

    // Identify parallelizable tasks
    let critical_path_set: HashSet<String> = critical_path.iter().cloned().collect();
    let parallelizable_tasks = identify_parallelizable_tasks(&task_map, &critical_path_set);

    // Calculate max depth
    let max_depth = execution_levels.len();

    info!(
        "Execution plan created: {} levels, {} tasks on critical path",
        max_depth,
        critical_path.len()
    );

    Ok(ExecutionPlan {
        execution_levels,
        execution_order,
        critical_path,
        parallelizable_tasks,
        max_depth,
        total_tasks: task_map.len(),
    })
}

/// Validates that all dependency references point to existing tasks
fn validate_dependencies(task_map: &HashMap<String, Task>) -> Result<()> {
    let mut missing_deps = Vec::new();

    for (task_id, task) in task_map.iter() {
        for dep_id in &task.depends_on {
            if !task_map.contains_key(dep_id) {
                missing_deps.push(format!(
                    "{} depends on non-existent task {}",
                    task_id, dep_id
                ));
            }
        }
    }

    if !missing_deps.is_empty() {
        bail!("Missing task dependencies:\n{}", missing_deps.join("\n"));
    }

    debug!("All dependencies validated successfully");
    Ok(())
}

/// Builds an adjacency list representation of the dependency graph
/// Returns a map where key = task_id, value = list of tasks that depend on it
fn build_dependency_graph(task_map: &HashMap<String, Task>) -> HashMap<String, Vec<String>> {
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();

    // Initialize all tasks with empty adjacency lists
    for task_id in task_map.keys() {
        graph.insert(task_id.clone(), Vec::new());
    }

    // Build edges: if A depends on B, add edge B -> A
    for (task_id, task) in task_map.iter() {
        for dep_id in &task.depends_on {
            graph
                .entry(dep_id.clone())
                .or_insert_with(Vec::new)
                .push(task_id.clone());
        }
    }

    debug!(
        "Built dependency graph with {} nodes and {} edges",
        graph.len(),
        graph.values().map(|v| v.len()).sum::<usize>()
    );

    graph
}

/// Detects circular dependencies using DFS with coloring
fn detect_cycles(task_map: &HashMap<String, Task>) -> Result<CycleDetectionResult> {
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    for task_id in task_map.keys() {
        if !visited.contains(task_id) {
            if let Some(cycle) =
                dfs_detect_cycle(task_id, task_map, &mut visited, &mut rec_stack, &mut path)
            {
                return Ok(CycleDetectionResult {
                    has_cycle: true,
                    cycle_path: cycle,
                });
            }
        }
    }

    Ok(CycleDetectionResult {
        has_cycle: false,
        cycle_path: Vec::new(),
    })
}

/// DFS helper for cycle detection
fn dfs_detect_cycle(
    task_id: &str,
    task_map: &HashMap<String, Task>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    visited.insert(task_id.to_string());
    rec_stack.insert(task_id.to_string());
    path.push(task_id.to_string());

    // Visit all dependencies
    if let Some(task) = task_map.get(task_id) {
        for dep_id in &task.depends_on {
            if !visited.contains(dep_id) {
                if let Some(cycle) = dfs_detect_cycle(dep_id, task_map, visited, rec_stack, path) {
                    return Some(cycle);
                }
            } else if rec_stack.contains(dep_id) {
                // Found a cycle
                let cycle_start = path.iter().position(|id| id == dep_id).unwrap();
                let mut cycle = path[cycle_start..].to_vec();
                cycle.push(dep_id.to_string());
                return Some(cycle);
            }
        }
    }

    path.pop();
    rec_stack.remove(task_id);
    None
}

/// Performs topological sort using Kahn's algorithm
fn topological_sort(
    task_map: &HashMap<String, Task>,
    graph: &HashMap<String, Vec<String>>,
) -> Result<Vec<String>> {
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    // Calculate in-degrees (number of dependencies for each task)
    for task_id in task_map.keys() {
        in_degree.insert(task_id.clone(), task_map[task_id].depends_on.len());
    }

    // Initialize queue with tasks that have no dependencies
    let mut queue: VecDeque<String> = task_map
        .keys()
        .filter(|id| in_degree[*id] == 0)
        .cloned()
        .collect();

    let mut result = Vec::new();

    while let Some(task_id) = queue.pop_front() {
        result.push(task_id.clone());

        // Reduce in-degree for all dependent tasks
        if let Some(dependents) = graph.get(&task_id) {
            for dep_id in dependents {
                if let Some(degree) = in_degree.get_mut(dep_id) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dep_id.clone());
                    }
                }
            }
        }
    }

    // Check if we processed all tasks (no cycles)
    if result.len() != task_map.len() {
        bail!(
            "Topological sort failed: processed {} tasks but {} tasks exist (possible cycle)",
            result.len(),
            task_map.len()
        );
    }

    debug!("Topological sort produced execution order: {:?}", result);
    Ok(result)
}

/// Groups tasks into execution levels where tasks in the same level can run in parallel
fn calculate_execution_levels(
    task_map: &HashMap<String, Task>,
    _graph: &HashMap<String, Vec<String>>,
    execution_order: &[String],
) -> Vec<Vec<Task>> {
    let mut levels: Vec<Vec<Task>> = Vec::new();
    let mut processed: HashSet<String> = HashSet::new();
    let mut remaining_tasks: HashSet<String> = execution_order.iter().cloned().collect();

    while !remaining_tasks.is_empty() {
        let mut current_level = Vec::new();

        // Find all tasks whose dependencies are satisfied
        for task_id in remaining_tasks.iter() {
            let task = &task_map[task_id];

            // Check if all dependencies are processed
            let can_execute = task
                .depends_on
                .iter()
                .all(|dep_id| processed.contains(dep_id));

            if can_execute {
                current_level.push(task.clone());
            }
        }

        // Add current level to the plan
        if !current_level.is_empty() {
            for task in &current_level {
                processed.insert(task.id.clone());
                remaining_tasks.remove(&task.id);
            }
            let level_size = current_level.len();
            levels.push(current_level);

            debug!(
                "Level {}: {} tasks can execute in parallel",
                levels.len(),
                level_size
            );
        } else {
            // This should never happen if we validated cycles
            warn!("No tasks ready for execution but some remain unprocessed");
            break;
        }
    }

    levels
}

/// Calculates the critical path (longest path through the dependency graph)
fn calculate_critical_path(
    task_map: &HashMap<String, Task>,
    graph: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut memo: HashMap<String, (Vec<String>, usize)> = HashMap::new();

    // Calculate longest path starting from each task
    let mut best_path = Vec::new();
    let mut best_length = 0;

    for task_id in task_map.keys() {
        let (path, length) = longest_path(task_id, task_map, graph, &mut memo);
        if length > best_length {
            best_length = length;
            best_path = path;
        }
    }

    debug!("Critical path: {:?} (length: {})", best_path, best_length);
    best_path
}

/// Recursively calculates the longest path from a given task using memoization
fn longest_path(
    task_id: &str,
    task_map: &HashMap<String, Task>,
    graph: &HashMap<String, Vec<String>>,
    memo: &mut HashMap<String, (Vec<String>, usize)>,
) -> (Vec<String>, usize) {
    if let Some(cached) = memo.get(task_id) {
        return cached.clone();
    }

    let dependents = graph.get(task_id).map(|v| v.as_slice()).unwrap_or(&[]);

    if dependents.is_empty() {
        // Leaf node
        let result = (vec![task_id.to_string()], 1);
        memo.insert(task_id.to_string(), result.clone());
        return result;
    }

    let mut best_path = Vec::new();
    let mut max_length = 0;

    for dep_id in dependents {
        let (path, length) = longest_path(dep_id, task_map, graph, memo);
        if length > max_length {
            max_length = length;
            best_path = path;
        }
    }

    let mut result = vec![task_id.to_string()];
    result.extend(best_path);
    let result_length = result.len();

    memo.insert(task_id.to_string(), (result.clone(), result_length));
    (result, result_length)
}

/// Identifies tasks that are not on the critical path (can be parallelized)
fn identify_parallelizable_tasks(
    task_map: &HashMap<String, Task>,
    critical_path: &HashSet<String>,
) -> HashSet<String> {
    let critical_set: HashSet<String> = critical_path.iter().cloned().collect();

    let parallelizable: HashSet<String> = task_map
        .keys()
        .filter(|id| !critical_set.contains(*id))
        .cloned()
        .collect();

    debug!(
        "Identified {} parallelizable tasks out of {} total",
        parallelizable.len(),
        task_map.len()
    );

    parallelizable
}

/// Calculates comprehensive statistics about the task graph
pub fn calculate_graph_statistics(task_map: &HashMap<String, Task>) -> Result<GraphStatistics> {
    let graph = build_dependency_graph(task_map);
    let total_tasks = task_map.len();
    let total_edges = graph.values().map(|v| v.len()).sum();

    // Count root tasks (no dependencies) and leaf tasks (no dependents)
    let root_tasks = task_map
        .values()
        .filter(|t| t.depends_on.is_empty())
        .count();

    let leaf_tasks = task_map
        .keys()
        .filter(|id| graph.get(*id).map_or(true, |deps| deps.is_empty()))
        .count();

    // Calculate critical path for depth
    let critical_path = calculate_critical_path(task_map, &graph);
    let max_depth = critical_path.len();
    let critical_path_length = critical_path.len();

    // Calculate parallelism factor
    // Average number of tasks that can run in parallel
    let parallelism_factor = if total_tasks > 0 && max_depth > 0 {
        total_tasks as f64 / max_depth as f64
    } else {
        1.0
    };

    Ok(GraphStatistics {
        total_tasks,
        total_edges,
        root_tasks,
        leaf_tasks,
        max_depth,
        critical_path_length,
        parallelism_factor,
    })
}

/// Generates a Mermaid diagram of the task dependency graph
pub fn generate_mermaid_diagram(task_map: &HashMap<String, Task>) -> String {
    let mut diagram = String::from("graph TD\n");

    for (task_id, task) in task_map.iter() {
        for dep_id in &task.depends_on {
            diagram.push_str(&format!("  {} --> {}\n", dep_id, task_id));
        }
    }

    diagram
}

/// Generates an ASCII visualization of the execution plan
pub fn visualize_execution_plan(plan: &ExecutionPlan) -> String {
    let mut visualization = String::from("Execution Plan:\n");
    visualization.push_str(&format!("Total Tasks: {}\n", plan.total_tasks));
    visualization.push_str(&format!("Max Depth: {}\n", plan.max_depth));
    visualization.push_str(&format!(
        "Critical Path Length: {}\n",
        plan.critical_path.len()
    ));
    visualization.push_str(&format!(
        "Parallelizable Tasks: {}\n",
        plan.parallelizable_tasks.len()
    ));
    visualization.push_str("\nExecution Levels:\n");

    for (level, tasks) in plan.execution_levels.iter().enumerate() {
        visualization.push_str(&format!("Level {}: ", level + 1));
        let task_ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
        visualization.push_str(&format!("{} (parallel)\n", task_ids.join(", ")));
    }

    visualization.push_str("\nCritical Path:\n");
    for (i, task_id) in plan.critical_path.iter().enumerate() {
        visualization.push_str(&format!("{}. {}\n", i + 1, task_id));
    }

    visualization
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_task(id: &str, deps: Vec<&str>) -> Task {
        let mut task = Task::new(id, id, format!("Task {}", id));
        task.depends_on = deps.into_iter().map(|s| s.to_string()).collect();
        task
    }

    #[test]
    fn test_simple_linear_chain() {
        let tasks = vec![
            create_test_task("task-1", vec![]),
            create_test_task("task-2", vec!["task-1"]),
            create_test_task("task-3", vec!["task-2"]),
        ];

        let plan = schedule_tasks(tasks).unwrap();

        assert_eq!(plan.execution_order, vec!["task-1", "task-2", "task-3"]);
        assert_eq!(plan.max_depth, 3); // Each task in its own level
        assert_eq!(plan.critical_path, vec!["task-1", "task-2", "task-3"]);
    }

    #[test]
    fn test_parallel_tasks() {
        let tasks = vec![
            create_test_task("task-1", vec![]),
            create_test_task("task-2", vec![]),
            create_test_task("task-3", vec!["task-1", "task-2"]),
        ];

        let plan = schedule_tasks(tasks).unwrap();

        // First level should have task-1 and task-2 in parallel
        assert_eq!(plan.execution_levels[0].len(), 2);
        assert_eq!(plan.execution_levels[1].len(), 1);
        assert_eq!(plan.max_depth, 2);
    }

    #[test]
    fn test_diamond_dependency() {
        let tasks = vec![
            create_test_task("task-1", vec![]),
            create_test_task("task-2", vec!["task-1"]),
            create_test_task("task-3", vec!["task-1"]),
            create_test_task("task-4", vec!["task-2", "task-3"]),
        ];

        let plan = schedule_tasks(tasks).unwrap();

        assert_eq!(plan.execution_levels[0].len(), 1); // task-1
        assert_eq!(plan.execution_levels[1].len(), 2); // task-2, task-3
        assert_eq!(plan.execution_levels[2].len(), 1); // task-4
        assert_eq!(plan.max_depth, 3);
    }

    #[test]
    fn test_cycle_detection() {
        let tasks = vec![
            create_test_task("task-1", vec!["task-2"]),
            create_test_task("task-2", vec!["task-3"]),
            create_test_task("task-3", vec!["task-1"]),
        ];

        let result = schedule_tasks(tasks);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circular dependency"));
    }

    #[test]
    fn test_self_cycle_detection() {
        let tasks = vec![create_test_task("task-1", vec!["task-1"])];

        let result = schedule_tasks(tasks);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_dependency() {
        let tasks = vec![
            create_test_task("task-1", vec![]),
            create_test_task("task-2", vec!["task-999"]), // Non-existent
        ];

        let result = schedule_tasks(tasks);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("non-existent task"));
    }

    #[test]
    fn test_critical_path_calculation() {
        let tasks = vec![
            create_test_task("task-1", vec![]),
            create_test_task("task-2", vec!["task-1"]),
            create_test_task("task-3", vec!["task-1"]),
            create_test_task("task-4", vec!["task-2"]),
            create_test_task("task-5", vec!["task-2"]),
            create_test_task("task-6", vec!["task-3"]),
        ];

        let plan = schedule_tasks(tasks).unwrap();

        // Critical path should start with task-1 and have length 3
        assert_eq!(plan.critical_path[0], "task-1");
        assert_eq!(plan.critical_path.len(), 3);

        // Path should be either task-1 -> task-2 -> task-4/task-5 or task-1 -> task-3 -> task-6
        assert!(
            plan.critical_path.contains(&"task-2".to_string())
                || plan.critical_path.contains(&"task-3".to_string())
        );
    }

    #[test]
    fn test_graph_statistics() {
        let tasks = vec![
            create_test_task("task-1", vec![]),
            create_test_task("task-2", vec![]),
            create_test_task("task-3", vec!["task-1", "task-2"]),
        ];

        let task_map: HashMap<String, Task> =
            tasks.into_iter().map(|t| (t.id.clone(), t)).collect();

        let stats = calculate_graph_statistics(&task_map).unwrap();

        assert_eq!(stats.total_tasks, 3);
        assert_eq!(stats.root_tasks, 2); // task-1 and task-2
        assert_eq!(stats.leaf_tasks, 1); // task-3
        assert!(stats.parallelism_factor > 1.0);
    }

    #[test]
    fn test_empty_task_list() {
        let tasks = vec![];
        let plan = schedule_tasks(tasks).unwrap();

        assert_eq!(plan.total_tasks, 0);
        assert_eq!(plan.execution_levels.len(), 0);
        assert_eq!(plan.execution_order.len(), 0);
    }

    #[test]
    fn test_single_task() {
        let tasks = vec![create_test_task("task-1", vec![])];
        let plan = schedule_tasks(tasks).unwrap();

        assert_eq!(plan.total_tasks, 1);
        assert_eq!(plan.execution_levels.len(), 1);
        assert_eq!(plan.execution_order, vec!["task-1"]);
    }

    #[test]
    fn test_mermaid_diagram_generation() {
        let tasks = vec![
            create_test_task("task-1", vec![]),
            create_test_task("task-2", vec!["task-1"]),
        ];

        let task_map: HashMap<String, Task> =
            tasks.into_iter().map(|t| (t.id.clone(), t)).collect();

        let diagram = generate_mermaid_diagram(&task_map);

        assert!(diagram.contains("graph TD"));
        assert!(diagram.contains("task-1 --> task-2"));
    }

    #[test]
    fn test_execution_plan_visualization() {
        let tasks = vec![
            create_test_task("task-1", vec![]),
            create_test_task("task-2", vec!["task-1"]),
        ];

        let plan = schedule_tasks(tasks).unwrap();
        let visualization = visualize_execution_plan(&plan);

        assert!(visualization.contains("Execution Plan"));
        assert!(visualization.contains("Total Tasks: 2"));
        assert!(visualization.contains("Critical Path"));
    }

    #[test]
    fn test_identify_parallelizable_tasks() {
        let tasks = vec![
            create_test_task("task-1", vec![]),
            create_test_task("task-2", vec!["task-1"]),
            create_test_task("task-3", vec!["task-1"]),
        ];

        let plan = schedule_tasks(tasks).unwrap();

        // task-1 must be on critical path (it's the root)
        assert!(plan.critical_path.contains(&"task-1".to_string()));

        // One of task-2 or task-3 should be parallelizable (equal length paths)
        // The implementation may choose either as the critical path
        assert!(
            plan.parallelizable_tasks.contains(&"task-2".to_string())
                || plan.parallelizable_tasks.contains(&"task-3".to_string()),
            "Either task-2 or task-3 should be parallelizable"
        );
    }
}
