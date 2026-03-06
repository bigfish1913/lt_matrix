//! Pipeline execution stages
//!
//! This module implements the 6-stage pipeline: Generate, Assess, Execute, Test, Verify, Commit

pub mod assess;
pub mod commit;
pub mod execute;
pub mod generate;
pub mod stage;
pub mod test;
pub mod verify;
