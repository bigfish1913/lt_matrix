// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Git repository operations
//!
//! This module provides high-level Git repository operations including
//! initialization, .gitignore generation, and branch management.

use anyhow::{Context, Result};
use git2::{Oid, Repository, RepositoryInitOptions, Signature, Time};
use std::path::Path;

/// Initializes a new Git repository in the specified directory.
///
/// This function creates a new Git repository with proper configuration:
/// - Initializes the repository
/// - Configures user.name and user.email
/// - Generates a comprehensive .gitignore file
/// - Handles nested repository protection (adds workspace to parent .gitignore)
///
/// # Arguments
///
/// * `path` - Path where the repository should be initialized
///
/// # Returns
///
/// Returns a `Result` containing the initialized `Repository` or an error.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::repository::init_repo;
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn init_repo(path: &Path) -> Result<Repository> {
    // Ensure the directory exists
    std::fs::create_dir_all(path).context("Failed to create repository directory")?;

    // Handle nested repository protection
    if let Some(parent) = path.parent() {
        if parent.exists() {
            protect_nested_repo(path, parent)?;
        }
    }

    // Initialize repository with options
    let mut opts = RepositoryInitOptions::new();
    opts.no_reinit(true); // Don't re-initialize if already a git repo

    let repo = Repository::init_opts(path, &opts).context("Failed to initialize git repository")?;

    // Configure user identity
    let mut config = repo.config()?;
    config
        .set_str("user.email", "ltmatrix@agent")
        .context("Failed to set user.email")?;
    config
        .set_str("user.name", "Ltmatrix Agent")
        .context("Failed to set user.name")?;

    // Generate .gitignore
    generate_gitignore(path)?;

    tracing::info!("Initialized git repository in: {}", path.display());

    Ok(repo)
}

/// Generates a comprehensive .gitignore file in the specified directory.
///
/// This function creates or updates a .gitignore file with patterns for
/// common development tools, build artifacts, and IDE files.
///
/// # Arguments
///
/// * `path` - Path to the repository directory
///
/// # Returns
///
/// Returns `Result<()>` indicating success or failure.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::repository::generate_gitignore;
/// use std::path::Path;
///
/// generate_gitignore(Path::new("/path/to/project"))?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn generate_gitignore(path: &Path) -> Result<()> {
    let gitignore_path = path.join(".gitignore");

    let gitignore_content =
        "# ── Node / JS / TS ──────────────────────────────────────────────────────────
node_modules/
npm-debug.log*
yarn-debug.log*
yarn-error.log*
pnpm-debug.log*
.pnpm-store/
.npm/
.yarn/
.pnp.*
.next/
.nuxt/
.output/
.svelte-kit/
dist/
out/
build/
.cache/
.parcel-cache/
.turbo/
.vercel/
.netlify/
storybook-static/

# ── Python ───────────────────────────────────────────────────────────────────
__pycache__/
*.py[cod]
*$py.class
*.so
*.pyd
.Python
venv/
.venv/
env/
ENV/
pip-log.txt
pip-delete-this-directory.txt
*.egg-info/
dist-eggs/
eggs/
.eggs/
lib/
lib64/
parts/
sdist/
var/
wheels/
*.egg
MANIFEST
.pytest_cache/
.coverage
htmlcov/
.tox/
.hypothesis/
.mypy_cache/
.dmypy.json
dmypy.json
.pyre/
.pytype/
*.log

# ── Rust ────────────────────────────────────────────────────────────────────
/target/
**/*.rs.bk
*.pdb
Cargo.lock

# ── Go ──────────────────────────────────────────────────────────────────────
/bin/
/pkg/
*.exe
*.exe~
*.dll
*.so
*.dylib
*.test
*.out
go.work
vendor/

# ── Java ────────────────────────────────────────────────────────────────────
*.class
*.jar
*.war
*.ear
target/
.mvn/
mvnw
mvnw.cmd

# ── IDEs ────────────────────────────────────────────────────────────────────
.idea/
.vscode/
*.swp
*.swo
*~
.DS_Store
Thumbs.db
*.sublime-project
*.sublime-workspace
.history/

# ── Build tools ─────────────────────────────────────────────────────────────
.cmake/
CMakeCache.txt
CMakeFiles/
cmake_install.cmake
Makefile
*.o
*.a

# ── Logs ────────────────────────────────────────────────────────────────────
*.log
logs/
npm-debug.log*
yarn-debug.log*
yarn-error.log*
pnpm-debug.log*

# ── Environment ─────────────────────────────────────────────────────────────
.env
.env.local
.env.*.local
.env.development.local
.env.test.local
.env.production.local

# ── Temporary files ─────────────────────────────────────────────────────────
*.tmp
*.temp
.cache/
*.bak
*.backup
";

    // Write the .gitignore file
    std::fs::write(&gitignore_path, gitignore_content)
        .context("Failed to write .gitignore file")?;

    tracing::debug!("Generated .gitignore in: {}", path.display());

    Ok(())
}

/// Checks out a branch in the repository.
///
/// This function creates a new branch if it doesn't exist and checks it out.
/// If the branch already exists, it simply switches to it.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `branch_name` - Name of the branch to checkout
///
/// # Returns
///
/// Returns `Result<Oid>` containing the commit ID at HEAD, or an error.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::repository::init_repo;
/// use ltmatrix::git::repository::checkout;
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// let head_commit = checkout(&repo, "feature-branch")?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn checkout(repo: &Repository, branch_name: &str) -> Result<Oid> {
    // Check if branch already exists
    let branch_exists = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .is_ok();

    if !branch_exists {
        // Create new branch at current HEAD
        let head = repo.head().context("Failed to get HEAD reference")?;
        let head_commit = head
            .peel_to_commit()
            .context("Failed to peel HEAD to commit")?;
        repo.branch(branch_name, &head_commit, false)
            .context("Failed to create branch")?;
    }

    // Get the target branch's commit
    let obj = repo
        .revparse_single(&format!("refs/heads/{}", branch_name))?
        .peel_to_commit()
        .context("Failed to peel to commit")?;

    let commit_id = obj.id();

    // First set HEAD to point to the branch
    repo.set_head(&format!("refs/heads/{}", branch_name))
        .context("Failed to set HEAD")?;

    // Then hard reset to clean both working directory and index
    repo.reset(obj.as_object(), git2::ResetType::Hard, None)
        .context("Failed to reset to branch")?;

    tracing::debug!("Checked out branch: {}", branch_name);

    Ok(commit_id)
}

/// Gets the current branch name of the repository.
///
/// # Arguments
///
/// * `repo` - The Git repository
///
/// # Returns
///
/// Returns `Result<String>` containing the branch name, or an error.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::repository::init_repo;
/// use ltmatrix::git::repository::get_current_branch;
/// use std::path::Path;
///
/// let repo = init_repo(Path::new("/path/to/project"))?;
/// let branch = get_current_branch(&repo)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_current_branch(repo: &Repository) -> Result<String> {
    let head = repo.head().context("Failed to get HEAD reference")?;

    let branch_name = head.shorthand().context("Failed to get branch name")?;

    Ok(branch_name.to_string())
}

/// Protects a nested repository by adding it to the parent's .gitignore.
///
/// This function ensures that when creating a new repository inside an existing
/// repository, the nested repository is added to the parent's .gitignore to
/// prevent tracking issues.
///
/// # Arguments
///
/// * `workspace_path` - Path to the nested workspace
/// * `parent_path` - Path to the parent directory
///
/// # Returns
///
/// Returns `Result<()>` indicating success or failure.
fn protect_nested_repo(workspace_path: &Path, parent_path: &Path) -> Result<()> {
    // Check if parent is a git repository
    let parent_repo = match Repository::open(parent_path) {
        Ok(repo) => repo,
        Err(_) => return Ok(()), // Parent is not a git repo, nothing to do
    };

    // Get the parent repository root
    let parent_workdir = parent_repo
        .workdir()
        .context("Failed to get parent repository workdir")?;

    // Calculate relative path from parent to workspace
    let rel_ws = workspace_path
        .strip_prefix(parent_workdir)
        .context("Workspace is not inside parent repository")?;

    let entry = format!("/{}/\n", rel_ws.display());
    let parent_gitignore = parent_workdir.join(".gitignore");

    // Read existing .gitignore or create empty string
    let existing = if parent_gitignore.exists() {
        std::fs::read_to_string(&parent_gitignore).context("Failed to read parent .gitignore")?
    } else {
        String::new()
    };

    // Add entry if not already present
    if !existing.contains(&entry.trim()) {
        let comment = "# ltmatrix workspace (nested git repo)\n";
        let mut content = existing;

        // Ensure newline at end
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }

        content.push_str(comment);
        content.push_str(&entry);

        std::fs::write(&parent_gitignore, content).context("Failed to write parent .gitignore")?;

        tracing::info!(
            "Added {} to parent .gitignore to hide nested repo",
            rel_ws.display()
        );
    }

    Ok(())
}

/// Creates a signature for Git commits with current timestamp.
///
/// # Arguments
///
/// * `name` - Author name
/// * `email` - Author email
///
/// # Returns
///
/// Returns a `Signature` for use in Git commits.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::git::repository::create_signature;
///
/// let sig = create_signature("Ltmatrix Agent", "ltmatrix@agent")?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn create_signature(name: &str, email: &str) -> Result<Signature<'static>> {
    // Use current time for the signature
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let time = Time::new(now as i64, 0);
    Ok(Signature::new(name, email, &time)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to create an initial commit for testing
    fn create_initial_commit(repo: &Repository) -> Result<Oid> {
        let sig = create_signature("Test", "test@example.com")?;

        // Write empty tree
        let tree_oid = repo.treebuilder(None)?.write()?;

        // Create commit
        let tree = repo.find_tree(tree_oid)?;
        let oid = repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

        Ok(oid)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use tempfile::TempDir;

        #[test]
        fn test_init_repo_creates_repository() {
            let temp_dir = TempDir::new().unwrap();
            let repo_path = temp_dir.path();

            let repo = init_repo(repo_path).unwrap();

            // Verify repository was created
            assert!(repo_path.join(".git").exists());
            assert!(repo_path.join(".gitignore").exists());

            // Verify git config
            let config = repo.config().unwrap();
            let email = config.get_string("user.email").unwrap();
            let name = config.get_string("user.name").unwrap();
            assert_eq!(email, "ltmatrix@agent");
            assert_eq!(name, "Ltmatrix Agent");
        }

        #[test]
        fn test_generate_gitignore_creates_file() {
            let temp_dir = TempDir::new().unwrap();
            let repo_path = temp_dir.path();

            generate_gitignore(repo_path).unwrap();

            let gitignore_path = repo_path.join(".gitignore");
            assert!(gitignore_path.exists());

            let content = std::fs::read_to_string(&gitignore_path).unwrap();

            // Verify some key patterns are present
            assert!(content.contains("node_modules/"));
            assert!(content.contains("__pycache__/"));
            assert!(content.contains("/target/"));
            assert!(content.contains(".env"));
        }

        #[test]
        fn test_checkout_creates_new_branch() {
            let temp_dir = TempDir::new().unwrap();
            let repo_path = temp_dir.path();

            let repo = init_repo(repo_path).unwrap();

            // Create an initial commit first
            create_initial_commit(&repo).unwrap();

            let _head_commit = checkout(&repo, "feature-branch").unwrap();

            // Verify we're on the new branch
            let branch = get_current_branch(&repo).unwrap();
            assert_eq!(branch, "feature-branch");
        }

        #[test]
        fn test_checkout_switches_existing_branch() {
            let temp_dir = TempDir::new().unwrap();
            let repo_path = temp_dir.path();

            let repo = init_repo(repo_path).unwrap();

            // Create an initial commit first
            create_initial_commit(&repo).unwrap();

            // Create a branch
            checkout(&repo, "existing-branch").unwrap();

            // Switch back to master (default branch)
            let _main_head = repo.head().unwrap();
            repo.set_head("refs/heads/master").unwrap();

            // Switch back to the existing branch
            checkout(&repo, "existing-branch").unwrap();

            let branch = get_current_branch(&repo).unwrap();
            assert_eq!(branch, "existing-branch");
        }

        #[test]
        fn test_get_current_branch() {
            let temp_dir = TempDir::new().unwrap();
            let repo_path = temp_dir.path();

            let repo = init_repo(repo_path).unwrap();

            // Create an initial commit first
            create_initial_commit(&repo).unwrap();

            // Default branch should be "master" (git2 default)
            let branch = get_current_branch(&repo).unwrap();
            assert_eq!(branch, "master");

            // Create and checkout a new branch
            checkout(&repo, "test-branch").unwrap();
            let branch = get_current_branch(&repo).unwrap();
            assert_eq!(branch, "test-branch");
        }

        #[test]
        fn test_create_signature() {
            let sig = create_signature("Test Author", "test@example.com").unwrap();

            assert_eq!(sig.name(), Some("Test Author"));
            assert_eq!(sig.email(), Some("test@example.com"));
        }

        #[test]
        fn test_protect_nested_repo() {
            // Create parent repository
            let parent_dir = TempDir::new().unwrap();
            let _parent_repo = Repository::init(parent_dir.path()).unwrap();

            // Create child workspace inside parent
            let workspace_path = parent_dir.path().join("workspace");

            protect_nested_repo(&workspace_path, parent_dir.path()).unwrap();

            // Verify parent .gitignore was updated
            let parent_gitignore = parent_dir.path().join(".gitignore");
            assert!(parent_gitignore.exists());

            let content = std::fs::read_to_string(&parent_gitignore).unwrap();
            assert!(content.contains("/workspace/"));
        }

        #[test]
        fn test_generate_gitignore_idempotent() {
            let temp_dir = TempDir::new().unwrap();
            let repo_path = temp_dir.path();

            // Call generate_gitignore twice
            generate_gitignore(repo_path).unwrap();
            let first_content = std::fs::read_to_string(repo_path.join(".gitignore")).unwrap();

            generate_gitignore(repo_path).unwrap();
            let second_content = std::fs::read_to_string(repo_path.join(".gitignore")).unwrap();

            // Content should be identical
            assert_eq!(first_content, second_content);
        }
    }
}
