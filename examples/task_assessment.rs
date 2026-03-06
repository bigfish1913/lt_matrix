//! Example usage of the task assessment stage
//!
//! This example demonstrates how to use the Assess stage to evaluate
//! task complexity and split complex tasks into subtasks.

use ltmatrix::models::{Task, TaskComplexity};
use ltmatrix::pipeline::assess::{assess_tasks, AssessConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create sample tasks
    let tasks = vec![
        Task::new(
            "task-1",
            "Add login form",
            "Create a simple login form with username and password fields",
        ),
        Task::new(
            "task-2",
            "Implement user authentication",
            "Build a complete authentication system with OAuth, JWT tokens, \
             session management, and multi-factor authentication support",
        ),
        Task::new(
            "task-3",
            "Write unit tests",
            "Add unit tests for the user registration module",
        ),
    ];

    println!("Assessing {} tasks...\n", tasks.len());

    // Create assessment config (standard mode)
    let config = AssessConfig::default();

    // Assess tasks (this would normally call Claude)
    // For demonstration, we'll simulate the assessment
    let mut assessed_tasks = tasks;

    // Manually set complexity for demonstration
    assessed_tasks[0].complexity = TaskComplexity::Simple;
    assessed_tasks[1].complexity = TaskComplexity::Complex;
    assessed_tasks[2].complexity = TaskComplexity::Moderate;

    // Add subtasks for the complex task
    assessed_tasks[1].subtasks = vec![
        Task::new(
            "task-2-1",
            "Design authentication schema",
            "Create database schema for users and sessions",
        ),
        Task::new(
            "task-2-2",
            "Implement OAuth providers",
            "Add support for Google, GitHub, and Twitter OAuth",
        ),
        Task::new(
            "task-2-3",
            "Create JWT token system",
            "Implement JWT generation, validation, and refresh logic",
        ),
    ];

    // Display results
    println!("Assessment Results:");
    println!("==================\n");

    for (i, task) in assessed_tasks.iter().enumerate() {
        println!("{}. {} ({:?})", i + 1, task.title, task.complexity);
        if !task.subtasks.is_empty() {
            println!("   Split into {} subtasks:", task.subtasks.len());
            for (j, subtask) in task.subtasks.iter().enumerate() {
                println!("     {}. {}", j + 1, subtask.title);
            }
        }
        println!();
    }

    // Calculate and display statistics
    let stats = ltmatrix::pipeline::assess::calculate_assessment_stats(&assessed_tasks);
    println!("{}", stats);

    Ok(())
}
