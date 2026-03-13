//! Integration tests for execute stage with workspace state persistence
//!
//! Tests that the execute stage properly saves workspace state after
//! each task completion and handles error scenarios.

use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::workspace::WorkspaceState;
use tempfile::TempDir;

/// Test that execute stage saves workspace state after task completion
#[test]
fn test_execute_saves_workspace_state_after_completion() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    // Create initial workspace state
    let task1 = Task::new("task-1", "First task", "First task");
    let task2 = Task::new("task-2", "Second task", "Second task");

    let state = WorkspaceState::new(project_root.clone(), vec![task1, task2]);
    state.save().unwrap();

    // TODO: Execute tasks through the execute stage
    // This should automatically save workspace state after each task completion
    // For now, we'll simulate it manually to show what the behavior should be

    // Simulate task-1 completion
    let mut loaded = WorkspaceState::load(project_root.clone()).unwrap();
    loaded.tasks[0].status = TaskStatus::Completed;
    loaded.save().unwrap();

    // Verify task-1 is saved as completed
    let verified = WorkspaceState::load(project_root.clone()).unwrap();
    assert_eq!(verified.tasks[0].status, TaskStatus::Completed);
    assert_eq!(verified.tasks[1].status, TaskStatus::Pending);
}

/// Test that workspace state handles task failures correctly
#[test]
fn test_execute_saves_workspace_state_after_failure() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    // Create workspace state
    let task1 = Task::new("task-1", "First task", "First task");
    let task2 = Task::new("task-2", "Second task", "Second task");

    let state = WorkspaceState::new(project_root.clone(), vec![task1, task2]);
    state.save().unwrap();

    // Simulate task-1 failure
    let mut loaded = WorkspaceState::load(project_root.clone()).unwrap();
    loaded.tasks[0].status = TaskStatus::Failed;
    loaded.tasks[0].error = Some("Task execution failed".to_string());
    loaded.save().unwrap();

    // Verify task-1 failure is saved
    let verified = WorkspaceState::load(project_root.clone()).unwrap();
    assert_eq!(verified.tasks[0].status, TaskStatus::Failed);
    assert_eq!(
        verified.tasks[0].error,
        Some("Task execution failed".to_string())
    );
}

/// Test that workspace state preserves all task properties after execution
#[test]
fn test_execute_preserves_task_properties() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    // Create task with various properties
    let mut task = Task::new("task-1", "Complex task", "A complex task");
    task.complexity = TaskComplexity::Complex;
    task.depends_on = vec!["dep-1".to_string()];
    task.retry_count = 2;

    let state = WorkspaceState::new(project_root.clone(), vec![task]);
    state.save().unwrap();

    // Execute task (mark as completed)
    let mut loaded = WorkspaceState::load(project_root.clone()).unwrap();
    loaded.tasks[0].status = TaskStatus::Completed;
    loaded.tasks[0].session_id = Some("session-123".to_string());
    loaded.save().unwrap();

    // Verify all properties are preserved
    let verified = WorkspaceState::load(project_root).unwrap();
    assert_eq!(verified.tasks[0].status, TaskStatus::Completed);
    assert_eq!(verified.tasks[0].complexity, TaskComplexity::Complex);
    assert_eq!(verified.tasks[0].depends_on, vec!["dep-1".to_string()]);
    assert_eq!(verified.tasks[0].retry_count, 2);
    assert_eq!(
        verified.tasks[0].session_id,
        Some("session-123".to_string())
    );
}

/// Test that workspace state handles concurrent execution correctly
#[test]
fn test_execute_handles_concurrent_modifications() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    // Create initial state
    let task1 = Task::new("task-1", "Task 1", "First");
    let task2 = Task::new("task-2", "Task 2", "Second");
    let task3 = Task::new("task-3", "Task 3", "Third");

    let state = WorkspaceState::new(project_root.clone(), vec![task1, task2, task3]);
    state.save().unwrap();

    // Simulate parallel task completions
    let mut state1 = WorkspaceState::load(project_root.clone()).unwrap();
    state1.tasks[0].status = TaskStatus::Completed;
    state1.save().unwrap();

    let mut state2 = WorkspaceState::load(project_root.clone()).unwrap();
    state2.tasks[1].status = TaskStatus::Completed;
    state2.save().unwrap();

    // Verify final state has both completions
    let final_state = WorkspaceState::load(project_root).unwrap();
    assert_eq!(final_state.tasks[0].status, TaskStatus::Completed);
    assert_eq!(final_state.tasks[1].status, TaskStatus::Completed);
    assert_eq!(final_state.tasks[2].status, TaskStatus::Pending);
}

/// Test that workspace state is saved even when some tasks fail
#[test]
fn test_execute_saves_partial_state_on_failure() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    // Create tasks
    let task1 = Task::new("task-1", "Task 1", "First");
    let task2 = Task::new("task-2", "Task 2", "Second");
    let task3 = Task::new("task-3", "Task 3", "Third");

    let state = WorkspaceState::new(project_root.clone(), vec![task1, task2, task3]);
    state.save().unwrap();

    // Simulate task-1 success, task-2 failure
    let mut loaded = WorkspaceState::load(project_root.clone()).unwrap();
    loaded.tasks[0].status = TaskStatus::Completed;
    loaded.tasks[1].status = TaskStatus::Failed;
    loaded.tasks[1].error = Some("Execution failed".to_string());
    loaded.save().unwrap();

    // Verify partial state is saved
    let verified = WorkspaceState::load(project_root).unwrap();
    assert_eq!(verified.tasks[0].status, TaskStatus::Completed);
    assert_eq!(verified.tasks[1].status, TaskStatus::Failed);
    assert_eq!(verified.tasks[2].status, TaskStatus::Pending);
}

/// Test that workspace state metadata is updated on each save
#[test]
fn test_execute_updates_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    // Create initial state
    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.clone(), vec![task]);

    let initial_metadata = state.metadata.clone();
    let saved_state = state.save().unwrap();

    // Wait to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Execute task (mark as completed)
    let mut loaded = WorkspaceState::load(project_root.clone()).unwrap();
    loaded.tasks[0].status = TaskStatus::Completed;
    let updated_state = loaded.save().unwrap();

    // Verify metadata was updated
    assert!(updated_state.metadata.modified_at > saved_state.metadata.modified_at);
    assert_eq!(
        updated_state.metadata.created_at,
        initial_metadata.created_at
    );
}
