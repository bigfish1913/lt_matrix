# macOS Build Guide for ltmatrix

This document provides specific guidance for building ltmatrix on macOS, including native builds, universal binaries, code signing, and distribution practices.

## System Requirements

### Minimum macOS Versions

- **Intel (x86_64)**: macOS 10.13 (High Sierra) or later
- **Apple Silicon (aarch64)**: macOS 11.0 (Big Sur) or later

These minimum versions are configured in `.cargo/config.toml`:

```toml
[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-mmacosx-version-min=10.13"]

[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-mmacosx-version-min=11.0"]
```

## Native macOS Builds

### Prerequisites

1. **Xcode Command Line Tools**
   ```bash
   xcode-select --install
   ```

2. **Rust Toolchain**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   ```

3. **Add Target Architectures** (for cross-architecture builds)
   ```bash
   # For Intel builds on Apple Silicon
   rustup target add x86_64-apple-darwin

   # For Apple Silicon builds on Intel
   rustup target add aarch64-apple-darwin
   ```

### Building for Intel (x86_64)

**On Intel Macs:**
```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Binary location
target/debug/ltmatrix
target/release/ltmatrix
```

**On Apple Silicon Macs (cross-compilation):**
```bash
# Add Intel target
rustup target add x86_64-apple-darwin

# Build for Intel
cargo build --release --target x86_64-apple-darwin

# Binary location
target/x86_64-apple-darwin/release/ltmatrix
```

### Building for Apple Silicon (aarch64)

**On Apple Silicon Macs:**
```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Binary location
target/debug/ltmatrix
target/release/ltmatrix
```

**On Intel Macs (cross-compilation):**
```bash
# Add Apple Silicon target
rustup target add aarch64-apple-darwin

# Build for Apple Silicon
cargo build --release --target aarch64-apple-darwin

# Binary location
target/aarch64-apple-darwin/release/ltmatrix
```

### Build Configuration

The `.cargo/config.toml` file is pre-configured for macOS builds:

```toml
[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-mmacosx-version-min=10.13"]

[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-mmacosx-version-min=11.0"]
```

This ensures compatibility with older macOS versions.

## Universal Binary Creation

A universal binary contains code for both Intel and Apple Silicon architectures in a single executable file.

### Requirements

- Both `x86_64-apple-darwin` and `aarch64-apple-darwin` binaries must exist
- macOS environment (the `lipo` tool is macOS-specific)

### Automated Creation

```bash
# Create universal binary (recommended)
./scripts/create-universal-binary.sh
```

**What the script does:**
1. Verifies both Intel and ARM binaries exist
2. Combines them using `lipo`
3. Verifies both architectures are present
4. Tests binary execution
5. Applies code signing
6. Creates convenience symlink

**Output:** `target/release/ltmatrix-universal`

### Manual Creation

```bash
# Build both architectures
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Create universal binary using lipo
lipo -create \
  target/x86_64-apple-darwin/release/ltmatrix \
  target/aarch64-apple-darwin/release/ltmatrix \
  -output target/release/ltmatrix-universal

# Verify architectures
lipo -info target/release/ltmatrix-universal
# Expected: Architectures in the fat file: ... are: x86_64 arm64
```

### Universal Binary Verification

```bash
# Check format
file target/release/ltmatrix-universal
# Expected: Mach-O universal binary with 2 architectures

# Check architectures
lipo -info target/release/ltmatrix-universal
# Expected: x86_64 arm64

# Test execution
./target/release/ltmatrix-universal --version
./target/release/ltmatrix-universal --help
```

## Code Signing

All macOS binaries should be code signed for proper execution. For development, ad-hoc signing is sufficient.

### Ad-Hoc Signing (Development)

```bash
# Sign with ad-hoc signature (self-signed)
codesign --force --deep --sign - target/release/ltmatrix

# Verify signature
codesign -v target/release/ltmatrix

# Display signature details
codesign -dvv target/release/ltmatrix
```

### Developer ID Signing (Distribution)

For distribution outside the App Store:

```bash
# Sign with Developer ID
codesign --force --deep --sign "Developer ID Application: Your Name (TEAM_ID)" target/release/ltmatrix

# Verify signature
codesign -v target/release/ltmatrix

# Check signing authority
codesign -dvv target/release/ltmatrix | grep Authority
```

### Notarization (Distribution)

For widespread distribution, notarize the binary with Apple:

```bash
# Submit for notarization
xcrun notarytool submit ltmatrix.tar.gz \
  --apple-id "your@email.com" \
  --team-id "TEAM_ID" \
  --password "app-specific-password" \
  --wait

# Staple notarization ticket
xcrun stapler staple target/release/ltmatrix

# Verify notarization
spctl -a -vv target/release/ltmatrix
```

## Verification Steps

### Binary Format Verification

```bash
# Check binary format
file target/release/ltmatrix

# Intel expected output:
# Mach-O 64-bit executable x86_64

# Apple Silicon expected output:
# Mach-O 64-bit executable arm64

# Universal expected output:
# Mach-O universal binary with 2 architectures: [x86_64] [arm64]
```

### Architecture Verification

```bash
# Check architectures
lipo -info target/release/ltmatrix-universal
# Expected: Architectures in the fat file: ... are: x86_64 arm64

# For single-architecture binaries
lipo -info target/x86_64-apple-darwin/release/ltmatrix
# Expected: Non-fat file: ... is architecture: x86_64
```

### Basic Functionality Tests

```bash
# Test version output
./target/release/ltmatrix --version
# Expected: ltmatrix 0.1.0

# Test help output
./target/release/ltmatrix --help

# Test JSON output
./target/release/ltmatrix --help --output json

# Test completions subcommand
./target/release/ltmatrix completions bash
```

### Dependency Check

```bash
# Check dynamic library dependencies
otool -L target/release/ltmatrix

# Expected output should include:
# /usr/lib/libSystem.B.dylib
# /System/Library/Frameworks/CoreFoundation.framework/...
# /System/Library/Frameworks/Security.framework/...

# Should NOT include:
# - Linux-specific libraries (linux-vdso, ld-linux)
# - Homebrew paths (/usr/local/lib, /opt/homebrew)
# - Non-system dylibs
```

### Code Signing Verification

```bash
# Verify code signature
codesign -v target/release/ltmatrix

# Expected output (no errors):
# (tool exits with code 0)

# Display signing details
codesign -dvv target/release/ltmatrix

# Expected output includes:
# Authority=adhoc (for ad-hoc signing)
# or
# Authority=Developer ID Application: ... (for distribution)
```

### Binary Size Check

```bash
# Check binary size
ls -lh target/release/ltmatrix

# Expected sizes:
# Single architecture: 5-50 MB
# Universal binary: 10-100 MB (combined)
```

## Common Issues

### Issue: "error: failed to find tool 'cc'"

**Cause:** Xcode Command Line Tools not installed

**Solution:**
```bash
xcode-select --install
```

### Issue: "error: cannot find crate for 'std'"

**Cause:** Rust target not installed for architecture

**Solution:**
```bash
# For Intel target
rustup target add x86_64-apple-darwin

# For Apple Silicon target
rustup target add aarch64-apple-darwin
```

### Issue: "cannot be opened because the developer cannot be verified"

**Cause:** macOS Gatekeeper blocking unsigned binary

**Solution:**
```bash
# Option 1: Remove quarantine attribute
xattr -cr target/release/ltmatrix

# Option 2: Sign the binary
codesign --force --deep --sign - target/release/ltmatrix

# Option 3: Allow in System Preferences
# System Preferences → Privacy & Security → Open Anyway
```

### Issue: "lipo: can't open input file"

**Cause:** One or both architecture binaries don't exist

**Solution:**
```bash
# Build both architectures first
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Verify binaries exist
ls target/x86_64-apple-darwin/release/ltmatrix
ls target/aarch64-apple-darwin/release/ltmatrix
```

### Issue: Binary crashes on older macOS

**Cause:** Minimum macOS version not set correctly

**Solution:**
```bash
# Verify minimum version in .cargo/config.toml
cat .cargo/config.toml

# Should have:
# [target.x86_64-apple-darwin]
# rustflags = ["-C", "link-arg=-mmacosx-version-min=10.13"]
#
# [target.aarch64-apple-darwin]
# rustflags = ["-C", "link-arg=-mmacosx-version-min=11.0"]
```

### Issue: "codesign: no such file or directory"

**Cause:** codesign not found in PATH

**Solution:**
```bash
# Verify Xcode Command Line Tools
xcode-select --print-path

# If empty or invalid, reinstall
sudo xcode-select --reset
xcode-select --install
```

### Issue: Build is slow

**Solution:** Enable build optimizations:
```toml
# In .cargo/config.toml or Cargo.toml
[profile.release]
lto = true          # Link-time optimization
codegen-units = 1   # Single codegen unit
strip = true        # Remove debug symbols
```

## Distribution

### Creating Distribution Package

```bash
# Automated packaging (recommended)
./scripts/package-macos.sh 0.1.0

# Output: target/dist/ltmatrix-0.1.0-macos-universal.tar.gz
```

**Package contents:**
```
ltmatrix-0.1.0-macos-universal/
├── ltmatrix          # Universal binary
├── README.md         # Installation documentation
├── install.sh        # Automated installation script
└── uninstall.sh      # Automated uninstallation script
```

### Manual Package Creation

```bash
# Create package directory
mkdir -p dist/ltmatrix-0.1.0-macos-universal

# Copy binary
cp target/release/ltmatrix-universal dist/ltmatrix-0.1.0-macos-universal/ltmatrix
chmod +x dist/ltmatrix-0.1.0-macos-universal/ltmatrix

# Create README
cat > dist/ltmatrix-0.1.0-macos-universal/README.md << 'EOF'
# ltmatrix - Installation

## Quick Install
sudo cp ltmatrix /usr/local/bin/

## Verify
ltmatrix --version
EOF

# Create tarball
cd dist
tar -czf ltmatrix-0.1.0-macos-universal.tar.gz ltmatrix-0.1.0-macos-universal

# Create checksum
shasum -a 256 ltmatrix-0.1.0-macos-universal.tar.gz > ltmatrix-0.1.0-macos-universal.tar.gz.sha256
```

### Homebrew Formula (Optional)

For Homebrew distribution:

```ruby
class Ltmatrix < Formula
  desc "Long-Time Agent Orchestrator"
  homepage "https://github.com/bigfish/ltmatrix"
  version "0.1.0"

  on_macos do
    on_intel do
      url "https://github.com/bigfish/ltmatrix/releases/download/v0.1.0/ltmatrix-0.1.0-macos-universal.tar.gz"
      sha256 "abc123..."
    end

    on_arm do
      url "https://github.com/bigfish/ltmatrix/releases/download/v0.1.0/ltmatrix-0.1.0-macos-universal.tar.gz"
      sha256 "abc123..."
    end
  end

  def install
    bin.install "ltmatrix"
  end

  test do
    assert_match "ltmatrix", shell_output("#{bin}/ltmatrix --version")
  end
end
```

## Performance Notes

### Build Times

- **Debug build:** ~5-15 seconds (incremental)
- **Release build:** ~30-90 seconds (incremental)
- **Full clean release:** ~2-5 minutes
- **Universal binary creation:** ~5 seconds (with both binaries)

### Binary Size

- **Debug build (single arch):** ~15-30 MB
- **Release build (single arch):** ~5-50 MB
- **Release build (universal):** ~10-100 MB (combined)
- **Release build (stripped):** ~3-40 MB

### Optimization Settings

The release profile is already optimized:
```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
strip = true        # Remove debug symbols
codegen-units = 1   # Better optimization
```

## Continuous Integration

### GitHub Actions - macOS Builds

```yaml
name: Build macOS

on:
  push:
    branches: [main]
  pull_request:

jobs:
  build-intel:
    name: Build Intel (x86_64)
    runs-on: macos-13  # Intel runner
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-apple-darwin

      - name: Build
        run: cargo build --release --target x86_64-apple-darwin

      - name: Test
        run: cargo test --target x86_64-apple-darwin

      - name: Sign binary
        run: codesign --force --deep --sign - target/x86_64-apple-darwin/release/ltmatrix

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ltmatrix-macos-intel
          path: target/x86_64-apple-darwin/release/ltmatrix

  build-arm:
    name: Build Apple Silicon (aarch64)
    runs-on: macos-14  # ARM runner
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin

      - name: Build
        run: cargo build --release --target aarch64-apple-darwin

      - name: Test
        run: cargo test --target aarch64-apple-darwin

      - name: Sign binary
        run: codesign --force --deep --sign - target/aarch64-apple-darwin/release/ltmatrix

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ltmatrix-macos-arm
          path: target/aarch64-apple-darwin/release/ltmatrix

  create-universal:
    name: Create Universal Binary
    needs: [build-intel, build-arm]
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4

      - name: Download Intel binary
        uses: actions/download-artifact@v4
        with:
          name: ltmatrix-macos-intel
          path: bin/intel

      - name: Download ARM binary
        uses: actions/download-artifact@v4
        with:
          name: ltmatrix-macos-arm
          path: bin/arm

      - name: Create universal binary
        run: |
          mkdir -p release
          lipo -create \
            bin/intel/ltmatrix \
            bin/arm/ltmatrix \
            -output release/ltmatrix-universal

      - name: Verify universal binary
        run: |
          file release/ltmatrix-universal
          lipo -info release/ltmatrix-universal
          ./release/ltmatrix-universal --version

      - name: Sign universal binary
        run: codesign --force --deep --sign - release/ltmatrix-universal

      - name: Upload universal binary
        uses: actions/upload-artifact@v4
        with:
          name: ltmatrix-macos-universal
          path: release/ltmatrix-universal
```

## Cross-Compilation Notes

### From Linux to macOS

**Not recommended.** Cross-compiling from Linux to macOS requires:

1. macOS SDK (Apple licensing restricts distribution)
2. Xcode toolchain
3. Complex cross-compilation setup

**Alternative:** Use GitHub Actions with macOS runners (free for public repos)

### From Windows to macOS

**Not possible.** Windows cannot build macOS binaries because:

1. No macOS SDK available for Windows
2. No Xcode toolchain for Windows
3. Native dependencies require macOS build tools

**Alternative:** Use GitHub Actions with macOS runners

## Dependency Management

### Native Dependencies

ltmatrix uses vendored dependencies to minimize system requirements:

```toml
[dependencies]
git2 = { version = "0.19", features = ["vendored-libgit2"] }
reqwest = { version = "0.12", features = ["rustls-tls"], default-features = false }
```

**Benefits:**
- `git2`: Statically links libgit2 (no system libgit2 required)
- `reqwest`: Uses pure Rust TLS (no OpenSSL required)
- No Homebrew or MacPorts dependencies needed

### Verification

```bash
# Verify no unexpected dependencies
otool -L target/release/ltmatrix

# Should NOT show:
# - /usr/local/lib (Homebrew Intel)
# - /opt/homebrew/lib (Homebrew ARM)
# - Any .dylib files except system frameworks
```

## Security Considerations

### Hardened Runtime (Optional)

For notarized apps, enable hardened runtime:

```bash
# Enable hardened runtime
codesign --options runtime --force --deep --sign - target/release/ltmatrix

# Verify
codesign -dvv target/release/ltmatrix | grep "Runtime"
```

### Entitlements

For special capabilities, create an entitlements file:

```xml
<!-- entitlements.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.network.client</key>
    <true/>
</dict>
</plist>
```

```bash
# Sign with entitlements
codesign --force --deep --sign - \
  --entitlements entitlements.plist \
  target/release/ltmatrix
```

## Additional Resources

- [Apple Code Signing Guide](https://developer.apple.com/library/archive/documentation/Security/Conceptual/CodeSigningGuide/)
- [Notarizing macOS Software](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)
- [Universal Binaries](https://developer.apple.com/documentation/apple-silicon/building-a-universal-macos-binary)
- [Rust Cross-Compilation](https://rust-lang.github.io/rustup/cross-compilation.html)
- [lipo Manual Page](https://ss64.com/osx/lipo.html)
- [codesign Manual Page](https://ss64.com/osx/codesign.html)
- [otool Manual Page](https://ss64.com/osx/otool.html)

## Summary

| Task | Command | Output |
|------|---------|--------|
| Build Intel (native) | `cargo build --release` | `target/release/ltmatrix` |
| Build Intel (cross) | `cargo build --release --target x86_64-apple-darwin` | `target/x86_64-apple-darwin/release/ltmatrix` |
| Build ARM (native) | `cargo build --release` | `target/release/ltmatrix` |
| Build ARM (cross) | `cargo build --release --target aarch64-apple-darwin` | `target/aarch64-apple-darwin/release/ltmatrix` |
| Create universal | `./scripts/create-universal-binary.sh` | `target/release/ltmatrix-universal` |
| Create package | `./scripts/package-macos.sh 0.1.0` | `target/dist/ltmatrix-0.1.0-macos-universal.tar.gz` |
| Sign binary | `codesign --force --deep --sign - target/release/ltmatrix` | (signs in place) |
| Verify signature | `codesign -v target/release/ltmatrix` | (exits 0 on success) |
| Check dependencies | `otool -L target/release/ltmatrix` | (shows linked libraries) |
| Check architectures | `lipo -info target/release/ltmatrix-universal` | `x86_64 arm64` |