//! Commit stage of the pipeline
//!
//! This module handles committing changes to git for completed tasks.
//! It implements per-task branching with squash merge strategy:
//! - Creates a task branch from base branch for each completed task
//! - Stages all changes made during task execution
//! - Commits with conventional commit message including task ID and title
//! - Squash merges task branch back to base branch on success
//! - Handles merge conflicts with user notification
//! - Skips if not a git repository or on error

use anyhow::{bail, Context, Result};
use git2::Repository;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

use crate::git::{
    checkout, commit_changes, create_branch, delete_branch, get_current_branch, merge_with_squash,
    stage_all,
};
use crate::models::Task;

/// Configuration for the commit stage
#[derive(Debug, Clone)]
pub struct CommitConfig {
    /// Whether to enable committing (can be disabled by config)
    pub enabled: bool,

    /// Base branch to merge task branches into (default: current branch)
    pub base_branch: Option<String>,

    /// Whether to skip if not in a git repository
    pub skip_if_no_repo: bool,

    /// Whether to delete task branches after successful merge
    pub delete_after_merge: bool,

    /// Whether to create per-task branches (vs direct commits)
    pub use_task_branches: bool,

    /// Commit message prefix for conventional commits
    pub commit_type: String,

    /// Working directory
    pub work_dir: PathBuf,
}

impl Default for CommitConfig {
    fn default() -> Self {
        CommitConfig {
            enabled: true,
            base_branch: None,
            skip_if_no_repo: true,
            delete_after_merge: true,
            use_task_branches: true,
            commit_type: "feat".to_string(),
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

impl CommitConfig {
    /// Create config for fast mode (simpler commits)
    pub fn fast_mode() -> Self {
        CommitConfig {
            enabled: true,
            base_branch: None,
            skip_if_no_repo: true,
            delete_after_merge: false,
            use_task_branches: false, // Direct commits in fast mode
            commit_type: "feat".to_string(),
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create config for expert mode (full branching strategy)
    pub fn expert_mode() -> Self {
        CommitConfig {
            enabled: true,
            base_branch: None,
            skip_if_no_repo: false, // Fail if not a git repo
            delete_after_merge: true,
            use_task_branches: true,
            commit_type: "feat".to_string(),
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

/// Result of committing a single task
#[derive(Debug, Clone)]
pub struct CommitResult {
    /// The task that was committed
    pub task: Task,

    /// Whether the commit was successful
    pub success: bool,

    /// Commit ID if successful
    pub commit_id: Option<String>,

    /// Branch name created for the task (if using task branches)
    pub branch_name: Option<String>,

    /// Error message if commit failed
    pub error: Option<String>,

    /// Whether there were merge conflicts (for task branches)
    pub had_conflicts: bool,

    /// Files changed in this commit
    pub files_changed: Vec<String>,
}

/// Summary of commit operations for multiple tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSummary {
    /// Total tasks processed
    pub total_tasks: usize,

    /// Tasks successfully committed
    pub committed_tasks: usize,

    /// Tasks that failed to commit
    pub failed_tasks: usize,

    /// Tasks skipped (not in git repo or no changes)
    pub skipped_tasks: usize,

    /// Tasks with merge conflicts
    pub conflicts: usize,

    /// Branches created
    pub branches_created: usize,

    /// Branches deleted after merge
    pub branches_deleted: usize,

    /// Total commits created
    pub total_commits: usize,

    /// Base branch used for merging
    pub base_branch: Option<String>,
}

impl CommitSummary {
    /// Returns true if all tasks were committed successfully
    pub fn is_complete_success(&self) -> bool {
        self.failed_tasks == 0 && self.conflicts == 0
    }

    /// Returns true if any tasks had conflicts
    pub fn has_conflicts(&self) -> bool {
        self.conflicts > 0
    }
}

/// Commit changes for completed tasks
///
/// This function processes a list of tasks and commits changes for each
/// completed task using the configured strategy (per-task branches or direct commits).
///
/// # Arguments
///
/// * `tasks` - Tasks to process (only completed tasks will be committed)
/// * `config` - Configuration for commit behavior
///
/// # Returns
///
/// Returns `Result<(Vec<Task>, CommitSummary)>` containing updated tasks and commit summary.
///
/// # Errors
///
/// - Returns error if not in a git repository and skip_if_no_repo is false
/// - Returns error if unable to open or work with the git repository
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::pipeline::commit::{commit_tasks, CommitConfig};
/// use ltmatrix::models::Task;
///
/// # async fn example() -> anyhow::Result<()> {
/// let tasks = vec![Task::new("task-1", "Add feature", "Implement new feature")];
/// let config = CommitConfig::default();
/// let (updated_tasks, summary) = commit_tasks(tasks, &config).await?;
/// # Ok(())
/// # }
/// ```
pub async fn commit_tasks(
    tasks: Vec<Task>,
    config: &CommitConfig,
) -> Result<(Vec<Task>, CommitSummary)> {
    let total_tasks = tasks.len();
    info!("Starting commit stage for {} tasks", total_tasks);

    // Check if committing is enabled
    if !config.enabled {
        info!("Commit stage is disabled, skipping");
        return Ok((
            tasks,
            CommitSummary {
                total_tasks,
                committed_tasks: 0,
                failed_tasks: 0,
                skipped_tasks: total_tasks,
                conflicts: 0,
                branches_created: 0,
                branches_deleted: 0,
                total_commits: 0,
                base_branch: config.base_branch.clone(),
            },
        ));
    }

    // Open git repository or skip/return error
    let repo = match open_repository(&config.work_dir) {
        Some(r) => r,
        None => {
            if config.skip_if_no_repo {
                info!("Not in a git repository, skipping commit stage");
                return Ok((
                    tasks,
                    CommitSummary {
                        total_tasks,
                        committed_tasks: 0,
                        failed_tasks: 0,
                        skipped_tasks: total_tasks,
                        conflicts: 0,
                        branches_created: 0,
                        branches_deleted: 0,
                        total_commits: 0,
                        base_branch: config.base_branch.clone(),
                    },
                ));
            } else {
                bail!("Not in a git repository and skip_if_no_repo is false");
            }
        }
    };

    // Get current branch as base branch
    let base_branch = if let Some(ref branch) = config.base_branch {
        branch.clone()
    } else {
        get_current_branch(&repo).context("Failed to get current branch")?
    };

    info!("Using base branch: {}", base_branch);

    // Filter for completed tasks
    let completed_tasks: Vec<Task> = tasks.into_iter().filter(|t| t.is_completed()).collect();

    debug!("Found {} completed tasks to commit", completed_tasks.len());

    let mut results = Vec::new();
    let mut summary = CommitSummary {
        total_tasks: completed_tasks.len(),
        committed_tasks: 0,
        failed_tasks: 0,
        skipped_tasks: 0,
        conflicts: 0,
        branches_created: 0,
        branches_deleted: 0,
        total_commits: 0,
        base_branch: Some(base_branch.clone()),
    };

    // Process each completed task
    for task in completed_tasks {
        let result = if config.use_task_branches {
            commit_task_with_branch(&repo, &task, &base_branch, config).await?
        } else {
            commit_task_direct(&repo, &task, config).await?
        };

        // Update summary
        if result.success {
            summary.committed_tasks += 1;
            if result.branch_name.is_some() {
                summary.branches_created += 1;
            }
            if result.commit_id.is_some() {
                summary.total_commits += 1;
            }
            if result.had_conflicts {
                summary.conflicts += 1;
            }
        } else {
            summary.failed_tasks += 1;
        }

        results.push(result.task);
    }

    info!(
        "Commit stage completed: {}/{} tasks committed, {} conflicts",
        summary.committed_tasks, total_tasks, summary.conflicts
    );

    Ok((results, summary))
}

/// Open git repository at the specified path
///
/// Returns None if not in a git repository, Some(Repository) if successful.
/// Uses Repository::open which only checks the exact directory, not parent directories.
pub fn open_repository(work_dir: &Path) -> Option<Repository> {
    match Repository::open(work_dir) {
        Ok(repo) => {
            debug!("Found git repository at: {:?}", repo.path());
            Some(repo)
        }
        Err(e) => {
            debug!("Not in a git repository: {}", e);
            None
        }
    }
}

/// Commit a task using per-task branch strategy
///
/// This creates a new branch for the task, commits changes, and squash merges
/// back to the base branch.
async fn commit_task_with_branch(
    repo: &Repository,
    task: &Task,
    base_branch: &str,
    config: &CommitConfig,
) -> Result<CommitResult> {
    let task_branch = format!("task-{}", task.id);
    let mut result = CommitResult {
        task: task.clone(),
        success: false,
        commit_id: None,
        branch_name: Some(task_branch.clone()),
        error: None,
        had_conflicts: false,
        files_changed: Vec::new(),
    };

    info!("Committing task {} using branch strategy", task.id);

    // Save current branch
    let original_branch = match get_current_branch(repo) {
        Ok(branch) => branch,
        Err(e) => {
            result.error = Some(format!("Failed to get current branch: {}", e));
            return Ok(result);
        }
    };

    // Create and checkout task branch
    if let Err(e) = create_and_checkout_task_branch(repo, &task_branch) {
        result.error = Some(format!("Failed to create task branch: {}", e));
        // Try to return to original branch
        let _ = checkout(repo, &original_branch);
        return Ok(result);
    }

    // Stage all changes
    if let Err(e) = stage_all(repo) {
        result.error = Some(format!("Failed to stage changes: {}", e));
        let _ = checkout(repo, &original_branch);
        let _ = delete_branch(repo, &task_branch);
        return Ok(result);
    }

    // Build commit message
    let commit_message = build_commit_message(task, &config.commit_type);

    // Commit changes
    let commit_oid = match commit_changes(repo, &commit_message) {
        Ok(oid) => oid,
        Err(e) => {
            // Check if it's because there are no changes
            if e.to_string().contains("No changes staged") {
                info!("No changes to commit for task {}", task.id);
                result.success = true;
                result.error = Some("No changes to commit".to_string());
                let _ = checkout(repo, &original_branch);
                let _ = delete_branch(repo, &task_branch);
                return Ok(result);
            } else {
                result.error = Some(format!("Failed to commit: {}", e));
                let _ = checkout(repo, &original_branch);
                let _ = delete_branch(repo, &task_branch);
                return Ok(result);
            }
        }
    };

    result.commit_id = Some(commit_oid.to_string());
    debug!("Created commit {} for task {}", commit_oid, task.id);

    // Return to base branch
    if let Err(e) = checkout(repo, base_branch) {
        result.error = Some(format!("Failed to return to base branch: {}", e));
        return Ok(result);
    }

    // Squash merge task branch into base branch
    match merge_with_squash(repo, &task_branch, &commit_message) {
        Ok(_) => {
            info!("Merged task branch {} into {}", task_branch, base_branch);
            result.success = true;

            // Delete task branch if configured
            if config.delete_after_merge {
                if let Err(e) = delete_branch(repo, &task_branch) {
                    warn!("Failed to delete task branch {}: {}", task_branch, e);
                } else {
                    debug!("Deleted task branch {}", task_branch);
                }
            }
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("conflict") {
                result.had_conflicts = true;
                result.error = Some(format!(
                    "Merge conflicts detected: {}. Please resolve manually.",
                    error_msg
                ));
                error!("Merge conflicts for task {}: {}", task.id, error_msg);

                // Don't delete the branch so user can resolve conflicts
                warn!(
                    "Task branch '{}' preserved for manual conflict resolution",
                    task_branch
                );
            } else {
                result.error = Some(format!("Failed to merge: {}", e));
                error!("Failed to merge task {}: {}", task.id, e);
            }
        }
    }

    Ok(result)
}

/// Commit a task using direct commit strategy (no branching)
async fn commit_task_direct(
    repo: &Repository,
    task: &Task,
    config: &CommitConfig,
) -> Result<CommitResult> {
    let mut result = CommitResult {
        task: task.clone(),
        success: false,
        commit_id: None,
        branch_name: None,
        error: None,
        had_conflicts: false,
        files_changed: Vec::new(),
    };

    info!("Committing task {} using direct commit strategy", task.id);

    // Stage all changes
    if let Err(e) = stage_all(repo) {
        result.error = Some(format!("Failed to stage changes: {}", e));
        return Ok(result);
    }

    // Build commit message
    let commit_message = build_commit_message(task, &config.commit_type);

    // Commit changes
    match commit_changes(repo, &commit_message) {
        Ok(oid) => {
            result.commit_id = Some(oid.to_string());
            result.success = true;
            info!("Created commit {} for task {}", oid, task.id);
        }
        Err(e) => {
            // Check if it's because there are no changes
            if e.to_string().contains("No changes staged") {
                info!("No changes to commit for task {}", task.id);
                result.success = true;
                result.error = Some("No changes to commit".to_string());
            } else {
                result.error = Some(format!("Failed to commit: {}", e));
            }
        }
    }

    Ok(result)
}

/// Create and checkout a task branch
///
/// This creates a new branch from the current HEAD and checks it out.
fn create_and_checkout_task_branch(repo: &Repository, branch_name: &str) -> Result<()> {
    // Check if branch already exists
    if crate::git::branch_exists(repo, branch_name) {
        warn!("Branch {} already exists, checking out", branch_name);
        checkout(repo, branch_name)?;
        return Ok(());
    }

    // Create new branch
    create_branch(repo, branch_name).context("Failed to create task branch")?;

    // Checkout the branch
    checkout(repo, branch_name).context("Failed to checkout task branch")?;

    debug!("Created and checked out task branch: {}", branch_name);

    Ok(())
}

/// Build a conventional commit message for a task
///
/// Format: "{type}: [{task-id}] {title}"
///
/// # Arguments
///
/// * `task` - The task to build a message for
/// * `commit_type` - The commit type (e.g., "feat", "fix", "docs")
///
/// # Returns
///
/// A formatted conventional commit message
fn build_commit_message(task: &Task, commit_type: &str) -> String {
    format!("{}: [{}] {}", commit_type, task.id, task.title)
}

/// Display commit summary to the user
pub fn display_commit_summary(summary: &CommitSummary) {
    println!("\n=== Commit Summary ===");
    println!("Total tasks: {}", summary.total_tasks);
    println!("Committed: {}", summary.committed_tasks);
    println!("Failed: {}", summary.failed_tasks);
    println!("Skipped: {}", summary.skipped_tasks);

    if summary.conflicts > 0 {
        println!("⚠️  Conflicts: {}", summary.conflicts);
        println!("\nMerge conflicts detected. Please resolve conflicts manually:");
        println!("1. Check conflicting files: git status");
        println!("2. Resolve conflicts in your editor");
        println!("3. Mark as resolved: git add <files>");
        println!("4. Complete merge: git commit");
    }

    if summary.branches_created > 0 {
        println!("Branches created: {}", summary.branches_created);
    }

    if summary.branches_deleted > 0 {
        println!("Branches deleted: {}", summary.branches_deleted);
    }

    if summary.total_commits > 0 {
        println!("Total commits: {}", summary.total_commits);
    }

    if let Some(ref base) = summary.base_branch {
        println!("Base branch: {}", base);
    }

    if summary.is_complete_success() {
        println!("\n✓ All tasks committed successfully");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_commit_config_default() {
        let config = CommitConfig::default();
        assert!(config.enabled);
        assert!(config.skip_if_no_repo);
        assert!(config.delete_after_merge);
        assert!(config.use_task_branches);
        assert_eq!(config.commit_type, "feat");
    }

    #[test]
    fn test_commit_config_fast_mode() {
        let config = CommitConfig::fast_mode();
        assert!(!config.use_task_branches); // Fast mode uses direct commits
        assert!(!config.delete_after_merge);
    }

    #[test]
    fn test_commit_config_expert_mode() {
        let config = CommitConfig::expert_mode();
        assert!(config.use_task_branches);
        assert!(!config.skip_if_no_repo);
        assert!(config.delete_after_merge);
    }

    #[test]
    fn test_build_commit_message() {
        let task = Task::new("task-123", "Add user authentication", "Implement login");
        let message = build_commit_message(&task, "feat");
        assert_eq!(message, "feat: [task-123] Add user authentication");
    }

    #[test]
    fn test_build_commit_message_with_fix_type() {
        let task = Task::new("task-456", "Fix login bug", "Repair authentication");
        let message = build_commit_message(&task, "fix");
        assert_eq!(message, "fix: [task-456] Fix login bug");
    }

    #[test]
    fn test_commit_summary_is_complete_success() {
        let mut summary = CommitSummary {
            total_tasks: 3,
            committed_tasks: 3,
            failed_tasks: 0,
            skipped_tasks: 0,
            conflicts: 0,
            branches_created: 3,
            branches_deleted: 3,
            total_commits: 3,
            base_branch: Some("main".to_string()),
        };
        assert!(summary.is_complete_success());

        summary.failed_tasks = 1;
        assert!(!summary.is_complete_success());

        summary.failed_tasks = 0;
        summary.conflicts = 1;
        assert!(!summary.is_complete_success());
    }

    #[test]
    fn test_commit_summary_has_conflicts() {
        let mut summary = CommitSummary {
            total_tasks: 3,
            committed_tasks: 2,
            failed_tasks: 0,
            skipped_tasks: 0,
            conflicts: 1,
            branches_created: 3,
            branches_deleted: 2,
            total_commits: 2,
            base_branch: Some("main".to_string()),
        };
        assert!(summary.has_conflicts());

        summary.conflicts = 0;
        assert!(!summary.has_conflicts());
    }

    #[tokio::test]
    async fn test_commit_tasks_not_in_repo() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path();

        let task = Task::new("task-1", "Test", "Description");
        let tasks = vec![task];

        let config = CommitConfig {
            work_dir: work_dir.to_path_buf(),
            skip_if_no_repo: true,
            ..Default::default()
        };

        let (updated_tasks, summary) = commit_tasks(tasks, &config).await.unwrap();

        assert_eq!(summary.total_tasks, 1);
        assert_eq!(summary.skipped_tasks, 1);
        assert_eq!(summary.committed_tasks, 0);
        assert_eq!(updated_tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_commit_tasks_disabled() {
        let task = Task::new("task-1", "Test", "Description");
        let tasks = vec![task];

        let config = CommitConfig {
            enabled: false,
            ..Default::default()
        };

        let (updated_tasks, summary) = commit_tasks(tasks, &config).await.unwrap();

        assert_eq!(summary.skipped_tasks, 1);
        assert_eq!(updated_tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_open_repository_not_a_repo() {
        let temp_dir = TempDir::new().unwrap();
        let repo = open_repository(temp_dir.path());
        assert!(repo.is_none());
    }

    #[test]
    fn test_create_and_checkout_task_branch() {
        let temp_dir = TempDir::new().unwrap();

        // Initialize a git repo
        let repo = Repository::init(temp_dir.path()).unwrap();

        // Create an initial commit first
        let sig = crate::git::repository::create_signature("Test", "test@example.com").unwrap();
        let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .unwrap();

        // Create and checkout task branch
        let result = create_and_checkout_task_branch(&repo, "task-test-1");

        assert!(result.is_ok());

        // Verify we're on the new branch
        let current = get_current_branch(&repo).unwrap();
        assert_eq!(current, "task-test-1");
    }

    #[test]
    fn test_create_and_checkout_existing_branch() {
        let temp_dir = TempDir::new().unwrap();

        // Initialize a git repo
        let repo = Repository::init(temp_dir.path()).unwrap();

        // Create an initial commit first
        let sig = crate::git::repository::create_signature("Test", "test@example.com").unwrap();
        let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .unwrap();

        // Create branch once
        create_and_checkout_task_branch(&repo, "task-test-2").unwrap();

        // Switch back to master
        checkout(&repo, "master").unwrap();

        // Try to create again (should just checkout existing)
        let result = create_and_checkout_task_branch(&repo, "task-test-2");
        assert!(result.is_ok());

        let current = get_current_branch(&repo).unwrap();
        assert_eq!(current, "task-test-2");
    }
}
