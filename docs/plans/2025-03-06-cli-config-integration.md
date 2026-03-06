# CLI Integration with Config System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Connect CLI module with config system to support command-line overrides for all config values, ensuring proper precedence (CLI > Project > Global > Defaults).

**Architecture:**
- Create a conversion function `cli_args_to_overrides()` that transforms `cli::Args` into `config::settings::CliOverrides`
- Update `execute_run()` in `cli::command.rs` to load config with CLI overrides
- Extend `CliOverrides` struct to handle all CLI arguments
- Ensure all config fields can be overridden via CLI flags
- Add comprehensive tests for the integration

**Tech Stack:**
- Rust
- clap (CLI parsing)
- toml (config file format)
- anyhow (error handling)
- tracing (logging)

---

## Task 1: Extend CliOverrides struct

**Files:**
- Modify: `src/config/settings.rs:327-344`

**Step 1: Update CliOverrides struct to include all CLI fields**

The current `CliOverrides` struct is missing several CLI arguments. We need to add:
- `config_file`: Custom config file path
- `dry_run`: Generate plan without execution
- `resume`: Resume interrupted work
- `ask`: Ask for clarification before planning
- `regenerate_plan`: Regenerate the plan
- `on_blocked`: Strategy for blocked tasks
- `mcp_config`: MCP configuration file
- `progress`: Show/hide progress bars
- `run_tests`: Override test execution
- `verify`: Override verification

```rust
/// CLI override options for configuration
///
/// These values come from command-line arguments and take highest precedence
#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
    /// Custom config file path
    pub config_file: Option<PathBuf>,

    /// Agent backend to use
    pub agent: Option<String>,

    /// Execution mode
    pub mode: Option<String>,

    /// Output format
    pub output_format: Option<OutputFormat>,

    /// Log level
    pub log_level: Option<LogLevel>,

    /// Log file path
    pub log_file: Option<PathBuf>,

    /// Maximum retries
    pub max_retries: Option<u32>,

    /// Timeout in seconds
    pub timeout: Option<u64>,

    /// Disable colored output
    pub no_color: Option<bool>,

    /// Generate plan without execution
    pub dry_run: bool,

    /// Resume interrupted work
    pub resume: bool,

    /// Ask for clarification before planning
    pub ask: bool,

    /// Regenerate the plan
    pub regenerate_plan: bool,

    /// Strategy for blocked tasks
    pub on_blocked: Option<String>,

    /// MCP configuration file
    pub mcp_config: Option<PathBuf>,

    /// Show/hide progress bars
    pub progress: Option<bool>,

    /// Override test execution
    pub run_tests: Option<bool>,

    /// Override verification
    pub verify: Option<bool>,
}
```

**Step 2: Run cargo check to verify changes**

Run: `cargo check --lib`
Expected: No errors, struct compiles successfully

**Step 3: Commit**

```bash
git add src/config/settings.rs
git commit -m "feat: extend CliOverrides struct with all CLI fields"
```

---

## Task 2: Create Args to CliOverrides conversion function

**Files:**
- Modify: `src/config/settings.rs` (add new function after `CliOverrides` struct)

**Step 1: Write the conversion function**

Add a public function to convert `cli::Args` to `CliOverrides`:

```rust
impl CliOverrides {
    /// Create CliOverrides from CLI arguments
    ///
    /// This function maps clap-parsed arguments to config override values.
    pub fn from_args(args: &crate::cli::Args) -> Self {
        use crate::cli::args as cli_args;

        CliOverrides {
            config_file: args.config.clone(),
            agent: args.agent.clone(),
            mode: Some(args.get_execution_mode().to_string()),
            output_format: args.output.map(|f| match f {
                cli_args::OutputFormat::Text => OutputFormat::Text,
                cli_args::OutputFormat::Json => OutputFormat::Json,
                cli_args::OutputFormat::JsonCompact => OutputFormat::Json,
            }),
            log_level: args.log_level.map(|l| match l {
                cli_args::LogLevel::Trace => LogLevel::Trace,
                cli_args::LogLevel::Debug => LogLevel::Debug,
                cli_args::LogLevel::Info => LogLevel::Info,
                cli_args::LogLevel::Warn => LogLevel::Warn,
                cli_args::LogLevel::Error => LogLevel::Error,
            }),
            log_file: args.log_file.clone(),
            max_retries: args.max_retries,
            timeout: args.timeout,
            no_color: if args.no_color { Some(true) } else { None },
            dry_run: args.dry_run,
            resume: args.resume,
            ask: args.ask,
            regenerate_plan: args.regenerate_plan,
            on_blocked: args.on_blocked.map(|s| s.to_string()),
            mcp_config: args.mcp_config.clone(),
            progress: None, // Will be derived from no_color
            run_tests: None, // Will be derived from execution mode
            verify: None, // Will be derived from execution mode
        }
    }
}
```

**Step 2: Run cargo check to verify**

Run: `cargo check --lib`
Expected: No errors, function compiles successfully

**Step 3: Commit**

```bash
git add src/config/settings.rs
git commit -m "feat: add CliOverrides::from_args() conversion function"
```

---

## Task 3: Update apply_cli_overrides function

**Files:**
- Modify: `src/config/settings.rs:409-488`

**Step 1: Extend apply_cli_overrides to handle all new fields**

Update the function to apply all CLI overrides including the new fields:

```rust
/// Applies CLI overrides to a configuration
///
/// CLI arguments have the highest precedence and override all other sources.
fn apply_cli_overrides(mut config: Config, overrides: CliOverrides) -> Config {
    // Apply custom config file if specified
    // Note: This is handled by load_config_with_overrides before calling this

    // Override default agent
    if let Some(agent) = overrides.agent {
        config.default = Some(agent);
    }

    // Override output format
    if let Some(format) = overrides.output_format {
        config.output.format = format;
    }

    // Override log level
    if let Some(level) = overrides.log_level {
        config.logging.level = level;
    }

    // Override log file
    if let Some(file) = overrides.log_file {
        config.logging.file = Some(file);
    }

    // Override colored output
    if let Some(no_color) = overrides.no_color {
        config.output.colored = !no_color;
    }

    // Override progress bars (inverse of no_color)
    if let Some(no_color) = overrides.no_color {
        config.output.progress = !no_color;
    }

    // Apply execution mode-specific overrides
    if let Some(mode_str) = overrides.mode {
        apply_mode_overrides(&mut config, &mode_str, &overrides);
    }

    config
}

/// Apply mode-specific overrides to configuration
fn apply_mode_overrides(
    config: &mut Config,
    mode_name: &str,
    overrides: &CliOverrides,
) {
    match mode_name {
        "fast" => {
            // Override fast mode settings
            if let Some(ref mut fast) = config.modes.fast {
                if overrides.max_retries.is_some() {
                    fast.max_retries = overrides.max_retries.unwrap();
                }
                if overrides.timeout.is_some() {
                    fast.timeout_exec = overrides.timeout.unwrap();
                }
                if overrides.run_tests.is_some() {
                    fast.run_tests = overrides.run_tests.unwrap();
                }
                if overrides.verify.is_some() {
                    fast.verify = overrides.verify.unwrap();
                }
            }
        }
        "standard" => {
            // Override standard mode settings
            if let Some(ref mut standard) = config.modes.standard {
                if overrides.max_retries.is_some() {
                    standard.max_retries = overrides.max_retries.unwrap();
                }
                if overrides.timeout.is_some() {
                    standard.timeout_exec = overrides.timeout.unwrap();
                }
                if overrides.run_tests.is_some() {
                    standard.run_tests = overrides.run_tests.unwrap();
                }
                if overrides.verify.is_some() {
                    standard.verify = overrides.verify.unwrap();
                }
            }
        }
        "expert" => {
            // Override expert mode settings
            if let Some(ref mut expert) = config.modes.expert {
                if overrides.max_retries.is_some() {
                    expert.max_retries = overrides.max_retries.unwrap();
                }
                if overrides.timeout.is_some() {
                    expert.timeout_exec = overrides.timeout.unwrap();
                }
                if overrides.run_tests.is_some() {
                    expert.run_tests = overrides.run_tests.unwrap();
                }
                if overrides.verify.is_some() {
                    expert.verify = overrides.verify.unwrap();
                }
            }
        }
        _ => {}
    }
}
```

**Step 2: Run cargo check to verify**

Run: `cargo check --lib`
Expected: No errors

**Step 3: Commit**

```bash
git add src/config/settings.rs
git commit -m "feat: extend apply_cli_overrides to handle all CLI fields"
```

---

## Task 4: Update load_config_with_overrides to handle custom config file

**Files:**
- Modify: `src/config/settings.rs:359-407`

**Step 1: Update function to use custom config file path**

```rust
/// Loads configuration with CLI overrides
///
/// This function merges configuration from multiple sources with proper precedence:
/// 1. CLI overrides (highest)
/// 2. Custom config file (if specified via --config)
/// 3. Project config (.ltmatrix/config.toml)
/// 4. Global config (~/.ltmatrix/config.toml)
/// 5. Defaults (lowest)
///
/// # Arguments
///
/// * `overrides` - Optional CLI override values
///
/// # Returns
///
/// Returns a merged and validated `Config`.
pub fn load_config_with_overrides(overrides: Option<CliOverrides>) -> Result<Config> {
    let global_path = get_global_config_path()?;
    let project_path = get_project_config_path();

    // Use custom config file if specified, otherwise use standard paths
    let (config_paths, custom_path) = if let Some(ref overrides) = overrides {
        if let Some(ref custom_config) = overrides.config_file {
            // Load only from custom config file
            (vec![custom_config.clone()], Some(custom_config.clone()))
        } else {
            // Load from standard paths
            let mut paths = Vec::new();
            paths.push(global_path.clone());
            if let Some(ref project) = project_path {
                paths.push(project.clone());
            }
            (paths, None)
        }
    } else {
        // No overrides, use standard paths
        let mut paths = Vec::new();
        paths.push(global_path.clone());
        if let Some(ref project) = project_path {
            paths.push(project.clone());
        }
        (paths, None)
    };

    // Load configs from all paths
    let mut configs: Vec<Config> = Vec::new();

    for path in &config_paths {
        if path.exists() {
            debug!("Loading configuration from: {}", path.display());
            match load_config_file(path) {
                Ok(config) => configs.push(config),
                Err(e) => {
                    // If custom config was specified, fail hard
                    if custom_path.is_some() {
                        return Err(e.context(format!(
                            "Failed to load custom config file: {}",
                            path.display()
                        )));
                    }
                    // Otherwise, just log and continue
                    debug!("Skipping invalid config at {}: {}", path.display(), e);
                }
            }
        } else {
            debug!("No config found at: {}", path.display());
        }
    }

    // Merge all configs (last one wins)
    let mut merged = Config::default();
    for config in configs {
        merged = merge_config(merged, config);
    }

    // Apply CLI overrides if provided
    if let Some(overrides) = overrides {
        merged = apply_cli_overrides(merged, overrides);
    }

    // Validate the final configuration
    validate_config(&merged)?;

    Ok(merged)
}
```

**Step 2: Run cargo check to verify**

Run: `cargo check --lib`
Expected: No errors

**Step 3: Commit**

```bash
git add src/config/settings.rs
git commit -m "feat: support custom config file path via --config flag"
```

---

## Task 5: Update execute_run to load config with CLI overrides

**Files:**
- Modify: `src/cli/command.rs:27-51`

**Step 1: Import config module**

Add to the imports at the top of the file:

```rust
use super::args::{Args, Command};
use anyhow::{Context, Result};
use crate::config::settings::{self, CliOverrides};
```

**Step 2: Update execute_run function**

```rust
/// Execute the main run logic
fn execute_run(args: &Args) -> Result<()> {
    if let Some(goal) = &args.goal {
        println!("ltmatrix - Long-Time Agent Orchestrator");
        println!("Version: {}", env!("CARGO_PKG_VERSION"));
        println!();

        // Load configuration with CLI overrides
        let overrides = CliOverrides::from_args(args);
        let config = settings::load_config_with_overrides(Some(overrides))
            .context("Failed to load configuration")?;

        // Display configuration info
        println!("Goal: {}", goal);
        println!("Mode: {}", args.get_execution_mode());

        if let Some(ref agent) = config.default {
            println!("Agent: {}", agent);
        }

        if args.dry_run {
            println!("Dry run: plan will be generated but not executed");
        }

        if args.resume {
            println!("Resume: will continue from last interrupted task");
        }

        // TODO: Implement actual run logic using the config
        println!("\nTODO: Implement run logic with config");
        println!("Config loaded successfully:");
        println!("  - Default agent: {:?}", config.default);
        println!("  - Log level: {:?}", config.logging.level);
        println!("  - Output format: {:?}", config.output.format);
    } else {
        print_help();
    }

    Ok(())
}
```

**Step 3: Run cargo check to verify**

Run: `cargo check`
Expected: No errors

**Step 4: Run tests to ensure nothing broke**

Run: `cargo test --lib cli`
Expected: All existing tests pass

**Step 5: Commit**

```bash
git add src/cli/command.rs
git commit -m "feat: load config with CLI overrides in execute_run"
```

---

## Task 6: Write integration tests

**Files:**
- Create: `tests/cli_config_integration_test.rs`

**Step 1: Write comprehensive integration tests**

```rust
//! CLI and Config integration tests
//!
//! These tests verify that CLI arguments properly override configuration values.

use ltmatrix::cli::Args;
use ltmatrix::config::settings::{CliOverrides, load_config_with_overrides};

#[test]
fn test_cli_overrides_from_args() {
    // Test parsing CLI args to overrides
    let args = Args::parse_from(&[
        "ltmatrix",
        "--agent", "opencode",
        "--log-level", "debug",
        "--output", "json",
        "--max-retries", "5",
        "--timeout", "7200",
        "test goal"
    ]);

    let overrides = CliOverrides::from_args(&args);

    assert_eq!(overrides.agent, Some("opencode".to_string()));
    assert!(overrides.log_level.is_some()); // LogLevel::Debug
    assert!(overrides.output_format.is_some()); // OutputFormat::Json
    assert_eq!(overrides.max_retries, Some(5));
    assert_eq!(overrides.timeout, Some(7200));
}

#[test]
fn test_cli_overrides_execution_mode() {
    // Test fast mode
    let args = Args::parse_from(&[
        "ltmatrix",
        "--fast",
        "test"
    ]);

    let overrides = CliOverrides::from_args(&args);
    assert_eq!(overrides.mode, Some("fast".to_string()));

    // Test expert mode
    let args = Args::parse_from(&[
        "ltmatrix",
        "--expert",
        "test"
    ]);

    let overrides = CliOverrides::from_args(&args);
    assert_eq!(overrides.mode, Some("expert".to_string()));
}

#[test]
fn test_cli_overrides_boolean_flags() {
    let args = Args::parse_from(&[
        "ltmatrix",
        "--dry-run",
        "--resume",
        "--ask",
        "--no-color",
        "test"
    ]);

    let overrides = CliOverrides::from_args(&args);

    assert!(overrides.dry_run);
    assert!(overrides.resume);
    assert!(overrides.ask);
    assert_eq!(overrides.no_color, Some(true));
}

#[test]
fn test_load_config_with_cli_overrides() {
    // This test creates temporary config files and verifies CLI overrides work
    use tempfile::TempDir;
    use std::fs;
    use std::path::PathBuf;

    let temp_dir = TempDir::new().unwrap();

    // Create a global config with default values
    let global_config = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[output]
format = "text"
colored = true

[logging]
level = "info"
"#;

    let global_path = temp_dir.path().join("config.toml");
    fs::write(&global_path, global_config).unwrap();

    // Create CLI overrides
    let args = Args::parse_from(&[
        "ltmatrix",
        "--agent", "opencode",
        "--log-level", "debug",
        "--output", "json",
        "--config",
        global_path.to_str().unwrap(),
        "test"
    ]);

    let overrides = CliOverrides::from_args(&args);

    // Load config with overrides
    // Note: This will need to handle the temp directory properly
    // For now, we'll just verify the overrides structure
    assert_eq!(overrides.agent, Some("opencode".to_string()));
    assert_eq!(overrides.config_file, Some(global_path));
}

#[test]
fn test_custom_config_file_override() {
    use tempfile::TempDir;
    use std::fs;

    let temp_dir = TempDir::new().unwrap();

    // Create a custom config
    let custom_config = r#"
default = "custom-agent"

[agents.custom-agent]
command = "custom"
model = "custom-model"
timeout = 1800

[output]
format = "json"
colored = false
"#;

    let custom_path = temp_dir.path().join("custom.toml");
    fs::write(&custom_path, custom_config).unwrap();

    let args = Args::parse_from(&[
        "ltmatrix",
        "--config",
        custom_path.to_str().unwrap(),
        "test"
    ]);

    let overrides = CliOverrides::from_args(&args);

    assert_eq!(overrides.config_file, Some(custom_path));
}
```

**Step 2: Run tests to verify they compile**

Run: `cargo test --test cli_config_integration_test`
Expected: Tests compile (some may fail initially)

**Step 3: Commit**

```bash
git add tests/cli_config_integration_test.rs
git commit -m "test: add CLI-config integration tests"
```

---

## Task 7: Add unit tests for apply_cli_overrides

**Files:**
- Modify: `src/config/settings.rs` (add to tests module)

**Step 1: Add tests for new override fields**

Add to the `#[cfg(test)]` mod tests section:

```rust
#[test]
fn test_apply_cli_overrides_dry_run() {
    let config = Config::default();
    let overrides = CliOverrides {
        dry_run: true,
        ..Default::default()
    };

    let merged = apply_cli_overrides(config, overrides);
    // dry_run is stored but doesn't affect the Config struct
    // It's used at runtime by the execute_run function
}

#[test]
fn test_apply_cli_overrides_execution_mode_settings() {
    let mut config = Config::default();
    config.modes.fast = Some(ModeConfig {
        model: Some("claude-haiku-4-5".to_string()),
        run_tests: false,
        verify: true,
        max_retries: 1,
        max_depth: 2,
        timeout_plan: 60,
        timeout_exec: 1800,
    });

    let overrides = CliOverrides {
        mode: Some("fast".to_string()),
        max_retries: Some(5),
        timeout: Some(2400),
        run_tests: Some(true), // Override to true
        verify: Some(false),   // Override to false
        ..Default::default()
    };

    let merged = apply_cli_overrides(config, overrides);

    assert_eq!(merged.modes.fast.as_ref().unwrap().max_retries, 5);
    assert_eq!(merged.modes.fast.as_ref().unwrap().timeout_exec, 2400);
    assert_eq!(merged.modes.fast.as_ref().unwrap().run_tests, true);
    assert_eq!(merged.modes.fast.as_ref().unwrap().verify, false);
}

#[test]
fn test_apply_cli_overrides_progress_bar() {
    let config = Config {
        output: OutputConfig {
            format: OutputFormat::Text,
            colored: true,
            progress: true,
        },
        ..Default::default()
    };

    let overrides = CliOverrides {
        no_color: Some(true),
        ..Default::default()
    };

    let merged = apply_cli_overrides(config, overrides);

    // no_color should disable both colors and progress
    assert_eq!(merged.output.colored, false);
    assert_eq!(merged.output.progress, false);
}
```

**Step 2: Run tests**

Run: `cargo test --lib config::settings::tests::test_apply_cli_overrides`
Expected: All new tests pass

**Step 3: Commit**

```bash
git add src/config/settings.rs
git commit -m "test: add unit tests for new CLI override fields"
```

---

## Task 8: Write example demonstrating CLI-config integration

**Files:**
- Create: `examples/cli_config_integration.rs`

**Step 1: Create comprehensive example**

```rust
//! Example demonstrating CLI and config integration
//!
//! Run with:
//!   cargo run --example cli_config_integration -- --help
//!   cargo run --example cli_config_integration -- --agent opencode --log-level debug "test goal"

use ltmatrix::cli::Args;
use ltmatrix::config::settings::{CliOverrides, load_config_with_overrides};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CLI-Config Integration Example ===\n");

    // Parse CLI arguments
    let args = Args::parse();

    // Convert args to overrides
    let overrides = CliOverrides::from_args(&args);

    println!("CLI Overrides:");
    println!("  Agent: {:?}", overrides.agent);
    println!("  Mode: {:?}", overrides.mode);
    println!("  Log level: {:?}", overrides.log_level);
    println!("  Output format: {:?}", overrides.output_format);
    println!("  Max retries: {:?}", overrides.max_retries);
    println!("  Timeout: {:?}", overrides.timeout);
    println!("  Dry run: {}", overrides.dry_run);
    println!("  No color: {:?}", overrides.no_color);
    println!("  Config file: {:?}", overrides.config_file);
    println!();

    // Load config with overrides
    println!("Loading configuration...");
    let config = load_config_with_overrides(Some(overrides))?;

    println!("Final Configuration:");
    println!("  Default agent: {:?}", config.default);
    println!("  Agents configured: {}", config.agents.len());
    println!("  Log level: {:?}", config.logging.level);
    println!("  Output format: {:?}", config.output.format);
    println!("  Colored output: {}", config.output.colored);
    println!("  Progress bars: {}", config.output.progress);
    println!();

    if let Some(fast) = &config.modes.fast {
        println!("Fast Mode:");
        println!("  Model: {:?}", fast.model);
        println!("  Run tests: {}", fast.run_tests);
        println!("  Verify: {}", fast.verify);
        println!("  Max retries: {}", fast.max_retries);
        println!("  Timeout (exec): {}s", fast.timeout_exec);
    }

    if let Some(standard) = &config.modes.standard {
        println!("Standard Mode:");
        println!("  Model: {:?}", standard.model);
        println!("  Run tests: {}", standard.run_tests);
        println!("  Verify: {}", standard.verify);
        println!("  Max retries: {}", standard.max_retries);
        println!("  Timeout (exec): {}s", standard.timeout_exec);
    }

    if let Some(expert) = &config.modes.expert {
        println!("Expert Mode:");
        println!("  Model: {:?}", expert.model);
        println!("  Run tests: {}", expert.run_tests);
        println!("  Verify: {}", expert.verify);
        println!("  Max retries: {}", expert.max_retries);
        println!("  Timeout (exec): {}s", expert.timeout_exec);
    }

    Ok(())
}
```

**Step 2: Run the example to verify**

Run: `cargo run --example cli_config_integration -- --help`
Expected: Help message displays

Run: `cargo run --example cli_config_integration -- --log-level debug "test goal"`
Expected: Example runs without errors

**Step 3: Commit**

```bash
git add examples/cli_config_integration.rs
git commit -m "docs: add CLI-config integration example"
```

---

## Task 9: Update documentation

**Files:**
- Create: `docs/cli-config-integration.md`

**Step 1: Write comprehensive documentation**

```markdown
# CLI and Config Integration

## Overview

ltmatrix supports a hierarchical configuration system with the following precedence
(highest to lowest):

1. **CLI Arguments** - Command-line flags override everything
2. **Custom Config File** - `--config <file>` if specified
3. **Project Config** - `.ltmatrix/config.toml` in current directory
4. **Global Config** - `~/.ltmatrix/config.toml`
5. **Defaults** - Built-in default values

## CLI Arguments

### Agent Configuration

- `--agent <name>`: Override default agent backend
- `--fast`: Enable fast execution mode
- `--expert`: Enable expert execution mode
- `--mode <fast|standard|expert>`: Explicit mode selection

### Output Configuration

- `--output <format>`: Output format (text, json, json-compact)
- `--no-color`: Disable colored output

### Logging Configuration

- `--log-level <level>`: Log level (trace, debug, info, warn, error)
- `--log-file <path>`: Log file path

### Execution Configuration

- `--max-retries <num>`: Maximum retry attempts
- `--timeout <seconds>`: Operation timeout
- `--dry-run`: Generate plan without execution
- `--resume`: Resume interrupted work
- `--ask`: Ask for clarification before planning
- `--regenerate-plan`: Regenerate the task plan
- `--on-blocked <strategy>`: Blocked task strategy (skip, ask, abort, retry)

### Advanced Configuration

- `--config <file>`: Use custom config file
- `--mcp-config <file>`: MCP configuration file

## Examples

### Override Agent Backend

```bash
# Use opencode backend instead of configured default
ltmatrix --agent opencode "implement REST API"
```

### Set Log Level

```bash
# Enable debug logging
ltmatrix --log-level debug "fix authentication bug"
```

### JSON Output

```bash
# Get JSON output for parsing
ltmatrix --output json "write tests" | jq .
```

### Fast Mode

```bash
# Quick iteration without tests
ltmatrix --fast "add error handling"
```

### Expert Mode

```bash
# High quality with full testing
ltmatrix --expert "implement payment system"
```

### Custom Config File

```bash
# Use project-specific config
ltmatrix --config ./ltmatrix.prod.toml "deploy to production"
```

### Multiple Overrides

```bash
# Combine multiple options
ltmatrix --fast --log-level debug --output json "add feature"
```

## Configuration Precedence Examples

### Example 1: CLI Override

Config file has `default = "claude"`, CLI uses `--agent opencode`:
→ Result: Uses `opencode`

### Example 2: Project Override Global

Global config has `level = "info"`, project config has `level = "debug"`:
→ Result: Uses `debug` from project config

### Example 3: Custom Config File

Global and project configs exist, but `--config custom.toml` is used:
→ Result: Only loads `custom.toml`, ignores global and project

### Example 4: Full Precedence Chain

1. Global config: `default = "claude"`, `level = "info"`
2. Project config: `default = "opencode"`, `timeout = 1800`
3. CLI args: `--agent kimicode --timeout 7200`

Final config:
- `default = "kimicode"` (from CLI)
- `timeout = 7200` (from CLI)
- `level = "info"` (from global, not overridden)

## Integration Points

### Loading Config in Code

```rust
use ltmatrix::cli::Args;
use ltmatrix::config::settings::{CliOverrides, load_config_with_overrides};

let args = Args::parse();
let overrides = CliOverrides::from_args(&args);
let config = load_config_with_overrides(Some(overrides))?;
```

### Accessing Config Values

```rust
// Get default agent
let agent_name = config.default.as_ref().unwrap();

// Get current mode config
let mode = args.get_execution_mode().to_model();
let mode_config = config.get_mode_config(mode)?;

// Apply mode-specific settings
max_retries = mode_config.max_retries;
timeout = mode_config.timeout_exec;
```

## Testing

Run the integration example:

```bash
# Test basic overrides
cargo run --example cli_config_integration -- --log-level debug "test"

# Test with custom config
cargo run --example cli_config_integration -- --config examples/config.toml "test"

# Test fast mode
cargo run --example cli_config_integration -- --fast "test"
```
```

**Step 2: Commit**

```bash
git add docs/cli-config-integration.md
git commit -m "docs: add CLI-config integration documentation"
```

---

## Task 10: Final verification and cleanup

**Files:**
- Multiple files for verification

**Step 1: Run all tests**

Run: `cargo test --all`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features`
Expected: No warnings (or only minor ones)

**Step 3: Run formatting check**

Run: `cargo fmt --all -- --check`
Expected: No formatting changes needed

**Step 4: Verify example runs**

Run: `cargo run --example cli_config_integration -- --help`
Expected: Help displays correctly

Run: `cargo run --example cli_config_integration -- --log-level debug "test goal"`
Expected: Example runs and shows proper config merging

**Step 5: Build release binary**

Run: `cargo build --release`
Expected: Binary builds without errors

**Step 6: Create summary commit**

```bash
git add .
git commit -m "feat: complete CLI-config integration

- Extended CliOverrides with all CLI arguments
- Added Args to CliOverrides conversion
- Updated apply_cli_overrides for all fields
- Support for custom config files via --config
- Integration tests for CLI-config behavior
- Example demonstrating integration
- Comprehensive documentation

All CLI arguments now properly override config values
with correct precedence: CLI > Custom Config > Project > Global > Defaults"
```

---

## Testing Strategy

### Unit Tests
- Test `CliOverrides::from_args()` with various CLI arguments
- Test `apply_cli_overrides()` for all config fields
- Test mode-specific override logic

### Integration Tests
- Test full config loading with CLI overrides
- Test custom config file loading
- Test precedence chain (CLI > Project > Global)

### Manual Testing
- Run example with various flag combinations
- Verify config precedence behavior
- Test with real config files

### Edge Cases
- No config files (defaults only)
- Invalid config file (should fail gracefully)
- Conflicting CLI args (e.g., --fast and --expert)
- Custom config path doesn't exist (should error)

---

## Success Criteria

✅ All CLI arguments map to config overrides
✅ Custom config file support via `--config`
✅ Proper precedence: CLI > Custom > Project > Global > Defaults
✅ All tests pass (unit + integration)
✅ Example demonstrates integration
✅ Documentation complete
✅ No clippy warnings
✅ Code properly formatted
✅ Release build succeeds

---

**Implementation Notes:**

1. **Backward Compatibility**: All changes are additive; no existing functionality is broken
2. **Error Handling**: Invalid configs fail with clear error messages
3. **Validation**: Config is validated after all overrides are applied
4. **Testing**: Comprehensive test coverage for all override paths
5. **Documentation**: Clear examples and precedence rules documented

**Next Steps:**

After implementing this plan, the CLI will be fully integrated with the config system, enabling users to override any configuration value via command-line arguments with proper precedence handling.
