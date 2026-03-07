//! Pipeline execution stages
//!
//! This module implements the 6-stage pipeline: Generate, Assess, Execute, Test, Verify, Commit
//! Plus coverage analysis, fix cycle triggering, and code review.

pub mod assess;
pub mod commit;
pub mod coverage;
pub mod execute;
pub mod fix_cycle;
pub mod generate;
pub mod review;
pub mod stage;
pub mod test;
pub mod verify;
