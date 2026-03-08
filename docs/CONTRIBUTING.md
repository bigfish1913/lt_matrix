# Contributing to ltmatrix

Thank you for your interest in contributing to ltmatrix! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Git Hooks](#git-hooks)
- [Commit Guidelines](#commit-guidelines)
- [Pull Request Process](#pull-request-process)
- [Code Style](#code-style)
- [Testing](#testing)
- [Documentation](#documentation)

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment. Please be considerate of others and follow standard open-source community guidelines.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/ltmatrix.git
   cd ltmatrix
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/bigfish/ltmatrix.git
   ```
4. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Setup

### Prerequisites

- **Rust**: Install via [rustup](https://rustup.rs/) (minimum version: 1.75.0)
- **Git**: For version control
- **Cargo**: Comes with Rust

### Build the Project

```bash
# Debug build (faster compilation, slower execution)
cargo build

# Release build (slower compilation, faster execution)
cargo build --release

# Run the application
cargo run -- --help
```

### Run Tests

```bash
# Run all tests
cargo test

# Run only unit tests (faster)
cargo test --lib

# Run a specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

## Git Hooks

We use Git hooks to ensure code quality before commits and pushes. **Installing the hooks is highly recommended.**

### Installation

**Linux/macOS:**
```bash
./scripts/install-hooks.sh
```

**Windows:**
```cmd
scripts\install-hooks.bat
```

**Manual installation:**
```bash
git config core.hooksPath .githooks
```

### Available Hooks

| Hook | When | What it does |
|------|------|--------------|
| `pre-commit` | Before each commit | Format check, clippy, fast unit tests |
| `pre-push` | Before each push | Full test suite, release build verification |
| `commit-msg` | When writing commit message | Validates conventional commit format |

### Hook Details

#### pre-commit

Runs these checks before allowing a commit:

1. **Format Check** (`cargo fmt --check`)
   - Ensures code follows Rust style guidelines
   - Fix: Run `cargo fmt`

2. **Clippy** (`cargo clippy`)
   - Lints code for common issues and improvements
   - Treats warnings as errors
   - Fix: Run `cargo clippy --fix` for auto-fixes

3. **Fast Tests** (`cargo test --lib`)
   - Runs unit tests only (skips slow integration tests)
   - Only runs if Rust files changed

#### pre-push

Runs comprehensive checks before allowing a push:

1. Format check
2. Clippy
3. **Full Test Suite** (`cargo test`)
   - Includes integration tests
4. **Release Build** (`cargo build --release`)
   - Verifies release build succeeds

#### commit-msg

Validates commit messages follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>
```

**Valid types:**
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation changes
- `style` - Code style changes (formatting, etc.)
- `refactor` - Code refactoring
- `test` - Adding or modifying tests
- `chore` - Maintenance tasks
- `perf` - Performance improvements
- `ci` - CI/CD changes
- `build` - Build system changes
- `revert` - Reverting changes

**Examples:**
```
feat(cli): add --fast flag for quick execution
fix(agent): handle timeout gracefully
docs(readme): update installation instructions
test(pipeline): add integration tests for verify stage
```

### Bypassing Hooks

If you need to bypass hooks temporarily:

```bash
# Skip pre-commit and commit-msg
git commit --no-verify

# Skip pre-push
git push --no-verify
```

**Warning:** Only bypass hooks when absolutely necessary. CI will still run all checks.

### Uninstalling Hooks

**Linux/macOS:**
```bash
./scripts/uninstall-hooks.sh
```

**Manual:**
```bash
git config --unset core.hooksPath
```

## Commit Guidelines

### Conventional Commits

All commit messages must follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Commit Message Format

1. **Subject line** (required):
   - Type and scope in lowercase
   - Imperative mood ("add" not "added")
   - No period at the end
   - Max 72 characters

2. **Body** (optional):
   - Separate from subject with blank line
   - Explain what and why, not how
   - Wrap at 72 characters

3. **Footer** (optional):
   - Breaking changes: `BREAKING CHANGE: description`
   - Close issues: `Closes #123`

### Examples

**Simple commit:**
```
feat(cli): add --expert flag for high-quality execution
```

**Commit with body:**
```
fix(agent): handle session timeout gracefully

Previously, session timeouts would cause the agent to fail without
proper cleanup. Now we gracefully close the session and allow retry
with a new session if needed.

Closes #42
```

**Breaking change:**
```
refactor(api)!: change AgentBackend trait signature

BREAKING CHANGE: The execute method now takes an immutable reference
instead of a mutable one. Implementors will need to update their
implementations.
```

## Pull Request Process

1. **Create a feature branch** from `main`
2. **Make your changes** following code style guidelines
3. **Add tests** for new functionality
4. **Update documentation** if needed
5. **Run all checks locally**:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test
   cargo build --release
   ```
6. **Push to your fork** and create a PR
7. **Wait for CI** to pass
8. **Address review feedback**

### PR Checklist

- [ ] Code follows project style guidelines
- [ ] Tests added/updated for new functionality
- [ ] Documentation updated if needed
- [ ] Commit messages follow conventional format
- [ ] All CI checks pass
- [ ] PR description explains the changes

## Code Style

### Rust Style

- Follow standard Rust formatting (`cargo fmt`)
- Address all clippy warnings (`cargo clippy`)
- Use meaningful variable and function names
- Add documentation comments for public APIs

### Documentation Comments

```rust
/// Brief description of the function.
///
/// More detailed description if needed.
///
/// # Arguments
///
/// * `param` - Description of the parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// Description of possible errors
///
/// # Examples
///
/// ```
/// use ltmatrix::module::function;
/// let result = function(arg);
/// ```
pub fn function(param: Type) -> Result<ReturnType> {
    // ...
}
```

### Module Organization

```
src/
├── module/
│   ├── mod.rs      # Module exports and documentation
│   ├── types.rs    # Type definitions
│   ├── impl.rs     # Core implementation
│   └── tests.rs    # Unit tests (or #[cfg(test)] module)
```

## Testing

### Running Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test '*'

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture

# Single thread (for debugging)
cargo test -- --test-threads=1
```

### Writing Tests

- Place unit tests in the same file using `#[cfg(test)]` module
- Place integration tests in `tests/` directory
- Use descriptive test names: `test_<function>_<scenario>_<expected>`
- Use `#[tokio::test]` for async tests

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_normal_input_returns_expected() {
        // Arrange
        let input = "normal";

        // Act
        let result = function(input);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_invalid_input_returns_error() {
        let result = function("invalid");
        assert!(result.is_err());
    }
}
```

## Documentation

### Building Documentation

```bash
cargo doc --open --no-deps
```

### Documentation Standards

- All public items must have documentation comments
- Include examples in doc comments when applicable
- Keep documentation up-to-date with code changes
- Use markdown formatting in doc comments

## Reporting Issues

### Bug Reports

When reporting bugs, please use our [Bug Report Template](https://github.com/bigfish/ltmatrix/issues/new?template=bug_report.yml) and include:

- **Clear description** of the bug
- **Steps to reproduce** the issue
- **Expected vs actual behavior**
- **Environment details** (OS, Rust version, ltmatrix version)
- **Relevant logs** (run with `--log-level debug` if possible)
- **Minimal configuration** that triggers the bug

<details>
<summary>Bug Report Quick Template</summary>

```markdown
## Description
[What is the bug?]

## Steps to Reproduce
1. [First step]
2. [Second step]
3. [What fails]

## Expected Behavior
[What should happen?]

## Actual Behavior
[What actually happens?]

## Environment
- ltmatrix version: [output of `ltmatrix --version`]
- OS: [e.g., Ubuntu 22.04, Windows 11, macOS 14]
- Rust version: [output of `rustc --version`]

## Logs
```
[Paste relevant log output here]
```

## Configuration
```toml
[Paste relevant config here, removing sensitive data]
```
```

</details>

### Feature Requests

For feature requests, please use our [Feature Request Template](https://github.com/bigfish/ltmatrix/issues/new?template=feature_request.yml) and include:

- **Problem statement**: What problem does this solve?
- **Proposed solution**: How should it work?
- **Alternatives considered**: What other approaches did you think about?
- **Category**: CLI, agent, pipeline, etc.

<details>
<summary>Feature Request Quick Template</summary>

```markdown
## Problem Statement
[What problem are you trying to solve?]

## Proposed Solution
[Describe your proposed solution]

## Alternatives Considered
[What other solutions did you consider?]

## Example Usage
```bash
# How would this feature be used?
ltmatrix --new-flag "your goal"
```

## Category
- [ ] CLI
- [ ] Agent Backend
- [ ] Pipeline
- [ ] Configuration
- [ ] Other: [specify]
```

</details>

### Questions and Discussions

For general questions or discussions:
- Use [GitHub Discussions](https://github.com/bigfish/ltmatrix/discussions) for broader topics
- Use our [Question Template](https://github.com/bigfish/ltmatrix/issues/new?template=question.yml) for specific questions

## Security Issues

**Do not report security vulnerabilities through public GitHub issues.**

Instead, please:
1. Email security concerns to the repository maintainers
2. Include "SECURITY" in the subject line
3. Provide details about the vulnerability
4. Allow time for response before public disclosure

## Getting Help

- Check the [documentation](https://docs.ltmatrix.dev) (if available)
- Search [existing issues](https://github.com/bigfish/ltmatrix/issues) before creating new ones
- Join [discussions](https://github.com/bigfish/ltmatrix/discussions) for community support

Thank you for contributing to ltmatrix!
