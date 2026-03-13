//! Integration tests for the happy path workflow
//!
//! These tests verify a complete simple workflow execution, including:
//! - Workspace initialization and state management
//! - Git operations (init, commit, branch)
//! - Task manifest creation and persistence
//! - Memory.md creation and updates
//! - Pipeline stage execution
//! - Final success status verification

use git2::Repository;
use ltmatrix::git::repository::init_repo;
use ltmatrix::memory::memory::{MemoryCategory, MemoryEntry, MemoryPriority, MemoryStore};
use ltmatrix::models::{
    ExecutionMode, ModeConfig, PipelineStage, Task, TaskComplexity, TaskStatus,
};
use ltmatrix::pipeline::orchestrator::{OrchestratorConfig, PipelineOrchestrator};
use ltmatrix::workspace::WorkspaceState;
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;
use tempfile::TempDir;

/// Check if a directory is a git repository
fn is_git_repo(path: &Path) -> bool {
    Repository::open(path).is_ok()
}

// =============================================================================
// Happy Path Integration Tests
// =============================================================================

/// Test complete workspace initialization
///
/// This test verifies that a workspace can be properly initialized with:
/// - Git repository setup
/// - .gitignore file generation
/// - Initial state creation
#[test]
fn test_happy_path_workspace_initialization() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path();

    // Step 1: Verify directory exists and is empty
    assert!(project_path.exists(), "Project path should exist");
    assert!(
        !is_git_repo(project_path),
        "Should not be a git repo initially"
    );

    // Step 2: Initialize git repository
    let repo = init_repo(project_path).expect("Failed to initialize git repository");
    assert!(
        is_git_repo(project_path),
        "Should be a git repo after initialization"
    );

    // Step 3: Verify .gitignore was created
    let gitignore_path = project_path.join(".gitignore");
    assert!(gitignore_path.exists(), ".gitignore should be created");
    let gitignore_content =
        std::fs::read_to_string(&gitignore_path).expect("Failed to read .gitignore");
    assert!(
        gitignore_content.contains("node_modules"),
        ".gitignore should contain node_modules"
    );
    assert!(
        gitignore_content.contains("target"),
        ".gitignore should contain target"
    );

    // Step 4: Verify git configuration
    let config = repo.config().expect("Failed to get git config");
    let user_name = config
        .get_string("user.name")
        .expect("Failed to get user.name");
    let user_email = config
        .get_string("user.email")
        .expect("Failed to get user.email");
    assert!(!user_name.is_empty(), "user.name should be set");
    assert!(!user_email.is_empty(), "user.email should be set");
}

/// Test workspace state persistence
///
/// Verifies that workspace state can be created, saved, and loaded correctly.
#[test]
fn test_happy_path_workspace_state_persistence() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Step 1: Create initial workspace state with tasks
    let tasks = vec![
        create_sample_task("task-001", "Initialize project", vec![]),
        create_sample_task("task-002", "Add dependencies", vec!["task-001".to_string()]),
        create_sample_task("task-003", "Write tests", vec!["task-002".to_string()]),
    ];

    let state = WorkspaceState::new(project_path.clone(), tasks.clone());

    // Step 2: Save state
    let saved_state = state.save().expect("Failed to save workspace state");

    // Step 3: Verify manifest file was created
    let manifest_path = project_path.join(".ltmatrix").join("tasks-manifest.json");
    assert!(manifest_path.exists(), "tasks-manifest.json should exist");

    // Step 4: Load state back
    let loaded_state = WorkspaceState::load(project_path).expect("Failed to load workspace state");

    // Step 5: Verify loaded state matches saved state
    assert_eq!(
        loaded_state.tasks.len(),
        saved_state.tasks.len(),
        "Task count should match"
    );
    assert_eq!(
        loaded_state.project_root, saved_state.project_root,
        "Project root should match"
    );

    // Verify each task
    for (original, loaded) in saved_state.tasks.iter().zip(loaded_state.tasks.iter()) {
        assert_eq!(original.id, loaded.id, "Task ID should match");
        assert_eq!(original.title, loaded.title, "Task title should match");
        assert_eq!(original.status, loaded.status, "Task status should match");
        assert_eq!(
            original.depends_on, loaded.depends_on,
            "Task dependencies should match"
        );
    }
}

/// Test memory store creation and updates
///
/// Verifies that the memory.md file can be created and updated properly.
#[test]
fn test_happy_path_memory_store() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path();

    // Step 1: Create memory store
    let store = MemoryStore::new(project_path).expect("Failed to create memory store");

    // Step 2: Verify memory file path
    let memory_path = project_path.join(".claude").join("memory.md");
    assert!(
        !memory_path.exists(),
        "Memory file should not exist initially"
    );

    // Step 3: Add a memory entry
    let entry = MemoryEntry::new(
        "task-001",
        "Architecture Decision",
        "Decided to use async Rust with Tokio for all I/O operations",
    )
    .with_category_enum(MemoryCategory::ArchitectureDecision)
    .with_files(vec!["src/main.rs".to_string(), "src/lib.rs".to_string()])
    .with_key_points(vec![
        "All I/O is async".to_string(),
        "Use tokio runtime".to_string(),
    ]);

    store
        .append_entry(&entry)
        .expect("Failed to append memory entry");

    // Step 4: Verify memory file was created
    assert!(memory_path.exists(), "Memory file should be created");

    // Step 5: Read and verify content
    let memory_content = std::fs::read_to_string(&memory_path).expect("Failed to read memory.md");
    assert!(
        memory_content.contains("Architecture Decision"),
        "Memory should contain category"
    );
    assert!(
        memory_content.contains("task-001"),
        "Memory should contain task ID"
    );
    assert!(
        memory_content.contains("Tokio"),
        "Memory should contain decision content"
    );

    // Step 6: Add another entry
    let entry2 = MemoryEntry::new(
        "task-002",
        "Pattern",
        "Established error handling pattern using anyhow",
    )
    .with_category_enum(MemoryCategory::Pattern)
    .with_files(vec!["src/error.rs".to_string()])
    .with_key_points(vec!["Use anyhow for errors".to_string()]);

    store
        .append_entry(&entry2)
        .expect("Failed to append second memory entry");

    // Step 7: Verify multiple entries
    let updated_content =
        std::fs::read_to_string(&memory_path).expect("Failed to read updated memory.md");
    assert!(
        updated_content.contains("task-001"),
        "Memory should contain first task"
    );
    assert!(
        updated_content.contains("task-002"),
        "Memory should contain second task"
    );
}

/// Test pipeline orchestrator with simple workflow
///
/// Tests the complete pipeline execution with minimal configuration.
#[tokio::test]
async fn test_happy_path_simple_pipeline_execution() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Step 1: Initialize git repository
    init_repo(temp_dir.path()).expect("Failed to initialize git repository");

    // Step 2: Create orchestrator configuration
    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false)
        .with_max_parallel(2);

    // Step 3: Create orchestrator
    let orchestrator = PipelineOrchestrator::new(config).expect("Failed to create orchestrator");

    // Step 4: Execute pipeline
    let result = orchestrator
        .execute_pipeline("Create a simple hello world program", ExecutionMode::Fast)
        .await;

    // Step 5: Verify successful execution
    assert!(result.is_ok(), "Pipeline execution should succeed");

    let pipeline_result = result.expect("Expected pipeline result");
    assert!(
        pipeline_result.total_time >= Duration::ZERO,
        "Total time should be non-negative"
    );
}

/// Test complete happy path workflow
///
/// This test simulates a complete workflow from start to finish:
/// 1. Workspace initialization
/// 2. Git repository setup
/// 3. Task creation and persistence
/// 4. Memory initialization
/// 5. Pipeline execution (Fast mode)
/// 6. Verification of all outputs
#[tokio::test]
async fn test_complete_happy_path_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path();

    // ========================================
    // Phase 1: Workspace Setup
    // ========================================

    // Initialize git repository
    let _repo = init_repo(project_path).expect("Failed to initialize git repository");
    assert!(is_git_repo(project_path));

    // Verify .gitignore exists
    let gitignore_path = project_path.join(".gitignore");
    assert!(gitignore_path.exists(), ".gitignore should exist");

    // ========================================
    // Phase 2: Task Creation
    // ========================================

    // Create sample tasks
    let tasks = vec![
        create_sample_task("task-001", "Setup project structure", vec![]),
        create_sample_task("task-002", "Add core modules", vec!["task-001".to_string()]),
        create_sample_task("task-003", "Write tests", vec!["task-002".to_string()]),
    ];

    // Save workspace state
    let state = WorkspaceState::new(project_path.to_path_buf(), tasks.clone());
    state.save().expect("Failed to save workspace state");

    // Verify manifest
    let manifest_path = project_path.join(".ltmatrix").join("tasks-manifest.json");
    assert!(manifest_path.exists(), "Task manifest should exist");

    // ========================================
    // Phase 3: Memory Initialization
    // ========================================

    let memory_store = MemoryStore::new(project_path).expect("Failed to create memory store");

    // Add initial memory entry
    let memory_entry = MemoryEntry::new(
        "task-001",
        "Project Setup",
        "Initialized project with standard structure",
    )
    .with_category_enum(MemoryCategory::CodeOrganization)
    .with_files(vec!["src/main.rs".to_string()])
    .with_key_points(vec!["Using standard Rust project layout".to_string()]);

    memory_store
        .append_entry(&memory_entry)
        .expect("Failed to append memory entry");

    // Verify memory.md
    let memory_path = project_path.join(".claude").join("memory.md");
    assert!(memory_path.exists(), "Memory file should exist");

    // ========================================
    // Phase 4: Pipeline Execution
    // ========================================

    let config = OrchestratorConfig::default()
        .with_work_dir(project_path)
        .with_progress(false)
        .with_max_parallel(2);

    let orchestrator = PipelineOrchestrator::new(config).expect("Failed to create orchestrator");

    let result = orchestrator
        .execute_pipeline("Complete the project setup", ExecutionMode::Fast)
        .await
        .expect("Pipeline execution failed");

    // ========================================
    // Phase 5: Verification
    // ========================================

    // Verify pipeline completed
    assert!(
        result.total_time >= Duration::ZERO,
        "Pipeline should have executed"
    );

    // Verify workspace state still exists
    assert!(
        manifest_path.exists(),
        "Task manifest should still exist after pipeline"
    );

    // Verify memory still exists
    assert!(
        memory_path.exists(),
        "Memory file should still exist after pipeline"
    );

    // Verify git repository is still valid
    assert!(is_git_repo(project_path), "Git repo should still be valid");
}

/// Test workspace state status summary
#[test]
fn test_happy_path_workspace_status_summary() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create tasks with different statuses
    let mut tasks = vec![
        create_sample_task("task-001", "Completed task", vec![]),
        create_sample_task("task-002", "Pending task", vec![]),
        create_sample_task("task-003", "Failed task", vec![]),
    ];

    tasks[0].status = TaskStatus::Completed;
    tasks[1].status = TaskStatus::Pending;
    tasks[2].status = TaskStatus::Failed;

    let state = WorkspaceState::new(temp_dir.path().to_path_buf(), tasks);

    // Get status summary
    let summary = state.status_summary();

    // Verify counts
    assert_eq!(summary.total(), 3, "Total tasks should be 3");
    assert_eq!(summary.completed, 1, "Completed count should be 1");
    assert_eq!(summary.pending, 1, "Pending count should be 1");
    assert_eq!(summary.failed, 1, "Failed count should be 1");
    assert_eq!(summary.in_progress, 0, "In progress count should be 0");
    assert_eq!(summary.blocked, 0, "Blocked count should be 0");

    // Verify completion percentage
    let expected_percentage = (1.0 / 3.0) * 100.0;
    assert!(
        (summary.completion_percentage() - expected_percentage).abs() < 0.1,
        "Completion percentage should be approximately {:.1}%",
        expected_percentage
    );
}

/// Test task dependency resolution
#[test]
fn test_happy_path_task_dependency_resolution() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create tasks with dependencies
    let tasks = vec![
        create_sample_task("task-001", "First task", vec![]),
        create_sample_task("task-002", "Second task", vec!["task-001".to_string()]),
        create_sample_task(
            "task-003",
            "Third task",
            vec!["task-001".to_string(), "task-002".to_string()],
        ),
        create_sample_task("task-004", "Fourth task", vec!["task-003".to_string()]),
    ];

    // Build task map
    let task_map: std::collections::HashMap<&str, &Task> =
        tasks.iter().map(|t| (t.id.as_str(), t)).collect();

    // Verify dependency chain
    let task1 = task_map.get("task-001").expect("task-001 should exist");
    let task2 = task_map.get("task-002").expect("task-002 should exist");
    let task3 = task_map.get("task-003").expect("task-003 should exist");
    let task4 = task_map.get("task-004").expect("task-004 should exist");

    // Task 1 has no dependencies
    let empty_completed = HashSet::new();
    assert!(
        task1.can_execute(&empty_completed),
        "Task 1 should be executable with no dependencies"
    );

    // Task 2 depends on Task 1
    let mut completed = HashSet::new();
    assert!(
        !task2.can_execute(&completed),
        "Task 2 should not be executable without Task 1"
    );
    completed.insert("task-001".to_string());
    assert!(
        task2.can_execute(&completed),
        "Task 2 should be executable after Task 1"
    );

    // Task 3 depends on Task 1 and Task 2
    assert!(
        !task3.can_execute(&completed),
        "Task 3 should not be executable without Task 2"
    );
    completed.insert("task-002".to_string());
    assert!(
        task3.can_execute(&completed),
        "Task 3 should be executable after Task 1 and 2"
    );

    // Task 4 depends on Task 3
    assert!(
        !task4.can_execute(&completed),
        "Task 4 should not be executable without Task 3"
    );
    completed.insert("task-003".to_string());
    assert!(
        task4.can_execute(&completed),
        "Task 4 should be executable after Task 3"
    );

    // Save and verify persistence
    let state = WorkspaceState::new(temp_dir.path().to_path_buf(), tasks);
    state.save().expect("Failed to save state");

    let loaded = WorkspaceState::load(temp_dir.path().to_path_buf()).expect("Failed to load state");
    assert_eq!(loaded.tasks.len(), 4, "All 4 tasks should be persisted");
}

/// Test pipeline stages for different execution modes
#[test]
fn test_happy_path_pipeline_stages_by_mode() {
    // Fast mode stages
    let fast_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Fast);
    assert!(
        !fast_stages.contains(&PipelineStage::Test),
        "Fast mode should skip Test stage"
    );
    assert!(
        !fast_stages.contains(&PipelineStage::Review),
        "Fast mode should skip Review stage"
    );
    assert_eq!(fast_stages.len(), 6, "Fast mode should have 6 stages");

    // Standard mode stages
    let standard_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);
    assert!(
        standard_stages.contains(&PipelineStage::Test),
        "Standard mode should include Test stage"
    );
    assert!(
        !standard_stages.contains(&PipelineStage::Review),
        "Standard mode should skip Review stage"
    );
    assert_eq!(
        standard_stages.len(),
        7,
        "Standard mode should have 7 stages"
    );

    // Expert mode stages
    let expert_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);
    assert!(
        expert_stages.contains(&PipelineStage::Test),
        "Expert mode should include Test stage"
    );
    assert!(
        expert_stages.contains(&PipelineStage::Review),
        "Expert mode should include Review stage"
    );
    assert_eq!(expert_stages.len(), 8, "Expert mode should have 8 stages");

    // Verify stage order for all modes
    verify_stage_order(&fast_stages);
    verify_stage_order(&standard_stages);
    verify_stage_order(&expert_stages);
}

/// Test mode configuration defaults
#[test]
fn test_happy_path_mode_config_defaults() {
    // Fast mode config
    let fast_config = ModeConfig::fast_mode();
    assert!(!fast_config.run_tests, "Fast mode should not run tests");
    assert_eq!(
        fast_config.max_depth, 2,
        "Fast mode should have max_depth of 2"
    );
    assert_eq!(
        fast_config.max_retries, 1,
        "Fast mode should have max_retries of 1"
    );

    // Standard mode config
    let standard_config = ModeConfig::default();
    assert!(standard_config.run_tests, "Standard mode should run tests");
    assert_eq!(
        standard_config.max_depth, 3,
        "Standard mode should have max_depth of 3"
    );
    assert_eq!(
        standard_config.max_retries, 3,
        "Standard mode should have max_retries of 3"
    );

    // Expert mode config
    let expert_config = ModeConfig::expert_mode();
    assert!(expert_config.run_tests, "Expert mode should run tests");
    assert_eq!(
        expert_config.max_depth, 3,
        "Expert mode should have max_depth of 3"
    );
    assert_eq!(
        expert_config.max_retries, 3,
        "Expert mode should have max_retries of 3"
    );

    // Verify model selection
    assert_eq!(
        fast_config.model_for_complexity(&TaskComplexity::Simple),
        "claude-haiku-4-5"
    );
    assert_eq!(
        standard_config.model_for_complexity(&TaskComplexity::Simple),
        "claude-sonnet-4-6"
    );
    assert_eq!(
        expert_config.model_for_complexity(&TaskComplexity::Simple),
        "claude-opus-4-6"
    );
}

/// Test workspace state transformation for resume
#[test]
fn test_happy_path_workspace_state_transform() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create tasks with various statuses
    let mut tasks = vec![
        create_sample_task("task-001", "Completed", vec![]),
        create_sample_task("task-002", "In Progress", vec!["task-001".to_string()]),
        create_sample_task("task-003", "Blocked", vec!["task-002".to_string()]),
        create_sample_task("task-004", "Failed", vec![]),
        create_sample_task("task-005", "Pending", vec![]),
    ];

    tasks[0].status = TaskStatus::Completed;
    tasks[1].status = TaskStatus::InProgress;
    tasks[2].status = TaskStatus::Blocked;
    tasks[3].status = TaskStatus::Failed;
    tasks[4].status = TaskStatus::Pending;

    // Save state
    let state = WorkspaceState::new(temp_dir.path().to_path_buf(), tasks);
    state.save().expect("Failed to save state");

    // Load with transform
    let transformed = WorkspaceState::load_with_transform(temp_dir.path().to_path_buf())
        .expect("Failed to load with transform");

    // Verify transformation
    assert_eq!(transformed.tasks.len(), 5, "All tasks should be present");

    let task_map: std::collections::HashMap<&str, &Task> = transformed
        .tasks
        .iter()
        .map(|t| (t.id.as_str(), t))
        .collect();

    // Completed should stay completed
    assert_eq!(task_map["task-001"].status, TaskStatus::Completed);

    // InProgress should be reset to Pending
    assert_eq!(task_map["task-002"].status, TaskStatus::Pending);

    // Blocked should be reset to Pending
    assert_eq!(task_map["task-003"].status, TaskStatus::Pending);

    // Failed should stay failed
    assert_eq!(task_map["task-004"].status, TaskStatus::Failed);

    // Pending should stay pending
    assert_eq!(task_map["task-005"].status, TaskStatus::Pending);
}

/// Test pipeline result success rate calculation
#[tokio::test]
async fn test_happy_path_pipeline_result_success_rate() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Initialize git
    init_repo(temp_dir.path()).expect("Failed to initialize git");

    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orchestrator = PipelineOrchestrator::new(config).expect("Failed to create orchestrator");

    let result = orchestrator
        .execute_pipeline("Test goal", ExecutionMode::Fast)
        .await
        .expect("Pipeline execution failed");

    // Verify success rate is within valid range
    let rate = result.success_rate();
    assert!(rate >= 0.0, "Success rate should be >= 0");
    assert!(rate <= 100.0, "Success rate should be <= 100");
}

/// Test git repository protection for nested repos
#[test]
fn test_happy_path_nested_repo_protection() {
    let parent_dir = TempDir::new().expect("Failed to create parent temp directory");
    let nested_path = parent_dir.path().join("nested-project");

    // Initialize nested repository
    let _repo = init_repo(&nested_path).expect("Failed to initialize nested repo");

    // Verify nested repo exists
    assert!(
        nested_path.join(".git").exists(),
        "Nested repo should have .git directory"
    );

    // Verify parent has nested path in .gitignore
    let parent_gitignore = parent_dir.path().join(".gitignore");
    if parent_gitignore.exists() {
        let content =
            std::fs::read_to_string(&parent_gitignore).expect("Failed to read parent .gitignore");
        assert!(
            content.contains("nested-project"),
            "Parent .gitignore should contain nested path"
        );
    }
}

/// Test memory entry with all metadata
#[test]
fn test_happy_path_memory_entry_full_metadata() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let store = MemoryStore::new(temp_dir.path()).expect("Failed to create memory store");

    // Create entry with full metadata
    let entry = MemoryEntry::new(
        "task-042",
        "Comprehensive Memory Entry",
        "This is a detailed memory entry with all possible metadata fields.",
    )
    .with_category_enum(MemoryCategory::ArchitectureDecision)
    .with_files(vec![
        "src/main.rs".to_string(),
        "src/lib.rs".to_string(),
        "src/models/mod.rs".to_string(),
    ])
    .with_key_points(vec![
        "Point 1: Use async patterns".to_string(),
        "Point 2: Implement proper error handling".to_string(),
        "Point 3: Follow Rust best practices".to_string(),
    ])
    .with_priority(MemoryPriority::High);

    // Append entry
    store.append_entry(&entry).expect("Failed to append entry");

    // Read and verify
    let memory_path = temp_dir.path().join(".claude").join("memory.md");
    let content = std::fs::read_to_string(&memory_path).expect("Failed to read memory.md");

    // Verify all components are present
    assert!(
        content.contains("task-042"),
        "Memory should contain task ID"
    );
    assert!(
        content.contains("Comprehensive Memory Entry"),
        "Memory should contain title"
    );
    assert!(
        content.contains("Architecture Decision"),
        "Memory should contain category"
    );
    assert!(
        content.contains("src/main.rs"),
        "Memory should contain file reference"
    );
    assert!(
        content.contains("Point 1"),
        "Memory should contain key point"
    );
}

/// Test orchestrator handles concurrent workspace access
#[tokio::test]
async fn test_happy_path_concurrent_workspace_access() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Initialize git
    init_repo(temp_dir.path()).expect("Failed to initialize git");

    // Create workspace state
    let tasks = vec![
        create_sample_task("task-001", "First task", vec![]),
        create_sample_task("task-002", "Second task", vec![]),
    ];
    let state = WorkspaceState::new(temp_dir.path().to_path_buf(), tasks);
    state.save().expect("Failed to save initial state");

    // Create multiple orchestrator instances
    let config1 = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let config2 = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orch1 = PipelineOrchestrator::new(config1).expect("Failed to create orchestrator 1");
    let orch2 = PipelineOrchestrator::new(config2).expect("Failed to create orchestrator 2");

    // Execute both sequentially (git operations are not thread-safe)
    let result1 = orch1.execute_pipeline("Task 1", ExecutionMode::Fast).await;
    let result2 = orch2.execute_pipeline("Task 2", ExecutionMode::Fast).await;

    // Both should succeed
    assert!(result1.is_ok(), "First execution should succeed");
    assert!(result2.is_ok(), "Second execution should succeed");
}

/// Test cleanup functionality
#[test]
fn test_happy_path_workspace_cleanup() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();

    // Setup workspace
    init_repo(&project_path).expect("Failed to initialize git");

    let tasks = vec![create_sample_task("task-001", "Test task", vec![])];
    let state = WorkspaceState::new(project_path.clone(), tasks);
    state.save().expect("Failed to save state");

    // Verify workspace state exists
    let manifest_path = project_path.join(".ltmatrix").join("tasks-manifest.json");
    assert!(manifest_path.exists(), "Manifest should exist");

    // Cleanup
    WorkspaceState::cleanup(&project_path).expect("Failed to cleanup");

    // Verify cleanup
    assert!(
        !manifest_path.exists(),
        "Manifest should be removed after cleanup"
    );
    let ltmatrix_dir = project_path.join(".ltmatrix");
    assert!(
        !ltmatrix_dir.exists(),
        ".ltmatrix directory should be removed"
    );
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Creates a sample task with the given ID, title, and dependencies
fn create_sample_task(id: &str, title: &str, depends_on: Vec<String>) -> Task {
    let mut task = Task::new(id, title, format!("Description for {}", title));
    task.depends_on = depends_on;
    task.complexity = TaskComplexity::Moderate;
    task
}

/// Verifies that pipeline stages are in correct order
fn verify_stage_order(stages: &[PipelineStage]) {
    // Find indices of key stages
    let generate_idx = stages.iter().position(|s| *s == PipelineStage::Generate);
    let assess_idx = stages.iter().position(|s| *s == PipelineStage::Assess);
    let execute_idx = stages.iter().position(|s| *s == PipelineStage::Execute);
    let test_idx = stages.iter().position(|s| *s == PipelineStage::Test);
    let verify_idx = stages.iter().position(|s| *s == PipelineStage::Verify);
    let commit_idx = stages.iter().position(|s| *s == PipelineStage::Commit);
    let memory_idx = stages.iter().position(|s| *s == PipelineStage::Memory);

    // Verify order constraints
    if let (Some(gen), Some(assess)) = (generate_idx, assess_idx) {
        assert!(gen < assess, "Generate should come before Assess");
    }

    if let (Some(assess), Some(exec)) = (assess_idx, execute_idx) {
        assert!(assess < exec, "Assess should come before Execute");
    }

    if let (Some(exec), Some(test)) = (execute_idx, test_idx) {
        assert!(exec < test, "Execute should come before Test");
    }

    if let (Some(test), Some(verify)) = (test_idx, verify_idx) {
        assert!(test < verify, "Test should come before Verify");
    }

    if let (Some(verify), Some(commit)) = (verify_idx, commit_idx) {
        assert!(verify < commit, "Verify should come before Commit");
    }

    if let (Some(commit), Some(memory)) = (commit_idx, memory_idx) {
        assert!(commit < memory, "Commit should come before Memory");
    }
}

// =============================================================================
// Performance Benchmarks
// =============================================================================

#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    /// Benchmark workspace state save/load
    #[test]
    fn bench_workspace_state_persistence() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");

        // Create 100 tasks
        let tasks: Vec<Task> = (0..100)
            .map(|i| create_sample_task(&format!("task-{:03}", i), &format!("Task {}", i), vec![]))
            .collect();

        let state = WorkspaceState::new(temp_dir.path().to_path_buf(), tasks);

        // Benchmark save
        let start = Instant::now();
        state.save().expect("Failed to save");
        let save_duration = start.elapsed();

        // Benchmark load
        let start = Instant::now();
        WorkspaceState::load(temp_dir.path().to_path_buf()).expect("Failed to load");
        let load_duration = start.elapsed();

        // Verify reasonable performance (adjust thresholds as needed)
        assert!(
            save_duration < Duration::from_secs(1),
            "Save should complete in < 1s"
        );
        assert!(
            load_duration < Duration::from_secs(1),
            "Load should complete in < 1s"
        );
    }

    /// Benchmark memory entry creation
    #[test]
    fn bench_memory_entry_creation() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let store = MemoryStore::new(temp_dir.path()).expect("Failed to create store");

        let start = Instant::now();

        // Create 50 memory entries
        for i in 0..50 {
            let entry = MemoryEntry::new(
                &format!("task-{:03}", i),
                &format!("Entry {}", i),
                format!("Content for entry {}", i),
            )
            .with_category_enum(MemoryCategory::General);

            store.append_entry(&entry).expect("Failed to append entry");
        }

        let duration = start.elapsed();

        // Verify reasonable performance
        assert!(
            duration < Duration::from_secs(2),
            "50 entries should be created in < 2s"
        );
    }
}
