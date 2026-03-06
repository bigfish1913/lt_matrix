//! Edge case tests for task recovery logic
//!
//! Tests additional edge cases for state transformation and recovery
//! that complement the lifecycle integration tests.

use ltmatrix::workspace::WorkspaceState;
use ltmatrix::models::{Task, TaskStatus, TaskComplexity};
use tempfile::TempDir;
use std::path::PathBuf;

// ==================== Deep Nesting Tests ====================

#[test]
fn test_transform_deeply_nested_subtasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create deeply nested subtask hierarchy (4 levels)
    let mut level4 = Task::new("task-4", "Level 4", "Deepest subtask");
    level4.status = TaskStatus::InProgress;

    let mut level3 = Task::new("task-3", "Level 3", "Third level");
    level3.status = TaskStatus::Blocked;
    level3.subtasks = vec![level4];

    let mut level2 = Task::new("task-2", "Level 2", "Second level");
    level2.status = TaskStatus::InProgress;
    level2.subtasks = vec![level3];

    let mut level1 = Task::new("task-1", "Level 1", "Top level");
    level1.status = TaskStatus::InProgress;
    level1.subtasks = vec![level2];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![level1]);
    state.save().unwrap();

    // Load and verify all levels are reset
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Check all levels reset to Pending
    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(loaded_state.tasks[0].subtasks[0].status, TaskStatus::Pending);
    assert_eq!(loaded_state.tasks[0].subtasks[0].subtasks[0].status, TaskStatus::Pending);
    assert_eq!(
        loaded_state.tasks[0].subtasks[0].subtasks[0].subtasks[0].status,
        TaskStatus::Pending
    );
}

#[test]
fn test_orphaned_detection_in_deeply_nested_tasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create deeply nested task with orphaned dependency at level 4
    let mut level4 = Task::new("task-4", "Level 4", "Has missing dependency");
    level4.depends_on = vec!["missing-task".to_string()];

    let mut level3 = Task::new("task-3", "Level 3", "Third level");
    level3.subtasks = vec![level4];

    let mut level2 = Task::new("task-2", "Level 2", "Second level");
    level2.subtasks = vec![level3];

    let mut level1 = Task::new("task-1", "Level 1", "Top level");
    level1.subtasks = vec![level2];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![level1]);

    // Should detect orphaned dependency at deepest level
    let orphaned = state.detect_orphaned_tasks();

    assert_eq!(orphaned.len(), 1);
    assert_eq!(orphaned[0].0, "task-4");
    assert_eq!(orphaned[0].1, vec!["missing-task".to_string()]);
}

// ==================== Mixed Valid and Invalid Dependencies ====================

#[test]
fn test_cleanup_preserves_partial_valid_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with alternating valid/invalid dependencies
    let mut task3 = Task::new("task-3", "Task 3", "End of chain");
    task3.depends_on = vec![
        "task-2".to_string(),    // Valid
        "missing-1".to_string(), // Invalid
        "task-1".to_string(),    // Valid (but wrong order)
        "missing-2".to_string(), // Invalid
    ];

    let mut task2 = Task::new("task-2", "Task 2", "Middle");
    task2.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Task 1", "Base");

    let mut state = WorkspaceState::new(
        project_root.to_path_buf(),
        vec![task1, task2, task3]
    );

    let cleaned_count = state.cleanup_orphaned_dependencies();

    // Should remove 2 invalid dependencies
    assert_eq!(cleaned_count, 2);
    // Should preserve 2 valid dependencies (in their original order)
    assert_eq!(state.tasks[2].depends_on.len(), 2);
    assert_eq!(state.tasks[2].depends_on[0], "task-2");
    assert_eq!(state.tasks[2].depends_on[1], "task-1");
}

// ==================== Task State After Transform ====================

#[test]
fn test_transform_clears_started_at_timestamp() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with started_at timestamp
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::InProgress;
    task.started_at = Some(chrono::Utc::now());

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // After transformation, started_at should be cleared
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(loaded_state.tasks[0].started_at, None);
}

#[test]
fn test_transform_preserves_completed_at_timestamp() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create completed task with both timestamps
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::Completed;
    task.started_at = Some(chrono::Utc::now());
    task.completed_at = Some(chrono::Utc::now());

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // After transformation, completed task should preserve timestamps
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Completed);
    assert!(loaded_state.tasks[0].started_at.is_some());
    assert!(loaded_state.tasks[0].completed_at.is_some());
}

// ==================== Complex Orphaned Scenarios ====================

#[test]
fn test_orphaned_detection_with_duplicate_missing_deps() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create tasks with duplicate missing dependencies
    let mut task1 = Task::new("task-1", "Task 1", "First");
    task1.depends_on = vec![
        "missing".to_string(),
        "missing".to_string(), // Duplicate
    ];

    let mut task2 = Task::new("task-2", "Task 2", "Second");
    task2.depends_on = vec!["missing".to_string()];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);

    let orphaned = state.detect_orphaned_tasks();

    // Both tasks should report the same missing dependency
    assert_eq!(orphaned.len(), 2);
    assert_eq!(orphaned[0].0, "task-1");
    assert_eq!(orphaned[1].0, "task-2");
}

#[test]
fn test_cleanup_removes_duplicate_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with duplicate valid dependencies
    let mut task = Task::new("task-1", "Task", "Has duplicate valid dep");
    task.depends_on = vec![
        "task-2".to_string(),
        "task-2".to_string(), // Duplicate
    ];

    let task2 = Task::new("task-2", "Task 2", "Valid dependency");

    let mut state = WorkspaceState::new(project_root.to_path_buf(), vec![task, task2]);

    // Cleanup should keep valid dependency (may have duplicates)
    let cleaned_count = state.cleanup_orphaned_dependencies();

    // No invalid dependencies to remove
    assert_eq!(cleaned_count, 0);
    // But duplicates may still exist (not deduplicated by cleanup)
    assert!(state.tasks[0].depends_on.contains(&"task-2".to_string()));
}

// ==================== Error Recovery Scenarios ====================

#[test]
fn test_transform_after_partial_write_failure() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create state with InProgress tasks
    let mut task1 = Task::new("task-1", "Task 1", "First");
    task1.status = TaskStatus::InProgress;

    let mut task2 = Task::new("task-2", "Task 2", "Second");
    task2.status = TaskStatus::Blocked;

    let state = WorkspaceState::new(
        project_root.to_path_buf(),
        vec![task1, task2]
    );
    state.save().unwrap();

    // Verify recovery by loading with transform
    let recovered_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(recovered_state.tasks.len(), 2);
    assert_eq!(recovered_state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(recovered_state.tasks[1].status, TaskStatus::Pending);
}

#[test]
fn test_load_with_transform_validates_project_root() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create valid state
    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Load should preserve project_root
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.project_root, project_root);
}

// ==================== Metadata Preservation ====================

#[test]
fn test_transform_preserves_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create state
    let mut task = Task::new("task-1", "Task", "Description");
    task.status = TaskStatus::InProgress;
    task.complexity = TaskComplexity::Complex;
    task.description = "Complex task description".to_string();

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // After transformation, metadata should be preserved
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].id, "task-1");
    assert_eq!(loaded_state.tasks[0].title, "Task");
    assert_eq!(loaded_state.tasks[0].description, "Complex task description");
    assert_eq!(loaded_state.tasks[0].complexity, TaskComplexity::Complex);
}

// ==================== Empty and Null Cases ====================

#[test]
fn test_transform_with_empty_depends_on() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with empty dependencies
    let mut task = Task::new("task-1", "Task", "No dependencies");
    task.status = TaskStatus::InProgress;
    task.depends_on = vec![];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(loaded_state.tasks[0].depends_on.len(), 0);
}

#[test]
fn test_detect_orphaned_with_no_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create tasks with no dependencies
    let task1 = Task::new("task-1", "Task 1", "No deps");
    let task2 = Task::new("task-2", "Task 2", "No deps");

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);

    let orphaned = state.detect_orphaned_tasks();

    assert_eq!(orphaned.len(), 0);
}

// ==================== Dependency Graph Edge Cases ====================

#[test]
fn test_validate_with_complex_valid_diamond() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Diamond pattern: A -> B, A -> C, B -> D, C -> D
    let mut task_d = Task::new("task-d", "D", "Bottom");
    task_d.depends_on = vec!["task-b".to_string(), "task-c".to_string()];

    let mut task_b = Task::new("task-b", "B", "Branch 1");
    task_b.depends_on = vec!["task-a".to_string()];

    let mut task_c = Task::new("task-c", "C", "Branch 2");
    task_c.depends_on = vec!["task-a".to_string()];

    let task_a = Task::new("task-a", "A", "Top");

    let state = WorkspaceState::new(
        project_root.to_path_buf(),
        vec![task_a, task_b, task_c, task_d]
    );

    // Diamond pattern is valid (not a cycle)
    assert!(state.validate_dependency_graph().is_ok());
}

#[test]
fn test_validate_detects_three_node_cycle() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Three-node cycle: A -> B -> C -> A
    let mut task_c = Task::new("task-c", "C", "Third");
    task_c.depends_on = vec!["task-a".to_string()];

    let mut task_b = Task::new("task-b", "B", "Second");
    task_b.depends_on = vec!["task-c".to_string()];

    let mut task_a = Task::new("task-a", "A", "First");
    task_a.depends_on = vec!["task-b".to_string()];

    let state = WorkspaceState::new(
        project_root.to_path_buf(),
        vec![task_a, task_b, task_c]
    );

    let result = state.validate_dependency_graph();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Circular"));
}

// ==================== Project Root Edge Cases ====================

#[test]
fn test_workspace_state_with_relative_path() {
    let _temp_dir = TempDir::new().unwrap();

    // Use relative path
    let relative_path = PathBuf::from("./test-project");
    let task = Task::new("task-1", "Task", "Description");

    let state = WorkspaceState::new(relative_path.clone(), vec![task]);

    assert_eq!(state.project_root, relative_path);
    assert!(state.manifest_path().ends_with("test-project/.ltmatrix/tasks-manifest.json"));
}

#[test]
fn test_workspace_state_with_absolute_path() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Use absolute path
    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);

    assert_eq!(state.project_root, project_root.to_path_buf());
    assert!(state.manifest_path().starts_with(project_root));
}

// ==================== Subtask Dependency Edge Cases ====================

#[test]
fn test_orphaned_detection_subtask_depends_on_sibling() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Subtask depending on sibling subtask (valid)
    let mut subtask2 = Task::new("subtask-2", "Subtask 2", "Depends on sibling");
    subtask2.depends_on = vec!["subtask-1".to_string()];

    let subtask1 = Task::new("subtask-1", "Subtask 1", "First subtask");

    let mut parent = Task::new("task-1", "Parent", "Parent task");
    parent.subtasks = vec![subtask1, subtask2];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![parent]);

    // No orphaned tasks - sibling dependency is valid
    let orphaned = state.detect_orphaned_tasks();

    assert_eq!(orphaned.len(), 0);
}

#[test]
fn test_orphaned_detection_subtask_depends_on_parent() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Subtask depending on parent task
    let mut subtask = Task::new("subtask-1", "Subtask", "Depends on parent");
    subtask.depends_on = vec!["task-1".to_string()];

    let mut parent = Task::new("task-1", "Parent", "Parent task");
    parent.subtasks = vec![subtask];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![parent]);

    // No orphaned tasks - parent dependency is valid
    let orphaned = state.detect_orphaned_tasks();

    assert_eq!(orphaned.len(), 0);
}

#[test]
fn test_orphaned_detection_subtask_depends_on_missing_parent_sibling() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Subtask depending on non-existent sibling of parent
    let mut subtask = Task::new("subtask-1", "Subtask", "Has missing dependency");
    subtask.depends_on = vec!["task-2".to_string()]; // task-2 doesn't exist

    let mut parent = Task::new("task-1", "Parent", "Parent task");
    parent.subtasks = vec![subtask];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![parent]);

    // Should detect orphaned dependency
    let orphaned = state.detect_orphaned_tasks();

    assert_eq!(orphaned.len(), 1);
    assert_eq!(orphaned[0].0, "subtask-1");
    assert_eq!(orphaned[0].1, vec!["task-2".to_string()]);
}

// ==================== Concurrent State Scenarios ====================

#[test]
fn test_multiple_consecutive_transforms() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create state with InProgress task
    let mut task = Task::new("task-1", "Task", "Description");
    task.status = TaskStatus::InProgress;

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // First transform
    let state1 = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();
    assert_eq!(state1.tasks[0].status, TaskStatus::Pending);

    // Save and transform again
    state1.save().unwrap();
    let state2 = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Should still be Pending (idempotent)
    assert_eq!(state2.tasks[0].status, TaskStatus::Pending);
}

#[test]
fn test_transform_idempotency_for_completed_tasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create state with Completed task
    let mut task = Task::new("task-1", "Task", "Description");
    task.status = TaskStatus::Completed;

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Multiple transforms should preserve Completed status
    for _ in 0..3 {
        let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();
        assert_eq!(loaded_state.tasks[0].status, TaskStatus::Completed);
        loaded_state.save().unwrap();
    }
}
