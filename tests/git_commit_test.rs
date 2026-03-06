//! Integration tests for Git commit operations
//!
//! This test suite verifies the functionality of the Git commit module including:
//! - Staging files and directories
//! - Creating commits with proper message validation
//! - Amending existing commits
//! - Error handling for edge cases
//! - Multi-line commit messages
//! - Commit message normalization

use ltmatrix::git::{
    init_repo,
    commit::{
        commit_changes, stage_files, stage_all, create_commit, get_head_commit,
        has_unstaged_changes, validate_commit_message,
        amend_commit, short_commit_id
    },
    create_signature
};
use tempfile::TempDir;
use git2::Repository;
use std::fs::{self, File};
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

/// Test basic commit workflow: stage file and commit
#[test]
fn test_basic_commit_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Stage and commit the .gitignore that was auto-generated
    stage_all(&repo).expect("Failed to stage .gitignore");
    commit_changes(&repo, "Initial commit with .gitignore")
        .expect("Failed to create initial commit");

    // Create a test file
    let file_path = repo_path.join("test.txt");
    File::create(&file_path)
        .expect("Failed to create test file")
        .write_all(b"Hello, World!")
        .expect("Failed to write to test file");

    // Verify no staged changes initially
    let index_before = repo.index().unwrap();
    let count_before = index_before.iter().count();

    // Stage the file
    stage_files(&repo, &["test.txt"])
        .expect("Failed to stage test file");

    // Verify there are more staged changes now
    let index_after = repo.index().unwrap();
    let count_after = index_after.iter().count();
    assert!(count_after > count_before, "Should have more staged files after staging");

    // Commit the changes
    let commit_id = commit_changes(&repo, "Add test file")
        .expect("Failed to commit changes");

    // Verify commit was created
    let commit = repo.find_commit(commit_id)
        .expect("Failed to find commit");
    assert_eq!(commit.message().unwrap(), "Add test file");

    // Verify the commit contains our file
    let tree = commit.tree().unwrap();
    assert!(tree.get_path(std::path::Path::new("test.txt")).is_ok(),
            "Committed tree should contain test.txt");
}

/// Test staging multiple files at once
#[test]
fn test_stage_multiple_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create multiple test files
    File::create(repo_path.join("file1.txt"))
        .unwrap().write_all(b"content1").unwrap();
    File::create(repo_path.join("file2.txt"))
        .unwrap().write_all(b"content2").unwrap();
    File::create(repo_path.join("file3.txt"))
        .unwrap().write_all(b"content3").unwrap();

    // Stage all files at once
    stage_files(&repo, &["file1.txt", "file2.txt", "file3.txt"])
        .expect("Failed to stage files");

    // Verify all files are staged
    let index = repo.index().unwrap();
    assert_eq!(index.iter().count(), 3, "Should have 3 staged files");
}

/// Test staging all changes with stage_all
#[test]
fn test_stage_all_changes() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create multiple test files (in addition to the .gitignore that init_repo creates)
    File::create(repo_path.join("a.txt"))
        .unwrap().write_all(b"a").unwrap();
    File::create(repo_path.join("b.txt"))
        .unwrap().write_all(b"b").unwrap();

    // Stage all changes
    stage_all(&repo).expect("Failed to stage all");

    // Verify all files are staged (a.txt, b.txt, and .gitignore)
    let index = repo.index().unwrap();
    assert_eq!(index.iter().count(), 3, "Should have 3 staged files");
}

/// Test commit fails with empty message
#[test]
fn test_commit_empty_message_fails() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create and stage a file
    File::create(repo_path.join("test.txt"))
        .unwrap().write_all(b"content").unwrap();
    stage_files(&repo, &["test.txt"]).unwrap();

    // Try to commit with empty message
    let result = commit_changes(&repo, "");
    assert!(result.is_err(), "Should fail with empty message");

    let err = result.unwrap_err().to_string();
    assert!(err.contains("cannot be empty"), "Error should mention empty message");
}

/// Test commit fails when nothing is staged
#[test]
fn test_commit_nothing_staged_fails() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");
    create_initial_commit(&repo).unwrap();

    // Try to commit without staging anything
    let result = commit_changes(&repo, "No changes");
    assert!(result.is_err(), "Should fail with nothing staged");

    let err = result.unwrap_err().to_string();
    assert!(err.contains("No changes staged"), "Error should mention no staged changes");
}

/// Test multi-line commit message
#[test]
fn test_multiline_commit_message() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");
    create_initial_commit(&repo).unwrap();

    // Create and stage a file
    File::create(repo_path.join("test.txt"))
        .unwrap().write_all(b"content").unwrap();
    stage_files(&repo, &["test.txt"]).unwrap();

    // Commit with multi-line message
    let message = "Add new feature\n\nThis adds a new feature that does X.\n\nCloses #123";
    let commit_id = commit_changes(&repo, message)
        .expect("Failed to commit with multi-line message");

    // Verify message is preserved
    let commit = repo.find_commit(commit_id).unwrap();
    assert_eq!(commit.message().unwrap(), message);
}

/// Test commit message trimming
#[test]
fn test_commit_message_trimmed() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");
    create_initial_commit(&repo).unwrap();

    // Create and stage a file
    File::create(repo_path.join("test.txt"))
        .unwrap().write_all(b"content").unwrap();
    stage_files(&repo, &["test.txt"]).unwrap();

    // Commit with whitespace-padded message
    let commit_id = commit_changes(&repo, "  Add feature  ")
        .expect("Failed to commit");

    // Verify message is trimmed
    let commit = repo.find_commit(commit_id).unwrap();
    assert_eq!(commit.message().unwrap(), "Add feature");
}

/// Test commit message line ending normalization
#[test]
fn test_commit_message_line_ending_normalization() {
    // Test that CRLF is normalized to LF
    let result = validate_commit_message("Test\r\nMessage");
    assert!(result.is_ok());

    let normalized = result.unwrap();
    assert!(!normalized.contains("\r\n"), "Should not contain CRLF");
    assert!(normalized.contains("\n"), "Should contain LF");
}

/// Test amend commit workflow
#[test]
fn test_amend_commit_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create and commit initial file
    File::create(repo_path.join("test.txt"))
        .unwrap().write_all(b"original").unwrap();
    stage_files(&repo, &["test.txt"]).unwrap();
    let original_id = commit_changes(&repo, "Original commit")
        .expect("Failed to create original commit");

    // Modify the file
    File::create(repo_path.join("test.txt"))
        .unwrap().write_all(b"modified").unwrap();

    // Amend the commit
    let amended_id = amend_commit(&repo, Some("Amended message"))
        .expect("Failed to amend commit");

    // Verify commit was amended
    assert_ne!(original_id, amended_id, "Commit ID should be different");

    let commit = repo.find_commit(amended_id).unwrap();
    assert_eq!(commit.message().unwrap(), "Amended message");

    // Verify HEAD points to new commit
    let head = repo.head().unwrap();
    let head_commit = head.peel_to_commit().unwrap();
    assert_eq!(head_commit.id(), amended_id, "HEAD should point to amended commit");
}

/// Test amend commit without new message
#[test]
fn test_amend_commit_keep_message() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create and commit initial file
    File::create(repo_path.join("test.txt"))
        .unwrap().write_all(b"original").unwrap();
    stage_files(&repo, &["test.txt"]).unwrap();
    let _original_id = commit_changes(&repo, "Original message")
        .expect("Failed to create original commit");

    // Modify the file
    File::create(repo_path.join("test.txt"))
        .unwrap().write_all(b"modified").unwrap();

    // Amend without new message
    let amended_id = amend_commit(&repo, None)
        .expect("Failed to amend commit");

    // Verify message is preserved
    let commit = repo.find_commit(amended_id).unwrap();
    assert_eq!(commit.message().unwrap(), "Original message");
}

/// Test amend fails with no commits
#[test]
fn test_amend_fails_no_commits() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Try to amend without any commits
    let result = amend_commit(&repo, Some("Message"));
    assert!(result.is_err(), "Should fail with no commits");

    let err = result.unwrap_err().to_string();
    assert!(err.contains("no commits"), "Error should mention no commits");
}

/// Test short commit ID generation
#[test]
fn test_short_commit_id() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create and commit a file
    File::create(repo_path.join("test.txt"))
        .unwrap().write_all(b"content").unwrap();
    stage_files(&repo, &["test.txt"]).unwrap();
    let commit_id = commit_changes(&repo, "Test commit")
        .expect("Failed to commit");

    // Get short ID
    let short_id = short_commit_id(&repo, &commit_id, 7)
        .expect("Failed to get short ID");

    assert_eq!(short_id.len(), 7, "Short ID should be 7 characters");

    // Verify it's a prefix of full ID
    let full_id = commit_id.to_string();
    assert!(full_id.starts_with(&short_id), "Short ID should be prefix of full ID");
}

/// Test get_head_commit returns None for empty repo
#[test]
fn test_get_head_commit_empty_repo() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // No commits yet
    let head = get_head_commit(&repo).expect("Failed to get HEAD");
    assert!(head.is_none(), "HEAD should be None for empty repo");
}

/// Test get_head_commit returns Some after commit
#[test]
fn test_get_head_commit_after_commit() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create and commit
    File::create(repo_path.join("test.txt"))
        .unwrap().write_all(b"content").unwrap();
    stage_files(&repo, &["test.txt"]).unwrap();
    commit_changes(&repo, "Test commit").unwrap();

    // Should have HEAD
    let head = get_head_commit(&repo).expect("Failed to get HEAD");
    assert!(head.is_some(), "HEAD should exist after commit");
}

/// Test has_unstaged_changes detection
#[test]
fn test_has_unstaged_changes() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Stage and commit the .gitignore that was auto-generated
    stage_all(&repo).expect("Failed to stage .gitignore");
    commit_changes(&repo, "Initial commit").unwrap();

    // Initially no changes
    assert!(!has_unstaged_changes(&repo).unwrap(), "Should have no unstaged changes initially");

    // Create a file (unstaged)
    File::create(repo_path.join("test.txt"))
        .unwrap().write_all(b"content").unwrap();

    // Should have unstaged changes
    assert!(has_unstaged_changes(&repo).unwrap(), "Should have unstaged changes after creating file");

    // Stage the file
    stage_files(&repo, &["test.txt"]).unwrap();

    // Should have no unstaged changes (all staged)
    assert!(!has_unstaged_changes(&repo).unwrap(), "Should have no unstaged changes after staging");
}

/// Test staging files in subdirectories
#[test]
fn test_stage_subdirectory_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create subdirectory structure
    let subdir = repo_path.join("src");
    fs::create_dir(&subdir).unwrap();
    File::create(subdir.join("main.rs"))
        .unwrap().write_all(b"fn main() {}").unwrap();

    // Stage file in subdirectory
    stage_files(&repo, &["src/main.rs"])
        .expect("Failed to stage subdirectory file");

    // Verify file is staged
    let index = repo.index().unwrap();
    assert_eq!(index.iter().count(), 1, "Should have 1 staged file");
}

/// Test committing multiple changes in sequence
#[test]
fn test_sequential_commits() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // First commit
    File::create(repo_path.join("file1.txt"))
        .unwrap().write_all(b"content1").unwrap();
    stage_files(&repo, &["file1.txt"]).unwrap();
    let commit1 = commit_changes(&repo, "First commit")
        .expect("Failed to create first commit");

    // Second commit
    File::create(repo_path.join("file2.txt"))
        .unwrap().write_all(b"content2").unwrap();
    stage_files(&repo, &["file2.txt"]).unwrap();
    let commit2 = commit_changes(&repo, "Second commit")
        .expect("Failed to create second commit");

    // Verify both commits exist and are different
    assert_ne!(commit1, commit2, "Commits should have different IDs");

    let c1 = repo.find_commit(commit1).unwrap();
    let c2 = repo.find_commit(commit2).unwrap();
    assert_eq!(c1.message().unwrap(), "First commit");
    assert_eq!(c2.message().unwrap(), "Second commit");

    // Verify second commit's parent is first commit
    let parents: Vec<git2::Commit> = c2.parents().collect();
    assert_eq!(parents.len(), 1, "Second commit should have one parent");
    assert_eq!(parents[0].id(), commit1, "Parent should be first commit");
}

/// Test commit message too long
#[test]
fn test_commit_message_too_long() {
    // Create a message that's too long (> 65536 characters)
    let long_message = "x".repeat(65537);
    let result = validate_commit_message(&long_message);
    assert!(result.is_err(), "Should fail with too long message");
}

/// Test stage_files handles non-existent files
#[test]
fn test_stage_nonexistent_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Try to stage a file that doesn't exist
    // This should not fail - it should just not add anything
    let result = stage_files(&repo, &["nonexistent.txt"]);
    // The function currently doesn't fail for non-existent files
    // It just skips them
    assert!(result.is_ok(), "Should handle non-existent files gracefully");
}

/// Test create_commit with custom parents
#[test]
fn test_create_commit_custom_parents() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    let parent_id = create_initial_commit(&repo).unwrap();

    // Create a custom commit
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();

    let commit_id = create_commit(
        &repo,
        "Custom commit",
        &tree,
        &[parent_id],
        "HEAD"
    ).expect("Failed to create custom commit");

    // Verify commit was created
    let commit = repo.find_commit(commit_id).unwrap();
    assert_eq!(commit.message().unwrap(), "Custom commit");

    // Verify parent
    let parents: Vec<git2::Commit> = commit.parents().collect();
    assert_eq!(parents.len(), 1);
    assert_eq!(parents[0].id(), parent_id);
}

/// Test committing files with different extensions
#[test]
fn test_commit_various_file_types() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");
    create_initial_commit(&repo).unwrap();

    // Create files with different extensions
    let files: Vec<(&str, &[u8])> = vec![
        ("readme.md", b"# Documentation"),
        ("config.json", b"{}"),
        ("script.sh", b"#!/bin/bash\necho test"),
        ("data.csv", b"a,b,c\n1,2,3"),
    ];

    for (filename, content) in &files {
        File::create(repo_path.join(filename))
            .unwrap()
            .write_all(content)
            .unwrap();
    }

    // Stage all files
    let file_names: Vec<&str> = files.iter().map(|(f, _)| *f).collect();
    stage_files(&repo, &file_names).expect("Failed to stage files");

    // Commit
    let commit_id = commit_changes(&repo, "Add various file types")
        .expect("Failed to commit");

    // Verify all files are in the commit
    let commit = repo.find_commit(commit_id).unwrap();
    let tree = commit.tree().unwrap();

    for (filename, _) in &files {
        let entry = tree.get_path(std::path::Path::new(filename));
        assert!(entry.is_ok(), "File {} should be in commit", filename);
    }
}

/// Test validate_commit_message edge cases
#[test]
fn test_validate_commit_message_edge_cases() {
    // Valid cases
    assert!(validate_commit_message("a").is_ok(), "Single character should be valid");
    assert!(validate_commit_message("  a  ").is_ok(), "Whitespace padding should be trimmed");

    // Invalid cases
    assert!(validate_commit_message("").is_err(), "Empty string should be invalid");
    assert!(validate_commit_message("   ").is_err(), "Whitespace only should be invalid");
    assert!(validate_commit_message("\n\n").is_err(), "Newlines only should be invalid");
    assert!(validate_commit_message("  \n  ").is_err(), "Mixed whitespace should be invalid");
}
