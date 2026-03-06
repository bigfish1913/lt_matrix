@echo off
REM build-linux.bat - Build Linux binaries for ltmatrix on Windows
REM
REM This script builds Linux binaries for x86_64 and aarch64 architectures.
REM It uses cargo-zigbuild for cross-compilation support.
REM
REM Usage: build-linux.bat [clean]
REM
REM Options:
REM   clean    Clean build artifacts before building
REM
REM Requirements:
REM   - Rust toolchain with cross-compilation targets installed
REM   - cargo-zigbuild (https://github.com/rust-cross/cargo-zigbuild)
REM   - Zig compiler (0.11+)

setlocal enabledelayedexpansion

REM Set color codes (Windows 10+)
set "GREEN=[92m"
set "RED=[91m"
set "YELLOW=[93m"
set "BLUE=[94m"
set "NC=[0m"

echo %BLUE%╔════════════════════════════════════════════════════════╗%NC%
echo %BLUE%║   ltmatrix Linux Build Script (Windows)               ║%NC%
echo %BLUE%╚════════════════════════════════════════════════════════╝%NC%
echo.

REM Check prerequisites
echo %BLUE%Checking prerequisites...%NC%

where cargo >nul 2>&1
if errorlevel 1 (
    echo %RED%Error: cargo not found. Please install Rust toolchain.%NC%
    exit /b 1
)

where cargo-zigbuild >nul 2>&1
if errorlevel 1 (
    echo %RED%Error: cargo-zigbuild not found.%NC%
    echo %YELLOW%Install with: cargo install cargo-zigbuild%NC%
    exit /b 1
)

where zig >nul 2>&1
if errorlevel 1 (
    echo %RED%Error: zig not found.%NC%
    echo %YELLOW%Install from: https://ziglang.org/download/%NC%
    exit /b 1
)

echo %GREEN%✓ All prerequisites satisfied!%NC%
echo.

REM Handle clean option
if "%1"=="clean" (
    echo %YELLOW%Cleaning build artifacts...%NC%
    cargo clean
    if errorlevel 1 (
        echo %RED%Clean failed!%NC%
        exit /b 1
    )
    echo %GREEN%✓ Clean complete!%NC%
    echo.
)

REM Build targets
echo %BLUE%Starting build process...%NC%
echo.

REM Record start time
set start_time=%time%

REM Build for x86_64
echo %BLUE%Building for x86_64-unknown-linux-gnu...%NC%
echo %YELLOW%Target: x86_64-unknown-linux-gnu%NC%
echo %YELLOW%Profile: release%NC%
cargo zigbuild --release --target x86_64-unknown-linux-gnu
if errorlevel 1 (
    echo %RED%✗ Build for x86_64 failed!%NC%
    exit /b 1
)
echo %GREEN%✓ Build for x86_64 completed!%NC%

REM Show binary size
if exist "target\x86_64-unknown-linux-gnu\release\ltmatrix.exe" (
    for %%A in ("target\x86_64-unknown-linux-gnu\release\ltmatrix.exe") do (
        echo %GREEN%Binary size: %%~zA bytes%NC%
    )
)
echo.

REM Build for aarch64
echo %BLUE%Building for aarch64-unknown-linux-gnu...%NC%
echo %YELLOW%Target: aarch64-unknown-linux-gnu%NC%
echo %YELLOW%Profile: release%NC%
cargo zigbuild --release --target aarch64-unknown-linux-gnu
if errorlevel 1 (
    echo %RED%✗ Build for aarch64 failed!%NC%
    exit /b 1
)
echo %GREEN%✓ Build for aarch64 completed!%NC%

REM Show binary size
if exist "target\aarch64-unknown-linux-gnu\release\ltmatrix.exe" (
    for %%A in ("target\aarch64-unknown-linux-gnu\release\ltmatrix.exe") do (
        echo %GREEN%Binary size: %%~zA bytes%NC%
    )
)
echo.

REM Summary
echo %GREEN%╔════════════════════════════════════════════════════════╗%NC%
echo %GREEN%║   Build Summary                                         ║%NC%
echo %GREEN%╚════════════════════════════════════════════════════════╝%NC%
echo %GREEN%✓ All builds completed successfully!%NC%
echo.
echo %BLUE%Binaries:%NC%
echo %BLUE%   - target\x86_64-unknown-linux-gnu\release\ltmatrix%NC%
echo %BLUE%   - target\aarch64-unknown-linux-gnu\release\ltmatrix%NC%
echo.
echo %YELLOW%Note: These are dynamically linked binaries requiring glibc 2.17+%NC%
echo %YELLOW%See docs\LINUX_BUILD_REPORT.md for details%NC%

endlocal
