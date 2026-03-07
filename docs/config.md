# ltmatrix Configuration Reference

Complete guide to configuring ltmatrix via TOML configuration files and command-line arguments.

## Table of Contents

- [Configuration Sources](#configuration-sources)
- [Global Configuration](#global-configuration)
- [Project Configuration](#project-configuration)
- [Configuration Precedence](#configuration-precedence)
- [Agent Configuration](#agent-configuration)
- [Execution Mode Configuration](#execution-mode-configuration)
- [Output Configuration](#output-configuration)
- [Logging Configuration](#logging-configuration)
- [Warmup Configuration](#warmup-configuration)
- [Session Pool Configuration](#session-pool-configuration)
- [Feature Flags](#feature-flags)
- [MCP Configuration](#mcp-configuration)
- [Example Configurations](#example-configurations)
- [CLI Flag Reference](#cli-flag-reference)

## Configuration Sources

ltmatrix loads configuration from multiple sources in order of precedence (highest to lowest):

1. **Command-line arguments** - Highest precedence, override all other settings
2. **Project config** - `.ltmatrix/config.toml` in current working directory
3. **Global config** - `~/.ltmatrix/config.toml` (user home directory)
4. **Default values** - Built-in defaults for all settings

## Global Configuration

Global configuration applies to all ltmatrix operations unless overridden by project config or CLI flags.

### Location

```bash
~/.ltmatrix/config.toml
```

### Example Global Config

```toml
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[output]
format = "text"
colored = true
progress = true

[logging]
level = "info"
```

## Project Configuration

Project configuration provides settings specific to a project workspace.

### Location

```bash
<project-root>/.ltmatrix/config.toml
```

Project-specific settings override global settings but can be overridden by CLI arguments.

### Example Project Config

```toml
default = "opencode"

[agents.opencode]
command = "opencode"
model = "gpt-4"
timeout = 1800

[modes.standard]
model = "claude-sonnet-4-6"
run_tests = true
verify = true
max_retries = 3
max_depth = 3
```

## Configuration Precedence

Settings are merged in this order:

1. Start with built-in defaults
2. Apply global config (if exists)
3. Apply project config (if exists)
4. Apply CLI argument overrides

### Example: Precedence in Action

**Global config** (`~/.ltmatrix/config.toml`):
```toml
default = "claude"
[output]
format = "json"
```

**Project config** (`.ltmatrix/config.toml`):
```toml
default = "opencode"
[logging]
level = "debug"
```

**CLI invocation**:
```bash
ltmatrix --output text --log-level warn "goal"
```

**Final effective settings**:
- `default` = "opencode" (from project)
- `output.format` = "text" (CLI overrides project/global)
- `logging.level` = "warn" (CLI overrides project)
- Other settings from defaults/global

## Agent Configuration

Configure multiple AI agent backends for different use cases.

### Supported Agents

| Agent | Command | Description |
|-------|----------|-------------|
| `claude` | `claude` | Anthropic Claude Code CLI (default) |
| `opencode` | `opencode` | OpenCode AI assistant |
| `kimi-code` | `kimi-code` | Moonshot KimiCode |
| `codex` | `codex` | OpenAI Codex |

### Agent Configuration Structure

```toml
[agents.<agent-name>]
command = "<cli-command>"
model = "<model-identifier>"
timeout = <seconds>
```

### Agent Fields

- **`command`** (optional): CLI command to invoke the agent. If not specified, defaults to the agent name.
- **`model`** (required): Model identifier to use with this agent.
- **`timeout`** (optional): Operation timeout in seconds. Default: 3600 (1 hour).

### Example Agent Configurations

```toml
# Claude agent configuration
[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

# OpenCode configuration
[agents.opencode]
command = "opencode"
model = "gpt-4"
timeout = 1800

# KimiCode configuration
[agents.kimi-code]
command = "kimi-code"
model = "moonshot-v1"
timeout = 3600
```

### Default Agent

Set the default agent with the `default` field:

```toml
default = "claude"
```

This can be overridden with the `--agent` CLI flag:

```bash
ltmatrix --agent opencode "goal"
```

## Execution Mode Configuration

Configure behavior for different execution modes (fast, standard, expert).

### Mode Configuration Structure

```toml
[modes.<mode-name>]
model = "<model-identifier>"
run_tests = <boolean>
verify = <boolean>
max_retries = <number>
max_depth = <number>
timeout_plan = <seconds>
timeout_exec = <seconds>
```

### Mode Fields

- **`model`** (optional): Default model to use in this mode
- **`run_tests`**: Whether to run tests during execution
- **`verify`**: Whether to verify task completion
- **`max_retries`**: Maximum number of retry attempts for failed tasks
- **`max_depth`**: Maximum depth for task decomposition (1-5)
- **`timeout_plan`**: Planning phase timeout in seconds
- **`timeout_exec`**: Execution phase timeout in seconds

### Fast Mode Configuration

Fast mode prioritizes speed over thoroughness:

```toml
[modes.fast]
model = "claude-haiku-4-5"
run_tests = false
verify = true
max_retries = 1
max_depth = 2
timeout_plan = 60
timeout_exec = 1800
```

**Characteristics:**
- Uses fast models (Haiku)
- Skips test execution
- Minimal verification
- Shallow task decomposition
- Shorter timeouts

### Standard Mode Configuration

Standard mode provides balanced execution:

```toml
[modes.standard]
model = "claude-sonnet-4-6"
run_tests = true
verify = true
max_retries = 3
max_depth = 3
timeout_plan = 120
timeout_exec = 3600
```

**Characteristics:**
- Uses standard models (Sonnet)
- Full test execution
- Complete verification
- Medium task decomposition
- Standard timeouts

### Expert Mode Configuration

Expert mode provides highest quality results:

```toml
[modes.expert]
model = "claude-opus-4-6"
run_tests = true
verify = true
max_retries = 5
max_depth = 4
timeout_plan = 300
timeout_exec = 7200
```

**Characteristics:**
- Uses best models (Opus)
- Full test execution
- Complete verification
- Deep task decomposition
- Extended timeouts
- Code review (when applicable)

### Selecting Execution Mode

Use CLI flags to select execution mode:

```bash
ltmatrix "goal"                    # Standard mode (default)
ltmatrix --fast "goal"            # Fast mode
ltmatrix --expert "goal"          # Expert mode
```

## Output Configuration

Control how ltmatrix presents information.

### Output Configuration Structure

```toml
[output]
format = "<format>"
colored = <boolean>
progress = <boolean>
```

### Output Fields

- **`format`**: Output format - `"text"` or `"json"`
- **`colored`**: Use ANSI colors in terminal output
- **`progress`**: Show progress bars and dynamic updates

### Example Output Configurations

```toml
[output]
format = "text"
colored = true
progress = true
```

**JSON output example:**
```toml
[output]
format = "json"
colored = false
progress = false
```

### CLI Overrides

```bash
ltmatrix --output json "goal"
ltmatrix --output text "goal"
ltmatrix --no-color "goal"
ltmatrix --no-progress "goal"
```

## Logging Configuration

Configure logging verbosity and output.

### Logging Configuration Structure

```toml
[logging]
level = "<log-level>"
file = "</path/to/logfile>"
```

### Logging Fields

- **`level`**: Minimum log level to display
  - `"trace"` - Most verbose, shows all API calls
  - `"debug"` - Task scheduling details, file changes
  - `"info"` - Task start/complete, progress (default)
  - `"warn"` - Retries, skipped tasks
  - `"error"` - Only failures
- **`file`** (optional): Path to log file. If not specified, logs to stderr only.

### Example Logging Configurations

```toml
[logging]
level = "info"
```

**Debug logging with file output:**
```toml
[logging]
level = "debug"
file = "/var/log/ltmatrix/ltmatrix.log"
```

**Trace-level debugging:**
```toml
[logging]
level = "trace"
file = "/tmp/ltmatrix-debug.log"
```

### CLI Overrides

```bash
ltmatrix --log-level debug "goal"
ltmatrix --log-level trace --log-file debug.log "goal"
```

## Warmup Configuration

Warmup configuration controls pre-execution queries to agent sessions to reduce latency for the first real task.

### Warmup Configuration Structure

```toml
[warmup]
enabled = <boolean>
max_queries = <number>
timeout_seconds = <seconds>
retry_on_failure = <boolean>
prompt_template = "<custom-prompt>"
```

### Warmup Fields

- **`enabled`**: Enable/disable warmup queries (default: `false`)
- **`max_queries`**: Maximum number of warmup queries to send (default: `3`)
- **`timeout_seconds`**: Timeout for warmup queries in seconds (default: `30`)
- **`retry_on_failure`**: Whether to retry warmup on failure (default: `false`)
- **`prompt_template`**: Custom prompt template for warmup queries (optional)

### Warmup Behavior

When warmup is enabled, ltmatrix sends simple queries to agents before actual task execution to:
- Initialize agent sessions
- Reduce cold-start latency
- Establish session context
- Verify agent availability

### Example Warmup Configurations

**Disabled warmup (default):**
```toml
[warmup]
enabled = false
```

**Basic warmup:**
```toml
[warmup]
enabled = true
max_queries = 3
timeout_seconds = 30
```

**Aggressive warmup with retries:**
```toml
[warmup]
enabled = true
max_queries = 5
timeout_seconds = 60
retry_on_failure = true
```

**Custom warmup prompt:**
```toml
[warmup]
enabled = true
prompt_template = "You are ready to assist with coding tasks. Respond with OK."
```

### CLI Overrides

There are no direct CLI flags for warmup configuration. Use config files to adjust warmup settings.

### Validation

Warmup configuration validates that:
- `max_queries` is greater than 0
- `timeout_seconds` is greater than 0
- `prompt_template` is not empty if provided

## Session Pool Configuration

Session pool configuration controls how agent sessions are managed, reused, and cleaned up.

### Session Pool Configuration Structure

```toml
[pool]
max_sessions = <number>
auto_cleanup = <boolean>
cleanup_interval_seconds = <seconds>
stale_threshold_seconds = <seconds>
enable_reuse = <boolean>
```

### Session Pool Fields

- **`max_sessions`**: Maximum number of sessions to keep in the pool (default: `100`)
- **`auto_cleanup`**: Automatically clean up stale sessions (default: `true`)
- **`cleanup_interval_seconds`**: Interval between cleanup runs in seconds (default: `300` = 5 minutes)
- **`stale_threshold_seconds`**: Session staleness threshold in seconds (default: `3600` = 1 hour)
- **`enable_reuse`**: Enable session reuse across tasks (default: `true`)

### Session Pool Behavior

The session pool:
- Manages agent sessions to reduce initialization overhead
- Reuses sessions for retries and dependent tasks
- Automatically cleans up stale sessions
- Prevents unbounded session growth

### Example Session Pool Configurations

**Default configuration:**
```toml
[pool]
max_sessions = 100
auto_cleanup = true
cleanup_interval_seconds = 300
stale_threshold_seconds = 3600
enable_reuse = true
```

**Conservative configuration (limited sessions):**
```toml
[pool]
max_sessions = 20
auto_cleanup = true
cleanup_interval_seconds = 600
stale_threshold_seconds = 1800
enable_reuse = true
```

**Aggressive configuration (maximum reuse):**
```toml
[pool]
max_sessions = 500
auto_cleanup = true
cleanup_interval_seconds = 120
stale_threshold_seconds = 7200
enable_reuse = true
```

**Disable session reuse:**
```toml
[pool]
enable_reuse = false
```

### CLI Overrides

There are no direct CLI flags for session pool configuration. Use config files to adjust pool settings.

### Validation

Session pool configuration validates that:
- `max_sessions` is greater than 0
- `cleanup_interval_seconds` is greater than 0
- `stale_threshold_seconds` is greater than 0

## Feature Flags

Feature flags control experimental or optional functionality in ltmatrix with support for gradual rollout and A/B testing.

### Feature Flags Structure

```toml
[agent_backend]
enable_claude_opus_backend = <boolean>
enable_opencode_backend = <boolean>
enable_kimi_code_backend = <boolean>
enable_codex_backend = <boolean>
enable_custom_backend = <boolean>

[pipeline]
enable_parallel_execution = <boolean>
enable_smart_cache = <boolean>
enable_incremental_builds = <boolean>
enable_distributed_tasks = <boolean>

[scheduler]
enable_priority_scheduler = <boolean>
enable_adaptive_scheduler = <boolean>

[rollout.<feature_name>]
percentage = <0-100>
users = ["user1", "user2", ...]
```

### Agent Backend Features

Control which agent backends are available:

- **`enable_claude_opus_backend`**: Enable Claude Opus backend (experimental)
- **`enable_opencode_backend`**: Enable OpenCode backend
- **`enable_kimi_code_backend`**: Enable KimiCode backend
- **`enable_codex_backend`**: Enable Codex backend
- **`enable_custom_backend`**: Enable custom agent backend support

### Pipeline Features

Control pipeline execution behavior:

- **`enable_parallel_execution`**: Enable parallel task execution
- **`enable_smart_cache`**: Enable smart caching for intermediate results
- **`enable_incremental_builds`**: Only rebuild changed components
- **`enable_distributed_tasks`**: Enable distributed task execution across machines

### Scheduler Features

Control task scheduling behavior:

- **`enable_priority_scheduler`**: Use priority-based task scheduling
- **`enable_adaptive_scheduler`**: Use adaptive scheduling based on task performance

### Gradual Rollout

Configure gradual rollout for features:

```toml
[rollout.enable_parallel_execution]
percentage = 50  # Enable for 50% of users
users = ["beta_tester_1", "beta_tester_2"]  # Always enable for specific users
```

### Example Feature Configuration

```toml
# Enable experimental features
[agent_backend]
enable_claude_opus_backend = true
enable_opencode_backend = false

[pipeline]
enable_parallel_execution = true
enable_smart_cache = true
enable_incremental_builds = false

[scheduler]
enable_priority_scheduler = false

# Gradual rollout for parallel execution
[rollout.enable_parallel_execution]
percentage = 25
users = ["beta_team"]
```

### CLI Overrides

Feature flags can be controlled via CLI flags specific to each feature:

```bash
ltmatrix --enable-parallel-execution "goal"
ltmatrix --disable-smart-cache "goal"
ltmatrix --enable-priority-scheduler "goal"
```

## MCP Configuration

MCP (Model Context Protocol) configuration enables integration with testing tools like Playwright for end-to-end testing.

### MCP Configuration File Location

MCP configuration is loaded from a separate file specified via the `--mcp-config` CLI flag:

```bash
ltmatrix --mcp-config .mcp.json "goal"
```

### MCP Configuration Structure (JSON)

```json
{
  "mcp": {
    "servers": {
      "playwright": {
        "type": "playwright",
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-playwright"],
        "env": {
          "HEADLESS": "true"
        },
        "cwd": null,
        "timeout": 60,
        "enabled": true
      }
    }
  }
}
```

### MCP Server Fields

- **`type`**: Server type (e.g., "playwright", "browser")
- **`command`**: Command to run the server (optional)
- **`args`**: Arguments to pass to the command (array of strings)
- **`env`**: Environment variables for the server (key-value pairs)
- **`cwd`**: Server working directory (optional, null for current directory)
- **`timeout`**: Server timeout in seconds (default: `60`)
- **`enabled`**: Whether the server is enabled (default: `true`)

### Example MCP Configurations

**Playwright for browser testing:**
```json
{
  "mcp": {
    "servers": {
      "playwright": {
        "type": "playwright",
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-playwright"],
        "env": {
          "HEADLESS": "true"
        },
        "timeout": 60,
        "enabled": true
      }
    }
  }
}
```

**Browser automation:**
```json
{
  "mcp": {
    "servers": {
      "browser": {
        "type": "browser",
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-browser"],
        "timeout": 120,
        "enabled": true
      }
    }
  }
}
```

### MCP Configuration Validation

MCP configuration validates that:
- Server type is not empty
- Command is not empty if provided
- Timeout is greater than 0
- Working directory exists if provided

### CLI Flag

Use the `--mcp-config` flag to specify the MCP configuration file:

```bash
ltmatrix --mcp-config .mcp.json "test the login page"
```

## Example Configurations

### Minimal Configuration

```toml
# ~/.ltmatrix/config.toml
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
```

### Development Configuration

```toml
# ~/.ltmatrix/config.toml
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[output]
format = "text"
colored = true
progress = true

[logging]
level = "debug"

# Enable warmup for reduced latency
[warmup]
enabled = true
max_queries = 3
timeout_seconds = 30

# Moderate session pool for development
[pool]
max_sessions = 50
auto_cleanup = true
cleanup_interval_seconds = 300
stale_threshold_seconds = 3600
enable_reuse = true

[modes.standard]
model = "claude-sonnet-4-6"
run_tests = true
verify = true
max_retries = 3
max_depth = 3
```

### CI/CD Configuration

```toml
# .ltmatrix/config.toml
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 7200

[output]
format = "json"
colored = false
progress = false

[logging]
level = "warn"

# Disable warmup in CI (adds latency without benefit)
[warmup]
enabled = false

# Conservative pool settings for CI
[pool]
max_sessions = 10
auto_cleanup = true
cleanup_interval_seconds = 120
stale_threshold_seconds = 600
enable_reuse = false

[modes.standard]
run_tests = true
verify = true
max_retries = 3
max_depth = 3
```

### Multi-Agent Configuration

```toml
# ~/.ltmatrix/config.toml
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[agents.opencode]
command = "opencode"
model = "gpt-4"
timeout = 1800

[agents.kimi-code]
command = "kimi-code"
model = "moonshot-v1-8k"
timeout = 3600
```

### Fast Development Configuration

```toml
# .ltmatrix/config.toml
default = "claude"

[agents.claude]
model = "claude-haiku-4-5"
timeout = 1800

[modes.fast]
run_tests = false
verify = false
max_retries = 1
max_depth = 2
timeout_plan = 30
timeout_exec = 600
```

### High-Quality Production Configuration

```toml
# .ltmatrix/config.toml
default = "claude"

[agents.claude]
model = "claude-opus-4-6"
timeout = 7200

[modes.expert]
run_tests = true
verify = true
max_retries = 5
max_depth = 5
timeout_plan = 600
timeout_exec = 10800

[output]
format = "text"
colored = true
progress = true

[logging]
level = "info"
```

### E2E Testing Configuration with MCP

```toml
# .ltmatrix/config.toml
default = "claude"

[agents.claude]
model = "claude-sonnet-4-6"
timeout = 7200

# Enable warmup for consistent test performance
[warmup]
enabled = true
max_queries = 5
timeout_seconds = 60
retry_on_failure = true

# Larger pool for concurrent test sessions
[pool]
max_sessions = 200
auto_cleanup = true
cleanup_interval_seconds = 600
stale_threshold_seconds = 7200
enable_reuse = true

[modes.standard]
run_tests = true
verify = true
max_retries = 5
max_depth = 3
timeout_plan = 300
timeout_exec = 7200
```

With MCP configuration file (`.mcp.json`):
```json
{
  "mcp": {
    "servers": {
      "playwright": {
        "type": "playwright",
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-playwright"],
        "env": {
          "HEADLESS": "true"
        },
        "timeout": 120,
        "enabled": true
      }
    }
  }
}
```

Usage:
```bash
ltmatrix --mcp-config .mcp.json "test the user authentication flow"
```

### High-Performance Batch Processing

```toml
# .ltmatrix/config.toml
default = "claude"

[agents.claude]
model = "claude-sonnet-4-6"
timeout = 1800

# Aggressive warmup for batch processing
[warmup]
enabled = true
max_queries = 10
timeout_seconds = 30

# Large session pool for many parallel tasks
[pool]
max_sessions = 500
auto_cleanup = true
cleanup_interval_seconds = 60
stale_threshold_seconds = 3600
enable_reuse = true

[modes.fast]
run_tests = false
verify = false
max_retries = 1
max_depth = 2
timeout_plan = 30
timeout_exec = 600
```

## CLI Flag Reference

### General Flags

| Flag | Description | Config Equivalent | Default |
|------|-------------|-------------------|---------|
| `--agent <name>` | Override default agent | `default` | From config or `claude` |
| `--mode <mode>` | Execution mode preset (fast, standard, expert) | N/A | `standard` |
| `--fast` | Use fast execution mode | N/A | Disabled |
| `--expert` | Use expert execution mode | N/A | Disabled |
| `--dry-run` | Generate tasks without executing | N/A | Disabled |
| `--resume` | Resume interrupted work from workspace state | N/A | Disabled |
| `--ask` | Ask for clarification before planning | N/A | Disabled |
| `--regenerate-plan` | Regenerate the plan | N/A | Disabled |

### Output Flags

| Flag | Description | Config Equivalent | Default |
|------|-------------|-------------------|---------|
| `--output <format>` | Output format (text, json, json-compact) | `output.format` | `text` |
| `--no-color` | Disable colored output | `output.colored` | `true` |
| `--no-progress` | Disable progress bars | `output.progress` | `true` |

### Logging Flags

| Flag | Description | Config Equivalent | Default |
|------|-------------|-------------------|---------|
| `--log-level <level>` | Set log level (trace, debug, info, warn, error) | `logging.level` | `info` |
| `--log-file <path>` | Write logs to file | `logging.file` | `stderr` only |

### Task Handling Flags

| Flag | Description | Config Equivalent | Default |
|------|-------------|-------------------|---------|
| `--on-blocked <strategy>` | Handle blocked tasks (skip, ask, abort, retry) | N/A | `skip` |
| `--max-retries <count>` | Maximum retry attempts per task | `modes.<mode>.max_retries` | From mode |
| `--timeout <seconds>` | Execution timeout in seconds | `modes.<mode>.timeout_exec` | From mode |

### Configuration Flags

| Flag | Description | Example |
|------|-------------|---------|
| `--config <path>` | `-c <path>` - Load config from specific path | `ltmatrix --config /custom/config.toml` |
| `--mcp-config <path>` | Load MCP configuration from file | `ltmatrix --mcp-config .mcp.json "test login"` |

### CLI Flag Override Behavior

CLI flags override configuration file settings in the following ways:

**Direct Overrides:**
- `--agent` overrides `default` in config
- `--output` overrides `output.format`
- `--log-level` overrides `logging.level`
- `--log-file` overrides `logging.file`
- `--max-retries` overrides `modes.<mode>.max_retries`
- `--timeout` overrides `modes.<mode>.timeout_exec`

**Mode Selection:**
- `--fast` / `--expert` / `--mode <mode>` selects which mode config to use
- All mode-specific settings come from the selected mode
- CLI flags like `--max-retries` can further override mode settings

**Priority Order:**
1. CLI flags (highest priority)
2. Project config (`.ltmatrix/config.toml`)
3. Global config (`~/.ltmatrix/config.toml`)
4. Built-in defaults (lowest priority)

**Example:**

Config file:
```toml
[output]
format = "json"
colored = false
```

CLI invocation:
```bash
ltmatrix --output text "goal"
```

Result: `format` is `text` (CLI override), `colored` is `false` (from config)

### Subcommand Flags

#### Release Subcommand
| Flag | Description | Default |
|------|-------------|---------|
| `--target <triple>` | Build target triple | Host platform |
| `--output <dir>` | Output directory | `./dist` |
| `--archive` | Create release archives | Disabled |
| `--all-targets` | Build for all supported targets | Disabled |

#### Completions Subcommand
| Flag | Description | Example |
|------|-------------|---------|
| `<shell>` | Shell type (bash, zsh, fish, powershell, elvish) | `ltmatrix completions bash` |
| `--install` | Print installation instructions | `ltmatrix completions zsh --install` |

#### Man Subcommand
| Flag | Description | Default |
|------|-------------|---------|
| `--output <dir>` | `-o <dir>` - Output directory | `./man` |

#### Cleanup Subcommand
| Flag | Description | Example |
|------|-------------|---------|
| `--remove` | Remove all workspace state files | `ltmatrix cleanup --remove --force` |
| `--reset-all` | Reset all tasks to pending | `ltmatrix cleanup --reset-all` |
| `--reset-failed` | Reset only failed tasks to pending | `ltmatrix cleanup --reset-failed` |
| `--force` | Force cleanup without confirmation | Required for destructive actions |
| `--dry-run` | Show what would be cleaned up | Preview changes |

## Environment Variables

ltmatrix respects the following environment variables:

### Configuration

- `LTMATRIX_CONFIG` - Path to custom configuration file
  ```bash
  export LTMATRIX_CONFIG=/etc/ltmatrix/config.toml
  ltmatrix "goal"
  ```

- `LTMATRIX_HOME` - Override ltmatrix home directory (default: `~/.ltmatrix`)
  ```bash
  export LTMATRIX_HOME=/custom/ltmatrix
  # Loads config from /custom/ltmatrix/config.toml
  ```

### Output Control

- `NO_COLOR` - Disable colored output (set to any value)
  ```bash
  export NO_COLOR=1
  ltmatrix "goal"  # Output without colors
  ```

### Debugging

- `RUST_LOG` - Rust tracing log format (for debugging ltmatrix itself)
  ```bash
  export RUST_LOG=ltmatrix=debug
  ltmatrix "goal"
  ```

### Agent-Specific Variables

Agent backends may respect their own environment variables:

**Claude:**
- `ANTHROPIC_API_KEY` - API key for Claude
- `CLAUDE_API_KEY` - Alternative API key variable

**OpenCode:**
- `OPENCODE_API_KEY` - API key for OpenCode

**Example:**
```bash
export ANTHROPIC_API_KEY=sk-ant-...
ltmatrix "goal"
```

## Configuration Validation

ltmatrix validates configuration on startup:

### Required Fields

- Agent configurations must have a `model` field
- Model identifiers must be valid for the agent backend
- Timeout values must be positive integers
- Log levels must be one of: trace, debug, info, warn, error

### Validation Errors

Example validation error:

```
Error: Failed to parse TOML from ~/.ltmatrix/config.toml

Caused by:
  Error processing agents.claude: missing 'model' field
```

## Best Practices

### 1. Start Simple

Begin with minimal configuration and add complexity as needed:

```toml
# Minimal starting configuration
default = "claude"

[agents.claude]
model = "claude-sonnet-4-6"
```

### 2. Use Project-Specific Configs

Keep global config generic, customize per project:

**Global config** (`~/.ltmatrix/config.toml`):
```toml
default = "claude"
[output]
colored = true
```

**Project config** (`.ltmatrix/config.toml`):
```toml
[modes.standard]
run_tests = true
verify = true
```

### 3. Version Control Configuration

Commit `.ltmatrix/config.toml` to version control for team consistency:

```bash
git add .ltmatrix/config.toml
git commit -m "Configure ltmatrix for project"
```

### 4. Document Custom Agents

Document non-standard agent configurations in project README:

```markdown
## ltmatrix Configuration

This project uses a custom agent configuration. See `.ltmatrix/config.toml` for details.
```

### 5. Sensitive Data

Avoid committing sensitive data (API keys, tokens) to config files:

```toml
# DON'T commit this
[agents.claude]
api_key = "sk-ant-..."  # ❌ Don't commit

# Use environment variables instead
[agents.claude]
# API key read from CLAUDE_API_KEY environment variable
```

## Troubleshooting

### Configuration Not Loading

1. **Check file paths:**
   ```bash
   # Verify global config exists
   ls -la ~/.ltmatrix/config.toml

   # Verify project config exists
   ls -la .ltmatrix/config.toml
   ```

2. **Validate TOML syntax:**
   ```bash
   # Use a TOML linter or parser
   ltmatrix --show-config
   ```

3. **Check effective config:**
   ```bash
   ltmatrix --show-config
   ```

### Agent Not Found

Error: `Agent 'custom-agent' not found in configuration`

**Solution:** Ensure agent is defined in config:
```toml
[agents.custom-agent]
command = "custom"
model = "model-id"
```

### Invalid TOML Errors

Error: `Failed to parse TOML`

**Common issues:**
- Missing closing brackets `]`
- Unclosed strings `"missing quote`
- Invalid boolean values (use `true`/`false`, not `yes`/`no`)

## Default Values Reference

Complete list of built-in defaults:

```toml
# Default configuration (built-in)
default = "claude"

[agents.claude]
command = "claude"      # Default: agent name
model = "claude-sonnet-4-6"  # No default, required
timeout = 3600        # Default: 3600 seconds (1 hour)

[output]
format = "text"        # Default: text
colored = true         # Default: true
progress = true        # Default: true

[logging]
level = "info"         # Default: info
file = null           # Default: null (stderr only)

[warmup]
enabled = false        # Default: false (disabled)
max_queries = 3        # Default: 3
timeout_seconds = 30   # Default: 30 seconds
retry_on_failure = false  # Default: false
prompt_template = null # Default: null (use built-in prompt)

[pool]
max_sessions = 100     # Default: 100 sessions
auto_cleanup = true    # Default: true
cleanup_interval_seconds = 300  # Default: 300 seconds (5 minutes)
stale_threshold_seconds = 3600   # Default: 3600 seconds (1 hour)
enable_reuse = true    # Default: true

[modes.fast]
model = "claude-haiku-4-5"
run_tests = false
verify = true
max_retries = 1
max_depth = 2
timeout_plan = 60
timeout_exec = 1800

[modes.standard]
model = "claude-sonnet-4-6"
run_tests = true
verify = true
max_retries = 3
max_depth = 3
timeout_plan = 120
timeout_exec = 3600

[modes.expert]
model = "claude-opus-4-6"
run_tests = true
verify = true
max_retries = 5
max_depth = 4
timeout_plan = 300
timeout_exec = 7200
```

## Migration from Python longtime.py

If you're migrating from the Python `longtime.py`, note these key differences:

### Configuration File Location

- **Python**: `~/.longtime.py/config.toml`
- **Rust**: `~/.ltmatrix/config.toml`

### Environment Variables

- **Python**: `LONGTIME_HOME`
- **Rust**: `LTMATRIX_HOME`

### Model Names

Updated to current Claude models:
- `MODEL_FAST`: `"claude-sonnet-4-6"` (was `"glm-5"`)
- `MODEL_SMART`: `"claude-opus-4-6"` (was `"glm-5"`)

### Configuration Compatibility

Most Python configuration files are compatible after updating model names. Simply rename/move your config:

```bash
# Backup old config
mv ~/.longtime.py/config.toml ~/.ltmatrix/config.toml.bak

# Update model names if needed
# MODEL_FAST -> claude-sonnet-4-6
# MODEL_SMART -> claude-opus-4-6
```

## Additional Resources

- [CLI Reference](./cli.md) - Complete command-line interface documentation
- [Agent Backends](./agents.md) - Configuring specific agent backends
- [Execution Modes](./modes.md) - Detailed mode behavior and use cases
- [Examples](./examples/) - Example configurations for various scenarios

## Contributing

When contributing new configuration options:

1. Update this documentation
2. Add tests to `tests/config_tests.rs`
3. Update built-in defaults if changing behavior
4. Document migration path for breaking changes

For configuration-related issues or questions, please open an issue on GitHub.
