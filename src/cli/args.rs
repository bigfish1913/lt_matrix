//! CLI argument parsing
//!
//! This module contains comprehensive command-line argument definitions using clap.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// ltmatrix - A high-performance, cross-platform long-time agent orchestrator
#[derive(Debug, Clone, Parser)]
#[command(
    name = "ltmatrix",
    author = "bigfish <bigfish@example.com>",
    version = env!("CARGO_PKG_VERSION"),
    about = "Automate software development tasks using AI agents",
    long_about = "ltmatrix is a long-time agent orchestrator that automates software development tasks \
                  using AI agents like Claude. It breaks down goals into tasks, executes them with full \
                  testing and verification, and commits changes to git.",
    after_help = "EXAMPLES:
  # Run with standard mode
  ltmatrix \"build a REST API\"

  # Fast mode for quick iterations
  ltmatrix --fast \"add error handling\"

  # Expert mode for highest quality
  ltmatrix --expert \"implement authentication system\"

  # Resume interrupted work
  ltmatrix --resume

  # Generate plan without execution
  ltmatrix --dry-run \"refactor database layer\"

  # Use JSON output for parsing
  ltmatrix --output json \"write unit tests\"

  # Generate shell completions
  ltmatrix completions bash

  # Generate man pages
  ltmatrix man --output ./man

For more information, visit: https://github.com/bigfish/ltmatrix"
)]
pub struct Args {
    /// The goal/task to accomplish
    #[arg(value_name = "GOAL")]
    pub goal: Option<String>,

    /// Agent backend to use
    #[arg(long, value_name = "AGENT")]
    pub agent: Option<String>,

    /// Execution mode preset
    #[arg(long, value_name = "MODE")]
    pub mode: Option<ExecutionModeArg>,

    /// Fast execution mode
    #[arg(long, conflicts_with = "expert", conflicts_with = "mode")]
    pub fast: bool,

    /// Expert execution mode
    #[arg(long, conflicts_with = "fast", conflicts_with = "mode")]
    pub expert: bool,

    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Output format
    #[arg(long, value_name = "FORMAT")]
    pub output: Option<OutputFormat>,

    /// Log level
    #[arg(long, value_name = "LEVEL")]
    pub log_level: Option<LogLevel>,

    /// Log file path
    #[arg(long, value_name = "FILE")]
    pub log_file: Option<PathBuf>,

    /// Maximum number of retries per task
    #[arg(long, value_name = "NUM")]
    pub max_retries: Option<u32>,

    /// Timeout for operations (in seconds)
    #[arg(long, value_name = "SECONDS")]
    pub timeout: Option<u64>,

    /// Generate plan without execution
    #[arg(long)]
    pub dry_run: bool,

    /// Resume interrupted work
    #[arg(long)]
    pub resume: bool,

    /// Ask for clarification before planning
    #[arg(long)]
    pub ask: bool,

    /// Regenerate the plan
    #[arg(long)]
    pub regenerate_plan: bool,

    /// Strategy for blocked tasks
    #[arg(long, value_name = "STRATEGY")]
    pub on_blocked: Option<BlockedStrategy>,

    /// MCP configuration file
    #[arg(long, value_name = "FILE")]
    pub mcp_config: Option<PathBuf>,

    /// Disable colored output (respects NO_COLOR env var)
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Enable anonymous usage telemetry (opt-in)
    #[arg(long)]
    pub telemetry: bool,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Option<Command>,
}

impl Args {
    /// Returns the effective execution mode based on flags
    pub fn get_execution_mode(&self) -> ExecutionModeArg {
        if self.fast {
            ExecutionModeArg::Fast
        } else if self.expert {
            ExecutionModeArg::Expert
        } else if let Some(mode) = self.mode {
            mode
        } else {
            ExecutionModeArg::Standard
        }
    }

    /// Returns true if running in default mode (no subcommand)
    pub fn is_run_command(&self) -> bool {
        self.command.is_none()
    }
}

/// Subcommands available in ltmatrix
#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Create a release build for distribution
    #[command(name = "release")]
    Release(ReleaseArgs),

    /// Generate shell completion scripts
    #[command(name = "completions")]
    Completions(CompletionsArgs),

    /// Generate man pages
    #[command(name = "man")]
    Man(ManArgs),

    /// Clean up workspace state
    #[command(name = "cleanup")]
    Cleanup(CleanupArgs),

    /// Manage project memory
    #[command(name = "memory")]
    Memory(MemoryArgs),
}

/// Arguments for the 'release' subcommand
#[derive(Debug, Clone, Parser)]
pub struct ReleaseArgs {
    /// Build target triple (e.g., x86_64-unknown-linux-musl)
    #[arg(long, value_name = "TRIPLE")]
    pub target: Option<String>,

    /// Output directory for release artifacts
    #[arg(long, value_name = "DIR", default_value = "./dist")]
    pub output: PathBuf,

    /// Create release archives (tar.gz, zip)
    #[arg(long)]
    pub archive: bool,

    /// Build for all supported targets
    #[arg(long)]
    pub all_targets: bool,
}

/// Arguments for the 'completions' subcommand
#[derive(Debug, Clone, Parser)]
pub struct CompletionsArgs {
    /// Shell type to generate completions for
    #[arg(value_name = "SHELL")]
    pub shell: Shell,

    /// Print installation instructions for the specified shell
    #[arg(long)]
    pub install: bool,
}

/// Arguments for the 'man' subcommand
#[derive(Debug, Clone, Parser)]
pub struct ManArgs {
    /// Output directory for man pages
    #[arg(short, long, value_name = "DIR", default_value = "./man")]
    pub output: PathBuf,
}

/// Arguments for the 'cleanup' subcommand
#[derive(Debug, Clone, Parser)]
pub struct CleanupArgs {
    /// Reset all tasks to pending status (keeps state file)
    #[arg(long, conflicts_with = "reset_failed")]
    pub reset_all: bool,

    /// Reset only failed tasks to pending status (keeps state file)
    #[arg(long)]
    pub reset_failed: bool,

    /// Remove all workspace state files
    #[arg(long, conflicts_with = "reset_all", conflicts_with = "reset_failed")]
    pub remove: bool,

    /// Force cleanup without confirmation
    #[arg(long)]
    pub force: bool,

    /// Show what would be cleaned up without actually doing it
    #[arg(long)]
    pub dry_run: bool,
}

/// Arguments for the 'memory' subcommand
#[derive(Debug, Clone, Parser)]
pub struct MemoryArgs {
    /// The action to perform on memory
    #[command(subcommand)]
    pub action: MemoryAction,
}

/// Memory subcommands
#[derive(Debug, Clone, Subcommand)]
pub enum MemoryAction {
    /// Summarize memory entries to reduce file size
    #[command(name = "summarize")]
    Summarize(MemorySummarizeArgs),

    /// Show memory status and statistics
    #[command(name = "status")]
    Status(MemoryStatusArgs),

    /// Clear all memory entries
    #[command(name = "clear")]
    Clear(MemoryClearArgs),
}

/// Arguments for memory summarize subcommand
#[derive(Debug, Clone, Parser)]
pub struct MemorySummarizeArgs {
    /// Force summarization even if thresholds aren't exceeded
    #[arg(long)]
    pub force: bool,

    /// Keep only this fraction of entries (0.0 to 1.0)
    #[arg(long, value_name = "FRACTION")]
    pub keep_fraction: Option<f64>,

    /// Project root directory (defaults to current directory)
    #[arg(long, value_name = "DIR")]
    pub project: Option<PathBuf>,

    /// Dry run - show what would be summarized without making changes
    #[arg(long)]
    pub dry_run: bool,
}

/// Arguments for memory status subcommand
#[derive(Debug, Clone, Parser)]
pub struct MemoryStatusArgs {
    /// Project root directory (defaults to current directory)
    #[arg(long, value_name = "DIR")]
    pub project: Option<PathBuf>,

    /// Output in JSON format
    #[arg(long)]
    pub json: bool,
}

/// Arguments for memory clear subcommand
#[derive(Debug, Clone, Parser)]
pub struct MemoryClearArgs {
    /// Project root directory (defaults to current directory)
    #[arg(long, value_name = "DIR")]
    pub project: Option<PathBuf>,

    /// Force clear without confirmation
    #[arg(long)]
    pub force: bool,
}

/// Execution mode presets
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ExecutionModeArg {
    Fast,
    Standard,
    Expert,
}

impl std::fmt::Display for ExecutionModeArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionModeArg::Fast => write!(f, "fast"),
            ExecutionModeArg::Standard => write!(f, "standard"),
            ExecutionModeArg::Expert => write!(f, "expert"),
        }
    }
}

impl ExecutionModeArg {
    pub fn to_model(self) -> crate::models::ExecutionMode {
        match self {
            ExecutionModeArg::Fast => crate::models::ExecutionMode::Fast,
            ExecutionModeArg::Standard => crate::models::ExecutionMode::Standard,
            ExecutionModeArg::Expert => crate::models::ExecutionMode::Expert,
        }
    }
}

/// Output format options
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    JsonCompact,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Text => write!(f, "text"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::JsonCompact => write!(f, "json-compact"),
        }
    }
}

/// Log level options
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

/// Strategy for handling blocked tasks
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum BlockedStrategy {
    Skip,
    Ask,
    Abort,
    Retry,
}

impl std::fmt::Display for BlockedStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockedStrategy::Skip => write!(f, "skip"),
            BlockedStrategy::Ask => write!(f, "ask"),
            BlockedStrategy::Abort => write!(f, "abort"),
            BlockedStrategy::Retry => write!(f, "retry"),
        }
    }
}

/// Shell types for completion generation
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    #[value(name = "powershell")]
    PowerShell,
    Elvish,
}

impl std::fmt::Display for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Shell::Bash => write!(f, "bash"),
            Shell::Zsh => write!(f, "zsh"),
            Shell::Fish => write!(f, "fish"),
            Shell::PowerShell => write!(f, "powershell"),
            Shell::Elvish => write!(f, "elvish"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let args = Args::try_parse_from(["ltmatrix"]).unwrap();
        assert!(args.goal.is_none());
        assert!(!args.fast);
        assert!(!args.expert);
        assert_eq!(args.get_execution_mode(), ExecutionModeArg::Standard);
    }

    #[test]
    fn test_fast_mode() {
        let args = Args::try_parse_from(["ltmatrix", "--fast", "test"]).unwrap();
        assert_eq!(args.get_execution_mode(), ExecutionModeArg::Fast);
        assert!(args.goal.is_some());
    }

    #[test]
    fn test_expert_mode() {
        let args = Args::try_parse_from(["ltmatrix", "--expert", "test"]).unwrap();
        assert_eq!(args.get_execution_mode(), ExecutionModeArg::Expert);
        assert!(args.goal.is_some());
    }

    #[test]
    fn test_log_level_parsing() {
        let args = Args::try_parse_from(["ltmatrix", "--log-level", "debug", "test"]).unwrap();
        assert_eq!(args.log_level, Some(LogLevel::Debug));
    }

    #[test]
    fn test_output_format_parsing() {
        let args = Args::try_parse_from(["ltmatrix", "--output", "json", "test"]).unwrap();
        assert_eq!(args.output, Some(OutputFormat::Json));
    }

    #[test]
    fn test_dry_run_flag() {
        let args = Args::try_parse_from(["ltmatrix", "--dry-run", "test"]).unwrap();
        assert!(args.dry_run);
    }

    #[test]
    fn test_completions_subcommand() {
        let args = Args::try_parse_from(["ltmatrix", "completions", "bash"]).unwrap();
        assert!(matches!(args.command, Some(Command::Completions(..))));

        // Test with --install flag
        let args = Args::try_parse_from(["ltmatrix", "completions", "bash", "--install"]).unwrap();
        assert!(matches!(args.command, Some(Command::Completions(..))));
    }
}
