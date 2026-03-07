#!/bin/bash
# Install Git hooks for ltmatrix
#
# This script configures Git to use the hooks in .githooks/ directory
# Run this script from the project root after cloning the repository
#
# Usage:
#   ./scripts/install-hooks.sh
#
# Or with make:
#   make install-hooks

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GITHOOKS_DIR="$PROJECT_ROOT/.githooks"

echo -e "${BLUE}=== ltmatrix Git Hooks Installer ===${NC}"
echo ""

# Check if we're in a Git repository
if [ ! -d "$PROJECT_ROOT/.git" ]; then
    echo -e "${RED}Error: Not a Git repository${NC}"
    echo "Please run this script from within the ltmatrix repository"
    exit 1
fi

# Check if .githooks directory exists
if [ ! -d "$GITHOOKS_DIR" ]; then
    echo -e "${RED}Error: .githooks directory not found${NC}"
    exit 1
fi

# Make all hooks executable
echo -e "${YELLOW}Making hooks executable...${NC}"
chmod +x "$GITHOOKS_DIR"/* 2>/dev/null || true

# Configure Git to use .githooks directory
echo -e "${YELLOW}Configuring Git to use .githooks directory...${NC}"
git -C "$PROJECT_ROOT" config core.hooksPath .githooks

# Verify configuration
HOOKS_PATH=$(git -C "$PROJECT_ROOT" config --get core.hooksPath)

if [ "$HOOKS_PATH" = ".githooks" ]; then
    echo -e "${GREEN}✓ Git hooks installed successfully!${NC}"
    echo ""
    echo -e "${BLUE}Installed hooks:${NC}"
    echo "  • pre-commit  - Runs fmt check, clippy, and fast tests"
    echo "  • pre-push    - Runs full test suite and release build"
    echo "  • commit-msg  - Validates conventional commit format"
    echo ""
    echo -e "${BLUE}To bypass hooks temporarily:${NC}"
    echo "  git commit --no-verify   # Skip pre-commit and commit-msg"
    echo "  git push --no-verify     # Skip pre-push"
    echo ""
    echo -e "${BLUE}To uninstall hooks:${NC}"
    echo "  git config --unset core.hooksPath"
else
    echo -e "${RED}✗ Failed to configure Git hooks${NC}"
    exit 1
fi

# Offer to run initial checks
echo ""
read -p "$(echo -e ${YELLOW}Run initial checks now? [y/N]: ${NC})" -n 1 -r
echo ""
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    echo -e "${YELLOW}Running pre-commit checks...${NC}"

    # Run formatting check
    echo -e "${YELLOW}[1/3] Checking formatting...${NC}"
    if cargo fmt --check 2>/dev/null; then
        echo -e "${GREEN}✓ Formatting OK${NC}"
    else
        echo -e "${RED}✗ Formatting issues found (run 'cargo fmt')${NC}"
    fi

    # Run clippy
    echo -e "${YELLOW}[2/3] Running clippy...${NC}"
    if cargo clippy --all-targets --all-features -- -D warnings 2>/dev/null; then
        echo -e "${GREEN}✓ Clippy OK${NC}"
    else
        echo -e "${RED}✗ Clippy issues found${NC}"
    fi

    # Run tests
    echo -e "${YELLOW}[3/3] Running tests...${NC}"
    if cargo test --lib -- --quiet 2>/dev/null; then
        echo -e "${GREEN}✓ Tests OK${NC}"
    else
        echo -e "${RED}✗ Some tests failed${NC}"
    fi

    echo ""
    echo -e "${GREEN}Initial checks complete!${NC}"
fi

exit 0
