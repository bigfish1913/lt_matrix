# Cross-Compilation Guide for ltmatrix

This guide explains how to build `ltmatrix` for all supported target platforms.

## Supported Target Platforms

| Platform | Architecture | Target Triple | Binary Type |
|----------|--------------|---------------|-------------|
| Linux | x86_64 | `x86_64-unknown-linux-musl` | Static |
| Linux | ARM64 | `aarch64-unknown-linux-musl` | Static |
| Windows | x86_64 | `x86_64-pc-windows-msvc` | Static CRT |
| Windows | ARM64 | `aarch64-pc-windows-msvc` | Static CRT |
| macOS | Intel | `x86_64-apple-darwin` | Dynamic |
| macOS | Apple Silicon | `aarch64-apple-darwin` | Dynamic |

## Prerequisites

### Option 1: Using Cross (Recommended for Linux)

```bash
# Install cross (requires Docker)
cargo install cross --git https://github.com/cross-rs/cross
```

### Option 2: Using cargo-zigbuild

```bash
# Install zigbuild
cargo install cargo-zigbuild

# Install Zig (required for zigbuild)
# Download from: https://ziglang.org/download/
```

### Option 3: Native Toolchains

#### Linux (Ubuntu/Debian)
```bash
# For musl targets (static binaries)
sudo apt-get install musl-tools musl-dev

# For ARM64 cross-compilation
sudo apt-get install gcc-aarch64-linux-gnu

# For x86_64 cross-compilation
sudo apt-get install gcc-x86-64-linux-gnu
```

#### macOS
No additional tools required - Xcode Command Line Tools provide everything needed.

#### Windows
Install [Rust GNU toolchain](https://rust-lang.github.io/rustup/installation/other-tools.html) and MSVC Build Tools.

## Building for Different Platforms

### Linux (x86_64) - Static Binary

```bash
# Using cross (recommended)
cross build --release --target x86_64-unknown-linux-musl

# Using cargo-zigbuild
cargo zigbuild --release --target x86_64-unknown-linux-musl

# Using native musl
cargo build --release --target x86_64-unknown-linux-musl
```

Output: `target/x86_64-unknown-linux-musl/release/ltmatrix`

### Linux (ARM64) - Static Binary

```bash
# Using cross (recommended)
cross build --release --target aarch64-unknown-linux-musl

# Using cargo-zigbuild
cargo zigbuild --release --target aarch64-unknown-linux-musl
```

Output: `target/aarch64-unknown-linux-musl/release/ltmatrix`

### Windows (x86_64)

```bash
# From Linux/macOS using cross
cross build --release --target x86_64-pc-windows-msvc

# From Windows with MSVC installed
cargo build --release --target x86_64-pc-windows-msvc
```

Output: `target/x86_64-pc-windows-msvc/release/ltmatrix.exe`

### Windows (ARM64)

```bash
# Using cross
cross build --release --target aarch64-pc-windows-msvc
```

Output: `target/aarch64-pc-windows-msvc/release/ltmatrix.exe`

### macOS (Intel)

```bash
# On macOS with x86_64
cargo build --release --target x86_64-apple-darwin

# Cross-compile from ARM64 macOS
cargo build --release --target x86_64-apple-darwin
```

Output: `target/x86_64-apple-darwin/release/ltmatrix`

### macOS (Apple Silicon)

```bash
# On macOS with Apple Silicon
cargo build --release --target aarch64-apple-darwin

# Cross-compile from Intel macOS
cargo build --release --target aarch64-apple-darwin
```

Output: `target/aarch64-apple-darwin/release/ltmatrix`

### Universal macOS Binary (Intel + Apple Silicon)

```bash
# Build both architectures first
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Create universal binary
lipo -create \
  target/x86_64-apple-darwin/release/ltmatrix \
  target/aarch64-apple-darwin/release/ltmatrix \
  -output target/universal-apple-darwin/release/ltmatrix
```

Output: `target/universal-apple-darwin/release/ltmatrix`

## Using Cargo Aliases

The `.cargo/config.toml` includes helpful aliases:

```bash
# Build for specific platforms
cargo build --alias linux-release
cargo build --alias linux-arm-release
cargo build --alias windows-release
cargo build --alias macos-release
cargo build --alias macos-arm-release
```

## Building All Targets

### Using Cross with Docker

```bash
# Build all targets using cross
for target in \
  x86_64-unknown-linux-musl \
  aarch64-unknown-linux-musl \
  x86_64-pc-windows-msvc \
  aarch64-pc-windows-msvc; do
  cross build --release --target $target
done
```

### Using cargo-zigbuild

```bash
# Build all Linux targets
cargo zigbuild --release --target x86_64-unknown-linux-musl
cargo zigbuild --release --target aarch64-unknown-linux-musl
```

## Verification

After building, verify the binary:

```bash
# Check binary architecture
file target/*/release/ltmatrix

# Check for dynamic dependencies
ldd target/x86_64-unknown-linux-gnu/release/ltmatrix  # Linux
otool -L target/x86_64-apple-darwin/release/ltmatrix  # macOS

# Check static linking (should show "not a dynamic executable")
ldd target/x86_64-unknown-linux-musl/release/ltmatrix

# Check binary size
ls -lh target/*/release/ltmatrix
```

## Optimization Settings

The release profile is configured for optimal size/performance:

- **Opt-level**: `z` (optimize for size)
- **LTO**: Enabled (link-time optimization)
- **Codegen-units**: 1 (maximum optimization)
- **Strip**: Enabled (remove debug symbols)
- **Panic**: `abort` (reduces binary size)

Expected binary sizes:
- Linux (musl): ~8-12 MB
- Windows (MSVC): ~6-10 MB
- macOS: ~10-15 MB

## GitHub Actions Workflow

For automated builds, see `.github/workflows/release.yml` (to be created).

Example workflow steps:

```yaml
- name: Build for Linux x86_64
  run: cross build --release --target x86_64-unknown-linux-musl

- name: Build for macOS ARM64
  run: cargo build --release --target aarch64-apple-darwin

- name: Build for Windows x86_64
  run: cross build --release --target x86_64-pc-windows-msvc
```

## Troubleshooting

### "linker not found" error

Install the appropriate toolchain for your target:

```bash
# Ubuntu/Debian
sudo apt-get install musl-tools gcc-aarch64-linux-gnu

# macOS
xcode-select --install

# Windows
# Install Visual Studio Build Tools
```

### "undefined reference to pthread_create"

Link against pthread:

```bash
RUSTFLAGS="-l pthread" cargo build --release
```

### "fatal error: openssl/sslv3.h: No such file or directory"

Install OpenSSL development headers:

```bash
# Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# macOS
brew install openssl

# Windows (vcpkg)
vcpkg install openssl:x64-windows-static
```

### "error: linker `aarch64-linux-gnu-gcc` not found"

Install the cross-compiler:

```bash
# Ubuntu/Debian
sudo apt-get install gcc-aarch64-linux-gnu
```

## Additional Resources

- [Cross Compilation in The Rust Reference](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
- [cross-rs GitHub](https://github.com/cross-rs/cross)
- [cargo-zigbuild GitHub](https://github.com/rust-cross/cargo-zigbuild)
- [Rust Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html)

## Installation from Source

```bash
# Clone repository
git clone https://github.com/bigfish/ltmatrix.git
cd ltmatrix

# Build for host platform
cargo build --release

# Install locally
cargo install --path .

# Or copy binary directly
cp target/release/ltmatrix ~/.local/bin/
```

## Distribution

Binaries built with musl are self-contained and can be distributed without requiring users to install any runtime dependencies.

For GitHub releases:

1. Build for all target platforms
2. Create release archives:
   ```bash
   tar -czf ltmatrix-linux-x86_64.tar.gz -C target/x86_64-unknown-linux-musl/release ltmatrix
   zip ltmatrix-windows-x86_64.zip target/x86_64-pc-windows-msvc/release/ltmatrix.exe
   ```
3. Upload to GitHub Releases

See [RELEASING.md](RELEASING.md) for complete release procedures.
