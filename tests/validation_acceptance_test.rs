//! Acceptance tests for validation utilities task
//!
//! This test file verifies all acceptance criteria from the task description:
//! - Create src/validate/mod.rs for input validation
//! - Validate: goal strings (non-empty, reasonable length)
//! - Validate: task IDs (format, uniqueness)
//! - Validate: agent availability (command exists)
//! - Validate: git repository state
//! - Validate: file system permissions
//! - Provide clear error messages for validation failures

use ltmatrix::validate::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ============================================================================
// Acceptance Criterion 1: Module exists and is accessible
// ============================================================================

#[test]
fn test_validation_module_exists() {
    // This test verifies that the validation module is properly exported
    // and accessible from the ltmatrix crate

    // Test that all validation functions are callable
    let goal = "Build a REST API";
    let _ = validate_goal(goal);

    let task_id = "task-1";
    let _ = validate_task_id_format(task_id);

    let tasks = vec!["task-1".to_string()];
    let _ = validate_task_ids_unique(&tasks);

    let temp_dir = TempDir::new().unwrap();
    let _ = validate_file_permissions(temp_dir.path(), false);

    // If we can compile and call these functions, the module exists
}

// ============================================================================
// Acceptance Criterion 2: Goal string validation (non-empty, reasonable length)
// ============================================================================

#[test]
fn test_goal_validation_rejects_empty_strings() {
    // Verify that empty goals are rejected
    let result = validate_goal("");
    assert!(result.is_err(), "Empty goal should be rejected");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("empty") || error_msg.contains("provide"),
        "Error message should mention empty goal: {}",
        error_msg
    );
}

#[test]
fn test_goal_validation_rejects_whitespace_only() {
    // Verify that whitespace-only goals are rejected
    let test_cases = vec!["   ", "\t\t", "\n\n", " \t\n "];

    for goal in test_cases {
        let result = validate_goal(goal);
        assert!(
            result.is_err(),
            "Whitespace-only goal '{}' should be rejected",
            goal
        );
    }
}

#[test]
fn test_goal_validation_enforces_minimum_length() {
    // Verify that goals meet minimum length requirements
    // Minimum is 1 character (as defined by MIN_GOAL_LENGTH)

    // Single character goal should be valid
    let result = validate_goal("a");
    assert!(result.is_ok(), "Single character goal should be valid");

    // Empty string should fail (0 characters)
    let result = validate_goal("");
    assert!(result.is_err(), "Empty goal should fail minimum length check");
}

#[test]
fn test_goal_validation_enforces_maximum_length() {
    // Verify that goals don't exceed maximum length
    const MAX_GOAL_LENGTH: usize = 10_000;

    // Goal at maximum length should be valid
    let max_goal = "a".repeat(MAX_GOAL_LENGTH);
    let result = validate_goal(&max_goal);
    assert!(
        result.is_ok(),
        "Goal at maximum length ({}) should be valid",
        MAX_GOAL_LENGTH
    );

    // Goal exceeding maximum should fail
    let too_long_goal = "a".repeat(MAX_GOAL_LENGTH + 1);
    let result = validate_goal(&too_long_goal);
    assert!(result.is_err(), "Goal exceeding maximum length should be rejected");

    // Verify error message is clear about the limit
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("too long") || error_msg.contains("Maximum"),
        "Error should mention length limit: {}",
        error_msg
    );
    assert!(
        error_msg.contains("10001") || error_msg.contains("10,001"),
        "Error should mention actual length: {}",
        error_msg
    );
}

#[test]
fn test_goal_validation_accepts_reasonable_goals() {
    // Verify that normal, reasonable goals are accepted
    let valid_goals = vec![
        "Build a REST API",
        "Fix the bug in user registration",
        "Add authentication to the application",
        "Refactor the database layer for better performance",
        "Create unit tests for the payment module",
        "Implement OAuth2 login with Google and GitHub",
        "Add responsive design for mobile devices",
    ];

    for goal in valid_goals {
        let result = validate_goal(goal);
        assert!(
            result.is_ok(),
            "Valid goal '{}' should be accepted: {:?}",
            goal,
            result
        );
    }
}

// ============================================================================
// Acceptance Criterion 3: Task ID format validation
// ============================================================================

#[test]
fn test_task_id_format_accepts_valid_patterns() {
    // Verify that valid task ID patterns are accepted
    let valid_ids = vec![
        "task-1",
        "task-2",
        "task-42",
        "task-123",
        "task-1-1",
        "task-1-2",
        "task-2-3-4",
        "task-100-200-300",
    ];

    for task_id in valid_ids {
        let result = validate_task_id_format(task_id);
        assert!(
            result.is_ok(),
            "Valid task ID '{}' should be accepted: {:?}",
            task_id,
            result
        );
    }
}

#[test]
fn test_task_id_format_rejects_invalid_patterns() {
    // Verify that invalid task ID patterns are rejected
    let invalid_ids = vec![
        "",
        "task",
        "Task-1",
        "task_1",
        "task-",
        "task-1-",
        "task--1",
        "1-task",
        "task-abc",
        "task-1a",
    ];

    for task_id in invalid_ids {
        let result = validate_task_id_format(task_id);
        assert!(
            result.is_err(),
            "Invalid task ID '{}' should be rejected",
            task_id
        );

        // Verify error message is helpful (empty string has special handling)
        let error_msg = result.unwrap_err().to_string();
        if !task_id.is_empty() {
            assert!(
                error_msg.contains("Invalid") || error_msg.contains("pattern"),
                "Error for '{}' should mention format issue: {}",
                task_id,
                error_msg
            );
        }
    }
}

#[test]
fn test_task_id_format_provides_clear_error_messages() {
    // Verify that error messages for invalid task IDs are clear and helpful
    let result = validate_task_id_format("invalid-id");
    let error_msg = result.unwrap_err().to_string();

    // Error should mention the expected pattern
    assert!(
        error_msg.contains("task-<number>") || error_msg.contains("pattern"),
        "Error message should explain expected format: {}",
        error_msg
    );

    // Error should show examples
    assert!(
        error_msg.contains("task-1") || error_msg.contains("e.g.,"),
        "Error message should provide examples: {}",
        error_msg
    );
}

// ============================================================================
// Acceptance Criterion 4: Task ID uniqueness validation
// ============================================================================

#[test]
fn test_task_id_uniqueness_accepts_unique_ids() {
    // Verify that collections of unique task IDs are accepted
    let unique_collections = vec![
        vec!["task-1".to_string()],
        vec!["task-1".to_string(), "task-2".to_string()],
        vec![
            "task-1".to_string(),
            "task-2".to_string(),
            "task-3".to_string(),
        ],
        (1..=10).map(|i| format!("task-{}", i)).collect::<Vec<_>>(),
    ];

    for collection in unique_collections {
        let result = validate_task_ids_unique(&collection);
        assert!(
            result.is_ok(),
            "Collection of unique task IDs should be accepted: {:?}",
            collection
        );
    }
}

#[test]
fn test_task_id_uniqueness_detects_duplicates() {
    // Verify that duplicate task IDs are detected
    let duplicate_collections = vec![
        vec!["task-1".to_string(), "task-1".to_string()],
        vec![
            "task-1".to_string(),
            "task-2".to_string(),
            "task-1".to_string(),
        ],
        vec![
            "task-1".to_string(),
            "task-2".to_string(),
            "task-2".to_string(),
            "task-3".to_string(),
            "task-1".to_string(),
        ],
    ];

    for collection in duplicate_collections {
        let result = validate_task_ids_unique(&collection);
        assert!(
            result.is_err(),
            "Collection with duplicates should be rejected: {:?}",
            collection
        );

        // Verify error message mentions duplicates
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Duplicate") || error_msg.contains("unique"),
            "Error should mention duplicates: {}",
            error_msg
        );
    }
}

#[test]
fn test_task_id_uniqueness_provides_clear_error_messages() {
    // Verify that error messages list the duplicate IDs
    let collection = vec![
        "task-1".to_string(),
        "task-2".to_string(),
        "task-1".to_string(),
    ];

    let result = validate_task_ids_unique(&collection);
    let error_msg = result.unwrap_err().to_string();

    // Error should mention which IDs are duplicated
    assert!(
        error_msg.contains("task-1") || error_msg.contains("task-2"),
        "Error should list duplicate IDs: {}",
        error_msg
    );
}

// ============================================================================
// Acceptance Criterion 5: Agent availability validation
// ============================================================================

#[test]
fn test_agent_availability_detects_missing_commands() {
    // Verify that missing agent commands are detected
    let fake_commands = vec![
        "nonexistent_agent_xyz123",
        "fake_cli_command_456",
        "definitely_not_a_real_command_789",
    ];

    for command in fake_commands {
        let result = validate_agent_available(command);
        assert!(
            result.is_err(),
            "Nonexistent command '{}' should be detected as unavailable",
            command
        );

        // Verify error message provides helpful installation hints
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("not available") || error_msg.contains("install"),
            "Error for '{}' should provide installation hint: {}",
            command,
            error_msg
        );
    }
}

#[test]
fn test_agent_availability_known_agents_provide_hints() {
    // Verify that known agents provide specific installation instructions
    let known_agents = vec![
        ("claude", "Claude Code"),
        ("opencode", "OpenCode"),
        ("kimi-code", "KimiCode"),
        ("codex", "Codex"),
    ];

    for (agent, expected_hint) in known_agents {
        let result = validate_agent_available(agent);

        // Whether the agent is installed or not, if it fails we should get a hint
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains(expected_hint) || error_msg.contains("install"),
                "Error for '{}' should mention '{}': {}",
                agent,
                expected_hint,
                error_msg
            );
        }
    }
}

// ============================================================================
// Acceptance Criterion 6: Git repository state validation
// ============================================================================

#[test]
fn test_git_validation_accepts_valid_repository() {
    // Verify that valid git repositories are accepted
    let temp_dir = TempDir::new().unwrap();
    git2::Repository::init(temp_dir.path()).unwrap();

    let result = validate_git_repository(temp_dir.path());
    assert!(
        result.is_ok(),
        "Valid git repository should be accepted: {:?}",
        temp_dir.path()
    );
}

#[test]
fn test_git_validation_rejects_non_existent_path() {
    // Verify that non-existent paths are rejected
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("does_not_exist");

    let result = validate_git_repository(&nonexistent);
    assert!(result.is_err(), "Non-existent path should be rejected");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("does not exist") || error_msg.contains("not found"),
        "Error should mention path doesn't exist: {}",
        error_msg
    );
}

#[test]
fn test_git_validation_rejects_non_git_directory() {
    // Verify that directories without .git are rejected
    let temp_dir = TempDir::new().unwrap();
    // Don't initialize git - just a regular directory

    let result = validate_git_repository(temp_dir.path());
    assert!(result.is_err(), "Non-git directory should be rejected");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("git") || error_msg.contains(".git"),
        "Error should mention git repository: {}",
        error_msg
    );
}

#[test]
fn test_git_validation_rejects_file_instead_of_directory() {
    // Verify that files (not directories) are rejected
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("not_a_directory.txt");
    fs::write(&file_path, "content").unwrap();

    let result = validate_git_repository(&file_path);
    assert!(result.is_err(), "File should be rejected as git repository");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("directory") || error_msg.contains("not a directory"),
        "Error should mention it must be a directory: {}",
        error_msg
    );
}

#[test]
fn test_git_validation_rejects_bare_repository() {
    // Verify that bare repositories are rejected
    let temp_dir = TempDir::new().unwrap();
    let bare_repo_path = temp_dir.path().join("bare.git");
    git2::Repository::init_bare(&bare_repo_path).unwrap();

    let result = validate_git_repository(&bare_repo_path);
    assert!(result.is_err(), "Bare repository should be rejected");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("bare") || error_msg.contains("working directory"),
        "Error should mention bare repository: {}",
        error_msg
    );
}

// ============================================================================
// Acceptance Criterion 7: File system permissions validation
// ============================================================================

#[test]
fn test_file_permissions_validates_read_access() {
    // Verify that read permission validation works
    let temp_dir = TempDir::new().unwrap();

    // Directory should be readable
    let result = validate_file_permissions(temp_dir.path(), false);
    assert!(
        result.is_ok(),
        "Readable directory should pass validation"
    );

    // File should be readable
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "content").unwrap();
    let result = validate_file_permissions(&file_path, false);
    assert!(result.is_ok(), "Readable file should pass validation");
}

#[test]
fn test_file_permissions_validates_write_access() {
    // Verify that write permission validation works
    let temp_dir = TempDir::new().unwrap();

    // Directory should be writable
    let result = validate_file_permissions(temp_dir.path(), true);
    assert!(
        result.is_ok(),
        "Writable directory should pass validation"
    );

    // File should be writable
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "content").unwrap();
    let result = validate_file_permissions(&file_path, true);
    assert!(result.is_ok(), "Writable file should pass validation");
}

#[test]
fn test_file_permissions_rejects_non_existent_path() {
    // Verify that non-existent paths are rejected
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("does_not_exist");

    let result = validate_file_permissions(&nonexistent, false);
    assert!(result.is_err(), "Non-existent path should be rejected");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("does not exist") || error_msg.contains("not found"),
        "Error should mention path doesn't exist: {}",
        error_msg
    );
}

#[test]
fn test_file_permissions_cleans_up_test_files() {
    // Verify that permission test doesn't leave artifacts
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join(".ltmatrix_permission_test");

    assert!(!test_file.exists(), "Test file should not exist before validation");

    validate_file_permissions(temp_dir.path(), true).unwrap();

    assert!(
        !test_file.exists(),
        "Test file should be cleaned up after validation"
    );
}

// ============================================================================
// Acceptance Criterion 8: Clear error messages for all validation failures
// ============================================================================

#[test]
fn test_all_validation_functions_provide_clear_errors() {
    // Verify that all validation functions provide clear, actionable error messages

    // Goal validation errors
    let goal_errors = vec!["", "   ", "!!!"];
    for goal in goal_errors {
        let err = validate_goal(goal).unwrap_err();
        let msg = err.to_string();
        assert!(
            !msg.is_empty() && msg.len() > 10,
            "Error for goal '{}' should be descriptive: {}",
            goal,
            msg
        );
    }

    // Task ID format errors
    let task_id_errors = vec!["invalid", "task_", "Task-1"];
    for task_id in task_id_errors {
        let err = validate_task_id_format(task_id).unwrap_err();
        let msg = err.to_string();
        assert!(
            !msg.is_empty() && msg.len() > 15,
            "Error for task_id '{}' should be descriptive: {}",
            task_id,
            msg
        );
    }

    // Uniqueness errors
    let duplicate_tasks = vec!["task-1".to_string(), "task-1".to_string()];
    let err = validate_task_ids_unique(&duplicate_tasks).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Duplicate") || msg.contains("unique"),
        "Uniqueness error should mention duplicates: {}",
        msg
    );
}

#[test]
fn test_error_messages_are_actionable() {
    // Verify that error messages provide guidance on how to fix the issue

    // Empty goal - should say what to provide
    let err = validate_goal("").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("provide") || msg.contains("description"),
        "Empty goal error should be actionable: {}",
        msg
    );

    // Invalid task ID - should show expected format
    let err = validate_task_id_format("bad").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("task-") || msg.contains("pattern") || msg.contains("e.g.,"),
        "Task ID error should show expected format: {}",
        msg
    );

    // Too long goal - should mention the limit
    let long_goal = "a".repeat(10_001);
    let err = validate_goal(&long_goal).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("concise") || msg.contains("Maximum"),
        "Length error should provide guidance: {}",
        msg
    );

    // Git repository error - should suggest git init
    let temp_dir = TempDir::new().unwrap();
    let err = validate_git_repository(temp_dir.path()).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("git init") || msg.contains(".git"),
        "Git error should suggest initialization: {}",
        msg
    );
}

// ============================================================================
// Acceptance Criterion 9: Complete workflow validation
// ============================================================================

#[test]
fn test_complete_validation_workflow() {
    // Test a realistic workflow that uses multiple validation functions

    // 1. Validate the goal
    let goal = "Build a REST API with authentication and rate limiting";
    assert!(
        validate_goal(goal).is_ok(),
        "Valid goal should pass validation"
    );

    // 2. Validate task ID formats
    let task_ids = vec!["task-1", "task-2", "task-1-1", "task-1-2"];
    for task_id in task_ids {
        assert!(
            validate_task_id_format(task_id).is_ok(),
            "Valid task ID '{}' should pass",
            task_id
        );
    }

    // 3. Validate task ID uniqueness
    let unique_tasks = vec![
        "task-1".to_string(),
        "task-2".to_string(),
        "task-3".to_string(),
    ];
    assert!(
        validate_task_ids_unique(&unique_tasks).is_ok(),
        "Unique task IDs should pass"
    );

    // 4. Validate workspace
    let temp_dir = TempDir::new().unwrap();
    git2::Repository::init(temp_dir.path()).unwrap();
    assert!(
        validate_workspace(temp_dir.path()).is_ok(),
        "Valid workspace should pass"
    );

    // 5. Validate file permissions
    assert!(
        validate_file_permissions(temp_dir.path(), true).is_ok(),
        "Writable directory should pass"
    );
}

#[test]
fn test_validation_catches_all_invalid_inputs() {
    // Verify that validation properly rejects invalid inputs

    // Invalid goal
    assert!(validate_goal("").is_err(), "Empty goal should be rejected");
    assert!(validate_goal("!!!").is_err(), "Non-alphanumeric goal should be rejected");

    // Invalid task ID
    assert!(
        validate_task_id_format("invalid").is_err(),
        "Invalid task ID format should be rejected"
    );

    // Duplicate task IDs
    let duplicates = vec!["task-1".to_string(), "task-1".to_string()];
    assert!(
        validate_task_ids_unique(&duplicates).is_err(),
        "Duplicate task IDs should be rejected"
    );

    // Non-existent path
    let nonexistent = Path::new("/this/path/definitely/does/not/exist");
    assert!(
        validate_file_permissions(nonexistent, false).is_err(),
        "Non-existent path should be rejected"
    );
    assert!(
        validate_git_repository(nonexistent).is_err(),
        "Non-existent git repo should be rejected"
    );

    // Non-existent agent
    assert!(
        validate_agent_available("fake_command_xyz").is_err(),
        "Non-existent agent should be rejected"
    );
}

// ============================================================================
// Performance and reliability tests
// ============================================================================

#[test]
fn test_validation_performance_is_acceptable() {
    // Verify that validation operations complete quickly

    // Goal validation should be fast even for max length
    let max_goal = "a".repeat(10_000);
    let start = std::time::Instant::now();
    validate_goal(&max_goal).unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 100,
        "Goal validation should complete in <100ms, took {}ms",
        elapsed.as_millis()
    );

    // Task ID uniqueness check should be fast even for many tasks
    let many_tasks: Vec<String> = (1..=1000).map(|i| format!("task-{}", i)).collect();
    let start = std::time::Instant::now();
    validate_task_ids_unique(&many_tasks).unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 100,
        "Uniqueness check should complete in <100ms, took {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn test_validation_does_not_have_side_effects() {
    // Verify that validation functions don't have unintended side effects

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "original content").unwrap();

    // Validation should not modify the file
    validate_file_permissions(&file_path, true).unwrap();
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(
        content, "original content",
        "Validation should not modify file content"
    );

    // Validation should not create unexpected files
    let entries: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name())
        .collect();

    assert!(
        !entries.iter().any(|n| n.to_string_lossy().contains("permission_test")),
        "Validation should clean up test files"
    );
}
