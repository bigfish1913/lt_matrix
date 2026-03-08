# Universal macOS Binary - Creation and Distribution Guide

**Status**: ⚠️ **PENDING - Requires macOS Hardware or CI/CD**

**Last Updated**: 2026-03-06

## Overview

This guide covers creating and distributing a universal macOS binary that runs natively on both Intel and Apple Silicon Macs.

## What is a Universal Binary?

A universal binary (also called a "fat binary") contains machine code for multiple architectures in a single file:

- **x86_64**: Intel-based Macs (macOS 10.13+)
- **arm64**: Apple Silicon Macs (macOS 11.0+)

When executed, macOS automatically selects the appropriate code for the current architecture.

## Requirements

### Prerequisites

1. **macOS Hardware** or **CI/CD** (GitHub Actions)
   - Cannot create universal binaries on Windows (lipo is macOS-specific)

2. **Both Architecture Binaries**
   - Intel: `target/x86_64-apple-darwin/release/ltmatrix`
   - ARM: `target/aarch64-apple-darwin/release/ltmatrix`

3. **macOS Tools**
   - `lipo`: Tool to create universal binaries
   - `codesign`: Code signing tool
   - Included with Xcode Command Line Tools

### Installing Xcode Command Line Tools

```bash
xcode-select --install
```

## Creation Process

### Option 1: Automated Script (Recommended)

```bash
./scripts/create-universal-binary.sh
```

**What the script does**:
1. Verifies both Intel and ARM binaries exist
2. Creates universal binary using `lipo`
3. Verifies both architectures are present
4. Tests binary execution
5. Applies code signing
6. Creates convenience symlink

**Output**: `target/release/ltmatrix-universal`

### Option 2: Manual Process

```bash
# Create universal binary
lipo -create \
  target/x86_64-apple-darwin/release/ltmatrix \
  target/aarch64-apple-darwin/release/ltmatrix \
  -output target/release/ltmatrix-universal

# Verify architectures
lipo -info target/release/ltmatrix-universal
# Expected: Architectures in the fat file: ... are: x86_64 arm64

# Test execution
./target/release/ltmatrix-universal --version

# Apply code signing
codesign --force --deep --sign - target/release/ltmatrix-universal

# Verify signature
codesign -v target/release/ltmatrix-universal
```

## Verification Checklist

After creating the universal binary, verify:

### Format Check
```bash
file target/release/ltmatrix-universal
```
**Expected**: `Mach-O universal binary with 2 architectures`

### Architecture Check
```bash
lipo -info target/release/ltmatrix-universal
```
**Expected**: `Architectures in the fat file: ... are: x86_64 arm64`

### Execution Test
```bash
./target/release/ltmatrix-universal --version
./target/release/ltmatrix-universal --help
```
**Expected**: Both commands execute successfully

### Code Signing Check
```bash
codesign -v target/release/ltmatrix-universal
```
**Expected**: `valid on disk` and `satisfies its Designated Requirement`

### Size Check
```bash
ls -lh target/release/ltmatrix-universal
```
**Expected**: Size between Intel and ARM binaries combined (typically 10-100 MB)

## Distribution Packaging

### Option 1: Automated Packaging (Recommended)

```bash
./scripts/package-macos.sh [version]
```

**Example**:
```bash
./scripts/package-macos.sh 0.1.0
```

**What the script does**:
1. Verifies universal binary exists
2. Creates package directory
3. Copies binary and creates README
4. Creates installation/uninstallation scripts
5. Creates tarball with SHA256 checksum
6. Displays summary

**Output**: `target/dist/ltmatrix-{version}-macos-universal.tar.gz`

### Option 2: Manual Packaging

```bash
# Create package directory
mkdir -p target/dist/ltmatrix-0.1.0-macos-universal

# Copy binary
cp target/release/ltmatrix-universal target/dist/ltmatrix-0.1.0-macos-universal/ltmatrix
chmod +x target/dist/ltmatrix-0.1.0-macos-universal/ltmatrix

# Create README (see package-macos.sh for template)
# Create install.sh script
# Create uninstall.sh script

# Create tarball
cd target/dist
tar -czf ltmatrix-0.1.0-macos-universal.tar.gz ltmatrix-0.1.0-macos-universal

# Create checksum
shasum -a 256 ltmatrix-0.1.0-macos-universal.tar.gz > ltmatrix-0.1.0-macos-universal.tar.gz.sha256
```

## Package Contents

The distribution package includes:

```
ltmatrix-{version}-macos-universal/
├── ltmatrix          # Universal binary (executable)
├── README.md         # Installation and usage documentation
├── install.sh        # Automated installation script
└── uninstall.sh      # Automated uninstallation script
```

## Installation (End User)

### Quick Install

```bash
# Extract tarball
tar -xzf ltmatrix-{version}-macos-universal.tar.gz
cd ltmatrix-{version}-macos-universal

# Run installation script
./install.sh

# Verify installation
ltmatrix --version
```

### Manual Install

```bash
# Install to /usr/local/bin (requires sudo)
sudo cp ltmatrix /usr/local/bin/
sudo chmod +x /usr/local/bin/ltmatrix

# Or install to user bin (no sudo)
mkdir -p ~/.local/bin
cp ltmatrix ~/.local/bin/
export PATH="$HOME/.local/bin:$PATH"
```

## Troubleshooting

### "lipo: command not found"

**Solution**: Install Xcode Command Line Tools
```bash
xcode-select --install
```

### "Binary not found"

**Cause**: Intel or ARM binary doesn't exist

**Solution**: Build both binaries first
```bash
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

### "Not a universal binary"

**Cause**: lipo command failed or incorrect input

**Solution**: Verify input binaries exist and are valid
```bash
file target/x86_64-apple-darwin/release/ltmatrix
file target/aarch64-apple-darwin/release/ltmatrix
```

### "Binary crashed on launch"

**Possible causes**:
- Code signing issue
- Missing macOS version requirement
- Corrupt binary

**Solutions**:
```bash
# Remove quarantine attribute
xattr -cr target/release/ltmatrix-universal

# Re-sign binary
codesign --force --deep --sign - target/release/ltmatrix-universal

# Rebuild from scratch
cargo clean
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
./scripts/create-universal-binary.sh
```

## CI/CD Integration

### GitHub Actions Workflow

The `.github/workflows/macos-verification.yml` workflow automatically:

1. Builds both Intel and ARM binaries
2. Verifies each binary separately
3. Creates universal binary using lipo
4. Tests universal binary execution
5. Applies code signing
6. Uploads all three binaries as artifacts

**Artifacts**:
- `ltmatrix-macos-intel`: Intel-only binary
- `ltmatrix-macos-arm`: ARM-only binary
- `ltmatrix-macos-universal`: Universal binary

### Automatic Packaging

Add packaging step to CI/CD:

```yaml
- name: Create distribution package
  run: |
    VERSION=${{ github.ref_name }}
    ./scripts/package-macos.sh ${VERSION#v}

- name: Upload package
  uses: actions/upload-artifact@v4
  with:
    name: ltmatrix-macos-dist
    path: target/dist/ltmatrix-*-macos-universal.tar.gz
```

## Best Practices

### 1. Always Test Both Architectures

Even though the universal binary works, test on actual hardware:
- Intel Mac (if available)
- Apple Silicon Mac (if available)
- Rosetta 2 mode (ARM Mac running Intel binary)

### 2. Code Signing

Always apply code signing, even for ad-hoc:
```bash
codesign --force --deep --sign - ltmatrix-universal
```

### 3. Verify Before Distribution

Run the full verification checklist:
- Format check
- Architecture check
- Execution test
- Code signing check
- Size check

### 4. Include Clear Documentation

Package should include:
- Installation instructions
- System requirements
- Troubleshooting guide
- Uninstallation instructions

### 5. Provide Checksums

Always provide SHA256 checksum for verification:
```bash
shasum -a 256 ltmatrix-{version}-macos-universal.tar.gz
```

## Current Status

### Infrastructure Ready ✅
- `scripts/create-universal-binary.sh` - Universal binary creation script
- `scripts/package-macos.sh` - Distribution packaging script
- `.github/workflows/macos-verification.yml` - Automated CI/CD

### Binaries Missing ❌
- Intel binary: Does not exist (cannot build from Windows)
- ARM binary: Does not exist (cannot build from Windows)
- Universal binary: Cannot create (requires both binaries)

### Platform Constraint
- **Cannot build macOS binaries from Windows**
- **Cannot create universal binaries on Windows** (lipo is macOS-specific)

## Next Steps

### To Complete This Task

1. **Push to GitHub** (Recommended)
   - Workflow automatically builds all binaries
   - Creates universal binary automatically
   - Packages for distribution

2. **Run on macOS Hardware**
   ```bash
   # Build both architectures
   cargo build --release --target x86_64-apple-darwin
   cargo build --release --target aarch64-apple-darwin

   # Create universal binary
   ./scripts/create-universal-binary.sh

   # Package for distribution
   ./scripts/package-macos.sh 0.1.0
   ```

3. **Download from CI/CD**
   - Wait for GitHub Actions to complete
   - Download universal binary from artifacts
   - Download distribution package from artifacts

## Expected Results

### Universal Binary

```bash
$ file target/release/ltmatrix-universal
Mach-O universal binary with 2 architectures: [x86_64:Mach-O 64-bit executable x86_64] [arm64:Mach-O 64-bit executable arm64]

$ lipo -info target/release/ltmatrix-universal
Architectures in the fat file: target/release/ltmatrix-universal are: x86_64 arm64

$ ./target/release/ltmatrix-universal --version
ltmatrix 0.1.0

$ ls -lh target/release/ltmatrix-universal
-rwxr-xr-x  1 user  staff   XXM Month Day Time target/release/ltmatrix-universal
```

### Distribution Package

```bash
$ ls -lh target/dist/
ltmatrix-0.1.0-macos-universal.tar.gz      (10-50 MB)
ltmatrix-0.1.0-macos-universal.tar.gz.sha256  (checksum file)

$ tar -tzf ltmatrix-0.1.0-macos-universal.tar.gz
ltmatrix-0.1.0-macos-universal/
ltmatrix-0.1.0-macos-universal/ltmatrix
ltmatrix-0.1.0-macos-universal/README.md
ltmatrix-0.1.0-macos-universal/install.sh
ltmatrix-0.1.0-macos-universal/uninstall.sh
```

## Conclusion

**Infrastructure Status**: ✅ Complete
- Scripts created and documented
- CI/CD workflow configured
- Documentation comprehensive

**Execution Status**: ⚠️ Pending macOS environment
- Cannot create universal binary from Windows
- Requires macOS hardware or CI/CD

**Confidence**: High (will succeed once executed on macOS)

---

**Next Action**: Push to GitHub or execute on macOS hardware
