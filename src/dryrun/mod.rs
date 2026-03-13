// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Dry-run mode for ltmatrix
//!
//! This module implements the --dry-run flag functionality that:
//! - Executes Generate and Assess stages only
//! - Displays task plan with dependencies, complexity estimates, and execution order
//! - Shows summary without making any changes or running agents
//! - Exits cleanly after plan display

use anyhow::{Context, Result};
use tracing::info;

use crate::pipeline::assess::{assess_tasks, AssessConfig};
use crate::tasks::scheduler::{schedule_tasks, ExecutionPlan};
use ltmatrix_core::{ExecutionMode, Task, TaskComplexity};

/// Configuration for dry-run mode
#[derive(Debug, Clone)]
pub struct DryRunConfig {
    /// Execution mode to use for assessment
    pub execution_mode: ExecutionMode,

    /// Assessment configuration
    pub assess_config: AssessConfig,

    /// Whether to output in JSON format
    pub json_output: bool,
}

impl Default for DryRunConfig {
    fn default() -> Self {
        DryRunConfig {
            execution_mode: ExecutionMode::Standard,
            assess_config: AssessConfig::default(),
            json_output: false,
        }
    }
}

/// Result of a dry-run execution
#[derive(Debug, Clone)]
pub struct DryRunResult {
    /// The generated and assessed tasks
    pub tasks: Vec<Task>,

    /// The execution plan showing order and dependencies
    pub execution_plan: ExecutionPlan,

    /// Statistics about the task graph
    pub statistics: DryRunStatistics,
}

/// Statistics about the dry-run execution
#[derive(Debug, Clone)]
pub struct DryRunStatistics {
    /// Total number of tasks
    pub total_tasks: usize,

    /// Number of tasks by complexity
    pub simple_tasks: usize,
    pub moderate_tasks: usize,
    pub complex_tasks: usize,

    /// Number of tasks with subtasks
    pub tasks_with_subtasks: usize,

    /// Total number of subtasks
    pub total_subtasks: usize,

    /// Execution depth (number of levels)
    pub execution_depth: usize,

    /// Tasks on critical path
    pub critical_path_length: usize,

    /// Parallelizable tasks
    pub parallelizable_count: usize,
}

/// Main entry point for dry-run mode
///
/// This function:
/// 1. Simulates task generation (placeholder until generate.rs is implemented)
/// 2. Assesses task complexity
/// 3. Creates execution plan with dependency resolution
/// 4. Displays comprehensive plan without executing anything
pub async fn run_dry_run(goal: &str, config: &DryRunConfig) -> Result<DryRunResult> {
    info!("Starting dry-run mode for goal: {}", goal);

    // Step 1: Generate tasks (placeholder - will be replaced by generate stage)
    let generated_tasks = generate_tasks_for_goal(goal)?;
    info!("Generated {} tasks", generated_tasks.len());

    // Step 2: Assess task complexity
    let assessed_tasks = assess_tasks(generated_tasks, &config.assess_config)
        .await
        .context("Failed to assess tasks")?;
    info!("Assessed {} tasks", assessed_tasks.len());

    // Step 3: Create execution plan
    let execution_plan =
        schedule_tasks(assessed_tasks.clone()).context("Failed to create execution plan")?;
    info!(
        "Created execution plan with {} levels",
        execution_plan.max_depth
    );

    // Step 4: Calculate statistics
    let statistics = calculate_dry_run_statistics(&assessed_tasks, &execution_plan);

    // Step 5: Display results
    if config.json_output {
        display_json_result(&assessed_tasks, &execution_plan, &statistics)?;
    } else {
        display_text_result(goal, &assessed_tasks, &execution_plan, &statistics)?;
    }

    info!("Dry-run completed successfully");

    Ok(DryRunResult {
        tasks: assessed_tasks,
        execution_plan,
        statistics,
    })
}

/// Placeholder function for task generation
///
/// TODO: Replace this with the actual generate stage implementation
/// This function simulates what the generate stage will do by creating
/// example tasks based on the goal
fn generate_tasks_for_goal(goal: &str) -> Result<Vec<Task>> {
    info!("Generating tasks for goal: {}", goal);

    // This is a placeholder that simulates task generation
    // In the full implementation, this will call the generate stage
    // which uses Claude to break down the goal into tasks

    let tasks = vec![
        Task::new(
            "task-1",
            "Analyze requirements",
            &format!("Understand and clarify requirements for: {}", goal),
        ),
        Task::new(
            "task-2",
            "Design solution",
            "Create a technical design for the implementation",
        ),
        Task::new(
            "task-3",
            "Implement core functionality",
            "Implement the main features and logic",
        ),
        Task::new(
            "task-4",
            "Write tests",
            "Create comprehensive unit and integration tests",
        ),
        Task::new(
            "task-5",
            "Documentation",
            "Write documentation and usage examples",
        ),
    ];

    // Add some dependencies to demonstrate the scheduling
    let mut tasks_with_deps = tasks;
    tasks_with_deps[1].depends_on = vec!["task-1".to_string()];
    tasks_with_deps[2].depends_on = vec!["task-2".to_string()];
    tasks_with_deps[3].depends_on = vec!["task-2".to_string()];
    tasks_with_deps[4].depends_on = vec!["task-3".to_string(), "task-3".to_string()];

    Ok(tasks_with_deps)
}

/// Calculates statistics for the dry-run result
fn calculate_dry_run_statistics(tasks: &[Task], plan: &ExecutionPlan) -> DryRunStatistics {
    let mut simple = 0;
    let mut moderate = 0;
    let mut complex = 0;
    let mut with_subtasks = 0;
    let mut total_subtasks = 0;

    for task in tasks {
        match task.complexity {
            TaskComplexity::Simple => simple += 1,
            TaskComplexity::Moderate => moderate += 1,
            TaskComplexity::Complex => complex += 1,
        }

        if !task.subtasks.is_empty() {
            with_subtasks += 1;
            total_subtasks += task.subtasks.len();
        }
    }

    DryRunStatistics {
        total_tasks: tasks.len(),
        simple_tasks: simple,
        moderate_tasks: moderate,
        complex_tasks: complex,
        tasks_with_subtasks: with_subtasks,
        total_subtasks,
        execution_depth: plan.max_depth,
        critical_path_length: plan.critical_path.len(),
        parallelizable_count: plan.parallelizable_tasks.len(),
    }
}

/// Displays results in text format
fn display_text_result(
    goal: &str,
    tasks: &[Task],
    plan: &ExecutionPlan,
    stats: &DryRunStatistics,
) -> Result<()> {
    use console::style;

    println!();
    println!(
        "{}",
        style("╔═══════════════════════════════════════════════════════════════╗").bold()
    );
    println!(
        "{}",
        style("║           LTMATRIX - DRY RUN MODE                            ║").bold()
    );
    println!(
        "{}",
        style("╚═══════════════════════════════════════════════════════════════╝").bold()
    );
    println!();

    // Display goal
    println!("{}", style("Goal:").bold().cyan());
    println!("  {}", goal);
    println!();

    // Display summary statistics
    println!("{}", style("Summary:").bold().cyan());
    println!("  Total Tasks: {}", stats.total_tasks);
    println!("  Execution Depth: {} levels", stats.execution_depth);
    println!(
        "  Critical Path Length: {} tasks",
        stats.critical_path_length
    );
    println!("  Parallelizable Tasks: {}", stats.parallelizable_count);
    println!();

    // Display complexity breakdown
    println!("{}", style("Complexity Breakdown:").bold().cyan());
    println!(
        "  Simple: {} {}",
        stats.simple_tasks,
        style("(fast model)").dim()
    );
    println!(
        "  Moderate: {} {}",
        stats.moderate_tasks,
        style("(standard model)").dim()
    );
    println!(
        "  Complex: {} {}",
        stats.complex_tasks,
        style("(smart model)").dim()
    );
    println!();

    // Display subtask information
    if stats.tasks_with_subtasks > 0 {
        println!("{}", style("Subtasks:").bold().cyan());
        println!("  Tasks with subtasks: {}", stats.tasks_with_subtasks);
        println!("  Total subtasks created: {}", stats.total_subtasks);
        println!();
    }

    // Display execution plan
    println!("{}", style("Execution Plan:").bold().cyan());
    for (level, tasks) in plan.execution_levels.iter().enumerate() {
        println!("  Level {} ({} tasks):", level + 1, tasks.len());
        for task in tasks {
            let complexity_label = match task.complexity {
                TaskComplexity::Simple => "⚡",
                TaskComplexity::Moderate => "⚙️",
                TaskComplexity::Complex => "🔧",
            };

            let deps = if task.depends_on.is_empty() {
                String::new()
            } else {
                format!(" [depends on: {}]", task.depends_on.join(", "))
            };

            println!(
                "    {} {}{} {}",
                complexity_label,
                task.id,
                deps,
                style(format!("- {}", task.title)).dim()
            );
        }
        println!();
    }

    // Display critical path
    println!("{}", style("Critical Path:").bold().cyan());
    for (i, task_id) in plan.critical_path.iter().enumerate() {
        if let Some(task) = tasks.iter().find(|t| &t.id == task_id) {
            println!("  {}. {} ({})", i + 1, task_id, task.title);
        }
    }
    println!();

    // Display notice
    println!("{}", style("Notice:").bold().yellow());
    println!("  This is a DRY RUN - no changes will be made");
    println!("  Remove --dry-run flag to execute the plan");
    println!();

    Ok(())
}

/// Displays results in JSON format
fn display_json_result(
    tasks: &[Task],
    plan: &ExecutionPlan,
    stats: &DryRunStatistics,
) -> Result<()> {
    use serde_json::json;

    let output = json!({
        "goal": tasks.get(0).map(|t| t.title.as_str()).unwrap_or("unknown"),
        "summary": {
            "total_tasks": stats.total_tasks,
            "execution_depth": stats.execution_depth,
            "critical_path_length": stats.critical_path_length,
            "parallelizable_tasks": stats.parallelizable_count,
        },
        "complexity_breakdown": {
            "simple": stats.simple_tasks,
            "moderate": stats.moderate_tasks,
            "complex": stats.complex_tasks,
        },
        "subtasks": {
            "tasks_with_subtasks": stats.tasks_with_subtasks,
            "total_subtasks": stats.total_subtasks,
        },
        "execution_plan": {
            "max_depth": plan.max_depth,
            "total_tasks": plan.total_tasks,
            "execution_order": plan.execution_order,
            "critical_path": plan.critical_path,
            "parallelizable_tasks": plan.parallelizable_tasks,
            "execution_levels": plan.execution_levels.iter().map(|level| {
                level.iter().map(|task| {
                    serde_json::json!({
                        "id": task.id,
                        "title": task.title,
                        "description": task.description,
                        "complexity": format!("{:?}", task.complexity),
                        "depends_on": task.depends_on,
                        "subtasks_count": task.subtasks.len(),
                    })
                }).collect::<Vec<_>>()
            }).collect::<Vec<_>>()
        },
        "tasks": tasks.iter().map(|task| {
            serde_json::json!({
                "id": task.id,
                "title": task.title,
                "description": task.description,
                "status": format!("{:?}", task.status),
                "complexity": format!("{:?}", task.complexity),
                "depends_on": task.depends_on,
                "subtasks": task.subtasks.iter().map(|st| {
                    serde_json::json!({
                        "id": st.id,
                        "title": st.title,
                        "description": st.description,
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>()
    });

    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_tasks_for_goal() {
        let tasks = generate_tasks_for_goal("build a REST API").unwrap();
        assert!(!tasks.is_empty());
        assert_eq!(tasks[0].id, "task-1");
        assert_eq!(tasks[1].depends_on, vec!["task-1".to_string()]);
    }

    #[test]
    fn test_calculate_dry_run_statistics() {
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "Simple", "Simple task");
                t.complexity = TaskComplexity::Simple;
                t
            },
            {
                let mut t = Task::new("task-2", "Complex", "Complex task");
                t.complexity = TaskComplexity::Complex;
                t.subtasks = vec![Task::new("sub-1", "Sub", "Subtask")];
                t
            },
        ];

        let plan = ExecutionPlan {
            execution_levels: vec![vec![tasks[0].clone()], vec![tasks[1].clone()]],
            execution_order: vec!["task-1".to_string(), "task-2".to_string()],
            critical_path: vec!["task-1".to_string(), "task-2".to_string()],
            parallelizable_tasks: std::collections::HashSet::new(),
            max_depth: 2,
            total_tasks: 2,
        };

        let stats = calculate_dry_run_statistics(&tasks, &plan);

        assert_eq!(stats.total_tasks, 2);
        assert_eq!(stats.simple_tasks, 1);
        assert_eq!(stats.complex_tasks, 1);
        assert_eq!(stats.tasks_with_subtasks, 1);
        assert_eq!(stats.total_subtasks, 1);
        assert_eq!(stats.execution_depth, 2);
    }

    #[test]
    fn test_dry_run_config_default() {
        let config = DryRunConfig::default();
        assert_eq!(config.execution_mode, ExecutionMode::Standard);
        assert!(!config.json_output);
    }
}
