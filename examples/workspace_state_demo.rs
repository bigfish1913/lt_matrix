//! Workspace state persistence demo
//!
//! This example demonstrates how to use the WorkspaceState API
//! to save and load task manifests.

use ltmatrix::models::{Task, TaskStatus, TaskComplexity};
use ltmatrix::workspace::WorkspaceState;
use std::path::PathBuf;

fn main() -> Result<(), anyhow::Error> {
    println!("=== Workspace State Persistence Demo ===\n");

    // Get current directory as project root
    let project_root = PathBuf::from(".");
    println!("Project root: {:?}\n", project_root);

    // Create some example tasks
    let mut task1 = Task::new("task-1", "Set up project", "Initialize project structure");
    task1.status = TaskStatus::Completed;
    task1.complexity = TaskComplexity::Simple;

    let mut task2 = Task::new("task-2", "Implement feature", "Core feature implementation");
    task2.status = TaskStatus::InProgress;
    task2.complexity = TaskComplexity::Moderate;
    task2.depends_on = vec!["task-1".to_string()];

    let mut task3 = Task::new("task-3", "Write tests", "Unit tests for feature");
    task3.status = TaskStatus::Pending;
    task3.complexity = TaskComplexity::Simple;
    task3.depends_on = vec!["task-2".to_string()];

    println!("Created tasks:");
    println!("  - {}: {} ({:?})", task1.id, task1.title, task1.status);
    println!("  - {}: {} ({:?})", task2.id, task2.title, task2.status);
    println!("  - {}: {} ({:?})", task3.id, task3.title, task3.status);
    println!();

    // Create workspace state
    let state = WorkspaceState::new(project_root.clone(), vec![task1, task2, task3]);
    println!("Created WorkspaceState with {} tasks", state.tasks.len());
    println!("Manifest path: {:?}\n", state.manifest_path());

    // Save the state
    println!("Saving workspace state...");
    let saved_state = state.save()?;
    println!("✓ Workspace state saved successfully!");
    println!("  Version: {}", saved_state.metadata.version);
    println!("  Created: {}", saved_state.metadata.created_at);
    println!("  Modified: {}\n", saved_state.metadata.modified_at);

    // Load the state
    println!("Loading workspace state...");
    let loaded_state = WorkspaceState::load(project_root)?;
    println!("✓ Workspace state loaded successfully!");
    println!("  Tasks: {}", loaded_state.tasks.len());

    println!("\nTask summary:");
    for task in &loaded_state.tasks {
        println!("  - [{:?}] {} - {}", task.status, task.id, task.title);
    }

    println!("\n=== Demo Complete ===");
    println!("Manifest file saved at: {:?}", loaded_state.manifest_path());

    Ok(())
}
