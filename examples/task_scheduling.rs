//! Example usage of the task dependency scheduler
//!
//! This example demonstrates how to use the scheduler to:
//! - Build dependency graphs from tasks
//! - Perform topological sorting
//! - Detect circular dependencies
//! - Calculate critical paths
//! - Identify parallelizable tasks
//! - Generate execution plans

use ltmatrix::models::Task;
use ltmatrix::tasks::scheduler::{
    calculate_graph_statistics, generate_mermaid_diagram, schedule_tasks, visualize_execution_plan,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create a set of tasks with dependencies
    let tasks = vec![
        {
            let mut task = Task::new("task-1", "Setup Project", "Initialize project structure");
            task.depends_on = vec![];
            task
        },
        {
            let mut task = Task::new("task-2", "Design Database", "Design database schema");
            task.depends_on = vec!["task-1".to_string()];
            task
        },
        {
            let mut task = Task::new("task-3", "Create API", "Implement REST API endpoints");
            task.depends_on = vec!["task-1".to_string()];
            task
        },
        {
            let mut task = Task::new("task-4", "Write Tests", "Write unit tests for API");
            task.depends_on = vec!["task-3".to_string()];
            task
        },
        {
            let mut task = Task::new("task-5", "Create Frontend", "Build frontend interface");
            task.depends_on = vec!["task-1".to_string()];
            task
        },
        {
            let mut task = Task::new("task-6", "Integration Tests", "Write integration tests");
            task.depends_on = vec!["task-3".to_string(), "task-5".to_string()];
            task
        },
        {
            let mut task = Task::new("task-7", "Deploy", "Deploy to production");
            task.depends_on = vec!["task-4".to_string(), "task-6".to_string()];
            task
        },
    ];

    println!("Task Scheduling Example");
    println!("======================\n");
    println!("Tasks with dependencies:");
    for task in &tasks {
        if task.depends_on.is_empty() {
            println!("  - {} (no dependencies)", task.id);
        } else {
            println!("  - {} depends on: {}", task.id, task.depends_on.join(", "));
        }
    }
    println!();

    // Build execution plan
    match schedule_tasks(tasks) {
        Ok(plan) => {
            println!("{}", visualize_execution_plan(&plan));
            println!();

            // Show execution levels
            println!("Detailed Execution Levels:");
            for (level, tasks) in plan.execution_levels.iter().enumerate() {
                println!("  Level {}: {} tasks", level + 1, tasks.len());
                for task in tasks {
                    let parallel = if plan.parallelizable_tasks.contains(&task.id) {
                        " (parallelizable)"
                    } else {
                        ""
                    };
                    println!("    - {}{}: {}", task.id, parallel, task.title);
                }
            }
            println!();

            // Calculate and display statistics
            let task_map: std::collections::HashMap<String, Task> = plan
                .execution_levels
                .iter()
                .flat_map(|level| level.iter().cloned())
                .map(|task| (task.id.clone(), task))
                .collect();

            let stats = calculate_graph_statistics(&task_map)?;
            println!("Graph Statistics:");
            println!("  - Total tasks: {}", stats.total_tasks);
            println!("  - Dependency edges: {}", stats.total_edges);
            println!("  - Root tasks: {}", stats.root_tasks);
            println!("  - Leaf tasks: {}", stats.leaf_tasks);
            println!("  - Max depth: {}", stats.max_depth);
            println!("  - Critical path length: {}", stats.critical_path_length);
            println!("  - Parallelism factor: {:.2}", stats.parallelism_factor);
            println!();

            // Generate Mermaid diagram
            println!("Mermaid Diagram:");
            println!("{}", generate_mermaid_diagram(&task_map));
        }
        Err(e) => {
            eprintln!("Error scheduling tasks: {}", e);
        }
    }

    Ok(())
}
