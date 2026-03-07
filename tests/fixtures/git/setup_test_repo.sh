#!/bin/bash
# Script to create a test git repository with sample commits
#
# Usage: ./setup_test_repo.sh <target_directory>
#
# Creates a git repository with:
# - Initial commit
# - Feature branch with commits
# - Merged changes
# - Sample tags

set -e

TARGET_DIR="${1:-./test-repo}"

# Create directory
mkdir -p "$TARGET_DIR"
cd "$TARGET_DIR"

# Initialize repo
git init
git config user.email "test@example.com"
git config user.name "Test User"

# Initial commit
echo "# Test Project" > README.md
echo "Initial content" > main.txt
git add .
git commit -m "Initial commit"

# Add more content
echo "Feature A implementation" > feature_a.txt
git add feature_a.txt
git commit -m "Add feature A"

# Create feature branch
git checkout -b feature/branch-test
echo "Feature B implementation" > feature_b.txt
git add feature_b.txt
git commit -m "Add feature B"

# Go back to main and merge
git checkout main
git merge feature/branch-test -m "Merge feature/branch-test"

# Create a tag
git tag v1.0.0

echo "Test repository created at $TARGET_DIR"
