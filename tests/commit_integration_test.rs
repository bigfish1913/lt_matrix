//! Integration tests for the commit stage
//!
//! These tests verify the complete commit workflow including:
//! - Per-task branch creation
//! - Staging and committing changes
//! - Squash merge to base branch
//! - Conflict detection and handling
//! - Direct commit strategy

use ltmatrix::{
    git::repository::init_repo,
    models::{Task, TaskStatus},
    pipeline::commit::{commit_tasks, CommitConfig},
};
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_commit_stage_with_task_branches() {
    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo = init_repo(temp_dir.path()).unwrap();

    // Create an initial commit
    let sig = ltmatrix::git::repository::create_signature("Test", "test@example.com").unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    // Create a completed task
    let mut task = Task::new("task-001", "Add feature X", "Implement feature X");
    task.status = TaskStatus::Completed;

    // Make some changes
    let test_file = temp_dir.path().join("test.rs");
    fs::write(&test_file, "fn main() { println!(\"Hello\"); }").unwrap();

    // Commit with task branch strategy
    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        use_task_branches: true,
        delete_after_merge: true,
        ..Default::default()
    };

    let (updated_tasks, summary) = commit_tasks(vec![task.clone()], &config).await.unwrap();

    // Verify results
    assert_eq!(summary.total_tasks, 1);
    assert_eq!(summary.committed_tasks, 1);
    assert_eq!(summary.failed_tasks, 0);
    assert_eq!(summary.branches_created, 1);
    assert_eq!(summary.total_commits, 1);
    assert!(summary.is_complete_success());

    // Verify task was updated
    assert_eq!(updated_tasks.len(), 1);
    assert_eq!(updated_tasks[0].status, TaskStatus::Completed);
}

#[tokio::test]
async fn test_commit_stage_direct_commits() {
    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo = init_repo(temp_dir.path()).unwrap();

    // Create an initial commit
    let sig = ltmatrix::git::repository::create_signature("Test", "test@example.com").unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    // Create a completed task
    let mut task = Task::new("task-002", "Add feature Y", "Implement feature Y");
    task.status = TaskStatus::Completed;

    // Make some changes
    let test_file = temp_dir.path().join("direct.rs");
    fs::write(&test_file, "fn test() { }").unwrap();

    // Commit with direct strategy (no task branches)
    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        use_task_branches: false, // Direct commits
        ..Default::default()
    };

    let (_updated_tasks, summary) = commit_tasks(vec![task.clone()], &config).await.unwrap();

    // Verify results
    assert_eq!(summary.total_tasks, 1);
    assert_eq!(summary.committed_tasks, 1);
    assert_eq!(summary.failed_tasks, 0);
    assert_eq!(summary.branches_created, 0); // No branches with direct commits
    assert_eq!(summary.total_commits, 1);
    assert!(summary.is_complete_success());
}

#[tokio::test]
async fn test_commit_stage_no_changes() {
    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo = init_repo(temp_dir.path()).unwrap();

    // Create an initial commit
    let sig = ltmatrix::git::repository::create_signature("Test", "test@example.com").unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    // Create a completed task
    let mut task = Task::new("task-003", "No changes", "Task with no changes");
    task.status = TaskStatus::Completed;

    // Don't make any changes

    // Commit with task branch strategy
    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        use_task_branches: true,
        ..Default::default()
    };

    let (_updated_tasks, summary) = commit_tasks(vec![task.clone()], &config).await.unwrap();

    // Should succeed but with no actual commit
    assert_eq!(summary.total_tasks, 1);
    // When there are no changes, the task is still processed successfully
    // but no actual commit is created
    assert!(summary.is_complete_success());
}

#[tokio::test]
async fn test_commit_stage_multiple_tasks() {
    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo = init_repo(temp_dir.path()).unwrap();

    // Create an initial commit
    let sig = ltmatrix::git::repository::create_signature("Test", "test@example.com").unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    // Create multiple completed tasks
    let mut task1 = Task::new("task-004", "Feature A", "Implement A");
    task1.status = TaskStatus::Completed;

    let mut task2 = Task::new("task-005", "Feature B", "Implement B");
    task2.status = TaskStatus::Completed;

    // Make changes for first task
    let file1 = temp_dir.path().join("feature_a.rs");
    fs::write(&file1, "// Feature A").unwrap();

    // Commit first task
    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        use_task_branches: true,
        ..Default::default()
    };

    let (_updated_tasks1, summary1) = commit_tasks(vec![task1.clone()], &config).await.unwrap();

    assert_eq!(summary1.committed_tasks, 1);

    // Make changes for second task
    let file2 = temp_dir.path().join("feature_b.rs");
    fs::write(&file2, "// Feature B").unwrap();

    // Commit second task
    let (_updated_tasks2, summary2) = commit_tasks(vec![task2.clone()], &config).await.unwrap();

    assert_eq!(summary2.committed_tasks, 1);

    // Total: 2 tasks, 2 commits
    let total_commits = summary1.total_commits + summary2.total_commits;
    assert_eq!(total_commits, 2);
}

#[tokio::test]
async fn test_commit_stage_only_completed_tasks() {
    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo = init_repo(temp_dir.path()).unwrap();

    // Create an initial commit
    let sig = ltmatrix::git::repository::create_signature("Test", "test@example.com").unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    // Create tasks with different statuses
    let mut completed = Task::new("task-006", "Completed", "Done task");
    completed.status = TaskStatus::Completed;

    let pending = Task::new("task-007", "Pending", "Not done yet");

    let mut failed = Task::new("task-008", "Failed", "Failed task");
    failed.status = TaskStatus::Failed;

    // Make some changes
    let file = temp_dir.path().join("completed.rs");
    fs::write(&file, "// Completed task").unwrap();

    // Commit
    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let (updated_tasks, summary) = commit_tasks(
        vec![completed.clone(), pending.clone(), failed.clone()],
        &config,
    )
    .await
    .unwrap();

    // Only the completed task should be processed
    assert_eq!(summary.total_tasks, 1); // Only completed tasks are counted
    assert_eq!(summary.committed_tasks, 1);
    assert_eq!(updated_tasks.len(), 1);
    assert_eq!(updated_tasks[0].id, "task-006");
}

#[tokio::test]
async fn test_commit_stage_fast_mode() {
    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo = init_repo(temp_dir.path()).unwrap();

    // Create an initial commit
    let sig = ltmatrix::git::repository::create_signature("Test", "test@example.com").unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    // Create a completed task
    let mut task = Task::new("task-009", "Fast mode", "Quick commit");
    task.status = TaskStatus::Completed;

    // Make changes
    let file = temp_dir.path().join("fast.rs");
    fs::write(&file, "// Fast mode").unwrap();

    // Use fast mode config
    let config = CommitConfig::fast_mode();
    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        ..config
    };

    let (_updated_tasks, summary) = commit_tasks(vec![task.clone()], &config).await.unwrap();

    // Fast mode uses direct commits (no task branches)
    assert_eq!(summary.branches_created, 0);
    assert_eq!(summary.committed_tasks, 1);
    assert!(summary.is_complete_success());
}

#[tokio::test]
async fn test_commit_stage_expert_mode() {
    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo = init_repo(temp_dir.path()).unwrap();

    // Create an initial commit
    let sig = ltmatrix::git::repository::create_signature("Test", "test@example.com").unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    // Create a completed task
    let mut task = Task::new("task-010", "Expert mode", "Full workflow");
    task.status = TaskStatus::Completed;

    // Make changes
    let file = temp_dir.path().join("expert.rs");
    fs::write(&file, "// Expert mode").unwrap();

    // Use expert mode config
    let config = CommitConfig::expert_mode();
    let config = CommitConfig {
        work_dir: temp_dir.path().to_path_buf(),
        ..config
    };

    let (_updated_tasks, summary) = commit_tasks(vec![task.clone()], &config).await.unwrap();

    // Expert mode uses task branches and deletes them after merge
    assert_eq!(summary.branches_created, 1);
    assert_eq!(summary.committed_tasks, 1);
    assert!(summary.is_complete_success());
}
