# Example Configurations Guide

This guide explains how to use ltmatrix example configurations for different development scenarios.

## Overview

ltmatrix provides pre-configured examples optimized for common development workflows:

- **[General Purpose](#general-purpose-configexampletoml)** - All defaults documented
- **[Web Development](#web-development-web-developmenttoml)** - Frontend/backend/full-stack
- **[CLI Tools](#cli-tools-cli-toolstoml)** - Command-line utilities
- **[Data Science](#data-science-data-sciencetoml)** - ML and analytics
- **[Mobile Apps](#mobile-apps-mobile-appstoml)** - iOS/Android/cross-platform

## Quick Start

### 1. Choose Your Scenario

Select the configuration that matches your project type:

| Scenario | Configuration File |
|----------|-------------------|
| General purpose / Learning | `config.example.toml` |
| Web development | `web-development.toml` |
| CLI tools | `cli-tools.toml` |
| Data science / ML | `data-science.toml` |
| Mobile apps | `mobile-apps.toml` |

### 2. Copy to Your Project

```bash
# Create .ltmatrix directory in your project
mkdir -p .ltmatrix

# Copy the appropriate example configuration
cp .ltmatrix/web-development.toml .ltmatrix/config.toml

# Or copy to global config (applies to all projects)
cp .ltmatrix/config.example.toml ~/.ltmatrix/config.toml
```

### 3. Customize (Optional)

Edit the configuration file to match your specific needs:

```bash
# Edit project-specific config
nano .ltmatrix/config.toml

# Or edit global config
nano ~/.ltmatrix/config.toml
```

## Configuration Scenarios

### General Purpose: config.example.toml

**Use case:** Learning about configuration options or creating custom configurations.

This file contains comprehensive documentation of all available options with their default values. It's the best reference for understanding what can be configured.

**Key features:**
- All configuration options documented
- Default values shown
- Examples of common overrides
- Explanations of each setting

**When to use:**
- Learning ltmatrix configuration
- Creating custom configurations
- Understanding default behavior
- Reference for all available options

### Web Development: web-development.toml

**Use case:** Web application development (frontend, backend, full-stack).

Optimized for modern web development workflows with automatic testing and rapid iteration.

**Key features:**
- Fast mode: Quick UI iterations without tests
- Standard mode: Full testing with JavaScript/TypeScript frameworks
- Expert mode: Complex features with Opus
- Automatic framework detection (React, Vue, Django, Rails, etc.)

**Example workflows:**

```bash
# Quick UI styling update
ltmatrix --fast "update the navigation bar with new branding"

# Feature development with testing
ltmatrix "add user authentication with JWT tokens"

# Complex refactoring
ltmatrix --expert "refactor state management to Redux Toolkit"

# Bug fix with tests
ltmatrix "fix the form validation bug and add regression tests"
```

**Supported frameworks:**
- Frontend: React, Vue, Angular, Svelte
- Backend: Express, Django, Rails, Spring, FastAPI
- Testing: Jest, Cypress, Playwright, Selenium, Pytest
- Build tools: Webpack, Vite, esbuild, Parcel

### CLI Tools: cli-tools.toml

**Use case:** Command-line interface tools and utilities.

Optimized for CLI development with emphasis on code quality, comprehensive testing, and documentation.

**Key features:**
- Debug logging for thorough testing feedback
- Comprehensive error handling
- Documentation generation focus
- Cross-platform compatibility considerations

**Example workflows:**

```bash
# Quick CLI prototype
ltmatrix --fast "create a CLI tool that processes CSV files"

# Add feature with tests
ltmatrix "add a --verbose flag with detailed output"

# Production-ready tool
ltmatrix --expert "build a production CLI tool with subcommands and man pages"
```

**CLI-specific considerations:**
- Proper exit codes for scripting
- Clear error messages
- Comprehensive help text
- Shell completion support
- Signal handling (Ctrl+C)
- Cross-platform compatibility (Windows, Linux, macOS)
- stdin/stdout pipe handling

### Data Science: data-science.toml

**Use case:** Data science, machine learning, and analytics projects.

Optimized for data processing workflows with extended timeouts and debug logging.

**Key features:**
- JSON output format for easy parsing
- Extended timeouts for data processing and model training
- Debug logging for pipeline tracking
- Support for experiment tracking

**Example workflows:**

```bash
# Quick data exploration
ltmatrix --fast "analyze the sales data and create visualizations"

# Feature engineering
ltmatrix "add time-based features to the dataset"

# Model development
ltmatrix "train a random forest model with hyperparameter tuning"

# Production ML system
ltmatrix --expert "build an end-to-end ML pipeline with monitoring"
```

**Supported libraries:**
- Data processing: pandas, NumPy, dask, Polars
- ML: scikit-learn, PyTorch, TensorFlow, XGBoost
- Visualization: matplotlib, seaborn, plotly, altair
- Experiment tracking: MLflow, Weights & Biases
- Pipelines: Airflow, Prefect, Dagster

**Data science considerations:**
- Reproducibility (random seeds, version locking)
- Data validation (schema checks, type validation)
- Performance (parallel processing, memory efficiency)
- Experiment tracking (metrics, parameters, artifacts)
- Model versioning (MLflow, DVC)
- Data privacy (PII handling, GDPR compliance)

### Mobile Apps: mobile-apps.toml

**Use case:** Mobile app development (iOS, Android, cross-platform).

Optimized for mobile development with platform-specific best practices and UI/UX focus.

**Key features:**
- Platform-specific optimization
- Extended timeouts for builds and emulators
- UI/UX focus with platform guidelines
- Cross-platform support

**Example workflows:**

```bash
# Quick UI prototype
ltmatrix --fast "create a login screen with form validation"

# Feature development
ltmatrix "add push notification support"

# Platform-specific optimization
ltmatrix --expert "implement offline data synchronization"
```

**Supported platforms:**
- iOS: Swift, SwiftUI, UIKit, Xcode
- Android: Kotlin, Jetpack Compose, Android Studio
- Cross-platform: React Native, Flutter, Xamarin

**Mobile-specific considerations:**
- Platform guidelines (Apple HIG, Material Design)
- Screen sizes (responsive design for phones/tablets)
- Performance (battery usage, memory, startup time)
- Offline support (caching, sync strategies)
- Permissions (camera, location, contacts)
- App store guidelines (Apple Review, Google Play policies)
- Accessibility (VoiceOver, TalkBack)
- Internationalization (multiple languages, RTL support)

## Advanced Usage

### Combining Multiple Configurations

You can layer configurations by using both global and project-specific configs:

**Global config** (`~/.ltmatrix/config.toml`):
```toml
# Default settings for all projects
default = "claude"
[output]
colored = true
progress = true
```

**Project config** (`.ltmatrix/config.toml`):
```toml
# Project-specific overrides
[modes.standard]
model = "claude-opus-4-6"  # Use best model for this project
run_tests = true
verify = true
max_retries = 5
```

### Using Multiple Agents

Configure multiple agents and choose between them:

```toml
[agents.claude]
model = "claude-sonnet-4-6"

[agents.opencode]
command = "opencode"
model = "gpt-4"
```

Usage:
```bash
# Use default agent (claude)
ltmatrix "implement feature X"

# Use specific agent
ltmatrix --agent opencode "implement feature X"
```

### Temporary Overrides

Override configuration temporarily with CLI flags:

```bash
# Use fast mode regardless of config
ltmatrix --fast "quick prototype"

# Use specific model
ltmatrix --model claude-opus-4-6 "complex task"

# Disable tests for this run
ltmatrix --no-tests "skip testing"

# JSON output for parsing
ltmatrix --output json "task" | jq .result
```

## Configuration Validation

### Check Configuration Validity

```bash
# This will fail if config is invalid
ltmatrix --help

# Try a simple task
ltmatrix "test"
```

### Common Configuration Errors

**Error: Missing 'model' field**
```toml
# ❌ Wrong
[agents.claude]
command = "claude"

# ✅ Correct
[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
```

**Error: Invalid log level**
```toml
# ❌ Wrong
[logging]
level = "verbose"

# ✅ Correct
[logging]
level = "debug"
```

Valid levels: `trace`, `debug`, `info`, `warn`, `error`

**Error: Invalid output format**
```toml
# ❌ Wrong
[output]
format = "markdown"

# ✅ Correct
[output]
format = "json"
```

Valid formats: `text`, `json`

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

Commit `.ltmatrix/config.toml` to version control:

```bash
git add .ltmatrix/config.toml
git commit -m "Configure ltmatrix for project"
```

### 4. Document Custom Choices

Add comments to explain why you chose specific values:

```toml
# Use Opus for this project - code quality is critical
[modes.standard]
model = "claude-opus-4-6"

# Extended timeout for large test suite
timeout_exec = 7200
```

### 5. Test Configuration Changes

After modifying configuration, test with a simple task:

```bash
ltmatrix "create a simple hello world function"
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
   # Use a TOML linter
   # https://www.toml-lint.com/
   ```

3. **Check effective configuration:**
   ```bash
   ltmatrix --help
   ```

### Agent Not Found

**Error:** `Agent 'xxx' not found in configuration`

**Solution:** Define the agent in your configuration:

```toml
[agents.xxx]
command = "xxx"
model = "model-id"
```

### Model Not Specified

**Error:** `Agent 'claude' missing 'model' field`

**Solution:** Add the model field:

```toml
[agents.claude]
model = "claude-sonnet-4-6"  # Required
```

### Timeout Too Short

**Symptoms:** Tasks fail with timeout errors

**Solution:** Increase timeout for the agent or mode:

```toml
[agents.claude]
timeout = 7200  # 2 hours

[modes.standard]
timeout_exec = 7200  # 2 hours for execution
```

## Migration from Other Tools

### From Python longtime.py

If you're migrating from the Python `longtime.py`, update your configuration:

**Old model names:**
```python
MODEL_FAST = "glm-5"
MODEL_SMART = "glm-5"
```

**New model names:**
```toml
[modes.fast]
model = "claude-haiku-4-5"  # or claude-sonnet-4-6

[modes.standard]
model = "claude-sonnet-4-6"

[modes.expert]
model = "claude-opus-4-6"
```

### From AI Coding Assistants

If you're used to other AI coding assistants, ltmatrix offers:

- **Multi-agent support**: Switch between Claude, OpenCode, KimiCode, Codex
- **Execution modes**: Fast for prototyping, Standard for development, Expert for production
- **Full workflow**: Planning → Execution → Testing → Verification → Commit
- **Git integration**: Automatic commits per task
- **Resume capability**: Continue interrupted work

## See Also

- [Configuration Reference](./config.md) - Complete configuration documentation
- [CLI Reference](./cli.md) - Command-line interface reference
- [Examples](../.ltmatrix/README.md) - Example configurations guide

## Getting Help

If you need help with configuration:

1. Check the [Configuration Reference](./config.md)
2. Review `config.example.toml` for all available options
3. Look at scenario-specific examples for your use case
4. Open an issue on [GitHub](https://github.com/bigfish/ltmatrix/issues)
