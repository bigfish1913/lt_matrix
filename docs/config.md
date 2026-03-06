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

## CLI Flag Reference

### General Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--agent <name>` | Override default agent | From config |
| `--fast` | Use fast execution mode | Standard mode |
| `--expert` | Use expert execution mode | Standard mode |
| `--dry-run` | Generate tasks without executing | Disabled |
| `--output <format>` | Output format (text, json) | text |
| `--no-color` | Disable colored output | Enabled |
| `--no-progress` | Disable progress bars | Enabled |

### Logging Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--log-level <level>` | Set log level (trace, debug, info, warn, error) | info |
| `--log-file <path>` | Write logs to file | stderr only |

### Task Handling Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--on-blocked <strategy>` | Handle blocked tasks (skip, ask, abort, retry) | skip |
| `--max-retries <count>` | Maximum retry attempts per task | From mode |
| `--timeout <seconds>` | Execution timeout | From mode |

### Configuration Flags

| Flag | Description | Example |
|------|-------------|---------|
| `--config <path>` | Load config from specific path | `ltmatrix --config /custom/config.toml` |
| `--show-config` | Print effective configuration and exit | `ltmatrix --show-config` |

## Environment Variables

ltmatrix respects the following environment variables:

- `LTMATRIX_CONFIG` - Path to custom configuration file
- `LTMATRIX_HOME` - Override ltmatrix home directory (default: `~/.ltmatrix`)
- `NO_COLOR` - Disable colored output (set to any value)
- `RUST_LOG` - Rust tracing log format (for debugging ltmatrix itself)

### Example

```bash
export LTMATRIX_CONFIG=/etc/ltmatrix/config.toml
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
