//! Tests for git operations module
//!
//! These tests verify the git operations used in release management.

use ltmatrix::release::GitError;

#[test]
fn test_git_error_display() {
    let err = GitError::CommandFailed("fatal: not a git repository".to_string());
    assert_eq!(err.to_string(), "Git command failed: fatal: not a git repository");

    let err = GitError::NotARepository;
    assert_eq!(err.to_string(), "Not a git repository");

    let err = GitError::TagExists("v1.0.0".to_string());
    assert_eq!(err.to_string(), "Tag already exists: v1.0.0");

    let err = GitError::UncommittedChanges;
    assert_eq!(err.to_string(), "Uncommitted changes");
}

#[test]
fn test_git_error_is_std_error() {
    // Verify GitError implements std::error::Error
    let err = GitError::NotARepository;
    let _: &dyn std::error::Error = &err;
}

#[test]
fn test_git_error_from_io_error() {
    // Verify GitError can be created from io::Error
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let git_err: GitError = io_err.into();

    match git_err {
        GitError::IoError(_) => {} // Expected
        _ => panic!("Expected IoError variant"),
    }
}