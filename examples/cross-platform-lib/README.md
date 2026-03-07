# Cross-Platform Library Example

This example demonstrates building a cross-platform Rust library that works on Windows, Linux, and macOS.

## Purpose

Learn how ltmatrix handles cross-platform considerations, platform-specific code paths, and CI/CD for multiple platforms.

## Command

```bash
ltmatrix "Create a cross-platform file system watcher library that works on Windows, Linux, and macOS. Include platform-specific optimizations."
```

## Expected Behavior

### 1. Generate Stage
The agent generates tasks with platform considerations:

```json
{
  "tasks": [
    {
      "id": "task-001",
      "title": "Define cross-platform trait",
      "description": "Create common Watcher trait for all platforms",
      "status": "pending"
    },
    {
      "id": "task-002",
      "title": "Implement Linux watcher (inotify)",
      "description": "Linux-specific implementation using inotify",
      "status": "pending",
      "target_platform": ["linux"]
    },
    {
      "id": "task-003",
      "title": "Implement macOS watcher (FSEvents)",
      "description": "macOS-specific implementation using FSEvents",
      "status": "pending",
      "target_platform": ["macos"]
    },
    {
      "id": "task-004",
      "title": "Implement Windows watcher (ReadDirectoryChanges)",
      "description": "Windows-specific implementation",
      "status": "pending",
      "target_platform": ["windows"]
    },
    {
      "id": "task-005",
      "title": "Create platform detection",
      "description": "Runtime platform detection and fallback",
      "status": "pending"
    }
  ]
}
```

### 2. Platform-Specific Code Generation

The agent uses conditional compilation:

```rust
// src/lib.rs
mod watcher;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux::InotifyWatcher as PlatformWatcher;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos::FSEventsWatcher as PlatformWatcher;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows::ReadDirectoryChangesWatcher as PlatformWatcher;

pub use watcher::Watcher;

/// Creates a new platform-specific watcher
pub fn new_watcher() -> Box<dyn Watcher> {
    Box::new(PlatformWatcher::new())
}
```

## Expected Output

```
$ ltmatrix "Create a cross-platform file system watcher library..."

Pipeline: Generate → Assess → Execute → Test → Verify → Commit → Memory
Mode: Standard

[Generate] Analyzing goal...
  ✓ Generated 5 tasks
  ✓ Detected platform-specific requirements

[Assess] Evaluating task complexity...
  ✓ task-001: Simple (trait definition)
  ✓ task-002: Complex (Linux inotify)
  ✓ task-003: Complex (macOS FSEvents)
  ✓ task-004: Complex (Windows API)
  ✓ task-005: Medium (platform detection)

[Execute] Implementing tasks...
  [task-001] Define cross-platform trait...
    ✓ Created src/watcher.rs
    ✓ Defined Watcher trait

  [task-002] Implement Linux watcher (inotify)...
    ✓ Created src/linux.rs
    ✓ Added inotify dependency (target: linux)

  [task-003] Implement macOS watcher (FSEvents)...
    ✓ Created src/macos.rs
    ✓ Added core-foundation dependency (target: macos)

  [task-004] Implement Windows watcher...
    ✓ Created src/windows.rs
    ✓ Added winapi dependency (target: windows)

  [task-005] Create platform detection...
    ✓ Created src/detect.rs
    ✓ Implemented runtime fallback

[Test] Running tests...
  [Linux] Running cargo test --target x86_64-unknown-linux-gnu...
    ✓ Unit tests passed (8/8)
  [Windows] Running cargo test --target x86_64-pc-windows-msvc...
    ✓ Unit tests passed (8/8)
  [macOS] Running cargo test --target x86_64-apple-darwin...
    ✓ Unit tests passed (8/8)

[Verify] Reviewing completion...
  ✓ All platform builds succeed
  ✓ Cross-platform tests pass
  ✓ Documentation generated

[Commit] Creating git commit...
  ✓ Committed: feat: Add cross-platform file watcher

[Memory] Recording decisions...
  ✓ Updated .claude/memory.md with platform notes

Summary: 5 tasks completed in 6 minutes
```

## Cargo.toml with Platform Dependencies

```toml
[package]
name = "fs-watcher"
version = "0.1.0"
edition = "2021"

[dependencies]
thiserror = "1.0"

[target.'cfg(target_os = "linux")'.dependencies]
inotify = "0.10"

[target.'cfg(target_os = "macos")'.dependencies]
core-foundation = "0.9"
fsevent-sys = "4.0"

[target.'cfg(target_os = "windows")'.dependencies]
winapi = { version = "0.3", features = ["winbase", "fileapi"] }

[dev-dependencies]
tempfile = "3.0"
tokio = { version = "1.0", features = ["full"] }
```

## CI/CD for Multiple Platforms

Generated `.github/workflows/ci.yml`:

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --all-features
      - run: cargo build --release
```

## Files Generated

```
fs-watcher/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── watcher.rs          # Cross-platform trait
│   ├── detect.rs           # Platform detection
│   ├── linux.rs            # Linux implementation
│   ├── macos.rs            # macOS implementation
│   └── windows.rs          # Windows implementation
├── tests/
│   └── integration_test.rs
├── .github/
│   └── workflows/
│       └── ci.yml          # Multi-platform CI
└── docs/
    └── platforms.md        # Platform-specific docs
```

## Cross-Platform Testing

Test on all platforms:

```bash
# Build for all targets
cargo build --target x86_64-unknown-linux-gnu
cargo build --target x86_64-pc-windows-msvc
cargo build --target x86_64-apple-darwin

# Run tests with cross (if cross-compiling)
cross test --target x86_64-unknown-linux-gnu
cross test --target x86_64-pc-windows-msvc
```

## Configuration

```toml
# .ltmatrix/config.toml
[platform]
targets = ["linux", "macos", "windows"]
default_target = "current"

[build]
cross_compile = true
test_all_targets = true
```

## Platform-Specific Notes

### Linux
- Uses inotify for efficient file watching
- Requires `inotify-tools` on some distros for testing

### macOS
- Uses FSEvents API for native integration
- Supports recursive watching out of the box

### Windows
- Uses `ReadDirectoryChangesW` API
- Handles long paths with `\\?\` prefix