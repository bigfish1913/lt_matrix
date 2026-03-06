//! Git integration
//!
//! This module provides Git operations including initialization, branching, committing, and merging.

pub mod repository;
pub mod branch;
pub mod commit;
pub mod merge;

pub use repository::Repository;
pub use branch::{Branch, BranchError};
pub use commit::{Commit, CommitError};
pub use merge::{MergeStrategy, MergeError};
