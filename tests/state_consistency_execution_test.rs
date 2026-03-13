//! State consistency tests across task execution lifecycle
//!
//! Comprehensive integration tests verifying workspace state integrity
//! through normal execution, interrupted execution, and recovery scenarios.
//!
//! Test Categories:
//! - Normal Execution: Complete task lifecycles without interruptions
//! - Interrupted Execution: Simulated crashes, timeouts, and failures
//! - Recovery Scenarios: State restoration and continuation after interruption
//! - State Integrity: Data consistency, validation, and verification

use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::workspace::WorkspaceState;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

// ==================== Helper Functions ====================

/// Creates a temporary project directory with workspace state
fn setup_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();
    (temp_dir, project_root)
}

/// Creates a sample task with specified status
fn create_task(id: &str, status: TaskStatus) -> Task {
    let mut task = Task::new(
        id,
        format!("Task {}", id),
        format!("Description for {}", id),
    );
    task.status = status;
    task
}

/// Creates a dependency chain of tasks
fn create_dependency_chain(count: usize) -> Vec<Task> {
    let mut tasks = Vec::new();
    for i in 0..count {
        let mut task = create_task(&format!("task-{}", i + 1), TaskStatus::Pending);
        if i > 0 {
            task.depends_on = vec![format!("task-{}", i)];
        }
        tasks.push(task);
    }
    tasks
}

/// Verifies all task properties are preserved
fn verify_task_properties(original: &Task, loaded: &Task) {
    assert_eq!(original.id, loaded.id, "Task ID should match");
    assert_eq!(original.title, loaded.title, "Task title should match");
    assert_eq!(
        original.description, loaded.description,
        "Task description should match"
    );
    assert_eq!(
        original.complexity, loaded.complexity,
        "Task complexity should match"
    );
    assert_eq!(
        original.depends_on, loaded.depends_on,
        "Task dependencies should match"
    );
    assert_eq!(
        original.retry_count, loaded.retry_count,
        "Retry count should match"
    );
    assert_eq!(
        original.session_id, loaded.session_id,
        "Session ID should match"
    );
    assert_eq!(
        original.parent_session_id, loaded.parent_session_id,
        "Parent session ID should match"
    );
}

// ==================== Normal Execution Scenarios ====================

#[test]
fn test_normal_execution_single_task() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create initial task
    let task = create_task("task-1", TaskStatus::Pending);
    let state = WorkspaceState::new(project_root.clone(), vec![task]);
    state.save().unwrap();

    // Simulate normal execution: Pending -> InProgress -> Completed
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::InProgress;
    state.tasks[0].started_at = Some(chrono::Utc::now());
    state.save().unwrap();

    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::Completed;
    state.tasks[0].completed_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Verify final state
    let final_state = WorkspaceState::load(project_root).unwrap();
    assert_eq!(final_state.tasks[0].status, TaskStatus::Completed);
    assert!(final_state.tasks[0].started_at.is_some());
    assert!(final_state.tasks[0].completed_at.is_some());
}

#[test]
fn test_normal_execution_multiple_tasks_sequential() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create multiple independent tasks
    let tasks = vec![
        create_task("task-1", TaskStatus::Pending),
        create_task("task-2", TaskStatus::Pending),
        create_task("task-3", TaskStatus::Pending),
    ];

    let state = WorkspaceState::new(project_root.clone(), tasks);
    state.save().unwrap();

    // Execute tasks sequentially
    for i in 0..3 {
        let mut state = WorkspaceState::load(project_root.clone()).unwrap();

        // Mark as InProgress
        state.tasks[i].status = TaskStatus::InProgress;
        state.tasks[i].started_at = Some(chrono::Utc::now());
        state.save().unwrap();

        // Mark as Completed
        let mut state = WorkspaceState::load(project_root.clone()).unwrap();
        state.tasks[i].status = TaskStatus::Completed;
        state.tasks[i].completed_at = Some(chrono::Utc::now());
        state.save().unwrap();

        // Verify all previously completed tasks remain completed
        let verified = WorkspaceState::load(project_root.clone()).unwrap();
        for j in 0..=i {
            assert_eq!(verified.tasks[j].status, TaskStatus::Completed);
        }
    }

    // Final verification
    let final_state = WorkspaceState::load(project_root).unwrap();
    assert!(final_state
        .tasks
        .iter()
        .all(|t| t.status == TaskStatus::Completed));
}

#[test]
fn test_normal_execution_with_dependencies() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create dependency chain: task-1 -> task-2 -> task-3
    let tasks = create_dependency_chain(3);
    let state = WorkspaceState::new(project_root.clone(), tasks);
    state.save().unwrap();

    // Execute in dependency order
    // task-1 (no dependencies)
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::Completed;
    state.tasks[0].started_at = Some(chrono::Utc::now());
    state.tasks[0].completed_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // task-2 (depends on task-1)
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[1].status = TaskStatus::Completed;
    state.tasks[1].started_at = Some(chrono::Utc::now());
    state.tasks[1].completed_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // task-3 (depends on task-2)
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[2].status = TaskStatus::Completed;
    state.tasks[2].started_at = Some(chrono::Utc::now());
    state.tasks[2].completed_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Verify dependency chain integrity
    let final_state = WorkspaceState::load(project_root).unwrap();
    assert_eq!(final_state.tasks[0].depends_on.len(), 0);
    assert_eq!(final_state.tasks[1].depends_on, vec!["task-1"]);
    assert_eq!(final_state.tasks[2].depends_on, vec!["task-2"]);
}

#[test]
fn test_normal_execution_with_retry() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create task that will be retried
    let task = create_task("task-1", TaskStatus::Pending);
    let state = WorkspaceState::new(project_root.clone(), vec![task]);
    state.save().unwrap();

    // First attempt: Pending -> InProgress -> Failed
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::InProgress;
    state.tasks[0].started_at = Some(chrono::Utc::now());
    state.save().unwrap();

    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::Failed;
    state.tasks[0].error = Some("First attempt failed".to_string());
    state.tasks[0].retry_count = 1;
    state.save().unwrap();

    // Second attempt: Failed -> InProgress -> Completed
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::InProgress;
    state.tasks[0].started_at = Some(chrono::Utc::now());
    state.save().unwrap();

    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::Completed;
    state.tasks[0].completed_at = Some(chrono::Utc::now());
    state.tasks[0].error = None;
    state.tasks[0].retry_count = 1;
    state.save().unwrap();

    // Verify retry state preserved
    let final_state = WorkspaceState::load(project_root).unwrap();
    assert_eq!(final_state.tasks[0].status, TaskStatus::Completed);
    assert_eq!(final_state.tasks[0].retry_count, 1);
    assert!(final_state.tasks[0].error.is_none());
}

#[test]
fn test_normal_execution_preserves_session_id() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create task with session ID
    let mut task = create_task("task-1", TaskStatus::Pending);
    task.session_id = Some("session-abc123".to_string());
    let state = WorkspaceState::new(project_root.clone(), vec![task]);
    state.save().unwrap();

    // Execute task
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::InProgress;
    state.tasks[0].started_at = Some(chrono::Utc::now());
    state.save().unwrap();

    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::Completed;
    state.tasks[0].completed_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Verify session ID preserved through execution
    let final_state = WorkspaceState::load(project_root).unwrap();
    assert_eq!(
        final_state.tasks[0].session_id,
        Some("session-abc123".to_string())
    );
}

// ==================== Interrupted Execution Scenarios ====================

#[test]
fn test_interrupted_execution_during_in_progress() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create task
    let task = create_task("task-1", TaskStatus::Pending);
    let state = WorkspaceState::new(project_root.clone(), vec![task]);
    state.save().unwrap();

    // Start execution
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::InProgress;
    state.tasks[0].started_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Simulate interruption: task is InProgress when saved

    // Load with transform should reset to Pending
    let recovered_state = WorkspaceState::load_with_transform(project_root.clone()).unwrap();
    assert_eq!(recovered_state.tasks[0].status, TaskStatus::Pending);
    assert!(
        recovered_state.tasks[0].started_at.is_none(),
        "started_at should be cleared on recovery"
    );
}

#[test]
fn test_interrupted_execution_multiple_tasks_partial_completion() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create multiple tasks
    let tasks = vec![
        create_task("task-1", TaskStatus::Pending),
        create_task("task-2", TaskStatus::Pending),
        create_task("task-3", TaskStatus::Pending),
        create_task("task-4", TaskStatus::Pending),
    ];

    let state = WorkspaceState::new(project_root.clone(), tasks);
    state.save().unwrap();

    // Complete task-1
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::Completed;
    state.tasks[0].started_at = Some(chrono::Utc::now());
    state.tasks[0].completed_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Complete task-2
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[1].status = TaskStatus::Completed;
    state.tasks[1].started_at = Some(chrono::Utc::now());
    state.tasks[1].completed_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Start task-3 (interrupted here)
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[2].status = TaskStatus::InProgress;
    state.tasks[2].started_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Simulate interruption and recovery
    let recovered_state = WorkspaceState::load_with_transform(project_root.clone()).unwrap();

    // Verify: task-1, task-2 completed; task-3 reset to pending; task-4 still pending
    assert_eq!(recovered_state.tasks[0].status, TaskStatus::Completed);
    assert_eq!(recovered_state.tasks[1].status, TaskStatus::Completed);
    assert_eq!(recovered_state.tasks[2].status, TaskStatus::Pending);
    assert!(recovered_state.tasks[2].started_at.is_none());
    assert_eq!(recovered_state.tasks[3].status, TaskStatus::Pending);
}

#[test]
fn test_interrupted_execution_with_blocked_task() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create tasks where one gets blocked
    let tasks = vec![
        create_task("task-1", TaskStatus::Pending),
        create_task("task-2", TaskStatus::Pending),
    ];

    let state = WorkspaceState::new(project_root.clone(), tasks);
    state.save().unwrap();

    // Complete task-1
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::Completed;
    state.save().unwrap();

    // Mark task-2 as blocked (waiting for external resource)
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[1].status = TaskStatus::Blocked;
    state.tasks[1].started_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Simulate interruption
    let recovered_state = WorkspaceState::load_with_transform(project_root).unwrap();

    // Blocked task should be reset to Pending
    assert_eq!(recovered_state.tasks[0].status, TaskStatus::Completed);
    assert_eq!(recovered_state.tasks[1].status, TaskStatus::Pending);
    assert!(recovered_state.tasks[1].started_at.is_none());
}

#[test]
fn test_interrupted_execution_during_retry() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create task
    let task = create_task("task-1", TaskStatus::Pending);
    let state = WorkspaceState::new(project_root.clone(), vec![task]);
    state.save().unwrap();

    // First attempt fails
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::InProgress;
    state.tasks[0].started_at = Some(chrono::Utc::now());
    state.save().unwrap();

    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::Failed;
    state.tasks[0].error = Some("Attempt 1 failed".to_string());
    state.tasks[0].retry_count = 1;
    state.save().unwrap();

    // Second attempt started but interrupted
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::InProgress;
    state.tasks[0].started_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Recovery should preserve retry count but reset status
    let recovered_state = WorkspaceState::load_with_transform(project_root).unwrap();
    assert_eq!(recovered_state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(
        recovered_state.tasks[0].retry_count, 1,
        "Retry count should be preserved during recovery"
    );
    assert!(recovered_state.tasks[0].started_at.is_none());
}

#[test]
fn test_interrupted_execution_with_nested_subtasks() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create parent task with subtasks
    let mut subtask1 = create_task("subtask-1", TaskStatus::Pending);
    subtask1.status = TaskStatus::Completed;

    let mut subtask2 = create_task("subtask-2", TaskStatus::Pending);
    subtask2.status = TaskStatus::InProgress;
    subtask2.started_at = Some(chrono::Utc::now());

    let mut parent = create_task("parent-1", TaskStatus::Pending);
    parent.subtasks = vec![subtask1, subtask2];

    let state = WorkspaceState::new(project_root.clone(), vec![parent]);
    state.save().unwrap();

    // Recovery should transform nested subtasks
    let recovered_state = WorkspaceState::load_with_transform(project_root).unwrap();

    assert_eq!(recovered_state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(
        recovered_state.tasks[0].subtasks[0].status,
        TaskStatus::Completed
    );
    assert_eq!(
        recovered_state.tasks[0].subtasks[1].status,
        TaskStatus::Pending
    );
    assert!(recovered_state.tasks[0].subtasks[1].started_at.is_none());
}

// ==================== Recovery Scenarios ====================

#[test]
fn test_recovery_after_interruption_continues_from_completed() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create and partially complete tasks
    let tasks = vec![
        create_task("task-1", TaskStatus::Pending),
        create_task("task-2", TaskStatus::Pending),
        create_task("task-3", TaskStatus::Pending),
    ];

    let state = WorkspaceState::new(project_root.clone(), tasks);
    state.save().unwrap();

    // Complete task-1
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::Completed;
    state.tasks[0].started_at = Some(chrono::Utc::now());
    state.tasks[0].completed_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Start task-2 (interrupted)
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[1].status = TaskStatus::InProgress;
    state.tasks[1].started_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Recovery: load with transform
    let recovered_state = WorkspaceState::load_with_transform(project_root.clone()).unwrap();

    // Resume execution: complete task-2 and task-3
    let mut state = recovered_state;
    state.tasks[1].status = TaskStatus::Completed;
    state.tasks[1].started_at = Some(chrono::Utc::now());
    state.tasks[1].completed_at = Some(chrono::Utc::now());
    state.save().unwrap();

    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[2].status = TaskStatus::Completed;
    state.tasks[2].started_at = Some(chrono::Utc::now());
    state.tasks[2].completed_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Verify all tasks completed
    let final_state = WorkspaceState::load(project_root).unwrap();
    assert!(final_state
        .tasks
        .iter()
        .all(|t| t.status == TaskStatus::Completed));
}

#[test]
fn test_recovery_preserves_failed_task_errors() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create task that fails
    let task = create_task("task-1", TaskStatus::Pending);
    let state = WorkspaceState::new(project_root.clone(), vec![task]);
    state.save().unwrap();

    // Execute and fail
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::Failed;
    state.tasks[0].error = Some("Critical error: cannot proceed".to_string());
    state.save().unwrap();

    // Recovery should preserve failed status and error
    let recovered_state = WorkspaceState::load_with_transform(project_root).unwrap();
    assert_eq!(recovered_state.tasks[0].status, TaskStatus::Failed);
    assert_eq!(
        recovered_state.tasks[0].error,
        Some("Critical error: cannot proceed".to_string())
    );
}

#[test]
fn test_recovery_with_dependency_chain() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create dependency chain
    let tasks = create_dependency_chain(4);
    let state = WorkspaceState::new(project_root.clone(), tasks);
    state.save().unwrap();

    // Complete task-1 and task-2
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::Completed;
    state.tasks[0].started_at = Some(chrono::Utc::now());
    state.tasks[0].completed_at = Some(chrono::Utc::now());
    state.save().unwrap();

    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[1].status = TaskStatus::Completed;
    state.tasks[1].started_at = Some(chrono::Utc::now());
    state.tasks[1].completed_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Start task-3 (interrupted)
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[2].status = TaskStatus::InProgress;
    state.tasks[2].started_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Recovery
    let recovered_state = WorkspaceState::load_with_transform(project_root.clone()).unwrap();

    // Verify dependency chain intact and status correct
    assert_eq!(recovered_state.tasks[0].status, TaskStatus::Completed);
    assert_eq!(recovered_state.tasks[1].status, TaskStatus::Completed);
    assert_eq!(recovered_state.tasks[2].status, TaskStatus::Pending);
    assert_eq!(recovered_state.tasks[3].status, TaskStatus::Pending);

    // Verify dependencies preserved
    assert_eq!(recovered_state.tasks[1].depends_on, vec!["task-1"]);
    assert_eq!(recovered_state.tasks[2].depends_on, vec!["task-2"]);
    assert_eq!(recovered_state.tasks[3].depends_on, vec!["task-3"]);
}

#[test]
fn test_recovery_multiple_interruptions() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create tasks
    let tasks = vec![
        create_task("task-1", TaskStatus::Pending),
        create_task("task-2", TaskStatus::Pending),
    ];

    let state = WorkspaceState::new(project_root.clone(), tasks);
    state.save().unwrap();

    // First interruption: task-1 InProgress
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::InProgress;
    state.tasks[0].started_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Recovery #1
    let _state = WorkspaceState::load_with_transform(project_root.clone()).unwrap();

    // Complete task-1
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::Completed;
    state.tasks[0].started_at = Some(chrono::Utc::now());
    state.tasks[0].completed_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Second interruption: task-2 InProgress
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[1].status = TaskStatus::InProgress;
    state.tasks[1].started_at = Some(chrono::Utc::now());
    state.save().unwrap();

    // Recovery #2
    let recovered_state = WorkspaceState::load_with_transform(project_root.clone()).unwrap();

    // Verify task-1 still completed, task-2 reset
    assert_eq!(recovered_state.tasks[0].status, TaskStatus::Completed);
    assert_eq!(recovered_state.tasks[1].status, TaskStatus::Pending);
}

// ==================== State Integrity Verification ====================

#[test]
fn test_state_integrity_all_fields_preserved() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create task with all fields set
    let mut task = Task::new("task-1", "Test Task", "Test Description");
    task.status = TaskStatus::Pending;
    task.complexity = TaskComplexity::Complex;
    task.depends_on = vec!["dep-1".to_string(), "dep-2".to_string()];
    task.retry_count = 2;
    task.session_id = Some("session-123".to_string());
    task.parent_session_id = Some("parent-session-456".to_string());
    task.error = Some("Previous error".to_string());

    let state = WorkspaceState::new(project_root.clone(), vec![task]);
    state.save().unwrap();

    // Load and verify all fields
    let loaded_state = WorkspaceState::load(project_root).unwrap();
    verify_task_properties(&state.tasks[0], &loaded_state.tasks[0]);
}

#[test]
fn test_state_integrity_metadata_updates() {
    let (_temp_dir, project_root) = setup_test_workspace();

    let task = create_task("task-1", TaskStatus::Pending);
    let state = WorkspaceState::new(project_root.clone(), vec![task]);

    // First save
    let state1 = state.save().unwrap();
    let created_at = state1.metadata.created_at;
    let modified1 = state1.metadata.modified_at;

    // Wait to ensure timestamp difference
    std::thread::sleep(Duration::from_millis(10));

    // Modify and save again
    let mut state2 = state1;
    state2.tasks[0].status = TaskStatus::Completed;
    let state3 = state2.save().unwrap();
    let modified2 = state3.metadata.modified_at;

    // Verify metadata integrity
    assert_eq!(state3.metadata.created_at, created_at);
    assert!(modified2 > modified1);
    assert_eq!(state3.metadata.version, "1.0");
}

#[test]
fn test_state_integrity_concurrent_safety() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create initial state
    let tasks = vec![
        create_task("task-1", TaskStatus::Pending),
        create_task("task-2", TaskStatus::Pending),
    ];

    let state = WorkspaceState::new(project_root.clone(), tasks);
    state.save().unwrap();

    // Simulate concurrent modifications
    let mut state1 = WorkspaceState::load(project_root.clone()).unwrap();
    state1.tasks[0].status = TaskStatus::Completed;
    state1.save().unwrap();

    let mut state2 = WorkspaceState::load(project_root.clone()).unwrap();
    state2.tasks[1].status = TaskStatus::InProgress;
    state2.save().unwrap();

    // Final state should have last write wins for task-2
    let final_state = WorkspaceState::load(project_root).unwrap();
    assert_eq!(final_state.tasks[0].status, TaskStatus::Completed);
    assert_eq!(final_state.tasks[1].status, TaskStatus::InProgress);
}

#[test]
fn test_state_integrity_after_multiple_load_save_cycles() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create task
    let task = create_task("task-1", TaskStatus::Pending);
    let state = WorkspaceState::new(project_root.clone(), vec![task]);

    // Perform multiple load/save cycles
    let mut current_state = state;
    for i in 0..10 {
        current_state = current_state.save().unwrap();
        current_state = WorkspaceState::load(project_root.clone()).unwrap();

        // Verify state consistency
        assert_eq!(current_state.tasks.len(), 1);
        assert_eq!(current_state.tasks[0].id, "task-1");

        // Update status
        if i % 2 == 0 {
            current_state.tasks[0].status = TaskStatus::InProgress;
        } else {
            current_state.tasks[0].status = TaskStatus::Pending;
        }
    }

    // Final verification
    let final_state = WorkspaceState::load(project_root).unwrap();
    assert_eq!(final_state.tasks[0].id, "task-1");
}

#[test]
fn test_state_integrity_empty_task_list() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create state with no tasks
    let state = WorkspaceState::new(project_root.clone(), vec![]);
    state.save().unwrap();

    // Load and verify
    let loaded_state = WorkspaceState::load(project_root.clone()).unwrap();
    assert_eq!(loaded_state.tasks.len(), 0);

    // Load with transform should also handle empty list
    let transformed_state = WorkspaceState::load_with_transform(project_root).unwrap();
    assert_eq!(transformed_state.tasks.len(), 0);
}

#[test]
fn test_state_integrity_large_number_of_tasks() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create large number of tasks
    let tasks: Vec<Task> = (0..100)
        .map(|i| create_task(&format!("task-{}", i), TaskStatus::Pending))
        .collect();

    let state = WorkspaceState::new(project_root.clone(), tasks);
    state.save().unwrap();

    // Load and verify all tasks present
    let loaded_state = WorkspaceState::load(project_root.clone()).unwrap();
    assert_eq!(loaded_state.tasks.len(), 100);

    // Verify task IDs preserved
    for (i, task) in loaded_state.tasks.iter().enumerate() {
        assert_eq!(task.id, format!("task-{}", i));
    }

    // Complete half the tasks
    let mut state = loaded_state;
    for i in 0..50 {
        state.tasks[i].status = TaskStatus::Completed;
    }
    state.save().unwrap();

    // Verify partial completion
    let final_state = WorkspaceState::load(project_root).unwrap();
    assert_eq!(final_state.tasks.len(), 100);
    assert_eq!(
        final_state.tasks[0..50]
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count(),
        50
    );
    assert_eq!(
        final_state.tasks[50..100]
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .count(),
        50
    );
}

#[test]
fn test_state_integrity_unicode_content() {
    let (_temp_dir, project_root) = setup_test_workspace();

    // Create task with unicode content
    let mut task = Task::new(
        "task-unicode-测试",
        "Tâche avec çäräçtërës spécïäux",
        "描述包含中文、العربية、עברית",
    );
    task.status = TaskStatus::Pending;

    let state = WorkspaceState::new(project_root.clone(), vec![task]);
    state.save().unwrap();

    // Load and verify unicode preserved
    let loaded_state = WorkspaceState::load(project_root).unwrap();
    assert_eq!(loaded_state.tasks[0].id, "task-unicode-测试");
    assert_eq!(
        loaded_state.tasks[0].title,
        "Tâche avec çäräçtërës spécïäux"
    );
    assert_eq!(
        loaded_state.tasks[0].description,
        "描述包含中文、العربية、עברית"
    );
}
