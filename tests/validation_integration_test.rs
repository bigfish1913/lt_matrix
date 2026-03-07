//! Integration tests for validation utilities

use ltmatrix::validate::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_validate_goal_integration() {
    // Valid goals
    assert!(validate_goal("Build a REST API with authentication").is_ok());
    assert!(validate_goal("Fix the bug in user registration").is_ok());
    assert!(validate_goal("Create a web scraper for news sites").is_ok());

    // Invalid goals
    assert!(validate_goal("").is_err());
    assert!(validate_goal("   ").is_err());
    assert!(validate_goal("!!!").is_err());

    // Too long
    let long_goal = "a".repeat(11_000);
    assert!(validate_goal(&long_goal).is_err());
}

#[test]
fn test_validate_task_ids_integration() {
    // Valid formats
    assert!(validate_task_id_format("task-1").is_ok());
    assert!(validate_task_id_format("task-42").is_ok());
    assert!(validate_task_id_format("task-1-1").is_ok());
    assert!(validate_task_id_format("task-1-2-3").is_ok());

    // Invalid formats
    assert!(validate_task_id_format("").is_err());
    assert!(validate_task_id_format("task").is_err());
    assert!(validate_task_id_format("task-").is_err());
    assert!(validate_task_id_format("Task-1").is_err());
    assert!(validate_task_id_format("task_1").is_err());

    // Uniqueness check
    let unique_ids = vec![
        "task-1".to_string(),
        "task-2".to_string(),
        "task-3".to_string(),
    ];
    assert!(validate_task_ids_unique(&unique_ids).is_ok());

    let duplicate_ids = vec![
        "task-1".to_string(),
        "task-2".to_string(),
        "task-1".to_string(),
    ];
    assert!(validate_task_ids_unique(&duplicate_ids).is_err());
}

#[test]
fn test_validate_git_repository_integration() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Not a git repository
    assert!(validate_git_repository(repo_path).is_err());

    // Initialize git repository
    git2::Repository::init(repo_path).unwrap();
    assert!(validate_git_repository(repo_path).is_ok());

    // Test with file instead of directory
    let file_path = repo_path.join("test.txt");
    fs::write(&file_path, "content").unwrap();
    assert!(validate_git_repository(&file_path).is_err());
}

#[test]
fn test_validate_file_permissions_integration() {
    let temp_dir = TempDir::new().unwrap();

    // Directory with read/write permissions
    assert!(validate_file_permissions(temp_dir.path(), false).is_ok());
    assert!(validate_file_permissions(temp_dir.path(), true).is_ok());

    // Create a file and test permissions
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "content").unwrap();

    assert!(validate_file_permissions(&file_path, false).is_ok());
    assert!(validate_file_permissions(&file_path, true).is_ok());

    // Nonexistent path
    assert!(
        validate_file_permissions(&temp_dir.path().join("nonexistent"), false).is_err()
    );
}

#[test]
fn test_validate_directory_creatable_integration() {
    let temp_dir = TempDir::new().unwrap();

    // Can create a new directory
    let new_dir = temp_dir.path().join("new_directory");
    assert!(validate_directory_creatable(&new_dir).is_ok());

    // Directory already exists (should check write permissions)
    assert!(validate_directory_creatable(temp_dir.path()).is_ok());

    // Parent doesn't exist
    let invalid_path = temp_dir.path().join("nonexistent").join("subdir");
    assert!(validate_directory_creatable(&invalid_path).is_err());
}

#[test]
fn test_validate_workspace_integration() {
    let temp_dir = TempDir::new().unwrap();

    // Workspace without git repository (should warn but not fail)
    assert!(validate_workspace(temp_dir.path()).is_ok());

    // Workspace with git repository
    git2::Repository::init(temp_dir.path()).unwrap();
    assert!(validate_workspace(temp_dir.path()).is_ok());
}

#[test]
fn test_validate_agent_available_integration() {
    // These commands are unlikely to exist
    assert!(
        validate_agent_available("nonexistent_agent_command_xyz123_abc").is_err()
    );
    assert!(validate_agent_available("another_fake_command_456").is_err());

    // The error message should include installation hints
    let result = validate_agent_available("claude");
    // This might pass or fail depending on the system, but should not panic
    match result {
        Ok(_) => println!("Claude CLI is available"),
        Err(e) => {
            let error_msg = e.to_string();
            assert!(error_msg.contains("Claude Code") || error_msg.contains("not available"));
        }
    }
}

#[test]
fn test_error_messages_are_user_friendly() {
    // Goal validation
    let err = validate_goal("").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("empty") || msg.contains("provide"));

    let err = validate_goal("!!!").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("alphanumeric"));

    // Task ID validation
    let err = validate_task_id_format("invalid").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Invalid task ID format") || msg.contains("pattern"));

    // Git repository validation
    let temp_dir = TempDir::new().unwrap();
    let err = validate_git_repository(temp_dir.path()).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("git") || msg.contains("repository"));
}

#[test]
fn test_combined_validation_workflow() {
    // Simulate a realistic validation workflow

    // 1. Validate goal
    let goal = "Build a REST API with authentication";
    assert!(validate_goal(goal).is_ok());

    // 2. Validate task IDs
    let tasks = vec![
        "task-1".to_string(),
        "task-2".to_string(),
        "task-3".to_string(),
    ];
    for task_id in &tasks {
        assert!(validate_task_id_format(task_id).is_ok());
    }
    assert!(validate_task_ids_unique(&tasks).is_ok());

    // 3. Validate workspace
    let temp_dir = TempDir::new().unwrap();
    git2::Repository::init(temp_dir.path()).unwrap();
    assert!(validate_workspace(temp_dir.path()).is_ok());

    // 4. Validate file permissions
    assert!(validate_file_permissions(temp_dir.path(), true).is_ok());

    // All validations passed - ready to proceed
}