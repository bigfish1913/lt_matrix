#!/bin/bash
# Uninstall Git hooks for ltmatrix
#
# This script removes the Git hooks configuration
# Run this script from the project root
#
# Usage:
#   ./scripts/uninstall-hooks.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo -e "${YELLOW}Uninstalling Git hooks...${NC}"

# Remove the hooksPath configuration
git -C "$PROJECT_ROOT" config --unset core.hooksPath 2>/dev/null || true

# Verify configuration is removed
HOOKS_PATH=$(git -C "$PROJECT_ROOT" config --get core.hooksPath 2>/dev/null || echo "")

if [ -z "$HOOKS_PATH" ]; then
    echo -e "${GREEN}✓ Git hooks uninstalled successfully!${NC}"
    echo ""
    echo "Git will now use the default hooks location (.git/hooks/)"
else
    echo -e "${RED}✗ Failed to uninstall Git hooks${NC}"
    exit 1
fi

exit 0
