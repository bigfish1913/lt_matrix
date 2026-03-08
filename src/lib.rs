// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! ltmatrix - A high-performance, cross-platform long-time agent orchestrator
//!
//! This library provides the core functionality for the ltmatrix tool, which
//! automates software development tasks using AI agents.

// Re-export from sub-crates for backward compatibility
pub use ltmatrix_core as models;
pub use ltmatrix_core::{
    Agent, ExecutionMode, ModeConfig, PipelineStage, Task, TaskComplexity, TaskStatus,
};
pub use ltmatrix_config as config;
pub use ltmatrix_config::feature;
pub use ltmatrix_config::telemetry;
pub use ltmatrix_agent as agent;
pub use ltmatrix_mcp as mcp;

// Modules that remain in the main crate
pub mod cli;
pub mod completions;
pub mod dryrun;
pub mod git;
pub mod guidelines;
pub mod interactive;
pub mod logging;
pub mod man;
pub mod memory;
pub mod output;
pub mod pipeline;
pub mod plugin;
pub mod progress;
pub mod release;
pub mod security;
pub mod tasks;
pub mod terminal;
pub mod testing;
pub mod validate;
pub mod workspace;

/// Main entry point for running ltmatrix
pub fn run(args: cli::Args) -> anyhow::Result<()> {
    cli::command::execute_command(args)
}
