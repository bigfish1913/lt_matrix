//! Integration tests for Git merge operations
//!
//! This test suite verifies the functionality of the Git merge module including:
//! - Squash merge operations
//! - Conflict detection and reporting
//! - User-friendly error messages
//! - Edge cases and error conditions
//! - Multi-file merge scenarios
//! - Branch state validation before merge

use ltmatrix::git::{
    init_repo,
    merge::merge_with_squash,
    commit::{commit_changes, stage_files, stage_all},
    create_signature,
    checkout,
    branch::{create_branch, list_branches},
};
use tempfile::TempDir;
use git2::Repository;
use std::fs::File;
use std::io::Write;

/// Helper function to create an initial commit for testing
fn create_initial_commit(repo: &Repository) -> anyhow::Result<git2::Oid> {
    let sig = create_signature("Test User", "test@example.com")?;

    // Write empty tree
    let tree_oid = repo.treebuilder(None)?.write()?;

    // Create commit
    let tree = repo.find_tree(tree_oid)?;
    let oid = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "Initial commit",
        &tree,
        &[],
    )?;

    Ok(oid)
}

/// Helper function to create and checkout a branch
fn create_and_checkout_branch(repo: &Repository, branch_name: &str) -> anyhow::Result<()> {
    let branch = create_branch(repo, branch_name)?;

    // Get the reference name and set HEAD
    let branch_ref = branch.get();
    repo.set_head(branch_ref.name().expect("Failed to get branch name"))?;

    let commit = branch_ref.peel_to_commit()?;

    // Hard reset to clean working directory and index
    repo.reset(commit.as_object(), git2::ResetType::Hard, None)?;

    Ok(())
}

/// Test basic squash merge workflow
#[test]
fn test_basic_squash_merge() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    stage_all(&repo).expect("Failed to stage .gitignore");
    let initial_commit = commit_changes(&repo, "Initial commit")
        .expect("Failed to create initial commit");

    // Create a feature branch from current commit
    let obj = repo.find_commit(initial_commit).unwrap();
    let branch = repo.branch("feature-branch", &obj, false).unwrap();

    // Checkout the feature branch
    repo.set_head(branch.get().name().unwrap()).unwrap();
    repo.checkout_head(None).unwrap();

    // Add files in a subdirectory and commit
    let feature_dir = repo_path.join("src");
    std::fs::create_dir_all(&feature_dir).unwrap();
    let file_path = feature_dir.join("main.rs");
    File::create(&file_path)
        .unwrap()
        .write_all(b"fn main() { println!(\"Hello\"); }")
        .unwrap();
    stage_files(&repo, &["src/main.rs"]).unwrap();
    let feature_commit = commit_changes(&repo, "Add feature")
        .expect("Failed to commit feature");

    // Verify the commit was created on the feature branch
    let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head_commit.id(), feature_commit, "HEAD should point to feature commit");

    // Switch back to master
    checkout(&repo, "master").expect("Failed to checkout master");

    // Verify master still points to initial commit
    let master_commit = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(master_commit.id(), initial_commit, "Master should still be at initial commit");

    // Squash merge the feature branch
    let merged_commit = merge_with_squash(&repo, "feature-branch", "Squashed feature merge")
        .expect("Failed to squash merge");

    // Verify the squashed commit was created
    assert_ne!(merged_commit, feature_commit, "Squashed commit should have different ID");

    let commit = repo.find_commit(merged_commit)
        .expect("Failed to find merged commit");
    assert_eq!(commit.message().unwrap(), "Squashed feature merge");
    assert_eq!(commit.parent_count(), 1, "Squash merge should have one parent");

    // Verify the file is in the merge commit tree
    let tree = commit.tree().unwrap();
    assert!(tree.get_path(std::path::Path::new("src/main.rs")).is_ok(),
            "Merged file should be in commit tree");
}

/// Test squash merge with multiple commits
#[test]
fn test_squash_merge_multiple_commits() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    stage_all(&repo).expect("Failed to stage .gitignore");
    let initial_commit = commit_changes(&repo, "Initial commit")
        .expect("Failed to create initial commit");

    // Create a feature branch from current commit
    let obj = repo.find_commit(initial_commit).unwrap();
    let branch = repo.branch("feature-branch", &obj, false).unwrap();
    repo.set_head(branch.get().name().unwrap()).unwrap();
    repo.checkout_head(None).unwrap();

    // Create multiple commits on the feature branch
    for i in 1..=3 {
        let file_path = repo_path.join(format!("file{}.txt", i));
        File::create(&file_path)
            .unwrap()
            .write_all(format!("content{}", i).as_bytes())
            .unwrap();
        stage_files(&repo, &[&format!("file{}.txt", i)]).unwrap();
        commit_changes(&repo, &format!("Add file {}", i))
            .expect("Failed to commit");
    }

    // Switch back to master
    checkout(&repo, "master").expect("Failed to checkout master");

    // Squash merge the feature branch
    let merged_commit = merge_with_squash(&repo, "feature-branch", "Squash merge 3 commits")
        .expect("Failed to squash merge");

    // Verify all files are present in the merge commit tree
    let commit = repo.find_commit(merged_commit).unwrap();
    let tree = commit.tree().unwrap();

    for i in 1..=3 {
        let path_str = format!("file{}.txt", i);
        let path = std::path::Path::new(&path_str);
        assert!(tree.get_path(path).is_ok(),
                "File {} should be in merge commit tree", i);
    }

    // Verify the squashed commit
    assert_eq!(commit.parent_count(), 1, "Squash merge should have one parent");
}

/// Test conflict detection in merge
#[test]
fn test_merge_conflict_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    stage_all(&repo).expect("Failed to stage .gitignore");
    commit_changes(&repo, "Initial commit")
        .expect("Failed to create initial commit");

    // Add a shared file on master
    let file_path = repo_path.join("shared.txt");
    File::create(&file_path)
        .unwrap()
        .write_all(b"original content")
        .unwrap();
    stage_files(&repo, &["shared.txt"]).unwrap();
    commit_changes(&repo, "Add shared file on master")
        .expect("Failed to commit");

    // Create a feature branch
    create_and_checkout_branch(&repo, "feature-branch")
        .expect("Failed to create feature branch");

    // Modify the shared file on feature branch
    File::create(&file_path)
        .unwrap()
        .write_all(b"feature content")
        .unwrap();
    stage_files(&repo, &["shared.txt"]).unwrap();
    commit_changes(&repo, "Modify shared file on feature")
        .expect("Failed to commit");

    // Switch back to master
    checkout(&repo, "master").expect("Failed to checkout master");

    // Modify the same file on master (creating conflict)
    File::create(&file_path)
        .unwrap()
        .write_all(b"master content")
        .unwrap();
    stage_files(&repo, &["shared.txt"]).unwrap();
    commit_changes(&repo, "Modify shared file on master")
        .expect("Failed to commit");

    // Attempt to merge - should detect conflict
    let result = merge_with_squash(&repo, "feature-branch", "Attempt merge");
    assert!(result.is_err(), "Merge should fail due to conflicts");

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("conflict") || error_msg.contains("conflicts"),
            "Error should mention conflicts");
}

/// Test merge fails with non-existent branch
#[test]
fn test_merge_nonexistent_branch_fails() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    stage_all(&repo).expect("Failed to stage .gitignore");
    commit_changes(&repo, "Initial commit")
        .expect("Failed to create initial commit");

    // Try to merge non-existent branch
    let result = merge_with_squash(&repo, "nonexistent-branch", "Merge");
    assert!(result.is_err(), "Should fail with non-existent branch");

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("not found"), "Error should mention branch not found");

    // Error message should list available branches
    let branches = list_branches(&repo).unwrap();
    if !branches.is_empty() {
        // Should mention available branches in error
        assert!(error_msg.contains("master") || error_msg.contains("main") ||
                error_msg.contains("Available branches"),
                "Error should mention available branches");
    }
}

/// Test merge fails when source branch has no new commits
#[test]
fn test_merge_no_new_commits_fails() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    stage_all(&repo).expect("Failed to stage .gitignore");
    commit_changes(&repo, "Initial commit")
        .expect("Failed to create initial commit");

    // Create a branch from current HEAD (no new commits)
    create_and_checkout_branch(&repo, "feature-branch")
        .expect("Failed to create feature branch");

    // Switch back to master
    checkout(&repo, "master").expect("Failed to checkout master");

    // Try to merge - should error because no new commits
    let result = merge_with_squash(&repo, "feature-branch", "Merge");
    assert!(result.is_err(), "Should fail when no new commits");

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("no new commits") || error_msg.contains("Nothing to merge"),
            "Error should mention no new commits");
}

/// Test merge fails with staged changes
#[test]
fn test_merge_with_staged_changes_fails() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    stage_all(&repo).expect("Failed to stage .gitignore");
    commit_changes(&repo, "Initial commit")
        .expect("Failed to create initial commit");

    // Create and commit on a feature branch
    create_and_checkout_branch(&repo, "feature-branch")
        .expect("Failed to create feature branch");

    let file_path = repo_path.join("feature.txt");
    File::create(&file_path)
        .unwrap()
        .write_all(b"feature")
        .unwrap();
    stage_files(&repo, &["feature.txt"]).unwrap();
    commit_changes(&repo, "Add feature")
        .expect("Failed to commit feature");

    // Switch back to master
    checkout(&repo, "master").expect("Failed to checkout master");

    // Create staged changes on master (not committed)
    let staged_file = repo_path.join("staged.txt");
    File::create(&staged_file)
        .unwrap()
        .write_all(b"staged")
        .unwrap();
    stage_files(&repo, &["staged.txt"]).unwrap();

    // Try to merge - should error due to staged changes
    let result = merge_with_squash(&repo, "feature-branch", "Merge");
    assert!(result.is_err(), "Should fail with staged changes");

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("staged"), "Error should mention staged changes");
}

/// Test merge error message suggests actions for staged changes
#[test]
fn test_merge_staged_changes_error_suggests_actions() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    stage_all(&repo).expect("Failed to stage .gitignore");
    commit_changes(&repo, "Initial commit")
        .expect("Failed to create initial commit");

    // Create feature branch with a commit
    create_and_checkout_branch(&repo, "feature")
        .expect("Failed to create feature branch");

    let file_path = repo_path.join("test.txt");
    File::create(&file_path).unwrap().write_all(b"test").unwrap();
    stage_files(&repo, &["test.txt"]).unwrap();
    commit_changes(&repo, "Test").unwrap();

    // Switch back to master and create staged changes
    checkout(&repo, "master").expect("Failed to checkout master");
    File::create(&file_path).unwrap().write_all(b"staged").unwrap();
    stage_files(&repo, &["test.txt"]).unwrap();

    let result = merge_with_squash(&repo, "feature", "Merge");
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    // Should suggest actions to resolve
    assert!(error_msg.contains("Commit") || error_msg.contains("Stash") ||
            error_msg.contains("commit") || error_msg.contains("stash"),
            "Error should suggest resolving actions");
}

/// Test merge preserves commit message formatting
#[test]
fn test_merge_commit_message_formatting() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    stage_all(&repo).expect("Failed to stage .gitignore");
    let initial_commit = commit_changes(&repo, "Initial commit")
        .expect("Failed to create initial commit");

    // Create feature branch
    let obj = repo.find_commit(initial_commit).unwrap();
    let branch = repo.branch("feature-branch", &obj, false).unwrap();
    repo.set_head(branch.get().name().unwrap()).unwrap();
    repo.checkout_head(None).unwrap();

    let file_path = repo_path.join("feature.txt");
    File::create(&file_path).unwrap().write_all(b"feature").unwrap();
    stage_files(&repo, &["feature.txt"]).unwrap();
    commit_changes(&repo, "Add feature").unwrap();

    // Switch back to master
    checkout(&repo, "master").expect("Failed to checkout master");
    let _ = std::fs::remove_file(&file_path);

    // Test with multi-line message
    let message = "Merge feature\n\nThis merges the feature branch.";
    let merged_commit = merge_with_squash(&repo, "feature-branch", message)
        .expect("Failed to merge");

    let commit = repo.find_commit(merged_commit).unwrap();
    assert_eq!(commit.message().unwrap(), message);
}

/// Test merge with whitespace-only commit message fails
#[test]
fn test_merge_whitespace_message_fails() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    stage_all(&repo).expect("Failed to stage .gitignore");
    let initial_commit = commit_changes(&repo, "Initial commit")
        .expect("Failed to create initial commit");

    // Create feature branch
    let obj = repo.find_commit(initial_commit).unwrap();
    let branch = repo.branch("feature-branch", &obj, false).unwrap();
    repo.set_head(branch.get().name().unwrap()).unwrap();
    repo.checkout_head(None).unwrap();

    let file_path = repo_path.join("feature.txt");
    File::create(&file_path).unwrap().write_all(b"feature").unwrap();
    stage_files(&repo, &["feature.txt"]).unwrap();
    commit_changes(&repo, "Add feature").unwrap();

    // Switch back to master
    checkout(&repo, "master").expect("Failed to checkout master");
    let _ = std::fs::remove_file(&file_path);

    // Try with whitespace-only message
    let result = merge_with_squash(&repo, "feature-branch", "   ");
    assert!(result.is_err(), "Should fail with whitespace-only message");

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("empty") || error_msg.contains("message"),
            "Error should mention message validation");
}

/// Test merge creates proper parent-child relationship
#[test]
fn test_merge_parent_relationship() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    stage_all(&repo).expect("Failed to stage .gitignore");
    let initial_commit = commit_changes(&repo, "Initial commit")
        .expect("Failed to create initial commit");

    // Get the initial commit ID
    let head_before = repo.head().unwrap().peel_to_commit().unwrap();

    // Create feature branch
    let obj = repo.find_commit(initial_commit).unwrap();
    let branch = repo.branch("feature-branch", &obj, false).unwrap();
    repo.set_head(branch.get().name().unwrap()).unwrap();
    repo.checkout_head(None).unwrap();

    let file_path = repo_path.join("feature.txt");
    File::create(&file_path).unwrap().write_all(b"feature").unwrap();
    stage_files(&repo, &["feature.txt"]).unwrap();
    commit_changes(&repo, "Add feature").unwrap();

    // Switch back to master
    checkout(&repo, "master").expect("Failed to checkout master");
    let _ = std::fs::remove_file(&file_path);

    // Merge
    let merged_commit = merge_with_squash(&repo, "feature-branch", "Merge feature")
        .expect("Failed to merge");

    let commit = repo.find_commit(merged_commit).unwrap();
    assert_eq!(commit.parent_count(), 1, "Squash merge should have one parent");

    let parent = commit.parent(0).unwrap();
    assert_eq!(parent.id(), head_before.id(), "Parent should be HEAD before merge");
}

/// Test merge with file additions only
#[test]
fn test_merge_with_file_additions() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit with a file
    stage_all(&repo).expect("Failed to stage .gitignore");
    let existing_file = repo_path.join("existing.txt");
    File::create(&existing_file).unwrap().write_all(b"existing").unwrap();
    stage_files(&repo, &["existing.txt"]).unwrap();
    let initial_commit = commit_changes(&repo, "Initial commit")
        .expect("Failed to create initial commit");

    // Create feature branch
    let obj = repo.find_commit(initial_commit).unwrap();
    let branch = repo.branch("feature-branch", &obj, false).unwrap();
    repo.set_head(branch.get().name().unwrap()).unwrap();
    repo.checkout_head(None).unwrap();

    // Add new file on feature branch
    let new_file = repo_path.join("new.txt");
    File::create(&new_file).unwrap().write_all(b"new").unwrap();
    stage_files(&repo, &["new.txt"]).unwrap();
    commit_changes(&repo, "Add new file")
        .expect("Failed to commit");

    // Switch back to master
    checkout(&repo, "master").expect("Failed to checkout master");

    // Merge
    let merged_commit = merge_with_squash(&repo, "feature-branch", "Merge new file")
        .expect("Failed to merge");

    // Verify both files are in commit tree
    let commit = repo.find_commit(merged_commit).unwrap();
    let tree = commit.tree().unwrap();

    assert!(tree.get_path(std::path::Path::new("new.txt")).is_ok(),
            "New file should be in commit");
    assert!(tree.get_path(std::path::Path::new("existing.txt")).is_ok(),
            "Existing file should still be in commit");
}

/// Test merge with directory structure changes
#[test]
fn test_merge_with_directory_structure() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    stage_all(&repo).expect("Failed to stage .gitignore");
    let initial_commit = commit_changes(&repo, "Initial commit")
        .expect("Failed to create initial commit");

    // Create feature branch
    let obj = repo.find_commit(initial_commit).unwrap();
    let branch = repo.branch("feature-branch", &obj, false).unwrap();
    repo.set_head(branch.get().name().unwrap()).unwrap();
    repo.checkout_head(None).unwrap();

    // Create nested directory structure
    let nested_dir = repo_path.join("src").join("components");
    std::fs::create_dir_all(&nested_dir).unwrap();
    let file_path = nested_dir.join("component.rs");
    File::create(&file_path)
        .unwrap()
        .write_all(b"component code")
        .unwrap();
    stage_files(&repo, &["src/components/component.rs"]).unwrap();
    commit_changes(&repo, "Add component")
        .expect("Failed to commit");

    // Switch back to master
    checkout(&repo, "master").expect("Failed to checkout master");

    // Merge
    let merged_commit = merge_with_squash(&repo, "feature-branch", "Merge component")
        .expect("Failed to merge");

    // Verify nested structure exists in commit tree
    let commit = repo.find_commit(merged_commit).unwrap();
    let tree = commit.tree().unwrap();
    assert!(tree.get_path(std::path::Path::new("src/components/component.rs")).is_ok(),
            "Nested file should be in commit");
}

/// Test merge handles empty message validation
#[test]
fn test_merge_empty_message_validation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    stage_all(&repo).expect("Failed to stage .gitignore");
    let initial_commit = commit_changes(&repo, "Initial commit")
        .expect("Failed to create initial commit");

    // Create feature branch
    let obj = repo.find_commit(initial_commit).unwrap();
    let branch = repo.branch("feature-branch", &obj, false).unwrap();
    repo.set_head(branch.get().name().unwrap()).unwrap();
    repo.checkout_head(None).unwrap();

    let file_path = repo_path.join("feature.txt");
    File::create(&file_path).unwrap().write_all(b"feature").unwrap();
    stage_files(&repo, &["feature.txt"]).unwrap();
    commit_changes(&repo, "Add feature").unwrap();

    // Switch back to master
    checkout(&repo, "master").expect("Failed to checkout master");
    let _ = std::fs::remove_file(&file_path);

    // Try with empty message
    let result = merge_with_squash(&repo, "feature-branch", "");
    assert!(result.is_err(), "Should fail with empty message");

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("empty") || error_msg.contains("message"),
            "Error should mention message validation");
}
