//! ltmatrix - A high-performance, cross-platform long-time agent orchestrator
//!
//! This library provides the core functionality for the ltmatrix tool, which
//! automates software development tasks using AI agents.

pub mod cli;
pub mod config;
pub mod agent;
pub mod pipeline;
pub mod tasks;
pub mod git;
pub mod memory;
pub mod logging;
pub mod progress;
