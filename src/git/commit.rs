//! Git commit operations
//!
//! This module provides commit functionality including staging changes,
//! creating commits with messages, and handling common commit failures.

use std::path::Path;
use anyhow::{Context, Result, bail};
use git2::{Repository, Oid, Tree};
use crate::git::repository::create_signature;

/// Commits staged changes with the specified message.
///
/// This function creates a commit with the current staged changes and
/// the provided commit message.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `message` - The commit message
///
/// # Returns
///
/// Returns `Result<Oid>` containing the commit ID or an error.
///
/// # Errors
///
/// - Returns an error if nothing is staged
/// - Returns an error if HEAD cannot be retrieved
/// - Returns an error if the commit fails
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, commit::commit_changes};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// let commit_id = commit_changes(&repo, "Add new feature")?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn commit_changes(repo: &Repository, message: &str) -> Result<Oid> {
    // Validate commit message
    let message = validate_commit_message(message)?;

    // Get the index to check if there are staged changes
    let mut index = repo.index()
        .context("Failed to get repository index")?;

    // Check if there are any staged changes
    if index.is_empty() {
        bail!("No changes staged for commit");
    }

    // Write the index to a tree
    let tree_oid = index.write_tree()
        .context("Failed to write tree from index")?;
    let tree = repo.find_tree(tree_oid)
        .context("Failed to find tree")?;

    // Get HEAD commit for parent
    let head_commit = get_head_commit(repo)?;

    // Create signature
    let sig = create_signature("Ltmatrix Agent", "ltmatrix@agent")?;

    // Build parents list
    let parents: Vec<&git2::Commit> = if let Some(parent) = &head_commit {
        vec![parent]
    } else {
        vec![]
    };

    // Create the commit
    let oid = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &message,
        &tree,
        parents.as_slice(),
    )
    .context("Failed to create commit")?;

    tracing::info!("Created commit: {}", oid);

    Ok(oid)
}

/// Stages specific files for commit.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `files` - Slice of file paths to stage
///
/// # Returns
///
/// Returns `Result<()>` indicating success or an error.
///
/// # Errors
///
/// - Returns an error if a file doesn't exist and is being added
/// - Returns an error if staging fails
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, commit::stage_files};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// stage_files(&repo, &["src/main.rs", "README.md"])?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn stage_files(repo: &Repository, files: &[&str]) -> Result<()> {
    let workdir = repo.workdir()
        .context("Failed to get repository workdir")?;
    let mut index = repo.index()
        .context("Failed to get repository index")?;

    for file in files {
        let path = Path::new(file);

        // Resolve to absolute path relative to repository workdir if not already absolute
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            workdir.join(path)
        };

        // Convert to relative path for git operations
        let relative_path = if path.is_absolute() {
            path.strip_prefix(workdir)
                .with_context(|| format!("File {} is not in repository", file))?
        } else {
            path
        };

        if full_path.exists() {
            // Add file using add_all with the specific file path as a pattern
            let path_str = relative_path.to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in path: {}", file))?;

            index.add_all(
                vec![path_str],
                git2::IndexAddOption::DEFAULT,
                None
            )
                .with_context(|| format!("Failed to stage file: {}", file))?;
        } else {
            // Remove file from index if it doesn't exist in working directory
            let _ = index.remove(relative_path, 0);
        }
    }

    // Write to persist all changes
    index.write()
        .context("Failed to write index")?;

    tracing::debug!("Staged {} file(s)", files.len());

    Ok(())
}

/// Stages all changes in the repository.
///
/// This stages all modified, new, and deleted files.
///
/// # Arguments
///
/// * `repo` - The Git repository
///
/// # Returns
///
/// Returns `Result<()>` indicating success or an error.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, commit::stage_all};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// stage_all(&repo)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn stage_all(repo: &Repository) -> Result<()> {
    let mut index = repo.index()
        .context("Failed to get repository index")?;

    // Stage all changes
    index.add_all(vec!["*"], git2::IndexAddOption::DEFAULT, None)
        .context("Failed to stage all changes")?;

    index.write()
        .context("Failed to write index")?;

    tracing::debug!("Staged all changes");

    Ok(())
}

/// Creates a commit with custom parents and reference.
///
/// This is a lower-level function for advanced commit operations.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `message` - The commit message
/// * `tree` - The tree object for the commit
/// * `parents` - Parent commit IDs
/// * `update_ref` - Reference to update (e.g., "HEAD")
///
/// # Returns
///
/// Returns `Result<Oid>` containing the commit ID or an error.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, commit::create_commit};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// // Get tree and parents...
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn create_commit(
    repo: &Repository,
    message: &str,
    tree: &Tree,
    parents: &[Oid],
    update_ref: &str,
) -> Result<Oid> {
    let message = validate_commit_message(message)?;

    // Create signature
    let sig = create_signature("Ltmatrix Agent", "ltmatrix@agent")?;

    // Resolve parent commits
    let parent_commits: Vec<git2::Commit> = parents
        .iter()
        .map(|oid| {
            repo.find_commit(*oid)
                .with_context(|| format!("Failed to find parent commit: {}", oid))
        })
        .collect::<Result<_>>()?;

    // Convert to slice of references
    let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();

    // Create the commit
    let oid = repo.commit(
        Some(update_ref),
        &sig,
        &sig,
        &message,
        tree,
        parent_refs.as_slice(),
    )
    .context("Failed to create commit")?;

    tracing::info!("Created commit {}: {}", oid, message.lines().next().unwrap_or(""));

    Ok(oid)
}

/// Gets the HEAD commit if it exists.
///
/// # Arguments
///
/// * `repo` - The Git repository
///
/// # Returns
///
/// Returns `Option<git2::Commit>` containing the HEAD commit or None if
/// the repository has no commits yet.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, commit::get_head_commit};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// if let Some(commit) = get_head_commit(&repo)? {
///     println!("HEAD: {}", commit.id());
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_head_commit(repo: &Repository) -> Result<Option<git2::Commit>> {
    match repo.head() {
        Ok(head) => {
            let commit = head.peel_to_commit()
                .context("Failed to peel HEAD to commit")?;
            Ok(Some(commit))
        }
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
            // Repository has no commits yet
            Ok(None)
        }
        Err(e) => {
            Err(e).context("Failed to get HEAD")
        }
    }
}

/// Checks if there are staged changes.
///
/// # Arguments
///
/// * `repo` - The Git repository
///
/// # Returns
///
/// Returns `Result<bool>` where true means there are staged changes.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, commit::has_staged_changes};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// if has_staged_changes(&repo)? {
///     println!("There are staged changes");
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn has_staged_changes(repo: &Repository) -> Result<bool> {
    let index = repo.index()
        .context("Failed to get repository index")?;

    Ok(!index.is_empty())
}

/// Checks if there are unstaged changes.
///
/// # Arguments
///
/// * `repo` - The Git repository
///
/// # Returns
///
/// Returns `Result<bool>` where true means there are unstaged changes.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, commit::has_unstaged_changes};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// if has_unstaged_changes(&repo)? {
///     println!("There are unstaged changes");
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn has_unstaged_changes(repo: &Repository) -> Result<bool> {
    let index = repo.index()
        .context("Failed to get repository index")?;

    // Create diff options to include untracked files
    let mut diffs = git2::DiffOptions::new();
    diffs.include_untracked(true);

    // Create diff from index to working directory
    let diff = repo.diff_index_to_workdir(Some(&index), Some(&mut diffs))
        .context("Failed to create diff")?;

    // Check if there are any differences
    Ok(diff.deltas().count() > 0)
}

/// Validates and normalizes a commit message.
///
/// Ensures the commit message is not empty and has reasonable length.
///
/// # Arguments
///
/// * `message` - The commit message to validate
///
/// # Returns
///
/// Returns `Result<String>` with the normalized message or an error.
///
/// # Errors
///
/// - Returns an error if the message is empty
/// - Returns an error if the message is too long
///
/// # Examples
///
/// ```
/// use ltmatrix::git::commit::validate_commit_message;
///
/// assert!(validate_commit_message("Add feature").is_ok());
/// assert!(validate_commit_message("").is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn validate_commit_message(message: &str) -> Result<String> {
    let trimmed = message.trim();

    if trimmed.is_empty() {
        bail!("Commit message cannot be empty");
    }

    if trimmed.len() > 65536 {
        bail!("Commit message too long (max 65536 characters)");
    }

    // Normalize line endings to \n
    let normalized = trimmed.replace("\r\n", "\n");

    Ok(normalized)
}

/// Amends the last commit with staged changes.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `message` - Optional new commit message (uses original if None)
///
/// # Returns
///
/// Returns `Result<Oid>` containing the new commit ID or an error.
///
/// # Errors
///
/// - Returns an error if there are no commits to amend
/// - Returns an error if there are no staged changes
/// - Returns an error if the amend fails
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, commit::amend_commit};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// let new_commit_id = amend_commit(&repo, Some("Updated message"))?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn amend_commit(repo: &Repository, message: Option<&str>) -> Result<Oid> {
    // Get HEAD commit and HEAD reference
    let head_commit = get_head_commit(repo)?
        .ok_or_else(|| anyhow::anyhow!("Cannot amend: no commits in repository"))?;

    let head = repo.head()
        .context("Failed to get HEAD reference")?;
    let head_name = head.shorthand()
        .context("Failed to get HEAD name")?
        .to_string();

    // Get current commit message if not provided
    let commit_message = if let Some(msg) = message {
        validate_commit_message(msg)?
    } else {
        head_commit.message()
            .context("Failed to get current commit message")?
            .to_string()
    };

    // Stage all changes if not already staged
    if !has_staged_changes(repo)? {
        stage_all(repo)?;
    }

    // Write the index to a tree
    let tree_oid = repo.index()
        .context("Failed to get repository index")?
        .write_tree()
        .context("Failed to write tree from index")?;
    let tree = repo.find_tree(tree_oid)
        .context("Failed to find tree")?;

    // Create signature
    let sig = create_signature("Ltmatrix Agent", "ltmatrix@agent")?;

    // Get parent commits (the HEAD commit's parents)
    let parent_ids: Vec<Oid> = head_commit.parent_ids().collect();
    let mut parent_commits = Vec::new();
    for oid in &parent_ids {
        let commit = repo.find_commit(*oid)
            .context("Failed to find parent commit")?;
        parent_commits.push(commit);
    }
    let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();

    // Create the amended commit without updating HEAD
    let oid = repo.commit(
        None, // Don't update HEAD yet
        &sig,
        &sig,
        &commit_message,
        &tree,
        parent_refs.as_slice(),
    )
    .context("Failed to create amended commit")?;

    // Update HEAD to point to the new commit
    repo.reference(
        &format!("refs/heads/{}", head_name),
        oid,
        true, // Force overwrite
        "Amend commit",
    )
    .context("Failed to update HEAD reference")?;

    // Set HEAD to the updated reference
    repo.set_head(&format!("refs/heads/{}", head_name))
        .context("Failed to set HEAD")?;

    // Checkout the tree to update the working directory
    let new_commit = repo.find_commit(oid)
        .context("Failed to find amended commit")?;
    let new_tree = new_commit.tree()
        .context("Failed to get amended commit tree")?;
    repo.checkout_tree(new_tree.as_object(), None)
        .context("Failed to checkout amended commit tree")?;

    tracing::info!("Amended commit: {}", oid);

    Ok(oid)
}

/// Gets a short commit ID (abbreviated SHA).
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `oid` - The commit ID
/// * `length` - Length of the short ID (default 7)
///
/// # Returns
///
/// Returns `Result<String>` containing the short commit ID or an error.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::{init_repo, commit::commit_changes, commit::short_commit_id};
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// let commit_id = commit_changes(&repo, "Add feature")?;
/// let short_id = short_commit_id(&repo, &commit_id, 7)?;
/// println!("Short ID: {}", short_id);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn short_commit_id(repo: &Repository, oid: &Oid, length: usize) -> Result<String> {
    let commit = repo.find_commit(*oid)
        .context("Failed to find commit")?;

    let short_id = commit
        .as_object()
        .short_id()
        .context("Failed to get short ID")?;

    // Convert to string and truncate to requested length
    let id_string = short_id.as_str().unwrap_or("");
    Ok(id_string.chars().take(length).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::{self, File};
    use std::io::Write;

    /// Helper function to create an initial commit
    fn create_initial_commit(repo: &Repository) -> Result<Oid> {
        let sig = create_signature("Test", "test@example.com")?;

        let tree_oid = repo.treebuilder(None)?.write()?;
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

    #[test]
    fn test_validate_commit_message_valid() {
        assert!(validate_commit_message("Add feature").is_ok());
        assert!(validate_commit_message("Add feature\n\nDescription").is_ok());
        assert!(validate_commit_message("  Trimmed message  ").is_ok());
    }

    #[test]
    fn test_validate_commit_message_invalid() {
        assert!(validate_commit_message("").is_err());
        assert!(validate_commit_message("   ").is_err());
        assert!(validate_commit_message("\n").is_err());
    }

    #[test]
    fn test_get_head_commit_no_commits() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();

        // No commits yet
        let head = get_head_commit(&repo).unwrap();
        assert!(head.is_none());
    }

    #[test]
    fn test_get_head_commit_with_commit() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();
        create_initial_commit(&repo).unwrap();

        // Has commit
        let head = get_head_commit(&repo).unwrap();
        assert!(head.is_some());
    }

    #[test]
    fn test_has_staged_changes() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();

        // No changes initially
        assert!(!has_staged_changes(&repo).unwrap());

        // Create and stage a file
        let file_path = repo_path.join("test.txt");
        File::create(&file_path).unwrap().write_all(b"content").unwrap();

        stage_files(&repo, &["test.txt"]).unwrap();

        // Should have staged changes
        assert!(has_staged_changes(&repo).unwrap());
    }

    #[test]
    fn test_stage_files() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();

        // Create test files
        File::create(repo_path.join("file1.txt")).unwrap().write_all(b"content1").unwrap();
        File::create(repo_path.join("file2.txt")).unwrap().write_all(b"content2").unwrap();

        // Stage files
        stage_files(&repo, &["file1.txt", "file2.txt"]).unwrap();

        // Verify files are staged
        let index = repo.index().unwrap();
        assert!(index.iter().count() == 2);
    }

    #[test]
    fn test_stage_files_remove() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();

        // Create and stage a file
        let file_path = repo_path.join("file.txt");
        File::create(&file_path).unwrap().write_all(b"content").unwrap();
        stage_files(&repo, &["file.txt"]).unwrap();

        // Remove the file and stage removal
        fs::remove_file(&file_path).unwrap();
        stage_files(&repo, &["file.txt"]).unwrap();

        // Verify file is removed from index
        let index = repo.index().unwrap();
        assert!(index.iter().count() == 0);
    }

    #[test]
    fn test_stage_all() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();

        // Create test files
        File::create(repo_path.join("file1.txt")).unwrap().write_all(b"content1").unwrap();
        File::create(repo_path.join("file2.txt")).unwrap().write_all(b"content2").unwrap();

        // Stage all
        stage_all(&repo).unwrap();

        // Verify all files are staged
        let index = repo.index().unwrap();
        assert!(index.iter().count() == 2);
    }

    #[test]
    fn test_commit_changes() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();

        // Create and stage a file
        File::create(repo_path.join("test.txt")).unwrap().write_all(b"content").unwrap();
        stage_files(&repo, &["test.txt"]).unwrap();

        // Commit the changes
        let commit_id = commit_changes(&repo, "Add test file").unwrap();

        // Verify commit was created
        let commit = repo.find_commit(commit_id).unwrap();
        assert_eq!(commit.message().unwrap(), "Add test file");
    }

    #[test]
    fn test_commit_changes_nothing_staged() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();

        // Try to commit without staging anything
        let result = commit_changes(&repo, "No changes");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No changes staged"));
    }

    #[test]
    fn test_amend_commit() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();

        // Create and commit a file
        File::create(repo_path.join("test.txt")).unwrap().write_all(b"content").unwrap();
        stage_files(&repo, &["test.txt"]).unwrap();
        let original_id = commit_changes(&repo, "Original message").unwrap();

        // Modify and stage the file
        File::create(repo_path.join("test.txt")).unwrap().write_all(b"updated").unwrap();
        stage_files(&repo, &["test.txt"]).unwrap();

        // Amend the commit
        let amended_id = amend_commit(&repo, Some("Amended message")).unwrap();

        // Verify commit was amended
        assert_ne!(original_id, amended_id);
        let commit = repo.find_commit(amended_id).unwrap();
        assert_eq!(commit.message().unwrap(), "Amended message");
    }

    #[test]
    fn test_short_commit_id() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();

        // Create initial commit
        let commit_id = create_initial_commit(&repo).unwrap();

        // Get short ID
        let short_id = short_commit_id(&repo, &commit_id, 7).unwrap();

        assert_eq!(short_id.len(), 7);
        // Short ID should be prefix of full ID
        let full_id = commit_id.to_string();
        assert!(full_id.starts_with(&short_id));
    }

    #[test]
    fn test_amend_commit_no_commits() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();

        // Try to amend without any commits
        let result = amend_commit(&repo, Some("Message"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no commits"));
    }

    #[test]
    fn test_has_unstaged_changes() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = Repository::init(repo_path).unwrap();

        // Initially no changes
        assert!(!has_unstaged_changes(&repo).unwrap());

        // Create a file (unstaged)
        File::create(repo_path.join("test.txt")).unwrap().write_all(b"content").unwrap();

        // Should have unstaged changes
        assert!(has_unstaged_changes(&repo).unwrap());
    }
}
