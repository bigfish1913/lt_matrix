#!/usr/bin/env bash
# Verification script for Linux builds of ltmatrix
#
# This script performs comprehensive checks on the compiled Linux binary
# to ensure it works correctly and is properly configured.
#
# Usage:
#   ./scripts/verify-linux-build.sh [--target <triple>]
#
# Examples:
#   ./scripts/verify-linux-build.sh
#   ./scripts/verify-linux-build.sh --target x86_64-unknown-linux-musl
#   ./scripts/verify-linux-build.sh --target aarch64-unknown-linux-musl

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_TARGET="x86_64-unknown-linux-musl"
TARGET="${DEFAULT_TARGET}"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --target)
            TARGET="$2"
            shift 2
            ;;
        --help)
            cat << EOF
Usage: $0 [OPTIONS]

Verify Linux build of ltmatrix binary.

OPTIONS:
    --target <triple>   Target triple to verify (default: x86_64-unknown-linux-musl)
    --help              Show this help message

EXAMPLES:
    $0
    $0 --target aarch64-unknown-linux-musl

EOF
            exit 0
            ;;
        *)
            echo -e "${RED}[ERROR]${NC} Unknown option: $1"
            exit 1
            ;;
    esac
done

BINARY_PATH="${PROJECT_ROOT}/target/${TARGET}/release/ltmatrix"

# Test tracking
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_WARNED=0

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
    ((TESTS_PASSED++))
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
    ((TESTS_FAILED++))
}

log_warn() {
    echo -e "${YELLOW}[⚠]${NC} $1"
    ((TESTS_WARNED++))
}

print_header() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

# Verification functions
verify_binary_exists() {
    print_header "Checking Binary Existence"

    if [ ! -f "$BINARY_PATH" ]; then
        log_error "Binary not found at $BINARY_PATH"
        log_info "Build the binary first: cargo build --release --target $TARGET"
        return 1
    fi

    log_success "Binary found at $BINARY_PATH"
    return 0
}

verify_executable() {
    print_header "Checking Executable Permissions"

    if [ ! -x "$BINARY_PATH" ]; then
        log_error "Binary is not executable"
        log_info "Run: chmod +x $BINARY_PATH"
        return 1
    fi

    log_success "Binary has executable permissions"
    return 0
}

verify_version() {
    print_header "Testing --version Flag"

    if ! OUTPUT=$("$BINARY_PATH" --version 2>&1); then
        log_error "Failed to execute --version command"
        log_info "Output: $OUTPUT"
        return 1
    fi

    if ! echo "$OUTPUT" | grep -q "ltmatrix"; then
        log_error "Version output doesn't contain 'ltmatrix'"
        log_info "Output: $OUTPUT"
        return 1
    fi

    log_success "Version command works correctly"
    log_info "Version: $OUTPUT"
    return 0
}

verify_help() {
    print_header "Testing --help Flag"

    if ! OUTPUT=$("$BINARY_PATH" --help 2>&1); then
        log_error "Failed to execute --help command"
        log_info "Output: $OUTPUT"
        return 1
    fi

    if ! echo "$OUTPUT" | grep -qiE "(usage|USAGE)"; then
        log_error "Help output doesn't contain usage information"
        log_info "Output: $OUTPUT"
        return 1
    fi

    log_success "Help command works correctly"
    return 0
}

verify_no_crash() {
    print_header "Testing Basic Commands (No Crash)"

    local commands=(
        "--version"
        "--help"
        "help"
    )

    for cmd in "${commands[@]}"; do
        if ! "$BINARY_PATH" $cmd >/dev/null 2>&1; then
            # Exit code 1 is acceptable for some commands, only check for signals
            if [ $? -gt 127 ]; then
                log_error "Command '$cmd' caused a crash"
                return 1
            fi
        fi
    done

    log_success "All basic commands executed without crashes"
    return 0
}

verify_static_linking() {
    print_header "Checking Static Linking"

    if ! command -v ldd &> /dev/null; then
        log_warn "ldd command not available, skipping static linking check"
        return 0
    fi

    LDD_OUTPUT=$(ldd "$BINARY_PATH" 2>&1)

    if echo "$LDD_OUTPUT" | grep -q "not a dynamic executable"; then
        log_success "Binary is fully statically linked (ideal for musl)"
        return 0
    fi

    # Count non-musl dependencies
    DEPS=$(echo "$LDD_OUTPUT" | grep -v "ld-musl" | grep -v "^$" | wc -l)

    if [ "$DEPS" -eq 0 ]; then
        log_success "Binary has minimal dynamic dependencies (only musl runtime)"
        return 0
    else
        log_warn "Binary has $DEPS dynamic dependencies (may not be fully static)"
        echo "$LDD_OUTPUT" | head -n 5
        return 0
    fi
}

verify_binary_size() {
    print_header "Checking Binary Size"

    if ! command -v du &> /dev/null; then
        log_warn "du command not available, skipping size check"
        return 0
    fi

    SIZE_BYTES=$(stat -f%z "$BINARY_PATH" 2>/dev/null || stat -c%s "$BINARY_PATH" 2>/dev/null)
    SIZE_MB=$(echo "scale=2; $SIZE_BYTES / (1024 * 1024)" | bc)

    log_info "Binary size: ${SIZE_MB} MB"

    # For Rust CLI with tokio and git2, expect 5-50 MB
    if (( $(echo "$SIZE_MB < 1.0" | bc -l) )); then
        log_error "Binary size suspiciously small (< 1 MB)"
        return 1
    elif (( $(echo "$SIZE_MB > 200.0" | bc -l) )); then
        log_warn "Binary size very large (> 200 MB), consider optimizing"
        return 0
    else
        log_success "Binary size within expected range"
        return 0
    fi
}

verify_file_type() {
    print_header "Checking File Type"

    if ! command -v file &> /dev/null; then
        log_warn "file command not available, skipping file type check"
        return 0
    fi

    FILE_TYPE=$(file "$BINARY_PATH")

    if ! echo "$FILE_TYPE" | grep -qE "(ELF|executable)"; then
        log_error "File doesn't appear to be a Linux executable"
        log_info "File type: $FILE_TYPE"
        return 1
    fi

    # Check architecture
    if echo "$FILE_TYPE" | grep -q "x86-64"; then
        log_success "File type: x86_64 Linux executable"
    elif echo "$FILE_TYPE" | grep -q "aarch64|ARM"; then
        log_success "File type: ARM64 Linux executable"
    else
        log_info "File type: $FILE_TYPE"
    fi

    return 0
}

verify_architecture() {
    print_header "Verifying Target Architecture"

    if ! command -v file &> /dev/null; then
        log_warn "file command not available, skipping architecture check"
        return 0
    fi

    FILE_TYPE=$(file "$BINARY_PATH")

    case "$TARGET" in
        *x86_64*)
            if ! echo "$FILE_TYPE" | grep -q "x86-64"; then
                log_error "Expected x86_64 binary but got different architecture"
                log_info "File type: $FILE_TYPE"
                return 1
            fi
            log_success "Architecture matches target (x86_64)"
            ;;
        *aarch64*)
            if ! echo "$FILE_TYPE" | grep -qE "aarch64|ARM aarch64"; then
                log_error "Expected ARM64 binary but got different architecture"
                log_info "File type: $FILE_TYPE"
                return 1
            fi
            log_success "Architecture matches target (aarch64)"
            ;;
        *)
            log_warn "Unknown target architecture, skipping verification"
            ;;
    esac

    return 0
}

verify_symbols() {
    print_header "Checking Binary Symbols"

    if ! command -v nm &> /dev/null; then
        log_warn "nm command not available, skipping symbol check"
        return 0
    fi

    # Check if binary is stripped
    SYMBOLS=$(nm "$BINARY_PATH" 2>/dev/null | wc -l)

    if [ "$SYMBOLS" -eq 0 ]; then
        log_success "Binary is stripped (no debug symbols)"
    else
        log_info "Binary contains $SYMBOLS symbols (not stripped)"
    fi

    return 0
}

# Main execution
main() {
    echo -e "${BLUE}ltmatrix Linux Build Verification${NC}"
    echo -e "${BLUE}Target: ${TARGET}${NC}"
    echo -e "${BLUE}Binary: ${BINARY_PATH}${NC}"

    # Run all verification checks
    verify_binary_exists || exit 1
    verify_executable || exit 1
    verify_file_type || exit 1
    verify_architecture || exit 1
    verify_binary_size || exit 1
    verify_version || exit 1
    verify_help || exit 1
    verify_no_crash || exit 1
    verify_static_linking || true  # Don't fail on warnings
    verify_symbols || true  # Don't fail on symbol check

    # Print summary
    print_header "Verification Summary"
    echo -e "${GREEN}Tests Passed: ${TESTS_PASSED}${NC}"
    if [ $TESTS_WARNED -gt 0 ]; then
        echo -e "${YELLOW}Tests Warned: ${TESTS_WARNED}${NC}"
    fi
    if [ $TESTS_FAILED -gt 0 ]; then
        echo -e "${RED}Tests Failed: ${TESTS_FAILED}${NC}"
        exit 1
    fi

    echo ""
    echo -e "${GREEN}✓ All verification checks passed!${NC}"
    echo -e "${GREEN}✓ Linux binary is ready for deployment${NC}"

    exit 0
}

main "$@"
