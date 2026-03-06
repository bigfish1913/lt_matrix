//! CLI argument parsing
//!
//! This module contains the command-line argument definitions using clap.

use clap::Parser;

/// Command-line arguments for ltmatrix
#[derive(Debug, Clone, Parser)]
pub struct Args {
    /// The goal/task to accomplish
    #[arg(value_name = "GOAL")]
    pub goal: Option<String>,

    /// Execution mode to use
    #[arg(long, value_name = "MODE")]
    pub mode: Option<String>,

    /// Agent backend to use
    #[arg(long, value_name = "AGENT")]
    pub agent: Option<String>,

    /// Resume interrupted work
    #[arg(long)]
    pub resume: bool,

    /// Generate plan without execution
    #[arg(long)]
    pub dry_run: bool,

    /// Ask for clarification before planning
    #[arg(long)]
    pub ask: bool,

    /// Regenerate the plan
    #[arg(long)]
    pub regenerate_plan: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, value_name = "LEVEL")]
    pub log_level: Option<String>,

    /// Log file path
    #[arg(long, value_name = "FILE")]
    pub log_file: Option<String>,

    /// Output format (text, json)
    #[arg(long, value_name = "FORMAT")]
    pub output: Option<String>,

    /// Strategy for blocked tasks
    #[arg(long, value_name = "STRATEGY")]
    pub on_blocked: Option<String>,

    /// MCP configuration file
    #[arg(long, value_name = "FILE")]
    pub mcp_config: Option<String>,
}
