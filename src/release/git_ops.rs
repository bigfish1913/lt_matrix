// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Git operations for release management
//!
//! This module provides git operations needed during the release process.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Git-related errors
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Git command failed: {0}")]
    CommandFailed(String),

    #[error("Not a git repository")]
    NotARepository,

    #[error("Tag already exists: {0}")]
    TagExists(String),

    #[error("Uncommitted changes")]
    UncommittedChanges,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Git operations wrapper
#[derive(Debug)]
pub struct GitOperations {
    repo_path: std::path::PathBuf,
}

impl GitOperations {
    /// Create new git operations handler
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let repo_path = path.as_ref().to_path_buf();

        // Verify it's a git repository
        let git_dir = repo_path.join(".git");
        if !git_dir.exists() {
            return Err(GitError::NotARepository.into());
        }

        Ok(Self { repo_path })
    }

    /// Run a git command
    fn run_git(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to execute git command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::CommandFailed(stderr.to_string()).into());
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get the current branch name
    pub fn current_branch(&self) -> Result<String> {
        self.run_git(&["branch", "--show-current"])
    }

    /// Check if working tree is clean
    pub fn is_clean(&self) -> Result<bool> {
        let output = self.run_git(&["status", "--porcelain"])?;
        Ok(output.is_empty())
    }

    /// Get the last tag
    pub fn get_last_tag(&self) -> Result<Option<String>> {
        let result = self.run_git(&["describe", "--tags", "--abbrev=0"]);

        match result {
            Ok(tag) => Ok(Some(tag)),
            Err(_) => Ok(None),
        }
    }

    /// Check if a tag exists
    pub fn tag_exists(&self, tag: &str) -> Result<bool> {
        let result = self.run_git(&["tag", "-l", tag]);
        match result {
            Ok(output) => Ok(output == tag),
            Err(_) => Ok(false),
        }
    }

    /// Create a tag
    pub fn create_tag(&self, tag: &str, message: &str) -> Result<()> {
        // Check if tag already exists
        if self.tag_exists(tag)? {
            return Err(GitError::TagExists(tag.to_string()).into());
        }

        self.run_git(&["tag", "-a", tag, "-m", message])?;
        Ok(())
    }

    /// Delete a tag
    pub fn delete_tag(&self, tag: &str) -> Result<()> {
        self.run_git(&["tag", "-d", tag])?;
        Ok(())
    }

    /// Add all changes to staging
    pub fn add_all(&self) -> Result<()> {
        self.run_git(&["add", "-A"])?;
        Ok(())
    }

    /// Commit changes
    pub fn commit(&self, message: &str) -> Result<()> {
        self.run_git(&["commit", "-m", message])?;
        Ok(())
    }

    /// Commit version bump changes
    pub fn commit_version_bump(&self, version: &super::version::Version) -> Result<()> {
        self.add_all()?;
        self.commit(&format!("chore: release v{}", version))?;
        Ok(())
    }

    /// Push to remote
    pub fn push(&self, remote: &str, branch: &str) -> Result<()> {
        self.run_git(&["push", remote, branch])?;
        Ok(())
    }

    /// Push tags to remote
    pub fn push_tags(&self, remote: &str) -> Result<()> {
        self.run_git(&["push", remote, "--tags"])?;
        Ok(())
    }

    /// Get list of commits between two refs
    pub fn commits_between(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let output = self.run_git(&["log", "--pretty=format:%s", &format!("{}..{}", from, to)])?;

        if output.is_empty() {
            return Ok(Vec::new());
        }

        Ok(output.lines().map(|s| s.to_string()).collect())
    }

    /// Get commit count since last tag
    pub fn commit_count_since_last_tag(&self) -> Result<usize> {
        let last_tag = self.get_last_tag()?;

        match last_tag {
            Some(tag) => {
                let count = self.run_git(&["rev-list", "--count", &format!("{}..HEAD", tag)])?;
                count.parse().context("Failed to parse commit count")
            }
            None => {
                let count = self.run_git(&["rev-list", "--count", "HEAD"])?;
                count.parse().context("Failed to parse commit count")
            }
        }
    }

    /// Get remote URL
    pub fn get_remote_url(&self, remote: &str) -> Result<Option<String>> {
        let result = self.run_git(&["remote", "get-url", remote]);

        match result {
            Ok(url) => Ok(Some(url)),
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Most tests require an actual git repository
    // These are basic unit tests for the error types

    #[test]
    fn test_git_error_display() {
        let err = GitError::CommandFailed("fatal: not a git repository".to_string());
        assert_eq!(
            err.to_string(),
            "Git command failed: fatal: not a git repository"
        );

        let err = GitError::NotARepository;
        assert_eq!(err.to_string(), "Not a git repository");

        let err = GitError::TagExists("v1.0.0".to_string());
        assert_eq!(err.to_string(), "Tag already exists: v1.0.0");
    }
}
