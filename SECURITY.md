# Security Policy

This document outlines the security practices and policies for the ltmatrix project.

## Table of Contents

- [Reporting Security Issues](#reporting-security-issues)
- [Security Architecture](#security-architecture)
- [Dependency Management](#dependency-management)
- [Secure Coding Practices](#secure-coding-practices)
- [Input Validation](#input-validation)
- [Command Execution Security](#command-execution-security)
- [Unsafe Code Guidelines](#unsafe-code-guidelines)
- [Security Audits](#security-audits)

## Reporting Security Issues

**Do not report security vulnerabilities through public GitHub issues.**

If you discover a security vulnerability, please report it by emailing:
- **Security Contact**: security@example.com (replace with actual contact)

Please include the following information:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if available)

We will respond within 48 hours and provide a timeline for the fix.

## Security Architecture

### Threat Model

ltmatrix is a development tool that executes external commands and manages code.
The primary security considerations are:

1. **Command Injection**: User input must be sanitized before use in shell commands
2. **Path Traversal**: File paths must be validated to prevent directory traversal attacks
3. **Credential Exposure**: API keys and tokens must be protected
4. **Supply Chain Security**: Dependencies must be audited for vulnerabilities

### Trust Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                     User Input (Untrusted)                   │
│  - CLI arguments, config files, prompts, environment vars   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Validation Layer                          │
│  - Input sanitization, path canonicalization, type checking │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Core Application (Trusted)                │
│  - Task execution, git operations, agent orchestration      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    External Systems                          │
│  - Git, Claude CLI, test frameworks, file system            │
└─────────────────────────────────────────────────────────────┘
```

## Dependency Management

### Automated Checks

We use the following tools to maintain dependency security:

1. **cargo-audit**: Checks for known vulnerabilities in dependencies
   ```bash
   cargo audit
   ```

2. **cargo-deny**: Comprehensive dependency linting
   ```bash
   cargo deny check
   ```

### Dependency Policy

- All dependencies must have OSI-approved licenses
- No GPL-family licenses allowed (MIT/Apache-2.0 preferred)
- Dependencies are pinned to specific versions
- Security advisories are checked before every release
- Duplicate dependencies are minimized

### Updating Dependencies

When updating dependencies:

1. Run `cargo audit` before and after updates
2. Review the changelog for breaking changes
3. Run the full test suite
4. Check for new transitive dependencies

## Secure Coding Practices

### General Principles

1. **Defense in Depth**: Multiple layers of validation
2. **Least Privilege**: Run with minimal required permissions
3. **Secure by Default**: Default configurations should be secure
4. **Fail Safely**: Errors should not expose sensitive information

### Memory Safety

Rust provides memory safety guarantees, but we still follow best practices:

1. Minimize use of `unsafe` code
2. Document all `unsafe` blocks with safety invariants
3. Use safe abstractions whenever possible
4. No unchecked indexing on untrusted input

### Error Handling

- Use `anyhow` for error propagation
- Never include sensitive data in error messages
- Log errors with appropriate detail level
- Use structured logging for audit trails

## Input Validation

### User Input Sources

All input from these sources must be validated:

- Command-line arguments
- Configuration files (TOML, JSON)
- Environment variables
- File system paths
- API responses

### Validation Rules

#### File Paths

```rust
use std::path::{Path, PathBuf, Component};

fn sanitize_path(path: &Path, base: &Path) -> Result<PathBuf, std::io::Error> {
    // Canonicalize to resolve symlinks and ..
    let canonical = path.canonicalize()?;

    // Ensure the path is within the base directory
    if !canonical.starts_with(base) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Path traversal attempt blocked"
        ));
    }

    Ok(canonical)
}
```

#### Command Arguments

```rust
fn validate_command_arg(arg: &str) -> Result<String, &'static str> {
    // Reject arguments that look like command injection attempts
    if arg.contains(&['|', '&', ';', '$', '`', '\n', '\r'][..]) {
        return Err("Invalid characters in argument");
    }
    Ok(arg.to_string())
}
```

#### Identifiers (Task IDs, Branch Names, etc.)

```rust
fn validate_identifier(id: &str) -> Result<String, &'static str> {
    // Only allow alphanumeric, dash, underscore
    if !id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid identifier format");
    }
    if id.is_empty() || id.len() > 256 {
        return Err("Invalid identifier length");
    }
    Ok(id.to_string())
}
```

## Command Execution Security

### Safe Command Execution Pattern

Always use array-style arguments, never string concatenation:

```rust
// CORRECT - Arguments passed as array
let output = Command::new("git")
    .args(["commit", "-m", message])  // Each argument is separate
    .output()?;

// WRONG - Shell injection possible
let output = Command::new("sh")
    .args(["-c", &format!("git commit -m '{}'", message)])  // DANGEROUS!
    .output()?;
```

### Current Implementation

The codebase follows safe patterns:

- `src/agent/claude.rs`: Uses `Command::new().args()` with separate arguments
- `src/release/git_ops.rs`: All git commands use array-style arguments
- `src/pipeline/test.rs`: Test commands use hardcoded framework commands

### Command Allowlisting

External commands are limited to:

| Command | Purpose | Allowed Arguments |
|---------|---------|-------------------|
| `git` | Version control | Standard git subcommands |
| `claude` | AI agent | `--prompt`, `--model`, `--version` |
| `cargo` | Rust toolchain | `test`, `build`, `clippy` |
| `pytest` | Python testing | `-v`, `--tb=short` |
| `npm` | Node.js package manager | `test` |
| `go` | Go toolchain | `test` |

## Unsafe Code Guidelines

### Current Unsafe Code

The project contains minimal `unsafe` code, documented below:

#### 1. LogGuardWrapper (src/main.rs:640-641)

```rust
unsafe impl Send for LogGuardWrapper {}
unsafe impl Sync for LogGuardWrapper {}
```

**Justification**: `LogGuard` contains a `WorkerGuard` from `tracing-appender` which is
actually thread-safe but doesn't implement `Send + Sync`. This wrapper allows storing
the guard in a global static. The guard is never accessed after initialization.

**Safety Invariant**: The guard is only stored once during startup and never moved or
accessed from multiple threads.

#### 2. COLOR_CONFIG (src/logging/formatter.rs:24-26)

```rust
static mut COLOR_CONFIG: Option<ColorConfig> = None;

pub fn init_color_config(config: ColorConfig) {
    unsafe {
        COLOR_CONFIG = Some(config);
    }
}
```

**Justification**: This is a write-once configuration pattern. The color config is set
once during initialization and only read afterwards.

**Safety Invariant**: `init_color_config` must only be called once during startup.

### When to Use Unsafe

Unsafe code may be used when:

1. Interfacing with external C libraries (via FFI)
2. Implementing thread-safe primitives
3. Performance-critical paths where the borrow checker is too restrictive
4. Interacting with `static mut` for global configuration

### Requirements for Unsafe Code

1. **Document the invariant**: Explain why the unsafe code is sound
2. **Minimize scope**: Keep unsafe blocks as small as possible
3. **Encapsulate**: Wrap unsafe code in safe public APIs
4. **Review**: All unsafe code must be reviewed by a second developer

## Security Audits

### Self-Audit Checklist

Before each release, verify:

- [ ] `cargo audit` reports no vulnerabilities
- [ ] `cargo deny check` passes all checks
- [ ] No new `unsafe` code without documented justification
- [ ] All user inputs are validated
- [ ] No credentials in logs or error messages
- [ ] Dependencies are up to date

### External Audits

For major releases (1.0, 2.0, etc.), consider:

1. Third-party security review
2. Dependency audit by security team
3. Penetration testing for CLI interfaces

## Security Headers

### File Permissions

- Configuration files: `0600` (owner read/write only)
- Log files: `0644` (owner read/write, group read)
- Cache files: `0644`

### Credential Handling

API keys and tokens are:

1. Never logged (even at TRACE level)
2. Read from environment variables or secure config
3. Passed via stdin or environment to child processes
4. Not persisted in configuration files

## Incident Response

If a security vulnerability is discovered:

1. **Triage**: Assess severity and impact
2. **Fix**: Develop and test a fix
3. **Release**: Publish a new version with the fix
4. **Notify**: Update the security advisory
5. **Document**: Add lessons learned to this document

## Security Contacts

- **Maintainer**: @bigfish
- **Security Email**: security@example.com

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-03-07 | Initial security policy |