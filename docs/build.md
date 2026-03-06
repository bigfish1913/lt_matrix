# Build Guide for ltmatrix

This document explains how to build ltmatrix for different platforms with proper static linking for portable, self-contained binaries.

## Prerequisites

### For cargo-zigbuild (Recommended)

```bash
# Install Zig (required by cargo-zigbuild)
# On Linux/macOS:
curl -fsSL https://ziglang.org/download/0.11.0/zig-linux-x86_64-0.11.0.tar.xz | tar xJ
sudo mv zig-linux-x86_64-0.11.0 /usr/local/zig
export PATH=/usr/local/zig:$PATH

# On macOS via Homebrew:
brew install zig

# Install cargo-zigbuild
cargo install cargo-zigbuild
```

### Traditional Cross-Compilation (Alternative)

```bash
# On Ubuntu/Debian:
sudo apt-get install musl-tools musl-dev gcc-aarch64-linux-gnu

# On macOS (requires cross-compilation toolchains):
brew install musl-cross
```

## Build Commands

### Native Build (Current Platform)

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

### Cross-Compilation with cargo-zigbuild

#### Linux Targets (Static Binaries)

```bash
# Linux x86_64 (static)
cargo zigbuild --release --target x86_64-unknown-linux-musl

# Linux ARM64 (static)
cargo zigbuild --release --target aarch64-unknown-linux-musl
```

#### macOS Targets

```bash
# macOS Intel
cargo zigbuild --release --target x86_64-apple-darwin

# macOS Apple Silicon
cargo zigbuild --release --target aarch64-apple-darwin
```

#### Windows Targets

```bash
# Windows x86_64
cargo zigbuild --release --target x86_64-pc-windows-msvc

# Windows ARM64
cargo zigbuild --release --target aarch64-pc-windows-msvc
```

### Using Cargo Aliases

The `.cargo/config.toml` includes convenient aliases:

```bash
# Build for Linux (static)
cargo build --alias linux-release

# Build for macOS Intel
cargo build --alias macos-release

# Build for macOS Apple Silicon
cargo build --alias macos-arm-release

# Build for Windows
cargo build --alias windows-release
```

Note: Aliases work best when using native toolchains. For cargo-zigbuild, use the full command.

## Understanding Static Linking

### What is Static Linking?

Static linking means all dependencies (including C libraries) are bundled into the final binary. This produces:

- ✅ **Portable binaries** - Run on any compatible Linux distribution
- ✅ **No external dependencies** - No need to install .so files
- ✅ **Simpler deployment** - Single file to distribute
- ⚠️ **Larger binaries** - All library code is included

### musl vs glibc

| Target | libc Type | Binary Type | Portability |
|--------|-----------|-------------|-------------|
| `x86_64-unknown-linux-musl` | musl | Static | Universal Linux |
| `x86_64-unknown-linux-gnu` | glibc | Dynamic | Depends on system |

**Recommendation:** Use musl targets for distribution, glibc for development.

## Dependency Configuration

### Native Dependencies with Static Linking

ltmatrix uses these native dependencies:

| Crate | Native Lib | Static Feature | Status |
|-------|------------|----------------|--------|
| `git2` | libgit2, OpenSSL | `vendored-libgit2`, `vendored-openssl` | ✅ Configured |
| `reqwest` | OpenSSL | `rustls-tls` | ✅ Pure Rust TLS |

All native dependencies are configured for static linking on musl targets.

### Build Script (build.rs)

The `build.rs` file automatically:

1. Detects musl targets and enables static linking
2. Configures libgit2 to use bundled vendored copy
3. Emits build configuration information

No manual intervention needed.

## Verification

### Check if Binary is Statically Linked

```bash
# Check binary dependencies
file target/x86_64-unknown-linux-musl/release/ltmatrix

# Should show: "statically linked" for musl targets

# Verify no dynamic dependencies
ldd target/x86_64-unknown-linux-musl/release/ltmatrix
# Should output: "not a dynamic executable"
```

### Test on Target Platform

```bash
# Copy binary to target system and test
scp target/x86_64-unknown-linux-musl/release/ltmatrix user@target-host:/tmp/
ssh user@target-host /tmp/ltmatrix --version
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Build Release Binaries

on:
  push:
    tags: ['v*']

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
          - target: aarch64-unknown-linux-musl
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Install Zig
        uses: goto-bus-stop/setup-zig@v2

      - name: Install cargo-zigbuild
        run: cargo install cargo-zigbuild

      - name: Build
        run: cargo zigbuild --release --target ${{ matrix.target }}

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ltmatrix-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/ltmatrix*
```

## Troubleshooting

### Common Issues

#### "linker not found" Error

**Problem:** Traditional linker not found for cross-compilation.

**Solution:** Use cargo-zigbuild instead:
```bash
cargo zigbuild --release --target x86_64-unknown-linux-musl
```

#### OpenSSL Headers Missing

**Problem:** Native OpenSSL headers required.

**Solution:** Ensure static features are enabled in Cargo.toml:
```toml
[dependencies]
git2 = { version = "0.19", features = ["vendored-libgit2", "vendored-openssl"] }
```

#### Binary Too Large

**Problem:** Static binaries can be 5-10x larger.

**Solutions:**
1. Use release profile optimizations (already configured in `.cargo/config.toml`)
2. Strip symbols: `strip target/*/release/ltmatrix`
3. Use UPX compression: `upx --best target/*/release/ltmatrix`

### Build Script Debugging

To see what the build script is doing:

```bash
# Show build script output
cargo build --release -vv

# Check environment variables
cargo build --release --target x86_64-unknown-linux-musl | grep "Build configuration"
```

## Release Checklist

Before releasing a new version:

1. ✅ Build for all target platforms
2. ✅ Test static linking: `ldd ltmatrix` (should say "not a dynamic executable")
3. ✅ Verify binary on actual target systems
4. ✅ Check binary size (consider compression if too large)
5. ✅ Run integration tests on each platform
6. ✅ Update version in Cargo.toml
7. ✅ Create git tag and push to trigger release workflow

## Further Reading

- [cargo-zigbuild Documentation](https://github.com/rust-cross/cargo-zigbuild)
- [Zig Cross-Compilation](https://andrewkelley.me/post/zig-c-interop.html)
- [Static Linking in Rust](https://doc.rust-lang.org/1.65.0/reference/linkage.html#static-and-dynamic-c-runtimes)
- [cross: Docker Cross-Compilation](https://github.com/cross-rs/cross)
