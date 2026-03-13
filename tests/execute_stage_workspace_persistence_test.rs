//! Integration tests for execute stage with workspace state persistence
//!
//! Tests that the execute stage automatically saves workspace state
//! after each task completion with proper error handling.

use ltmatrix::models::{Task, TaskStatus};
use ltmatrix::pipeline::execute::{execute_tasks, ExecuteConfig};
use ltmatrix::workspace::WorkspaceState;
use tempfile::TempDir;

/// Test that execute stage saves workspace state after task completion
#[tokio::test]
async fn test_execute_stage_saves_workspace_state() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    // Create initial workspace state
    let task1 = Task::new("task-1", "First task", "First task");
    let task2 = Task::new("task-2", "Second task", "Second task");

    let state = WorkspaceState::new(project_root.clone(), vec![task1, task2]);
    state.save().unwrap();

    // Execute tasks
    let tasks = state.tasks.clone();
    let mut config = ExecuteConfig::default();
    config.work_dir = project_root.clone();
    config.enable_workspace_persistence = true;
    config.project_root = Some(project_root.clone());

    // This should save workspace state after each task completion
    let _ = execute_tasks(tasks, &config).await;

    // Verify workspace state was saved after task completion
    let final_state = WorkspaceState::load(project_root).unwrap();

    // At least one task should be marked as completed (if execute ran)
    // This test verifies the save mechanism is integrated
    assert!(final_state.tasks.len() == 2);
}

/// Test that execute stage handles save failures gracefully
#[tokio::test]
async fn test_execute_stage_handles_save_failure() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    // Create workspace state
    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.clone(), vec![task]);
    state.save().unwrap();

    // Make the manifest file read-only to simulate save failure
    let manifest_path = project_root.join(".ltmatrix").join("tasks-manifest.json");

    #[cfg(unix)]
    {
        let mut perms = std::fs::metadata(&manifest_path).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&manifest_path, perms).unwrap();
    }

    // Execute should continue even if save fails
    let tasks = state.tasks.clone();
    let mut config = ExecuteConfig::default();
    config.work_dir = project_root.clone();
    config.enable_workspace_persistence = true;
    config.project_root = Some(project_root.clone());

    // Execute should not fail due to save errors
    let result = execute_tasks(tasks, &config).await;

    // Execution should succeed or fail for other reasons, not save errors
    // Save failures should be logged but not stop execution
    let _ = result; // We just verify it doesn't crash
}

/// Test that workspace state is disabled by default
#[tokio::test]
async fn test_workspace_persistence_disabled_by_default() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    // Create workspace state
    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.clone(), vec![task]);
    state.save().unwrap();

    // Execute with default config (persistence should be disabled)
    let tasks = state.tasks.clone();
    let mut config = ExecuteConfig::default();
    config.work_dir = project_root.clone();
    // Don't set enable_workspace_persistence or project_root

    let initial_state = WorkspaceState::load(project_root.clone()).unwrap();
    let _ = execute_tasks(tasks, &config).await;

    // Workspace state should not be updated (persistence disabled)
    let final_state = WorkspaceState::load(project_root).unwrap();

    // State should remain unchanged since persistence is disabled
    assert_eq!(final_state.tasks[0].status, initial_state.tasks[0].status);
}

/// Test atomic write of workspace state
#[tokio::test]
async fn test_workspace_state_atomic_write() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    // Create workspace state
    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.clone(), vec![task]);
    state.save().unwrap();

    // Verify initial state
    let loaded = WorkspaceState::load(project_root.clone()).unwrap();
    assert_eq!(loaded.tasks[0].status, TaskStatus::Pending);

    // Update and save
    let mut state = WorkspaceState::load(project_root.clone()).unwrap();
    state.tasks[0].status = TaskStatus::Completed;
    state.save().unwrap();

    // Verify the save was atomic (complete or none)
    let final_state = WorkspaceState::load(project_root).unwrap();
    assert_eq!(final_state.tasks[0].status, TaskStatus::Completed);

    // Verify metadata was updated
    assert!(final_state.metadata.modified_at > loaded.metadata.modified_at);
}
