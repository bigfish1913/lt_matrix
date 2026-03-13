//! Tests for Git hooks configuration
//!
//! This test module verifies the pre-commit hooks setup including:
//! - Hook file existence and structure
//! - Installation scripts
//! - Hook content validation
//! - Documentation coverage

use std::fs;
use std::path::Path;

/// Project root directory
fn project_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// .githooks directory path
fn githooks_dir() -> std::path::PathBuf {
    project_root().join(".githooks")
}

/// Scripts directory path
fn scripts_dir() -> std::path::PathBuf {
    project_root().join("scripts")
}

// ============================================================================
// Hook File Existence Tests
// ============================================================================

#[test]
fn test_githooks_directory_exists() {
    let githooks = githooks_dir();
    assert!(
        githooks.exists(),
        ".githooks directory should exist at {:?}",
        githooks
    );
    assert!(githooks.is_dir(), ".githooks should be a directory");
}

#[test]
fn test_pre_commit_hook_exists() {
    let hook = githooks_dir().join("pre-commit");
    assert!(hook.exists(), "pre-commit hook should exist at {:?}", hook);
}

#[test]
fn test_pre_push_hook_exists() {
    let hook = githooks_dir().join("pre-push");
    assert!(hook.exists(), "pre-push hook should exist at {:?}", hook);
}

#[test]
fn test_commit_msg_hook_exists() {
    let hook = githooks_dir().join("commit-msg");
    assert!(hook.exists(), "commit-msg hook should exist at {:?}", hook);
}

// ============================================================================
// Installation Script Tests
// ============================================================================

#[test]
fn test_install_hooks_script_exists_unix() {
    let script = scripts_dir().join("install-hooks.sh");
    assert!(
        script.exists(),
        "install-hooks.sh should exist at {:?}",
        script
    );
}

#[test]
fn test_install_hooks_script_exists_windows() {
    let script = scripts_dir().join("install-hooks.bat");
    assert!(
        script.exists(),
        "install-hooks.bat should exist at {:?}",
        script
    );
}

#[test]
fn test_uninstall_hooks_script_exists() {
    let script = scripts_dir().join("uninstall-hooks.sh");
    assert!(
        script.exists(),
        "uninstall-hooks.sh should exist at {:?}",
        script
    );
}

// ============================================================================
// Hook Content Validation Tests
// ============================================================================

#[test]
fn test_pre_commit_hook_contains_cargo_fmt() {
    let hook = githooks_dir().join("pre-commit");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-commit hook");

    assert!(
        content.contains("cargo fmt --check"),
        "pre-commit hook should run 'cargo fmt --check'"
    );
}

#[test]
fn test_pre_commit_hook_contains_cargo_clippy() {
    let hook = githooks_dir().join("pre-commit");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-commit hook");

    assert!(
        content.contains("cargo clippy"),
        "pre-commit hook should run 'cargo clippy'"
    );
}

#[test]
fn test_pre_commit_hook_contains_cargo_test() {
    let hook = githooks_dir().join("pre-commit");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-commit hook");

    assert!(
        content.contains("cargo test"),
        "pre-commit hook should run 'cargo test'"
    );
}

#[test]
fn test_pre_commit_hook_runs_fast_tests() {
    let hook = githooks_dir().join("pre-commit");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-commit hook");

    // Pre-commit should run fast tests (--lib) not full test suite
    assert!(
        content.contains("--lib"),
        "pre-commit hook should run fast unit tests with --lib flag"
    );
}

#[test]
fn test_pre_push_hook_contains_cargo_fmt() {
    let hook = githooks_dir().join("pre-push");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-push hook");

    assert!(
        content.contains("cargo fmt --check"),
        "pre-push hook should run 'cargo fmt --check'"
    );
}

#[test]
fn test_pre_push_hook_contains_cargo_clippy() {
    let hook = githooks_dir().join("pre-push");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-push hook");

    assert!(
        content.contains("cargo clippy"),
        "pre-push hook should run 'cargo clippy'"
    );
}

#[test]
fn test_pre_push_hook_contains_full_test_suite() {
    let hook = githooks_dir().join("pre-push");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-push hook");

    // Pre-push should run full test suite
    assert!(
        content.contains("cargo test"),
        "pre-push hook should run 'cargo test'"
    );
}

#[test]
fn test_pre_push_hook_contains_release_build() {
    let hook = githooks_dir().join("pre-push");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-push hook");

    assert!(
        content.contains("cargo build --release"),
        "pre-push hook should run 'cargo build --release'"
    );
}

#[test]
fn test_commit_msg_hook_validates_format() {
    let hook = githooks_dir().join("commit-msg");
    let content = fs::read_to_string(&hook).expect("Failed to read commit-msg hook");

    // Should contain conventional commit type validation
    let valid_types = [
        "feat", "fix", "docs", "style", "refactor", "test", "chore", "perf", "ci", "build",
        "revert",
    ];
    for commit_type in valid_types {
        assert!(
            content.contains(commit_type),
            "commit-msg hook should validate '{}' commit type",
            commit_type
        );
    }
}

// ============================================================================
// Hook Structure Tests
// ============================================================================

#[test]
fn test_pre_commit_hook_has_shebang() {
    let hook = githooks_dir().join("pre-commit");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-commit hook");

    assert!(
        content.starts_with("#!/bin/bash"),
        "pre-commit hook should start with bash shebang"
    );
}

#[test]
fn test_pre_push_hook_has_shebang() {
    let hook = githooks_dir().join("pre-push");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-push hook");

    assert!(
        content.starts_with("#!/bin/bash"),
        "pre-push hook should start with bash shebang"
    );
}

#[test]
fn test_commit_msg_hook_has_shebang() {
    let hook = githooks_dir().join("commit-msg");
    let content = fs::read_to_string(&hook).expect("Failed to read commit-msg hook");

    assert!(
        content.starts_with("#!/bin/bash"),
        "commit-msg hook should start with bash shebang"
    );
}

#[test]
fn test_pre_commit_hook_has_error_handling() {
    let hook = githooks_dir().join("pre-commit");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-commit hook");

    assert!(
        content.contains("set -e"),
        "pre-commit hook should have 'set -e' for error handling"
    );
}

#[test]
fn test_pre_push_hook_has_error_handling() {
    let hook = githooks_dir().join("pre-push");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-push hook");

    assert!(
        content.contains("set -e"),
        "pre-push hook should have 'set -e' for error handling"
    );
}

#[test]
fn test_commit_msg_hook_has_error_handling() {
    let hook = githooks_dir().join("commit-msg");
    let content = fs::read_to_string(&hook).expect("Failed to read commit-msg hook");

    assert!(
        content.contains("set -e"),
        "commit-msg hook should have 'set -e' for error handling"
    );
}

// ============================================================================
// Installation Script Structure Tests
// ============================================================================

#[test]
fn test_install_hooks_script_has_shebang() {
    let script = scripts_dir().join("install-hooks.sh");
    let content = fs::read_to_string(&script).expect("Failed to read install-hooks.sh");

    assert!(
        content.starts_with("#!/bin/bash"),
        "install-hooks.sh should start with bash shebang"
    );
}

#[test]
fn test_uninstall_hooks_script_has_shebang() {
    let script = scripts_dir().join("uninstall-hooks.sh");
    let content = fs::read_to_string(&script).expect("Failed to read uninstall-hooks.sh");

    assert!(
        content.starts_with("#!/bin/bash"),
        "uninstall-hooks.sh should start with bash shebang"
    );
}

#[test]
fn test_install_hooks_script_configures_git() {
    let script = scripts_dir().join("install-hooks.sh");
    let content = fs::read_to_string(&script).expect("Failed to read install-hooks.sh");

    assert!(
        content.contains("git") && content.contains("config") && content.contains("core.hooksPath"),
        "install-hooks.sh should configure git core.hooksPath"
    );
}

#[test]
fn test_uninstall_hooks_script_removes_config() {
    let script = scripts_dir().join("uninstall-hooks.sh");
    let content = fs::read_to_string(&script).expect("Failed to read uninstall-hooks.sh");

    assert!(
        content.contains("--unset"),
        "uninstall-hooks.sh should unset the hooksPath config"
    );
}

// ============================================================================
// Windows Script Tests
// ============================================================================

#[test]
fn test_install_hooks_bat_has_echo_off() {
    let script = scripts_dir().join("install-hooks.bat");
    let content = fs::read_to_string(&script).expect("Failed to read install-hooks.bat");

    assert!(
        content.contains("@echo off"),
        "install-hooks.bat should have '@echo off'"
    );
}

#[test]
fn test_install_hooks_bat_configures_git() {
    let script = scripts_dir().join("install-hooks.bat");
    let content = fs::read_to_string(&script).expect("Failed to read install-hooks.bat");

    assert!(
        content.contains("git config core.hooksPath"),
        "install-hooks.bat should configure git core.hooksPath"
    );
}

// ============================================================================
// Documentation Tests
// ============================================================================

#[test]
fn test_contributing_guide_exists() {
    let guide = project_root().join("CONTRIBUTING.md");
    assert!(guide.exists(), "CONTRIBUTING.md should exist");
}

#[test]
fn test_contributing_guide_documents_hooks() {
    let guide = project_root().join("CONTRIBUTING.md");
    let content = fs::read_to_string(&guide).expect("Failed to read CONTRIBUTING.md");

    assert!(
        content.contains("Git Hooks") || content.contains("git hooks"),
        "CONTRIBUTING.md should document Git Hooks"
    );
}

#[test]
fn test_contributing_guide_documents_pre_commit() {
    let guide = project_root().join("CONTRIBUTING.md");
    let content = fs::read_to_string(&guide).expect("Failed to read CONTRIBUTING.md");

    assert!(
        content.contains("pre-commit"),
        "CONTRIBUTING.md should document pre-commit hook"
    );
}

#[test]
fn test_contributing_guide_documents_installation() {
    let guide = project_root().join("CONTRIBUTING.md");
    let content = fs::read_to_string(&guide).expect("Failed to read CONTRIBUTING.md");

    assert!(
        content.contains("install-hooks"),
        "CONTRIBUTING.md should document hook installation"
    );
}

#[test]
fn test_contributing_guide_documents_bypass() {
    let guide = project_root().join("CONTRIBUTING.md");
    let content = fs::read_to_string(&guide).expect("Failed to read CONTRIBUTING.md");

    assert!(
        content.contains("--no-verify"),
        "CONTRIBUTING.md should document how to bypass hooks"
    );
}

#[test]
fn test_contributing_guide_documents_conventional_commits() {
    let guide = project_root().join("CONTRIBUTING.md");
    let content = fs::read_to_string(&guide).expect("Failed to read CONTRIBUTING.md");

    assert!(
        content.contains("Conventional Commits") || content.contains("conventional commit"),
        "CONTRIBUTING.md should document Conventional Commits"
    );
}

// ============================================================================
// Hook Bypass Documentation Tests
// ============================================================================

#[test]
fn test_pre_commit_hook_documents_bypass() {
    let hook = githooks_dir().join("pre-commit");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-commit hook");

    assert!(
        content.contains("--no-verify"),
        "pre-commit hook should document --no-verify bypass option"
    );
}

#[test]
fn test_pre_push_hook_documents_bypass() {
    let hook = githooks_dir().join("pre-push");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-push hook");

    assert!(
        content.contains("--no-verify"),
        "pre-push hook should document --no-verify bypass option"
    );
}

#[test]
fn test_commit_msg_hook_documents_bypass() {
    let hook = githooks_dir().join("commit-msg");
    let content = fs::read_to_string(&hook).expect("Failed to read commit-msg hook");

    assert!(
        content.contains("--no-verify"),
        "commit-msg hook should document --no-verify bypass option"
    );
}

// ============================================================================
// Commit Message Format Validation Tests
// ============================================================================

#[test]
fn test_commit_msg_hook_has_valid_regex_pattern() {
    let hook = githooks_dir().join("commit-msg");
    let content = fs::read_to_string(&hook).expect("Failed to read commit-msg hook");

    // Should have a regex pattern for validating commit messages
    // Uses grep -qE (quiet mode with extended regex)
    assert!(
        content.contains("grep") && (content.contains("-qE") || content.contains("-E")),
        "commit-msg hook should use extended regex for validation"
    );
}

#[test]
fn test_commit_msg_hook_skips_merge_commits() {
    let hook = githooks_dir().join("commit-msg");
    let content = fs::read_to_string(&hook).expect("Failed to read commit-msg hook");

    assert!(
        content.contains("Merge branch") || content.contains("Merge pull request"),
        "commit-msg hook should skip validation for merge commits"
    );
}

#[test]
fn test_commit_msg_hook_handles_revert_commits() {
    let hook = githooks_dir().join("commit-msg");
    let content = fs::read_to_string(&hook).expect("Failed to read commit-msg hook");

    assert!(
        content.contains("Revert"),
        "commit-msg hook should skip validation for revert commits"
    );
}

// ============================================================================
// Performance Tests (hooks should be fast)
// ============================================================================

#[test]
fn test_pre_commit_hook_uses_conditional_test_run() {
    let hook = githooks_dir().join("pre-commit");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-commit hook");

    // Pre-commit should only run tests if Rust files changed
    assert!(
        content.contains("STAGED_RUST_FILES")
            || content.contains("rust")
            || content.contains(".rs"),
        "pre-commit hook should conditionally run tests based on changed files"
    );
}

#[test]
fn test_pre_commit_hook_skips_tests_when_no_rust_changes() {
    let hook = githooks_dir().join("pre-commit");
    let content = fs::read_to_string(&hook).expect("Failed to read pre-commit hook");

    assert!(
        content.contains("Skipping tests"),
        "pre-commit hook should indicate when tests are skipped"
    );
}
