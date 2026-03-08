# macOS Binary Verification Guide

**Status**: ⚠️ **PENDING - Requires macOS Hardware or CI/CD**

**Last Updated**: 2026-03-06

## Current Situation

### What Cannot Be Done (From Windows)
- ❌ **Cannot build macOS binaries** from Windows (missing macOS SDK, Xcode toolchain)
- ❌ **Cannot execute macOS binaries** on Windows (incompatible executable format)
- ❌ **Cannot verify code signing** on Windows (macOS-specific tools required)

### What Has Been Prepared
- ✅ **Build configuration**: `.cargo/config.toml` properly configured for both architectures
- ✅ **Source code**: Cross-platform compatible, no macOS-specific issues
- ✅ **Verification scripts**: Ready to run on macOS hardware
- ✅ **CI/CD workflow**: GitHub Actions workflow for automated testing
- ✅ **Test suite**: Comprehensive integration tests for macOS

## Evidence: No Binaries Available

```bash
# Current state on Windows
$ ls target/x86_64-apple-darwin/release/
# Result: Build directories exist, but NO ltmatrix binary

$ ls target/aarch64-apple-darwin/release/
# Result: Build directories exist, but NO ltmatrix binary
```

**Conclusion**: Binaries do not exist and cannot be built from Windows.

## Verification Options

### Option 1: GitHub Actions (Recommended)

**Advantages**:
- Free macOS runners (Intel and ARM)
- Automated testing on every push
- Artifacts available for download
- No local macOS hardware required

**Steps**:
1. Push code to GitHub
2. Workflow automatically runs: `.github/workflows/macos-verification.yml`
3. View results in Actions tab
4. Download binaries from artifacts

**What the workflow does**:
- Builds for both Intel (x86_64) and ARM (aarch64)
- Verifies binary format and size
- Tests `--version` and `--help` commands
- Checks dynamic dependencies with `otool`
- Applies ad-hoc code signing
- Verifies code signature
- Runs unit tests
- Creates universal binary
- Uploads all three binaries as artifacts

**Expected time**: 10-15 minutes per architecture

### Option 2: Local macOS Hardware

**Requirements**:
- macOS 10.13+ (Intel) or macOS 11.0+ (Apple Silicon)
- Xcode Command Line Tools: `xcode-select --install`
- Rust toolchain: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

**Steps**:

1. **Clone repository**:
   ```bash
   git clone https://github.com/bigfish/ltmatrix.git
   cd ltmatrix
   ```

2. **Run verification script**:
   ```bash
   # Auto-detect architecture
   ./scripts/verify-macos.sh

   # Or specify architecture
   ./scripts/verify-macos.sh x86_64      # Intel
   ./scripts/verify-macos.sh aarch64     # Apple Silicon
   ./scripts/verify-macos.sh universal   # Universal binary
   ```

3. **Manual verification** (alternative):
   ```bash
   # Build
   cargo build --release --target x86_64-apple-darwin

   # Test
   target/x86_64-apple-darwin/release/ltmatrix --version
   target/x86_64-apple-darwin/release/ltmatrix --help

   # Check dependencies
   otool -L target/x86_64-apple-darwin/release/ltmatrix

   # Sign
   codesign --force --deep --sign - target/x86_64-apple-darwin/release/ltmatrix

   # Verify
   codesign -v target/x86_64-apple-darwin/release/ltmatrix
   ```

**Expected time**: 5-10 minutes

### Option 3: macOS Virtual Machine

**Options**:
- **VMware Fusion** / **Parallels Desktop** (paid)
- **VirtualBox** (free, but ARM virtualization limited)
- **UTM** (free, ARM Macs only)

**Steps**:
1. Install macOS in VM
2. Install Xcode Command Line Tools
3. Install Rust
4. Run verification script (same as Option 2)

**Expected time**: 30-60 minutes (setup) + 5-10 minutes (verification)

## Verification Checklist

When binaries are available, verify:

### Build Verification
- [ ] Binary exists at expected path
- [ ] Correct Mach-O format (x86_64, arm64, or universal)
- [ ] Binary size reasonable (1-200 MB)
- [ ] No build errors or warnings

### Functionality Tests
- [ ] `--version` command works and shows "ltmatrix"
- [ ] `--help` command displays usage information
- [ ] No crashes on basic commands

### Dependency Verification
- [ ] No unexpected dependencies (`/usr/local/lib`, Homebrew paths)
- [ ] Links to system frameworks (CoreFoundation, Security)
- [ ] No Linux-specific libraries

### Code Signing
- [ ] Ad-hoc signature applied
- [ ] Signature verification passes
- [ ] Can display signing details

### Testing
- [ ] Unit tests pass
- [ ] Integration tests pass (if available)
- [ ] No test failures

### Architecture-Specific
- [ ] **Intel (x86_64)**: Runs on Intel Macs
- [ ] **ARM (aarch64)**: Runs on Apple Silicon Macs
- [ ] **Universal**: Runs on both architectures

## What Gets Verified

### Binary Format
```bash
file target/x86_64-apple-darwin/release/ltmatrix
# Expected: Mach-O 64-bit executable x86_64
```

### Dynamic Dependencies
```bash
otool -L target/x86_64-apple-darwin/release/ltmatrix
# Expected: System frameworks only, no homebrew/local paths
```

### Code Signing
```bash
codesign -v target/x86_64-apple-darwin/release/ltmatrix
# Expected: "valid on disk" and "satisfies its Designated Requirement"

codesign -dvv target/x86_64-apple-darwin/release/ltmatrix
# Expected: Signature details (adhoc or developer)
```

### Universal Binary
```bash
lipo -info release/ltmatrix-universal
# Expected: "Architectures in the fat file: ... are: x86_64 arm64"
```

## Expected Results

### Intel Binary (x86_64-apple-darwin)
- **Format**: Mach-O 64-bit executable x86_64
- **Min macOS**: 10.13 (High Sierra)
- **Size**: ~5-50 MB
- **Dependencies**: System frameworks only
- **Code Signing**: Ad-hoc or developer

### ARM Binary (aarch64-apple-darwin)
- **Format**: Mach-O 64-bit executable arm64
- **Min macOS**: 11.0 (Big Sur)
- **Size**: ~5-50 MB
- **Dependencies**: System frameworks only
- **Code Signing**: Ad-hoc or developer

### Universal Binary
- **Format**: Mach-O universal binary with x86_64 and arm64
- **Min macOS**: 10.13 (Intel), 11.0 (ARM)
- **Size**: ~10-100 MB (combined)
- **Dependencies**: System frameworks only
- **Code Signing**: Ad-hoc or developer

## Troubleshooting

### "command not found: codesign"
**Solution**: Install Xcode Command Line Tools
```bash
xcode-select --install
```

### "target not found"
**Solution**: Install Rust target
```bash
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

### "cannot allocate memory"
**Solution**: Close other applications or increase swap

### Code signing fails
**Solution**:
```bash
# Remove extended attributes
xattr -cr target/x86_64-apple-darwin/release/ltmatrix

# Try signing again
codesign --force --deep --sign - target/x86_64-apple-darwin/release/ltmatrix
```

### Binary crashes on launch
**Solution**:
- Check minimum OS version
- Verify dependencies: `otool -L ltmatrix`
- Check code signature: `codesign -v ltmatrix`
- Run with diagnostics: `DYLD_PRINT_APIS=1 ./ltmatrix --version`

## Next Steps

### Immediate (To Complete This Task)
1. **Push to GitHub** to trigger automated CI/CD
2. **Monitor Actions tab** for build results
3. **Download artifacts** to get verified binaries
4. **Test locally** (if you have macOS access)

### For Production
1. **Set up code signing certificate** (Apple Developer account)
2. **Configure CI/CD** for signed builds
3. **Create release workflow** for GitHub Releases
4. **Test on real hardware** (Intel and Apple Silicon)
5. **Distribute binaries** via GitHub Releases or Homebrew

## Verification Status

| Task | Status | Notes |
|------|--------|-------|
| Build configuration | ✅ Complete | Properly configured in `.cargo/config.toml` |
| Source code readiness | ✅ Complete | Cross-platform compatible |
| Verification scripts | ✅ Complete | Ready to run on macOS |
| CI/CD workflow | ✅ Complete | GitHub Actions workflow created |
| Intel binary built | ❌ Pending | Requires macOS hardware or CI/CD |
| ARM binary built | ❌ Pending | Requires macOS hardware or CI/CD |
| Universal binary created | ❌ Pending | Requires both Intel and ARM binaries |
| Functionality tests | ❌ Pending | Requires built binaries |
| Dependency verification | ❌ Pending | Requires built binaries |
| Code signing verification | ❌ Pending | Requires built binaries |

## Conclusion

**The task cannot be completed from Windows** because:
1. macOS SDK and Xcode toolchain are not available on Windows
2. Binaries cannot be cross-compiled for macOS from Windows
3. macOS-specific verification tools (codesign, otool) don't exist on Windows

**What is ready**:
- ✅ Build configuration for both architectures
- ✅ Cross-platform source code
- ✅ Comprehensive verification scripts
- ✅ Automated CI/CD workflow

**What needs to happen**:
- ⚠️ Run verification on macOS hardware OR
- ⚠️ Push to GitHub to trigger automated CI/CD

**Path forward**:
1. Push code to GitHub
2. GitHub Actions builds and verifies binaries
3. Download verified binaries from artifacts
4. Optionally test on local macOS hardware

---

**Document Status**: ⚠️ PENDING VERIFICATION
**Action Required**: Run on macOS hardware or enable CI/CD
