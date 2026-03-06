//! Mermaid diagram generation demo
//!
//! This example demonstrates Mermaid diagram generation for task dependencies
//! and execution flow visualization.

use ltmatrix::models::{Task, TaskStatus};
use ltmatrix::tasks::scheduler::schedule_tasks;
use ltmatrix::tasks::topology::{
    generate_mermaid_flowchart, generate_mermaid_graph, export_mermaid_to_file,
};

fn main() {
    println!("=== Mermaid Diagram Generation Demo ===\n");

    // Example 1: Simple dependency chain
    println!("1. Simple Dependency Chain:");
    println!("{}", "=".repeat(50));

    let mut task3 = Task::new("task-3", "Database Setup", "Setup database");
    let mut task2 = Task::new("task-2", "API Development", "Build API");
    task2.depends_on = vec!["task-3".to_string()];
    let mut task1 = Task::new("task-1", "Frontend", "Build frontend");
    task1.depends_on = vec!["task-2".to_string()];

    let tasks = vec![task3, task2, task1];

    let mermaid = generate_mermaid_flowchart(&tasks, Some(false));
    println!("{}", mermaid);

    // Example 2: Diamond dependency pattern
    println!("2. Diamond Dependency Pattern:");
    println!("{}", "=".repeat(50));

    let mut task4 = Task::new("task-4", "Integration Tests", "Run integration tests");
    task4.depends_on = vec!["task-2".to_string(), "task-3".to_string()];

    let mut task2 = Task::new("task-2", "Frontend", "Frontend development");
    task2.depends_on = vec!["task-1".to_string()];

    let mut task3 = Task::new("task-3", "Backend", "Backend development");
    task3.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Setup", "Project setup");

    let tasks2 = vec![task1, task2, task3, task4];

    let mermaid2 = generate_mermaid_flowchart(&tasks2, Some(false));
    println!("{}", mermaid2);

    // Example 3: With status indicators
    println!("3. With Status Indicators:");
    println!("{}", "=".repeat(50));

    let mut task3 = Task::new("task-3", "Completed Task", "Done");
    task3.status = TaskStatus::Completed;

    let mut task2 = Task::new("task-2", "In Progress Task", "Working");
    task2.status = TaskStatus::InProgress;
    task2.depends_on = vec!["task-3".to_string()];

    let mut task1 = Task::new("task-1", "Pending Task", "Not started");
    task1.status = TaskStatus::Pending;
    task1.depends_on = vec!["task-2".to_string()];

    let tasks3 = vec![task3, task2, task1];

    let mermaid3 = generate_mermaid_flowchart(&tasks3, Some(true));
    println!("{}", mermaid3);

    // Example 4: Execution plan visualization
    println!("4. Execution Plan Visualization:");
    println!("{}", "=".repeat(50));

    let plan = schedule_tasks(tasks2).unwrap();
    let mermaid4 = generate_mermaid_graph(&plan, Some(true));
    println!("{}", mermaid4);

    // Example 5: Export to file
    println!("5. Export to File:");
    println!("{}", "=".repeat(50));

    let output_path = std::path::Path::new("demo_mermaid_diagram.mmd");
    match export_mermaid_to_file(&tasks3, output_path, Some(true)) {
        Ok(_) => println!("Mermaid diagram exported to: {:?}", output_path),
        Err(e) => println!("Error exporting: {}", e),
    }

    println!("\n=== Usage ===");
    println!("You can paste the Mermaid code into:");
    println!("  - https://mermaid.live (online editor)");
    println!("  - Markdown files with Mermaid support");
    println!("  - Documentation systems that support Mermaid");
    println!("\nThe .mmd file can be opened in any Mermaid-compatible viewer.");
}
