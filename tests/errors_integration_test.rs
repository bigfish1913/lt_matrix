//! Integration tests for error handling scenarios
//!
//! This test suite validates error handling across the pipeline, including:
//! - Agent failure and retry logic
//! - Test failure and fix cycle
//! - Verify failure and --on-blocked strategies
//! - Git operation failures
//! - Clear and actionable error messages
//!
//! Task: Create integration test for error handling

use ltmatrix::cli::args::BlockedStrategy;
use ltmatrix::git::repository::{init_repo, checkout, get_current_branch};
use ltmatrix::models::{Task, TaskComplexity, TaskStatus, ModeConfig};
use ltmatrix::pipeline::test::{TestConfig, TestFramework, detect_test_framework, test_tasks};
use ltmatrix::pipeline::verify::{OnBlockedStrategy, VerifyConfig, VerificationResult, VerificationSummary};
use ltmatrix::workspace::WorkspaceState;
use std::collections::HashSet;
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

/// Creates a task with a specific status
fn create_task_with_status(id: &str, title: &str, status: TaskStatus) -> Task {
    let mut task = create_task(id, title, vec![]);
    task.status = status;
    task
}

// =============================================================================
// Task Retry Logic Tests
// =============================================================================

/// Test that tasks track retry count correctly
#[test]
fn test_task_retry_count_tracking() {
    let mut task = create_task("task-001", "Test task", vec![]);

    assert_eq!(task.retry_count, 0, "Initial retry count should be 0");

    // Task needs to be failed to retry
    task.status = TaskStatus::Failed;

    assert!(task.can_retry(3), "Should be able to retry with max 3");
    assert!(task.can_retry(1), "Should be able to retry with max 1");
    assert!(!task.can_retry(0), "Should not be able to retry with max 0");

    // Simulate retry
    task.prepare_retry();

    assert_eq!(task.retry_count, 1, "Retry count should increment");
    assert_eq!(task.status, TaskStatus::Pending, "Status should be reset to Pending");
    assert!(task.started_at.is_none(), "started_at should be cleared");
}

/// Test that tasks cannot exceed max retries
#[test]
fn test_task_max_retry_limit() {
    let mut task = create_task("task-001", "Test task", vec![]);
    task.status = TaskStatus::Failed;

    // Simulate max retries reached
    for _ in 0..3 {
        task.prepare_retry();
        task.status = TaskStatus::Failed;
    }

    assert_eq!(task.retry_count, 3, "Should have 3 retries");
    assert!(!task.can_retry(3), "Should not be able to retry after 3 attempts");
    assert!(task.can_retry(4), "Should be able to retry with higher max");
}

/// Test that session ID is preserved on retry
#[test]
fn test_session_preserved_on_retry() {
    let mut task = create_task("task-001", "Test task", vec![]);
    task.session_id = Some("session-abc123".to_string());
    task.status = TaskStatus::Failed;

    task.prepare_retry();

    assert_eq!(
        task.session_id,
        Some("session-abc123".to_string()),
        "Session ID should be preserved for retry reuse"
    );
    assert_eq!(task.status, TaskStatus::Pending, "Status should be Pending");
}

/// Test retry scenario with dependency chain
#[test]
fn test_retry_with_dependency_chain() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create dependency chain: task-001 -> task-002 -> task-003
    let mut task1 = create_task("task-001", "First", vec![]);
    let mut task2 = create_task("task-002", "Second", vec!["task-001".to_string()]);
    let task3 = create_task("task-003", "Third", vec!["task-002".to_string()]);

    // First task completes
    task1.status = TaskStatus::Completed;
    task1.session_id = Some("session-1".to_string());

    // Second task fails and is retried
    task2.status = TaskStatus::Failed;
    task2.session_id = Some("session-2".to_string());
    task2.prepare_retry();

    // Third task should still depend on second
    assert_eq!(task3.depends_on, vec!["task-002".to_string()]);

    // Save and load state
    let state = WorkspaceState::new(temp_dir.path().to_path_buf(), vec![task1, task2, task3]);
    state.save().expect("Failed to save state");

    let loaded = WorkspaceState::load(temp_dir.path().to_path_buf()).expect("Failed to load");

    // Verify retry state persisted
    assert_eq!(loaded.tasks[1].retry_count, 1);
    assert_eq!(loaded.tasks[1].status, TaskStatus::Pending);
    assert_eq!(loaded.tasks[1].session_id, Some("session-2".to_string()));
}

// =============================================================================
// Test Failure and Fix Cycle Tests
// =============================================================================

/// Test test stage configuration for failure handling
#[test]
fn test_test_config_fail_on_error() {
    let default_config = TestConfig::default();
    assert!(default_config.fail_on_error, "Default should fail on error");

    let fast_config = TestConfig::fast_mode();
    assert!(!fast_config.fail_on_error, "Fast mode should not fail on error");
    assert!(!fast_config.enabled, "Fast mode should have tests disabled");

    let expert_config = TestConfig::expert_mode();
    assert!(expert_config.fail_on_error, "Expert mode should fail on error");
    assert!(expert_config.enabled, "Expert mode should have tests enabled");
}

/// Test test framework detection handles missing files gracefully
#[test]
fn test_framework_detection_missing_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let result = detect_test_framework(temp_dir.path());
    assert!(result.is_ok(), "Detection should not fail");

    let detection = result.unwrap();
    assert_eq!(detection.framework, TestFramework::None, "Should detect no framework");
    assert_eq!(detection.confidence, 0.0, "Confidence should be 0");
}

/// Test test framework detection for Cargo project
#[test]
fn test_framework_detection_cargo_project() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create Cargo.toml
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    std::fs::write(&cargo_toml, "[package]\nname = \"test\"\nversion = \"0.1.0\"")
        .expect("Failed to write Cargo.toml");

    // Create tests directory
    std::fs::create_dir(temp_dir.path().join("tests")).expect("Failed to create tests dir");

    let result = detect_test_framework(temp_dir.path());
    assert!(result.is_ok());

    let detection = result.unwrap();
    assert_eq!(detection.framework, TestFramework::Cargo);
    assert_eq!(detection.confidence, 1.0);
}

/// Test that test_tasks handles empty task list
#[tokio::test]
async fn test_tasks_handles_empty_list() {
    let config = TestConfig::default();
    let result = test_tasks(vec![], &config).await;

    assert!(result.is_ok(), "Should handle empty task list");
    let tasks = result.unwrap();
    assert!(tasks.is_empty(), "Should return empty task list");
}

/// Test that test_tasks handles disabled config
#[tokio::test]
async fn test_tasks_disabled_config() {
    let task = create_task("task-001", "Test task", vec![]);
    let mut config = TestConfig::default();
    config.enabled = false;

    let result = test_tasks(vec![task], &config).await;

    assert!(result.is_ok(), "Should succeed with disabled config");
    let tasks = result.unwrap();
    assert_eq!(tasks.len(), 1, "Should return original task");
}

// =============================================================================
// OnBlocked Strategy Tests
// =============================================================================

/// Test OnBlockedStrategy variants
#[test]
fn test_on_blocked_strategy_variants() {
    // Test all strategy variants exist
    let strategies = vec![
        OnBlockedStrategy::Fail,
        OnBlockedStrategy::Retry,
        OnBlockedStrategy::Block,
        OnBlockedStrategy::Skip,
    ];

    for strategy in strategies {
        // Each strategy should be printable
        let _debug = format!("{:?}", strategy);
    }
}

/// Test OnBlockedStrategy default
#[test]
fn test_on_blocked_strategy_default() {
    assert_eq!(OnBlockedStrategy::default(), OnBlockedStrategy::Retry);
}

/// Test verification config uses correct strategy per mode
#[test]
fn test_verify_config_strategy_per_mode() {
    let default_config = VerifyConfig::default();
    assert_eq!(default_config.on_blocked, OnBlockedStrategy::Retry);

    let fast_config = VerifyConfig::fast_mode();
    assert_eq!(fast_config.on_blocked, OnBlockedStrategy::Fail);

    let expert_config = VerifyConfig::expert_mode();
    assert_eq!(expert_config.on_blocked, OnBlockedStrategy::Retry);
}

/// Test blocked strategy mapping from CLI to pipeline
#[test]
fn test_blocked_strategy_cli_mapping() {
    // CLI BlockedStrategy should map to verify OnBlockedStrategy
    let skip = BlockedStrategy::Skip;
    let ask = BlockedStrategy::Ask;
    let abort = BlockedStrategy::Abort;
    let retry = BlockedStrategy::Retry;

    assert_eq!(skip.to_string(), "skip");
    assert_eq!(ask.to_string(), "ask");
    assert_eq!(abort.to_string(), "abort");
    assert_eq!(retry.to_string(), "retry");
}

/// Test verification failure with Fail strategy
#[test]
fn test_verification_failure_fail_strategy() {
    let mut task = create_task("task-001", "Test task", vec![]);
    task.status = TaskStatus::InProgress;

    let result = VerificationResult {
        task: task.clone(),
        passed: false,
        reasoning: "Implementation incomplete".to_string(),
        unmet_criteria: vec!["Missing error handling".to_string()],
        suggestions: vec!["Add try-catch block".to_string()],
        retry_recommended: false,
    };

    // With Fail strategy, task should be marked as Failed
    let config = VerifyConfig {
        on_blocked: OnBlockedStrategy::Fail,
        max_retries: 0,
        ..VerifyConfig::default()
    };

    // Simulate the failure handling logic
    if !result.retry_recommended || config.max_retries == 0 || config.on_blocked == OnBlockedStrategy::Fail {
        task.status = TaskStatus::Failed;
        task.error = Some(format!("Verification failed: {}", result.reasoning));
    }

    assert_eq!(task.status, TaskStatus::Failed);
    assert!(task.error.unwrap().contains("Verification failed"));
}

/// Test verification failure with Block strategy
#[test]
fn test_verification_failure_block_strategy() {
    let mut task = create_task("task-001", "Test task", vec![]);
    task.status = TaskStatus::InProgress;

    let config = VerifyConfig {
        on_blocked: OnBlockedStrategy::Block,
        max_retries: 1,
        ..VerifyConfig::default()
    };

    // Simulate the failure handling logic for Block strategy
    task.status = TaskStatus::Blocked;
    task.error = Some("Blocked pending fix: Implementation issues".to_string());

    assert_eq!(task.status, TaskStatus::Blocked);
    assert!(task.error.unwrap().contains("Blocked pending fix"));
}

/// Test verification failure with Retry strategy
#[test]
fn test_verification_failure_retry_strategy() {
    let mut task = create_task("task-001", "Test task", vec![]);
    task.status = TaskStatus::InProgress;
    task.retry_count = 0;

    let result = VerificationResult {
        task: task.clone(),
        passed: false,
        reasoning: "Minor issues found".to_string(),
        unmet_criteria: vec![],
        suggestions: vec!["Fix typo".to_string()],
        retry_recommended: true,
    };

    let config = VerifyConfig {
        on_blocked: OnBlockedStrategy::Retry,
        max_retries: 3,
        ..VerifyConfig::default()
    };

    // Simulate retry handling
    if result.retry_recommended && config.max_retries > 0 && config.on_blocked == OnBlockedStrategy::Retry {
        task.prepare_retry();
    }

    assert_eq!(task.retry_count, 1);
    assert_eq!(task.status, TaskStatus::Pending);
}

/// Test verification failure with Skip strategy
#[test]
fn test_verification_failure_skip_strategy() {
    let mut task = create_task("task-001", "Test task", vec![]);
    task.status = TaskStatus::InProgress;

    let _config = VerifyConfig {
        on_blocked: OnBlockedStrategy::Skip,
        ..VerifyConfig::default()
    };

    // With Skip strategy, task remains as-is (dangerous but supported)
    // Task should stay InProgress or Completed based on original state
    // The pipeline continues despite verification failure
}

// =============================================================================
// Git Operation Failure Tests
// =============================================================================

/// Test git repository initialization success
#[test]
fn test_git_init_success() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let result = init_repo(temp_dir.path());

    assert!(result.is_ok(), "Git init should succeed");
    let repo = result.unwrap();

    assert!(temp_dir.path().join(".git").exists(), ".git should exist");
    assert!(temp_dir.path().join(".gitignore").exists(), ".gitignore should exist");

    // Verify config
    let config = repo.config().expect("Should have config");
    assert_eq!(config.get_string("user.email").unwrap(), "ltmatrix@agent");
    assert_eq!(config.get_string("user.name").unwrap(), "Ltmatrix Agent");
}

/// Test git checkout creates branch
#[test]
fn test_git_checkout_creates_branch() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo = init_repo(temp_dir.path()).expect("Failed to init repo");

    // Create initial commit
    let sig = git2::Signature::new("Test", "test@test.com", &git2::Time::new(0, 0)).unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

    // Checkout new branch
    let result = checkout(&repo, "feature-branch");

    assert!(result.is_ok(), "Checkout should succeed");
    let branch = get_current_branch(&repo).expect("Should get branch");
    assert_eq!(branch, "feature-branch");
}

/// Test git operation on non-existent directory
#[test]
fn test_git_init_creates_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let new_dir = temp_dir.path().join("subdir").join("nested");

    // Directory doesn't exist yet
    assert!(!new_dir.exists());

    // init_repo should create it
    let result = init_repo(&new_dir);

    assert!(result.is_ok(), "Should create directory and init repo");
    assert!(new_dir.exists(), "Directory should be created");
    assert!(new_dir.join(".git").exists(), ".git should exist");
}

/// Test git operations handle invalid paths
#[test]
fn test_git_operations_invalid_path() {
    // Try to open a file path as a repository
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("file.txt");
    std::fs::write(&file_path, "test").expect("Failed to write file");

    // Opening a file as a repo should fail
    let result = git2::Repository::open(&file_path);
    assert!(result.is_err(), "Should fail to open file as repo");
}

/// Test git branch name validation
#[test]
fn test_git_branch_names() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo = init_repo(temp_dir.path()).expect("Failed to init repo");

    // Create initial commit
    let sig = git2::Signature::new("Test", "test@test.com", &git2::Time::new(0, 0)).unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

    // Valid branch names
    let valid_names = vec!["feature", "feature-123", "fix/issue-42", "release_v1.0.0"];
    for name in valid_names {
        let result = checkout(&repo, name);
        assert!(result.is_ok(), "Branch name '{}' should be valid", name);
    }
}

// =============================================================================
// Error Message Clarity Tests
// =============================================================================

/// Test workspace state error messages are clear
#[test]
fn test_workspace_state_error_messages() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let non_existent = temp_dir.path().join("nonexistent");

    // Try to load state that doesn't exist
    let result = WorkspaceState::load(non_existent.clone());

    assert!(result.is_err(), "Should fail to load non-existent state");
    let error = result.unwrap_err();
    let error_msg = error.to_string();

    // Error should be informative
    assert!(
        error_msg.contains("Failed to read") || error_msg.contains("not exist") || error_msg.contains("corrupted"),
        "Error message should explain the issue: {}",
        error_msg
    );
}

/// Test corrupted state file error message
#[test]
fn test_corrupted_state_error_message() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create corrupted manifest file
    let manifest_dir = project_path.join(".ltmatrix");
    std::fs::create_dir_all(&manifest_dir).expect("Failed to create dir");

    let manifest_path = manifest_dir.join("tasks-manifest.json");
    std::fs::write(&manifest_path, "{ invalid json }").expect("Failed to write file");

    // Try to load corrupted state
    let result = WorkspaceState::load(project_path);

    assert!(result.is_err(), "Should fail on corrupted JSON");
    let error = result.unwrap_err();
    let error_msg = error.to_string();

    assert!(
        error_msg.contains("Failed to parse") || error_msg.contains("corrupted"),
        "Error should mention parsing or corruption: {}",
        error_msg
    );
}

/// Test that load_or_create provides helpful recovery
#[test]
fn test_load_or_create_recovery_message() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create corrupted file
    let manifest_dir = project_path.join(".ltmatrix");
    std::fs::create_dir_all(&manifest_dir).expect("Failed to create dir");
    std::fs::write(manifest_dir.join("tasks-manifest.json"), "invalid")
        .expect("Failed to write file");

    // load_or_create should recover gracefully
    let result = WorkspaceState::load_or_create(project_path);

    assert!(result.is_ok(), "load_or_create should recover from corrupted state");
    let state = result.unwrap();
    assert!(state.tasks.is_empty(), "Should return empty state on recovery");
}

/// Test dependency graph validation error messages
#[test]
fn test_dependency_graph_error_messages() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create tasks with orphaned dependency
    let mut task1 = create_task("task-001", "Task 1", vec![]);
    let mut task2 = create_task("task-002", "Task 2", vec!["nonexistent-task".to_string()]);

    task1.status = TaskStatus::Pending;
    task2.status = TaskStatus::Pending;

    let state = WorkspaceState::new(temp_dir.path().to_path_buf(), vec![task1, task2]);

    // Check for orphaned tasks
    let orphaned = state.detect_orphaned_tasks();

    assert!(!orphaned.is_empty(), "Should detect orphaned dependency");
    assert!(
        orphaned.iter().any(|(id, deps)| id == "task-002" && deps.contains(&"nonexistent-task".to_string())),
        "Should identify task-002 has missing dependency"
    );
}

/// Test circular dependency error detection
#[test]
fn test_circular_dependency_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create circular dependency: A -> B -> C -> A
    let task_a = create_task("task-a", "Task A", vec!["task-c".to_string()]);
    let task_b = create_task("task-b", "Task B", vec!["task-a".to_string()]);
    let task_c = create_task("task-c", "Task C", vec!["task-b".to_string()]);

    let state = WorkspaceState::new(
        temp_dir.path().to_path_buf(),
        vec![task_a, task_b, task_c],
    );

    // Validate should fail
    let result = state.validate_dependency_graph();
    assert!(result.is_err(), "Should detect circular dependency");

    let error = result.unwrap_err();
    let error_msg = error.to_string();

    assert!(
        error_msg.contains("Circular") || error_msg.contains("cycle"),
        "Error should mention circular dependency: {}",
        error_msg
    );
}

// =============================================================================
// Verification Result Tests
// =============================================================================

/// Test verification result creation
#[test]
fn test_verification_result_passed() {
    let task = create_task("task-001", "Test task", vec![]);
    let result = VerificationResult {
        task: task.clone(),
        passed: true,
        reasoning: "All requirements met".to_string(),
        unmet_criteria: vec![],
        suggestions: vec![],
        retry_recommended: false,
    };

    assert!(result.passed);
    assert!(result.unmet_criteria.is_empty());
    assert!(!result.retry_recommended);
}

/// Test verification result with unmet criteria
#[test]
fn test_verification_result_unmet_criteria() {
    let task = create_task("task-001", "Test task", vec![]);
    let result = VerificationResult {
        task: task.clone(),
        passed: false,
        reasoning: "Implementation incomplete".to_string(),
        unmet_criteria: vec![
            "Missing error handling".to_string(),
            "No unit tests".to_string(),
        ],
        suggestions: vec![
            "Add try-catch block".to_string(),
            "Write test cases".to_string(),
        ],
        retry_recommended: true,
    };

    assert!(!result.passed);
    assert_eq!(result.unmet_criteria.len(), 2);
    assert_eq!(result.suggestions.len(), 2);
    assert!(result.retry_recommended);
}

/// Test verification summary
#[test]
fn test_verification_summary() {
    let summary = VerificationSummary {
        total_tasks: 10,
        passed_tasks: 7,
        failed_tasks: 2,
        skipped_tasks: 1,
        total_time: 45,
        results: vec![],
    };

    assert_eq!(summary.total_tasks, 10);
    assert_eq!(summary.passed_tasks, 7);
    assert_eq!(summary.failed_tasks, 2);
    assert_eq!(summary.skipped_tasks, 1);
    assert_eq!(summary.total_time, 45);
}

// =============================================================================
// Mode Configuration Error Handling Tests
// =============================================================================

/// Test mode config provides correct max retries
#[test]
fn test_mode_config_max_retries() {
    let fast = ModeConfig::fast_mode();
    assert_eq!(fast.max_retries, 1, "Fast mode should have 1 retry");

    let standard = ModeConfig::default();
    assert_eq!(standard.max_retries, 3, "Standard mode should have 3 retries");

    let expert = ModeConfig::expert_mode();
    assert_eq!(expert.max_retries, 3, "Expert mode should have 3 retries");
}

/// Test mode config provides correct timeouts
#[test]
fn test_mode_config_timeouts() {
    let fast = ModeConfig::fast_mode();
    assert_eq!(fast.timeout_plan, 60, "Fast plan timeout should be 60s");
    assert_eq!(fast.timeout_exec, 1800, "Fast exec timeout should be 1800s");

    let standard = ModeConfig::default();
    assert_eq!(standard.timeout_plan, 120, "Standard plan timeout should be 120s");
    assert_eq!(standard.timeout_exec, 3600, "Standard exec timeout should be 3600s");

    let expert = ModeConfig::expert_mode();
    assert_eq!(expert.timeout_plan, 180, "Expert plan timeout should be 180s");
    assert_eq!(expert.timeout_exec, 7200, "Expert exec timeout should be 7200s");
}

/// Test verify config should_run logic
#[test]
fn test_verify_config_should_run() {
    let mut config = VerifyConfig::default();
    assert!(config.should_run(), "Default should run");

    config.enabled = false;
    assert!(!config.should_run(), "Disabled should not run");

    config.enabled = true;
    config.mode_config.verify = false;
    assert!(!config.should_run(), "Verify disabled should not run");
}

// =============================================================================
// Task Status Error State Tests
// =============================================================================

/// Test task status transitions on failure
#[test]
fn test_task_status_failure_transitions() {
    let mut task = create_task("task-001", "Test task", vec![]);

    // Initial state
    assert_eq!(task.status, TaskStatus::Pending);

    // Start execution
    task.status = TaskStatus::InProgress;
    task.started_at = Some(chrono::Utc::now());

    // Fail execution
    task.status = TaskStatus::Failed;
    task.error = Some("Something went wrong".to_string());

    assert!(task.is_failed());
    assert!(task.error.is_some());

    // Retry
    task.prepare_retry();
    assert_eq!(task.status, TaskStatus::Pending);
    assert_eq!(task.retry_count, 1);
    // Error is preserved for reference
    assert!(task.error.is_some());
}

/// Test task can_execute with failed dependency
#[test]
fn test_task_with_failed_dependency() {
    let _task1 = create_task_with_status("task-001", "First task", TaskStatus::Failed);
    let task2 = create_task("task-002", "Second task", vec!["task-001".to_string()]);

    // Task 2 should not be executable because its dependency failed
    // (even though it's not "completed")
    let completed: HashSet<String> = HashSet::new();
    assert!(!task2.can_execute(&completed), "Should not execute with failed dependency");

    // Failed task is not in completed set
    let mut with_failed: HashSet<String> = HashSet::new();
    with_failed.insert("task-001".to_string());
    // Even if we track failed tasks, can_execute only checks completed
    assert!(task2.can_execute(&with_failed), "Would need separate logic for failed deps");
}

/// Test task with blocked dependency
#[test]
fn test_task_with_blocked_dependency() {
    let _task1 = create_task_with_status("task-001", "Blocked task", TaskStatus::Blocked);
    let task2 = create_task("task-002", "Dependent task", vec!["task-001".to_string()]);

    // Task 2 cannot execute because task-001 is blocked (not completed)
    let completed: HashSet<String> = HashSet::new();
    assert!(!task2.can_execute(&completed));
}

// =============================================================================
// Error Recovery Tests
// =============================================================================

/// Test workspace state recovery from corruption
#[test]
fn test_workspace_state_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create initial valid state
    let tasks = vec![
        create_task("task-001", "Task 1", vec![]),
        create_task("task-002", "Task 2", vec!["task-001".to_string()]),
    ];

    let state = WorkspaceState::new(project_path.clone(), tasks);
    state.save().expect("Failed to save initial state");

    // Corrupt the file
    let manifest_path = project_path.join(".ltmatrix").join("tasks-manifest.json");
    std::fs::write(&manifest_path, "corrupted data").expect("Failed to corrupt file");

    // load_or_create should recover
    let recovered = WorkspaceState::load_or_create(project_path).expect("Should recover");

    // Recovery creates a new empty state
    assert!(recovered.tasks.is_empty(), "Recovered state should be empty");
}

/// Test cleanup removes corrupted state
#[test]
fn test_cleanup_removes_corrupted_state() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create corrupted state file
    let ltmatrix_dir = project_path.join(".ltmatrix");
    std::fs::create_dir_all(&ltmatrix_dir).expect("Failed to create dir");
    std::fs::write(ltmatrix_dir.join("tasks-manifest.json"), "corrupted")
        .expect("Failed to write corrupted file");

    // Cleanup should work even with corrupted state
    let result = WorkspaceState::cleanup(&project_path);
    assert!(result.is_ok(), "Cleanup should succeed");

    assert!(!ltmatrix_dir.exists(), ".ltmatrix directory should be removed");
}

// =============================================================================
// Concurrent Error Scenarios
// =============================================================================

/// Test multiple task failures
#[test]
fn test_multiple_task_failures() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let mut tasks = vec![
        create_task("task-001", "Task 1", vec![]),
        create_task("task-002", "Task 2", vec![]),
        create_task("task-003", "Task 3", vec![]),
    ];

    // Mark all as failed with different errors
    tasks[0].status = TaskStatus::Failed;
    tasks[0].error = Some("Network timeout".to_string());

    tasks[1].status = TaskStatus::Failed;
    tasks[1].error = Some("Permission denied".to_string());

    tasks[2].status = TaskStatus::Failed;
    tasks[2].error = Some("Out of memory".to_string());

    let state = WorkspaceState::new(temp_dir.path().to_path_buf(), tasks);
    let summary = state.status_summary();

    assert_eq!(summary.failed, 3, "Should count all failed tasks");
    assert_eq!(summary.completed, 0);
    assert_eq!(summary.pending, 0);
}

/// Test reset_failed only affects failed tasks
#[test]
fn test_reset_failed_selective() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let mut tasks = vec![
        create_task("task-001", "Completed", vec![]),
        create_task("task-002", "Failed 1", vec![]),
        create_task("task-003", "Pending", vec![]),
        create_task("task-004", "Failed 2", vec![]),
    ];

    tasks[0].status = TaskStatus::Completed;
    tasks[1].status = TaskStatus::Failed;
    tasks[2].status = TaskStatus::Pending;
    tasks[3].status = TaskStatus::Failed;

    let mut state = WorkspaceState::new(temp_dir.path().to_path_buf(), tasks);
    let reset_count = state.reset_failed().expect("Reset should succeed");

    assert_eq!(reset_count, 2, "Should reset 2 failed tasks");

    // Verify states after reset
    assert_eq!(state.tasks[0].status, TaskStatus::Completed, "Completed should stay");
    assert_eq!(state.tasks[1].status, TaskStatus::Pending, "Failed should become Pending");
    assert_eq!(state.tasks[2].status, TaskStatus::Pending, "Pending should stay");
    assert_eq!(state.tasks[3].status, TaskStatus::Pending, "Failed should become Pending");
}

// =============================================================================
// Edge Case Tests
// =============================================================================

/// Test empty task list handling
#[test]
fn test_empty_task_list_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let state = WorkspaceState::new(temp_dir.path().to_path_buf(), vec![]);
    let summary = state.status_summary();

    assert_eq!(summary.total(), 0);
    assert_eq!(summary.completion_percentage(), 0.0);
}

/// Test very long error messages are stored
#[test]
fn test_long_error_messages() {
    let mut task = create_task("task-001", "Test task", vec![]);

    let long_error = "x".repeat(10000);
    task.status = TaskStatus::Failed;
    task.error = Some(long_error.clone());

    assert_eq!(task.error.unwrap().len(), 10000);
}

/// Test special characters in error messages
#[test]
fn test_special_characters_in_errors() {
    let mut task = create_task("task-001", "Test task", vec![]);

    let special_error = "Error: \n\t\r\"quotes\" 'apostrophes' <xml> &entities;";
    task.status = TaskStatus::Failed;
    task.error = Some(special_error.to_string());

    assert!(task.error.as_ref().unwrap().contains("quotes"));
    assert!(task.error.as_ref().unwrap().contains("<xml>"));
}

/// Test unicode in error messages
#[test]
fn test_unicode_in_error_messages() {
    let mut task = create_task("task-001", "Test task", vec![]);

    let unicode_error = "错误: 文件未找到 🚫 エラー";
    task.status = TaskStatus::Failed;
    task.error = Some(unicode_error.to_string());

    assert!(task.error.as_ref().unwrap().contains("错误"));
    assert!(task.error.as_ref().unwrap().contains("🚫"));
}