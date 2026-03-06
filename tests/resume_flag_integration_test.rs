//! Integration tests for --resume flag functionality
//!
//! Tests the complete resume workflow including:
//! - CLI argument parsing for --resume flag
//! - State loading with transformation on resume
//! - Continuation of interrupted execution
//! - Integration with workspace persistence

use clap::Parser;
use ltmatrix::cli::args::Args;
use ltmatrix::workspace::WorkspaceState;
use ltmatrix::models::{Task, TaskStatus};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ==================== CLI Argument Parsing Tests ====================

#[test]
fn test_resume_flag_parsing() {
    // Test that --resume flag is parsed correctly
    let args = Args::try_parse_from(["ltmatrix", "--resume"]).unwrap();
    assert!(args.resume, "Resume flag should be true");
    assert!(args.goal.is_none(), "Goal should be optional with --resume");
}

#[test]
fn test_resume_flag_with_goal() {
    // Test that --resume can be combined with a goal
    let args = Args::try_parse_from(["ltmatrix", "--resume", "complete the task"]).unwrap();
    assert!(args.resume, "Resume flag should be true");
    assert_eq!(args.goal, Some("complete the task".to_string()));
}

#[test]
fn test_resume_flag_with_other_flags() {
    // Test that --resume works with other flags
    let args = Args::try_parse_from([
        "ltmatrix",
        "--resume",
        "--fast",
        "continue work"
    ]).unwrap();
    assert!(args.resume, "Resume flag should be true");
    assert!(args.fast, "Fast flag should also be true");
    assert_eq!(args.goal, Some("continue work".to_string()));
}

#[test]
fn test_resume_default_is_false() {
    // Test that resume defaults to false
    let args = Args::try_parse_from(["ltmatrix", "test goal"]).unwrap();
    assert!(!args.resume, "Resume flag should be false by default");
}

#[test]
fn test_resume_flag_with_dry_run() {
    // Test that --resume and --dry-run can be combined
    let args = Args::try_parse_from([
        "ltmatrix",
        "--resume",
        "--dry-run"
    ]).unwrap();
    assert!(args.resume, "Resume flag should be true");
    assert!(args.dry_run, "Dry run flag should also be true");
}

#[test]
fn test_resume_flag_with_config() {
    // Test that --resume works with --config flag
    let args = Args::try_parse_from([
        "ltmatrix",
        "--resume",
        "--config",
        "custom-config.toml"
    ]).unwrap();
    assert!(args.resume, "Resume flag should be true");
    assert_eq!(args.config, Some(PathBuf::from("custom-config.toml")));
}

// ==================== Resume Workflow Integration Tests ====================

#[test]
fn test_resume_loads_previous_state() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create and save a state with some tasks
    let task1 = Task::new("task-1", "Completed Task", "Done");
    let mut task2 = Task::new("task-2", "In Progress Task", "Working on it");
    task2.status = TaskStatus::InProgress;

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);
    state.save().unwrap();

    // Simulate resume: load state with transform
    let loaded_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Verify state was loaded and transformed
    assert_eq!(loaded_state.tasks.len(), 2);
    assert_eq!(loaded_state.tasks[0].id, "task-1");
    assert_eq!(loaded_state.tasks[1].id, "task-2");
    assert_eq!(loaded_state.tasks[1].status, TaskStatus::Pending,
        "InProgress task should be reset to Pending on resume");
}

#[test]
fn test_resume_preserves_completed_tasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create a state with mix of completed and pending tasks
    let mut task1 = Task::new("task-1", "Completed", "Done");
    task1.status = TaskStatus::Completed;
    task1.completed_at = Some(chrono::Utc::now());

    let task2 = Task::new("task-2", "Pending", "Not started");

    let mut task3 = Task::new("task-3", "In Progress", "Started but interrupted");
    task3.status = TaskStatus::InProgress;

    let state = WorkspaceState::new(
        project_root.to_path_buf(),
        vec![task1, task2, task3]
    );
    state.save().unwrap();

    // Resume: load with transform
    let resumed_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Completed tasks should stay completed
    assert_eq!(resumed_state.tasks[0].status, TaskStatus::Completed);
    assert!(resumed_state.tasks[0].completed_at.is_some(),
        "Completed timestamp should be preserved");

    // Pending tasks should stay pending
    assert_eq!(resumed_state.tasks[1].status, TaskStatus::Pending);

    // InProgress tasks should be reset to Pending
    assert_eq!(resumed_state.tasks[2].status, TaskStatus::Pending);
}

#[test]
fn test_resume_handles_blocked_tasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create a state with blocked tasks
    let mut task1 = Task::new("task-1", "Blocked Task", "Waiting for something");
    task1.status = TaskStatus::Blocked;
    task1.started_at = Some(chrono::Utc::now());

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1]);
    state.save().unwrap();

    // Resume: load with transform
    let resumed_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Blocked tasks should be reset to Pending
    assert_eq!(resumed_state.tasks[0].status, TaskStatus::Pending);
    assert!(resumed_state.tasks[0].started_at.is_none(),
        "Started timestamp should be cleared on resume");
}

#[test]
fn test_resume_preserves_failed_tasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create a state with failed task
    let mut task1 = Task::new("task-1", "Failed Task", "Didn't work");
    task1.status = TaskStatus::Failed;
    task1.error = Some("Something went wrong".to_string());
    task1.retry_count = 2;

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1]);
    state.save().unwrap();

    // Resume: load with transform
    let resumed_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Failed status and error should be preserved
    assert_eq!(resumed_state.tasks[0].status, TaskStatus::Failed);
    assert_eq!(resumed_state.tasks[0].error,
        Some("Something went wrong".to_string()));
    assert_eq!(resumed_state.tasks[0].retry_count, 2,
        "Retry count should be preserved");
}

#[test]
fn test_resume_with_dependency_chain() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create a dependency chain: task-1 -> task-2 -> task-3 -> task-4
    let mut task1 = Task::new("task-1", "First", "Start here");
    task1.status = TaskStatus::Completed;

    let mut task2 = Task::new("task-2", "Second", "After first");
    task2.status = TaskStatus::Completed;
    task2.depends_on = vec!["task-1".to_string()];

    let mut task3 = Task::new("task-3", "Third", "After second");
    task3.status = TaskStatus::InProgress;
    task3.depends_on = vec!["task-2".to_string()];

    let mut task4 = Task::new("task-4", "Fourth", "After third");
    task4.status = TaskStatus::Pending;
    task4.depends_on = vec!["task-3".to_string()];

    let state = WorkspaceState::new(
        project_root.to_path_buf(),
        vec![task1, task2, task3, task4]
    );
    state.save().unwrap();

    // Resume: load with transform
    let resumed_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Verify dependency chain preserved
    assert_eq!(resumed_state.tasks[0].status, TaskStatus::Completed);
    assert_eq!(resumed_state.tasks[1].status, TaskStatus::Completed);
    assert_eq!(resumed_state.tasks[2].status, TaskStatus::Pending,
        "InProgress task should be reset");
    assert_eq!(resumed_state.tasks[3].status, TaskStatus::Pending);

    // Verify dependencies intact
    assert_eq!(resumed_state.tasks[1].depends_on, vec!["task-1"]);
    assert_eq!(resumed_state.tasks[2].depends_on, vec!["task-2"]);
    assert_eq!(resumed_state.tasks[3].depends_on, vec!["task-3"]);
}

#[test]
fn test_resume_with_nested_subtasks() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create parent task with subtasks in various states
    let mut subtask1 = Task::new("subtask-1", "Completed Subtask", "Done");
    subtask1.status = TaskStatus::Completed;

    let mut subtask2 = Task::new("subtask-2", "Interrupted Subtask", "Was running");
    subtask2.status = TaskStatus::InProgress;

    let mut parent = Task::new("parent-1", "Parent Task", "Has subtasks");
    parent.status = TaskStatus::InProgress;
    parent.subtasks = vec![subtask1, subtask2];

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![parent]);
    state.save().unwrap();

    // Resume: load with transform
    let resumed_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Both parent and interrupted subtask should be reset
    assert_eq!(resumed_state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(resumed_state.tasks[0].subtasks[0].status, TaskStatus::Completed,
        "Completed subtask should remain completed");
    assert_eq!(resumed_state.tasks[0].subtasks[1].status, TaskStatus::Pending,
        "InProgress subtask should be reset");
}

#[test]
fn test_resume_with_orphaned_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create task with orphaned dependencies
    let task1 = Task::new("task-1", "Valid Task", "Exists");
    let mut task2 = Task::new("task-2", "Broken Dependencies", "Has missing deps");
    task2.depends_on = vec![
        "task-1".to_string(),
        "missing-task".to_string(),
    ];

    let state = WorkspaceState::new(
        project_root.to_path_buf(),
        vec![task1, task2]
    );
    state.save().unwrap();

    // Resume: load and detect orphaned tasks
    let resumed_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();
    let orphaned = resumed_state.detect_orphaned_tasks();

    // Should detect the orphaned dependency
    assert_eq!(orphaned.len(), 1);
    assert_eq!(orphaned[0].0, "task-2");
    assert!(orphaned[0].1.contains(&"missing-task".to_string()));
}

#[test]
fn test_resume_handles_missing_state_file() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Don't create any state file

    // Attempting to resume with missing state should fail with clear error
    let result = WorkspaceState::load(project_root.to_path_buf());

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(error_msg.contains("failed to read") ||
            error_msg.contains("not found") ||
            error_msg.contains("no such file"));
}

#[test]
fn test_resume_handles_corrupted_state_file() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create corrupted state file
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let manifest_path = ltmatrix_dir.join("tasks-manifest.json");
    fs::write(&manifest_path, "{ corrupted json content }").unwrap();

    // Resume should detect corruption and provide clear error
    let result = WorkspaceState::load(project_root.to_path_buf());

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(error_msg.contains("parse") || error_msg.contains("corrupted"));
}

#[test]
fn test_resume_with_load_or_create_fallback() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Don't create any state file

    // load_or_create should create new empty state
    let result = WorkspaceState::load_or_create(project_root.to_path_buf());

    assert!(result.is_ok());
    let state = result.unwrap();

    assert_eq!(state.tasks.len(), 0);
    assert_eq!(state.project_root, project_root);

    // Should have created the state file
    let manifest_path = project_root.join(".ltmatrix").join("tasks-manifest.json");
    assert!(manifest_path.exists());
}

#[test]
fn test_resume_preserves_session_ids() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create tasks with session IDs
    let mut task1 = Task::new("task-1", "Task 1", "Description");
    task1.status = TaskStatus::Completed;
    task1.session_id = Some("session-abc123".to_string());

    let mut task2 = Task::new("task-2", "Task 2", "Description");
    task2.status = TaskStatus::InProgress;
    task2.session_id = Some("session-def456".to_string());

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1, task2]);
    state.save().unwrap();

    // Resume: load with transform
    let resumed_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Session IDs should be preserved
    assert_eq!(resumed_state.tasks[0].session_id, Some("session-abc123".to_string()));
    assert_eq!(resumed_state.tasks[1].session_id, Some("session-def456".to_string()));
}

#[test]
fn test_resume_multiple_times() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create initial state
    let mut task1 = Task::new("task-1", "Task 1", "Description");
    task1.status = TaskStatus::InProgress;

    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1]);
    state.save().unwrap();

    // First resume
    let resumed_state1 = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();
    assert_eq!(resumed_state1.tasks[0].status, TaskStatus::Pending);

    // Save again
    resumed_state1.save().unwrap();

    // Second resume should be idempotent
    let resumed_state2 = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();
    assert_eq!(resumed_state2.tasks[0].status, TaskStatus::Pending);
}

#[test]
fn test_resume_preserves_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create state with metadata
    let task1 = Task::new("task-1", "Task", "Description");
    let original_state = WorkspaceState::new(project_root.to_path_buf(), vec![task1]);
    let saved_state = original_state.save().unwrap();

    let created_at = saved_state.metadata.created_at;

    // Resume: load with transform
    let resumed_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // Metadata should be preserved
    assert_eq!(resumed_state.metadata.created_at, created_at);
    assert_eq!(resumed_state.metadata.version, "1.0");
}

#[test]
fn test_resume_with_large_task_list() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create many tasks in various states
    let tasks: Vec<Task> = (0..50)
        .map(|i| {
            let mut task = Task::new(
                &format!("task-{}", i),
                &format!("Task {}", i),
                "Description"
            );

            // Set different statuses
            match i % 4 {
                0 => task.status = TaskStatus::Completed,
                1 => task.status = TaskStatus::Pending,
                2 => task.status = TaskStatus::InProgress,
                3 => task.status = TaskStatus::Blocked,
                _ => {}
            }

            task
        })
        .collect();

    let state = WorkspaceState::new(project_root.to_path_buf(), tasks);
    state.save().unwrap();

    // Resume: load with transform
    let resumed_state = WorkspaceState::load_with_transform(project_root.to_path_buf()).unwrap();

    // All tasks should be loaded
    assert_eq!(resumed_state.tasks.len(), 50);

    // Count tasks by status
    let completed_count = resumed_state.tasks.iter()
        .filter(|t| t.status == TaskStatus::Completed).count();
    let pending_count = resumed_state.tasks.iter()
        .filter(|t| t.status == TaskStatus::Pending).count();

    // All Completed tasks should still be completed (~12-13 tasks)
    assert!(completed_count >= 12);
    // All InProgress and Blocked should be reset to Pending (~25 tasks)
    // Plus the original Pending tasks (~12-13 tasks)
    assert!(pending_count >= 25);
}
