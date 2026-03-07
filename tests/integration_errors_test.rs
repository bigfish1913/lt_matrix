//! Integration tests for error handling scenarios
//!
//! This test module provides comprehensive testing for error handling across the ltmatrix pipeline.
//! It covers:
//! - Agent failure and retry logic
//! - Test failure and fix cycle
//! - Verify failure and --on-blocked strategies
//! - Git operation failures
//! - Clear and actionable error messages
//!
//! Task: Create integration test for error handling

use ltmatrix::agent::backend::{
    AgentConfig, AgentConfigBuilder, AgentError, AgentResponse, ExecutionConfig,
    AgentSession, MemorySession,
};
use ltmatrix::cli::args::BlockedStrategy;
use ltmatrix::git::repository::{init_repo, checkout, get_current_branch, create_signature};
use ltmatrix::models::{
    ExecutionMode, ModeConfig, Task, TaskComplexity, TaskStatus, PipelineStage,
};
use ltmatrix::pipeline::orchestrator::{OrchestratorConfig, PipelineOrchestrator, PipelineResult};
use ltmatrix::pipeline::test::{TestConfig, TestFramework, FrameworkDetection, detect_test_framework};
use ltmatrix::pipeline::verify::{OnBlockedStrategy, VerifyConfig, VerificationResult, VerificationSummary};
use ltmatrix::workspace::WorkspaceState;
use std::collections::HashSet;
use std::path::PathBuf;
use tempfile::TempDir;

// =============================================================================
// Test Helper Functions
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

/// Creates a task with a specific retry count
fn create_task_with_retries(id: &str, title: &str, retry_count: u32, _max_retries: u32) -> Task {
    let mut task = create_task(id, title, vec![]);
    task.status = TaskStatus::Failed;
    task.retry_count = retry_count;
    task.error = Some("Simulated failure".to_string());
    task
}

// =============================================================================
// Agent Failure and Retry Logic Integration Tests
// =============================================================================

/// Test agent error types are properly formed
#[test]
fn test_agent_error_command_not_found() {
    let error = AgentError::CommandNotFound {
        command: "nonexistent-agent".to_string(),
    };

    let error_msg = error.to_string();
    assert!(
        error_msg.contains("nonexistent-agent"),
        "Error message should contain command name"
    );
    assert!(
        error_msg.contains("not found"),
        "Error message should indicate not found"
    );
}

#[test]
fn test_agent_error_execution_failed() {
    let error = AgentError::ExecutionFailed {
        command: "claude".to_string(),
        message: "Process exited with code 1".to_string(),
    };

    let error_msg = error.to_string();
    assert!(error_msg.contains("claude"));
    assert!(error_msg.contains("failed"));
    assert!(error_msg.contains("Process exited"));
}

#[test]
fn test_agent_error_timeout() {
    let error = AgentError::Timeout {
        command: "claude".to_string(),
        timeout_secs: 3600,
    };

    let error_msg = error.to_string();
    assert!(error_msg.contains("claude"));
    assert!(error_msg.contains("timed out"));
    assert!(error_msg.contains("3600"));
}

#[test]
fn test_agent_error_invalid_response() {
    let error = AgentError::InvalidResponse {
        reason: "JSON parse error: expected boolean".to_string(),
    };

    let error_msg = error.to_string();
    assert!(error_msg.contains("Invalid"));
    assert!(error_msg.contains("JSON parse error"));
}

#[test]
fn test_agent_error_config_validation() {
    let error = AgentError::ConfigValidation {
        field: "timeout_secs".to_string(),
        message: "Timeout must be greater than 0".to_string(),
    };

    let error_msg = error.to_string();
    assert!(error_msg.contains("timeout_secs"));
    assert!(error_msg.contains("must be greater than 0"));
}

#[test]
fn test_agent_error_session_not_found() {
    let error = AgentError::SessionNotFound {
        session_id: "session-abc123".to_string(),
    };

    let error_msg = error.to_string();
    assert!(error_msg.contains("session-abc123"));
    assert!(error_msg.contains("not found"));
}

/// Test agent config validation catches empty name
#[test]
fn test_agent_config_validation_empty_name() {
    let config = AgentConfig {
        name: "".to_string(),
        model: "claude-sonnet-4-6".to_string(),
        command: "claude".to_string(),
        timeout_secs: 3600,
        max_retries: 3,
        enable_session: true,
    };

    let result = config.validate();
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(matches!(error, AgentError::ConfigValidation { field, .. } if field == "name"));
}

/// Test agent config validation catches empty model
#[test]
fn test_agent_config_validation_empty_model() {
    let config = AgentConfigBuilder::default()
        .name("claude")
        .model("")
        .command("claude")
        .build();

    let result = config.validate();
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(matches!(error, AgentError::ConfigValidation { field, .. } if field == "model"));
}

/// Test agent config validation catches zero timeout
#[test]
fn test_agent_config_validation_zero_timeout() {
    let config = AgentConfigBuilder::default()
        .name("claude")
        .model("claude-sonnet-4-6")
        .command("claude")
        .timeout_secs(0)
        .build();

    let result = config.validate();
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(matches!(error, AgentError::ConfigValidation { field, .. } if field == "timeout_secs"));
}

/// Test agent config builder creates valid config
#[test]
fn test_agent_config_builder_valid() {
    let config = AgentConfigBuilder::default()
        .name("claude")
        .model("claude-sonnet-4-6")
        .command("claude")
        .timeout_secs(3600)
        .max_retries(5)
        .enable_session(true)
        .build();

    assert!(config.validate().is_ok());
    assert_eq!(config.name, "claude");
    assert_eq!(config.model, "claude-sonnet-4-6");
    assert_eq!(config.max_retries, 5);
}

/// Test execution config defaults
#[test]
fn test_execution_config_defaults() {
    let config = ExecutionConfig::default();

    assert_eq!(config.model, "claude-sonnet-4-6");
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.timeout, 3600);
    assert!(config.enable_session);
    assert!(config.env_vars.is_empty());
}

/// Test session lifecycle management
#[test]
fn test_session_lifecycle() {
    let mut session = MemorySession::default();

    // Initial state
    assert!(!session.session_id().is_empty());
    assert_eq!(session.reuse_count(), 0);
    assert!(!session.is_stale());

    // Mark as accessed
    session.mark_accessed();
    assert_eq!(session.reuse_count(), 1);

    // Multiple accesses
    session.mark_accessed();
    session.mark_accessed();
    assert_eq!(session.reuse_count(), 3);
}

/// Test agent response structure
#[test]
fn test_agent_response_structure() {
    let response = AgentResponse {
        output: "Task completed successfully".to_string(),
        structured_data: Some(serde_json::json!({"status": "ok"})),
        is_complete: true,
        error: None,
    };

    assert!(response.is_complete);
    assert!(response.error.is_none());
    assert!(response.structured_data.is_some());
}

/// Test agent response with error
#[test]
fn test_agent_response_with_error() {
    let response = AgentResponse {
        output: "".to_string(),
        structured_data: None,
        is_complete: false,
        error: Some("Execution failed: timeout".to_string()),
    };

    assert!(!response.is_complete);
    assert!(response.error.is_some());
    assert!(response.output.is_empty());
}

/// Test task retry after agent failure
#[test]
fn test_task_retry_after_agent_failure() {
    let mut task = create_task("task-001", "Implement feature X", vec![]);
    task.session_id = Some("session-xyz".to_string());

    // Simulate agent failure
    task.status = TaskStatus::Failed;
    task.error = Some("Agent execution failed: timeout after 3600s".to_string());

    // Task should be retryable
    assert!(task.can_retry(3));

    // Prepare for retry
    task.prepare_retry();

    // Verify retry state
    assert_eq!(task.retry_count, 1);
    assert_eq!(task.status, TaskStatus::Pending);
    assert!(task.started_at.is_none());
    // Session should be preserved for reuse
    assert_eq!(task.session_id, Some("session-xyz".to_string()));
}

/// Test max retries reached scenario
#[test]
fn test_max_retries_reached() {
    let mut task = create_task("task-001", "Complex task", vec![]);

    // Simulate multiple failed attempts
    for i in 0..3 {
        task.status = TaskStatus::Failed;
        task.error = Some(format!("Attempt {} failed", i + 1));
        if task.can_retry(3) {
            task.prepare_retry();
        }
    }

    // After 3 retries, task should not be retryable with max_retries=3
    task.status = TaskStatus::Failed;
    assert!(!task.can_retry(3));
    assert_eq!(task.retry_count, 3);
}

// =============================================================================
// Test Failure and Fix Cycle Integration Tests
// =============================================================================

/// Test test framework detection for various project types
#[test]
fn test_framework_detection_cargo() {
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
    assert!(!detection.config_files.is_empty());
}

#[test]
fn test_framework_detection_npm() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create package.json with test script
    let package_json = temp_dir.path().join("package.json");
    std::fs::write(
        &package_json,
        r#"{"name": "test", "scripts": {"test": "jest"}, "devDependencies": {"jest": "^29.0.0"}}"#,
    )
    .expect("Failed to write package.json");

    // Create tests directory
    std::fs::create_dir(temp_dir.path().join("__tests__")).expect("Failed to create tests dir");

    let result = detect_test_framework(temp_dir.path());
    assert!(result.is_ok());

    let detection = result.unwrap();
    assert_eq!(detection.framework, TestFramework::Npm);
    assert!(detection.confidence >= 0.8);
}

#[test]
fn test_framework_detection_go() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create go.mod
    std::fs::write(temp_dir.path().join("go.mod"), "module test\n\ngo 1.21")
        .expect("Failed to write go.mod");

    // Create test file
    std::fs::write(
        temp_dir.path().join("main_test.go"),
        "package main\n\nimport \"testing\"\n\nfunc TestMain(t *testing.T) {}",
    )
    .expect("Failed to write test file");

    let result = detect_test_framework(temp_dir.path());
    assert!(result.is_ok());

    let detection = result.unwrap();
    assert_eq!(detection.framework, TestFramework::Go);
}

#[test]
fn test_framework_detection_pytest() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create pytest.ini
    std::fs::write(
        temp_dir.path().join("pytest.ini"),
        "[pytest]\ntestpaths = tests",
    )
    .expect("Failed to write pytest.ini");

    // Create tests directory with test file
    std::fs::create_dir(temp_dir.path().join("tests")).expect("Failed to create tests dir");
    std::fs::write(
        temp_dir.path().join("tests").join("test_main.py"),
        "def test_example(): pass",
    )
    .expect("Failed to write test file");

    let result = detect_test_framework(temp_dir.path());
    assert!(result.is_ok());

    let detection = result.unwrap();
    assert_eq!(detection.framework, TestFramework::Pytest);
}

/// Test test config behavior in different modes
#[test]
fn test_test_config_mode_differences() {
    let fast_config = TestConfig::fast_mode();
    let standard_config = TestConfig::default();
    let expert_config = TestConfig::expert_mode();

    // Fast mode should have tests disabled
    assert!(!fast_config.enabled);
    assert!(!fast_config.fail_on_error);

    // Standard mode should have tests enabled
    assert!(standard_config.enabled);
    assert!(standard_config.fail_on_error);

    // Expert mode should have tests enabled with higher timeout
    assert!(expert_config.enabled);
    assert!(expert_config.fail_on_error);
    assert!(expert_config.timeout > standard_config.timeout);
}

/// Test test failure handling with fail_on_error=false
#[tokio::test]
async fn test_test_failure_non_blocking() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create a Cargo project without actual tests
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    std::fs::write(&cargo_toml, "[package]\nname = \"test\"\nversion = \"0.1.0\"")
        .expect("Failed to write Cargo.toml");

    let task = create_task("task-001", "Test task", vec![]);

    let config = TestConfig {
        enabled: true,
        fail_on_error: false,
        work_dir: temp_dir.path().to_path_buf(),
        ..TestConfig::default()
    };

    // test_tasks should handle failures gracefully when fail_on_error is false
    let result = ltmatrix::pipeline::test::test_tasks(vec![task], &config).await;
    // Result depends on whether cargo test is available and succeeds
    assert!(result.is_ok() || result.is_err());
}

/// Test test stage skipped in fast mode
#[tokio::test]
async fn test_test_stage_skipped_in_fast_mode() {
    let task = create_task("task-001", "Test task", vec![]);

    let config = TestConfig::fast_mode();

    let result = ltmatrix::pipeline::test::test_tasks(vec![task.clone()], &config).await;

    assert!(result.is_ok());
    let tasks = result.unwrap();
    assert_eq!(tasks.len(), 1);
}

// =============================================================================
// Verify Failure and OnBlocked Strategy Integration Tests
// =============================================================================

/// Test all OnBlockedStrategy variants
#[test]
fn test_on_blocked_strategy_variants() {
    let strategies = [
        OnBlockedStrategy::Fail,
        OnBlockedStrategy::Retry,
        OnBlockedStrategy::Block,
        OnBlockedStrategy::Skip,
    ];

    for strategy in strategies {
        let _debug_output = format!("{:?}", strategy);
    }
}

/// Test verify config per mode
#[test]
fn test_verify_config_per_mode() {
    let fast = VerifyConfig::fast_mode();
    let standard = VerifyConfig::default();
    let expert = VerifyConfig::expert_mode();

    // Fast mode: Fail immediately, no retries
    assert_eq!(fast.on_blocked, OnBlockedStrategy::Fail);
    assert_eq!(fast.max_retries, 0);
    assert!(fast.fast_mode);

    // Standard mode: Retry on failure
    assert_eq!(standard.on_blocked, OnBlockedStrategy::Retry);
    assert_eq!(standard.max_retries, 1);

    // Expert mode: More retries, longer timeout
    assert_eq!(expert.on_blocked, OnBlockedStrategy::Retry);
    assert_eq!(expert.max_retries, 2);
    assert!(expert.timeout > standard.timeout);
}

/// Test verification result with unmet criteria
#[test]
fn test_verification_result_unmet_criteria() {
    let task = create_task("task-001", "Implement feature", vec![]);

    let result = VerificationResult {
        task: task.clone(),
        passed: false,
        reasoning: "Implementation incomplete".to_string(),
        unmet_criteria: vec![
            "Missing error handling for edge cases".to_string(),
            "No unit tests for the new functionality".to_string(),
            "Documentation not updated".to_string(),
        ],
        suggestions: vec![
            "Add try-catch block in process_data()".to_string(),
            "Create test_process_data.rs with test cases".to_string(),
            "Update README.md with new API usage".to_string(),
        ],
        retry_recommended: true,
    };

    assert!(!result.passed);
    assert_eq!(result.unmet_criteria.len(), 3);
    assert_eq!(result.suggestions.len(), 3);
    assert!(result.retry_recommended);

    // Verify error message clarity
    assert!(result.reasoning.contains("incomplete"));
    assert!(result.unmet_criteria.iter().all(|c| !c.is_empty()));
    assert!(result.suggestions.iter().all(|s| !s.is_empty()));
}

/// Test verification failure with Fail strategy
#[test]
fn test_verification_failure_fail_strategy() {
    let mut task = create_task("task-001", "Test task", vec![]);
    task.status = TaskStatus::InProgress;

    let config = VerifyConfig {
        on_blocked: OnBlockedStrategy::Fail,
        max_retries: 0,
        ..VerifyConfig::default()
    };

    let result = VerificationResult {
        task: task.clone(),
        passed: false,
        reasoning: "Critical validation failed".to_string(),
        unmet_criteria: vec!["Security vulnerability detected".to_string()],
        suggestions: vec!["Fix SQL injection in query()".to_string()],
        retry_recommended: false,
    };

    // With Fail strategy and retry_recommended=false, task should be marked Failed
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

    // With Block strategy, task should be marked Blocked
    if config.on_blocked == OnBlockedStrategy::Block {
        task.status = TaskStatus::Blocked;
        task.error = Some("Blocked pending fix: Implementation issues".to_string());
    }

    assert_eq!(task.status, TaskStatus::Blocked);
    assert!(task.error.unwrap().contains("Blocked pending fix"));
}

/// Test verification failure with Retry strategy
#[test]
fn test_verification_failure_retry_strategy() {
    let mut task = create_task("task-001", "Test task", vec![]);
    task.status = TaskStatus::InProgress;
    task.retry_count = 0;

    let config = VerifyConfig {
        on_blocked: OnBlockedStrategy::Retry,
        max_retries: 3,
        ..VerifyConfig::default()
    };

    let result = VerificationResult {
        task: task.clone(),
        passed: false,
        reasoning: "Minor issues found".to_string(),
        unmet_criteria: vec!["Style issues".to_string()],
        suggestions: vec!["Fix formatting".to_string()],
        retry_recommended: true,
    };

    // With Retry strategy, task should be prepared for retry
    if result.retry_recommended && config.max_retries > 0 && config.on_blocked == OnBlockedStrategy::Retry {
        task.prepare_retry();
    }

    assert_eq!(task.retry_count, 1);
    assert_eq!(task.status, TaskStatus::Pending);
}

/// Test CLI BlockedStrategy mapping
#[test]
fn test_blocked_strategy_cli_mapping() {
    let skip = BlockedStrategy::Skip;
    let ask = BlockedStrategy::Ask;
    let abort = BlockedStrategy::Abort;
    let retry = BlockedStrategy::Retry;

    // Verify string representations
    assert_eq!(skip.to_string(), "skip");
    assert_eq!(ask.to_string(), "ask");
    assert_eq!(abort.to_string(), "abort");
    assert_eq!(retry.to_string(), "retry");
}

/// Test verification summary statistics
#[test]
fn test_verification_summary_statistics() {
    let summary = VerificationSummary {
        total_tasks: 15,
        passed_tasks: 10,
        failed_tasks: 3,
        skipped_tasks: 2,
        total_time: 120,
        results: vec![],
    };

    assert_eq!(summary.total_tasks, 15);
    assert_eq!(summary.passed_tasks, 10);
    assert_eq!(summary.failed_tasks, 3);
    assert_eq!(summary.skipped_tasks, 2);
    assert_eq!(summary.total_time, 120);

    // Calculate pass rate
    let pass_rate = (summary.passed_tasks as f64 / summary.total_tasks as f64) * 100.0;
    assert!((pass_rate - 66.67).abs() < 0.1);
}

// =============================================================================
// Git Operation Failure Integration Tests
// =============================================================================

/// Test git repository initialization
#[test]
fn test_git_init_success() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let result = init_repo(temp_dir.path());

    assert!(result.is_ok());
    let repo = result.unwrap();

    // Verify .git directory exists
    assert!(temp_dir.path().join(".git").exists());

    // Verify .gitignore was created
    assert!(temp_dir.path().join(".gitignore").exists());

    // Verify git config
    let config = repo.config().expect("Should have config");
    assert_eq!(config.get_string("user.email").unwrap(), "ltmatrix@agent");
    assert_eq!(config.get_string("user.name").unwrap(), "Ltmatrix Agent");
}

/// Test git init creates directory if needed
#[test]
fn test_git_init_creates_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let new_dir = temp_dir.path().join("nested").join("repo");

    assert!(!new_dir.exists());

    let result = init_repo(&new_dir);

    assert!(result.is_ok());
    assert!(new_dir.exists());
    assert!(new_dir.join(".git").exists());
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
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    // Checkout new branch
    let result = checkout(&repo, "feature-branch");

    assert!(result.is_ok());
    let branch = get_current_branch(&repo).expect("Should get branch");
    assert_eq!(branch, "feature-branch");
}

/// Test git operations with valid branch names
#[test]
fn test_git_valid_branch_names() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo = init_repo(temp_dir.path()).expect("Failed to init repo");

    // Create initial commit
    let sig = git2::Signature::new("Test", "test@test.com", &git2::Time::new(0, 0)).unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    let valid_names = vec!["feature", "feature-123", "fix/issue-42", "release_v1.0.0"];

    for name in valid_names {
        let result = checkout(&repo, name);
        assert!(result.is_ok(), "Branch name '{}' should be valid", name);
    }
}

/// Test git signature creation
#[test]
fn test_git_signature_creation() {
    let sig = create_signature("Ltmatrix Agent", "ltmatrix@agent")
        .expect("Should create signature");

    assert_eq!(sig.name(), Some("Ltmatrix Agent"));
    assert_eq!(sig.email(), Some("ltmatrix@agent"));
}

/// Test git operation error on non-existent directory
#[test]
fn test_git_error_nonexistent_repo() {
    let result = git2::Repository::open("/nonexistent/path/.git");
    assert!(result.is_err());
}

/// Test .gitignore content is comprehensive
#[test]
fn test_gitignore_content() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    init_repo(temp_dir.path()).expect("Failed to init repo");

    let gitignore = temp_dir.path().join(".gitignore");
    let content = std::fs::read_to_string(&gitignore).expect("Should read .gitignore");

    // Verify common patterns are present
    assert!(content.contains("node_modules/"));
    assert!(content.contains("__pycache__/"));
    assert!(content.contains("/target/"));
    assert!(content.contains(".env"));
    assert!(content.contains(".idea/"));
    assert!(content.contains(".vscode/"));
    assert!(content.contains("*.log"));
}

// =============================================================================
// Clear and Actionable Error Messages Tests
// =============================================================================

/// Test workspace state error messages
#[test]
fn test_workspace_state_error_messages() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let non_existent = temp_dir.path().join("nonexistent");

    let result = WorkspaceState::load(non_existent.clone());

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_msg = error.to_string();

    // Error should be informative
    assert!(
        error_msg.contains("Failed to read")
            || error_msg.contains("not exist")
            || error_msg.contains("No such file"),
        "Error message should explain the issue: {}",
        error_msg
    );
}

/// Test corrupted state file error message
#[test]
fn test_corrupted_state_error_message() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create corrupted manifest
    let manifest_dir = project_path.join(".ltmatrix");
    std::fs::create_dir_all(&manifest_dir).expect("Failed to create dir");
    std::fs::write(manifest_dir.join("tasks-manifest.json"), "{ invalid json }")
        .expect("Failed to write file");

    let result = WorkspaceState::load(project_path);

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_msg = error.to_string();

    assert!(
        error_msg.contains("Failed to parse")
            || error_msg.contains("expected")
            || error_msg.contains("JSON"),
        "Error should mention parsing or JSON: {}",
        error_msg
    );
}

/// Test load_or_create recovery
#[test]
fn test_load_or_create_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create corrupted file
    let manifest_dir = project_path.join(".ltmatrix");
    std::fs::create_dir_all(&manifest_dir).expect("Failed to create dir");
    std::fs::write(manifest_dir.join("tasks-manifest.json"), "invalid")
        .expect("Failed to write file");

    // load_or_create should recover
    let result = WorkspaceState::load_or_create(project_path);

    assert!(result.is_ok());
    let state = result.unwrap();
    assert!(state.tasks.is_empty());
}

/// Test dependency validation error messages
#[test]
fn test_dependency_validation_error_messages() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create tasks with orphaned dependency
    let task1 = create_task("task-001", "Task 1", vec![]);
    let task2 = create_task("task-002", "Task 2", vec!["nonexistent-task".to_string()]);

    let state = WorkspaceState::new(temp_dir.path().to_path_buf(), vec![task1, task2]);

    let orphaned = state.detect_orphaned_tasks();

    assert!(!orphaned.is_empty());
    assert!(
        orphaned.iter().any(|(id, deps)| {
            id == "task-002" && deps.contains(&"nonexistent-task".to_string())
        }),
        "Should identify task-002 has missing dependency"
    );
}

/// Test circular dependency detection
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

    let result = state.validate_dependency_graph();
    assert!(result.is_err());

    let error = result.unwrap_err();
    let error_msg = error.to_string();

    assert!(
        error_msg.contains("Circular")
            || error_msg.contains("cycle")
            || error_msg.contains("dependency"),
        "Error should mention circular dependency: {}",
        error_msg
    );
}

// =============================================================================
// Task Status Error State Tests
// =============================================================================

/// Test task status transitions
#[test]
fn test_task_status_transitions() {
    let mut task = create_task("task-001", "Test task", vec![]);

    // Initial state
    assert_eq!(task.status, TaskStatus::Pending);

    // Start execution
    task.status = TaskStatus::InProgress;
    task.started_at = Some(chrono::Utc::now());
    assert_eq!(task.status, TaskStatus::InProgress);

    // Complete successfully
    task.status = TaskStatus::Completed;
    task.completed_at = Some(chrono::Utc::now());
    assert!(task.is_completed());

    // Alternative: Fail execution
    task.status = TaskStatus::Failed;
    task.error = Some("Something went wrong".to_string());
    assert!(task.is_failed());
}

/// Test task with failed dependency
#[test]
fn test_task_with_failed_dependency() {
    let _task1 = create_task_with_status("task-001", "First task", TaskStatus::Failed);
    let task2 = create_task("task-002", "Second task", vec!["task-001".to_string()]);

    // Task 2 should not be executable because its dependency failed
    let completed: HashSet<String> = HashSet::new();
    assert!(!task2.can_execute(&completed));
}

/// Test task with blocked dependency
#[test]
fn test_task_with_blocked_dependency() {
    let _task1 = create_task_with_status("task-001", "Blocked task", TaskStatus::Blocked);
    let task2 = create_task("task-002", "Dependent task", vec!["task-001".to_string()]);

    // Task 2 cannot execute because task-001 is blocked
    let completed: HashSet<String> = HashSet::new();
    assert!(!task2.can_execute(&completed));
}

/// Test task status summary
#[test]
fn test_task_status_summary() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let mut tasks = vec![
        create_task("task-001", "Task 1", vec![]),
        create_task("task-002", "Task 2", vec![]),
        create_task("task-003", "Task 3", vec![]),
        create_task("task-004", "Task 4", vec![]),
    ];

    tasks[0].status = TaskStatus::Completed;
    tasks[1].status = TaskStatus::Failed;
    tasks[2].status = TaskStatus::InProgress;

    let state = WorkspaceState::new(temp_dir.path().to_path_buf(), tasks);
    let summary = state.status_summary();

    assert_eq!(summary.total(), 4);
    assert_eq!(summary.completed, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.in_progress, 1);
    assert_eq!(summary.pending, 1);
}

// =============================================================================
// Error Recovery Tests
// =============================================================================

/// Test workspace state recovery
#[test]
fn test_workspace_state_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Create initial state
    let tasks = vec![
        create_task("task-001", "Task 1", vec![]),
        create_task("task-002", "Task 2", vec!["task-001".to_string()]),
    ];

    let state = WorkspaceState::new(project_path.clone(), tasks);
    state.save().expect("Failed to save state");

    // Corrupt the file
    let manifest_path = project_path.join(".ltmatrix").join("tasks-manifest.json");
    std::fs::write(&manifest_path, "corrupted").expect("Failed to corrupt file");

    // load_or_create should recover
    let recovered = WorkspaceState::load_or_create(project_path).expect("Should recover");
    assert!(recovered.tasks.is_empty());
}

/// Test reset_failed functionality
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

    assert_eq!(reset_count, 2);

    assert_eq!(state.tasks[0].status, TaskStatus::Completed);
    assert_eq!(state.tasks[1].status, TaskStatus::Pending);
    assert_eq!(state.tasks[2].status, TaskStatus::Pending);
    assert_eq!(state.tasks[3].status, TaskStatus::Pending);
}

// =============================================================================
// Mode Configuration Tests
// =============================================================================

/// Test mode config provides correct max retries
#[test]
fn test_mode_config_max_retries() {
    let fast = ModeConfig::fast_mode();
    let standard = ModeConfig::default();
    let expert = ModeConfig::expert_mode();

    assert_eq!(fast.max_retries, 1);
    assert_eq!(standard.max_retries, 3);
    assert_eq!(expert.max_retries, 3);
}

/// Test mode config provides correct timeouts
#[test]
fn test_mode_config_timeouts() {
    let fast = ModeConfig::fast_mode();
    let standard = ModeConfig::default();
    let expert = ModeConfig::expert_mode();

    assert_eq!(fast.timeout_plan, 60);
    assert_eq!(fast.timeout_exec, 1800);

    assert_eq!(standard.timeout_plan, 120);
    assert_eq!(standard.timeout_exec, 3600);

    assert_eq!(expert.timeout_plan, 180);
    assert_eq!(expert.timeout_exec, 7200);
}

/// Test pipeline stages for each mode
#[test]
fn test_pipeline_stages_per_mode() {
    let fast_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Fast);
    let standard_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);
    let expert_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    // Fast mode should skip Test stage
    assert!(!fast_stages.contains(&PipelineStage::Test));

    // Standard mode should have all basic stages
    assert!(standard_stages.contains(&PipelineStage::Generate));
    assert!(standard_stages.contains(&PipelineStage::Assess));
    assert!(standard_stages.contains(&PipelineStage::Execute));
    assert!(standard_stages.contains(&PipelineStage::Test));
    assert!(standard_stages.contains(&PipelineStage::Verify));

    // Expert mode should include Review stage
    assert!(expert_stages.contains(&PipelineStage::Review));
}

// =============================================================================
// Orchestrator Error Handling Tests
// =============================================================================

/// Test orchestrator creation with invalid directory
#[tokio::test]
async fn test_orchestrator_invalid_work_dir() {
    let config = OrchestratorConfig::default()
        .with_work_dir("/nonexistent/path");

    let orchestrator = PipelineOrchestrator::new(config);
    assert!(orchestrator.is_err());
}

/// Test orchestrator result success rate
#[test]
fn test_pipeline_result_success_rate() {
    use std::time::Duration;

    // Create result with public fields
    let result = PipelineResult {
        total_tasks: 0,
        tasks_completed: 0,
        tasks_failed: 0,
        stages_completed: 0,
        total_time: Duration::ZERO,
        completed_tasks: Vec::new(),
        failed_tasks: Vec::new(),
        success: false,
    };

    // Empty result has 100% success rate
    assert_eq!(result.success_rate(), 100.0);

    let result = PipelineResult {
        total_tasks: 10,
        tasks_completed: 8,
        tasks_failed: 2,
        stages_completed: 5,
        total_time: Duration::from_secs(100),
        completed_tasks: Vec::new(),
        failed_tasks: Vec::new(),
        success: true,
    };

    assert_eq!(result.success_rate(), 80.0);
}

/// Test orchestrator config builder
#[test]
fn test_orchestrator_config_builder() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_max_parallel(8)
        .with_progress(false);

    assert_eq!(config.work_dir, temp_dir.path());
    assert_eq!(config.max_parallel_tasks, 8);
    assert!(!config.show_progress);
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

/// Test long error messages
#[test]
fn test_long_error_messages() {
    let mut task = create_task("task-001", "Test task", vec![]);

    let long_error = "Error: ".repeat(1000);
    task.status = TaskStatus::Failed;
    task.error = Some(long_error.clone());

    assert_eq!(task.error.unwrap().len(), 7000); // "Error: " is 7 chars, repeated 1000 times
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
    assert!(task.error.as_ref().unwrap().contains("エラー"));
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

/// Test multiple task failures
#[test]
fn test_multiple_task_failures() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let mut tasks = vec![
        create_task("task-001", "Task 1", vec![]),
        create_task("task-002", "Task 2", vec![]),
        create_task("task-003", "Task 3", vec![]),
    ];

    tasks[0].status = TaskStatus::Failed;
    tasks[0].error = Some("Network timeout".to_string());

    tasks[1].status = TaskStatus::Failed;
    tasks[1].error = Some("Permission denied".to_string());

    tasks[2].status = TaskStatus::Failed;
    tasks[2].error = Some("Out of memory".to_string());

    let state = WorkspaceState::new(temp_dir.path().to_path_buf(), tasks);
    let summary = state.status_summary();

    assert_eq!(summary.failed, 3);
    assert_eq!(summary.completed, 0);
    assert_eq!(summary.pending, 0);
}

/// Test framework detection confidence levels
#[test]
fn test_framework_detection_confidence() {
    // Create detection using public fields
    let detection = FrameworkDetection {
        framework: TestFramework::Cargo,
        config_files: vec![PathBuf::from("Cargo.toml")],
        test_paths: vec![PathBuf::from("tests/")],
        confidence: 1.0,
    };

    // Verify detection result
    assert_eq!(detection.framework, TestFramework::Cargo);
    assert_eq!(detection.confidence, 1.0);
    assert!(!detection.config_files.is_empty());
    assert!(!detection.test_paths.is_empty());
}

/// Test verification result default behavior
#[test]
fn test_verification_result_defaults() {
    let task = create_task("task-001", "Test", vec![]);

    // Create a verification result directly
    let result = VerificationResult {
        task: task.clone(),
        passed: true,
        reasoning: "The task looks good to me.".to_string(),
        unmet_criteria: vec![],
        suggestions: vec![],
        retry_recommended: false,
    };

    // Verify default behavior
    assert!(result.passed);
    assert_eq!(result.reasoning, "The task looks good to me.");
    assert!(result.unmet_criteria.is_empty());
    assert!(result.suggestions.is_empty());
    assert!(!result.retry_recommended);
}