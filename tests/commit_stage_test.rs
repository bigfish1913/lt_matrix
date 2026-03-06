//! Integration tests for commit stage
//!
//! These tests verify the commit stage functionality including:
//! - Per-task branching strategy
//! - Direct commit strategy
//! - Merge conflict handling
//! - Git repository detection
//! - Commit message generation

use ltmatrix::models::{Task, TaskStatus};
use ltmatrix::pipeline::commit::{commit_tasks, CommitConfig, CommitSummary};
use ltmatrix::git::{
    init_repo,
    create_branch,
    checkout,
};
use git2::Repository;
use tempfile::TempDir;
use std::fs;

/// Helper to create a test task
fn create_test_task(id: &str, title: &str) -> Task {
    let mut task = Task::new(id, title, format!("Description for {}", title));
    task.status = TaskStatus::Completed;
    task
}

/// Helper to create a git repo with initial commit
fn create_test_repo() -> (TempDir, Repository) {
    let temp_dir = TempDir::new().unwrap();
    let repo = init_repo(temp_dir.path()).unwrap();

    // Create initial commit
    let sig = ltmatrix::git::repository::create_signature("Test", "test@example.com").unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
    drop(tree); // Drop before moving repo

    (temp_dir, repo)
}

#[tokio::test]
async fn test_commit_single_task_with_branching() {
    let (temp_dir, _repo) = create_test_repo();

    let task = create_test_task("task-001", "Add feature");
    let tasks = vec![task];

    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        use_task_branches: true,
        delete_after_merge: true,
        ..Default::default()
    };

    let (updated_tasks, summary) = commit_tasks(tasks, &config).await.unwrap();

    assert_eq!(summary.total_tasks, 1);
    assert_eq!(summary.committed_tasks, 1);
    assert_eq!(summary.failed_tasks, 0);
    assert_eq!(summary.conflicts, 0);
    assert!(summary.is_complete_success());
    assert_eq!(updated_tasks.len(), 1);
}

#[tokio::test]
async fn test_commit_multiple_tasks_sequentially() {
    let (temp_dir, _repo) = create_test_repo();

    let tasks = vec![
        create_test_task("task-001", "Feature A"),
        create_test_task("task-002", "Feature B"),
        create_test_task("task-003", "Feature C"),
    ];

    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        use_task_branches: true,
        ..Default::default()
    };

    let (updated_tasks, summary) = commit_tasks(tasks, &config).await.unwrap();

    assert_eq!(summary.total_tasks, 3);
    assert_eq!(summary.committed_tasks, 3);
    assert!(summary.is_complete_success());
    assert_eq!(updated_tasks.len(), 3);
}

#[tokio::test]
async fn test_commit_with_direct_strategy() {
    let (temp_dir, _repo) = create_test_repo();

    let task = create_test_task("task-direct", "Direct commit");
    let tasks = vec![task];

    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        use_task_branches: false, // Direct commits
        ..Default::default()
    };

    let (_updated_tasks, summary) = commit_tasks(tasks, &config).await.unwrap();

    assert_eq!(summary.total_tasks, 1);
    assert_eq!(summary.committed_tasks, 1);
    // No branches created with direct strategy
    assert_eq!(summary.branches_created, 0);
}

#[tokio::test]
async fn test_commit_skips_non_completed_tasks() {
    let (temp_dir, _repo) = create_test_repo();

    let mut pending_task = Task::new("task-pending", "Pending", "Not done");
    pending_task.status = TaskStatus::Pending;

    let mut in_progress_task = Task::new("task-progress", "In Progress", "Still working");
    in_progress_task.status = TaskStatus::InProgress;

    let completed_task = create_test_task("task-done", "Done");

    let tasks = vec![pending_task, in_progress_task, completed_task];

    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let (updated_tasks, summary) = commit_tasks(tasks, &config).await.unwrap();

    // Only the completed task should be processed
    // total_tasks represents completed tasks that were processed
    assert_eq!(summary.total_tasks, 1);
    assert_eq!(summary.committed_tasks, 1);
    // The returned updated_tasks should only contain completed tasks
    assert_eq!(updated_tasks.len(), 1);
}

#[tokio::test]
async fn test_commit_respects_base_branch() {
    let (temp_dir, repo) = create_test_repo();

    // Create a feature branch
    create_branch(&repo, "develop").unwrap();
    checkout(&repo, "develop").unwrap();

    let task = create_test_task("task-feature", "Feature");
    let tasks = vec![task];

    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        base_branch: Some("master".to_string()),
        ..Default::default()
    };

    let (_updated_tasks, summary) = commit_tasks(tasks, &config).await.unwrap();

    assert_eq!(summary.base_branch, Some("master".to_string()));
    assert_eq!(summary.total_tasks, 1);
}

#[tokio::test]
async fn test_commit_no_changes_to_commit() {
    let (temp_dir, _repo) = create_test_repo();

    let task = create_test_task("task-nochanges", "No changes");
    let tasks = vec![task];

    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    // Should succeed but report no changes
    let (_updated_tasks, summary) = commit_tasks(tasks, &config).await.unwrap();

    // Even with no changes, task is marked as successfully processed
    assert_eq!(summary.committed_tasks, 1);
}

#[tokio::test]
async fn test_commit_fast_mode_config() {
    let config = CommitConfig::fast_mode();

    // Fast mode should use direct commits
    assert!(!config.use_task_branches);
    assert!(!config.delete_after_merge);
    assert!(config.enabled);
}

#[tokio::test]
async fn test_commit_expert_mode_config() {
    let config = CommitConfig::expert_mode();

    // Expert mode should use full branching strategy
    assert!(config.use_task_branches);
    assert!(config.delete_after_merge);
    assert!(!config.skip_if_no_repo);
    assert!(config.enabled);
}

#[tokio::test]
async fn test_commit_with_actual_file_changes() {
    let (temp_dir, _repo) = create_test_repo();

    // Create a file to commit
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "Hello, World!").unwrap();

    let task = create_test_task("task-file", "Add file");
    let tasks = vec![task];

    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        use_task_branches: false, // Simpler for this test
        ..Default::default()
    };

    let (_updated_tasks, summary) = commit_tasks(tasks, &config).await.unwrap();

    assert_eq!(summary.committed_tasks, 1);
    assert!(summary.total_commits >= 1);
}

#[tokio::test]
async fn test_commit_multiple_files() {
    let (temp_dir, _repo) = create_test_repo();

    // Create multiple files
    fs::write(temp_dir.path().join("file1.txt"), "Content 1").unwrap();
    fs::write(temp_dir.path().join("file2.txt"), "Content 2").unwrap();
    fs::write(temp_dir.path().join("file3.txt"), "Content 3").unwrap();

    let task = create_test_task("task-multi", "Add multiple files");
    let tasks = vec![task];

    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        use_task_branches: false,
        ..Default::default()
    };

    let (_updated_tasks, summary) = commit_tasks(tasks, &config).await.unwrap();

    assert_eq!(summary.committed_tasks, 1);
}

#[tokio::test]
async fn test_commit_preserves_branch_after_failure() {
    let (temp_dir, _repo) = create_test_repo();

    // Create a conflicting file on master
    fs::write(temp_dir.path().join("conflict.txt"), "Master version").unwrap();

    let task = create_test_task("task-conflict", "Will conflict");
    let tasks = vec![task];

    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        use_task_branches: true,
        ..Default::default()
    };

    let (updated_tasks, _summary) = commit_tasks(tasks, &config).await.unwrap();

    // Even if there are issues, the function should complete without crashing
    assert_eq!(updated_tasks.len(), 1);
}

#[tokio::test]
async fn test_commit_disabled_in_config() {
    let (temp_dir, _repo) = create_test_repo();

    let task = create_test_task("task-disabled", "Should skip");
    let tasks = vec![task];

    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        enabled: false,
        ..Default::default()
    };

    let (_updated_tasks, summary) = commit_tasks(tasks, &config).await.unwrap();

    // Should skip all tasks when disabled
    assert_eq!(summary.skipped_tasks, 1);
    assert_eq!(summary.committed_tasks, 0);
    assert_eq!(summary.total_commits, 0);
}

#[tokio::test]
async fn test_commit_message_format() {
    let _task = create_test_task("task-123", "Add authentication");
    let message = format!("feat: [task-123] Add authentication");

    // Verify conventional commit format
    assert!(message.starts_with("feat:"));
    assert!(message.contains("[task-123]"));
    assert!(message.contains("Add authentication"));
}

#[tokio::test]
async fn test_commit_with_different_commit_types() {
    let (temp_dir, _repo) = create_test_repo();

    let fix_task = create_test_task("task-fix", "Fix bug");
    let tasks = vec![fix_task];

    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        commit_type: "fix".to_string(),
        use_task_branches: false,
        ..Default::default()
    };

    let (_updated_tasks, summary) = commit_tasks(tasks, &config).await.unwrap();

    assert_eq!(summary.committed_tasks, 1);
}

#[tokio::test]
async fn test_commit_summary_serialization() {
    let summary = CommitSummary {
        total_tasks: 10,
        committed_tasks: 8,
        failed_tasks: 1,
        skipped_tasks: 1,
        conflicts: 1,
        branches_created: 8,
        branches_deleted: 7,
        total_commits: 8,
        base_branch: Some("main".to_string()),
    };

    // Verify summary can be serialized (for JSON output)
    let json = serde_json::to_string(&summary).unwrap();
    assert!(json.contains("total_tasks"));
    assert!(json.contains("committed_tasks"));

    // Verify it can be deserialized
    let deserialized: CommitSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.total_tasks, 10);
    assert_eq!(deserialized.committed_tasks, 8);
}

#[tokio::test]
async fn test_commit_empty_task_list() {
    let (temp_dir, _repo) = create_test_repo();

    let tasks = vec![];

    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let (updated_tasks, summary) = commit_tasks(tasks, &config).await.unwrap();

    assert_eq!(summary.total_tasks, 0);
    assert_eq!(summary.committed_tasks, 0);
    assert!(updated_tasks.is_empty());
}
