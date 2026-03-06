# Linux Build Verification Guide

This guide explains how to verify that Linux builds of ltmatrix work correctly.

## Overview

The `ltmatrix` project includes automated tests and scripts to verify Linux builds for:
- `x86_64-unknown-linux-musl` (Intel/AMD 64-bit, static)
- `aarch64-unknown-linux-musl` (ARM64, static)

## Prerequisites

### For Native Linux Builds

On a Linux system (x86_64 or ARM64):
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install cross-compilation tools (if building for different architecture)
sudo apt install gcc-aarch64-linux-gnu  # For ARM64 on x86_64
```

### For Cross-Compilation from Other Platforms

#### Option 1: Using cross-rs (Recommended)
```bash
cargo install cross
```

#### Option 2: Using cargo-zigbuild
```bash
# Install Zig
# On macOS
brew install zig

# On Linux
# Download from https://ziglang.org/download/

cargo install cargo-zigbuild
```

## Building Linux Targets

### Build for x86_64 Linux
```bash
# Using native cargo on Linux
cargo build --release --target x86_64-unknown-linux-musl

# Using cross from any platform
cross build --release --target x86_64-unknown-linux-musl

# Using cargo-zigbuild
cargo zigbuild --release --target x86_64-unknown-linux-musl
```

### Build for ARM64 Linux
```bash
# Using cross
cross build --release --target aarch64-unknown-linux-musl

# Using cargo-zigbuild
cargo zigbuild --release --target aarch64-unknown-linux-musl
```

### Build All Linux Targets
```bash
./scripts/build-all.sh --linux
```

## Verification Methods

### Method 1: Rust Integration Tests

Run the Rust integration test suite:

```bash
# Set the target you want to test
export TARGET=x86_64-unknown-linux-musl

# Or for ARM64
export TARGET=aarch64-unknown-linux-musl

# Run tests
cargo test --test linux_build_verification
```

**What it checks:**
- ✓ Binary exists and is executable
- ✓ `--version` flag works
- ✓ `--help` flag works
- ✓ Static linking (minimal dependencies)
- ✓ No crashes on basic commands
- ✓ Reasonable binary size
- ✓ Proper file permissions

### Method 2: Bash Verification Script

The bash script provides more comprehensive checks and better output formatting:

```bash
# Verify x86_64 build
./scripts/verify-linux-build.sh

# Verify specific target
./scripts/verify-linux-build.sh --target x86_64-unknown-linux-musl
./scripts/verify-linux-build.sh --target aarch64-unknown-linux-musl
```

**What it checks:**
- All checks from Method 1, plus:
- ✓ File type verification (ELF executable)
- ✓ Architecture matches target
- ✓ Symbol information (stripped/unstripped)
- ✓ Detailed dependency analysis
- ✓ Color-coded output with summary

### Method 3: Manual Verification

Quick manual checks:

```bash
# Check binary exists
ls -lh target/x86_64-unknown-linux-musl/release/ltmatrix

# Check version
./target/x86_64-unknown-linux-musl/release/ltmatrix --version

# Check help
./target/x86_64-unknown-linux-musl/release/ltmatrix --help

# Check dependencies (should show "not a dynamic executable" for musl)
ldd target/x86_64-unknown-linux-musl/release/ltmatrix

# Check file type
file target/x86_64-unknown-linux-musl/release/ltmatrix

# Check binary size
du -h target/x86_64-unknown-linux-musl/release/ltmatrix
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Build and Test Linux

on: [push, pull_request]

jobs:
  test-linux:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - x86_64-unknown-linux-musl
          - aarch64-unknown-linux-musl

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: ${{ matrix.target }}
          override: true

      - name: Install cross
        run: cargo install cross

      - name: Build
        run: cross build --release --target ${{ matrix.target }}

      - name: Verify build
        run: ./scripts/verify-linux-build.sh --target ${{ matrix.target }}

      - name: Run integration tests
        run: cargo test --test linux_build_verification
        env:
          TARGET: ${{ matrix.target }}
```

### GitLab CI Example

```yaml
linux-build:
  image: rust:latest
  parallel:
    matrix:
      - TARGET: [x86_64-unknown-linux-musl, aarch64-unknown-linux-musl]
  script:
    - cargo install cross
    - cross build --release --target $TARGET
    - ./scripts/verify-linux-build.sh --target $TARGET
  artifacts:
    paths:
      - target/$TARGET/release/ltmatrix
    expire_in: 1 week
```

## Troubleshooting

### Build Fails with "linker not found"

**Problem:** Missing cross-compilation toolchain

**Solution:**
```bash
# Use cross-rs instead
cargo install cross
cross build --release --target <target>
```

### Binary Crashes with "cannot execute binary file"

**Problem:** Building for wrong architecture

**Solution:**
- Verify your host architecture matches the target
- Use `cross` or `cargo-zigbuild` for proper cross-compilation
- Check with: `file target/<target>/release/ltmatrix`

### LDD Shows Many Dependencies

**Problem:** Binary not statically linked

**Solution:**
```bash
# Ensure using musl target (not gnu)
cargo build --release --target x86_64-unknown-linux-musl

# Ensure static feature is enabled
cargo build --release --target x86_64-unknown-linux-musl --features static
```

### Tests Fail on Windows/macOS

**Problem:** Tests require Linux to execute

**Solution:**
- Run tests in Linux environment (WSL2, Docker, or CI)
- Or skip tests on non-Linux platforms

## Static Linking Verification

A properly statically linked binary for musl should show:

```bash
$ ldd target/x86_64-unknown-linux-musl/release/ltmatrix
  not a dynamic executable
```

If it shows dependencies, check:
1. You're using `musl` target (not `gnu`)
2. `static` feature is enabled
3. Dependencies use `vendored` features where available

## Performance Considerations

### Binary Size

Expected sizes for release builds:
- **Minimal:** ~3-5 MB
- **Typical:** ~8-15 MB
- **Large:** ~20-50 MB (with all features)

To reduce size:
```bash
# Already configured in Cargo.toml
[profile.release]
opt-level = "z"  # Optimize for size
lto = true       # Link-time optimization
strip = true     # Remove debug symbols
```

### Build Time

Typical build times (first build, cold cache):
- **Native (x86_64 on x86_64):** 2-5 minutes
- **Cross (ARM64 on x86_64):** 5-15 minutes

Subsequent builds will be much faster due to caching.

## Additional Resources

- [Cross-Compilation Guide](https://rust-lang.github.io/rustup/cross-compilation.html)
- [cross-rs Documentation](https://github.com/cross-rs/cross)
- [cargo-zigbuild Documentation](https://github.com/rust-cross/cargo-zigbuild)
- [Static Linking in Rust](https://doc.rust-lang.org/1.66.0/rustc/link-args.html)
