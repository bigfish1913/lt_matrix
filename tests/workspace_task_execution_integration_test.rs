//! Workspace state integration with task execution lifecycle
//!
//! Comprehensive tests for workspace state persistence integrated with
//! the task execution pipeline, including timestamp handling, error recovery,
//! and state consistency verification.

use ltmatrix::workspace::WorkspaceState;
use ltmatrix::models::{Task, TaskStatus, TaskComplexity};
use std::fs;
use tempfile::TempDir;

// ==================== Timestamp Preservation Tests ====================

#[test]
fn test_transform_clears_started_at_for_in_progress() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with started_at timestamp
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::InProgress;
    task.started_at = Some(chrono::Utc::now());

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Load with transform - should clear started_at
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Pending);
    assert!(loaded_state.tasks[0].started_at.is_none(),
        "started_at should be cleared when resetting InProgress to Pending");
}

#[test]
fn test_transform_clears_started_at_for_blocked() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create blocked task with started_at timestamp
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::Blocked;
    task.started_at = Some(chrono::Utc::now());

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Load with transform - should clear started_at
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Pending);
    assert!(loaded_state.tasks[0].started_at.is_none(),
        "started_at should be cleared when resetting Blocked to Pending");
}

#[test]
fn test_transform_preserves_completed_at_for_completed() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create completed task with timestamps
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::Completed;
    task.started_at = Some(chrono::Utc::now());
    task.completed_at = Some(chrono::Utc::now());

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Load with transform - should preserve completed task
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Completed);
    assert!(loaded_state.tasks[0].started_at.is_some(),
        "started_at should be preserved for Completed tasks");
    assert!(loaded_state.tasks[0].completed_at.is_some(),
        "completed_at should be preserved for Completed tasks");
}

#[test]
fn test_transform_preserves_error_for_failed() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create failed task with error message
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::Failed;
    task.error = Some("Something went wrong".to_string());

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Load with transform - should preserve failed state and error
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Failed);
    assert_eq!(loaded_state.tasks[0].error, Some("Something went wrong".to_string()),
        "error should be preserved for Failed tasks");
}

#[test]
fn test_transform_preserves_retry_count() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with retry count
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::InProgress;
    task.retry_count = 2;

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Load with transform - should preserve retry count
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(loaded_state.tasks[0].retry_count, 2,
        "retry_count should be preserved through transformation");
}

#[test]
fn test_transform_preserves_session_id() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with session ID
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::InProgress;
    task.session_id = Some("session-123".to_string());

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Load with transform - should preserve session ID
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(loaded_state.tasks[0].session_id, Some("session-123".to_string()),
        "session_id should be preserved through transformation");
}

#[test]
fn test_transform_preserves_parent_session_id() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with parent session ID
    let mut task = Task::new("task-1", "Test Task", "Description");
    task.status = TaskStatus::Blocked;
    task.parent_session_id = Some("parent-session-456".to_string());

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Load with transform - should preserve parent session ID
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(loaded_state.tasks[0].parent_session_id, Some("parent-session-456".to_string()),
        "parent_session_id should be preserved through transformation");
}

#[test]
fn test_transform_preserves_complexity() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create tasks with different complexities
    let mut task1 = Task::new("task-1", "Simple", "Description");
    task1.status = TaskStatus::InProgress;
    task1.complexity = TaskComplexity::Simple;

    let mut task2 = Task::new("task-2", "Moderate", "Description");
    task2.status = TaskStatus::Blocked;
    task2.complexity = TaskComplexity::Moderate;

    let mut task3 = Task::new("task-3", "Complex", "Description");
    task3.status = TaskStatus::Completed;
    task3.complexity = TaskComplexity::Complex;

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2, task3]);
    state.save().unwrap();

    // Load with transform - should preserve all complexities
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded_state.tasks[0].complexity, TaskComplexity::Simple);
    assert_eq!(loaded_state.tasks[1].complexity, TaskComplexity::Moderate);
    assert_eq!(loaded_state.tasks[2].complexity, TaskComplexity::Complex);
}

// ==================== Nested Subtask Tests ====================

#[test]
fn test_transform_deeply_nested_subtasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create deeply nested subtasks (max depth: 3)
    let mut level3 = Task::new("task-3", "Level 3", "Deepest subtask");
    level3.status = TaskStatus::InProgress;
    level3.started_at = Some(chrono::Utc::now());

    let mut level2 = Task::new("task-2", "Level 2", "Middle subtask");
    level2.status = TaskStatus::Blocked;
    level2.started_at = Some(chrono::Utc::now());
    level2.subtasks = vec![level3];

    let mut level1 = Task::new("task-1", "Level 1", "Top level task");
    level1.status = TaskStatus::InProgress;
    level1.started_at = Some(chrono::Utc::now());
    level1.subtasks = vec![level2];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![level1]);
    state.save().unwrap();

    // Load with transform - should reset all levels
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Check level 1
    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Pending);
    assert!(loaded_state.tasks[0].started_at.is_none());

    // Check level 2
    assert_eq!(loaded_state.tasks[0].subtasks[0].status, TaskStatus::Pending);
    assert!(loaded_state.tasks[0].subtasks[0].started_at.is_none());

    // Check level 3
    assert_eq!(loaded_state.tasks[0].subtasks[0].subtasks[0].status, TaskStatus::Pending);
    assert!(loaded_state.tasks[0].subtasks[0].subtasks[0].started_at.is_none());
}

#[test]
fn test_transform_mixed_status_subtasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create parent with mixed status subtasks
    let mut subtask1 = Task::new("task-2", "Completed Subtask", "Done");
    subtask1.status = TaskStatus::Completed;

    let mut subtask2 = Task::new("task-3", "InProgress Subtask", "Working");
    subtask2.status = TaskStatus::InProgress;
    subtask2.started_at = Some(chrono::Utc::now());

    let mut parent = Task::new("task-1", "Parent", "Parent task");
    parent.status = TaskStatus::Pending;
    parent.subtasks = vec![subtask1, subtask2];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![parent]);
    state.save().unwrap();

    // Load with transform
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Parent should remain Pending (not reset)
    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Pending);

    // First subtask should remain Completed (not reset)
    assert_eq!(loaded_state.tasks[0].subtasks[0].status, TaskStatus::Completed);

    // Second subtask should be reset from InProgress to Pending
    assert_eq!(loaded_state.tasks[0].subtasks[1].status, TaskStatus::Pending);
    assert!(loaded_state.tasks[0].subtasks[1].started_at.is_none());
}

// ==================== State Consistency Tests ====================

#[test]
fn test_state_consistency_after_concurrent_modifications() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create initial state
    let task1 = Task::new("task-1", "Task 1", "Description 1");
    let task2 = Task::new("task-2", "Task 2", "Description 2");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);
    state.save().unwrap();

    // Simulate first modification
    let mut state1 = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    state1.tasks[0].status = TaskStatus::Completed;
    state1.save().unwrap();

    // Simulate second modification (overwrites first)
    let mut state2 = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    state2.tasks[1].status = TaskStatus::InProgress;
    state2.save().unwrap();

    // Verify final state has both modifications
    let final_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    assert_eq!(final_state.tasks[0].status, TaskStatus::Completed);
    assert_eq!(final_state.tasks[1].status, TaskStatus::InProgress);
}

#[test]
fn test_state_consistency_with_dependency_chains() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create dependency chain: task1 -> task2 -> task3
    let mut task1 = Task::new("task-1", "First", "First in chain");
    task1.status = TaskStatus::Completed;

    let mut task2 = Task::new("task-2", "Second", "Depends on first");
    task2.status = TaskStatus::InProgress;
    task2.depends_on = vec!["task-1".to_string()];
    task2.started_at = Some(chrono::Utc::now());

    let mut task3 = Task::new("task-3", "Third", "Depends on second");
    task3.status = TaskStatus::Pending;
    task3.depends_on = vec!["task-2".to_string()];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2, task3]);
    state.save().unwrap();

    // Load with transform
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Verify dependency chain preserved
    assert_eq!(loaded_state.tasks[0].status, TaskStatus::Completed);
    assert_eq!(loaded_state.tasks[1].status, TaskStatus::Pending); // Reset from InProgress
    assert_eq!(loaded_state.tasks[1].depends_on, vec!["task-1".to_string()]);
    assert!(loaded_state.tasks[1].started_at.is_none());
    assert_eq!(loaded_state.tasks[2].status, TaskStatus::Pending);
    assert_eq!(loaded_state.tasks[2].depends_on, vec!["task-2".to_string()]);
}

#[test]
fn test_state_consistency_with_metadata_updates() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);

    // First save
    let state1 = state.save().unwrap();
    let created_at = state1.metadata.created_at;
    let modified1 = state1.metadata.modified_at;

    // Wait to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Modify and save again
    let mut state2 = state1.clone();
    state2.tasks[0].status = TaskStatus::Completed;
    let state3 = state2.save().unwrap();
    let modified2 = state3.metadata.modified_at;

    // Verify metadata consistency
    assert_eq!(state3.metadata.created_at, created_at,
        "created_at should remain constant");
    assert_eq!(state3.metadata.modified_at, modified2,
        "modified_at should be updated");
    assert!(modified2 > modified1,
        "modified_at should increase over time");
    assert_eq!(state3.metadata.version, "1.0",
        "version should be preserved");
}

// ==================== Error Recovery Tests ====================

#[test]
fn test_error_recovery_partial_write() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create initial valid state
    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Simulate partial write by truncating the file
    let manifest_path = project_root.join(".ltmatrix").join("tasks-manifest.json");
    let mut content = fs::read_to_string(&manifest_path).unwrap();
    content.truncate(content.len() / 2);
    fs::write(&manifest_path, content).unwrap();

    // Attempt to load - should fail gracefully
    let result = WorkspaceState::load_with_transform(project_root.to_path_buf());
    assert!(result.is_err(),
        "Should fail gracefully when loading truncated JSON");

    // Verify error message is meaningful
    if let Err(e) = result {
        let error_msg = e.to_string().to_lowercase();
        assert!(error_msg.contains("json") || error_msg.contains("parse") || error_msg.contains("invalid"),
            "Error message should indicate JSON parsing problem");
    }
}

#[test]
fn test_error_recovery_wrong_file_format() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create directory but write wrong file format
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let manifest_path = ltmatrix_dir.join("tasks-manifest.json");
    fs::write(&manifest_path, "This is not JSON at all").unwrap();

    // Attempt to load - should fail gracefully
    let result = WorkspaceState::load_with_transform(project_root.to_path_buf());
    assert!(result.is_err());

    if let Err(e) = result {
        let error_msg = e.to_string().to_lowercase();
        assert!(error_msg.contains("json") || error_msg.contains("parse"),
            "Error should indicate JSON parsing problem");
    }
}

#[test]
fn test_error_recovery_extra_fields() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create JSON with extra unknown fields (should still deserialize)
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let manifest_path = ltmatrix_dir.join("tasks-manifest.json");
    let json_with_extra = r#"{
        "project_root": "/test",
        "tasks": [],
        "metadata": {
            "version": "1.0",
            "created_at": "2024-01-01T00:00:00Z",
            "modified_at": "2024-01-01T00:00:00Z"
        },
        "unknown_field": "should be ignored",
        "extra_data": 123
    }"#;

    fs::write(&manifest_path, json_with_extra).unwrap();

    // Should load successfully, ignoring extra fields
    let result = WorkspaceState::load_with_transform(project_root.to_path_buf());
    assert!(result.is_ok(),
        "Should successfully load JSON with extra fields");
}

// ==================== Execute Stage Integration Tests ====================

#[test]
fn test_execute_stage_save_after_each_task() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create multiple tasks
    let task1 = Task::new("task-1", "First", "First task");
    let task2 = Task::new("task-2", "Second", "Second task");
    let task3 = Task::new("task-3", "Third", "Third task");

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2, task3]);
    state.save().unwrap();

    // Simulate executing tasks one at a time, saving after each
    for i in 0..3 {
        let mut loaded = WorkspaceState::load(project_root.to_path_buf()).unwrap();
        loaded.tasks[i].status = TaskStatus::Completed;
        loaded.save().unwrap();

        // Verify the save persisted
        let verified = WorkspaceState::load(project_root.to_path_buf()).unwrap();
        for j in 0..=i {
            assert_eq!(verified.tasks[j].status, TaskStatus::Completed,
                "Task {} should be marked as completed after save", j + 1);
        }
    }

    // Final state should have all tasks completed
    let final_state = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    assert!(final_state.tasks.iter().all(|t| t.status == TaskStatus::Completed));
}

#[test]
fn test_execute_stage_with_failed_tasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create tasks that will fail
    let mut task1 = Task::new("task-1", "First", "Will succeed");
    task1.status = TaskStatus::Completed;

    let mut task2 = Task::new("task-2", "Second", "Will fail");
    task2.status = TaskStatus::Failed;
    task2.error = Some("Task failed with error".to_string());

    let mut task3 = Task::new("task-3", "Third", "Will be blocked");
    task3.status = TaskStatus::InProgress;
    task3.started_at = Some(chrono::Utc::now());

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2, task3]);
    state.save().unwrap();

    // Load with transform should only reset InProgress/Blocked
    let loaded = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    assert_eq!(loaded.tasks[0].status, TaskStatus::Completed,
        "Completed task should remain completed");
    assert_eq!(loaded.tasks[1].status, TaskStatus::Failed,
        "Failed task should remain failed");
    assert_eq!(loaded.tasks[1].error, Some("Task failed with error".to_string()),
        "Error message should be preserved");
    assert_eq!(loaded.tasks[2].status, TaskStatus::Pending,
        "InProgress task should be reset to Pending");
    assert!(loaded.tasks[2].started_at.is_none(),
        "started_at should be cleared");
}

#[test]
fn test_execute_stage_preserves_task_order() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create tasks with specific order
    let task1 = Task::new("task-1", "First", "First");
    let task2 = Task::new("task-2", "Second", "Second");
    let task3 = Task::new("task-3", "Third", "Third");
    let task4 = Task::new("task-4", "Fourth", "Fourth");

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2, task3, task4]);
    state.save().unwrap();

    // Verify order is preserved after load
    let loaded = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    assert_eq!(loaded.tasks[0].id, "task-1");
    assert_eq!(loaded.tasks[1].id, "task-2");
    assert_eq!(loaded.tasks[2].id, "task-3");
    assert_eq!(loaded.tasks[3].id, "task-4");

    // Modify and save
    let mut modified = loaded;
    modified.tasks[1].status = TaskStatus::Completed;
    modified.save().unwrap();

    // Verify order still preserved after modification
    let reloaded = WorkspaceState::load(project_root.to_path_buf()).unwrap();
    assert_eq!(reloaded.tasks[0].id, "task-1");
    assert_eq!(reloaded.tasks[1].id, "task-2");
    assert_eq!(reloaded.tasks[2].id, "task-3");
    assert_eq!(reloaded.tasks[3].id, "task-4");
}
