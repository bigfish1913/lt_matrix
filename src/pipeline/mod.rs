//! Pipeline execution stages
//!
//! This module implements the 6-stage pipeline: Generate, Assess, Execute, Test, Verify, Commit

pub mod stage;
pub mod generate;
pub mod assess;
pub mod execute;
pub mod test;
pub mod verify;
pub mod commit;

#[cfg(test)]
mod assess_tests;
