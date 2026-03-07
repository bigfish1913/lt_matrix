# Cross-Platform CLI Example

An example demonstrating cross-platform CLI tool development with ltmatrix.

## Purpose

This example shows how to use ltmatrix to:
- Build a cross-platform command-line tool
- Handle platform-specific code paths
- Create cross-platform tests
- Generate platform-appropriate distributions
- Ensure consistent behavior across OSes

## Project Structure

```
cross-platform-cli/
├── README.md
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── args.rs          # Argument parsing
│   │   └── commands/        # Command implementations
│   ├── platform/
│   │   ├── mod.rs
│   │   ├── unix.rs          # Unix-specific code
│   │   ├── windows.rs       # Windows-specific code
│   │   └── common.rs        # Shared platform code
│   └── utils/
│       └── mod.rs
├── tests/
│   ├── cli_test.rs          # CLI tests
│   └── platform_test.rs     # Platform-specific tests
├── .github/
│   └── workflows/
│       └── release.yml      # Cross-platform release CI
└── Cargo.toml
```

## Usage

### Basic CLI Development

```bash
# Navigate to this example
cd examples/cross-platform-cli

# Run ltmatrix to build the CLI
ltmatrix "Build a file processing CLI tool that works on Windows, macOS, and Linux"
```

### Platform-Specific Features

```bash
# Specify target platforms
ltmatrix --target "x86_64-pc-windows-msvc,x86_64-unknown-linux-gnu,aarch64-apple-darwin" \
         "Build a CLI tool..."
```

## Expected Output

### Console Output

```
ltmatrix v0.1.0 - Long-Time Agent Network

Goal: Build a file processing CLI tool that works on Windows, macOS, and Linux

Phase 1: Generate
  ✓ Generated 7 tasks
  ✓ Identified 2 platform-specific tasks

Phase 2: Assess
  ✓ Task complexity analysis
    - 1 simple task
    - 4 moderate tasks
    - 2 complex tasks (platform abstraction)

Phase 3: Execute
  ✓ [1/7] Setup CLI project with clap
  ✓ [2/7] Implement core file operations
  ✓ [3/7] Create Unix platform module
     - Symlink handling
     - Permission management
     - Signal handling
  ✓ [4/7] Create Windows platform module
     - Junction points
     - ACL handling
     - Console API
  ✓ [5/7] Implement cross-platform abstractions
  ✓ [6/7] Write cross-platform tests
  ✓ [7/7] Add CI/CD for multi-platform builds

Phase 4: Test
  Running tests on current platform (windows)...
    ✓ test_cli_args
    ✓ test_file_operations
    ✓ test_platform_detection
    ✓ test_path_handling
    ✓ test_permission_checks

  Simulating Unix tests...
    ✓ test_unix_symlinks (simulated)
    ✓ test_unix_permissions (simulated)

  All tests passed!

Phase 5: Verify
  ✓ CLI works on current platform
  ✓ Cross-platform paths handled correctly
  ✓ Platform detection accurate

Phase 6: Commit
  ✓ 7 commits created

Phase 7: Release Preparation
  ✓ Release workflow generated
  ✓ Binary targets configured

Summary
  Tasks completed: 7/7
  Platforms supported: 3
  Time elapsed: 6m 20s
  Status: SUCCESS
```

## Generated CLI

```bash
# Build and run the CLI
cargo build --release

# Show help
./target/release/fileproc --help

# Process files
./target/release/fileproc process input.txt output.txt

# With options
./target/release/fileproc process input.txt output.txt \
    --format json \
    --compress \
    --verbose
```

## Platform-Specific Code

### Unix Module (src/platform/unix.rs)

```rust
#[cfg(unix)]
pub fn create_symlink(src: &Path, dst: &Path) -> Result<()> {
    std::os::unix::fs::symlink(src, dst)?;
    Ok(())
}

#[cfg(unix)]
pub fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}
```

### Windows Module (src/platform/windows.rs)

```rust
#[cfg(windows)]
pub fn create_symlink(src: &Path, dst: &Path) -> Result<()> {
    // Use junction points for directories on Windows
    if src.is_dir() {
        std::os::windows::fs::symlink_dir(src, dst)?;
    } else {
        std::os::windows::fs::symlink_file(src, dst)?;
    }
    Ok(())
}

#[cfg(windows)]
pub fn set_executable(path: &Path) -> Result<()> {
    // On Windows, executability is determined by extension
    // No action needed
    Ok(())
}
```

### Cross-Platform Abstraction (src/platform/mod.rs)

```rust
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

// Common functions work on all platforms
pub fn normalize_path(path: &Path) -> PathBuf {
    // Cross-platform path normalization
}
```

## Cross-Platform Tests

```rust
#[test]
fn test_path_handling() {
    // Test that works on all platforms
    let path = Path::new("some/path/file.txt");
    assert!(path.is_relative());
}

#[cfg(unix)]
#[test]
fn test_unix_symlinks() {
    // Unix-only test
}

#[cfg(windows)]
#[test]
fn test_windows_paths() {
    // Windows-only test
}
```

## CI/CD Configuration

The generated `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            ext: .exe
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            ext: ''
          - target: aarch64-apple-darwin
            os: macos-latest
            ext: ''

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          target: ${{ matrix.target }}
      - uses: actions-rs/cargo@v1
        with:
          command: build
          args: --release --target ${{ matrix.target }}
      - uses: softprops/action-gh-release@v1
        with:
          files: target/${{ matrix.target }}/release/fileproc${{ matrix.ext }}
```

## Key Concepts

### Platform Detection

```rust
#[cfg(target_os = "windows")]
const PLATFORM: &str = "windows";

#[cfg(target_os = "linux")]
const PLATFORM: &str = "linux";

#[cfg(target_os = "macos")]
const PLATFORM: &str = "macos";
```

### Conditional Compilation

```rust
// Code that only compiles on Unix
#[cfg(unix)]
fn unix_specific() { /* ... */ }

// Code that only compiles on Windows
#[cfg(windows)]
fn windows_specific() { /* ... */ }

// Code for either Unix or Windows (not other platforms)
#[cfg(any(unix, windows))]
fn unix_or_windows() { /* ... */ }
```

### Path Handling

```rust
use std::path::Path;

// Always use Path/PathBuf, never string concatenation
fn join_paths(base: &Path, name: &str) -> PathBuf {
    base.join(name) // Works correctly on all platforms
}
```

## Testing Strategy

```bash
# Run all tests
cargo test

# Run only platform-specific tests
cargo test --features platform-tests

# Run tests for specific platform
cargo test --target x86_64-pc-windows-msvc
```

## Build Commands

```bash
# Build for current platform
cargo build --release

# Build for specific target
cargo build --release --target x86_64-unknown-linux-gnu

# Build for all targets (requires cross-compilation setup)
cargo build --release --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target aarch64-apple-darwin
```

## Next Steps

- Review the [Tutorial](../../docs/tutorial.md) for a complete walkthrough
- See [multi-task-project](../multi-task-project/) for dependency management
- Check [web-api-testing](../web-api-testing/) for web development
