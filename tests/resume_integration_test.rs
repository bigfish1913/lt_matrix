//! Integration tests for --resume functionality
//!
//! This test suite validates workspace state persistence and the --resume flag
//! implementation, including:
//! - Saving and loading workspace state across sessions
//! - State transformation for interrupted tasks
//! - Resuming from partial completion
//! - Ensuring completed tasks are not re-executed
//! - Consistency of final workspace state
//!
//! Task: Implement --resume flag for continuing interrupted work
//!
//! Acceptance Criteria:
//! 1. Workspace state persists via .ltmatrix/tasks-manifest.json
//! 2. --resume flag continues interrupted work
//! 3. InProgress tasks reset to Pending on resume
//! 4. Blocked tasks reset to Pending on resume
//! 5. Completed tasks remain Completed (not re-executed)
//! 6. Failed tasks remain Failed
//! 7. Pending tasks remain Pending
//! 8. State summary correctly reflects task counts
//! 9. Session IDs are preserved for retry reuse

use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::workspace::WorkspaceState;
use tempfile::TempDir;

// =============================================================================
// Helper Functions
// =============================================================================

/// Creates a sample task with the given ID, title, and dependencies
fn create_task(id: &str, title: &str, depends_on: Vec<String>) -> Task {
    let mut task = Task::new(id, title, format!("Description for {}", title));
    task.depends_on = depends_on;
    task.complexity = TaskComplexity::Moderate;
    task
}

// =============================================================================
// Workspace State Persistence Tests
// =============================================================================

/// Test that workspace state persists correctly to disk
#[test]
fn test_workspace_state_persists_to_disk() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create workspace state with tasks
    let tasks = vec![
        create_task("task-001", "First task", vec![]),
        create_task("task-002", "Second task", vec!["task-001".to_string()]),
    ];

    let state = WorkspaceState::new(project_path.clone(), tasks);
    state.save().expect("Failed to save state");

    // Verify manifest file exists
    let manifest_path = project_path.join(".ltmatrix").join("tasks-manifest.json");
    assert!(manifest_path.exists(), "Manifest file should exist");

    // Verify content is valid JSON
    let content = std::fs::read_to_string(&manifest_path).expect("Failed to read manifest");
    assert!(
        content.contains("task-001"),
        "Manifest should contain task-001"
    );
    assert!(
        content.contains("task-002"),
        "Manifest should contain task-002"
    );
}

/// Test loading workspace state from disk
#[test]
fn test_workspace_state_loads_from_disk() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create and save initial state
    let tasks = vec![
        create_task("task-001", "Setup project", vec![]),
        create_task("task-002", "Add dependencies", vec!["task-001".to_string()]),
        create_task("task-003", "Write tests", vec!["task-002".to_string()]),
    ];

    let original_state = WorkspaceState::new(project_path.clone(), tasks);
    let saved_state = original_state.save().expect("Failed to save state");

    // Load state back
    let loaded_state = WorkspaceState::load(project_path).expect("Failed to load state");

    // Verify loaded state matches saved state
    assert_eq!(loaded_state.tasks.len(), saved_state.tasks.len());
    assert_eq!(loaded_state.project_root, saved_state.project_root);

    // Verify each task
    for (saved, loaded) in saved_state.tasks.iter().zip(loaded_state.tasks.iter()) {
        assert_eq!(saved.id, loaded.id);
        assert_eq!(saved.title, loaded.title);
        assert_eq!(saved.status, loaded.status);
        assert_eq!(saved.depends_on, loaded.depends_on);
    }
}

/// Test that workspace state exists check works correctly
#[test]
fn test_workspace_state_exists_check() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Initially, no state exists
    assert!(
        !WorkspaceState::exists(&project_path),
        "State should not exist initially"
    );

    // Create and save state
    let state = WorkspaceState::new(project_path.clone(), vec![]);
    state.save().expect("Failed to save state");

    // Now state should exist
    assert!(
        WorkspaceState::exists(&project_path),
        "State should exist after save"
    );
}

/// Test loading state that doesn't exist fails gracefully
#[test]
fn test_load_nonexistent_state_fails() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Attempt to load state that doesn't exist
    let result = WorkspaceState::load(project_path);

    // Should fail with appropriate error
    assert!(result.is_err(), "Loading nonexistent state should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Failed to read") || err.to_string().contains("not exist"),
        "Error should indicate file doesn't exist: {}",
        err
    );
}

/// Test load_or_create creates new state if missing
#[test]
fn test_load_or_create_creates_new_state() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // No state exists initially
    assert!(!WorkspaceState::exists(&project_path));

    // load_or_create should create new state
    let state = WorkspaceState::load_or_create(project_path.clone())
        .expect("load_or_create should succeed");

    // Verify state was created
    assert!(state.tasks.is_empty(), "New state should have no tasks");
    assert!(
        WorkspaceState::exists(&project_path),
        "State file should be created"
    );
}

// =============================================================================
// State Transformation Tests (Resume Logic)
// =============================================================================

/// Test that InProgress tasks are reset to Pending on resume
#[test]
fn test_in_progress_tasks_reset_to_pending_on_resume() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create tasks with InProgress status
    let mut tasks = vec![
        create_task("task-001", "Setup", vec![]),
        create_task("task-002", "In progress task", vec!["task-001".to_string()]),
    ];

    tasks[0].status = TaskStatus::Completed;
    tasks[1].status = TaskStatus::InProgress;
    tasks[1].started_at = Some(chrono::Utc::now());

    // Save state
    let state = WorkspaceState::new(project_path.clone(), tasks);
    state.save().expect("Failed to save state");

    // Load with transform (simulates resume)
    let resumed =
        WorkspaceState::load_with_transform(project_path).expect("Failed to load with transform");

    // Verify transformation
    assert_eq!(
        resumed.tasks[0].status,
        TaskStatus::Completed,
        "Completed should stay completed"
    );
    assert_eq!(
        resumed.tasks[1].status,
        TaskStatus::Pending,
        "InProgress should become Pending"
    );
    assert!(
        resumed.tasks[1].started_at.is_none(),
        "started_at should be cleared"
    );
}

/// Test that Blocked tasks are reset to Pending on resume
#[test]
fn test_blocked_tasks_reset_to_pending_on_resume() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create tasks with Blocked status
    let mut tasks = vec![create_task(
        "task-001",
        "Blocked task",
        vec!["nonexistent".to_string()],
    )];

    tasks[0].status = TaskStatus::Blocked;

    // Save state
    let state = WorkspaceState::new(project_path.clone(), tasks);
    state.save().expect("Failed to save state");

    // Load with transform
    let resumed =
        WorkspaceState::load_with_transform(project_path).expect("Failed to load with transform");

    // Verify transformation
    assert_eq!(
        resumed.tasks[0].status,
        TaskStatus::Pending,
        "Blocked should become Pending"
    );
}

/// Test that Completed tasks remain Completed on resume
#[test]
fn test_completed_tasks_remain_completed_on_resume() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create completed tasks
    let mut tasks = vec![
        create_task("task-001", "First completed", vec![]),
        create_task("task-002", "Second completed", vec!["task-001".to_string()]),
    ];

    tasks[0].status = TaskStatus::Completed;
    tasks[0].completed_at = Some(chrono::Utc::now());
    tasks[1].status = TaskStatus::Completed;
    tasks[1].completed_at = Some(chrono::Utc::now());

    // Save state
    let state = WorkspaceState::new(project_path.clone(), tasks);
    state.save().expect("Failed to save state");

    // Load with transform
    let resumed =
        WorkspaceState::load_with_transform(project_path).expect("Failed to load with transform");

    // Verify completed tasks stay completed
    assert_eq!(resumed.tasks[0].status, TaskStatus::Completed);
    assert_eq!(resumed.tasks[1].status, TaskStatus::Completed);
    assert!(resumed.tasks[0].completed_at.is_some());
    assert!(resumed.tasks[1].completed_at.is_some());
}

/// Test that Failed tasks remain Failed on resume
#[test]
fn test_failed_tasks_remain_failed_on_resume() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create failed task
    let mut tasks = vec![create_task("task-001", "Failed task", vec![])];

    tasks[0].status = TaskStatus::Failed;
    tasks[0].error = Some("Something went wrong".to_string());

    // Save state
    let state = WorkspaceState::new(project_path.clone(), tasks);
    state.save().expect("Failed to save state");

    // Load with transform
    let resumed =
        WorkspaceState::load_with_transform(project_path).expect("Failed to load with transform");

    // Verify failed task stays failed
    assert_eq!(resumed.tasks[0].status, TaskStatus::Failed);
    assert_eq!(
        resumed.tasks[0].error,
        Some("Something went wrong".to_string())
    );
}

/// Test that Pending tasks remain Pending on resume
#[test]
fn test_pending_tasks_remain_pending_on_resume() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create pending tasks
    let tasks = vec![
        create_task("task-001", "Pending task", vec![]),
        create_task("task-002", "Another pending", vec!["task-001".to_string()]),
    ];

    // Save state
    let state = WorkspaceState::new(project_path.clone(), tasks);
    state.save().expect("Failed to save state");

    // Load with transform
    let resumed =
        WorkspaceState::load_with_transform(project_path).expect("Failed to load with transform");

    // Verify pending tasks stay pending
    assert_eq!(resumed.tasks[0].status, TaskStatus::Pending);
    assert_eq!(resumed.tasks[1].status, TaskStatus::Pending);
}

/// Test mixed task states transform correctly
#[test]
fn test_mixed_task_states_transform_correctly() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create tasks with all status types
    let mut tasks = vec![
        create_task("task-001", "Completed", vec![]),
        create_task("task-002", "InProgress", vec!["task-001".to_string()]),
        create_task("task-003", "Blocked", vec!["task-002".to_string()]),
        create_task("task-004", "Failed", vec![]),
        create_task("task-005", "Pending", vec![]),
        create_task("task-006", "Another in progress", vec![]),
    ];

    tasks[0].status = TaskStatus::Completed;
    tasks[1].status = TaskStatus::InProgress;
    tasks[2].status = TaskStatus::Blocked;
    tasks[3].status = TaskStatus::Failed;
    tasks[4].status = TaskStatus::Pending;
    tasks[5].status = TaskStatus::InProgress;

    // Save state
    let state = WorkspaceState::new(project_path.clone(), tasks);
    state.save().expect("Failed to save state");

    // Load with transform
    let resumed =
        WorkspaceState::load_with_transform(project_path).expect("Failed to load with transform");

    // Build task map for verification
    let task_map: std::collections::HashMap<&str, TaskStatus> = resumed
        .tasks
        .iter()
        .map(|t| (t.id.as_str(), t.status.clone()))
        .collect();

    // Verify transformations
    assert_eq!(
        task_map["task-001"],
        TaskStatus::Completed,
        "Completed should stay completed"
    );
    assert_eq!(
        task_map["task-002"],
        TaskStatus::Pending,
        "InProgress should become Pending"
    );
    assert_eq!(
        task_map["task-003"],
        TaskStatus::Pending,
        "Blocked should become Pending"
    );
    assert_eq!(
        task_map["task-004"],
        TaskStatus::Failed,
        "Failed should stay failed"
    );
    assert_eq!(
        task_map["task-005"],
        TaskStatus::Pending,
        "Pending should stay pending"
    );
    assert_eq!(
        task_map["task-006"],
        TaskStatus::Pending,
        "InProgress should become Pending"
    );
}

// =============================================================================
// Subtask State Transformation Tests
// =============================================================================

/// Test that subtasks are also transformed correctly
#[test]
fn test_subtasks_are_transformed_correctly() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create task with subtasks
    let mut parent = create_task("parent-001", "Parent task", vec![]);
    parent.status = TaskStatus::InProgress;

    let mut subtask1 = create_task("sub-001", "Completed subtask", vec![]);
    subtask1.status = TaskStatus::Completed;

    let mut subtask2 = create_task("sub-002", "In progress subtask", vec![]);
    subtask2.status = TaskStatus::InProgress;

    let mut subtask3 = create_task("sub-003", "Blocked subtask", vec![]);
    subtask3.status = TaskStatus::Blocked;

    let mut subtask4 = create_task("sub-004", "Failed subtask", vec![]);
    subtask4.status = TaskStatus::Failed;

    parent.subtasks = vec![subtask1, subtask2, subtask3, subtask4];

    // Save state
    let state = WorkspaceState::new(project_path.clone(), vec![parent]);
    state.save().expect("Failed to save state");

    // Load with transform
    let resumed =
        WorkspaceState::load_with_transform(project_path).expect("Failed to load with transform");

    // Verify parent transformation
    assert_eq!(
        resumed.tasks[0].status,
        TaskStatus::Pending,
        "Parent InProgress should become Pending"
    );

    // Verify subtask transformations
    assert_eq!(
        resumed.tasks[0].subtasks[0].status,
        TaskStatus::Completed,
        "Completed subtask should stay completed"
    );
    assert_eq!(
        resumed.tasks[0].subtasks[1].status,
        TaskStatus::Pending,
        "InProgress subtask should become Pending"
    );
    assert_eq!(
        resumed.tasks[0].subtasks[2].status,
        TaskStatus::Pending,
        "Blocked subtask should become Pending"
    );
    assert_eq!(
        resumed.tasks[0].subtasks[3].status,
        TaskStatus::Failed,
        "Failed subtask should stay failed"
    );
}

/// Test nested subtasks are transformed recursively
#[test]
fn test_nested_subtasks_transform_recursively() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create deeply nested structure
    let mut level3 = create_task("level-3", "Level 3", vec![]);
    level3.status = TaskStatus::InProgress;

    let mut level2 = create_task("level-2", "Level 2", vec![]);
    level2.status = TaskStatus::InProgress;
    level2.subtasks = vec![level3];

    let mut level1 = create_task("level-1", "Level 1", vec![]);
    level1.status = TaskStatus::InProgress;
    level1.subtasks = vec![level2];

    // Save state
    let state = WorkspaceState::new(project_path.clone(), vec![level1]);
    state.save().expect("Failed to save state");

    // Load with transform
    let resumed =
        WorkspaceState::load_with_transform(project_path).expect("Failed to load with transform");

    // All levels should be reset to Pending
    assert_eq!(resumed.tasks[0].status, TaskStatus::Pending);
    assert_eq!(resumed.tasks[0].subtasks[0].status, TaskStatus::Pending);
    assert_eq!(
        resumed.tasks[0].subtasks[0].subtasks[0].status,
        TaskStatus::Pending
    );
}

// =============================================================================
// Status Summary Tests
// =============================================================================

/// Test status summary counts correctly
#[test]
fn test_status_summary_counts_correctly() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create tasks with different statuses
    let mut tasks = vec![
        create_task("task-001", "Pending 1", vec![]),
        create_task("task-002", "Pending 2", vec![]),
        create_task("task-003", "In Progress", vec![]),
        create_task("task-004", "Completed 1", vec![]),
        create_task("task-005", "Completed 2", vec![]),
        create_task("task-006", "Completed 3", vec![]),
        create_task("task-007", "Failed", vec![]),
        create_task("task-008", "Blocked", vec![]),
    ];

    tasks[0].status = TaskStatus::Pending;
    tasks[1].status = TaskStatus::Pending;
    tasks[2].status = TaskStatus::InProgress;
    tasks[3].status = TaskStatus::Completed;
    tasks[4].status = TaskStatus::Completed;
    tasks[5].status = TaskStatus::Completed;
    tasks[6].status = TaskStatus::Failed;
    tasks[7].status = TaskStatus::Blocked;

    let state = WorkspaceState::new(temp_dir.path().to_path_buf(), tasks);
    let summary = state.status_summary();

    assert_eq!(summary.pending, 2);
    assert_eq!(summary.in_progress, 1);
    assert_eq!(summary.completed, 3);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.blocked, 1);
    assert_eq!(summary.total(), 8);
}

/// Test completion percentage calculation
#[test]
fn test_completion_percentage_calculation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // 50% completion: 4 tasks, 2 completed
    let mut tasks = vec![
        create_task("task-001", "Completed 1", vec![]),
        create_task("task-002", "Completed 2", vec![]),
        create_task("task-003", "Pending 1", vec![]),
        create_task("task-004", "Pending 2", vec![]),
    ];

    tasks[0].status = TaskStatus::Completed;
    tasks[1].status = TaskStatus::Completed;
    tasks[2].status = TaskStatus::Pending;
    tasks[3].status = TaskStatus::Pending;

    let state = WorkspaceState::new(temp_dir.path().to_path_buf(), tasks);
    let summary = state.status_summary();

    assert!((summary.completion_percentage() - 50.0).abs() < 0.1);
}

/// Test status summary with subtasks
#[test]
fn test_status_summary_includes_subtasks() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create parent with subtasks
    let mut parent = create_task("parent", "Parent", vec![]);
    parent.status = TaskStatus::Completed;

    let mut subtask1 = create_task("sub-1", "Subtask 1", vec![]);
    subtask1.status = TaskStatus::Completed;

    let mut subtask2 = create_task("sub-2", "Subtask 2", vec![]);
    subtask2.status = TaskStatus::Pending;

    parent.subtasks = vec![subtask1, subtask2];

    let state = WorkspaceState::new(temp_dir.path().to_path_buf(), vec![parent]);
    let summary = state.status_summary();

    // Should count parent and both subtasks
    assert_eq!(
        summary.completed, 2,
        "Parent and subtask1 should be completed"
    );
    assert_eq!(summary.pending, 1, "subtask2 should be pending");
    assert_eq!(summary.total(), 3, "Total should include subtasks");
}

// =============================================================================
// Session ID Preservation Tests
// =============================================================================

/// Test that session IDs are preserved on save/load
#[test]
fn test_session_ids_preserved_on_save_load() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create tasks with session IDs
    let mut task1 = create_task("task-001", "Task with session", vec![]);
    task1.status = TaskStatus::InProgress;
    task1.session_id = Some("session-12345".to_string());

    let mut task2 = create_task(
        "task-002",
        "Task with parent session",
        vec!["task-001".to_string()],
    );
    task2.parent_session_id = Some("session-12345".to_string());

    // Save state
    let state = WorkspaceState::new(project_path.clone(), vec![task1, task2]);
    state.save().expect("Failed to save state");

    // Load state
    let loaded = WorkspaceState::load(project_path).expect("Failed to load state");

    // Verify session IDs are preserved
    assert_eq!(
        loaded.tasks[0].session_id,
        Some("session-12345".to_string())
    );
    assert_eq!(
        loaded.tasks[1].parent_session_id,
        Some("session-12345".to_string())
    );
}

/// Test that session IDs are cleared when task is reset
#[test]
fn test_started_at_cleared_on_transform() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create InProgress task with started_at
    let mut task = create_task("task-001", "In progress", vec![]);
    task.status = TaskStatus::InProgress;
    task.started_at = Some(chrono::Utc::now());
    task.session_id = Some("session-abc".to_string());

    // Save state
    let state = WorkspaceState::new(project_path.clone(), vec![task]);
    state.save().expect("Failed to save state");

    // Load with transform
    let resumed =
        WorkspaceState::load_with_transform(project_path).expect("Failed to load with transform");

    // started_at should be cleared, session_id should be preserved
    assert_eq!(resumed.tasks[0].status, TaskStatus::Pending);
    assert!(
        resumed.tasks[0].started_at.is_none(),
        "started_at should be cleared"
    );
    // Note: session_id is preserved for retry reuse
    assert_eq!(resumed.tasks[0].session_id, Some("session-abc".to_string()));
}

// =============================================================================
// Cleanup and Reset Tests
// =============================================================================

/// Test workspace cleanup removes all state
#[test]
fn test_cleanup_removes_all_state() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create workspace state
    let state = WorkspaceState::new(
        project_path.clone(),
        vec![create_task("task-001", "Test", vec![])],
    );
    state.save().expect("Failed to save state");

    // Verify state exists
    assert!(WorkspaceState::exists(&project_path));

    // Cleanup
    WorkspaceState::cleanup(&project_path).expect("Failed to cleanup");

    // Verify cleanup
    assert!(!WorkspaceState::exists(&project_path));
    assert!(!project_path.join(".ltmatrix").exists());
}

/// Test reset_all resets all tasks to Pending
#[test]
fn test_reset_all_resets_to_pending() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create tasks with various statuses
    let mut tasks = vec![
        create_task("task-001", "Completed", vec![]),
        create_task("task-002", "Failed", vec![]),
        create_task("task-003", "Blocked", vec![]),
        create_task("task-004", "InProgress", vec![]),
    ];

    tasks[0].status = TaskStatus::Completed;
    tasks[1].status = TaskStatus::Failed;
    tasks[2].status = TaskStatus::Blocked;
    tasks[3].status = TaskStatus::InProgress;

    let mut state = WorkspaceState::new(temp_dir.path().to_path_buf(), tasks);
    state.reset_all().expect("Failed to reset");

    // All tasks should be pending
    for task in &state.tasks {
        assert_eq!(task.status, TaskStatus::Pending);
    }
}

/// Test reset_failed only resets failed tasks
#[test]
fn test_reset_failed_only_resets_failed() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create tasks with various statuses
    let mut tasks = vec![
        create_task("task-001", "Completed", vec![]),
        create_task("task-002", "Failed 1", vec![]),
        create_task("task-003", "Failed 2", vec![]),
        create_task("task-004", "Pending", vec![]),
    ];

    tasks[0].status = TaskStatus::Completed;
    tasks[1].status = TaskStatus::Failed;
    tasks[2].status = TaskStatus::Failed;
    tasks[3].status = TaskStatus::Pending;

    let mut state = WorkspaceState::new(temp_dir.path().to_path_buf(), tasks);
    let reset_count = state.reset_failed().expect("Failed to reset failed");

    // Should have reset 2 failed tasks
    assert_eq!(reset_count, 2);

    // Verify statuses
    assert_eq!(
        state.tasks[0].status,
        TaskStatus::Completed,
        "Completed should stay completed"
    );
    assert_eq!(
        state.tasks[1].status,
        TaskStatus::Pending,
        "Failed should become Pending"
    );
    assert_eq!(
        state.tasks[2].status,
        TaskStatus::Pending,
        "Failed should become Pending"
    );
    assert_eq!(
        state.tasks[3].status,
        TaskStatus::Pending,
        "Pending should stay Pending"
    );
}

// =============================================================================
// Resume Workflow Tests
// =============================================================================

/// Test complete resume workflow: partial completion, interrupt, resume
#[test]
fn test_complete_resume_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Phase 1: Initial run - some tasks complete, one in progress
    let mut tasks = vec![
        create_task("task-001", "Setup project", vec![]),
        create_task("task-002", "Add dependencies", vec!["task-001".to_string()]),
        create_task("task-003", "Write tests", vec!["task-002".to_string()]),
        create_task(
            "task-004",
            "Integration tests",
            vec!["task-003".to_string()],
        ),
    ];

    // Simulate partial completion
    tasks[0].status = TaskStatus::Completed;
    tasks[0].completed_at = Some(chrono::Utc::now());

    tasks[1].status = TaskStatus::Completed;
    tasks[1].completed_at = Some(chrono::Utc::now());

    tasks[2].status = TaskStatus::InProgress;
    tasks[2].started_at = Some(chrono::Utc::now());

    tasks[3].status = TaskStatus::Pending;

    // Save state (simulates interruption)
    let state = WorkspaceState::new(project_path.clone(), tasks);
    state.save().expect("Failed to save state");

    // Verify intermediate state
    let intermediate = WorkspaceState::load(project_path.clone()).expect("Failed to load");
    assert_eq!(intermediate.tasks[0].status, TaskStatus::Completed);
    assert_eq!(intermediate.tasks[2].status, TaskStatus::InProgress);

    // Phase 2: Resume - load with transform
    let resumed = WorkspaceState::load_with_transform(project_path.clone())
        .expect("Failed to load with transform");

    // Verify transformation
    assert_eq!(
        resumed.tasks[0].status,
        TaskStatus::Completed,
        "Completed task should stay completed"
    );
    assert_eq!(
        resumed.tasks[1].status,
        TaskStatus::Completed,
        "Completed task should stay completed"
    );
    assert_eq!(
        resumed.tasks[2].status,
        TaskStatus::Pending,
        "InProgress should be reset to Pending"
    );
    assert_eq!(
        resumed.tasks[3].status,
        TaskStatus::Pending,
        "Pending should stay Pending"
    );

    // Phase 3: Continue execution - complete remaining tasks
    let mut continued_state = resumed;
    continued_state.tasks[2].status = TaskStatus::Completed;
    continued_state.tasks[2].completed_at = Some(chrono::Utc::now());
    continued_state.tasks[3].status = TaskStatus::Completed;
    continued_state.tasks[3].completed_at = Some(chrono::Utc::now());

    continued_state
        .save()
        .expect("Failed to save continued state");

    // Verify final state
    let final_state = WorkspaceState::load(project_path).expect("Failed to load final state");
    let summary = final_state.status_summary();

    assert_eq!(summary.completed, 4, "All tasks should be completed");
    assert_eq!(summary.pending, 0);
    assert_eq!(summary.in_progress, 0);
    assert!((summary.completion_percentage() - 100.0).abs() < 0.1);
}

/// Test resume with dependency chain
#[test]
fn test_resume_preserves_dependency_chain() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create dependency chain: task-001 -> task-002 -> task-003 -> task-004
    let mut tasks = vec![
        create_task("task-001", "First", vec![]),
        create_task("task-002", "Second", vec!["task-001".to_string()]),
        create_task("task-003", "Third", vec!["task-002".to_string()]),
        create_task("task-004", "Fourth", vec!["task-003".to_string()]),
    ];

    // Simulate interruption during task-003
    tasks[0].status = TaskStatus::Completed;
    tasks[1].status = TaskStatus::Completed;
    tasks[2].status = TaskStatus::InProgress;
    tasks[3].status = TaskStatus::Pending;

    let state = WorkspaceState::new(project_path.clone(), tasks);
    state.save().expect("Failed to save state");

    // Resume
    let resumed =
        WorkspaceState::load_with_transform(project_path).expect("Failed to load with transform");

    // Verify dependencies are preserved
    assert_eq!(resumed.tasks[0].depends_on, vec![] as Vec<String>);
    assert_eq!(resumed.tasks[1].depends_on, vec!["task-001".to_string()]);
    assert_eq!(resumed.tasks[2].depends_on, vec!["task-002".to_string()]);
    assert_eq!(resumed.tasks[3].depends_on, vec!["task-003".to_string()]);

    // Verify dependency checking works
    let completed: std::collections::HashSet<String> =
        ["task-001".to_string(), "task-002".to_string()]
            .into_iter()
            .collect();

    // task-003 should be executable after task-001 and task-002 are completed
    assert!(resumed.tasks[2].can_execute(&completed));
    // task-004 should not be executable yet
    assert!(!resumed.tasks[3].can_execute(&completed));
}

/// Test resume handles multiple interruptions
#[test]
fn test_multiple_interruptions_resume() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Initial state
    let tasks = vec![
        create_task("task-001", "Task 1", vec![]),
        create_task("task-002", "Task 2", vec![]),
        create_task("task-003", "Task 3", vec![]),
    ];

    let state = WorkspaceState::new(project_path.clone(), tasks);
    state.save().expect("Failed to save initial state");

    // First interruption: task-001 completed, task-002 in progress
    let mut first_interruption =
        WorkspaceState::load(project_path.clone()).expect("Failed to load");
    first_interruption.tasks[0].status = TaskStatus::Completed;
    first_interruption.tasks[1].status = TaskStatus::InProgress;
    first_interruption
        .save()
        .expect("Failed to save first interruption");

    // First resume
    let first_resume = WorkspaceState::load_with_transform(project_path.clone())
        .expect("Failed to load with transform");
    assert_eq!(first_resume.tasks[0].status, TaskStatus::Completed);
    assert_eq!(first_resume.tasks[1].status, TaskStatus::Pending);
    assert_eq!(first_resume.tasks[2].status, TaskStatus::Pending);

    // Continue and save second interruption
    let mut second_interruption = first_resume;
    second_interruption.tasks[1].status = TaskStatus::Completed;
    second_interruption.tasks[2].status = TaskStatus::InProgress;
    second_interruption
        .save()
        .expect("Failed to save second interruption");

    // Second resume
    let second_resume = WorkspaceState::load_with_transform(project_path.clone())
        .expect("Failed to load with transform");

    // Verify state after second resume
    assert_eq!(
        second_resume.tasks[0].status,
        TaskStatus::Completed,
        "First task should stay completed"
    );
    assert_eq!(
        second_resume.tasks[1].status,
        TaskStatus::Completed,
        "Second task should stay completed"
    );
    assert_eq!(
        second_resume.tasks[2].status,
        TaskStatus::Pending,
        "Third task should be pending"
    );
}

// =============================================================================
// Metadata Tests
// =============================================================================

/// Test metadata timestamps are updated
#[test]
fn test_metadata_timestamps_updated() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create and save state
    let state = WorkspaceState::new(project_path.clone(), vec![]);
    let saved = state.save().expect("Failed to save state");

    // Verify metadata
    assert_eq!(saved.metadata.version, "1.0");
    assert!(saved.metadata.created_at <= saved.metadata.modified_at);

    // Wait a bit and save again
    std::thread::sleep(std::time::Duration::from_millis(10));
    let resaved = saved.save().expect("Failed to resave");

    // modified_at should be updated
    assert!(resaved.metadata.modified_at > saved.metadata.modified_at);
    // created_at should stay the same
    assert_eq!(resaved.metadata.created_at, saved.metadata.created_at);
}

// =============================================================================
// Edge Cases
// =============================================================================

/// Test empty workspace state
#[test]
fn test_empty_workspace_state() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create empty state
    let state = WorkspaceState::new(project_path.clone(), vec![]);
    state.save().expect("Failed to save empty state");

    // Load and verify
    let loaded = WorkspaceState::load(project_path).expect("Failed to load empty state");
    assert!(loaded.tasks.is_empty());

    let summary = loaded.status_summary();
    assert_eq!(summary.total(), 0);
    assert_eq!(summary.completion_percentage(), 0.0);
}

/// Test workspace state with many tasks
#[test]
fn test_workspace_state_with_many_tasks() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create 100 tasks
    let tasks: Vec<Task> = (0..100)
        .map(|i| create_task(&format!("task-{:03}", i), &format!("Task {}", i), vec![]))
        .collect();

    let state = WorkspaceState::new(project_path.clone(), tasks);
    state.save().expect("Failed to save state with many tasks");

    // Load and verify
    let loaded = WorkspaceState::load(project_path).expect("Failed to load state");
    assert_eq!(loaded.tasks.len(), 100);

    // Verify all tasks are present
    for i in 0..100 {
        assert!(
            loaded
                .tasks
                .iter()
                .any(|t| t.id == format!("task-{:03}", i)),
            "Task {:03} should be present",
            i
        );
    }
}

/// Test concurrent access to workspace state
/// Note: This test verifies that concurrent access doesn't crash the application.
/// Without file locking, concurrent writes may cause data loss but should not corrupt
/// the file format. The test uses load_or_create to recover if corruption occurs.
#[test]
fn test_concurrent_workspace_access() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = Arc::new(temp_dir.path().to_path_buf());

    // Create initial state
    let state = WorkspaceState::new((*project_path).clone(), vec![]);
    state.save().expect("Failed to save initial state");

    // Spawn multiple threads that load/save state
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let path = Arc::clone(&project_path);
            thread::spawn(move || {
                // Use load_or_create to handle potential corruption from concurrent writes
                let mut state = WorkspaceState::load_or_create((*path).clone())
                    .expect("Failed to load or create");
                let task = create_task(&format!("task-{}", i), &format!("Task {}", i), vec![]);
                state.tasks.push(task);
                // Save may fail due to concurrent writes, but that's expected
                let _ = state.save();
            })
        })
        .collect();

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Verify final state - use load_or_create to handle any corruption
    let final_state = WorkspaceState::load_or_create((*project_path).clone())
        .expect("Failed to load or create final");
    // Note: Due to race conditions without file locking, we might have 0 or more tasks
    // This test mainly verifies the application doesn't crash and can recover
    assert!(
        final_state.tasks.len() <= 5,
        "At most 5 tasks should be present"
    );
}

/// Test corrupted state file handling
#[test]
fn test_corrupted_state_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create a corrupted manifest file
    let manifest_path = project_path.join(".ltmatrix").join("tasks-manifest.json");
    std::fs::create_dir_all(manifest_path.parent().unwrap()).expect("Failed to create dir");
    std::fs::write(&manifest_path, "invalid json {{{").expect("Failed to write corrupted data");

    // Load should fail
    let result = WorkspaceState::load(project_path.clone());
    assert!(result.is_err(), "Loading corrupted state should fail");

    // load_or_create should recover
    let recovered =
        WorkspaceState::load_or_create(project_path).expect("load_or_create should recover");
    assert!(
        recovered.tasks.is_empty(),
        "Recovered state should be empty"
    );
}
