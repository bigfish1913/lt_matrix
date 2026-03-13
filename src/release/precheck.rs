// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Pre-release validation checks
//!
//! This module provides pre-release validation to ensure code quality
//! before creating a release.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Status of a single check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Passed,
    Failed,
    Skipped,
    Warning,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckStatus::Passed => write!(f, "✓"),
            CheckStatus::Failed => write!(f, "✗"),
            CheckStatus::Skipped => write!(f, "○"),
            CheckStatus::Warning => write!(f, "⚠"),
        }
    }
}

/// Result of a single check
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: Option<String>,
    pub duration_ms: u64,
}

impl CheckResult {
    pub fn passed(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Passed,
            message: None,
            duration_ms: 0,
        }
    }

    pub fn failed(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Failed,
            message: Some(message.into()),
            duration_ms: 0,
        }
    }

    pub fn skipped(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Skipped,
            message: Some(message.into()),
            duration_ms: 0,
        }
    }

    pub fn warning(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Warning,
            message: Some(message.into()),
            duration_ms: 0,
        }
    }
}

/// Pre-release check runner
#[derive(Debug)]
pub struct PreReleaseCheck {
    project_root: std::path::PathBuf,
    results: Vec<CheckResult>,
}

impl PreReleaseCheck {
    /// Create new pre-release check runner
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            results: Vec::new(),
        }
    }

    /// Run all pre-release checks
    pub fn run_all(&mut self) -> Result<()> {
        self.results.clear();

        // Code quality checks
        self.check_tests()?;
        self.check_format()?;
        self.check_clippy()?;
        self.check_docs()?;

        // Git checks
        self.check_clean_working_tree()?;
        self.check_main_branch()?;

        // Changelog checks
        self.check_changelog_exists()?;

        Ok(())
    }

    /// Run cargo test
    fn check_tests(&mut self) -> Result<()> {
        let start = std::time::Instant::now();

        let output = Command::new("cargo")
            .args(["test", "--all", "--", "--quiet"])
            .current_dir(&self.project_root)
            .output();

        let mut result = match output {
            Ok(output) => {
                if output.status.success() {
                    CheckResult::passed("Tests")
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    CheckResult::failed(
                        "Tests",
                        stderr.lines().take(10).collect::<Vec<_>>().join("\n"),
                    )
                }
            }
            Err(e) => CheckResult::failed("Tests", format!("Failed to run tests: {}", e)),
        };

        result.duration_ms = start.elapsed().as_millis() as u64;
        self.results.push(result);
        Ok(())
    }

    /// Run cargo fmt --check
    fn check_format(&mut self) -> Result<()> {
        let start = std::time::Instant::now();

        let output = Command::new("cargo")
            .args(["fmt", "--check"])
            .current_dir(&self.project_root)
            .output();

        let mut result = match output {
            Ok(output) => {
                if output.status.success() {
                    CheckResult::passed("Format")
                } else {
                    CheckResult::failed("Format", "Code is not formatted. Run 'cargo fmt'")
                }
            }
            Err(e) => CheckResult::warning("Format", format!("Failed to check format: {}", e)),
        };

        result.duration_ms = start.elapsed().as_millis() as u64;
        self.results.push(result);
        Ok(())
    }

    /// Run cargo clippy
    fn check_clippy(&mut self) -> Result<()> {
        let start = std::time::Instant::now();

        let output = Command::new("cargo")
            .args(["clippy", "--", "-D", "warnings"])
            .current_dir(&self.project_root)
            .output();

        let mut result = match output {
            Ok(output) => {
                if output.status.success() {
                    CheckResult::passed("Clippy")
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    CheckResult::failed(
                        "Clippy",
                        stderr.lines().take(20).collect::<Vec<_>>().join("\n"),
                    )
                }
            }
            Err(e) => CheckResult::warning("Clippy", format!("Failed to run clippy: {}", e)),
        };

        result.duration_ms = start.elapsed().as_millis() as u64;
        self.results.push(result);
        Ok(())
    }

    /// Check documentation builds
    fn check_docs(&mut self) -> Result<()> {
        let start = std::time::Instant::now();

        let output = Command::new("cargo")
            .args(["doc", "--no-deps"])
            .current_dir(&self.project_root)
            .output();

        let mut result = match output {
            Ok(output) => {
                if output.status.success() {
                    CheckResult::passed("Documentation")
                } else {
                    CheckResult::warning("Documentation", "Documentation build had warnings")
                }
            }
            Err(e) => CheckResult::skipped("Documentation", format!("Failed to build docs: {}", e)),
        };

        result.duration_ms = start.elapsed().as_millis() as u64;
        self.results.push(result);
        Ok(())
    }

    /// Check working tree is clean
    fn check_clean_working_tree(&mut self) -> Result<()> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.project_root)
            .output()
            .context("Failed to run git status")?;

        let result = if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim().is_empty() {
                CheckResult::passed("Clean working tree")
            } else {
                CheckResult::failed(
                    "Clean working tree",
                    "Uncommitted changes detected. Commit or stash them first.",
                )
            }
        } else {
            CheckResult::failed("Clean working tree", "Failed to check git status")
        };

        self.results.push(result);
        Ok(())
    }

    /// Check we're on main branch
    fn check_main_branch(&mut self) -> Result<()> {
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&self.project_root)
            .output()
            .context("Failed to get current branch")?;

        let result = if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if branch == "main" || branch == "master" {
                CheckResult::passed("Main branch")
            } else {
                CheckResult::warning(
                    "Main branch",
                    format!(
                        "Currently on branch '{}'. Releases should be from main.",
                        branch
                    ),
                )
            }
        } else {
            CheckResult::skipped("Main branch", "Could not determine current branch")
        };

        self.results.push(result);
        Ok(())
    }

    /// Check CHANGELOG.md exists
    fn check_changelog_exists(&mut self) -> Result<()> {
        let changelog_path = self.project_root.join("CHANGELOG.md");

        let result = if changelog_path.exists() {
            CheckResult::passed("CHANGELOG.md exists")
        } else {
            CheckResult::warning(
                "CHANGELOG.md exists",
                "CHANGELOG.md not found. Consider creating one.",
            )
        };

        self.results.push(result);
        Ok(())
    }

    /// Check if all checks passed
    pub fn all_passed(&self) -> bool {
        self.results
            .iter()
            .all(|r| r.status == CheckStatus::Passed || r.status == CheckStatus::Skipped)
    }

    /// Get summary of all checks
    pub fn summary(&self) -> String {
        let mut summary = String::new();

        for result in &self.results {
            summary.push_str(&format!(
                "{} {} ({:.2}s)\n",
                result.status,
                result.name,
                result.duration_ms as f64 / 1000.0
            ));

            if let Some(ref msg) = result.message {
                for line in msg.lines() {
                    summary.push_str(&format!("    {}\n", line));
                }
            }
        }

        summary
    }

    /// Get all results
    pub fn results(&self) -> &[CheckResult] {
        &self.results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_result_passed() {
        let result = CheckResult::passed("Test");
        assert_eq!(result.status, CheckStatus::Passed);
        assert!(result.message.is_none());
    }

    #[test]
    fn test_check_result_failed() {
        let result = CheckResult::failed("Test", "Error message");
        assert_eq!(result.status, CheckStatus::Failed);
        assert_eq!(result.message, Some("Error message".to_string()));
    }

    #[test]
    fn test_check_status_display() {
        assert_eq!(CheckStatus::Passed.to_string(), "✓");
        assert_eq!(CheckStatus::Failed.to_string(), "✗");
        assert_eq!(CheckStatus::Skipped.to_string(), "○");
        assert_eq!(CheckStatus::Warning.to_string(), "⚠");
    }
}
