//! Mermaid diagram generation tests
//!
//! This test suite validates Mermaid diagram syntax generation for task dependencies
//! and execution flow visualization.

use ltmatrix::models::{Task, TaskStatus};
use ltmatrix::tasks::scheduler::schedule_tasks;
use ltmatrix::tasks::topology::{
    export_mermaid_to_file, generate_mermaid_flowchart, generate_mermaid_graph,
};

#[test]
fn test_generate_mermaid_flowchart_empty_tasks() {
    let tasks = vec![];
    let mermaid = generate_mermaid_flowchart(&tasks, None);

    // Should produce valid Mermaid syntax
    assert!(mermaid.contains("graph TD"));
    assert!(mermaid.contains("```mermaid") || mermaid.contains("```"));
}

#[test]
fn test_generate_mermaid_flowchart_single_task() {
    let task = Task::new("task-1", "Single Task", "A task with no dependencies");
    let mermaid = generate_mermaid_flowchart(&[task], None);

    // Should contain the task
    assert!(mermaid.contains("task-1"));
    assert!(mermaid.contains("Single Task"));

    // Should have valid Mermaid graph syntax
    assert!(mermaid.contains("graph TD"));
}

#[test]
fn test_generate_mermaid_flowchart_with_dependencies() {
    let mut task2 = Task::new("task-2", "Dependent Task", "Depends on task-1");
    task2.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Base Task", "Base task");
    let tasks = vec![task1, task2];

    let mermaid = generate_mermaid_flowchart(&tasks, None);

    // Should contain both tasks
    assert!(mermaid.contains("task-1"));
    assert!(mermaid.contains("task-2"));

    // Should show dependency relationship
    assert!(mermaid.contains("-->") || mermaid.contains("->"));
}

#[test]
fn test_generate_mermaid_flowchain_with_status() {
    let mut task2 = Task::new("task-2", "Completed Task", "Done");
    task2.status = TaskStatus::Completed;
    task2.depends_on = vec!["task-1".to_string()];

    let mut task1 = Task::new("task-1", "Pending Task", "Not started");
    task1.status = TaskStatus::Pending;

    let tasks = vec![task1, task2];
    let mermaid = generate_mermaid_flowchart(&tasks, Some(true));

    // Should contain status indicators
    assert!(mermaid.contains("Pending") || mermaid.contains("○"));
    assert!(mermaid.contains("Completed") || mermaid.contains("✓"));
}

#[test]
fn test_generate_mermaid_graph_from_execution_plan() {
    let task1 = Task::new("task-1", "Task 1", "First");
    let task2 = Task::new("task-2", "Task 2", "Second");
    let tasks = vec![task1, task2];

    let plan = schedule_tasks(tasks).unwrap();
    let mermaid = generate_mermaid_graph(&plan, None);

    // Should produce valid Mermaid syntax
    assert!(mermaid.contains("graph TD") || mermaid.contains("graph LR"));
    assert!(mermaid.contains("task-1"));
    assert!(mermaid.contains("task-2"));
}

#[test]
fn test_generate_mermaid_graph_execution_levels() {
    let mut task2 = Task::new("task-2", "Task 2", "Second");
    task2.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Task 1", "First");
    let tasks = vec![task1, task2];

    let plan = schedule_tasks(tasks).unwrap();
    let mermaid = generate_mermaid_graph(&plan, Some(true));

    // Should show execution levels
    assert!(mermaid.contains("Level") || mermaid.contains("level"));
}

#[test]
fn test_export_mermaid_to_file() {
    use std::fs;
    use std::path::PathBuf;

    let task = Task::new("task-1", "Export Test", "Test export functionality");
    let tasks = vec![task];

    // Create a temporary directory for the test
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_mermaid.mmd");

    // Export to file
    let result = export_mermaid_to_file(&tasks, &test_file, None);

    // Should succeed
    assert!(result.is_ok());

    // File should exist
    assert!(test_file.exists());

    // File should contain Mermaid syntax
    let content = fs::read_to_string(&test_file).unwrap();
    assert!(content.contains("graph TD"));
    assert!(content.contains("task-1"));

    // Cleanup
    let _ = fs::remove_file(&test_file);
}

#[test]
fn test_export_mermaid_to_file_creates_directory() {
    use std::fs;

    let task = Task::new("task-1", "Directory Test", "Test directory creation");
    let tasks = vec![task];

    // Create a path with non-existent directories
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("nonexistent").join("dir").join("test.mmd");

    // Export should create parent directories
    let result = export_mermaid_to_file(&tasks, &test_file, None);

    // Should succeed
    assert!(result.is_ok());

    // File should exist
    assert!(test_file.exists());

    // Cleanup
    let _ = fs::remove_file(&test_file);
    let _ = fs::remove_dir_all(temp_dir.join("nonexistent"));
}

#[test]
fn test_generate_mermaid_flowchart_complex_dag() {
    // Diamond pattern:
    //     task-1
    //     /     \
    // task-2   task-3
    //     \     /
    //     task-4

    let mut task4 = Task::new("task-4", "Merge", "Merge point");
    task4.depends_on = vec!["task-2".to_string(), "task-3".to_string()];

    let mut task2 = Task::new("task-2", "Branch A", "First branch");
    task2.depends_on = vec!["task-1".to_string()];

    let mut task3 = Task::new("task-3", "Branch B", "Second branch");
    task3.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Root", "Root task");

    let tasks = vec![task1, task2, task3, task4];
    let mermaid = generate_mermaid_flowchart(&tasks, None);

    // All tasks should be present
    assert!(mermaid.contains("task-1"));
    assert!(mermaid.contains("task-2"));
    assert!(mermaid.contains("task-3"));
    assert!(mermaid.contains("task-4"));

    // Should have valid Mermaid syntax
    assert!(mermaid.contains("graph TD"));
}

#[test]
fn test_generate_mermaid_without_status() {
    let mut task = Task::new("task-1", "Task", "Description");
    task.status = TaskStatus::InProgress;

    let tasks = vec![task];
    let mermaid = generate_mermaid_flowchart(&tasks, Some(false));

    // Should not include status when disabled
    // (Status symbols or text should not appear)
    let has_status = mermaid.contains("○")
        || mermaid.contains("⚙")
        || mermaid.contains("✓")
        || mermaid.contains("✗")
        || mermaid.contains("Pending")
        || mermaid.contains("InProgress");

    assert!(
        !has_status,
        "Should not contain status indicators when disabled"
    );
}

#[test]
fn test_generate_mermaid_id_sanitization() {
    // Task IDs that might need sanitization for Mermaid syntax
    let mut task2 = Task::new("task-with-dash", "Task 2", "Has dash");
    task2.depends_on = vec!["task.with.dot".to_string()];

    let mut task1 = Task::new("task.with.dot", "Task 1", "Has dots");
    let tasks = vec![task1, task2];

    let mermaid = generate_mermaid_flowchart(&tasks, None);

    // Should generate valid Mermaid syntax even with special characters
    assert!(mermaid.contains("graph TD"));

    // IDs should be properly escaped or quoted
    // (Mermaid can handle most characters, but let's verify it's valid)
    assert!(!mermaid.is_empty());
}
