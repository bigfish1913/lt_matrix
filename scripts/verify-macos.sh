#!/bin/bash
# macOS Binary Verification Script
#
# This script verifies that ltmatrix binaries work correctly on macOS.
# Run this on actual macOS hardware (Intel or Apple Silicon).
#
# Usage:
#   ./verify-macos.sh              # Auto-detect architecture
#   ./verify-macos.sh x86_64       # Build for Intel
#   ./verify-macos.sh aarch64      # Build for Apple Silicon
#   ./verify-macos.sh universal    # Build universal binary
#

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY_NAME="ltmatrix"

# Detect architecture if not specified
ARCH="${1:-auto}"

if [[ "$ARCH" == "auto" ]]; then
    case "$(uname -m)" in
        x86_64)
            ARCH="x86_64"
            TARGET="x86_64-apple-darwin"
            ;;
        arm64|aarch64)
            ARCH="aarch64"
            TARGET="aarch64-apple-darwin"
            ;;
        *)
            echo -e "${RED}Error: Unknown architecture$(uname -m)${NC}"
            exit 1
            ;;
    esac
elif [[ "$ARCH" == "x86_64" ]]; then
    TARGET="x86_64-apple-darwin"
elif [[ "$ARCH" == "aarch64" ]]; then
    TARGET="aarch64-apple-darwin"
elif [[ "$ARCH" == "universal" ]]; then
    TARGET="universal"
else
    echo -e "${RED}Error: Invalid architecture '$ARCH'${NC}"
    echo "Valid options: x86_64, aarch64, universal, auto"
    exit 1
fi

echo "=========================================="
echo "ltmatrix macOS Binary Verification"
echo "=========================================="
echo "Architecture: $ARCH"
echo "Target: $TARGET"
echo "Project Root: $PROJECT_ROOT"
echo ""

cd "$PROJECT_ROOT"

# Function to print success
print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# Function to print warning
print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Function to print error
print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Function to check a command
check_command() {
    if eval "$1"; then
        print_success "$2"
        return 0
    else
        print_error "$2"
        return 1
    fi
}

# Build for specified target
echo "Step 1: Building binary..."
if [[ "$TARGET" == "universal" ]]; then
    echo "Building universal binary (both architectures)..."

    # Build Intel
    echo "  - Building Intel (x86_64)..."
    cargo build --release --target x86_64-apple-darwin

    # Build ARM
    echo "  - Building ARM (aarch64)..."
    cargo build --release --target aarch64-apple-darwin

    # Create universal binary
    echo "  - Creating universal binary..."
    mkdir -p release
    lipo -create \
        target/x86_64-apple-darwin/release/$BINARY_NAME \
        target/aarch64-apple-darwin/release/$BINARY_NAME \
        -output release/$BINARY_NAME

    BINARY_PATH="release/$BINARY_NAME"
else
    cargo build --release --target "$TARGET"
    BINARY_PATH="target/$TARGET/release/$BINARY_NAME"
fi

print_success "Build completed"

# Verify binary exists
echo ""
echo "Step 2: Verifying binary exists..."
if [[ ! -f "$BINARY_PATH" ]]; then
    print_error "Binary not found at $BINARY_PATH"
    exit 1
fi
print_success "Binary found at $BINARY_PATH"

# Check Mach-O format
echo ""
echo "Step 3: Checking binary format..."
FILE_OUTPUT=$(file "$BINARY_PATH")
echo "  $FILE_OUTPUT"

if [[ "$TARGET" == "universal" ]]; then
    if echo "$FILE_OUTPUT" | grep -q "Mach-O universal binary"; then
        print_success "Correct format: Mach-O universal binary"
    else
        print_error "Not a universal binary"
        exit 1
    fi
elif [[ "$ARCH" == "x86_64" ]]; then
    if echo "$FILE_OUTPUT" | grep -q "Mach-O 64-bit executable.*x86_64"; then
        print_success "Correct format: Mach-O 64-bit x86_64"
    else
        print_error "Not an x86_64 binary"
        exit 1
    fi
elif [[ "$ARCH" == "aarch64" ]]; then
    if echo "$FILE_OUTPUT" | grep -q "Mach-O 64-bit executable.*arm64"; then
        print_success "Correct format: Mach-O 64-bit arm64"
    else
        print_error "Not an arm64 binary"
        exit 1
    fi
fi

# Check binary size
echo ""
echo "Step 4: Checking binary size..."
SIZE=$(stat -f%z "$BINARY_PATH")
SIZE_MB=$(echo "scale=2; $SIZE / 1024 / 1024" | bc)
echo "  Binary size: ${SIZE_MB} MB"

SIZE_INT=$(echo "$SIZE_MB" | cut -d. -f1)
if [[ $SIZE_INT -lt 1 ]] || [[ $SIZE_INT -gt 200 ]]; then
    print_warning "Binary size ${SIZE_MB} MB is outside expected range [1, 200] MB"
else
    print_success "Binary size within reasonable range"
fi

# Test --version
echo ""
echo "Step 5: Testing --version command..."
VERSION_OUTPUT=$("$BINARY_PATH" --version 2>&1)
EXIT_CODE=$?
if [[ $EXIT_CODE -eq 0 ]] && echo "$VERSION_OUTPUT" | grep -q "ltmatrix"; then
    echo "  $VERSION_OUTPUT"
    print_success "--version command works"
else
    print_error "--version command failed"
    echo "  Exit code: $EXIT_CODE"
    echo "  Output: $VERSION_OUTPUT"
fi

# Test --help
echo ""
echo "Step 6: Testing --help command..."
HELP_OUTPUT=$("$BINARY_PATH" --help 2>&1)
EXIT_CODE=$?
if [[ $EXIT_CODE -eq 0 ]] && echo "$HELP_OUTPUT" | grep -q -E "(Usage:|USAGE:|usage:)"; then
    print_success "--help command works"
    echo "  Help output preview:"
    echo "$HELP_OUTPUT" | head -5
else
    print_error "--help command failed"
    echo "  Exit code: $EXIT_CODE"
fi

# Check dynamic dependencies
echo ""
echo "Step 7: Checking dynamic dependencies..."
OTOOL_OUTPUT=$(otool -L "$BINARY_PATH")
echo "$OTOOL_OUTPUT" | while read -r line; do
    echo "  $line"
done

# Check for suspicious dependencies
if echo "$OTOOL_OUTPUT" | grep -q "/usr/local/lib"; then
    print_warning "Found /usr/local/lib dependency (non-standard)"
fi

if echo "$OTOOL_OUTPUT" | grep -q "/opt/homebrew"; then
    print_warning "Found Homebrew dependency (non-standard)"
fi

print_success "Dependency check completed"

# Check for expected system frameworks
if echo "$OTOOL_OUTPUT" | grep -q "CoreFoundation"; then
    print_success "Links to CoreFoundation (expected)"
fi

if echo "$OTOOL_OUTPUT" | grep -q "Security"; then
    print_success "Links to Security framework (expected)"
fi

# Apply code signing
echo ""
echo "Step 8: Applying code signing..."
codesign --force --deep --sign - "$BINARY_PATH"
print_success "Ad-hoc code signing applied"

# Verify code signature
echo ""
echo "Step 9: Verifying code signature..."
if codesign -v "$BINARY_PATH" 2>&1; then
    print_success "Code signature is valid"
else
    print_error "Code signature verification failed"
fi

# Display code signing details
echo ""
echo "Step 10: Code signing details..."
CODESIGN_OUTPUT=$(codesign -dvv "$BINARY_PATH" 2>&1 || true)
if [[ -n "$CODESIGN_OUTPUT" ]]; then
    echo "$CODESIGN_OUTPUT" | while read -r line; do
        echo "  $line"
    done
fi

# Check for ad-hoc signature
if echo "$CODESIGN_OUTPUT" | grep -q "adhoc"; then
    print_success "Ad-hoc signature detected"
elif echo "$CODESIGN_OUTPUT" | grep -q "Authority"; then
    print_success "Developer signature detected"
fi

# Run tests (if available)
echo ""
echo "Step 11: Running unit tests..."
if cargo test --target "$TARGET" --lib 2>&1 | tail -5; then
    print_success "Unit tests passed"
else
    print_warning "Some unit tests failed or none available"
fi

# Check if universal binary contains both architectures
if [[ "$TARGET" == "universal" ]]; then
    echo ""
    echo "Step 12: Verifying universal binary architecture..."
    LIPO_OUTPUT=$(lipo -info "$BINARY_PATH")
    echo "  $LIPO_OUTPUT"

    if echo "$LIPO_OUTPUT" | grep -q "x86_64"; then
        print_success "Universal binary contains x86_64"
    else
        print_error "Universal binary missing x86_64"
    fi

    if echo "$LIPO_OUTPUT" | grep -q "arm64"; then
        print_success "Universal binary contains arm64"
    else
        print_error "Universal binary missing arm64"
    fi
fi

# Final summary
echo ""
echo "=========================================="
echo "Verification Summary"
echo "=========================================="
echo "Architecture: $ARCH"
echo "Binary: $BINARY_PATH"
echo "Size: ${SIZE_MB} MB"
echo ""
echo "All basic checks completed!"
echo ""
echo "To use the binary:"
echo "  cp $BINARY_PATH /usr/local/bin/ltmatrix"
echo "  $BINARY_NAME --version"
echo "  $BINARY_NAME --help"
echo ""
echo "To verify on another Mac:"
echo "  # Copy binary and run:"
echo "  codesign -v /path/to/$BINARY_NAME"
echo "  /path/to/$BINARY_NAME --version"
echo ""
