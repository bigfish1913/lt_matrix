//! ltmatrix - A high-performance, cross-platform long-time agent orchestrator
//!
//! This library provides the core functionality for the ltmatrix tool, which
//! automates software development tasks using AI agents.

// Core data models (must come first as other modules depend on them)
pub mod models;

pub mod cli;
pub mod config;
pub mod agent;
pub mod pipeline;
pub mod tasks;
pub mod git;
pub mod memory;
pub mod logging;
pub mod progress;

// Re-export commonly used types
pub use models::{
    Agent, ExecutionMode, ModeConfig, PipelineStage, Task, TaskComplexity, TaskStatus,
};

/// Main entry point for running ltmatrix
pub fn run(_args: cli::Args) -> anyhow::Result<()> {
    // TODO: Implement main run logic
    Ok(())
}
