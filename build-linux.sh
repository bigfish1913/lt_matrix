#!/bin/bash
# build-linux.sh - Build Linux binaries for ltmatrix
#
# This script builds Linux binaries for x86_64 and aarch64 architectures.
# It uses cargo-zigbuild for cross-compilation support.
#
# Usage: ./build-linux.sh [clean]
#
# Options:
#   clean    Clean build artifacts before building
#
# Requirements:
#   - Rust toolchain with cross-compilation targets installed
#   - cargo-zigbuild (https://github.com/rust-cross/cargo-zigbuild)
#   - Zig compiler (0.11+)

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored message
print_msg() {
    local color=$1
    shift
    echo -e "${color}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $@"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
check_prerequisites() {
    print_msg "${BLUE}" "Checking prerequisites..."

    if ! command_exists cargo; then
        print_msg "${RED}" "Error: cargo not found. Please install Rust toolchain."
        exit 1
    fi

    if ! command_exists cargo-zigbuild; then
        print_msg "${RED}" "Error: cargo-zigbuild not found."
        print_msg "${YELLOW}" "Install with: cargo install cargo-zigbuild"
        exit 1
    fi

    if ! command_exists zig; then
        print_msg "${RED}" "Error: zig not found."
        print_msg "${YELLOW}" "Install from: https://ziglang.org/download/"
        exit 1
    fi

    # Check Rust targets
    print_msg "${BLUE}" "Checking Rust targets..."
    local targets=("x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu")
    for target in "${targets[@]}"; do
        if rustup target list --installed | grep -q "$target"; then
            print_msg "${GREEN}" "✓ Target $target installed"
        else
            print_msg "${YELLOW}" "Installing target $target..."
            rustup target add "$target"
        fi
    done

    print_msg "${GREEN}" "All prerequisites satisfied!"
}

# Clean build artifacts
clean_build() {
    print_msg "${YELLOW}" "Cleaning build artifacts..."
    cargo clean
    print_msg "${GREEN}" "Clean complete!"
}

# Build for specific target
build_target() {
    local target=$1
    local features=$2

    print_msg "${BLUE}" "Building for ${target}..."
    print_msg "${YELLOW}" "Target: ${target}"
    print_msg "${YELLOW}" "Features: ${features}"
    print_msg "${YELLOW}" "Profile: release"

    local start_time=$(date +%s)

    if cargo zigbuild --release --target "${target}" --features "${features}"; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        print_msg "${GREEN}" "✓ Build for ${target} completed in ${duration}s"

        # Show binary size
        local binary="target/${target}/release/ltmatrix"
        if [ -f "$binary" ]; then
            local size=$(du -h "$binary" | cut -f1)
            print_msg "${GREEN}" "Binary size: ${size}"
        fi
    else
        print_msg "${RED}" "✗ Build for ${target} failed!"
        return 1
    fi
}

# Main build process
main() {
    print_msg "${BLUE}" "╔════════════════════════════════════════════════════════╗"
    print_msg "${BLUE}" "║   ltmatrix Linux Build Script                          ║"
    print_msg "${BLUE}" "╚════════════════════════════════════════════════════════╝"

    # Handle clean option
    if [ "$1" = "clean" ]; then
        clean_build
    fi

    # Check prerequisites
    check_prerequisites

    # Build targets
    print_msg "${BLUE}" "Starting build process..."

    local build_start=$(date +%s)

    # Build for x86_64 (dynamic linking with glibc)
    if ! build_target "x86_64-unknown-linux-gnu" ""; then
        print_msg "${RED}" "x86_64 build failed!"
        exit 1
    fi

    # Build for aarch64 (dynamic linking with glibc)
    if ! build_target "aarch64-unknown-linux-gnu" ""; then
        print_msg "${RED}" "aarch64 build failed!"
        exit 1
    fi

    local build_end=$(date +%s)
    local total_duration=$((build_end - build_start))

    # Summary
    echo ""
    print_msg "${GREEN}" "╔════════════════════════════════════════════════════════╗"
    print_msg "${GREEN}" "║   Build Summary                                       ║"
    print_msg "${GREEN}" "╚════════════════════════════════════════════════════════╝"
    print_msg "${GREEN}" "✓ All builds completed successfully!"
    print_msg "${GREEN}" "✓ Total time: ${total_duration}s"
    echo ""
    print_msg "${BLUE}" "Binaries:"
    print_msg "${BLUE}" "  - target/x86_64-unknown-linux-gnu/release/ltmatrix"
    print_msg "${BLUE}" "  - target/aarch64-unknown-linux-gnu/release/ltmatrix"
    echo ""
    print_msg "${YELLOW}" "Note: These are dynamically linked binaries requiring glibc 2.17+"
    print_msg "${YELLOW}" "See docs/LINUX_BUILD_REPORT.md for details"
}

# Run main function
main "$@"
