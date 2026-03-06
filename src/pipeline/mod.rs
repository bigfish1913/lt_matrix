//! Pipeline execution stages
//!
//! This module implements the 6-stage pipeline: Generate → Assess → Execute → Test → Verify → Commit

pub mod stage;
pub mod generate;
pub mod assess;
pub mod execute;
pub mod test;
pub mod verify;
pub mod commit;

pub use stage::{Stage, StageResult};
pub use generate::GenerateStage;
pub use assess::AssessStage;
pub use execute::ExecuteStage;
pub use test::TestStage;
pub use verify::VerifyStage;
pub use commit::CommitStage;
