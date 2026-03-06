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

pub mod repository;
pub mod branch;
pub mod commit;
pub mod merge;

// Re-export commonly used functions
pub use repository::{
    init_repo,
    checkout,
    generate_gitignore,
    get_current_branch,
    create_signature,
};
