//! ltmatrix - A high-performance, cross-platform long-time agent orchestrator
//!
//! This library provides the core functionality for the ltmatrix tool, which
//! automates software development tasks using AI agents.

// Core data models (must come first as other modules depend on them)
pub mod models;

pub mod agent;
pub mod cli;
pub mod completions;
pub mod config;
pub mod dryrun;
pub mod feature;
pub mod git;
pub mod interactive;
pub mod logging;
pub mod man;
pub mod mcp;
pub mod memory;
pub mod output;
pub mod pipeline;
pub mod progress;
pub mod release;
pub mod tasks;
pub mod telemetry;
pub mod terminal;
pub mod testing;
pub mod validate;
pub mod workspace;

// Re-export commonly used types
pub use models::{
    Agent, ExecutionMode, ModeConfig, PipelineStage, Task, TaskComplexity, TaskStatus,
};

/// Main entry point for running ltmatrix
pub fn run(args: cli::Args) -> anyhow::Result<()> {
    cli::command::execute_command(args)
}
