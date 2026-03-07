// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Git integration
//!
//! This module provides Git operations including branching, committing, and merging
//!
//! # Examples
//!
//! ```no_run
//! use ltmatrix::git::{init_repo, checkout, generate_gitignore};
//! use std::path::Path;
//!
//! // Initialize a new repository
//! let repo = init_repo(Path::new("/path/to/project"))?;
//!
//! // Generate .gitignore
//! generate_gitignore(Path::new("/path/to/project"))?;
//!
//! // Checkout a branch
//! checkout(&repo, "feature-branch")?;
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod branch;
pub mod commit;
pub mod merge;
pub mod repository;

// Re-export commonly used functions
pub use repository::{
    checkout, create_signature, generate_gitignore, get_current_branch, init_repo,
};

// Re-export branch functions
pub use branch::{
    branch_exists, create_branch, delete_branch, get_current_branch_name, is_head_detached,
    list_branches, validate_branch_name,
};

// Re-export commit functions
pub use commit::{
    amend_commit, commit_changes, create_commit, get_head_commit, has_staged_changes,
    has_unstaged_changes, short_commit_id, stage_all, stage_files, validate_commit_message,
};

// Re-export merge functions
pub use merge::merge_with_squash;
