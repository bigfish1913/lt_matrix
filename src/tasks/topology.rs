// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Topological visualization of task dependencies and execution order
//!
//! This module provides ASCII art visualization for:
//! - Dependency DAG (Directed Acyclic Graph)
//! - Execution order with levels
//! - Critical path highlighting
//! - Parallel task grouping
//!
//! Also provides Mermaid diagram generation for:
//! - Flowchart representation of task dependencies
//! - Graph visualization of execution plans
//! - Export to .mmd files

use ltmatrix_core::{Task, TaskStatus};
use crate::tasks::scheduler::ExecutionPlan;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Configuration for topology visualization
#[derive(Debug, Clone, Copy)]
pub struct TopologyConfig {
    /// Show task status in visualization
    pub show_status: bool,

    /// Highlight critical path
    pub highlight_critical: bool,

    /// Show execution levels
    pub show_levels: bool,

    /// Use compact mode (less whitespace)
    pub compact: bool,
}

impl Default for TopologyConfig {
    fn default() -> Self {
        TopologyConfig {
            show_status: true,
            highlight_critical: true,
            show_levels: true,
            compact: false,
        }
    }
}

/// Visualizes the dependency graph as ASCII art
///
/// # Arguments
///
/// * `tasks` - Slice of tasks to visualize
/// * `config` - Optional visualization configuration
///
/// # Returns
///
/// A string containing the ASCII art visualization
#[must_use]
pub fn visualize_dependency_graph(tasks: &[Task], config: Option<TopologyConfig>) -> String {
    let config = config.unwrap_or_default();
    let mut result = String::new();

    // Build task map for quick lookup
    let task_map: HashMap<&str, &Task> = tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect();

    // Find root tasks (no dependencies)
    let root_tasks: Vec<&Task> = tasks
        .iter()
        .filter(|task| task.depends_on.is_empty())
        .map(|task| task)
        .collect();

    // Header
    result.push_str("╔════════════════════════════════════════════════════════════╗\n");
    result.push_str("║           Task Dependency Graph (DAG)                        ║\n");
    result.push_str("╚════════════════════════════════════════════════════════════╝\n\n");

    if root_tasks.is_empty() && !tasks.is_empty() {
        result.push_str("⚠ Warning: Circular dependencies detected - no root tasks found!\n");
        return result;
    }

    if tasks.is_empty() {
        result.push_str("No tasks to visualize.\n");
        return result;
    }

    // Build adjacency structure for visualization
    for (index, root) in root_tasks.iter().enumerate() {
        if index > 0 {
            result.push_str("\n");
        }
        visualize_dependency_tree(root, &task_map, "", true, &config, &mut result);
    }

    result
}

/// Visualizes execution plan with levels and order
///
/// # Arguments
///
/// * `plan` - The execution plan to visualize
/// * `config` - Optional visualization configuration
///
/// # Returns
///
/// A string containing the ASCII art visualization
#[must_use]
pub fn visualize_execution_plan(plan: &ExecutionPlan, config: Option<TopologyConfig>) -> String {
    let config = config.unwrap_or_default();
    let mut result = String::new();

    // Header
    result.push_str("╔════════════════════════════════════════════════════════════╗\n");
    result.push_str("║              Execution Plan & Topology                      ║\n");
    result.push_str("╚════════════════════════════════════════════════════════════╝\n\n");

    // Statistics
    result.push_str(&format!("Total Tasks: {}\n", plan.total_tasks));
    result.push_str(&format!("Max Depth: {}\n", plan.max_depth));
    result.push_str(&format!("Critical Path Length: {}\n", plan.critical_path.len()));
    result.push_str(&format!("Parallelizable Tasks: {}\n\n", plan.parallelizable_tasks.len()));

    // Execution Levels
    if config.show_levels {
        result.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        result.push_str("Execution Levels (tasks in same level can run in parallel)\n");
        result.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

        for (level_index, level_tasks) in plan.execution_levels.iter().enumerate() {
            result.push_str(&format!("Level {} ({} tasks):\n", level_index, level_tasks.len()));

            for (task_index, task) in level_tasks.iter().enumerate() {
                let prefix = if config.compact { "  " } else { "    " };
                let connector = if task_index == level_tasks.len() - 1 {
                    "└── "
                } else {
                    "├── "
                };

                let status = if config.show_status {
                    format!(" [{}]", status_to_symbol(&task.status))
                } else {
                    String::new()
                };

                let critical = if config.highlight_critical && plan.critical_path.contains(&task.id) {
                    " ★ CRITICAL"
                } else {
                    ""
                };

                result.push_str(&format!("{}{}{}{}{}\n", prefix, connector, task.id, status, critical));
            }
            result.push('\n');
        }
    }

    // Critical Path
    if config.highlight_critical && !plan.critical_path.is_empty() {
        result.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        result.push_str("Critical Path (longest dependency chain)\n");
        result.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

        for (index, task_id) in plan.critical_path.iter().enumerate() {
            let arrow = if index < plan.critical_path.len() - 1 {
                " ↓ "
            } else {
                ""
            };
            result.push_str(&format!("{}{}\n", task_id, arrow));
        }
        result.push('\n');
    }

    // Flattened Execution Order
    result.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    result.push_str("Flattened Execution Order\n");
    result.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    for (index, task_id) in plan.execution_order.iter().enumerate() {
        let num = format!("{:3}.", index + 1);
        result.push_str(&format!("{} {}\n", num, task_id));
    }

    result
}

/// Visualizes dependencies for a single task and its descendants
///
/// # Arguments
///
/// * `task` - The task to visualize
/// * `task_map` - Map of all tasks for dependency lookup
/// * `config` - Optional visualization configuration
///
/// # Returns
///
/// A string containing the ASCII art visualization
#[must_use]
pub fn visualize_task_dependencies(task: &Task, tasks: &[Task], config: Option<TopologyConfig>) -> String {
    let config = config.unwrap_or_default();
    let mut result = String::new();

    // Build task map
    let task_map: HashMap<&str, &Task> = tasks
        .iter()
        .map(|t| (t.id.as_str(), t))
        .collect();

    result.push_str(&format!("╔════════════════════════════════════════════════════════════╗\n"));
    result.push_str(&format!("║  Dependencies for: {}                             ║\n", task.id));
    result.push_str("╚════════════════════════════════════════════════════════════╝\n\n");

    // Show the task itself
    let status = if config.show_status {
        format!(" [{}]", status_to_symbol(&task.status))
    } else {
        String::new()
    };
    result.push_str(&format!("{}{}: {}\n\n", task.id, status, task.title));

    // Show dependencies
    if !task.depends_on.is_empty() {
        result.push_str("Dependencies:\n");
        for dep_id in &task.depends_on {
            let dep_task = task_map.get(dep_id.as_str());
            let dep_status = if let Some(t) = dep_task {
                if config.show_status {
                    format!(" [{}]", status_to_symbol(&t.status))
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            result.push_str(&format!("  ↓ {}{}\n", dep_id, dep_status));
        }
        result.push('\n');
    } else {
        result.push_str("No dependencies (root task)\n\n");
    }

    // Show dependents (tasks that depend on this one)
    let dependents: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.depends_on.contains(&task.id))
        .collect();

    if !dependents.is_empty() {
        result.push_str("Dependents (tasks that depend on this one):\n");
        for dependent in dependents {
            let status = if config.show_status {
                format!(" [{}]", status_to_symbol(&dependent.status))
            } else {
                String::new()
            };
            result.push_str(&format!("  ↑ {}{}: {}\n", dependent.id, status, dependent.title));
        }
    } else {
        result.push_str("No dependents (leaf task)\n");
    }

    result
}

/// Creates a compact ASCII matrix visualization of task dependencies
///
/// # Arguments
///
/// * `tasks` - Slice of tasks to visualize
/// * `config` - Optional visualization configuration
///
/// # Returns
///
/// A string containing the matrix visualization
#[must_use]
pub fn visualize_dependency_matrix(tasks: &[Task], config: Option<TopologyConfig>) -> String {
    let config = config.unwrap_or_default();
    let mut result = String::new();

    if tasks.is_empty() {
        result.push_str("No tasks to visualize.\n");
        return result;
    }

    // Header
    result.push_str("╔════════════════════════════════════════════════════════════╗\n");
    result.push_str("║           Dependency Matrix (× = depends on)                ║\n");
    result.push_str("╚════════════════════════════════════════════════════════════╝\n\n");

    // Sort tasks by execution order (topological sort simulation)
    let mut tasks_vec = tasks.to_vec();
    let sorted_tasks = simple_topological_sort(&mut tasks_vec);

    // Calculate column widths
    let max_id_len = sorted_tasks.iter()
        .map(|t| t.id.len())
        .max()
        .unwrap_or(10);

    // Header row
    result.push_str(&format!("{:width$} |", "", width = max_id_len + 2));
    for task in &sorted_tasks {
        result.push_str(&format!(" {} ", &task.id[..task.id.len().min(3)]));
    }
    result.push_str("\n");

    // Separator
    result.push_str(&format!("{:->width$}-+", "", width = max_id_len + 2));
    for _ in &sorted_tasks {
        result.push_str("---");
    }
    result.push_str("\n");

    // Data rows
    for task in &sorted_tasks {
        result.push_str(&format!("{:width$} |", task.id, width = max_id_len + 2));

        for other in &sorted_tasks {
            if task.depends_on.contains(&other.id) {
                result.push_str(" × ");
            } else {
                result.push_str(" . ");
            }
        }
        result.push('\n');
    }

    // Legend
    result.push_str("\nLegend:\n");
    result.push_str("  ×  = depends on\n");
    result.push_str("  .  = no dependency\n");

    // Status legend if enabled
    if config.show_status {
        result.push_str("\nStatus Symbols:\n");
        result.push_str("  ○ = Pending\n");
        result.push_str("  ⚙ = In Progress\n");
        result.push_str("  ✓ = Completed\n");
        result.push_str("  ✗ = Failed\n");
        result.push_str("  ⚠ = Blocked\n");
    }

    result
}

/// Helper function to visualize a dependency tree recursively
fn visualize_dependency_tree(
    task: &Task,
    task_map: &HashMap<&str, &Task>,
    prefix: &str,
    _is_last: bool,
    config: &TopologyConfig,
    output: &mut String,
) {
    let status = if config.show_status {
        format!(" [{}]", status_to_symbol(&task.status))
    } else {
        String::new()
    };

    output.push_str(&format!("{}{}{}{}\n", prefix, task.id, status, format!(": {}", task.title)));

    // Find tasks that depend on this one (its children in the dependency tree)
    let dependents: Vec<&Task> = task_map
        .values()
        .filter(|t| t.depends_on.contains(&task.id))
        .copied()
        .collect();

    if !dependents.is_empty() {
        let child_count = dependents.len();

        for (index, dependent) in dependents.iter().enumerate() {
            let is_last_child = index == child_count - 1;
            let connector = if is_last_child { "└── " } else { "├── " };
            let child_prefix = format!("{}{}", prefix, connector);

            // Build continuation prefix for grandchildren
            // Pass the original prefix (without connector) to build_tree_continuation
            let continuation_str = if prefix.is_empty() {
                String::new()
            } else {
                build_tree_continuation(prefix, is_last_child)
            };

            format_task_with_dependencies(
                dependent,
                task_map,
                &child_prefix,
                &continuation_str,
                config,
                output,
            );
        }
    }
}

/// Helper to format task with its dependency connections
fn format_task_with_dependencies(
    task: &Task,
    task_map: &HashMap<&str, &Task>,
    prefix: &str,
    continuation: &String,
    config: &TopologyConfig,
    output: &mut String,
) {
    let status = if config.show_status {
        format!(" [{}]", status_to_symbol(&task.status))
    } else {
        String::new()
    };

    output.push_str(&format!("{}{}{}\n", prefix, task.id, status));

    // Recursively process tasks that depend on this one
    let dependents: Vec<&Task> = task_map
        .values()
        .filter(|t| t.depends_on.contains(&task.id))
        .copied()
        .collect();

    for (index, dependent) in dependents.iter().enumerate() {
        let is_last = index == dependents.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let new_prefix = format!("{}{}", continuation, connector);
        let new_continuation_str = if continuation.is_empty() {
            String::new()
        } else {
            build_tree_continuation(continuation, is_last)
        };

        format_task_with_dependencies(dependent, task_map, &new_prefix, &new_continuation_str, config, output);
    }
}

/// Builds continuation prefix for tree drawing
fn build_tree_continuation(parent_prefix: &str, parent_is_last: bool) -> String {
    if parent_prefix.is_empty() {
        if parent_is_last {
            "    ".to_string()
        } else {
            "│   ".to_string()
        }
    } else {
        let base = if parent_prefix.ends_with("├── ") {
            format!("{}│   ", &parent_prefix[..parent_prefix.len() - 4])
        } else if parent_prefix.ends_with("└── ") {
            format!("{}    ", &parent_prefix[..parent_prefix.len() - 4])
        } else {
            parent_prefix.to_string()
        };

        if parent_is_last {
            format!("{}    ", base)
        } else {
            format!("{}│   ", base)
        }
    }
}

/// Converts task status to a visual symbol
#[must_use]
fn status_to_symbol(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "○",
        TaskStatus::InProgress => "⚙",
        TaskStatus::Completed => "✓",
        TaskStatus::Failed => "✗",
        TaskStatus::Blocked => "⚠",
        TaskStatus::SkippedModeDisabled => "⏭",
    }
}

/// Simple topological sort for visualization purposes
fn simple_topological_sort(tasks: &mut Vec<Task>) -> Vec<Task> {
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    // Initialize in-degrees
    for task in &*tasks {
        in_degree.insert(task.id.clone(), 0);
    }

    // Calculate in-degrees based on dependencies
    for task in &*tasks {
        *in_degree.entry(task.id.clone()).or_insert(0) += task.depends_on.len();
    }

    // Sort using Kahn's algorithm approach (simplified for stability)
    let mut sorted = Vec::new();
    let mut remaining = tasks.clone();

    while !remaining.is_empty() {
        // Find task with minimum in-degree
        let min_pos = remaining
            .iter()
            .enumerate()
            .min_by_key(|(_, t)| *in_degree.get(&t.id).unwrap_or(&0))
            .map(|(i, _)| i)
            .unwrap_or(0);

        let task = remaining.remove(min_pos);
        let task_id = task.id.clone();
        sorted.push(task);

        // Decrease in-degree for dependents
        for other in &mut remaining {
            if other.depends_on.contains(&task_id) {
                if let Some(degree) = in_degree.get_mut(&other.id) {
                    *degree = degree.saturating_sub(1);
                }
            }
        }
    }

    sorted
}

/// Generates Mermaid flowchart syntax for task dependencies
///
/// # Arguments
///
/// * `tasks` - Slice of tasks to visualize
/// * `show_status` - Optional flag to include task status in the diagram
///
/// # Returns
///
/// A string containing Mermaid flowchart syntax
#[must_use]
pub fn generate_mermaid_flowchart(tasks: &[Task], show_status: Option<bool>) -> String {
    let include_status = show_status.unwrap_or(false);
    let mut result = String::new();

    // Mermaid diagram header
    result.push_str("```mermaid\n");
    result.push_str("graph TD\n");

    if tasks.is_empty() {
        result.push_str("    empty[\"No tasks\"]\n");
        result.push_str("```\n");
        return result;
    }

    // Build task map for quick lookup
    let task_map: HashMap<&str, &Task> = tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect();

    // Generate node definitions
    for task in tasks {
        let node_id = sanitize_mermaid_id(&task.id);

        let label = if include_status {
            format!("{}[\"{}\\n{}: {}\"]",
                node_id,
                task.id,
                status_to_mermaid_text(&task.status),
                task.title
            )
        } else {
            format!("{}[\"{}: {}\"]", node_id, task.id, task.title)
        };

        result.push_str("    ");
        result.push_str(&label);
        result.push('\n');
    }

    // Generate dependency edges
    for task in tasks {
        if !task.depends_on.is_empty() {
            let source = sanitize_mermaid_id(&task.id);
            for dep_id in &task.depends_on {
                let target = sanitize_mermaid_id(dep_id);
                result.push_str(&format!("    {} --> {}\n", target, source));
            }
        }
    }

    // Add styling based on status if requested
    if include_status {
        result.push_str("\n    %% Style nodes based on status\n");
        for task in tasks {
            let node_id = sanitize_mermaid_id(&task.id);
            let style = match task.status {
                TaskStatus::Pending => "    ".to_string(),
                TaskStatus::InProgress => format!("    classDef {}Style fill:#fff4e6,stroke:#ffa500,stroke-width:2px;\n", node_id),
                TaskStatus::Completed => format!("    classDef {}Style fill:#e6f7e6,stroke:#00cc00,stroke-width:2px;\n", node_id),
                TaskStatus::Failed => format!("    classDef {}Style fill:#ffe6e6,stroke:#cc0000,stroke-width:2px;\n", node_id),
                TaskStatus::Blocked => format!("    classDef {}Style fill:#f0f0f0,stroke:#666666,stroke-width:2px,stroke-dasharray: 5 5;\n", node_id),
                TaskStatus::SkippedModeDisabled => format!("    classDef {}Style fill:#f5f5f5,stroke:#999999,stroke-width:1px,stroke-dasharray: 3 3;\n", node_id),
            };
            result.push_str(&style);
        }
    }

    result.push_str("```\n");
    result
}

/// Generates Mermaid graph from execution plan
///
/// # Arguments
///
/// * `plan` - The execution plan to visualize
/// * `show_levels` - Optional flag to include execution levels
///
/// # Returns
///
/// A string containing Mermaid graph syntax
#[must_use]
pub fn generate_mermaid_graph(plan: &ExecutionPlan, show_levels: Option<bool>) -> String {
    let include_levels = show_levels.unwrap_or(false);
    let mut result = String::new();

    // Mermaid diagram header
    result.push_str("```mermaid\n");
    result.push_str("graph TD\n");

    // Title
    result.push_str("    title[\"Task Execution Plan\"]\n");

    if plan.execution_order.is_empty() {
        result.push_str("    empty[\"No tasks\"]\n");
        result.push_str("```\n");
        return result;
    }

    // Group by execution levels if requested
    if include_levels {
        result.push_str("\n    %% Execution Levels\n");
        for (level_index, level_tasks) in plan.execution_levels.iter().enumerate() {
            result.push_str(&format!("    subgraph Level{} [\"Level {}\"]\n", level_index, level_index));
            for task in level_tasks {
                let node_id = sanitize_mermaid_id(&task.id);
                result.push_str(&format!("        {}[\"{}\"]\n", node_id, task.id));
            }
            result.push_str("    end\n");
        }
    } else {
        // Simple flat list of tasks
        for task_id in &plan.execution_order {
            let node_id = sanitize_mermaid_id(task_id);
            result.push_str(&format!("    {}[\"{}\"]\n", node_id, task_id));
        }
    }

    // Add execution order edges
    result.push_str("\n    %% Execution Order\n");
    for (index, task_id) in plan.execution_order.iter().enumerate() {
        if index < plan.execution_order.len() - 1 {
            let current = sanitize_mermaid_id(task_id);
            let next = sanitize_mermaid_id(&plan.execution_order[index + 1]);
            result.push_str(&format!("    {} -->|next| {}\n", current, next));
        }
    }

    // Add statistics
    result.push_str("\n    %% Statistics\n");
    result.push_str(&format!("    stats[\"Total: {} | Max Depth: {} | Critical Path: {}\"]\n",
        plan.total_tasks,
        plan.max_depth,
        plan.critical_path.len()
    ));

    result.push_str("```\n");
    result
}

/// Exports Mermaid diagram to a file
///
/// # Arguments
///
/// * `tasks` - Slice of tasks to visualize
/// * `path` - Path to the output file
/// * `show_status` - Optional flag to include task status
///
/// # Returns
///
/// Result indicating success or failure
pub fn export_mermaid_to_file(
    tasks: &[Task],
    path: &Path,
    show_status: Option<bool>,
) -> std::io::Result<()> {
    let mermaid_content = generate_mermaid_flowchart(tasks, show_status);

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write to file
    let mut file = fs::File::create(path)?;
    file.write_all(mermaid_content.as_bytes())?;
    file.sync_all()?;

    Ok(())
}

/// Sanitizes task ID for use in Mermaid diagrams
///
/// Mermaid IDs must start with a letter and contain only alphanumeric characters,
/// underscores, and hyphens. This function replaces invalid characters.
fn sanitize_mermaid_id(id: &str) -> String {
    // If ID is already valid, return as-is
    if id.chars().next().map_or(false, |c| c.is_alphabetic()) {
        let sanitized: String = id
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect();
        return sanitized;
    }

    // If ID doesn't start with a letter, prefix it
    format!("id_{}", id)
}

/// Converts task status to Mermaid-friendly text representation
fn status_to_mermaid_text(status: &TaskStatus) -> &str {
    match status {
        TaskStatus::Pending => "○ Pending",
        TaskStatus::InProgress => "⚙ In Progress",
        TaskStatus::Completed => "✓ Completed",
        TaskStatus::Failed => "✗ Failed",
        TaskStatus::Blocked => "⚠ Blocked",
        TaskStatus::SkippedModeDisabled => "⏭ Skipped",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_visualize_dependency_graph_empty() {
        let tasks = vec![];
        let result = visualize_dependency_graph(&tasks, None);
        assert!(result.contains("No tasks to visualize"));
    }

    #[test]
    fn test_visualize_dependency_graph_single() {
        let task = Task::new("task-1", "Single Task", "No dependencies");
        let result = visualize_dependency_graph(&[task], None);
        assert!(result.contains("task-1"));
        assert!(result.contains("Single Task"));
    }

    #[test]
    fn test_visualize_dependency_graph_chain() {
        let task3 = Task::new("task-3", "Third", "Last task");
        let mut task2 = Task::new("task-2", "Second", "Middle task");
        task2.depends_on = vec!["task-3".to_string()];
        let mut task1 = Task::new("task-1", "First", "First task");
        task1.depends_on = vec!["task-2".to_string()];

        let tasks = vec![task3, task2, task1];
        let result = visualize_dependency_graph(&tasks, None);

        assert!(result.contains("task-1"));
        assert!(result.contains("task-2"));
        assert!(result.contains("task-3"));
    }

    #[test]
    fn test_visualize_execution_plan_empty() {
        let plan = ExecutionPlan {
            execution_levels: vec![],
            execution_order: vec![],
            critical_path: vec![],
            parallelizable_tasks: HashSet::new(),
            max_depth: 0,
            total_tasks: 0,
        };

        let result = visualize_execution_plan(&plan, None);
        assert!(result.contains("Total Tasks: 0"));
    }

    #[test]
    fn test_topology_config_default() {
        let config = TopologyConfig::default();
        assert!(config.show_status);
        assert!(config.highlight_critical);
        assert!(config.show_levels);
        assert!(!config.compact);
    }

    #[test]
    fn test_visualize_task_dependencies() {
        let mut task2 = Task::new("task-2", "Dependent Task", "Depends on task-1");
        task2.depends_on = vec!["task-1".to_string()];

        let task1 = Task::new("task-1", "Independent Task", "No dependencies");
        let tasks = vec![task1.clone(), task2];

        let result = visualize_task_dependencies(&task1, &tasks, None);
        assert!(result.contains("task-1"));
        assert!(result.contains("No dependencies"));
        assert!(result.contains("Dependents"));
    }

    #[test]
    fn test_visualize_dependency_matrix() {
        let mut task2 = Task::new("task-2", "Task 2", "Second");
        task2.depends_on = vec!["task-1".to_string()];

        let task1 = Task::new("task-1", "Task 1", "First");
        let tasks = vec![task1, task2];

        let result = visualize_dependency_matrix(&tasks, None);
        assert!(result.contains("Dependency Matrix"));
        assert!(result.contains("task-1"));
        assert!(result.contains("task-2"));
        assert!(result.contains("Legend"));
    }

    #[test]
    fn test_status_to_symbol() {
        assert_eq!(status_to_symbol(&TaskStatus::Pending), "○");
        assert_eq!(status_to_symbol(&TaskStatus::InProgress), "⚙");
        assert_eq!(status_to_symbol(&TaskStatus::Completed), "✓");
        assert_eq!(status_to_symbol(&TaskStatus::Failed), "✗");
        assert_eq!(status_to_symbol(&TaskStatus::Blocked), "⚠");
    }

    #[test]
    fn test_simple_topological_sort() {
        let task3 = Task::new("task-3", "Third", "C");
        let mut task2 = Task::new("task-2", "Second", "B");
        task2.depends_on = vec!["task-3".to_string()];
        let mut task1 = Task::new("task-1", "First", "A");
        task1.depends_on = vec!["task-2".to_string()];

        let mut tasks = vec![task1, task2, task3];
        let sorted = simple_topological_sort(&mut tasks);

        assert_eq!(sorted[0].id, "task-3"); // No dependencies
        assert_eq!(sorted[1].id, "task-2"); // Depends on task-3
        assert_eq!(sorted[2].id, "task-1"); // Depends on task-2
    }
}
