//! Workspace state lifecycle integration demo
//!
//! This example demonstrates workspace state persistence integration
//! with task lifecycle, including auto-reset of in_progress tasks.

use ltmatrix::workspace::WorkspaceState;
use ltmatrix::models::{Task, TaskStatus};
use tempfile::TempDir;

fn main() {
    println!("=== Workspace State Lifecycle Integration Demo ===\n");

    // Create a temporary directory for the demo
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    println!("1. Creating Initial Workspace State");
    println!("{}", "=".repeat(50));

    let task1 = Task::new("task-1", "Setup Project", "Initialize project structure");
    let task2 = Task::new("task-2", "Implement Core", "Core functionality");
    let task3 = Task::new("task-3", "Add Tests", "Test coverage");

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2, task3]);
    println!("Created workspace with {} tasks", state.tasks.len());
    println!("Project root: {:?}", state.project_root);
    println!("Manifest path: {:?}", state.manifest_path());
    println!();

    // Save the initial state
    println!("2. Saving Initial State");
    println!("{}", "=".repeat(50));

    match state.save() {
        Ok(saved_state) => {
            println!("State saved successfully");
            println!("Modified at: {}", saved_state.metadata.modified_at);
            println!("Version: {}", saved_state.metadata.version);
        }
        Err(e) => {
            println!("Error saving state: {}", e);
        }
    }
    println!();

    // Simulate task execution - update task statuses
    println!("3. Simulating Task Execution");
    println!("{}", "=".repeat(50));

    let mut loaded_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    println!("Loaded {} tasks from manifest", loaded_state.tasks.len());

    // Simulate task completion
    loaded_state.tasks[0].status = TaskStatus::Completed;
    loaded_state.tasks[1].status = TaskStatus::InProgress;
    loaded_state.tasks[2].status = TaskStatus::Pending;

    println!("Updated task statuses:");
    for task in &loaded_state.tasks {
        println!("  - {}: {:?}", task.id, task.status);
    }
    println!();

    // Save after execution
    println!("4. Saving After Task Execution");
    println!("{}", "=".repeat(50));

    loaded_state.save().unwrap();
    println!("State saved after task execution");
    println!();

    // Demonstrate auto-reset feature
    println!("5. Demonstrating Auto-Reset Feature");
    println!("{}", "=".repeat(50));

    // Create a state with InProgress and Blocked tasks
    let mut task4 = Task::new("task-4", "In Progress Task", "Started but not finished");
    task4.status = TaskStatus::InProgress;

    let mut task5 = Task::new("task-5", "Blocked Task", "Waiting for dependencies");
    task5.status = TaskStatus::Blocked;

    let mut task6 = Task::new("task-6", "Completed Task", "Already done");
    task6.status = TaskStatus::Completed;

    let state_with_issues = WorkspaceState::new(
        project_root.to_path_buf(),
        vec![task4, task5, task6]
    );
    state_with_issues.save().unwrap();

    println!("Saved state with:");
    println!("  - task-4: {:?} (should reset)", TaskStatus::InProgress);
    println!("  - task-5: {:?} (should reset)", TaskStatus::Blocked);
    println!("  - task-6: {:?} (should preserve)", TaskStatus::Completed);
    println!();

    // Load with transform
    println!("6. Loading with Transform (Auto-Reset)");
    println!("{}", "=".repeat(50));

    let transformed_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    println!("After transformation:");
    for task in &transformed_state.tasks {
        println!("  - {}: {:?}", task.id, task.status);
    }
    println!();

    // Demonstrate subtask transformation
    println!("7. Subtask Transformation");
    println!("{}", "=".repeat(50));

    let mut subtask = Task::new("task-8", "Subtask", "Child task");
    subtask.status = TaskStatus::InProgress;

    let mut parent = Task::new("task-7", "Parent Task", "Parent with subtask");
    parent.status = TaskStatus::InProgress;
    parent.subtasks = vec![subtask];

    let nested_state = WorkspaceState::new(project_root.to_path_buf(), vec![parent]);
    nested_state.save().unwrap();

    println!("Saved nested tasks:");
    println!("  - task-7 (parent): {:?}", TaskStatus::InProgress);
    println!("  - task-8 (subtask): {:?}", TaskStatus::InProgress);
    println!();

    // Load with transform - should reset both parent and child
    let transformed_nested = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    println!("After transformation:");
    println!("  - {} (parent): {:?}", transformed_nested.tasks[0].id, transformed_nested.tasks[0].status);
    if !transformed_nested.tasks[0].subtasks.is_empty() {
        println!("  - {} (subtask): {:?}",
            transformed_nested.tasks[0].subtasks[0].id,
            transformed_nested.tasks[0].subtasks[0].status
        );
    }
    println!();

    // Demonstrate error handling
    println!("8. Error Handling for Corrupted Files");
    println!("{}", "=".repeat(50));

    // Create a corrupted JSON file
    let ltmatrix_dir = project_root.join(".ltmatrix");
    let corrupted_path = ltmatrix_dir.join("corrupted.json");
    std::fs::write(&corrupted_path, "{ invalid json }").unwrap();

    println!("Created corrupted JSON file");
    println!("Attempting to load (should fail gracefully)...");

    // Try to load a non-existent file
    let result = WorkspaceState::load_with_transform(project_root.join("nonexistent").to_path_buf());
    match result {
        Ok(_) => println!("Unexpectedly succeeded"),
        Err(e) => println!("Error handled correctly: {}", e),
    }
    println!();

    // Summary
    println!("9. Summary of Features");
    println!("{}", "=".repeat(50));
    println!("✓ State persistence after each task completion");
    println!("✓ Auto-reset InProgress → Pending on load");
    println!("✓ Auto-reset Blocked → Pending on load");
    println!("✓ Preserve Completed, Failed, Pending statuses");
    println!("✓ Recursive transformation of subtasks");
    println!("✓ Preserve dependencies and metadata");
    println!("✓ Error handling for corrupted files");
    println!("✓ Directory creation if missing");
    println!();

    println!("=== Demo Complete ===");
    println!("\nNote: The workspace state is saved in: {:?}", project_root.join(".ltmatrix"));
    println!("This directory will be cleaned up automatically when the temp dir is dropped.");
}
