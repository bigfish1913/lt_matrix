# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Telemetry & Analytics
- Telemetry and analytics system (opt-in, privacy-focused)
  - Anonymous usage tracking with UUID session IDs
  - Tracks execution mode, agent backends, task counts, success rates
  - Configurable endpoint with batch sending and retry logic
  - `--telemetry` CLI flag to enable

#### Pipeline & Orchestration
- Pipeline orchestrator for coordinating all pipeline stages
  - Sequential stage execution: Generate → Assess → Execute → Test → Verify → Commit → Memory
  - Mode-based stage skipping (Fast skips Test, Expert adds Review)
  - Progress tracking and state management
- Review stage for code quality checks (Expert mode)
  - Security vulnerability detection
  - Performance issue identification
  - Best practices compliance
  - Code complexity analysis
- Test coverage analysis and fix cycle triggering
- Security and performance checks in review stage

#### Workspace & State Management
- Workspace management system
  - Persistent workspace state across sessions
  - State recovery for interrupted runs
  - Automatic cleanup on completion
- State persistence core with error handling for corrupted files
- State transformation logic for task recovery
- State consistency validation across task execution

#### Validation & Testing
- Validation utilities module
  - Input validation helpers
  - Configuration validation
  - Data integrity checks
- Testing module with framework detection
  - Auto-detect pytest, npm test, Go test, Cargo test
  - Framework-specific configuration handling
  - Test command mapping

#### Memory System
- Memory system for project context
  - Persistent memory storage in `.claude/memory.md`
  - Memory extraction from completed tasks
  - Architecture decision tracking

#### Git Integration
- Git integration with per-task branching
  - Automatic branch creation per task
  - Squash merge to main branch
  - Conflict detection and resolution
- Git repository management
  - Automatic init and `.gitignore` generation
  - Branch management utilities
  - Commit utilities with conventional commit support

#### Agent System
- AgentPool with session management
  - Session reuse for retries
  - Session inheritance for dependent tasks
  - Warmup queries for session initialization
- Multi-agent backend support
  - Claude (default) with model selection (Sonnet/Opus)
  - OpenCode agent backend
  - KimiCode agent backend
  - Codex agent backend
- AgentFactory for backend instantiation
- AgentBackend trait for unified interface

#### Task Management
- Task dependency topological sort
- Task hierarchy tree view
- Topological ASCII art visualization
- Mermaid diagram generation
- Task assessment stage with complexity evaluation

#### Configuration
- Configuration system with TOML support
  - Global config (`~/.ltmatrix/config.toml`)
  - Project config (`.ltmatrix/config.toml`)
  - CLI argument override support
  - Configuration merge with precedence
- MCP configuration support

#### Execution Modes
- Three execution modes
  - Fast mode: Quick iterations, skips tests
  - Standard mode: Full 6-stage pipeline
  - Expert mode: Highest quality with code review
- `--expert` flag for expert mode execution

#### CLI & User Experience
- Cross-platform binary builds
  - Windows (x86_64, ARM64)
  - Linux (x86_64, ARM64, musl static linking)
  - macOS (Intel, Apple Silicon, universal binary)
- CLI with rich terminal UX
  - Real-time progress bars with ETA estimation
  - Colorized output
  - JSON output format support (`--output json`)
  - Shell completions (bash, zsh, fish, PowerShell, Elvish)
  - Man page generation
- `--resume` flag for workspace recovery
- `--dry-run` flag for plan-only execution
- Interactive clarification with `--ask` flag
- Cleanup commands for workspace management

#### Logging
- Logging and tracing system
  - Structured logging with tracing
  - Multiple log levels (TRACE, DEBUG, INFO, WARN, ERROR)
  - Log file output with rotation
  - Environment-based filtering
- Logging directory structure

#### Progress & Visualization
- Progress bar infrastructure
- ETA estimation and enhanced metrics
- Task hierarchy tree view
- Topological ASCII art visualization
- Mermaid diagram generation

#### Documentation
- Configuration reference documentation
- macOS build guide documentation
- Cross-compilation guide

### Changed
- Improved error messages with context
- Enhanced task scheduling with better parallel execution
- Optimized memory usage for large projects
- Better handling of git merge conflicts
- Improved session management performance

### Fixed
- Model name errors (corrected to claude-sonnet-4-6 and claude-opus-4-6)
- Task status detection for resume functionality
- Circular dependency detection in task graphs
- Chinese language hardcoding in interactive prompts

### Security
- No sensitive data in telemetry collection
- Secure HTTP-only transmission for analytics
- No IP address logging at telemetry endpoint

## [0.1.0] - 2025-01-15

### Added

#### Core Infrastructure
- Initial Rust implementation of ltmatrix
- Core data models (Task, Agent, ExecutionMode, PipelineStage)
- Basic CLI structure with clap
- Project structure and module organization
- Cross-compilation infrastructure
- Build scripts for static linking
- Development dependencies setup

#### Modules
- `cli/` - Command-line interface parsing and handling
- `config/` - Configuration management (TOML-based)
- `agent/` - Agent execution and management
- `pipeline/` - Task pipeline orchestration
- `tasks/` - Individual task definitions
- `git/` - Git operations wrapper
- `memory/` - Persistent memory/context management
- `logging/` - Structured logging with tracing
- `progress/` - Progress bars and user feedback
- `testing/` - Test framework detection
- `validate/` - Validation utilities
- `workspace/` - Workspace state management
- `telemetry/` - Analytics and telemetry
- `mcp/` - MCP protocol support
- `interactive/` - Interactive clarification
- `output/` - Output formatting
- `man/` - Man page generation
- `completions/` - Shell completions
- `dryrun/` - Dry-run mode support
- `feature/` - Feature flags

### Changed
- Migrated from Python to Rust for better performance
- Rewritten from longtime.py baseline

### Notes
- This is the initial release marking the completion of the Rust rewrite
- Feature parity with Python baseline plus significant enhancements
- Ready for production use with comprehensive testing

---

## Version Numbering

This project follows [Semantic Versioning](https://semver.org/):

- **MAJOR version (X)**: Incompatible API changes
- **MINOR version (Y)**: Backwards-compatible new features
- **PATCH version (Z)**: Backwards-compatible bug fixes

### Version Upgrade Guidelines

**MAJOR (X.0.0):**
- Breaking changes to CLI arguments
- Removal of public APIs
- Changes to configuration file format
- Incompatible pipeline stage modifications

**MINOR (0.Y.0):**
- New CLI flags/options
- New pipeline stages
- New agent backends
- New configuration options
- Performance improvements

**PATCH (0.0.Z):**
- Bug fixes
- Documentation updates
- Internal refactoring
- Test improvements

---

## Breaking Changes Policy

Breaking changes require:
1. **Major version bump** (X.0.0)
2. **Migration guide** in release notes
3. **Deprecation warning** in previous minor release (if possible)
4. **Update to documentation** reflecting changes

### What Constitutes a Breaking Change?

**CLI Breaking Changes:**
- Removing or renaming CLI flags
- Changing default values that affect behavior
- Changing command syntax

**Configuration Breaking Changes:**
- Removing configuration options
- Changing configuration file format
- Changing default values

**API Breaking Changes:**
- Removing public functions/structs
- Changing function signatures
- Changing trait definitions

**Pipeline Breaking Changes:**
- Removing pipeline stages
- Changing stage execution order
- Modifying task execution behavior

---

## Changelog Maintenance

### When to Update

Update the changelog when:
1. **Merging PRs** - Add entry to "Unreleased" section
2. **Before release** - Move "Unreleased" to new version section
3. **After release** - Clear "Unreleased" section

### How to Categorize Changes

- **Added**: New features, new CLI flags, new modules
- **Changed**: Improvements to existing features, behavior changes
- **Deprecated**: Features planned for removal (include version)
- **Removed**: Features removed in this release
- **Fixed**: Bug fixes, error handling improvements
- **Security**: Security patches, vulnerability fixes

### Commit Message Format

Use conventional commits for better changelog generation:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature (→ Added)
- `change`: Behavior change (→ Changed)
- `deprecate`: Feature deprecation (→ Deprecated)
- `remove`: Feature removal (→ Removed)
- `fix`: Bug fix (→ Fixed)
- `security`: Security fix (→ Security)
- `docs`: Documentation (→ Changed)
- `refactor`: Code refactoring (no changelog entry needed)
- `test`: Test improvements (no changelog entry needed)
- `chore`: Maintenance tasks (no changelog entry needed)

**Scopes:**
- `cli`: Command-line interface
- `config`: Configuration system
- `agent`: Agent backends
- `pipeline`: Pipeline stages
- `tasks`: Task management
- `git`: Git operations
- `memory`: Memory system
- `logging`: Logging system
- `progress`: Progress reporting
- `testing`: Test framework
- `telemetry`: Analytics
- `docs`: Documentation

---

## Release Notes Template

Use this template for future releases:

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- New features

### Changed
- Changes to existing features

### Deprecated
- Features to be removed in future releases

### Removed
- Features removed in this release

### Fixed
- Bug fixes

### Security
- Security improvements and vulnerability fixes

### Migration Guide (for breaking changes)
- Step-by-step instructions for upgrading
```

---

## Automated Changelog Generation

The release process includes automated changelog generation:

1. **Parse commit messages** since last tag
2. **Categorize by type** (feat, fix, etc.)
3. **Group by scope** (cli, config, pipeline, etc.)
4. **Generate changelog section**
5. **Update CHANGELOG.md**
6. **Commit and tag release**

See [RELEASING.md](RELEASING.md) for automation details.