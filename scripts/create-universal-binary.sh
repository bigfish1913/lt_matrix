#!/bin/bash
# Create Universal macOS Binary
#
# This script combines Intel (x86_64) and ARM (aarch64) binaries
# into a universal binary that runs on both architectures.
#
# Requirements:
#   - Both x86_64-apple-darwin and aarch64-apple-darwin binaries must exist
#   - Must run on macOS (lipo is a macOS-specific tool)
#
# Usage:
#   ./create-universal-binary.sh

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INTEL_BINARY="$PROJECT_ROOT/target/x86_64-apple-darwin/release/ltmatrix"
ARM_BINARY="$PROJECT_ROOT/target/aarch64-apple-darwin/release/ltmatrix"
UNIVERSAL_BINARY="$PROJECT_ROOT/target/release/ltmatrix-universal"
BINARY_NAME="ltmatrix"

echo "=========================================="
echo "Universal Binary Creator"
echo "=========================================="
echo ""

# Function to print success
print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# Function to print error
print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Function to print warning
print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Check if we're on macOS
if [[ "$(uname)" != "Darwin" ]]; then
    print_error "This script must be run on macOS"
    echo "Reason: lipo is a macOS-specific tool"
    exit 1
fi
print_success "Running on macOS"

# Check if lipo is available
if ! command -v lipo &> /dev/null; then
    print_error "lipo command not found"
    echo "Install Xcode Command Line Tools: xcode-select --install"
    exit 1
fi
print_success "lipo command available"

# Check if Intel binary exists
echo ""
echo "Step 1: Checking for Intel binary..."
if [[ ! -f "$INTEL_BINARY" ]]; then
    print_error "Intel binary not found: $INTEL_BINARY"
    echo ""
    echo "To build Intel binary:"
    echo "  cargo build --release --target x86_64-apple-darwin"
    exit 1
fi
print_success "Intel binary found"

# Check if ARM binary exists
echo ""
echo "Step 2: Checking for ARM binary..."
if [[ ! -f "$ARM_BINARY" ]]; then
    print_error "ARM binary not found: $ARM_BINARY"
    echo ""
    echo "To build ARM binary:"
    echo "  cargo build --release --target aarch64-apple-darwin"
    exit 1
fi
print_success "ARM binary found"

# Verify both binaries are executable
echo ""
echo "Step 3: Verifying binary executability..."
if [[ ! -x "$INTEL_BINARY" ]]; then
    print_warning "Intel binary not executable, fixing..."
    chmod +x "$INTEL_BINARY"
fi
if [[ ! -x "$ARM_BINARY" ]]; then
    print_warning "ARM binary not executable, fixing..."
    chmod +x "$ARM_BINARY"
fi
print_success "Both binaries are executable"

# Create output directory
echo ""
echo "Step 4: Creating output directory..."
mkdir -p "$(dirname "$UNIVERSAL_BINARY")"
print_success "Output directory ready"

# Create universal binary
echo ""
echo "Step 5: Creating universal binary..."
echo "  Intel:  $INTEL_BINARY"
echo "  ARM:    $ARM_BINARY"
echo "  Output: $UNIVERSAL_BINARY"
echo ""

lipo -create \
    "$INTEL_BINARY" \
    "$ARM_BINARY" \
    -output "$UNIVERSAL_BINARY"

if [[ -f "$UNIVERSAL_BINARY" ]]; then
    print_success "Universal binary created"
else
    print_error "Failed to create universal binary"
    exit 1
fi

# Verify universal binary
echo ""
echo "Step 6: Verifying universal binary..."

# Check it's a universal binary
FILE_OUTPUT=$(file "$UNIVERSAL_BINARY")
echo "  $FILE_OUTPUT"

if echo "$FILE_OUTPUT" | grep -q "Mach-O universal binary"; then
    print_success "Correct format: Mach-O universal binary"
else
    print_error "Not a universal binary"
    exit 1
fi

# Check it contains both architectures
echo ""
echo "Step 7: Verifying architectures..."
LIPO_OUTPUT=$(lipo -info "$UNIVERSAL_BINARY")
echo "  $LIPO_OUTPUT"

if echo "$LIPO_OUTPUT" | grep -q "x86_64"; then
    print_success "Contains x86_64 (Intel)"
else
    print_error "Missing x86_64 architecture"
    exit 1
fi

if echo "$LIPO_OUTPUT" | grep -q "arm64"; then
    print_success "Contains arm64 (Apple Silicon)"
else
    print_error "Missing arm64 architecture"
    exit 1
fi

# Test universal binary
echo ""
echo "Step 8: Testing universal binary..."
VERSION_OUTPUT=$("$UNIVERSAL_BINARY" --version 2>&1)
EXIT_CODE=$?

if [[ $EXIT_CODE -eq 0 ]] && echo "$VERSION_OUTPUT" | grep -q "ltmatrix"; then
    echo "  $VERSION_OUTPUT"
    print_success "Universal binary executes correctly"
else
    print_error "Universal binary failed to execute"
    echo "  Exit code: $EXIT_CODE"
    exit 1
fi

# Check binary size
echo ""
echo "Step 9: Checking binary size..."
SIZE=$(stat -f%z "$UNIVERSAL_BINARY")
SIZE_MB=$(echo "scale=2; $SIZE / 1024 / 1024" | bc)
echo "  Binary size: ${SIZE_MB} MB"

SIZE_INT=$(echo "$SIZE_MB" | cut -d. -f1)
if [[ $SIZE_INT -lt 1 ]] || [[ $SIZE_INT -gt 200 ]]; then
    print_warning "Binary size ${SIZE_MB} MB is outside expected range [1, 200] MB"
else
    print_success "Binary size within reasonable range"
fi

# Apply code signing
echo ""
echo "Step 10: Applying code signing..."
codesign --force --deep --sign - "$UNIVERSAL_BINARY"
print_success "Ad-hoc code signing applied"

# Verify code signature
echo ""
echo "Step 11: Verifying code signature..."
if codesign -v "$UNIVERSAL_BINARY" 2>&1; then
    print_success "Code signature is valid"
else
    print_error "Code signature verification failed"
    exit 1
fi

# Display code signing details
echo ""
echo "Step 12: Code signing details..."
CODESIGN_OUTPUT=$(codesign -dvv "$UNIVERSAL_BINARY" 2>&1 || true)
if [[ -n "$CODESIGN_OUTPUT" ]]; then
    echo "$CODESIGN_OUTPUT" | while read -r line; do
        echo "  $line"
    done
fi

# Create symlink for convenience
echo ""
echo "Step 13: Creating convenience symlink..."
SYMLINK_PATH="$PROJECT_ROOT/target/release/ltmatrix"
if [[ -L "$SYMLINK_PATH" ]]; then
    rm "$SYMLINK_PATH"
fi
ln -s "$UNIVERSAL_BINARY" "$SYMLINK_PATH"
print_success "Symlink created: $SYMLINK_PATH -> $UNIVERSAL_BINARY"

# Final summary
echo ""
echo "=========================================="
echo "Universal Binary Creation Complete"
echo "=========================================="
echo ""
echo "Binary created: $UNIVERSAL_BINARY"
echo "Symlink:        $SYMLINK_PATH"
echo ""
echo "Architectures:"
echo "  - x86_64 (Intel)"
echo "  - arm64 (Apple Silicon)"
echo ""
echo "Size: ${SIZE_MB} MB"
echo ""
echo "To use the binary:"
echo "  $SYMLINK_PATH --version"
echo "  $SYMLINK_PATH --help"
echo ""
echo "To install system-wide:"
echo "  sudo cp $UNIVERSAL_BINARY /usr/local/bin/ltmatrix"
echo ""
echo "To create distribution package:"
echo "  ./scripts/package-macos.sh"
echo ""
