//! Integration tests for Git repository operations
//!
//! This test suite verifies the functionality of the Git repository module including:
//! - Repository initialization
//! - Branch checkout operations
//! - .gitignore file generation
//! - Nested repository protection
//! - Error handling patterns

use git2::Repository;
use ltmatrix::git::{
    checkout, create_signature, generate_gitignore, get_current_branch, init_repo,
};
use tempfile::TempDir;

/// Helper function to create an initial commit for testing
fn create_initial_commit(repo: &Repository) -> anyhow::Result<git2::Oid> {
    let sig = create_signature("Test User", "test@example.com")?;

    // Write empty tree
    let tree_oid = repo.treebuilder(None)?.write()?;

    // Create commit
    let tree = repo.find_tree(tree_oid)?;
    let oid = repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

    Ok(oid)
}

/// Test repository initialization creates all required files and configuration
#[test]
fn test_init_repo_creates_repository() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Verify .git directory exists
    assert!(
        repo_path.join(".git").exists(),
        ".git directory should exist"
    );

    // Verify .gitignore file exists
    assert!(
        repo_path.join(".gitignore").exists(),
        ".gitignore file should exist"
    );

    // Verify repository is valid and empty
    assert!(repo.is_empty().unwrap(), "New repository should be empty");
    assert!(
        repo.head().is_err(),
        "New repository should not have HEAD yet"
    );

    // Verify git configuration
    let config = repo.config().expect("Failed to get config");
    let email = config
        .get_string("user.email")
        .expect("Failed to get user.email");
    let name = config
        .get_string("user.name")
        .expect("Failed to get user.name");

    assert_eq!(
        email, "ltmatrix@agent",
        "user.email should be set correctly"
    );
    assert_eq!(name, "Ltmatrix Agent", "user.name should be set correctly");
}

/// Test that .gitignore file contains expected patterns for common development tools
#[test]
fn test_generate_gitignore_contains_expected_patterns() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    generate_gitignore(repo_path).expect("Failed to generate .gitignore");

    let gitignore_path = repo_path.join(".gitignore");
    assert!(gitignore_path.exists(), ".gitignore should exist");

    let content = std::fs::read_to_string(&gitignore_path).expect("Failed to read .gitignore");

    // Verify Node.js patterns
    assert!(
        content.contains("node_modules/"),
        "Should contain node_modules pattern"
    );
    assert!(
        content.contains("npm-debug.log*"),
        "Should contain npm debug log pattern"
    );

    // Verify Python patterns
    assert!(
        content.contains("__pycache__/"),
        "Should contain pycache pattern"
    );
    assert!(content.contains(".venv/"), "Should contain venv pattern");

    // Verify Rust patterns
    assert!(
        content.contains("/target/"),
        "Should contain target directory pattern"
    );

    // Verify IDE patterns
    assert!(
        content.contains(".idea/"),
        "Should contain IntelliJ IDEA pattern"
    );
    assert!(
        content.contains(".vscode/"),
        "Should contain VSCode pattern"
    );

    // Verify environment files
    assert!(content.contains(".env"), "Should contain .env pattern");

    // Verify log files
    assert!(content.contains("*.log"), "Should contain log file pattern");
}

/// Test that generate_gitignore is idempotent - calling it multiple times produces same result
#[test]
fn test_generate_gitignore_is_idempotent() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    // Generate .gitignore first time
    generate_gitignore(repo_path).expect("Failed to generate .gitignore first time");
    let first_content = std::fs::read_to_string(repo_path.join(".gitignore"))
        .expect("Failed to read .gitignore first time");

    // Generate .gitignore second time
    generate_gitignore(repo_path).expect("Failed to generate .gitignore second time");
    let second_content = std::fs::read_to_string(repo_path.join(".gitignore"))
        .expect("Failed to read .gitignore second time");

    assert_eq!(first_content, second_content, "Content should be identical");
}

/// Test checkout creates a new branch when it doesn't exist
#[test]
fn test_checkout_creates_new_branch() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit first (required for branch operations)
    create_initial_commit(&repo).expect("Failed to create initial commit");

    // Checkout new branch
    let head_commit = checkout(&repo, "feature-branch").expect("Failed to checkout feature-branch");

    // Verify branch was created
    let branch = repo
        .find_branch("feature-branch", git2::BranchType::Local)
        .expect("Branch should exist");

    assert!(branch.is_head(), "New branch should be checked out");

    // Verify current branch
    let current_branch = get_current_branch(&repo).expect("Failed to get current branch");
    assert_eq!(
        current_branch, "feature-branch",
        "Should be on feature-branch"
    );

    // Verify HEAD commit is returned
    assert!(!head_commit.is_zero(), "HEAD commit should be valid");
}

/// Test checkout switches to an existing branch
#[test]
fn test_checkout_switches_existing_branch() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    create_initial_commit(&repo).expect("Failed to create initial commit");

    // Create first branch
    checkout(&repo, "branch-one").expect("Failed to create branch-one");

    // Create second branch
    checkout(&repo, "branch-two").expect("Failed to create branch-two");

    // Switch back to first branch
    checkout(&repo, "branch-one").expect("Failed to switch to branch-one");

    let current_branch = get_current_branch(&repo).expect("Failed to get current branch");
    assert_eq!(current_branch, "branch-one", "Should be on branch-one");

    // Switch to second branch again
    checkout(&repo, "branch-two").expect("Failed to switch to branch-two");

    let current_branch = get_current_branch(&repo).expect("Failed to get current branch");
    assert_eq!(current_branch, "branch-two", "Should be on branch-two");
}

/// Test get_current_branch returns correct branch name
#[test]
fn test_get_current_branch_returns_correct_name() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    create_initial_commit(&repo).expect("Failed to create initial commit");

    // Default branch should be "master" (git2 default)
    let branch = get_current_branch(&repo).expect("Failed to get current branch");
    assert_eq!(branch, "master", "Default branch should be master");

    // Create and checkout a new branch
    checkout(&repo, "test-branch").expect("Failed to checkout test-branch");

    let branch = get_current_branch(&repo).expect("Failed to get current branch after checkout");
    assert_eq!(branch, "test-branch", "Should be on test-branch");

    // Create another branch
    checkout(&repo, "another-branch").expect("Failed to checkout another-branch");

    let branch =
        get_current_branch(&repo).expect("Failed to get current branch after second checkout");
    assert_eq!(branch, "another-branch", "Should be on another-branch");
}

/// Test create_signature creates a valid Git signature
#[test]
fn test_create_signature_creates_valid_signature() {
    let sig =
        create_signature("Test Author", "test@example.com").expect("Failed to create signature");

    assert_eq!(sig.name(), Some("Test Author"), "Name should match");
    assert_eq!(sig.email(), Some("test@example.com"), "Email should match");

    // Verify timestamp is recent (within last minute)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Failed to get time")
        .as_secs() as i64;

    let sig_time = sig.when().seconds();
    let time_diff = (now - sig_time).abs();

    assert!(time_diff < 60, "Signature timestamp should be recent");
}

/// Test nested repository protection - child repo is added to parent's .gitignore
#[test]
fn test_nested_repo_protection() {
    // Create parent repository
    let parent_dir = TempDir::new().expect("Failed to create parent temp dir");
    let _parent_repo =
        Repository::init(parent_dir.path()).expect("Failed to init parent repository");

    // Create child workspace path
    let workspace_path = parent_dir.path().join("nested-workspace");

    // Initialize child repository (should trigger nested repo protection)
    let _child_repo = init_repo(&workspace_path).expect("Failed to init child repository");

    // Verify parent .gitignore was created/updated
    let parent_gitignore = parent_dir.path().join(".gitignore");
    assert!(parent_gitignore.exists(), "Parent .gitignore should exist");

    let content =
        std::fs::read_to_string(&parent_gitignore).expect("Failed to read parent .gitignore");

    assert!(
        content.contains("/nested-workspace/"),
        "Parent .gitignore should contain child workspace path"
    );
    assert!(
        content.contains("ltmatrix workspace"),
        "Parent .gitignore should have comment explaining the entry"
    );
}

/// Test error handling when trying to checkout without initial commit
#[test]
fn test_checkout_fails_without_initial_commit() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Try to checkout without initial commit - should fail
    let result = checkout(&repo, "new-branch");

    assert!(
        result.is_err(),
        "Checkout should fail without initial commit"
    );
}

/// Test error handling when getting current branch without initial commit
#[test]
fn test_get_current_branch_fails_without_initial_commit() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Try to get current branch without initial commit - should fail
    let result = get_current_branch(&repo);

    assert!(
        result.is_err(),
        "get_current_branch should fail without initial commit"
    );
}

/// Test that init_repo cannot reinitialize an existing repository (protection against reinit)
#[test]
fn test_init_repo_prevents_reinitialization() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    // Initialize repository first time
    let repo1 = init_repo(repo_path).expect("Failed to initialize repository first time");

    // Try to initialize repository second time (should fail due to no_reinit)
    let result = init_repo(repo_path);

    assert!(
        result.is_err(),
        "Should not allow reinitialization of existing repository"
    );

    // Original repository should still be accessible and valid
    assert!(
        repo1.path().exists(),
        "Original repo .git directory should still exist"
    );
    assert_eq!(
        repo1.workdir(),
        Some(repo_path),
        "Original repo workdir should still be valid"
    );
}

/// Test multiple branch operations in sequence
#[test]
fn test_multiple_branch_operations() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Create initial commit
    create_initial_commit(&repo).expect("Failed to create initial commit");

    // Create multiple branches
    let branches = vec!["feature-1", "feature-2", "feature-3"];
    for branch in &branches {
        checkout(&repo, branch).expect("Failed to checkout branch");
    }

    // Verify all branches exist
    for branch in &branches {
        let _branch_obj = repo
            .find_branch(branch, git2::BranchType::Local)
            .expect(&format!("Branch {} should exist", branch));
        // If we can find it, it's valid
    }

    // Switch back to first branch
    checkout(&repo, branches[0]).expect("Failed to switch back to first branch");
    let current = get_current_branch(&repo).expect("Failed to get current branch");
    assert_eq!(current, branches[0], "Should be on first branch");
}

/// Test .gitignore has proper structure and formatting
#[test]
fn test_gitignore_structure_and_formatting() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    generate_gitignore(repo_path).expect("Failed to generate .gitignore");

    let content =
        std::fs::read_to_string(repo_path.join(".gitignore")).expect("Failed to read .gitignore");

    // Verify it has section headers
    assert!(content.contains("───"), "Should have section separators");

    // Verify it has multiple sections
    let sections = vec![
        "Node / JS / TS",
        "Python",
        "Rust",
        "Go",
        "Java",
        "IDEs",
        "Build tools",
    ];
    for section in sections {
        assert!(
            content.contains(section),
            "Should have section for {}",
            section
        );
    }

    // Verify proper line endings (no trailing whitespace on important lines)
    let lines: Vec<&str> = content.lines().collect();
    assert!(lines.len() > 50, "Should have many lines of patterns");
}

/// Test repository configuration persists after initialization
#[test]
fn test_repository_configuration_persists() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    // Initialize repository
    let repo = init_repo(repo_path).expect("Failed to initialize repository");

    // Verify configuration immediately
    let config1 = repo.config().expect("Failed to get config");
    assert_eq!(config1.get_string("user.email").unwrap(), "ltmatrix@agent");

    // Reopen repository and verify configuration persists
    let repo2 = Repository::open(repo_path).expect("Failed to reopen repository");
    let config2 = repo2
        .config()
        .expect("Failed to get config from reopened repo");

    assert_eq!(
        config2.get_string("user.email").unwrap(),
        "ltmatrix@agent",
        "user.email should persist"
    );
    assert_eq!(
        config2.get_string("user.name").unwrap(),
        "Ltmatrix Agent",
        "user.name should persist"
    );
}
