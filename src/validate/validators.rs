//! Core validation functions
//!
//! Provides validation utilities for goals, task IDs, agent availability,
//! git repository state, and file system permissions.

use anyhow::{bail, Context, Result};
use std::collections::HashSet;
use std::path::Path;
use tracing::{debug, warn};

/// Maximum allowed length for goal strings (to prevent memory issues and improve UX)
const MAX_GOAL_LENGTH: usize = 10_000;

/// Minimum allowed length for goal strings
const MIN_GOAL_LENGTH: usize = 1;

/// Task ID pattern: task-<number> or task-<number>-<subnumber>
const TASK_ID_PATTERN: &str = r"^task-\d+(-\d+)*$";

/// Validates a goal string
///
/// # Arguments
///
/// * `goal` - The goal string to validate
///
/// # Returns
///
/// Returns `Ok(())` if the goal is valid, or an error with a clear message.
///
/// # Examples
///
/// ```
/// use ltmatrix::validate::validate_goal;
///
/// validate_goal("Build a REST API").unwrap();
/// validate_goal("").unwrap_err(); // Empty goal
/// ```
pub fn validate_goal(goal: &str) -> Result<()> {
    let trimmed = goal.trim();

    // Check for empty goal
    if trimmed.is_empty() {
        bail!(
            "Goal cannot be empty. Please provide a description of what you want to accomplish."
        );
    }

    // Check minimum length
    if trimmed.len() < MIN_GOAL_LENGTH {
        bail!(
            "Goal is too short. Please provide at least {} character(s).",
            MIN_GOAL_LENGTH
        );
    }

    // Check maximum length
    if trimmed.len() > MAX_GOAL_LENGTH {
        bail!(
            "Goal is too long ({} characters). Maximum allowed length is {} characters. \
             Please provide a more concise description.",
            trimmed.len(),
            MAX_GOAL_LENGTH
        );
    }

    // Check for reasonable content (not just whitespace or special characters)
    let has_alphanumeric = trimmed.chars().any(|c| c.is_alphanumeric());
    if !has_alphanumeric {
        bail!(
            "Goal must contain at least one alphanumeric character. \
             Please provide a meaningful description."
        );
    }

    debug!("Goal validation passed: {} characters", trimmed.len());
    Ok(())
}

/// Validates a task ID format
///
/// Task IDs must follow the pattern: `task-<number>` or `task-<number>-<subnumber>`
///
/// # Arguments
///
/// * `task_id` - The task ID to validate
///
/// # Returns
///
/// Returns `Ok(())` if the task ID is valid, or an error with a clear message.
///
/// # Examples
///
/// ```
/// use ltmatrix::validate::validate_task_id_format;
///
/// validate_task_id_format("task-1").unwrap();
/// validate_task_id_format("task-2-1").unwrap();
/// validate_task_id_format("invalid").unwrap_err();
/// ```
pub fn validate_task_id_format(task_id: &str) -> Result<()> {
    // Check for empty ID
    if task_id.is_empty() {
        bail!("Task ID cannot be empty");
    }

    // Validate format using regex pattern
    let re = regex::Regex::new(TASK_ID_PATTERN)
        .context("Failed to compile task ID pattern regex")?;

    if !re.is_match(task_id) {
        bail!(
            "Invalid task ID format: '{}'. Task IDs must follow the pattern 'task-<number>' \
             (e.g., 'task-1', 'task-2') or 'task-<number>-<subnumber>' for subtasks \
             (e.g., 'task-1-1', 'task-2-3').",
            task_id
        );
    }

    debug!("Task ID format validation passed: {}", task_id);
    Ok(())
}

/// Validates task ID uniqueness in a collection
///
/// # Arguments
///
/// * `task_ids` - Collection of task IDs to check for uniqueness
///
/// # Returns
///
/// Returns `Ok(())` if all task IDs are unique, or an error listing duplicates.
///
/// # Examples
///
/// ```
/// use ltmatrix::validate::validate_task_ids_unique;
///
/// validate_task_ids_unique(&["task-1".to_string(), "task-2".to_string()]).unwrap();
/// validate_task_ids_unique(&["task-1".to_string(), "task-1".to_string()]).unwrap_err();
/// ```
pub fn validate_task_ids_unique(task_ids: &[String]) -> Result<()> {
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();

    for id in task_ids {
        if !seen.insert(id) {
            duplicates.push(id.clone());
        }
    }

    if !duplicates.is_empty() {
        bail!(
            "Duplicate task IDs found: {}. Each task must have a unique ID.",
            duplicates.join(", ")
        );
    }

    debug!("Task ID uniqueness validation passed: {} unique IDs", task_ids.len());
    Ok(())
}

/// Validates that an agent command is available on the system
///
/// # Arguments
///
/// * `command` - The command name to check (e.g., "claude", "opencode")
///
/// # Returns
///
/// Returns `Ok(())` if the command is available, or an error with installation hints.
///
/// # Examples
///
/// ```
/// use ltmatrix::validate::validate_agent_available;
///
/// // Check if 'claude' command is available
/// validate_agent_available("claude").unwrap();
/// ```
pub fn validate_agent_available(command: &str) -> Result<()> {
    debug!("Checking availability of agent command: {}", command);

    match which::which(command) {
        Ok(path) => {
            debug!("Agent command '{}' found at: {:?}", command, path);
            Ok(())
        }
        Err(_) => {
            let hint = get_agent_installation_hint(command);
            bail!(
                "Agent command '{}' is not available on your system. {}",
                command,
                hint
            );
        }
    }
}

/// Gets installation hints for known agents
fn get_agent_installation_hint(command: &str) -> String {
    match command {
        "claude" => "Install Claude Code CLI: npm install -g @anthropic-ai/claude-code".to_string(),
        "opencode" => "Install OpenCode: visit https://github.com/opencode/opencode".to_string(),
        "kimi-code" => "Install KimiCode: visit https://github.com/moonshot/kimi-code".to_string(),
        "codex" => "Install Codex CLI: pip install openai[codex]".to_string(),
        _ => format!(
            "Please ensure '{}' is installed and available in your PATH.",
            command
        ),
    }
}

/// Validates git repository state
///
/// # Arguments
///
/// * `repo_path` - Path to the git repository
///
/// # Returns
///
/// Returns `Ok(())` if the path is a valid git repository, or an error with details.
///
/// # Examples
///
/// ```
/// use ltmatrix::validate::validate_git_repository;
/// use std::path::Path;
///
/// validate_git_repository(Path::new(".")).unwrap();
/// ```
pub fn validate_git_repository(repo_path: &Path) -> Result<()> {
    debug!("Validating git repository at: {:?}", repo_path);

    // Check if path exists
    if !repo_path.exists() {
        bail!(
            "Path does not exist: '{}'. Cannot validate git repository.",
            repo_path.display()
        );
    }

    // Check if path is a directory
    if !repo_path.is_dir() {
        bail!(
            "Path is not a directory: '{}'. A git repository must be a directory.",
            repo_path.display()
        );
    }

    // Check for .git directory or file (for worktrees)
    let git_path = repo_path.join(".git");
    if !git_path.exists() {
        bail!(
            "Not a git repository: '{}'. No .git directory found. \
             Initialize a git repository with: git init",
            repo_path.display()
        );
    }

    // Try to open the repository with git2
    let repo = git2::Repository::discover(repo_path).context(format!(
        "Failed to open git repository at '{}'. The repository may be corrupted.",
        repo_path.display()
    ))?;

    // Check if repository is bare
    if repo.is_bare() {
        bail!(
            "Git repository at '{}' is a bare repository. \
             ltmatrix requires a working directory.",
            repo_path.display()
        );
    }

    debug!("Git repository validation passed: {:?}", repo_path);
    Ok(())
}

/// Validates file system permissions for read/write access
///
/// # Arguments
///
/// * `path` - Path to check permissions for
/// * `require_write` - Whether to require write permission
///
/// # Returns
///
/// Returns `Ok(())` if permissions are sufficient, or an error with details.
///
/// # Examples
///
/// ```
/// use ltmatrix::validate::validate_file_permissions;
/// use std::path::Path;
///
/// // Check read permission
/// validate_file_permissions(Path::new("."), false).unwrap();
///
/// // Check read/write permission
/// validate_file_permissions(Path::new("."), true).unwrap();
/// ```
pub fn validate_file_permissions(path: &Path, require_write: bool) -> Result<()> {
    debug!(
        "Validating file permissions for: {:?} (write={})",
        path, require_write
    );

    // Check if path exists
    if !path.exists() {
        bail!(
            "Path does not exist: '{}'. Cannot validate permissions.",
            path.display()
        );
    }

    // Check read permission
    std::fs::metadata(path).context(format!(
        "Failed to read metadata for '{}'. Insufficient permissions or path is inaccessible.",
        path.display()
    ))?;

    // For directories, check if we can list contents
    if path.is_dir() {
        std::fs::read_dir(path).context(format!(
            "Cannot read directory '{}'. Permission denied.",
            path.display()
        ))?;

        if require_write {
            // Try to create a temporary file to test write permission
            let test_file = path.join(".ltmatrix_permission_test");
            match std::fs::File::create(&test_file) {
                Ok(_) => {
                    // Clean up the test file
                    let _ = std::fs::remove_file(&test_file);
                }
                Err(e) => {
                    bail!(
                        "Cannot write to directory '{}'. Permission denied. Error: {}",
                        path.display(),
                        e
                    );
                }
            }
        }
    } else {
        // For files, check if we can read
        std::fs::File::open(path).context(format!(
            "Cannot read file '{}'. Permission denied.",
            path.display()
        ))?;

        if require_write {
            // Try to open for writing (append mode to preserve content)
            std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(path)
                .context(format!(
                    "Cannot write to file '{}'. Permission denied.",
                    path.display()
                ))?;
        }
    }

    debug!("File permissions validation passed: {:?}", path);
    Ok(())
}

/// Validates that a directory can be created (parent exists and is writable)
///
/// # Arguments
///
/// * `dir_path` - Path to the directory to validate
///
/// # Returns
///
/// Returns `Ok(())` if the directory can be created, or an error with details.
///
/// # Examples
///
/// ```
/// use ltmatrix::validate::validate_directory_creatable;
/// use std::path::Path;
///
/// validate_directory_creatable(Path::new("./new_dir")).unwrap();
/// ```
pub fn validate_directory_creatable(dir_path: &Path) -> Result<()> {
    debug!("Validating directory can be created: {:?}", dir_path);

    // If directory already exists, check permissions
    if dir_path.exists() {
        return validate_file_permissions(dir_path, true);
    }

    // Check parent directory
    let parent = dir_path
        .parent()
        .context(format!(
            "Cannot determine parent directory for '{}'.",
            dir_path.display()
        ))?;

    if !parent.exists() {
        bail!(
            "Parent directory does not exist: '{}'. Cannot create directory '{}'.",
            parent.display(),
            dir_path.display()
        );
    }

    // Check if parent is writable
    validate_file_permissions(parent, true)?;

    debug!("Directory creation validation passed: {:?}", dir_path);
    Ok(())
}

/// Validates workspace directory for ltmatrix operations
///
/// Checks that the workspace is suitable for ltmatrix operations:
/// - Directory exists and is writable
/// - If a git repository, it's in a valid state
///
/// # Arguments
///
/// * `workspace_path` - Path to the workspace directory
///
/// # Returns
///
/// Returns `Ok(())` if the workspace is valid, or an error with details.
///
/// # Examples
///
/// ```
/// use ltmatrix::validate::validate_workspace;
/// use std::path::Path;
///
/// validate_workspace(Path::new(".")).unwrap();
/// ```
pub fn validate_workspace(workspace_path: &Path) -> Result<()> {
    debug!("Validating workspace: {:?}", workspace_path);

    // Check basic permissions
    validate_file_permissions(workspace_path, true)?;

    // Optionally validate git repository (don't fail if not a git repo)
    if workspace_path.join(".git").exists() {
        validate_git_repository(workspace_path)?;
    } else {
        warn!(
            "Workspace '{}' is not a git repository. Git integration features will be disabled.",
            workspace_path.display()
        );
    }

    debug!("Workspace validation passed: {:?}", workspace_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_validate_goal_valid() {
        assert!(validate_goal("Build a REST API").is_ok());
        assert!(validate_goal("Create a web application with authentication").is_ok());
        assert!(validate_goal("Fix bug in user registration").is_ok());
    }

    #[test]
    fn test_validate_goal_empty() {
        assert!(validate_goal("").is_err());
        assert!(validate_goal("   ").is_err());
        assert!(validate_goal("\n\t").is_err());
    }

    #[test]
    fn test_validate_goal_too_long() {
        let long_goal = "a".repeat(MAX_GOAL_LENGTH + 1);
        assert!(validate_goal(&long_goal).is_err());
    }

    #[test]
    fn test_validate_goal_no_alphanumeric() {
        assert!(validate_goal("!!!").is_err());
        assert!(validate_goal("---").is_err());
        assert!(validate_goal("   ").is_err());
    }

    #[test]
    fn test_validate_task_id_format_valid() {
        assert!(validate_task_id_format("task-1").is_ok());
        assert!(validate_task_id_format("task-2").is_ok());
        assert!(validate_task_id_format("task-123").is_ok());
        assert!(validate_task_id_format("task-1-1").is_ok());
        assert!(validate_task_id_format("task-2-3-4").is_ok());
    }

    #[test]
    fn test_validate_task_id_format_invalid() {
        assert!(validate_task_id_format("").is_err());
        assert!(validate_task_id_format("task").is_err());
        assert!(validate_task_id_format("task-").is_err());
        assert!(validate_task_id_format("Task-1").is_err());
        assert!(validate_task_id_format("task_1").is_err());
        assert!(validate_task_id_format("1-task").is_err());
        assert!(validate_task_id_format("task-1-").is_err());
        assert!(validate_task_id_format("task--1").is_err());
    }

    #[test]
    fn test_validate_task_ids_unique_valid() {
        assert!(validate_task_ids_unique(&["task-1".to_string()]).is_ok());
        assert!(
            validate_task_ids_unique(&["task-1".to_string(), "task-2".to_string()]).is_ok()
        );
        assert!(
            validate_task_ids_unique(&[
                "task-1".to_string(),
                "task-2".to_string(),
                "task-3".to_string()
            ])
            .is_ok()
        );
    }

    #[test]
    fn test_validate_task_ids_unique_duplicates() {
        assert!(validate_task_ids_unique(&["task-1".to_string(), "task-1".to_string()]).is_err());
        assert!(
            validate_task_ids_unique(&[
                "task-1".to_string(),
                "task-2".to_string(),
                "task-1".to_string()
            ])
            .is_err()
        );
    }

    #[test]
    fn test_validate_git_repository_valid() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Initialize a git repository
        git2::Repository::init(repo_path).unwrap();

        assert!(validate_git_repository(repo_path).is_ok());
    }

    #[test]
    fn test_validate_git_repository_not_found() {
        let temp_dir = TempDir::new().unwrap();
        assert!(validate_git_repository(temp_dir.path()).is_err());
    }

    #[test]
    fn test_validate_git_repository_not_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("not_a_dir");
        fs::write(&file_path, "content").unwrap();

        assert!(validate_git_repository(&file_path).is_err());
    }

    #[test]
    fn test_validate_file_permissions_read() {
        let temp_dir = TempDir::new().unwrap();
        assert!(validate_file_permissions(temp_dir.path(), false).is_ok());
    }

    #[test]
    fn test_validate_file_permissions_write() {
        let temp_dir = TempDir::new().unwrap();
        assert!(validate_file_permissions(temp_dir.path(), true).is_ok());
    }

    #[test]
    fn test_validate_file_permissions_nonexistent() {
        assert!(validate_file_permissions(Path::new("/nonexistent/path"), false).is_err());
    }

    #[test]
    fn test_validate_directory_creatable() {
        let temp_dir = TempDir::new().unwrap();
        let new_dir = temp_dir.path().join("new_directory");

        assert!(validate_directory_creatable(&new_dir).is_ok());
    }

    #[test]
    fn test_validate_directory_creatable_already_exists() {
        let temp_dir = TempDir::new().unwrap();

        assert!(validate_directory_creatable(temp_dir.path()).is_ok());
    }

    #[test]
    fn test_validate_workspace_valid() {
        let temp_dir = TempDir::new().unwrap();
        assert!(validate_workspace(temp_dir.path()).is_ok());
    }

    #[test]
    fn test_validate_workspace_with_git() {
        let temp_dir = TempDir::new().unwrap();
        git2::Repository::init(temp_dir.path()).unwrap();

        assert!(validate_workspace(temp_dir.path()).is_ok());
    }

    #[test]
    fn test_validate_agent_available_invalid() {
        // This command is unlikely to exist
        assert!(validate_agent_available("nonexistent_agent_command_xyz123").is_err());
    }

    #[test]
    fn test_get_agent_installation_hint() {
        assert!(get_agent_installation_hint("claude").contains("Claude Code"));
        assert!(get_agent_installation_hint("opencode").contains("OpenCode"));
        assert!(get_agent_installation_hint("kimi-code").contains("KimiCode"));
        assert!(get_agent_installation_hint("codex").contains("Codex"));
        assert!(get_agent_installation_hint("unknown").contains("PATH"));
    }
}