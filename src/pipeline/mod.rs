// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Pipeline execution stages
//!
//! This module implements the 6-stage pipeline: Generate, Assess, Execute, Test, Verify, Commit
//! Plus coverage analysis, fix cycle triggering, code review, and orchestration.

pub mod assess;
pub mod commit;
pub mod coverage;
pub mod execute;
pub mod fix_cycle;
pub mod generate;
pub mod memory;
pub mod orchestrator;
pub mod review;
pub mod stage;
pub mod test;
pub mod verify;

pub use orchestrator::{OrchestratorConfig, PipelineOrchestrator, PipelineResult};
