# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build, Test, and Lint Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Release build

# Test
cargo test --all-features      # Run all tests
cargo test --lib               # Run unit tests only
cargo test --test '*'          # Run integration tests only
cargo test <test_name>         # Run specific test

# Code Quality
cargo fmt                      # Format code
cargo fmt --check              # Check formatting
cargo clippy --all-targets --all-features -- -D warnings  # Lint

# All-in-one check
make check                     # Runs fmt-check, clippy, and test

# Documentation
cargo doc --no-deps            # Build docs
```

## Architecture Overview

ltmatrix is a long-time agent orchestrator written in Rust. It coordinates AI agents through a multi-stage pipeline to complete complex software engineering tasks.

### Layered Architecture

```
CLI Layer (main.rs, src/cli/)
    ↓
Configuration Layer (src/config/)
    ↓
Orchestration Layer (src/pipeline/orchestrator.rs)
    ↓
Stage Layer (src/pipeline/*.rs - 6 stages)
    ↓
Agent Layer (src/agent/)
    ↓
Infrastructure Layer (src/git/, src/mcp/, src/workspace/, src/memory/)
```

### Key Modules

| Module | Purpose |
|--------|---------|
| `src/models/` | Core data structures: `Task`, `Agent`, `ExecutionMode`, `PipelineStage` |
| `src/agent/` | Agent backend trait and implementations (Claude, OpenCode, KimiCode, Codex) |
| `src/pipeline/` | 6-stage pipeline: Generate → Assess → Execute → Test → Verify → Commit |
| `src/tasks/` | Task scheduling, dependency graph, parallel execution |
| `src/config/` | Configuration loading with precedence: CLI > Project > Global > Defaults |
| `src/git/` | Git operations (branching, commits, merging) |
| `src/mcp/` | Model Context Protocol client for external tool integration |
| `src/workspace/` | State persistence in `.ltmatrix/tasks-manifest.json` |

### Key Patterns

- **AgentBackend trait** (`src/agent/backend.rs`): Strategy pattern for agent implementations. All backends implement this trait.
- **AgentFactory** (`src/agent/factory.rs`): Creates agent backends by name.
- **Async/Await**: All I/O uses async with tokio runtime. Use `#[tokio::test]` for async tests.
- **Error Handling**: `anyhow::Result` for application errors, custom error types for domain-specific errors.

### Configuration Precedence (Highest to Lowest)

1. CLI Arguments
2. Project Config (`.ltmatrix/config.toml`)
3. Global Config (`~/.config/ltmatrix/config.toml`)
4. Defaults

## Commit Convention

This project uses Conventional Commits:

```
<type>(<scope>): <description>
```

Valid types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `perf`, `ci`, `build`, `revert`

Examples:
- `feat(cli): add --fast flag for quick execution`
- `fix(agent): handle timeout gracefully`
- `test(pipeline): add integration tests for verify stage`

## Git Hooks

Git hooks are located in `.githooks/`. Install with `make install-hooks`.

| Hook | Runs |
|------|------|
| pre-commit | Format check, clippy, fast unit tests |
| pre-push | Full test suite, release build |
| commit-msg | Conventional commit format validation |

## Test Organization

- Unit tests: Inline in source files using `#[cfg(test)]` modules
- Integration tests: `tests/` directory
- Test fixtures: `tests/fixtures/`
