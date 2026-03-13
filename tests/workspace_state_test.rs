//! Workspace state persistence tests
//!
//! Tests the WorkspaceState struct and its save/load functionality
//! for tasks-manifest.json serialization and deserialization.

use ltmatrix::models::{Task, TaskStatus};
use ltmatrix::workspace::WorkspaceState;
use std::fs;
use tempfile::TempDir;

/// Test that WorkspaceState can be created with a list of tasks
#[test]
fn test_workspace_state_creation() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    let tasks = vec![
        Task::new("task-1", "First task", "Description 1"),
        Task::new("task-2", "Second task", "Description 2"),
    ];

    let state = WorkspaceState::new(project_root.clone(), tasks);

    assert_eq!(state.tasks.len(), 2);
    assert_eq!(state.tasks[0].id, "task-1");
    assert_eq!(state.project_root, project_root);
}

/// Test that WorkspaceState serializes to valid JSON
#[test]
fn test_workspace_state_serialization() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    let mut task1 = Task::new("task-1", "First task", "Description 1");
    task1.status = TaskStatus::Completed;

    let mut task2 = Task::new("task-2", "Second task", "Description 2");
    task2.status = TaskStatus::InProgress;
    task2.depends_on = vec!["task-1".to_string()];

    let state = WorkspaceState::new(project_root, vec![task1, task2]);

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&state).unwrap();

    // Verify JSON contains expected fields
    assert!(json.contains("\"tasks\""));
    assert!(json.contains("\"task-1\""));
    assert!(json.contains("\"task-2\""));
    assert!(json.contains("\"project_root\""));
    assert!(json.contains("\"status\""));
    assert!(json.contains("\"completed\""));
    assert!(json.contains("\"in_progress\""));
}

/// Test that WorkspaceState deserializes from JSON correctly
#[test]
fn test_workspace_state_deserialization() {
    let json = r#"{
        "project_root": "/test/project",
        "tasks": [
            {
                "id": "task-1",
                "title": "First task",
                "description": "Description 1",
                "status": "pending",
                "complexity": "moderate",
                "depends_on": [],
                "subtasks": [],
                "retry_count": 0,
                "created_at": "2024-01-01T00:00:00Z"
            },
            {
                "id": "task-2",
                "title": "Second task",
                "description": "Description 2",
                "status": "completed",
                "complexity": "simple",
                "depends_on": ["task-1"],
                "subtasks": [],
                "retry_count": 0,
                "created_at": "2024-01-01T00:00:00Z"
            }
        ],
        "metadata": {
            "version": "1.0",
            "created_at": "2024-01-01T00:00:00Z",
            "modified_at": "2024-01-01T01:00:00Z"
        }
    }"#;

    let state: WorkspaceState = serde_json::from_str(json).unwrap();

    assert_eq!(state.tasks.len(), 2);
    assert_eq!(state.tasks[0].id, "task-1");
    assert_eq!(state.tasks[1].id, "task-2");
    assert_eq!(state.tasks[0].status, TaskStatus::Pending);
    assert_eq!(state.tasks[1].status, TaskStatus::Completed);
    assert_eq!(state.tasks[1].depends_on.len(), 1);
    assert_eq!(state.tasks[1].depends_on[0], "task-1");
}

/// Test that save function creates tasks-manifest.json file
#[test]
fn test_workspace_state_save() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    let task1 = Task::new("task-1", "First task", "Description 1");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task1]);

    // Save the state
    let result = state.save();
    assert!(result.is_ok());

    // Verify file was created
    let manifest_path = project_root.join(".ltmatrix").join("tasks-manifest.json");
    assert!(manifest_path.exists());

    // Verify file contains valid JSON
    let content = fs::read_to_string(&manifest_path).unwrap();
    let loaded: WorkspaceState = serde_json::from_str(&content).unwrap();

    assert_eq!(loaded.tasks.len(), 1);
    assert_eq!(loaded.tasks[0].id, "task-1");
}

/// Test that load function reads tasks-manifest.json correctly
#[test]
fn test_workspace_state_load() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create a manifest file manually
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let manifest_path = ltmatrix_dir.join("tasks-manifest.json");
    let json_content = r#"{
        "project_root": "/test/project",
        "tasks": [
            {
                "id": "task-1",
                "title": "First task",
                "description": "Description 1",
                "status": "pending",
                "complexity": "moderate",
                "depends_on": [],
                "subtasks": [],
                "retry_count": 0,
                "created_at": "2024-01-01T00:00:00Z"
            }
        ],
        "metadata": {
            "version": "1.0",
            "created_at": "2024-01-01T00:00:00Z",
            "modified_at": "2024-01-01T01:00:00Z"
        }
    }"#;

    fs::write(&manifest_path, json_content).unwrap();

    // Load the state
    let result = WorkspaceState::load(project_root.to_path_buf());
    assert!(result.is_ok());

    let state = result.unwrap();
    assert_eq!(state.tasks.len(), 1);
    assert_eq!(state.tasks[0].id, "task-1");
}

/// Test that load returns error when manifest file doesn't exist
#[test]
fn test_workspace_state_load_missing_file() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Don't create manifest file
    let result = WorkspaceState::load(project_root.to_path_buf());
    assert!(result.is_err());
}

/// Test that state file path resolution works correctly
#[test]
fn test_workspace_state_manifest_path() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    let state = WorkspaceState::new(project_root.clone(), vec![]);

    // Get the manifest path
    let manifest_path = state.manifest_path();

    // Verify it's in .ltmatrix directory
    assert!(manifest_path.ends_with(".ltmatrix/tasks-manifest.json"));
    assert!(manifest_path.starts_with(&project_root));
}

/// Test that serialization preserves all task properties
#[test]
fn test_workspace_state_full_task_serialization() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    let mut task = Task::new("task-1", "Complex task", "A task with many properties");
    task.status = TaskStatus::Failed;
    task.complexity = ltmatrix::models::TaskComplexity::Complex;
    task.depends_on = vec!["dep-1".to_string(), "dep-2".to_string()];
    task.retry_count = 3;
    task.error = Some("Test error".to_string());

    let state = WorkspaceState::new(project_root, vec![task.clone()]);

    // Serialize and deserialize
    let json = serde_json::to_string(&state).unwrap();
    let loaded_state: WorkspaceState = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded_state.tasks.len(), 1);
    let loaded_task = &loaded_state.tasks[0];

    assert_eq!(loaded_task.id, task.id);
    assert_eq!(loaded_task.title, task.title);
    assert_eq!(loaded_task.description, task.description);
    assert_eq!(loaded_task.status, task.status);
    assert_eq!(loaded_task.complexity, task.complexity);
    assert_eq!(loaded_task.depends_on, task.depends_on);
    assert_eq!(loaded_task.retry_count, task.retry_count);
    assert_eq!(loaded_task.error, task.error);
}

/// Test that metadata is updated on save
#[test]
fn test_workspace_state_metadata_update() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    let state = WorkspaceState::new(project_root.clone(), vec![]);

    // Get initial metadata
    let initial_modified = state.metadata.modified_at;

    // Wait a bit to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Save the state
    let updated_state = state.save().unwrap();

    // Verify modified_at was updated
    assert!(updated_state.metadata.modified_at > initial_modified);
}

/// Test round-trip serialization: save then load
#[test]
fn test_workspace_state_round_trip() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    let mut task1 = Task::new("task-1", "First", "First task");
    task1.status = TaskStatus::Completed;

    let mut task2 = Task::new("task-2", "Second", "Second task");
    task2.status = TaskStatus::InProgress;
    task2.depends_on = vec!["task-1".to_string()];

    let original_state = WorkspaceState::new(project_root.clone(), vec![task1, task2]);

    // Save
    original_state.save().unwrap();

    // Load
    let loaded_state = WorkspaceState::load(project_root).unwrap();

    // Verify equality
    assert_eq!(loaded_state.tasks.len(), original_state.tasks.len());
    assert_eq!(loaded_state.tasks[0].id, original_state.tasks[0].id);
    assert_eq!(loaded_state.tasks[0].status, original_state.tasks[0].status);
    assert_eq!(loaded_state.tasks[1].id, original_state.tasks[1].id);
    assert_eq!(loaded_state.tasks[1].status, original_state.tasks[1].status);
    assert_eq!(
        loaded_state.tasks[1].depends_on,
        original_state.tasks[1].depends_on
    );
}
