# Makefile for ltmatrix
#
# Usage:
#   make <target>
#
# Targets:
#   build          - Build debug version
#   release        - Build release version
#   test           - Run all tests
#   test-unit      - Run unit tests only
#   test-integration - Run integration tests only
#   fmt            - Format code
#   fmt-check      - Check code formatting
#   clippy         - Run clippy
#   check          - Run all checks (fmt, clippy, test)
#   install-hooks  - Install Git hooks
#   uninstall-hooks - Uninstall Git hooks
#   docs           - Build documentation
#   clean          - Clean build artifacts
#   help           - Show this help

.PHONY: build release test test-unit test-integration fmt fmt-check clippy check install-hooks uninstall-hooks docs clean help

# Default target
.DEFAULT_GOAL := help

# ====================
# Build Targets
# ====================

build:
	@echo "Building debug version..."
	cargo build

release:
	@echo "Building release version..."
	cargo build --release

# ====================
# Test Targets
# ====================

test:
	@echo "Running all tests..."
	cargo test --all-features

test-unit:
	@echo "Running unit tests..."
	cargo test --lib

test-integration:
	@echo "Running integration tests..."
	cargo test --test '*'

# ====================
# Code Quality Targets
# ====================

fmt:
	@echo "Formatting code..."
	cargo fmt

fmt-check:
	@echo "Checking code formatting..."
	cargo fmt --check

clippy:
	@echo "Running clippy..."
	cargo clippy --all-targets --all-features -- -D warnings

check: fmt-check clippy test
	@echo "All checks passed!"

# ====================
# Git Hooks Targets
# ====================

install-hooks:
	@echo "Installing Git hooks..."
	@if command -v bash >/dev/null 2>&1; then \
		bash scripts/install-hooks.sh; \
	elif command -v cmd >/dev/null 2>&1; then \
		scripts\\install-hooks.bat; \
	else \
		git config core.hooksPath .githooks && echo "Hooks installed"; \
	fi

uninstall-hooks:
	@echo "Uninstalling Git hooks..."
	git config --unset core.hooksPath 2>/dev/null || true
	@echo "Hooks uninstalled"

# ====================
# Documentation Targets
# ====================

docs:
	@echo "Building documentation..."
	cargo doc --no-deps

docs-open:
	@echo "Building and opening documentation..."
	cargo doc --open --no-deps

# ====================
# Utility Targets
# ====================

clean:
	@echo "Cleaning build artifacts..."
	cargo clean

help:
	@echo "ltmatrix - Long-Time Agent Orchestrator"
	@echo ""
	@echo "Usage: make <target>"
	@echo ""
	@echo "Build Targets:"
	@echo "  build          - Build debug version"
	@echo "  release        - Build release version"
	@echo ""
	@echo "Test Targets:"
	@echo "  test           - Run all tests"
	@echo "  test-unit      - Run unit tests only"
	@echo "  test-integration - Run integration tests only"
	@echo ""
	@echo "Code Quality Targets:"
	@echo "  fmt            - Format code"
	@echo "  fmt-check      - Check code formatting"
	@echo "  clippy         - Run clippy"
	@echo "  check          - Run all checks (fmt, clippy, test)"
	@echo ""
	@echo "Git Hooks Targets:"
	@echo "  install-hooks  - Install Git hooks"
	@echo "  uninstall-hooks - Uninstall Git hooks"
	@echo ""
	@echo "Documentation Targets:"
	@echo "  docs           - Build documentation"
	@echo "  docs-open      - Build and open documentation"
	@echo ""
	@echo "Utility Targets:"
	@echo "  clean          - Clean build artifacts"
	@echo "  help           - Show this help"
