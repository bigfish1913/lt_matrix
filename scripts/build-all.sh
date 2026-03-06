#!/usr/bin/env bash
# Build script for ltmatrix - Compiles for all supported target platforms
#
# Usage:
#   ./scripts/build-all.sh              # Build all targets
#   ./scripts/build-all.sh --linux      # Build Linux targets only
#   ./scripts/build-all.sh --windows    # Build Windows targets only
#   ./scripts/build-all.sh --macos      # Build macOS targets only
#   ./scripts/build-all.sh --release    # Build and package for release

set -e  # Exit on error
set -u  # Exit on undefined variable

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO="${CARGO:-cargo}"
CROSS="${CROSS:-cross}"
RELEASE_DIR="${PROJECT_ROOT}/target/release"

# Targets organized by platform
LINUX_TARGETS=(
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-musl"
)

WINDOWS_TARGETS=(
    "x86_64-pc-windows-msvc"
    "aarch64-pc-windows-msvc"
)

MACOS_TARGETS=(
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
)

ALL_TARGETS=("${LINUX_TARGETS[@]}" "${WINDOWS_TARGETS[@]}" "${MACOS_TARGETS[@]}")

# Functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_command() {
    if ! command -v "$1" &> /dev/null; then
        log_error "$1 not found. Please install it first."
        return 1
    fi
    return 0
}

build_target() {
    local target=$1
    local build_cmd=$2

    log_info "Building for ${target}..."

    cd "$PROJECT_ROOT"

    if $build_cmd build --release --target "$target"; then
        log_info "✓ Successfully built ${target}"
        return 0
    else
        log_error "✗ Failed to build ${target}"
        return 1
    fi
}

build_platform() {
    local platform=$1
    local targets=()
    local build_cmd=$2

    case "$platform" in
        linux)
            targets=("${LINUX_TARGETS[@]}")
            ;;
        windows)
            targets=("${WINDOWS_TARGETS[@]}")
            ;;
        macos)
            targets=("${MACOS_TARGETS[@]}")
            ;;
        *)
            log_error "Unknown platform: $platform"
            return 1
            ;;
    esac

    local failed=0
    for target in "${targets[@]}"; do
        if ! build_target "$target" "$build_cmd"; then
            failed=1
        fi
    done

    return $failed
}

create_macos_universal() {
    log_info "Creating macOS universal binary..."

    local x86_binary="${RELEASE_DIR}/x86_64-apple-darwin/ltmatrix"
    local arm_binary="${RELEASE_DIR}/aarch64-apple-darwin/ltmatrix"
    local universal_dir="${PROJECT_ROOT}/target/universal-apple-darwin/release"
    local universal_binary="${universal_dir}/ltmatrix"

    # Check if both binaries exist
    if [ ! -f "$x86_binary" ]; then
        log_warn "x86_64 macOS binary not found, skipping universal binary creation"
        return 0
    fi

    if [ ! -f "$arm_binary" ]; then
        log_warn "ARM64 macOS binary not found, skipping universal binary creation"
        return 0
    fi

    # Create universal binary
    mkdir -p "$universal_dir"
    if lipo -create "$x86_binary" "$arm_binary" -output "$universal_binary"; then
        log_info "✓ Universal binary created: ${universal_binary}"
    else
        log_error "Failed to create universal binary"
        return 1
    fi
}

package_binaries() {
    log_info "Packaging binaries for distribution..."

    cd "$PROJECT_ROOT"
    mkdir -p dist

    # Package Linux binaries
    for target in "${LINUX_TARGETS[@]}"; do
        local binary="${RELEASE_DIR}/${target}/ltmatrix"
        if [ -f "$binary" ]; then
            local archive="dist/ltmatrix-$(echo $target | tr '_' '-').tar.gz"
            tar -czf "$archive" -C "${RELEASE_DIR}/${target}" ltmatrix
            log_info "✓ Created ${archive}"
        fi
    done

    # Package Windows binaries
    for target in "${WINDOWS_TARGETS[@]}"; do
        local binary="${RELEASE_DIR}/${target}/ltmatrix.exe"
        if [ -f "$binary" ]; then
            local archive="dist/ltmatrix-$(echo $target | tr '_' '-').zip"
            zip -q "$archive" "$binary"
            log_info "✓ Created ${archive}"
        fi
    done

    # Package macOS binaries
    for target in "${MACOS_TARGETS[@]}"; do
        local binary="${RELEASE_DIR}/${target}/ltmatrix"
        if [ -f "$binary" ]; then
            local archive="dist/ltmatrix-$(echo $target | tr '_' '-').tar.gz"
            tar -czf "$archive" -C "${RELEASE_DIR}/${target}" ltmatrix
            log_info "✓ Created ${archive}"
        fi
    done

    # Package universal macOS binary if it exists
    local universal="${PROJECT_ROOT}/target/universal-apple-darwin/release/ltmatrix"
    if [ -f "$universal" ]; then
        local archive="dist/ltmatrix-universal-apple-darwin.tar.gz"
        tar -czf "$archive" -C "${PROJECT_ROOT}/target/universal-apple-darwin/release" ltmatrix
        log_info "✓ Created ${archive}"
    fi
}

print_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Build ltmatrix for all supported target platforms.

OPTIONS:
    --linux          Build Linux targets only
    --windows        Build Windows targets only
    --macos          Build macOS targets only
    --release        Build and package for release
    --help           Show this help message

ENVIRONMENT VARIABLES:
    CARGO            Path to cargo command (default: cargo)
    CROSS            Path to cross command (default: cross)

EXAMPLES:
    $0                           # Build all targets
    $0 --linux                   # Build Linux targets only
    $0 --release                 # Build and package for release
    CROSS=cross $0 --linux        # Use cross for Linux builds

EOF
}

main() {
    local build_all=true
    local build_linux=false
    local build_windows=false
    local build_macos=false
    local do_release=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --linux)
                build_all=false
                build_linux=true
                shift
                ;;
            --windows)
                build_all=false
                build_windows=true
                shift
                ;;
            --macos)
                build_all=false
                build_macos=true
                shift
                ;;
            --release)
                do_release=true
                shift
                ;;
            --help)
                print_usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                print_usage
                exit 1
                ;;
        esac
    done

    # Check if we're using cross or cargo
    local build_cmd="$CARGO"
    if check_command "$CROSS"; then
        build_cmd="$CROSS"
        log_info "Using cross for compilation"
    else
        log_warn "cross not found, using cargo (may not work for all targets)"
        build_cmd="$CARGO"
    fi

    # Build targets
    local build_start=$(date +%s)
    local failed=0

    if [ "$build_all" = true ]; then
        log_info "Building all targets..."
        for target in "${ALL_TARGETS[@]}"; do
            if ! build_target "$target" "$build_cmd"; then
                failed=1
            fi
        done
    else
        if [ "$build_linux" = true ]; then
            if ! build_platform "linux" "$build_cmd"; then
                failed=1
            fi
        fi

        if [ "$build_windows" = true ]; then
            if ! build_platform "windows" "$build_cmd"; then
                failed=1
            fi
        fi

        if [ "$build_macos" = true ]; then
            if ! build_platform "macos" "$build_cmd"; then
                failed=1
            fi
            create_macos_universal
        fi
    fi

    local build_end=$(date +%s)
    local build_time=$((build_end - build_start))

    # Package if requested
    if [ "$do_release" = true ]; then
        package_binaries
    fi

    # Summary
    echo ""
    if [ $failed -eq 0 ]; then
        log_info "All builds completed successfully in ${build_time}s"
    else
        log_error "Some builds failed. Check output above."
        exit 1
    fi
}

main "$@"
