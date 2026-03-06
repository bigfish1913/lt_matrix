//! Configuration management
//!
//! This module handles loading, parsing, and managing configuration from
//! TOML files and command-line arguments.
//!
//! # Configuration Precedence
//!
//! Configuration is merged from multiple sources with the following precedence
//! (highest to lowest):
//!
//! 1. **CLI Arguments** - Command-line flags override everything
//! 2. **Project Config** - `.ltmatrix/config.toml` in current directory
//! 3. **Global Config** - `~/.ltmatrix/config.toml`
//! 4. **Defaults** - Hard-coded default values
//!
//! # Usage Examples
//!
//! ## Load config with CLI overrides
//!
//! ```no_run
//! use ltmatrix::cli::Args;
//! use ltmatrix::config::settings::load_config_from_args;
//! use clap::Parser;
//!
//! let args = Args::parse_from(["ltmatrix", "--agent", "claude", "goal"]);
//! let config = load_config_from_args(args)?;
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! ## Load config from files only
//!
//! ```no_run
//! use ltmatrix::config::settings::load_config;
//!
//! let config = load_config()?;
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # CLI Override Mapping
//!
//! The following CLI arguments map to config fields:
//!
//! | CLI Argument | Config Field | Type |
//! |--------------|--------------|------|
//! | `--agent <NAME>` | `default` | String |
//! | `--mode {fast\|standard\|expert}` | Mode settings | String |
//! | `--fast` | Equivalent to `--mode fast` | Flag |
//! | `--expert` | Equivalent to `--mode expert` | Flag |
//! | `--output {text\|json}` | `output.format` | OutputFormat |
//! | `--log-level <LEVEL>` | `logging.level` | LogLevel |
//! | `--log-file <PATH>` | `logging.file` | PathBuf |
//! | `--max-retries <NUM>` | Mode max_retries | u32 |
//! | `--timeout <SECONDS>` | Mode timeout | u64 |
//! | `--mcp-config <PATH>` | `mcp` | LoadedMcpConfig |
//! | `--no-color` | `output.colored` | bool |
//!
//! # Config File Format
//!
//! Example `.ltmatrix/config.toml`:
//!
//! ```toml
//! default = "claude"
//!
//! [agents.claude]
//! command = "claude"
//! model = "claude-sonnet-4-6"
//! timeout = 3600
//!
//! [modes.fast]
//! model = "claude-haiku-4-5"
//! run_tests = false
//! verify = true
//! max_retries = 1
//!
//! [output]
//! format = "text"
//! colored = true
//!
//! [logging]
//! level = "info"
//! file = "/tmp/ltmatrix.log"
//! ```
//!
//! # MCP Configuration
//!
//! MCP (Model Context Protocol) servers can be configured via a separate
//! config file specified with `--mcp-config`:
//!
//! ```bash
//! ltmatrix --mcp-config mcp-servers.toml "test my application"
//! ```
//!
//! Example MCP config file:
//!
//! ```toml
//! [mcp.servers.playwright]
//! type = "playwright"
//! command = "npx"
//! args = ["-y", "@playwright/mcp@latest"]
//! timeout = 60
//!
//! [mcp.servers.browser]
//! type = "browser"
//! command = "mcp-server-browser"
//! timeout = 30
//! ```

pub mod agent;
pub mod mcp;
pub mod modes;
pub mod settings;
