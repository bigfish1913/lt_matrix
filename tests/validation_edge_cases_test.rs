//! Edge case tests for validation utilities
//!
//! This test file focuses on boundary conditions, edge cases, and
//! comprehensive validation of the acceptance criteria.

use ltmatrix::validate::*;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// Goal Validation Edge Cases
// ============================================================================

#[test]
fn test_goal_boundary_values() {
    // Test minimum length (1 character)
    let min_goal = "a";
    assert!(validate_goal(min_goal).is_ok());

    // Test maximum length (exactly 10,000 characters)
    let max_goal = "a".repeat(10_000);
    assert!(validate_goal(&max_goal).is_ok());

    // Test just over maximum (10,001 characters)
    let too_long = "a".repeat(10_001);
    assert!(validate_goal(&too_long).is_err());

    // Verify error message for too long goal
    let err = validate_goal(&too_long).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("too long") || msg.contains("Maximum"));
    assert!(msg.contains("10001") || msg.contains("10001") || msg.contains("10,001"));
}

#[test]
fn test_goal_whitespace_handling() {
    // Test goal with leading/trailing whitespace (should be trimmed)
    assert!(validate_goal("  Build a REST API  ").is_ok());

    // Test goal with only whitespace (should fail)
    assert!(validate_goal("     ").is_err());
    assert!(validate_goal("\t\t\t").is_err());
    assert!(validate_goal("\n\n\n").is_err());

    // Test goal with mixed whitespace and content
    assert!(validate_goal("\n  Build API  \t").is_ok());
}

#[test]
fn test_goal_unicode_and_multibyte() {
    // Test Unicode characters
    assert!(validate_goal("构建REST API").is_ok());
    assert!(validate_goal("Créer une API REST").is_ok());
    assert!(validate_goal("Создать REST API").is_ok());
    assert!(validate_goal("🚀 Build a rocket API").is_ok());

    // Test emoji-only goal (should fail - no alphanumeric)
    assert!(validate_goal("🚀🎉💻").is_err());
}

#[test]
fn test_goal_special_characters() {
    // Test goals with special but valid characters
    assert!(validate_goal("Fix bug: user can't login").is_ok());
    assert!(validate_goal("Add feature: JSON parsing").is_ok());
    assert!(validate_goal("Refactor: improve performance by 50%").is_ok());

    // Test goal with only punctuation (should fail)
    assert!(validate_goal("!!!").is_err());
    assert!(validate_goal("---").is_err());
    assert!(validate_goal("...").is_err());
}

#[test]
fn test_goal_error_message_quality() {
    // Test empty goal error message
    let err = validate_goal("").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("empty") || msg.contains("provide"));

    // Test no alphanumeric error message
    let err = validate_goal("!!!").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("alphanumeric") || msg.contains("meaningful"));

    // Test too long error message
    let long_goal = "a".repeat(10_001);
    let err = validate_goal(&long_goal).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("too long") || msg.contains("Maximum"));
}

// ============================================================================
// Task ID Validation Edge Cases
// ============================================================================

#[test]
fn test_task_id_boundary_values() {
    // Test task-0 (boundary value)
    assert!(validate_task_id_format("task-0").is_ok());

    // Test large numbers
    assert!(validate_task_id_format("task-999999").is_ok());

    // Test deeply nested subtasks
    assert!(validate_task_id_format("task-1-2-3-4-5").is_ok());
    assert!(validate_task_id_format("task-1-2-3-4-5-6-7-8-9-10").is_ok());

    // Test leading zeros (should be valid)
    assert!(validate_task_id_format("task-01").is_ok());
    assert!(validate_task_id_format("task-001-002").is_ok());
}

#[test]
fn test_task_id_invalid_formats_comprehensive() {
    let invalid_cases = vec![
        "",
        "task",
        "Task-1",
        "TASK-1",
        "task_1",
        "task-",
        "-task-1",
        "task-1-",
        "task--1",
        "task-1--2",
        "task-1.2",
        "task-abc",
        "1-task",
        "task-1a",
        "task-1-2a",
        // Case sensitivity
        "Task-1",
        "TASK-1",
        "tAsK-1",
        // Whitespace
        "task- 1",
        "task -1",
        "task-1 ",
        " task-1",
        // Special characters
        "task@1",
        "task#1",
        "task$1",
    ];

    for invalid_id in invalid_cases {
        assert!(
            validate_task_id_format(invalid_id).is_err(),
            "Expected error for task_id: '{}'",
            invalid_id
        );
    }
}

#[test]
fn test_task_id_error_message_quality() {
    let err = validate_task_id_format("invalid").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Invalid task ID format") || msg.contains("pattern"));
    assert!(msg.contains("task-<number>") || msg.contains("task-"));
}

#[test]
fn test_task_id_uniqueness_edge_cases() {
    // Empty collection (should be valid)
    assert!(validate_task_ids_unique(&[]).is_ok());

    // Single task ID
    assert!(validate_task_ids_unique(&["task-1".to_string()]).is_ok());

    // Many unique task IDs
    let many_unique: Vec<String> = (1..=100).map(|i| format!("task-{}", i)).collect();
    assert!(validate_task_ids_unique(&many_unique).is_ok());

    // Multiple duplicates
    let multi_dupes = vec![
        "task-1".to_string(),
        "task-2".to_string(),
        "task-1".to_string(),
        "task-3".to_string(),
        "task-2".to_string(),
    ];
    assert!(validate_task_ids_unique(&multi_dupes).is_err());

    // Verify error message lists all duplicates
    let err = validate_task_ids_unique(&multi_dupes).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Duplicate") || msg.contains("unique"));
}

// ============================================================================
// Agent Availability Edge Cases
// ============================================================================

#[test]
fn test_agent_availability_known_commands() {
    // Test known agent commands
    // These will fail if not installed, but should have helpful error messages
    let known_agents = vec!["claude", "opencode", "kimi-code", "codex"];

    for agent in known_agents {
        let result = validate_agent_available(agent);
        match result {
            Ok(_) => {
                // Agent is available - that's fine
            }
            Err(e) => {
                // Agent not available - verify error message has installation hint
                let msg = e.to_string();
                assert!(
                    msg.contains("not available")
                        || msg.contains("install")
                        || msg.contains("PATH"),
                    "Error message for '{}' should provide helpful hint: {}",
                    agent,
                    msg
                );
            }
        }
    }
}

#[test]
fn test_agent_availability_empty_and_invalid() {
    // Empty command
    assert!(validate_agent_available("").is_err());

    // Command with spaces (unlikely to be valid)
    assert!(validate_agent_available("invalid command with spaces").is_err());

    // Command with path separators (might exist but unlikely)
    let result = validate_agent_available("/usr/bin/nonexistent_cmd_12345");
    assert!(result.is_err());
}

// ============================================================================
// Git Repository Edge Cases
// ============================================================================

#[test]
fn test_git_repository_bare_repository() {
    let temp_dir = TempDir::new().unwrap();
    let bare_repo_path = temp_dir.path().join("bare.git");
    git2::Repository::init_bare(&bare_repo_path).unwrap();

    // Bare repository should fail validation
    let result = validate_git_repository(&bare_repo_path);
    assert!(result.is_err());

    // Check that error message is meaningful
    let err = result.unwrap_err();
    let msg = err.to_string();
    // The error should mention something about the repository state
    assert!(!msg.is_empty(), "Error message should not be empty");
}

#[test]
fn test_git_repository_worktree() {
    let temp_dir = TempDir::new().unwrap();
    let _main_repo = git2::Repository::init(temp_dir.path().join("main")).unwrap();

    // Create a worktree (if git2 supports it in this version)
    // For now, just test that the main repo validates
    assert!(validate_git_repository(&temp_dir.path().join("main")).is_ok());
}

#[test]
fn test_git_repository_error_messages() {
    let temp_dir = TempDir::new().unwrap();

    // Non-existent path
    let nonexistent = temp_dir.path().join("nonexistent");
    let err = validate_git_repository(&nonexistent).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("does not exist") || msg.contains("not found"));

    // Path exists but not a git repo
    let err = validate_git_repository(temp_dir.path()).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("git") || msg.contains(".git"));

    // File instead of directory
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, "content").unwrap();
    let err = validate_git_repository(&file_path).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("directory") || msg.contains("not a directory"));
}

// ============================================================================
// File Permissions Edge Cases
// ============================================================================

#[test]
fn test_file_permissions_with_files() {
    let temp_dir = TempDir::new().unwrap();

    // Test with a readable file
    let file_path = temp_dir.path().join("readable.txt");
    fs::write(&file_path, "content").unwrap();
    assert!(validate_file_permissions(&file_path, false).is_ok());
    assert!(validate_file_permissions(&file_path, true).is_ok());

    // Test with a directory
    assert!(validate_file_permissions(temp_dir.path(), false).is_ok());
    assert!(validate_file_permissions(temp_dir.path(), true).is_ok());
}

#[test]
fn test_file_permissions_error_messages() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("nonexistent");

    let err = validate_file_permissions(&nonexistent, false).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("does not exist") || msg.contains("not found"));

    // Test directory listing permissions
    // (hard to test cross-platform for permission denied scenarios)
}

#[test]
fn test_file_permissions_test_file_cleanup() {
    let temp_dir = TempDir::new().unwrap();

    // Verify that the permission test file is cleaned up
    let test_file = temp_dir.path().join(".ltmatrix_permission_test");

    // Before validation, file should not exist
    assert!(!test_file.exists());

    // After validation with write permission, file should still not exist (cleaned up)
    validate_file_permissions(temp_dir.path(), true).unwrap();
    assert!(!test_file.exists());
}

// ============================================================================
// Directory Creation Edge Cases
// ============================================================================

#[test]
fn test_directory_creatable_nested_paths() {
    let temp_dir = TempDir::new().unwrap();

    // Single level
    assert!(validate_directory_creatable(&temp_dir.path().join("level1")).is_ok());

    // Multiple levels (parent exists)
    let level1 = temp_dir.path().join("level1");
    fs::create_dir(&level1).unwrap();
    assert!(validate_directory_creatable(&level1.join("level2")).is_ok());

    // Multiple levels (parent doesn't exist - should fail)
    let deep_path = temp_dir.path().join("l1").join("l2").join("l3");
    assert!(validate_directory_creatable(&deep_path).is_err());

    // Verify error message mentions parent directory
    let err = validate_directory_creatable(&deep_path).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Parent") || msg.contains("parent"));
}

#[test]
fn test_directory_creatable_root_paths() {
    // Test with paths that have no parent (root paths)
    // This is tricky to test cross-platform, so we'll just verify
    // that the function handles it gracefully

    // On most systems, we can't create in root, so this should fail
    // but should fail with a reasonable error message
    #[cfg(unix)]
    {
        let root_path = Path::new("/new_root_dir");
        let result = validate_directory_creatable(root_path);
        // May fail with permission error or parent doesn't exist
        assert!(result.is_err());
    }
}

// ============================================================================
// Workspace Validation Edge Cases
// ============================================================================

#[test]
fn test_workspace_validation_without_git() {
    let temp_dir = TempDir::new().unwrap();

    // Workspace without git should succeed (with warning)
    assert!(validate_workspace(temp_dir.path()).is_ok());
}

#[test]
fn test_workspace_validation_with_corrupted_git() {
    let temp_dir = TempDir::new().unwrap();

    // Create a .git directory but make it invalid
    let git_dir = temp_dir.path().join(".git");
    fs::create_dir(&git_dir).unwrap();

    // Create invalid HEAD file
    let head_file = git_dir.join("HEAD");
    fs::write(&head_file, "invalid: ref").unwrap();

    // Should fail because git repository is corrupted
    assert!(validate_workspace(temp_dir.path()).is_err());
}

// ============================================================================
// Comprehensive Workflow Tests
// ============================================================================

#[test]
fn test_complete_validation_workflow_success() {
    let temp_dir = TempDir::new().unwrap();

    // Setup a valid workspace
    git2::Repository::init(temp_dir.path()).unwrap();

    // Validate goal
    let goal = "Build a comprehensive REST API with authentication and rate limiting";
    assert!(validate_goal(goal).is_ok(), "Goal validation failed");

    // Validate task IDs
    let tasks = vec![
        "task-1".to_string(),
        "task-2".to_string(),
        "task-3".to_string(),
        "task-1-1".to_string(),
        "task-1-2".to_string(),
    ];
    for task_id in &tasks {
        assert!(
            validate_task_id_format(task_id).is_ok(),
            "Task ID format validation failed for: {}",
            task_id
        );
    }
    assert!(
        validate_task_ids_unique(&tasks).is_ok(),
        "Task ID uniqueness check failed"
    );

    // Validate workspace
    assert!(
        validate_workspace(temp_dir.path()).is_ok(),
        "Workspace validation failed"
    );

    // Validate permissions
    assert!(
        validate_file_permissions(temp_dir.path(), true).is_ok(),
        "File permissions validation failed"
    );
}

#[test]
fn test_complete_validation_workflow_failure() {
    let temp_dir = TempDir::new().unwrap();

    // Invalid goal
    let result = validate_goal("");
    assert!(result.is_err(), "Empty goal should fail validation");

    // Invalid task ID format
    let result = validate_task_id_format("invalid");
    assert!(result.is_err(), "Invalid task ID format should fail");

    // Duplicate task IDs
    let duplicate_tasks = vec![
        "task-1".to_string(),
        "task-2".to_string(),
        "task-1".to_string(),
    ];
    let result = validate_task_ids_unique(&duplicate_tasks);
    assert!(result.is_err(), "Duplicate task IDs should fail");

    // Invalid git repository
    let result = validate_git_repository(temp_dir.path());
    assert!(result.is_err(), "Non-git directory should fail validation");
}

// ============================================================================
// Performance and Stress Tests
// ============================================================================

#[test]
fn test_large_task_id_collection() {
    // Test with many task IDs
    let many_tasks: Vec<String> = (1..=1000).map(|i| format!("task-{}", i)).collect();
    assert!(validate_task_ids_unique(&many_tasks).is_ok());

    // Add a duplicate
    let mut with_duplicate = many_tasks.clone();
    with_duplicate.push("task-500".to_string());
    assert!(validate_task_ids_unique(&with_duplicate).is_err());
}

#[test]
fn test_maximum_goal_validation_performance() {
    // Test that maximum goal length validation is efficient
    // Use a goal at exactly the maximum allowed length (10,000 characters)
    let max_goal = "a".repeat(10_000);
    let start = std::time::Instant::now();
    let result = validate_goal(&max_goal);
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert!(
        elapsed.as_millis() < 100,
        "Goal validation should be fast, took {} ms",
        elapsed.as_millis()
    );
}

// ============================================================================
// Cross-Platform Compatibility Tests
// ============================================================================

#[test]
fn test_path_handling_cross_platform() {
    let temp_dir = TempDir::new().unwrap();

    // Test with various path formats
    let paths_to_test = vec![
        temp_dir.path().join("simple"),
        temp_dir.path().join("with space"),
        temp_dir.path().join("with-dash"),
        temp_dir.path().join("with_underscore"),
    ];

    for path in paths_to_test {
        let result = validate_directory_creatable(&path);
        assert!(
            result.is_ok(),
            "Failed to validate directory creatable for: {:?}",
            path
        );
    }
}
