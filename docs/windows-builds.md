# Windows Build Guide for ltmatrix

This document provides specific guidance for building ltmatrix on Windows, including native builds and cross-compilation limitations.

## Native Windows Build (x86_64)

### Prerequisites

- Windows 10 or later
- Visual Studio 2019 or later with C++ build tools
- Rust toolchain (MSVC)

### Building

```powershell
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Binary location
target/debug/ltmatrix.exe
target/release/ltmatrix.exe
```

### Build Configuration

The `.cargo/config.toml` file is pre-configured for Windows builds:

```toml
[target.x86_64-pc-windows-msvc]
linker = "rust-lld.exe"
ar = "rust-lld.exe"
```

This uses Rust's built-in linker (LLD) for faster linking.

### Verification

```powershell
# Check version
.\target\release\ltmatrix.exe --version

# Display help
.\target\release\ltmatrix.exe --help

# Run basic test
.\target\release\ltmatrix.exe --help
```

## Windows ARM64 Builds

### Known Limitations

**Cross-compilation from x86_64 Windows to ARM64 Windows is not directly supported** due to:

1. Missing ARM64 Windows SDK libraries on x86_64 systems
2. Visual Studio doesn't include ARM64 build tools by default
3. The `cc` crate cannot find the required ARM64 compilers

### Solutions

#### Option 1: Build on Native ARM64 Hardware (Recommended)

If you have an ARM64 Windows machine (e.g., Surface Pro X, Windows on ARM device):

```powershell
# Install Rust with ARM64 support
# Download rustup-init for ARM64 Windows from https://rustup.rs

# Build directly
cargo build --release

# Binary will be ARM64 native
target/release/ltmatrix.exe
```

#### Option 2: Use GitHub Actions (Recommended for CI/CD)

GitHub Actions provides native ARM64 Windows runners:

```yaml
name: Build Windows ARM64

on:
  push:
    tags: ['v*']

jobs:
  build-windows-arm64:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          targets: aarch64-pc-windows-msvc

      - name: Build for ARM64
        run: cargo build --release --target aarch64-pc-windows-msvc

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ltmatrix-aarch64-pc-windows-msvc
          path: target/aarch64-pc-windows-msvc/release/ltmatrix.exe
```

#### Option 3: Cross-Compile with Proper Toolchain

If you need to cross-compile and have ARM64 build tools:

1. Install Visual Studio 2019/2022 with ARM64 build tools
2. Install the ARM64 Windows SDK
3. Configure environment variables:

```powershell
# Set environment for ARM64 cross-compilation
$env:CMAKE_SYSTEM_PROCESSOR = "ARM64"
$env:CC = "clang"
$env:CXX = "clang++"

# Build
cargo build --release --target aarch64-pc-windows-msvc
```

## cargo-zigbuild on Windows

**Note:** `cargo-zigbuild` has limited support for Windows targets due to Zig's incomplete Windows SDK headers, particularly for ARM64.

For Windows x86_64, use the native MSVC build (recommended).

## Static Linking on Windows

Windows uses dynamic linking by runtime (DLLs). For truly static binaries on Windows:

### Option 1: Use `-C target-feature=+crt-static`

```toml
# In .cargo/config.toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

This statically links the C runtime but not all dependencies.

### Option 2: Use `gnullvm` Target

```
x86_64-pc-windows-gnullvm
```

This target uses MinGW-w64 for potentially more static linking, but may have compatibility issues.

### Recommendation

For Windows distribution, **use the standard MSVC build**. Windows users expect:
- `.exe` files
- Possible DLL dependencies (MSVC runtime)
- Installation via installer or zip archive

The MSVC runtime is widely available on Windows systems.

## Testing Windows Builds

### Basic Functionality Tests

```powershell
# Test version output
.\target\release\ltmatrix.exe --version

# Test help output
.\target\release\ltmatrix.exe --help

# Test JSON output
.\target\release\ltmatrix.exe --help --output json

# Test completions subcommand
.\target\release\ltmatrix.exe completions powershell
```

### Dependency Check

```powershell
# Check binary dependencies (if any)
dumpbin /dependents target\release\ltmatrix.exe

# Or using PowerShell
Get-Command target\release\ltmatrix.exe | Select-Object -ExpandProperty Path
```

## Distribution

### Creating Release Archives

```powershell
# Create distribution directory
New-Item -ItemType Directory -Force -Path dist

# Copy binary
Copy-Item target\release\ltmatrix.exe dist\

# Create README
Get-Content README | Out-File -Encoding UTF8 dist\README.txt

# Create zip archive
Compress-Archive -Path dist\* -DestinationPath ltmatrix-windows-x86_64.zip
```

### Installer Creation (Optional)

For production releases, consider creating an installer using:
- **WiX Toolset** - Create MSI installers
- **Inno Setup** - Create EXE installers
- **Scoop bucket** - For Scoop package manager users

## Common Issues

### Issue: "error: linker `link.exe` not found"

**Solution:** Install Visual Studio with C++ build tools, or use `gnu` toolchain:

```powershell
rustup default stable-x86_64-pc-windows-gnu
```

### Issue: "VCRUNTIME140.dll not found"

**Solution:** Install Visual C++ Redistributable:
- Download from Microsoft: https://aka.ms/vs/17/release/vc_redist.x64.exe
- Or statically link CRT (see Static Linking section)

### Issue: Build is slow

**Solution:** Use LLD linker (already configured) and build optimizations:
```toml
# In .cargo/config.toml
[profile.release]
lto = true
codegen-units = 1
strip = true
```

## Performance Notes

### Build Times

- **Debug build:** ~5-10 seconds (incremental)
- **Release build:** ~30-60 seconds (incremental)
- **Full clean release:** ~2-5 minutes

### Binary Size

- **Debug build:** ~15-20 MB
- **Release build (default):** ~4-5 MB
- **Release build (with strip):** ~400-600 KB

### Optimization

For smaller binaries, the release profile is already optimized:
```toml
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
strip = true        # Remove debug symbols
```

## Continuous Integration

### GitHub Actions - Windows Build

```yaml
name: Build Windows

on:
  push:
    branches: [main]
  pull_request:

jobs:
  build-windows-x86_64:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Build
        run: cargo build --release

      - name: Test
        run: cargo test --release

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ltmatrix-windows-x86_64
          path: target/release/ltmatrix.exe
```

## Additional Resources

- [Rust and Windows on the Rust Blog](https://blog.rust-lang.org/2024-04-09-windows-msvc-tci.html)
- [cargo-zigbuild Documentation](https://github.com/rust-cross/cargo-zigbuild)
- [Cross-Compilation in Rust](https://rust-lang.github.io/rustup/cross-compilation.html)
- [Windows SDK Documentation](https://docs.microsoft.com/en-us/windows/win32/sdk/)
