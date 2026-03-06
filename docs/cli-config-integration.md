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

### Resume Interrupted Work

```bash
# Continue from where you left off
ltmatrix --resume
```

### Dry Run

```bash
# Generate plan without executing
ltmatrix --dry-run "refactor database"
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
let overrides = CliOverrides::from(args.clone());
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

## Configuration File Format

### Global Config: `~/.ltmatrix/config.toml`

```toml
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[agents.opencode]
command = "opencode"
model = "gpt-4"
timeout = 1800

[output]
format = "text"
colored = true
progress = true

[logging]
level = "info"
```

### Project Config: `.ltmatrix/config.toml`

```toml
default = "opencode"

[agents.opencode]
timeout = 7200

[modes.fast]
model = "claude-haiku-4-5"
run_tests = false
verify = true
max_retries = 1
max_depth = 2

[output]
format = "json"
colored = false
```

## Feature Flags Integration

The feature flag system integrates seamlessly with CLI overrides:

```toml
[features.pipeline]
enable_parallel_execution = true
enable_smart_cache = true
```

CLI overrides:
```bash
ltmatrix --output json "goal"
```

Both systems work together:
1. Config file sets up feature flags
2. CLI args can override output format, log level, etc.
3. Feature flags control experimental functionality

## Error Handling

### Missing Config File

If `--config` points to a non-existent file:
```bash
ltmatrix --config /path/to/missing.toml "goal"
# Error: Failed to load custom config file: /path/to/missing.toml
```

### Invalid TOML

If config file has invalid TOML:
```bash
ltmatrix --config invalid.toml "goal"
# Error: Failed to load custom config file: invalid.toml
# Caused by: TOML parse error at line 5, column 10
```

### Missing Default Agent

If config references undefined agent:
```bash
ltmatrix --agent undefined-agent "goal"
# Error: Default agent 'undefined-agent' is not defined in configuration
# Available agents: claude, opencode
```

## Best Practices

### 1. Start with Defaults, Then Override

Begin with sensible defaults in global config:
```toml
# ~/.ltmatrix/config.toml
[output]
format = "text"
colored = true

[logging]
level = "info"
```

Then override per-project:
```toml
# .ltmatrix/config.toml
[output]
format = "json"
```

Or per-invocation:
```bash
ltmatrix --log-level debug "fix bug"
```

### 2. Use Custom Configs for Environments

Production:
```bash
ltmatrix --config prod.toml "deploy to production"
```

Development:
```bash
ltmatrix --config dev.toml "add feature"
```

Testing:
```bash
ltmatrix --config test.toml --dry-run "verify fix"
```

### 3. Document Project-Specific Settings

Use `.ltmatrix/config.toml` to document project requirements:
```toml
# Project-specific configuration
default = "opencode"

[modes.fast]
run_tests = false  # Quick iterations during development

[modes.standard]
model = "claude-sonnet-4-6"  # Use stable model for standard mode
timeout_exec = 7200  # Give tasks more time

[features.pipeline]
enable_parallel_execution = true
enable_smart_cache = true
```

### 4. Leverage CLI Flags for One-Off Overrides

Temporarily enable debug logging:
```bash
ltmatrix --log-level debug "investigate issue"
```

Quick test without execution:
```bash
ltmatrix --dry-run "verify approach"
```

## Implementation Details

### CliOverrides Structure

The `CliOverrides` struct captures all CLI arguments that can override configuration:

```rust
pub struct CliOverrides {
    pub config_file: Option<PathBuf>,
    pub agent: Option<String>,
    pub mode: Option<String>,
    pub output_format: Option<OutputFormat>,
    pub log_level: Option<LogLevel>,
    pub log_file: Option<PathBuf>,
    pub max_retries: Option<u32>,
    pub timeout: Option<u64>,
    pub no_color: Option<bool>,
    pub dry_run: bool,
    pub resume: bool,
    pub ask: bool,
    pub regenerate_plan: bool,
    pub on_blocked: Option<String>,
    pub mcp_config: Option<PathBuf>,
    pub progress: Option<bool>,
    pub run_tests: Option<bool>,
    pub verify: Option<bool>,
}
```

### Conversion Pipeline

1. **CLI Args** → `Args::parse()`
2. **Args** → `CliOverrides::from(args)` (using From trait)
3. **CliOverrides** → `load_config_with_overrides(Some(overrides))`
4. **Merged Config** → final configuration with proper precedence

### Mode Handling

Execution modes work as follows:
- `--fast`: Sets mode to "fast"
- `--expert`: Sets mode to "expert"
- `--mode <mode>`: Explicit mode selection
- No flag: Uses mode from config file or defaults to "standard"

Mode-specific settings (max_retries, timeout, run_tests, verify) can be overridden via CLI and will apply to the selected mode's configuration.

## Troubleshooting

### Config Not Loading?

1. Check precedence: CLI > Custom > Project > Global
2. Verify file paths are correct
3. Check for invalid TOML syntax
4. Look for error messages in output

### Override Not Working?

1. Ensure you're using the correct flag name
2. Check if a custom config file is overriding your override
3. Verify the precedence chain

### Mode Not Applying?

1. Check if mode is configured in config file
2. Verify mode-specific settings exist
3. Use `--mode` for explicit selection

## See Also

- [Feature Flag System](./feature-flag-system.md) - Using feature flags with CLI
- [Configuration Reference](./configuration.md) - Full config documentation
- [Examples](../examples/) - Example usage
