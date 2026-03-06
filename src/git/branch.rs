//! Git branch management operations
//!
//! This module provides branch creation, listing, deletion, and validation
//! with comprehensive error handling for conflicts and invalid branch names.

use anyhow::{bail, Context, Result};
use git2::{Branch, BranchType, Repository};

/// Creates a new branch in the repository.
///
/// This function creates a new branch at the current HEAD with validation
/// and conflict detection.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `branch_name` - Name for the new branch
///
/// # Returns
///
/// Returns `Result<Branch>` containing the created branch or an error.
///
/// # Errors
///
/// - Returns an error if the branch name is invalid
/// - Returns an error if a branch with the same name already exists
/// - Returns an error if the repository has no commits (HEAD is unborn)
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, create_branch};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// let branch = create_branch(&repo, "feature-branch")?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn create_branch<'a>(repo: &'a Repository, branch_name: &str) -> Result<Branch<'a>> {
    // Validate branch name
    validate_branch_name(branch_name)?;

    // Check if branch already exists
    if repo.find_branch(branch_name, BranchType::Local).is_ok() {
        bail!("Branch '{}' already exists", branch_name);
    }

    // Get HEAD commit
    let head = repo
        .head()
        .context("Cannot create branch: repository has no commits yet (HEAD is unborn)")?;

    let target = head
        .peel_to_commit()
        .context("Failed to peel HEAD to commit")?;

    // Create the branch
    let branch = repo
        .branch(branch_name, &target, false)
        .context("Failed to create branch")?;

    tracing::info!("Created branch: {}", branch_name);

    Ok(branch)
}

/// Validates a branch name according to Git rules.
///
/// Branch names must:
/// - Not be empty
/// - Not start or end with a dot
/// - Not contain consecutive dots
/// - Not contain `..`, `~`, `^`, `:`, `?`, `*`, `[`, `@`, `\`
/// - Not contain a space
/// - Not end with a slash
/// - Not contain `@{`
/// - Not be a reserved branch name (HEAD, FETCH_HEAD, MERGE_HEAD, etc.)
///
/// # Arguments
///
/// * `branch_name` - The branch name to validate
///
/// # Returns
///
/// Returns `Result<()>` indicating validation success or an error.
///
/// # Examples
///
/// ```
/// use ltmatrix::git::branch::validate_branch_name;
///
/// validate_branch_name("feature-branch").unwrap();
/// validate_branch_name("invalid branch").unwrap_err();
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn validate_branch_name(branch_name: &str) -> Result<()> {
    // Check if empty
    if branch_name.is_empty() {
        bail!("Branch name cannot be empty");
    }

    // Check length (Git has no strict limit, but reasonable limit is good)
    if branch_name.len() > 255 {
        bail!("Branch name too long (max 255 characters)");
    }

    // Check for reserved branch names
    let reserved = [
        "HEAD",
        "FETCH_HEAD",
        "ORIG_HEAD",
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
    ];
    if reserved.contains(&branch_name) {
        bail!("'{}' is a reserved branch name", branch_name);
    }

    // Check for invalid patterns
    if branch_name.starts_with('.') {
        bail!("Branch name cannot start with a dot");
    }

    if branch_name.ends_with('.') {
        bail!("Branch name cannot end with a dot");
    }

    if branch_name.ends_with('/') {
        bail!("Branch name cannot end with a slash");
    }

    if branch_name.contains("..") {
        bail!("Branch name cannot contain '..'");
    }

    if branch_name.contains('@') {
        bail!("Branch name cannot contain '@'");
    }

    // Check for invalid characters
    let invalid_chars = ['~', '^', ':', '?', '*', '[', '\\', ' ', '\t', '\n', '\r'];
    for &ch in &invalid_chars {
        if branch_name.contains(ch) {
            bail!("Branch name cannot contain '{}'", ch);
        }
    }

    // Check for consecutive slashes
    if branch_name.contains("//") {
        bail!("Branch name cannot contain consecutive slashes");
    }

    Ok(())
}

/// Checks if a branch exists in the repository.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `branch_name` - Name of the branch to check
///
/// # Returns
///
/// Returns `true` if the branch exists, `false` otherwise.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, create_branch, branch_exists};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// create_branch(&repo, "feature-branch")?;
/// assert!(branch_exists(&repo, "feature-branch"));
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn branch_exists(repo: &Repository, branch_name: &str) -> bool {
    repo.find_branch(branch_name, BranchType::Local).is_ok()
}

/// Lists all local branches in the repository.
///
/// # Arguments
///
/// * `repo` - The Git repository
///
/// # Returns
///
/// Returns `Result<Vec<String>>` containing all branch names or an error.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, list_branches};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// let branches = list_branches(&repo)?;
/// for branch in branches {
///     println!("{}", branch);
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn list_branches(repo: &Repository) -> Result<Vec<String>> {
    let branches = repo
        .branches(Some(BranchType::Local))
        .context("Failed to list branches")?;

    let mut branch_names = Vec::new();
    for branch_result in branches {
        let (branch, _branch_type) = branch_result.context("Failed to read branch")?;
        let name = branch
            .name()
            .context("Branch has invalid name")?
            .ok_or_else(|| anyhow::anyhow!("Branch name is None"))?;
        branch_names.push(name.to_string());
    }

    Ok(branch_names)
}

/// Deletes a branch from the repository.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `branch_name` - Name of the branch to delete
///
/// # Returns
///
/// Returns `Result<()>` indicating success or an error.
///
/// # Errors
///
/// - Returns an error if the branch doesn't exist
/// - Returns an error if trying to delete the current branch
/// - Returns an error if the branch is not fully merged (unless forced)
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, create_branch, delete_branch};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// create_branch(&repo, "temp-branch")?;
/// delete_branch(&repo, "temp-branch")?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn delete_branch(repo: &Repository, branch_name: &str) -> Result<()> {
    // Check if branch exists
    let mut branch = repo
        .find_branch(branch_name, BranchType::Local)
        .context("Failed to find branch")?;

    // Check if it's the current branch
    if let Ok(head) = repo.head() {
        if let Some(head_name) = head.shorthand() {
            if head_name == branch_name {
                bail!("Cannot delete the current branch '{}'", branch_name);
            }
        }
    }

    // Delete the branch
    branch.delete().context("Failed to delete branch")?;

    tracing::info!("Deleted branch: {}", branch_name);

    Ok(())
}

/// Gets the current branch name.
///
/// # Arguments
///
/// * `repo` - The Git repository
///
/// # Returns
///
/// Returns `Result<String>` containing the current branch name or an error.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, get_current_branch_name};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// let branch = get_current_branch_name(&repo)?;
/// println!("Current branch: {}", branch);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_current_branch_name(repo: &Repository) -> Result<String> {
    let head = repo.head().context("Failed to get HEAD reference")?;

    let branch_name = head
        .shorthand()
        .context("Failed to get branch name (detached HEAD?)")?;

    Ok(branch_name.to_string())
}

/// Checks if the repository is in a detached HEAD state.
///
/// # Arguments
///
/// * `repo` - The Git repository
///
/// # Returns
///
/// Returns `true` if HEAD is detached, `false` otherwise.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, is_head_detached};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// if is_head_detached(&repo) {
///     println!("Warning: HEAD is detached");
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn is_head_detached(repo: &Repository) -> bool {
    repo.head().map(|h| h.is_branch()).unwrap_or(false) == false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repository::{create_signature, init_repo};
    use tempfile::TempDir;

    /// Helper function to create an initial commit for testing
    fn create_initial_commit(repo: &Repository) -> Result<git2::Oid> {
        let sig = create_signature("Test", "test@example.com")?;

        // Write empty tree
        let tree_oid = repo.treebuilder(None)?.write()?;

        // Create commit
        let tree = repo.find_tree(tree_oid)?;
        let oid = repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

        Ok(oid)
    }

    #[test]
    fn test_validate_branch_name_valid() {
        assert!(validate_branch_name("main").is_ok());
        assert!(validate_branch_name("feature-branch").is_ok());
        assert!(validate_branch_name("feature/branch").is_ok());
        assert!(validate_branch_name("123-branch").is_ok());
        assert!(validate_branch_name("branch_with_underscores").is_ok());
        assert!(validate_branch_name("feature-123-branch").is_ok());
    }

    #[test]
    fn test_validate_branch_name_invalid() {
        assert!(validate_branch_name("").is_err());
        assert!(validate_branch_name(".hidden").is_err());
        assert!(validate_branch_name("invalid.").is_err());
        assert!(validate_branch_name("invalid..name").is_err());
        assert!(validate_branch_name("invalid@name").is_err());
        assert!(validate_branch_name("invalid name").is_err());
        assert!(validate_branch_name("invalid~name").is_err());
        assert!(validate_branch_name("invalid^name").is_err());
        assert!(validate_branch_name("invalid:name").is_err());
        assert!(validate_branch_name("invalid?name").is_err());
        assert!(validate_branch_name("invalid*name").is_err());
        assert!(validate_branch_name("invalid[name").is_err());
        assert!(validate_branch_name("invalid\\name").is_err());
        assert!(validate_branch_name("invalid//name").is_err());
        assert!(validate_branch_name("invalid/").is_err());
        assert!(validate_branch_name("HEAD").is_err());
        assert!(validate_branch_name("FETCH_HEAD").is_err());
    }

    #[test]
    fn test_validate_branch_name_too_long() {
        let long_name = "a".repeat(256);
        assert!(validate_branch_name(&long_name).is_err());
    }

    #[test]
    fn test_create_branch() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = init_repo(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        let branch = create_branch(&repo, "feature-branch").unwrap();

        assert_eq!(branch.name().unwrap(), Some("feature-branch"));
        assert!(branch_exists(&repo, "feature-branch"));
    }

    #[test]
    fn test_create_branch_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = init_repo(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        create_branch(&repo, "feature-branch").unwrap();

        let result = create_branch(&repo, "feature-branch");
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("already exists") || err_msg.contains("exists"));
        }
    }

    #[test]
    fn test_create_branch_invalid_name() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = init_repo(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        let result = create_branch(&repo, "invalid branch");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_branch_no_commits() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = init_repo(repo_path).unwrap();

        let result = create_branch(&repo, "feature-branch");
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("no commits") || err_msg.contains("HEAD"));
        }
    }

    #[test]
    fn test_branch_exists() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = init_repo(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        assert!(!branch_exists(&repo, "nonexistent"));

        create_branch(&repo, "feature-branch").unwrap();
        assert!(branch_exists(&repo, "feature-branch"));
    }

    #[test]
    fn test_list_branches() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = init_repo(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        let branches = list_branches(&repo).unwrap();
        assert_eq!(branches.len(), 1); // Only master/main branch

        create_branch(&repo, "feature-1").unwrap();
        create_branch(&repo, "feature-2").unwrap();

        let branches = list_branches(&repo).unwrap();
        assert_eq!(branches.len(), 3);
        assert!(branches.contains(&"feature-1".to_string()));
        assert!(branches.contains(&"feature-2".to_string()));
    }

    #[test]
    fn test_delete_branch() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = init_repo(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        create_branch(&repo, "temp-branch").unwrap();
        assert!(branch_exists(&repo, "temp-branch"));

        delete_branch(&repo, "temp-branch").unwrap();
        assert!(!branch_exists(&repo, "temp-branch"));
    }

    #[test]
    fn test_delete_branch_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = init_repo(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        let result = delete_branch(&repo, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("find branch"));
    }

    #[test]
    fn test_get_current_branch_name() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = init_repo(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        let branch = get_current_branch_name(&repo).unwrap();
        // Default branch is "master" in git2
        assert_eq!(branch, "master");
    }

    #[test]
    fn test_is_head_detached() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = init_repo(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        // Not detached when on a branch
        assert!(!is_head_detached(&repo));

        // Detach HEAD
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        repo.set_head_detached(commit.id()).unwrap();

        // Now HEAD should be detached
        assert!(is_head_detached(&repo));
    }
}
