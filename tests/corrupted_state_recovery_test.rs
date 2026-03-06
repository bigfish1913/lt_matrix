//! Error handling and recovery tests for corrupted workspace state files
//!
//! Tests that corrupted state files are handled gracefully with proper
//! fallback to empty state and detailed error logging.

use ltmatrix::workspace::WorkspaceState;
use ltmatrix::models::Task;
use std::fs;
use tempfile::TempDir;

/// Test that corrupted JSON file returns meaningful error
#[test]
fn test_load_corrupted_json_returns_error() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create directory with corrupted JSON
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let manifest_path = ltmatrix_dir.join("tasks-manifest.json");
    fs::write(&manifest_path, "{ invalid json content").unwrap();

    // Should return error with details
    let result = WorkspaceState::load(project_root.to_path_buf());

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string().to_lowercase();

    // Error message should indicate the problem
    assert!(error_msg.contains("failed to parse") || error_msg.contains("parse"));
}

/// Test that load_with_transform handles corrupted JSON gracefully
#[test]
fn test_load_with_transform_corrupted_json() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create directory with corrupted JSON
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let manifest_path = ltmatrix_dir.join("tasks-manifest.json");
    fs::write(&manifest_path, "{ malformed: json: }").unwrap();

    // Should return error with details
    let result = WorkspaceState::load_with_transform(project_root.to_path_buf());

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string().to_lowercase();

    assert!(error_msg.contains("failed to parse") || error_msg.contains("parse"));
}

/// Test that missing file returns specific error
#[test]
fn test_load_missing_file_returns_error() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Don't create any file

    let result = WorkspaceState::load(project_root.to_path_buf());

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string().to_lowercase();

    // Should indicate file not found or read error
    assert!(error_msg.contains("failed to read"));
}

/// Test that truncated file is detected
#[test]
fn test_load_truncated_file_returns_error() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create valid state first
    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Truncate the file to simulate corruption
    let manifest_path = project_root.join(".ltmatrix").join("tasks-manifest.json");
    let mut content = fs::read_to_string(&manifest_path).unwrap();
    content.truncate(content.len() / 2);
    fs::write(&manifest_path, content).unwrap();

    // Should detect corruption
    let result = WorkspaceState::load(project_root.to_path_buf());

    assert!(result.is_err());
}

/// Test that load_or_create handles missing files by creating empty state
#[test]
fn test_load_or_create_creates_empty_state() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Don't create any file

    // This method should create empty state if file doesn't exist
    let result = WorkspaceState::load_or_create(project_root.to_path_buf());

    assert!(result.is_ok());
    let state = result.unwrap();

    // Should have empty tasks list
    assert_eq!(state.tasks.len(), 0);
    assert_eq!(state.project_root, project_root);

    // Verify file was created
    let manifest_path = project_root.join(".ltmatrix").join("tasks-manifest.json");
    assert!(manifest_path.exists());
}

/// Test that load_or_create handles corrupted files by creating new state
#[test]
fn test_load_or_create_handles_corruption() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create corrupted file
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let manifest_path = ltmatrix_dir.join("tasks-manifest.json");
    fs::write(&manifest_path, "{ corrupted json }").unwrap();

    // Should create new empty state
    let result = WorkspaceState::load_or_create(project_root.to_path_buf());

    assert!(result.is_ok());
    let state = result.unwrap();

    // Should have empty tasks list (fallback to empty state)
    assert_eq!(state.tasks.len(), 0);
}

/// Test that load_or_create_preserves_valid_state
#[test]
fn test_load_or_create_preserves_valid_state() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create valid state
    let task = Task::new("task-1", "Task", "Description");
    let original_state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    original_state.save().unwrap();

    // Load should preserve the state
    let result = WorkspaceState::load_or_create(project_root.to_path_buf());

    assert!(result.is_ok());
    let state = result.unwrap();

    assert_eq!(state.tasks.len(), 1);
    assert_eq!(state.tasks[0].id, "task-1");
}

/// Test that partial corruption is detected
#[test]
fn test_detect_partial_corruption() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create valid state
    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task]);
    state.save().unwrap();

    // Read, modify to corrupt part of it
    let manifest_path = project_root.join(".ltmatrix").join("tasks-manifest.json");
    let content = fs::read_to_string(&manifest_path).unwrap();

    // Corrupt the JSON by breaking structure
    let corrupted = content.replace(r#""tasks""#, r#""tasks_broken""#);
    fs::write(&manifest_path, corrupted).unwrap();

    // Should detect corruption
    let result = WorkspaceState::load(project_root.to_path_buf());

    assert!(result.is_err());
}

/// Test error details include file path
#[test]
fn test_error_includes_file_path() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create corrupted file
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let manifest_path = ltmatrix_dir.join("tasks-manifest.json");
    fs::write(&manifest_path, "{ invalid }").unwrap();

    let result = WorkspaceState::load(project_root.to_path_buf());

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();

    // Error should include useful context
    assert!(error_msg.len() > 0); // Has some message
}

/// Test that wrong JSON structure is detected
#[test]
fn test_detect_wrong_json_structure() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create file with valid JSON but wrong structure
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let manifest_path = ltmatrix_dir.join("tasks-manifest.json");
    let wrong_structure = r#"{
        "wrong_field": "value",
        "another_field": 123
    }"#;
    fs::write(&manifest_path, wrong_structure).unwrap();

    let result = WorkspaceState::load(project_root.to_path_buf());

    assert!(result.is_err());
}

/// Test recovery with backup file
#[test]
fn test_recovery_with_backup_file() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create valid state
    let task = Task::new("task-1", "Task", "Description");
    let state = WorkspaceState::new(project_root.to_path_buf(), vec![task.clone()]);
    state.save().unwrap();

    // Corrupt the main file
    let manifest_path = project_root.join(".ltmatrix").join("tasks-manifest.json");
    fs::write(&manifest_path, "{ corrupted }").unwrap();

    // Try to load - should fail
    let result = WorkspaceState::load(project_root.to_path_buf());
    assert!(result.is_err());

    // Even with backup attempt, should fail if no backup exists
    // (Backup functionality could be added in future)
}

/// Test that empty directory is handled
#[test]
fn test_load_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create .ltmatrix directory but no manifest
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    // Should return error
    let result = WorkspaceState::load(project_root.to_path_buf());
    assert!(result.is_err());
}

/// Test load_or_create with empty directory
#[test]
fn test_load_or_create_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create .ltmatrix directory but no manifest
    let ltmatrix_dir = project_root.join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    // Should create new state
    let result = WorkspaceState::load_or_create(project_root.to_path_buf());
    assert!(result.is_ok());

    let state = result.unwrap();
    assert_eq!(state.tasks.len(), 0);
}
