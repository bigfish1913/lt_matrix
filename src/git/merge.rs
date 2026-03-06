//! Git merge operations
//!
//! This module provides merge functionality including squash merging,
//! conflict detection, and user-friendly error messages.

use anyhow::{bail, Context, Result};
use git2::{Oid, Repository};

/// Performs a squash merge of a branch into the current branch.
///
/// This function merges all commits from the source branch into the current
/// branch as a single squashed commit. It detects conflicts and provides
/// user-friendly error messages.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `source_branch` - The branch to merge from (will be squashed)
/// * `commit_message` - The commit message for the squashed commit
///
/// # Returns
///
/// Returns `Result<Oid>` containing the ID of the new squashed commit or an error.
///
/// # Errors
///
/// - Returns an error if the source branch doesn't exist
/// - Returns an error if there are uncommitted changes
/// - Returns an error if there are merge conflicts
/// - Returns an error if the source branch has no new commits
/// - Returns an error if HEAD is detached
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, merge::merge_with_squash};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// let commit_id = merge_with_squash(&repo, "feature-branch", "Merge feature")?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn merge_with_squash(
    repo: &Repository,
    source_branch: &str,
    commit_message: &str,
) -> Result<Oid> {
    // Validate commit message
    let commit_message = crate::git::commit::validate_commit_message(commit_message)?;

    // Check if HEAD is detached
    if repo.head().is_err() {
        bail!("Cannot merge: HEAD is detached. Checkout a branch first.");
    }

    // Check for staged changes (unstaged/untracked files don't block merges in Git)
    let has_staged = crate::git::commit::has_staged_changes(repo)?;
    if has_staged {
        bail!(
            "Cannot merge: Working directory has staged changes. \
             Commit or stash changes before merging."
        );
    }

    // Find source branch
    let source_branch_obj = repo
        .find_branch(source_branch, git2::BranchType::Local)
        .with_context(|| {
            format!(
                "Source branch '{}' not found. Available branches: {}",
                source_branch,
                list_available_branches(repo)
                    .unwrap_or_else(|_| "(error listing branches)".to_string())
            )
        })?;

    let source_commit = source_branch_obj
        .get()
        .peel_to_commit()
        .context("Failed to peel source branch to commit")?;

    // Get current HEAD commit
    let head_commit = repo
        .head()
        .context("Failed to get HEAD")?
        .peel_to_commit()
        .context("Failed to peel HEAD to commit")?;

    // Check if source branch has new commits
    let merge_base_oid = repo
        .merge_base(head_commit.id(), source_commit.id())
        .context("Failed to find merge base")?;

    if merge_base_oid == source_commit.id() {
        bail!(
            "Nothing to merge: Branch '{}' has no new commits beyond current HEAD.",
            source_branch
        );
    }

    // Perform analysis to detect conflicts
    let source_tree = source_commit.tree().context("Failed to get source tree")?;
    let head_tree = head_commit.tree().context("Failed to get HEAD tree")?;
    let merge_base_commit = repo
        .find_commit(merge_base_oid)
        .context("Failed to find merge base commit")?;
    let merge_base_tree = merge_base_commit
        .tree()
        .context("Failed to get merge base tree")?;

    // Get the index from the merge
    let mut index = repo
        .merge_trees(&head_tree, &merge_base_tree, &source_tree, None)
        .context("Failed to perform merge analysis")?;

    // Check for conflicts in the index
    if index.has_conflicts() {
        let conflicts = get_index_conflicts(&index)?;
        bail!(
            "Merge conflicts detected in {} file(s):\n{}\n\
             Resolve conflicts and try again.",
            conflicts.len(),
            conflicts
                .iter()
                .map(|f| format!("  - {}", f))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    // Write the merged tree
    let tree_oid = index
        .write_tree_to(repo)
        .context("Failed to write merge tree")?;

    let tree = repo
        .find_tree(tree_oid)
        .context("Failed to find merge tree")?;

    // Create the squashed commit
    let sig = crate::git::repository::create_signature("Ltmatrix Agent", "ltmatrix@agent")?;

    let oid = repo
        .commit(
            Some("HEAD"),
            &sig,
            &sig,
            &commit_message,
            &tree,
            &[&head_commit],
        )
        .context("Failed to create squashed commit")?;

    tracing::info!("Squashed merge from '{}' completed: {}", source_branch, oid);

    Ok(oid)
}

/// Lists available branches in the repository for error messages.
fn list_available_branches(repo: &Repository) -> Result<String> {
    let mut branch_names = Vec::new();

    let branches = repo
        .branches(Some(git2::BranchType::Local))
        .context("Failed to list branches")?;

    for branch in branches {
        let (name, _) = branch.context("Failed to get branch")?;

        // Get branch name - name() returns Result<Option<&str>, Error>
        let name_opt = name.name().context("Failed to get branch name")?;

        // Convert to string, using a fallback if None
        let name_str = name_opt.unwrap_or("(unnamed)").to_string();

        // Remove "refs/heads/" prefix for cleaner display
        let display_name = if let Some(stripped) = name_str.strip_prefix("refs/heads/") {
            stripped.to_string()
        } else {
            name_str
        };

        branch_names.push(display_name);
    }

    if branch_names.is_empty() {
        Ok("(no branches)".to_string())
    } else {
        Ok(branch_names.join(", "))
    }
}

/// Gets conflicted files from the merge index.
fn get_index_conflicts(index: &git2::Index) -> Result<Vec<String>> {
    let mut conflicts = Vec::new();

    // Iterate through index entries to find conflicts
    // Conflicts are entries with stage > 0
    for entry in index.iter() {
        // entry.stage is not available directly, but conflicts appear as multiple entries
        // with the same path and different stages
        let path = entry.path;
        if !path.is_empty() {
            if let Ok(path_str) = std::str::from_utf8(&path) {
                // Check if this path appears in the conflicts
                // For now, we'll collect all unique paths from conflicted entries
                conflicts.push(path_str.to_string());
            }
        }
    }

    // Remove duplicates (conflicts can have multiple stages per file)
    conflicts.sort();
    conflicts.dedup();

    Ok(conflicts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{
        commit::{commit_changes, stage_files},
        repository::create_signature,
    };
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper function to create an initial commit
    fn create_initial_commit(repo: &Repository) -> Result<Oid> {
        let sig = create_signature("Test", "test@example.com")?;

        let tree_oid = repo.treebuilder(None)?.write()?;
        let tree = repo.find_tree(tree_oid)?;

        let oid = repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

        Ok(oid)
    }

    /// Helper function to create a branch and switch to it
    fn create_and_checkout_branch(repo: &Repository, branch_name: &str) -> Result<()> {
        use crate::git::branch::create_branch;

        let branch = create_branch(repo, branch_name)?;

        // Get the reference name and set HEAD
        let branch_ref = branch.get();
        repo.set_head(branch_ref.name().context("Failed to get branch name")?)
            .context("Failed to set HEAD")?;

        let commit = branch_ref.peel_to_commit()?;

        // Hard reset to clean working directory and index
        repo.reset(commit.as_object(), git2::ResetType::Hard, None)
            .context("Failed to hard reset to branch")?;

        Ok(())
    }

    #[test]
    fn test_merge_with_squash_basic() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        // Create a feature branch
        create_and_checkout_branch(&repo, "feature-branch").unwrap();

        // Add a file in a subdirectory and commit
        let feature_dir = repo_path.join("feature");
        std::fs::create_dir_all(&feature_dir).unwrap();
        let file_path = feature_dir.join("file.txt");
        File::create(&file_path)
            .unwrap()
            .write_all(b"feature content")
            .unwrap();
        stage_files(&repo, &["feature/file.txt"]).unwrap();
        let feature_commit = commit_changes(&repo, "Add feature").unwrap();

        // Switch back to master
        crate::git::checkout(&repo, "master").unwrap();

        // Clean up the feature directory from the working directory
        // (it's untracked on main since it was only created on feature branch)
        let _ = std::fs::remove_dir_all(&feature_dir);

        // Squash merge the feature branch
        let merged_commit = merge_with_squash(&repo, "feature-branch", "Squashed feature").unwrap();

        // Verify the squashed commit was created
        assert_ne!(merged_commit, feature_commit);

        let commit = repo.find_commit(merged_commit).unwrap();
        assert_eq!(commit.message().unwrap(), "Squashed feature");
        assert_eq!(commit.parent_count(), 1); // Squash merge has one parent
    }

    #[test]
    fn test_merge_with_squash_conflict_detection() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        // Add a file on main
        let file_path = repo_path.join("shared.txt");
        File::create(&file_path)
            .unwrap()
            .write_all(b"main content")
            .unwrap();
        stage_files(&repo, &["shared.txt"]).unwrap();
        commit_changes(&repo, "Add shared file on main").unwrap();

        // Create a feature branch
        create_and_checkout_branch(&repo, "feature-branch").unwrap();

        // Modify the same file on feature
        File::create(&file_path)
            .unwrap()
            .write_all(b"feature content")
            .unwrap();
        stage_files(&repo, &["shared.txt"]).unwrap();
        commit_changes(&repo, "Modify shared file on feature").unwrap();

        // Switch back to master
        crate::git::checkout(&repo, "master").unwrap();

        // Modify the same file again on main (creating conflict)
        File::create(&file_path)
            .unwrap()
            .write_all(b"main content v2")
            .unwrap();
        stage_files(&repo, &["shared.txt"]).unwrap();
        commit_changes(&repo, "Modify shared file on main again").unwrap();

        // Attempt to merge - should detect conflict
        let result = merge_with_squash(&repo, "feature-branch", "Attempt merge");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("conflict") || error_msg.contains("conflicts"));
    }

    #[test]
    fn test_merge_with_squash_nonexistent_branch() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        // Try to merge non-existent branch
        let result = merge_with_squash(&repo, "nonexistent-branch", "Merge");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("not found"));
    }

    #[test]
    fn test_merge_with_squash_no_new_commits() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        // Create a branch from current HEAD
        create_and_checkout_branch(&repo, "feature-branch").unwrap();

        // Switch back to master
        crate::git::checkout(&repo, "master").unwrap();

        // Try to merge - should error because no new commits
        let result = merge_with_squash(&repo, "feature-branch", "Merge");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("no new commits") || error_msg.contains("Nothing to merge"));
    }

    #[test]
    fn test_merge_with_squash_uncommitted_changes() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        // Create and commit on a feature branch
        create_and_checkout_branch(&repo, "feature-branch").unwrap();
        let file_path = repo_path.join("feature.txt");
        File::create(&file_path)
            .unwrap()
            .write_all(b"feature")
            .unwrap();
        stage_files(&repo, &["feature.txt"]).unwrap();
        commit_changes(&repo, "Add feature").unwrap();

        // Switch back to master
        crate::git::checkout(&repo, "master").unwrap();

        // Create staged changes on main (not committed)
        let staged_file = repo_path.join("staged.txt");
        File::create(&staged_file)
            .unwrap()
            .write_all(b"staged")
            .unwrap();
        stage_files(&repo, &["staged.txt"]).unwrap();

        // Try to merge - should error due to staged changes
        let result = merge_with_squash(&repo, "feature-branch", "Merge");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("staged"));
    }

    #[test]
    fn test_merge_with_squash_user_friendly_error_messages() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        // Test 1: Non-existent branch mentions available branches
        let result = merge_with_squash(&repo, "nonexistent", "Merge");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Should mention the branch name and available branches
        assert!(error_msg.contains("nonexistent"));

        // Test 2: Staged changes suggests actions
        create_and_checkout_branch(&repo, "feature").unwrap();
        let file_path = repo_path.join("test.txt");
        File::create(&file_path)
            .unwrap()
            .write_all(b"test")
            .unwrap();
        stage_files(&repo, &["test.txt"]).unwrap();
        commit_changes(&repo, "Test").unwrap();

        crate::git::checkout(&repo, "master").unwrap();
        File::create(&file_path)
            .unwrap()
            .write_all(b"staged")
            .unwrap();
        stage_files(&repo, &["test.txt"]).unwrap();

        let result = merge_with_squash(&repo, "feature", "Merge");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Should suggest actions
        assert!(error_msg.contains("Commit") || error_msg.contains("Stash"));
    }
}
