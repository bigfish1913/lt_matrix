//! Workspace state lifecycle integration tests
//!
//! Tests the integration of workspace state persistence with task lifecycle,
//! including auto-reset of in_progress tasks and error handling.

use ltmatrix::workspace::WorkspaceState;
use ltmatrix::models::{Task, TaskStatus};
use std::fs;
use tempfile::TempDir;

// ==================== State Transformation Tests ====================

#[test]
fn test_load_resets_in_progress_to_pending() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create state with InProgress task
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::InProgress;

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Load should reset InProgress to Pending
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks.len(), 1);
    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(loaded_state.tasks[0].id, "task-1");
}

#[test]
fn test_load_preserves_completed_status() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create state with Completed task
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::Completed;

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Load should preserve Completed status
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Completed);
}

#[test]
fn test_load_preserves_failed_status() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create state with Failed task
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::Failed;

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Load should preserve Failed status
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Failed);
}

#[test]
fn test_load_resets_blocked_to_pending() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create state with Blocked task
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::Blocked;

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Load should reset Blocked to Pending
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Pending);
}

#[test]
fn test_load_transforms_mixed_statuses() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create state with various statuses
    let mut task1 = Task::new("task-1", "Completed", "Done");
    task1.status = TaskStatus::Completed;

    let mut task2 = Task::new("task-2", "In Progress", "Working");
    task2.status = TaskStatus::InProgress;

    let mut task3 = Task::new("task-3", "Blocked", "Waiting");
    task3.status = TaskStatus::Blocked;

    let mut task4 = Task::new("task-4", "Pending", "Not started");
    task4.status = TaskStatus::Pending;

    let mut task5 = Task::new("task-5", "Failed", "Error");
    task5.status = TaskStatus::Failed;

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![
        task1, task2, task3, task4, task5
    ]);
    state.save().unwrap();

    // Load should transform appropriately
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Completed); // Preserved
    assert_eq!(loaded_state.tasks[1].status, TaskStatus::Pending); // Reset from InProgress
    assert_eq!(loaded_state.tasks[2].status, TaskStatus::Pending); // Reset from Blocked
    assert_eq!(loaded_state.tasks[3].status, TaskStatus::Pending); // Preserved
    assert_eq!(loaded_state.tasks[4].status, TaskStatus::Failed); // Preserved
}

// ==================== Error Handling Tests ====================

#[test]
fn test_load_handles_corrupted_json() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create corrupted JSON file
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let manifest_path = ltmatrix_dir.join("tasks-manifest.json");
    fs::write(&manifest_path, "{ invalid json }").unwrap();

    // Load should handle error gracefully
    let result = WorkspaceState::load_with_transform(project_root.to_path_buf());

    assert!(result.is_err());
}

#[test]
fn test_load_handles_truncated_json() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create truncated JSON file
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let manifest_path = ltmatrix_dir.join("tasks-manifest.json");
    fs::write(&manifest_path, "{\"tasks\": [").unwrap();

    // Load should handle error gracefully
    let result = WorkspaceState::load_with_transform(project_root.to_path_buf());

    assert!(result.is_err());
}

#[test]
fn test_load_handles_missing_required_fields() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create JSON with missing required fields
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let manifest_path = ltmatrix_dir.join("tasks-manifest.json");
    let incomplete_json = r#"{
        "project_root": "/test",
        "tasks": [
            {
                "id": "task-1",
                "title": "Task"
                // Missing required fields like status, created_at
            }
        ]
    }"#;

    fs::write(&manifest_path, incomplete_json).unwrap();

    // Load should handle error gracefully
    let result = WorkspaceState::load_with_transform(project_root.to_path_buf());

    assert!(result.is_err());
}

#[test]
fn test_load_returns_error_when_file_missing() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Don't create manifest file
    let result = WorkspaceState::load_with_transform(project_root.to_path_buf());

    assert!(result.is_err());
}

#[test]
fn test_save_creates_directory_if_missing() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Don't create .ltmatrix directory
    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);

    // Save should create directory
    let result = state.save();
    assert!(result.is_ok());

    // Verify directory exists
    let ltmatrix_dir = project_root.join(".ltmatrix");
    assert!(ltmatrix_dir.exists());

    // Verify file exists
    let manifest_path = ltmatrix_dir.join("tasks-manifest.json");
    assert!(manifest_path.exists());
}

// ==================== Execute Stage Integration Tests ====================

#[test]
fn test_save_after_task_execution() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create initial state
    let task1 = Task::new("task-1", "First", "First task");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1]);
    state.save().unwrap();

    // Simulate task completion (update task status)
    let mut loaded_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    loaded_state.tasks[0].status = TaskStatus::Completed;

    // Save after execution
    let result = loaded_state.save();
    assert!(result.is_ok());

    // Verify persistence
    let final_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    assert_eq!(final_state.tasks[0].status, TaskStatus::Completed);
}

#[test]
fn test_state_consistency_after_multiple_saves() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create initial state
    let task1 = Task::new("task-1", "Task 1", "Description 1");
    let task2 = Task::new("task-2", "Task 2", "Description 2");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);
    state.save().unwrap();

    // First update
    let mut state1 = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    state1.tasks[0].status = TaskStatus::Completed;
    state1.save().unwrap();

    // Second update
    let mut state2 = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    state2.tasks[1].status = TaskStatus::Completed;
    state2.save().unwrap();

    // Verify both updates persisted
    let final_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    assert_eq!(final_state.tasks[0].status, TaskStatus::Completed);
    assert_eq!(final_state.tasks[1].status, TaskStatus::Completed);
}

#[test]
fn test_metadata_updated_on_each_save() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);

    // First save
    let state1 = state.save().unwrap();
    let modified1 = state1.metadata.modified_at;

    // Wait to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Second save
    let state2 = state1.save().unwrap();
    let modified2 = state2.metadata.modified_at;

    // Verify timestamp updated
    assert!(modified2 > modified1);
}

#[test]
fn test_preserves_subtasks_during_transform() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with subtasks
    let mut subtask = Task::new("task-2", "Subtask", "Child task");
    subtask.status = TaskStatus::InProgress;

    let mut parent = Task::new("task-1", "Parent", "Parent task");
    parent.status = TaskStatus::InProgress;
    parent.subtasks = vec![subtask];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![parent]);
    state.save().unwrap();

    // Load with transform
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Verify parent and child both reset to Pending
    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(loaded_state.tasks[0].subtasks.len(), 1);
    assert_eq!(loaded_state.tasks[0].subtasks[0].status, TaskStatus::Pending);
}

#[test]
fn test_preserves_dependencies_during_transform() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create dependent tasks
    let mut task2 = Task::new("task-2", "Dependent", "Depends on task-1");
    task2.status = TaskStatus::InProgress;
    task2.depends_on = vec!["task-1".to_string()];

    let mut task1 = Task::new("task-1", "Dependency", "Base task");
    task1.status = TaskStatus::Completed;

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);
    state.save().unwrap();

    // Load with transform
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Verify dependencies preserved
    assert_eq!(loaded_state.tasks[1].depends_on.len(), 1);
    assert_eq!(loaded_state.tasks[1].depends_on[0], "task-1");

    // Verify statuses transformed correctly
    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Completed); // Preserved
    assert_eq!(loaded_state.tasks[1].status, TaskStatus::Pending); // Reset from InProgress
}

#[test]
fn test_handles_empty_task_list() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create state with no tasks
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![]);
    state.save().unwrap();

    // Load should handle gracefully
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks.len(), 0);
}

// ==================== Orphaned Task Detection Tests ====================

#[test]
fn test_detect_orphaned_tasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with dependency on non-existent task
    let mut task2 = Task::new("task-2", "Dependent Task", "Depends on missing task");
    task2.depends_on = vec!["non-existent-task".to_string()];

    let task1 = Task::new("task-1", "Valid Task", "No dependencies");

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);

    // Detect orphaned tasks
    let orphaned = state.detect_orphaned_tasks();

    assert_eq!(orphaned.len(), 1);
    assert_eq!(orphaned[0].0, "task-2");
    assert_eq!(orphaned[0].1, vec!["non-existent-task".to_string()]);
}

#[test]
fn test_detect_orphaned_tasks_multiple_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with multiple missing dependencies
    let mut task = Task::new("task-1", "Multi-Dep", "Multiple missing deps");
    task.depends_on = vec![
        "missing-1".to_string(),
        "missing-2".to_string(),
        "missing-3".to_string(),
    ];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);

    let orphaned = state.detect_orphaned_tasks();

    assert_eq!(orphaned.len(), 1);
    assert_eq!(orphaned[0].0, "task-1");
    assert_eq!(orphaned[0].1.len(), 3);
}

#[test]
fn test_detect_orphaned_tasks_in_subtasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create nested task with orphaned dependency
    let mut subtask = Task::new("subtask-1", "Subtask", "Has missing dependency");
    subtask.depends_on = vec!["missing-subtask-dep".to_string()];

    let mut parent = Task::new("task-1", "Parent", "Parent task");
    parent.subtasks = vec![subtask];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![parent]);

    let orphaned = state.detect_orphaned_tasks();

    assert_eq!(orphaned.len(), 1);
    assert_eq!(orphaned[0].0, "subtask-1");
    assert_eq!(orphaned[0].1, vec!["missing-subtask-dep".to_string()]);
}

#[test]
fn test_detect_orphaned_tasks_no_orphans() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create valid dependency chain
    let mut task2 = Task::new("task-2", "Dependent", "Depends on task-1");
    task2.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Base", "Base task");

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);

    let orphaned = state.detect_orphaned_tasks();

    assert_eq!(orphaned.len(), 0);
}

#[test]
fn test_cleanup_orphaned_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with mix of valid and invalid dependencies
    let mut task2 = Task::new("task-2", "Mixed Deps", "Mixed dependencies");
    task2.depends_on = vec![
        "task-1".to_string(),      // Valid
        "missing-1".to_string(),   // Invalid
        "missing-2".to_string(),   // Invalid
    ];

    let task1 = Task::new("task-1", "Valid", "Valid dependency");

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);

    // Cleanup orphaned dependencies
    let cleaned_count = state.cleanup_orphaned_dependencies();

    assert_eq!(cleaned_count, 2); // Two invalid dependencies removed
    assert_eq!(state.tasks[1].depends_on.len(), 1); // Only valid dep remains
    assert_eq!(state.tasks[1].depends_on[0], "task-1");
}

#[test]
fn test_cleanup_orphaned_dependencies_in_subtasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create subtask with invalid dependencies
    let mut subtask = Task::new("subtask-1", "Subtask", "Has invalid deps");
    subtask.depends_on = vec!["missing".to_string()];

    let mut parent = Task::new("task-1", "Parent", "Parent task");
    parent.subtasks = vec![subtask];

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![parent]);

    // Cleanup should affect subtasks too
    let cleaned_count = state.cleanup_orphaned_dependencies();

    assert_eq!(cleaned_count, 1);
    assert_eq!(state.tasks[0].subtasks[0].depends_on.len(), 0);
}

#[test]
fn test_validate_dependency_graph_valid() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create valid linear dependency chain
    let mut task3 = Task::new("task-3", "Third", "Depends on task-2");
    task3.depends_on = vec!["task-2".to_string()];

    let mut task2 = Task::new("task-2", "Second", "Depends on task-1");
    task2.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "First", "Base task");

    let state = WorkspaceState::new(
        project_root.to_path_buf(),
        vec![task1, task2, task3]
    );

    // Should validate successfully
    assert!(state.validate_dependency_graph().is_ok());
}

#[test]
fn test_validate_dependency_graph_orphaned() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with orphaned dependency
    let mut task = Task::new("task-1", "Task", "Has missing dependency");
    task.depends_on = vec!["missing-task".to_string()];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);

    // Should fail validation
    let result = state.validate_dependency_graph();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("orphaned dependencies"));
}

#[test]
fn test_validate_dependency_graph_circular() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create circular dependency: task-1 -> task-2 -> task-1
    let mut task2 = Task::new("task-2", "Second", "Depends on task-1");
    task2.depends_on = vec!["task-1".to_string()];

    let mut task1 = Task::new("task-1", "First", "Depends on task-2");
    task1.depends_on = vec!["task-2".to_string()];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);

    // Should detect circular dependency
    let result = state.validate_dependency_graph();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Circular"));
}

#[test]
fn test_validate_dependency_graph_self_dependency() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create self-referencing task
    let mut task = Task::new("task-1", "Self Dep", "Depends on itself");
    task.depends_on = vec!["task-1".to_string()];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);

    // Should detect circular dependency (self-reference is a cycle)
    let result = state.validate_dependency_graph();
    assert!(result.is_err());
}

#[test]
fn test_detect_orphaned_tasks_diamond_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Diamond pattern with one broken edge
    //     task-1
    //     /     \
    // task-2   task-3 (missing)
    //     \     /
    //     task-4

    let mut task4 = Task::new("task-4", "Merge", "Merge point");
    task4.depends_on = vec!["task-2".to_string(), "task-3".to_string()];

    let mut task2 = Task::new("task-2", "Branch A", "First branch");
    task2.depends_on = vec!["task-1".to_string()];

    let mut task3 = Task::new("task-3", "Branch B", "Second branch");
    task3.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Root", "Root task");

    // Create state without task-3 to simulate it being deleted
    let state = WorkspaceState::new(
        project_root.to_path_buf(),
        vec![task1, task2, task4] // task-3 is missing
    );

    let orphaned = state.detect_orphaned_tasks();

    // task-4 should be detected as having orphaned dependency on task-3
    assert_eq!(orphaned.len(), 1);
    assert_eq!(orphaned[0].0, "task-4");
    assert!(orphaned[0].1.contains(&"task-3".to_string()));
}

#[test]
fn test_cleanup_and_save_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create state with orphaned dependencies
    let mut task2 = Task::new("task-2", "Orphaned", "Has missing dependency");
    task2.depends_on = vec!["missing".to_string()];

    let task1 = Task::new("task-1", "Valid", "Valid task");

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);
    state.save().unwrap();

    // Load, cleanup, and save
    let mut loaded_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    let cleaned_count = loaded_state.cleanup_orphaned_dependencies();
    loaded_state.save().unwrap();

    assert_eq!(cleaned_count, 1);

    // Load again and verify cleanup persisted
    let final_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    assert_eq!(final_state.tasks[1].depends_on.len(), 0);
}
