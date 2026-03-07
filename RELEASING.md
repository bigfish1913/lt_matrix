# Release Process

This document describes the release process for ltmatrix, including versioning strategy, changelog management, and automation.

## Table of Contents

1. [Versioning Strategy](#versioning-strategy)
2. [Release Types](#release-types)
3. [Pre-Release Checklist](#pre-release-checklist)
4. [Release Procedure](#release-procedure)
5. [Changelog Management](#changelog-management)
6. [Automated Changelog Generation](#automated-changelog-generation)
7. [Breaking Changes](#breaking-changes)
8. [Post-Release Tasks](#post-release-tasks)

---

## Versioning Strategy

ltmatrix follows [Semantic Versioning 2.0.0](https://semver.org/).

### Version Format

```
MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]
```

### Version Components

| Component | When to Increment | Example |
|-----------|-------------------|---------|
| **MAJOR** | Breaking changes | 0.1.0 → 1.0.0 |
| **MINOR** | New features (backwards-compatible) | 0.1.0 → 0.2.0 |
| **PATCH** | Bug fixes (backwards-compatible) | 0.1.0 → 0.1.1 |
| **PRERELEASE** | Pre-release versions | 1.0.0-alpha.1, 1.0.0-beta.2, 1.0.0-rc.1 |
| **BUILD** | Build metadata | 1.0.0+build.123 |

### Pre-Release Naming Convention

- `alpha`: Early testing, may have incomplete features
- `beta`: Feature complete, needs testing
- `rc`: Release candidate, ready for production pending final verification

Example progression:
```
1.0.0-alpha.1 → 1.0.0-alpha.2 → 1.0.0-beta.1 → 1.0.0-rc.1 → 1.0.0
```

---

## Release Types

### Patch Release (0.0.Z)

**Triggers:**
- Bug fixes that don't change behavior
- Documentation updates
- Internal refactoring
- Performance improvements (no API changes)

**Example Commits:**
```
fix(cli): correct version flag display
fix(pipeline): handle empty task list gracefully
docs: update installation instructions
refactor(agent): simplify session management
```

### Minor Release (0.Y.0)

**Triggers:**
- New CLI flags/options
- New agent backends
- New configuration options
- New pipeline stages
- Performance improvements with visible impact

**Example Commits:**
```
feat(cli): add --parallel flag for concurrent execution
feat(agent): add Gemini agent backend
feat(config): support environment variable interpolation
feat(pipeline): add code formatting stage
```

### Major Release (X.0.0)

**Triggers:**
- Removed or renamed CLI flags
- Configuration format changes
- Removed public APIs
- Changed pipeline execution order
- Changed default behaviors

**Example Commits:**
```
feat(cli)!: remove deprecated --no-verify flag
feat(config)!: change config file format to YAML
feat(pipeline)!: reorder test stage before execute
```

---

## Pre-Release Checklist

Before creating a release, verify the following:

### Code Quality

- [ ] All tests pass: `cargo test --all`
- [ ] Code is formatted: `cargo fmt --check`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] Documentation builds: `cargo doc --no-deps`

### Changelog

- [ ] CHANGELOG.md is updated with all changes
- [ ] All changes are properly categorized
- [ ] Breaking changes are documented with migration guides
- [ ] Version number and date are correct

### Documentation

- [ ] README.md reflects current functionality
- [ ] CLI help text is accurate: `ltmatrix --help`
- [ ] Configuration examples are valid
- [ ] Migration guide exists for breaking changes

### Build Verification

- [ ] Linux x86_64 builds successfully
- [ ] macOS (Intel and ARM64) builds successfully
- [ ] Windows x86_64 builds successfully
- [ ] Binary size is reasonable (< 20MB)
- [ ] Binary runs without errors

---

## Release Procedure

### Step 1: Prepare Release Branch

```bash
# Ensure you're on main and up to date
git checkout main
git pull origin main

# Create release branch
git checkout -b release/vX.Y.Z
```

### Step 2: Update Version

Update version in the following files:

**Cargo.toml:**
```toml
[package]
version = "X.Y.Z"
```

**CHANGELOG.md:**
Move items from `[Unreleased]` to new version section.

### Step 3: Update Changelog

```bash
# Generate changelog from commits
./scripts/generate-changelog.sh vX.Y.Z

# Or manually update CHANGELOG.md
```

### Step 4: Commit and Tag

```bash
# Stage changes
git add Cargo.toml Cargo.lock CHANGELOG.md

# Create release commit
git commit -m "chore: release v$(grep '^version =' Cargo.toml | cut -d'"' -f2)"

# Create annotated tag
git tag -a vX.Y.Z -m "Release vX.Y.Z

## Summary
<Brief summary of changes>

## Breaking Changes
<List breaking changes if any>

## New Features
<List major new features>

## Bug Fixes
<List significant bug fixes>
"
```

### Step 5: Push and Create Release

```bash
# Push branch and tag
git push origin release/vX.Y.Z
git push origin vX.Y.Z

# Create GitHub release (manually or via CLI)
gh release create vX.Y.Z \
  --title "vX.Y.Z" \
  --notes-file RELEASE_NOTES.md \
  target/x86_64-unknown-linux-musl/release/ltmatrix \
  target/aarch64-unknown-linux-musl/release/ltmatrix \
  target/x86_64-apple-darwin/release/ltmatrix \
  target/aarch64-apple-darwin/release/ltmatrix \
  target/x86_64-pc-windows-msvc/release/ltmatrix.exe
```

### Step 6: Merge to Main

```bash
# Switch to main
git checkout main

# Merge release branch
git merge --no-ff release/vX.Y.Z -m "Merge release/vX.Y.Z"

# Push to main
git push origin main

# Clean up
git branch -d release/vX.Y.Z
```

---

## Changelog Management

### CHANGELOG.md Structure

```markdown
# Changelog

## [Unreleased]

### Added
- New feature description (#PR)

### Changed
- Change description (#PR)

### Deprecated
- Deprecation notice

### Removed
- Removal notice

### Fixed
- Bug fix description (#PR)

### Security
- Security fix description

## [X.Y.Z] - YYYY-MM-DD
...
```

### Adding Entries

**When merging a PR:**
1. Add entry to appropriate section under `[Unreleased]`
2. Include PR number: `Description (#123)`
3. Keep entries concise and user-focused

**Example entries:**
```markdown
### Added
- `--parallel` flag for concurrent task execution (#42)
- Gemini agent backend support (#45)

### Changed
- Improved error messages for failed tasks (#43)

### Fixed
- Fixed session reuse in retry scenarios (#44)
```

### Categorization Guidelines

| Category | Description |
|----------|-------------|
| **Added** | New features, new flags, new modules |
| **Changed** | Changes to existing functionality |
| **Deprecated** | Features to be removed in future |
| **Removed** | Features removed in this release |
| **Fixed** | Bug fixes and error handling |
| **Security** | Security-related changes |

---

## Automated Changelog Generation

### Script Location

```
scripts/generate-changelog.sh
```

### Usage

```bash
# Generate changelog since last tag
./scripts/generate-changelog.sh

# Generate changelog since specific tag
./scripts/generate-changelog.sh v0.1.0

# Generate changelog between two tags
./scripts/generate-changelog.sh v0.1.0 v0.2.0
```

### How It Works

1. **Collect Commits**: `git log --pretty=format:"%s" TAG..HEAD`
2. **Parse Conventional Commits**: Extract type, scope, description
3. **Categorize**: Map commit types to changelog sections
4. **Group by Scope**: Organize entries by affected component
5. **Generate Output**: Create markdown-formatted changelog

### Commit Type Mapping

| Commit Type | Changelog Section |
|-------------|-------------------|
| `feat` | Added |
| `change` | Changed |
| `deprecate` | Deprecated |
| `remove` | Removed |
| `fix` | Fixed |
| `security` | Security |
| `docs` | Changed |
| `refactor` | (Not included) |
| `test` | (Not included) |
| `chore` | (Not included) |

### Example Output

```markdown
### Added

#### CLI
- `--parallel` flag for concurrent task execution (#42)
- Shell completions for Elvish (#38)

#### Agent
- Gemini agent backend support (#45)
- Session warmup queries (#41)

### Fixed

#### Pipeline
- Fixed session reuse in retry scenarios (#44)

#### Git
- Fixed branch creation for task execution (#43)
```

---

## Breaking Changes

### Documentation Requirements

Breaking changes MUST include:

1. **Changelog Entry**: Under "Breaking Changes" subsection
2. **Migration Guide**: Step-by-step instructions
3. **Deprecation Notice**: In prior release if possible
4. **Documentation Update**: Update all affected docs

### Example Breaking Change Entry

```markdown
### Breaking Changes

#### Configuration File Format Changed

The configuration file format has changed from TOML to YAML.

**Migration Guide:**

1. Convert existing `~/.ltmatrix/config.toml` to YAML:
   ```bash
   ltmatrix config migrate --format yaml
   ```

2. Or manually convert:
   ```yaml
   # Before (TOML)
   [default]
   agent = "claude"

   # After (YAML)
   default:
     agent: claude
   ```

3. Update any scripts that parse the config file.

**Deprecation:** The TOML format was deprecated in v0.9.0.
```

### Breaking Change Checklist

- [ ] Breaking change clearly identified in commit message (`feat!:` or `feat(scope)!:`)
- [ ] CHANGELOG.md has "Breaking Changes" subsection
- [ ] Migration guide provided
- [ ] Prior version has deprecation warning (if applicable)
- [ ] All documentation updated
- [ ] Version bump is MAJOR

---

## Post-Release Tasks

### Immediate (within 1 hour)

- [ ] Verify GitHub release is published
- [ ] Verify binaries are downloadable
- [ ] Test binary installation on each platform
- [ ] Verify Homebrew formula updates (if applicable)

### Within 24 hours

- [ ] Update documentation site (if applicable)
- [ ] Announce release on relevant channels
- [ ] Update `latest` tag in container registry
- [ ] Verify download statistics

### Within 1 week

- [ ] Monitor issue tracker for release-related bugs
- [ ] Prepare patch release if critical bugs found
- [ ] Update roadmap based on feedback

---

## Release Commands Quick Reference

```bash
# Create a patch release
cargo release patch

# Create a minor release
cargo release minor

# Create a major release
cargo release major

# Create a pre-release
cargo release minor --pre-release alpha.1

# Dry run (preview without executing)
cargo release patch --dry-run
```

---

## Troubleshooting

### Build Failures

**Problem**: Build fails for specific target.

**Solution**:
```bash
# Ensure target is installed
rustup target add x86_64-unknown-linux-musl

# Use cross for cross-compilation
cross build --release --target x86_64-unknown-linux-musl
```

### Changelog Generation Issues

**Problem**: Commits not appearing in changelog.

**Solution**:
- Ensure commits use conventional commit format
- Check that commit type is not filtered (refactor, test, chore)
- Verify git history is correct

### Version Mismatch

**Problem**: Version mismatch between Cargo.toml and git tag.

**Solution**:
```bash
# Update version in Cargo.toml
# Then recreate tag
git tag -d vX.Y.Z
git tag -a vX.Y.Z -m "Release vX.Y.Z"
```

---

## Appendix: GitHub Actions Release Workflow

The following workflow automates binary builds on tag push:

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
          - target: aarch64-unknown-linux-musl
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Package (Unix)
        if: runner.os != 'Windows'
        run: |
          cd target/${{ matrix.target }}/release
          tar -czf ltmatrix-${{ matrix.target }}.tar.gz ltmatrix

      - name: Package (Windows)
        if: runner.os == 'Windows'
        run: |
          cd target/${{ matrix.target }}/release
          7z a ltmatrix-${{ matrix.target }}.zip ltmatrix.exe

      - uses: softprops/action-gh-release@v1
        with:
          files: target/${{ matrix.target }}/release/ltmatrix-*

  # Additional job for macOS universal binary
  universal-macos:
    runs-on: macos-latest
    needs: build
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-apple-darwin,aarch64-apple-darwin

      - name: Build Intel
        run: cargo build --release --target x86_64-apple-darwin

      - name: Build Apple Silicon
        run: cargo build --release --target aarch64-apple-darwin

      - name: Create Universal Binary
        run: |
          mkdir -p target/universal
          lipo -create \
            target/x86_64-apple-darwin/release/ltmatrix \
            target/aarch64-apple-darwin/release/ltmatrix \
            -output target/universal/ltmatrix

      - name: Package
        run: |
          cd target/universal
          tar -czf ltmatrix-universal-apple-darwin.tar.gz ltmatrix

      - uses: softprops/action-gh-release@v1
        with:
          files: target/universal/ltmatrix-*
```

---

## Version History

| Version | Date | Summary |
|---------|------|---------|
| 0.1.0 | 2025-01-15 | Initial Rust release |