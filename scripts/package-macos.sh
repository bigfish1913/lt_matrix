#!/bin/bash
# Package Universal macOS Binary for Distribution
#
# Creates a tarball distribution with the universal binary,
# README, and installation instructions.
#
# Requirements:
#   - Universal binary must exist (run create-universal-binary.sh first)
#   - Must run on macOS
#
# Usage:
#   ./package-macos.sh [version]
#
# Example:
#   ./package-macos.sh 0.1.0

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UNIVERSAL_BINARY="$PROJECT_ROOT/target/release/ltmatrix-universal"
VERSION="${1:-$(grep '^version' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/')}"
DIST_DIR="$PROJECT_ROOT/target/dist"
PACKAGE_NAME="ltmatrix-${VERSION}-macos-universal"
PACKAGE_DIR="$DIST_DIR/$PACKAGE_NAME"
TARBALL="$DIST_DIR/${PACKAGE_NAME}.tar.gz"

echo "=========================================="
echo "macOS Distribution Packager"
echo "=========================================="
echo ""
echo "Version: $VERSION"
echo "Output:  $TARBALL"
echo ""

# Function to print success
print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# Function to print error
print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Function to print info
print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

# Check if we're on macOS
if [[ "$(uname)" != "Darwin" ]]; then
    print_error "This script must be run on macOS"
    exit 1
fi
print_success "Running on macOS"

# Check if universal binary exists
echo ""
echo "Step 1: Checking for universal binary..."
if [[ ! -f "$UNIVERSAL_BINARY" ]]; then
    print_error "Universal binary not found: $UNIVERSAL_BINARY"
    echo ""
    echo "To create universal binary:"
    echo "  ./scripts/create-universal-binary.sh"
    exit 1
fi
print_success "Universal binary found"

# Verify universal binary is valid
echo ""
echo "Step 2: Verifying universal binary..."
FILE_OUTPUT=$(file "$UNIVERSAL_BINARY")
if ! echo "$FILE_OUTPUT" | grep -q "Mach-O universal binary"; then
    print_error "Not a valid universal binary"
    echo "  $FILE_OUTPUT"
    exit 1
fi

# Check architectures
LIPO_OUTPUT=$(lipo -info "$UNIVERSAL_BINARY")
if ! echo "$LIPO_OUTPUT" | grep -q "x86_64" || ! echo "$LIPO_OUTPUT" | grep -q "arm64"; then
    print_error "Universal binary missing required architectures"
    echo "  $LIPO_OUTPUT"
    exit 1
fi
print_success "Universal binary is valid (x86_64 + arm64)"

# Test binary executes
if ! "$UNIVERSAL_BINARY" --version &> /dev/null; then
    print_error "Universal binary failed to execute"
    exit 1
fi
print_success "Universal binary executes correctly"

# Clean previous build
echo ""
echo "Step 3: Cleaning previous build..."
if [[ -d "$PACKAGE_DIR" ]]; then
    rm -rf "$PACKAGE_DIR"
    print_info "Removed previous package directory"
fi
mkdir -p "$PACKAGE_DIR"
print_success "Package directory created"

# Copy binary
echo ""
echo "Step 4: Copying binary..."
cp "$UNIVERSAL_BINARY" "$PACKAGE_DIR/ltmatrix"
chmod +x "$PACKAGE_DIR/ltmatrix"
print_success "Binary copied"

# Create README
echo ""
echo "Step 5: Creating README..."
cat > "$PACKAGE_DIR/README.md" << 'EOF'
# ltmatrix - Long-Time Agent Orchestrator

A high-performance, cross-platform agent orchestrator for automated software development tasks.

## What's Included

This package contains a **universal macOS binary** that runs natively on both:
- Intel Macs (x86_64) - macOS 10.13 (High Sierra) or later
- Apple Silicon Macs (arm64) - macOS 11.0 (Big Sur) or later

## Installation

### Quick Install

```bash
# Install to /usr/local/bin
sudo cp ltmatrix /usr/local/bin/

# Verify installation
ltmatrix --version
```

### Alternative Locations

```bash
# Install to user bin (no sudo required)
mkdir -p ~/.local/bin
cp ltmatrix ~/.local/bin/

# Add to PATH if needed
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

## Verification

After installation, verify the binary:

```bash
# Check version
ltmatrix --version

# Check help
ltmatrix --help

# Verify universal binary
file $(which ltmatrix)
# Should show: Mach-O universal binary with 2 architectures

# Verify architectures
lipo -info $(which ltmatrix)
# Should show: Architectures in the fat file: ... are: x86_64 arm64
```

## Quick Start

```bash
# Generate and execute tasks
ltmatrix "Add user authentication to my app"

# Dry run (plan only, don't execute)
ltmatrix --dry-run "Create a REST API"

# Fast mode (skip tests)
ltmatrix --fast "Fix the login bug"

# Expert mode (highest quality)
ltmatrix --expert "Implement OAuth2"
```

## Configuration

Create a configuration file at `~/.ltmatrix/config.toml`:

```toml
[default]
agent = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[agents.claude]
model_fast = "claude-sonnet-4-6"
model_smart = "claude-opus-4-6"
```

## Uninstall

```bash
# Remove from /usr/local/bin
sudo rm /usr/local/bin/ltmatrix

# Or from user bin
rm ~/.local/bin/ltmatrix

# Remove configuration (optional)
rm -rf ~/.ltmatrix
```

## Troubleshooting

### "Command not found" error

```bash
# Check if binary is in PATH
which ltmatrix

# If not, add to PATH
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### "Cannot be opened because the developer cannot be verified"

This is macOS Gatekeeper protecting you. For unsigned binaries:

```bash
# Remove quarantine attribute
xattr -cr ltmatrix

# Or allow execution for this binary
sudo spctl --add --label "Executable" ltmatrix
```

### Code signing verification

```bash
# Verify signature
codesign -v $(which ltmatrix)

# Display signing details
codesign -dvv $(which ltmatrix)
```

## System Requirements

- **Intel Macs**: macOS 10.13 (High Sierra) or later
- **Apple Silicon Macs**: macOS 11.0 (Big Sur) or later
- **Memory**: 4GB RAM minimum, 8GB recommended
- **Disk**: 100MB free space
- **Network**: Internet connection for Claude API

## Support

- **Documentation**: https://github.com/bigfish/ltmatrix
- **Issues**: https://github.com/bigfish/ltmatrix/issues
- **Discussions**: https://github.com/bigfish/ltmatrix/discussions

## License

MIT License - See https://github.com/bigfish/ltmatrix/blob/main/LICENSE

## What's Next?

1. Read the full documentation: https://github.com/bigfish/ltmatrix
2. Join the community: https://github.com/bigfish/ltmatrix/discussions
3. Report issues: https://github.com/bigfish/ltmatrix/issues
4. Contribute: https://github.com/bigfish/ltmatrix/pulls

---

**Version**: PLACEHOLDER_VERSION
**Platform**: macOS Universal (Intel + Apple Silicon)
**Build Date**: PLACEHOLDER_DATE
EOF

# Replace placeholders
sed -i '' "s/PLACEHOLDER_VERSION/$VERSION/g" "$PACKAGE_DIR/README.md"
sed -i '' "s/PLACEHOLDER_DATE/$(date +%Y-%m-%d)/g" "$PACKAGE_DIR/README.md"

print_success "README created"

# Create installation script
echo ""
echo "Step 6: Creating installation script..."
cat > "$PACKAGE_DIR/install.sh" << 'EOF'
#!/bin/bash
# ltmatrix Installation Script

set -e

INSTALL_DIR="/usr/local/bin"
BINARY_NAME="ltmatrix"

echo "Installing $BINARY_NAME to $INSTALL_DIR..."

# Check if running as root
if [[ $EUID -eq 0 ]]; then
    INSTALL_CMD="cp"
else
    echo "This script requires sudo to install to $INSTALL_DIR"
    echo "Alternative: Install to ~/.local/bin (no sudo)"
    read -p "Install to ~/.local/bin instead? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        INSTALL_DIR="$HOME/.local/bin"
        mkdir -p "$INSTALL_DIR"
        INSTALL_CMD="cp"
    else
        INSTALL_CMD="sudo cp"
    fi
fi

# Copy binary
$INSTALL_CMD "$BINARY_NAME" "$INSTALL_DIR/"

# Verify installation
if [[ -x "$INSTALL_DIR/$BINARY_NAME" ]]; then
    echo "✓ Installation successful!"
    echo ""
    echo "Binary installed to: $INSTALL_DIR/$BINARY_NAME"
    echo ""
    echo "To verify:"
    echo "  $BINARY_NAME --version"
    echo ""
    echo "To get started:"
    echo "  $BINARY_NAME --help"
else
    echo "✗ Installation failed"
    exit 1
fi
EOF

chmod +x "$PACKAGE_DIR/install.sh"
print_success "Installation script created"

# Create uninstall script
echo ""
echo "Step 7: Creating uninstall script..."
cat > "$PACKAGE_DIR/uninstall.sh" << 'EOF'
#!/bin/bash
# ltmatrix Uninstallation Script

set -e

BINARY_NAME="ltmatrix"
INSTALL_DIRS=(
    "/usr/local/bin"
    "$HOME/.local/bin"
)

echo "Uninstalling $BINARY_NAME..."
echo ""

for dir in "${INSTALL_DIRS[@]}"; do
    if [[ -f "$dir/$BINARY_NAME" ]]; then
        if [[ -w "$dir/$BINARY_NAME" ]] || [[ $EUID -eq 0 ]]; then
            rm "$dir/$BINARY_NAME"
            echo "✓ Removed from $dir/"
        else
            echo "  Found in $dir/ (requires sudo to remove)"
            sudo rm "$dir/$BINARY_NAME"
            echo "✓ Removed from $dir/"
        fi
    fi
done

echo ""
echo "Uninstallation complete!"
echo ""
echo "Note: Configuration directory not removed:"
echo "  ~/.ltmatrix/"
echo ""
echo "To remove configuration:"
echo "  rm -rf ~/.ltmatrix"
EOF

chmod +x "$PACKAGE_DIR/uninstall.sh"
print_success "Uninstall script created"

# Create checksums
echo ""
echo "Step 8: Creating checksums..."
cd "$DIST_DIR"
shasum -a 256 "${PACKAGE_NAME}.tar.gz" > "${PACKAGE_NAME}.tar.gz.sha256"
print_success "SHA256 checksum created"

# Display package contents
echo ""
echo "Step 9: Package contents..."
echo ""
ls -lh "$PACKAGE_DIR"
echo ""

# Create tarball
echo ""
echo "Step 10: Creating tarball..."
tar -czf "$TARBALL" -C "$DIST_DIR" "$PACKAGE_NAME"
print_success "Tarball created"

# Get file size
TARBALL_SIZE=$(stat -f%z "$TARBALL")
TARBALL_SIZE_MB=$(echo "scale=2; $TARBALL_SIZE / 1024 / 1024" | bc)

# Final summary
echo ""
echo "=========================================="
echo "Distribution Package Created"
echo "=========================================="
echo ""
echo "Package:  $PACKAGE_NAME"
echo "Version:  $VERSION"
echo "Tarball:  $TARBALL"
echo "Size:     ${TARBALL_SIZE_MB} MB"
echo ""
echo "Contents:"
echo "  - ltmatrix          Universal binary"
echo "  - README.md          Documentation"
echo "  - install.sh        Installation script"
echo "  - uninstall.sh      Uninstallation script"
echo ""
echo "Checksum: ${PACKAGE_NAME}.tar.gz.sha256"
echo ""
echo "To install:"
echo "  tar -xzf $TARBALL"
echo "  cd $PACKAGE_NAME"
echo "  ./install.sh"
echo ""
echo "To verify checksum:"
echo "  shasum -a 256 $TARBALL | diff - ${PACKAGE_NAME}.tar.gz.sha256 -"
echo ""
