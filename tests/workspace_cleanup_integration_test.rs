//! Integration tests for workspace cleanup and reset functionality
//!
//! Tests the cleanup operations including:
//! - Orphaned dependency cleanup
//! - State file removal
//! - Workspace reset
//! - Dependency graph validation

use ltmatrix::models::{Task, TaskStatus};
use ltmatrix::workspace::WorkspaceState;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ==================== Orphaned Dependency Cleanup Tests ====================

#[test]
fn test_cleanup_orphaned_dependencies_simple() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create tasks with orphaned dependencies
    let task1 = Task::new("task-1", "Valid Task", "No deps");
    let mut task2 = Task::new("task-2", "Task With Orphan", "Has missing dep");
    task2.depends_on = vec!["missing-task".to_string()];

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);

    // Detect orphaned dependencies
    let orphaned = state.detect_orphaned_tasks();
    assert_eq!(orphaned.len(), 1);
    assert_eq!(orphaned[0].0, "task-2");
    assert_eq!(orphaned[0].1, vec!["missing-task".to_string()]);

    // Cleanup orphaned dependencies
    let cleaned_count = state.cleanup_orphaned_dependencies();
    assert_eq!(cleaned_count, 1);

    // Verify no more orphaned dependencies
    let orphaned_after = state.detect_orphaned_tasks();
    assert_eq!(orphaned_after.len(), 0);
    assert_eq!(state.tasks[1].depends_on.len(), 0);
}

#[test]
fn test_cleanup_orphaned_dependencies_multiple() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create multiple tasks with various orphaned dependencies
    let task1 = Task::new("task-1", "Valid", "Exists");
    let mut task2 = Task::new("task-2", "One Orphan", "Has one missing");
    task2.depends_on = vec!["task-1".to_string(), "missing-1".to_string()];

    let mut task3 = Task::new("task-3", "Two Orphans", "Has two missing");
    task3.depends_on = vec!["missing-2".to_string(), "missing-3".to_string()];

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2, task3]);

    // Cleanup orphaned dependencies
    let cleaned_count = state.cleanup_orphaned_dependencies();

    // Should remove 3 orphaned dependencies
    assert_eq!(cleaned_count, 3);

    // Verify task2 kept only valid dependency
    assert_eq!(state.tasks[1].depends_on, vec!["task-1"]);

    // Verify task3 has no dependencies left (both were invalid)
    assert_eq!(state.tasks[2].depends_on.len(), 0);
}

#[test]
fn test_cleanup_preserves_valid_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create valid dependency chain
    let task1 = Task::new("task-1", "First", "Start");
    let mut task2 = Task::new("task-2", "Second", "After first");
    task2.depends_on = vec!["task-1".to_string()];
    let mut task3 = Task::new("task-3", "Third", "After second");
    task3.depends_on = vec!["task-2".to_string()];

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2, task3]);

    // Cleanup should not remove any valid dependencies
    let cleaned_count = state.cleanup_orphaned_dependencies();
    assert_eq!(cleaned_count, 0);

    // Verify all dependencies preserved
    assert_eq!(state.tasks[1].depends_on, vec!["task-1"]);
    assert_eq!(state.tasks[2].depends_on, vec!["task-2"]);
}

#[test]
fn test_cleanup_with_nested_subtasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create parent task with subtasks having orphaned dependencies
    let mut subtask1 = Task::new("subtask-1", "Subtask 1", "Has orphan");
    subtask1.depends_on = vec!["missing-subtask".to_string()];

    let mut parent = Task::new("parent-1", "Parent", "Has subtask with orphan");
    parent.subtasks = vec![subtask1];

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![parent]);

    // Cleanup should handle nested subtasks
    let cleaned_count = state.cleanup_orphaned_dependencies();
    assert_eq!(cleaned_count, 1);

    // Verify subtask's orphaned dependency removed
    assert_eq!(state.tasks[0].subtasks[0].depends_on.len(), 0);
}

#[test]
fn test_cleanup_with_duplicate_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with duplicate valid dependencies
    let task1 = Task::new("task-1", "Valid", "Exists");
    let mut task2 = Task::new("task-2", "Duplicate Deps", "Has duplicate");
    task2.depends_on = vec![
        "task-1".to_string(),
        "task-1".to_string(), // Duplicate
    ];

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);

    // Cleanup should not remove valid dependencies (even duplicates)
    let cleaned_count = state.cleanup_orphaned_dependencies();
    assert_eq!(cleaned_count, 0);

    // Dependencies should remain (may still have duplicates)
    assert!(state.tasks[1].depends_on.contains(&"task-1".to_string()));
}

#[test]
fn test_cleanup_empty_dependency_list() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with no dependencies
    let task1 = Task::new("task-1", "No Deps", "Independent");

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task1]);

    // Cleanup should handle empty dependency list
    let cleaned_count = state.cleanup_orphaned_dependencies();
    assert_eq!(cleaned_count, 0);
}

// ==================== Dependency Graph Validation Tests ====================

#[test]
fn test_validate_valid_dependency_graph() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create valid linear dependency chain
    let task1 = Task::new("task-1", "First", "Start");
    let mut task2 = Task::new("task-2", "Second", "After first");
    task2.depends_on = vec!["task-1".to_string()];
    let mut task3 = Task::new("task-3", "Third", "After second");
    task3.depends_on = vec!["task-2".to_string()];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2, task3]);

    // Validation should pass
    assert!(state.validate_dependency_graph().is_ok());
}

#[test]
fn test_validate_detects_orphaned_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with orphaned dependency
    let task1 = Task::new("task-1", "Valid", "Exists");
    let mut task2 = Task::new("task-2", "With Orphan", "Has missing dep");
    task2.depends_on = vec!["missing-task".to_string()];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);

    // Validation should fail
    let result = state.validate_dependency_graph();
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(error_msg.contains("orphaned") || error_msg.contains("broken"));
}

#[test]
fn test_validate_detects_circular_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create circular dependency: A -> B -> A
    let mut task_a = Task::new("task-a", "Task A", "Depends on B");
    task_a.depends_on = vec!["task-b".to_string()];

    let mut task_b = Task::new("task-b", "Task B", "Depends on A");
    task_b.depends_on = vec!["task-a".to_string()];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task_a, task_b]);

    // Validation should fail
    let result = state.validate_dependency_graph();
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(error_msg.contains("circular") || error_msg.contains("cycle"));
}

#[test]
fn test_validate_detects_self_dependency() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task that depends on itself
    let mut task1 = Task::new("task-1", "Self Dependent", "Depends on itself");
    task1.depends_on = vec!["task-1".to_string()];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1]);

    // Validation should fail (self-dependency is a cycle)
    let result = state.validate_dependency_graph();
    assert!(result.is_err());
}

#[test]
fn test_validate_diamond_dependency_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create diamond pattern: A -> B, A -> C, B -> D, C -> D
    let task_a = Task::new("task-a", "A", "Top");

    let mut task_b = Task::new("task-b", "B", "Branch 1");
    task_b.depends_on = vec!["task-a".to_string()];

    let mut task_c = Task::new("task-c", "C", "Branch 2");
    task_c.depends_on = vec!["task-a".to_string()];

    let mut task_d = Task::new("task-d", "D", "Merge point");
    task_d.depends_on = vec!["task-b".to_string(), "task-c".to_string()];

    let state = WorkspaceState::new(
        project_root.to_path_buf(),
        vec![task_a, task_b, task_c, task_d],
    );

    // Diamond pattern is valid (not a cycle)
    assert!(state.validate_dependency_graph().is_ok());
}

#[test]
fn test_validate_with_complex_graph() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create complex valid dependency graph
    let task1 = Task::new("task-1", "Base 1", "Foundation");

    let mut task2 = Task::new("task-2", "Base 2", "Foundation");
    task2.depends_on = vec!["task-1".to_string()];

    let mut task3 = Task::new("task-3", "Branch 1", "Depends on 2");
    task3.depends_on = vec!["task-2".to_string()];

    let mut task4 = Task::new("task-4", "Branch 2", "Depends on 2");
    task4.depends_on = vec!["task-2".to_string()];

    let mut task5 = Task::new("task-5", "Merge", "Depends on 3 and 4");
    task5.depends_on = vec!["task-3".to_string(), "task-4".to_string()];

    let state = WorkspaceState::new(
        project_root.to_path_buf(),
        vec![task1, task2, task3, task4, task5],
    );

    // Complex graph should be valid
    assert!(state.validate_dependency_graph().is_ok());
}

// ==================== State File Cleanup Tests ====================

#[test]
fn test_remove_workspace_state_file() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create and save workspace state
    let task1 = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1]);
    state.save().unwrap();

    // Verify file exists
    let manifest_path = project_root.join(".ltmatrix").join("tasks-manifest.json");
    assert!(manifest_path.exists());

    // Remove the state file
    fs::remove_file(&manifest_path).unwrap();

    // Verify file is gone
    assert!(!manifest_path.exists());

    // Loading should fail
    let result = WorkspaceState::load(project_root.to_path_buf());
    assert!(result.is_err());
}

#[test]
fn test_remove_entire_workspace_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create and save workspace state
    let task1 = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1]);
    state.save().unwrap();

    // Verify directory exists
    let ltmatrix_dir = project_root.join(".ltmatrix");
    assert!(ltmatrix_dir.exists());

    // Remove entire directory
    fs::remove_dir_all(&ltmatrix_dir).unwrap();

    // Verify directory is gone
    assert!(!ltmatrix_dir.exists());

    // Loading should fail
    let result = WorkspaceState::load(project_root.to_path_buf());
    assert!(result.is_err());
}

#[test]
fn test_reset_workspace_creates_empty_state() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create and save workspace state with tasks
    let task1 = Task::new("task-1", "Task 1", "Description");
    let task2 = Task::new("task-2", "Task 2", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);
    state.save().unwrap();

    // Verify state has tasks
    let loaded_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    assert_eq!(loaded_state.tasks.len(), 2);

    // Reset: create new empty state and save
    let empty_state = WorkspaceState::new(project_root.to_path_buf(), vec![]);
    empty_state.save().unwrap();

    // Verify state is now empty
    let reset_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    assert_eq!(reset_state.tasks.len(), 0);
}

#[test]
fn test_cleanup_then_save_persist_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create state with orphaned dependencies
    let task1 = Task::new("task-1", "Valid", "Exists");
    let mut task2 = Task::new("task-2", "With Orphan", "Has missing dep");
    task2.depends_on = vec!["missing".to_string()];

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);
    state.save().unwrap();

    // Load, cleanup, and save
    let mut loaded_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    loaded_state.cleanup_orphaned_dependencies();
    loaded_state.save().unwrap();

    // Reload and verify cleanup persisted
    let reloaded_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    assert_eq!(reloaded_state.tasks[1].depends_on.len(), 0);
}

#[test]
fn test_workspace_state_integrity_after_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create complex state with various issues
    let mut task1 = Task::new("task-1", "Valid", "No issues");
    let mut task2 = Task::new("task-2", "Has Orphan", "Missing dep");
    task2.depends_on = vec!["missing-1".to_string()];
    let mut task3 = Task::new("task-3", "Valid Dep", "Valid dep");
    task3.depends_on = vec!["task-1".to_string()];

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2, task3]);
    state.save().unwrap();

    // Load and cleanup
    let mut loaded_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();

    // Verify initial state
    assert_eq!(loaded_state.tasks.len(), 3);
    assert_eq!(loaded_state.tasks[1].depends_on.len(), 1);

    // Cleanup
    let cleaned_count = loaded_state.cleanup_orphaned_dependencies();
    assert_eq!(cleaned_count, 1);

    // Verify state integrity after cleanup
    assert_eq!(loaded_state.tasks.len(), 3);
    assert_eq!(loaded_state.tasks[0].id, "task-1");
    assert_eq!(loaded_state.tasks[1].id, "task-2");
    assert_eq!(loaded_state.tasks[2].id, "task-3");

    // Verify correct dependencies
    assert_eq!(loaded_state.tasks[1].depends_on.len(), 0);
    assert_eq!(loaded_state.tasks[2].depends_on, vec!["task-1"]);

    // Verify graph is now valid
    assert!(loaded_state.validate_dependency_graph().is_ok());
}

#[test]
fn test_cleanup_with_corrupted_state_recovery() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create valid state
    let task1 = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1]);
    state.save().unwrap();

    // Corrupt the state file
    let manifest_path = project_root.join(".ltmatrix").join("tasks-manifest.json");
    fs::write(&manifest_path, "{ corrupted json }").unwrap();

    // Attempting to load should fail
    let result = WorkspaceState::load(project_root.to_path_buf());
    assert!(result.is_err());

    // load_or_create should recover with new empty state
    let recovered_state = WorkspaceState::load_or_create(project_root.to_path_buf()).unwrap();
    assert_eq!(recovered_state.tasks.len(), 0);

    // Verify new valid file was created
    let reloaded = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    assert_eq!(reloaded.tasks.len(), 0);
}

#[test]
fn test_cleanup_preserves_task_properties() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with all properties set
    let mut task1 = Task::new("task-1", "Valid", "No issues");
    task1.status = TaskStatus::Completed;
    task1.complexity = ltmatrix::models::TaskComplexity::Complex;

    let mut task2 = Task::new("task-2", "Has Issues", "With missing deps");
    task2.status = TaskStatus::InProgress;
    task2.depends_on = vec!["missing".to_string()];
    task2.description = "Will be cleaned up".to_string();

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);
    state.save().unwrap();

    // Load and cleanup
    let mut loaded_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    loaded_state.cleanup_orphaned_dependencies();
    loaded_state.save().unwrap();

    // Reload and verify all properties preserved
    let final_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();

    assert_eq!(final_state.tasks[0].id, "task-1");
    assert_eq!(final_state.tasks[0].title, "Valid");
    assert_eq!(final_state.tasks[0].status, TaskStatus::Completed);
    assert_eq!(
        final_state.tasks[0].complexity,
        ltmatrix::models::TaskComplexity::Complex
    );

    assert_eq!(final_state.tasks[1].id, "task-2");
    assert_eq!(final_state.tasks[1].title, "Has Issues");
    assert_eq!(final_state.tasks[1].description, "Will be cleaned up");
    assert_eq!(final_state.tasks[1].status, TaskStatus::InProgress);
    assert_eq!(final_state.tasks[1].depends_on.len(), 0);
}

// ==================== Workspace Removal Tests ====================

#[test]
fn test_workspace_cleanup_removes_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create workspace state
    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Verify directory exists
    let ltmatrix_dir = project_root.join(".ltmatrix");
    assert!(ltmatrix_dir.exists());

    // Cleanup workspace
    WorkspaceState::cleanup(&project_root.to_path_buf()).unwrap();

    // Verify directory removed
    assert!(!ltmatrix_dir.exists());
}

#[test]
fn test_workspace_cleanup_with_no_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // No workspace exists
    let ltmatrix_dir = project_root.join(".ltmatrix");
    assert!(!ltmatrix_dir.exists());

    // Cleanup should succeed (no-op)
    WorkspaceState::cleanup(&project_root.to_path_buf()).unwrap();

    // Still doesn't exist
    assert!(!ltmatrix_dir.exists());
}

#[test]
fn test_workspace_cleanup_removes_all_files() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create workspace state
    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Create additional files in .ltmatrix directory
    let ltmatrix_dir = project_root.join(".ltmatrix");
    let extra_file = ltmatrix_dir.join("extra.txt");
    fs::write(&extra_file, "extra content").unwrap();

    // Verify both files exist
    assert!(ltmatrix_dir.join("tasks-manifest.json").exists());
    assert!(extra_file.exists());

    // Cleanup workspace
    WorkspaceState::cleanup(&project_root.to_path_buf()).unwrap();

    // Verify entire directory removed
    assert!(!ltmatrix_dir.exists());
}

// ==================== Workspace Exists Tests ====================

#[test]
fn test_workspace_exists_true() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create workspace state
    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Verify exists returns true
    assert!(WorkspaceState::exists(&project_root.to_path_buf()));
}

#[test]
fn test_workspace_exists_false() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // No workspace created
    assert!(!WorkspaceState::exists(&project_root.to_path_buf()));
}

#[test]
fn test_workspace_exists_with_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create .ltmatrix directory but no manifest file
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    // Verify exists returns false (no manifest file)
    assert!(!WorkspaceState::exists(&project_root.to_path_buf()));
}

// ==================== Reset All Tests ====================

#[test]
fn test_reset_all_clears_all_statuses() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create tasks with various statuses
    let mut task1 = Task::new("task-1", "Completed", "First");
    task1.status = TaskStatus::Completed;
    task1.started_at = Some(chrono::Utc::now());
    task1.completed_at = Some(chrono::Utc::now());

    let mut task2 = Task::new("task-2", "In Progress", "Second");
    task2.status = TaskStatus::InProgress;
    task2.started_at = Some(chrono::Utc::now());

    let mut task3 = Task::new("task-3", "Failed", "Third");
    task3.status = TaskStatus::Failed;
    task3.error = Some("Error".to_string());

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2, task3]);

    // Reset all tasks
    state.reset_all().unwrap();

    // Verify all tasks reset to pending
    assert_eq!(state.tasks[0].status, TaskStatus::Pending);
    assert!(state.tasks[0].started_at.is_none());
    assert!(state.tasks[0].completed_at.is_none());

    assert_eq!(state.tasks[1].status, TaskStatus::Pending);
    assert!(state.tasks[1].started_at.is_none());

    assert_eq!(state.tasks[2].status, TaskStatus::Pending);
    assert!(state.tasks[2].error.is_none());
}

#[test]
fn test_reset_all_preserves_other_properties() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with various properties
    let mut task = Task::new("task-1", "Task", "Description");
    task.status = TaskStatus::Completed;
    task.complexity = ltmatrix::models::TaskComplexity::Complex;
    task.depends_on = vec!["dep-1".to_string()];
    task.retry_count = 3;
    task.session_id = Some("session-123".to_string());

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);

    // Reset all tasks
    state.reset_all().unwrap();

    // Verify properties preserved
    assert_eq!(state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(
        state.tasks[0].complexity,
        ltmatrix::models::TaskComplexity::Complex
    );
    assert_eq!(state.tasks[0].depends_on, vec!["dep-1".to_string()]);
    assert_eq!(state.tasks[0].retry_count, 3);
    assert_eq!(state.tasks[0].session_id, Some("session-123".to_string()));
}

#[test]
fn test_reset_all_with_nested_subtasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create parent with nested subtasks
    let mut subtask = Task::new("subtask-1", "Subtask", "Nested");
    subtask.status = TaskStatus::InProgress;
    subtask.started_at = Some(chrono::Utc::now());

    let mut parent = Task::new("parent-1", "Parent", "Parent task");
    parent.status = TaskStatus::Completed;
    parent.started_at = Some(chrono::Utc::now());
    parent.completed_at = Some(chrono::Utc::now());
    parent.subtasks = vec![subtask];

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![parent]);

    // Reset all tasks
    state.reset_all().unwrap();

    // Verify both parent and subtask reset
    assert_eq!(state.tasks[0].status, TaskStatus::Pending);
    assert!(state.tasks[0].started_at.is_none());
    assert!(state.tasks[0].completed_at.is_none());

    assert_eq!(state.tasks[0].subtasks[0].status, TaskStatus::Pending);
    assert!(state.tasks[0].subtasks[0].started_at.is_none());
}

// ==================== Reset Failed Tests ====================

#[test]
fn test_reset_failed_only_resets_failed_tasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create tasks with various statuses
    let mut task1 = Task::new("task-1", "Completed", "First");
    task1.status = TaskStatus::Completed;

    let mut task2 = Task::new("task-2", "Failed", "Second");
    task2.status = TaskStatus::Failed;
    task2.error = Some("Error".to_string());

    let mut task3 = Task::new("task-3", "Pending", "Third");
    task3.status = TaskStatus::Pending;

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2, task3]);

    // Reset failed tasks
    let reset_count = state.reset_failed().unwrap();

    // Verify only failed task was reset
    assert_eq!(reset_count, 1);
    assert_eq!(state.tasks[0].status, TaskStatus::Completed); // Unchanged
    assert_eq!(state.tasks[1].status, TaskStatus::Pending); // Reset from failed
    assert!(state.tasks[1].error.is_none());
    assert_eq!(state.tasks[2].status, TaskStatus::Pending); // Unchanged
}

#[test]
fn test_reset_failed_with_nested_subtasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create parent with mixed status subtasks
    let mut subtask1 = Task::new("subtask-1", "Completed Sub", "Done");
    subtask1.status = TaskStatus::Completed;

    let mut subtask2 = Task::new("subtask-2", "Failed Sub", "Error");
    subtask2.status = TaskStatus::Failed;
    subtask2.error = Some("Subtask error".to_string());

    let mut parent = Task::new("parent-1", "Parent", "Parent task");
    parent.status = TaskStatus::Pending;
    parent.subtasks = vec![subtask1, subtask2];

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![parent]);

    // Reset failed tasks
    let reset_count = state.reset_failed().unwrap();

    // Verify only failed subtask was reset
    assert_eq!(reset_count, 1);
    assert_eq!(state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(state.tasks[0].subtasks[0].status, TaskStatus::Completed); // Unchanged
    assert_eq!(state.tasks[0].subtasks[1].status, TaskStatus::Pending); // Reset from failed
    assert!(state.tasks[0].subtasks[1].error.is_none());
}

#[test]
fn test_reset_failed_returns_correct_count() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create multiple failed tasks
    let mut task1 = Task::new("task-1", "Failed 1", "First");
    task1.status = TaskStatus::Failed;

    let mut task2 = Task::new("task-2", "Failed 2", "Second");
    task2.status = TaskStatus::Failed;

    let mut task3 = Task::new("task-3", "OK", "Third");
    task3.status = TaskStatus::Completed;

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2, task3]);

    // Reset failed tasks
    let reset_count = state.reset_failed().unwrap();

    // Verify count is correct
    assert_eq!(reset_count, 2);
}

// ==================== Status Summary Tests ====================

#[test]
fn test_status_summary_empty_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create empty workspace
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![]);

    // Get summary
    let summary = state.status_summary();

    assert_eq!(summary.total(), 0);
    assert_eq!(summary.pending, 0);
    assert_eq!(summary.in_progress, 0);
    assert_eq!(summary.completed, 0);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.blocked, 0);
    assert_eq!(summary.completion_percentage(), 0.0);
}

#[test]
fn test_status_summary_all_statuses() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create tasks with all different statuses
    let mut task1 = Task::new("task-1", "Pending", "First");
    task1.status = TaskStatus::Pending;

    let mut task2 = Task::new("task-2", "In Progress", "Second");
    task2.status = TaskStatus::InProgress;

    let mut task3 = Task::new("task-3", "Completed", "Third");
    task3.status = TaskStatus::Completed;

    let mut task4 = Task::new("task-4", "Failed", "Fourth");
    task4.status = TaskStatus::Failed;

    let mut task5 = Task::new("task-5", "Blocked", "Fifth");
    task5.status = TaskStatus::Blocked;

    let state = WorkspaceState::new(
        project_root.to_path_buf(),
        vec![task1, task2, task3, task4, task5],
    );

    // Get summary
    let summary = state.status_summary();

    assert_eq!(summary.total(), 5);
    assert_eq!(summary.pending, 1);
    assert_eq!(summary.in_progress, 1);
    assert_eq!(summary.completed, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.blocked, 1);
    assert_eq!(summary.completion_percentage(), 20.0);
}

#[test]
fn test_status_summary_counts_subtasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create parent with subtasks
    let subtask1 = Task::new("subtask-1", "Sub 1", "First sub");
    let subtask2 = Task::new("subtask-2", "Sub 2", "Second sub");

    let parent = Task::new("parent-1", "Parent", "Parent task");

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![parent]);
    state.tasks[0].subtasks = vec![subtask1, subtask2];

    // Get summary
    let summary = state.status_summary();

    // Should count parent + subtasks
    assert_eq!(summary.total(), 3);
    assert_eq!(summary.pending, 3);
}

#[test]
fn test_status_summary_completion_percentage() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create 10 tasks, half completed
    let tasks: Vec<Task> = (0..10)
        .map(|i| {
            let mut task = Task::new(&format!("task-{}", i), "Task", "Description");
            if i < 5 {
                task.status = TaskStatus::Completed;
            }
            task
        })
        .collect();

    let state = WorkspaceState::new(project_root.to_path_buf(), tasks);

    // Get summary
    let summary = state.status_summary();

    assert_eq!(summary.total(), 10);
    assert_eq!(summary.completed, 5);
    assert_eq!(summary.completion_percentage(), 50.0);
}

#[test]
fn test_status_summary_with_transform() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create tasks with InProgress status
    let mut task1 = Task::new("task-1", "Completed", "First");
    task1.status = TaskStatus::Completed;

    let mut task2 = Task::new("task-2", "In Progress", "Second");
    task2.status = TaskStatus::InProgress;
    task2.started_at = Some(chrono::Utc::now());

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);
    state.save().unwrap();

    // Load with transform
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Get summary
    let summary = loaded_state.status_summary();

    // InProgress should be reset to Pending
    assert_eq!(summary.total(), 2);
    assert_eq!(summary.completed, 1);
    assert_eq!(summary.pending, 1); // task-2 reset
    assert_eq!(summary.in_progress, 0);
}
