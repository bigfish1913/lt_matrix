//! Release management module
//!
//! This module provides version management and release automation for ltmatrix.
//! It handles semantic versioning, changelog generation, and release orchestration.

mod version;
mod changelog;
mod precheck;
mod git_ops;

pub use version::{Version, VersionBump, BumpError};
pub use changelog::{Changelog, ChangelogEntry, ChangelogSection};
pub use precheck::{PreReleaseCheck, CheckResult, CheckStatus};
pub use git_ops::{GitOperations, GitError};

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Release manager handles the complete release process
pub struct ReleaseManager {
    project_root: PathBuf,
    version: Version,
    dry_run: bool,
}

impl ReleaseManager {
    /// Create a new release manager
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            version: Version::current(&project_root).unwrap_or_default(),
            dry_run: false,
        }
    }

    /// Enable dry-run mode
    pub fn dry_run(mut self, enabled: bool) -> Self {
        self.dry_run = enabled;
        self
    }

    /// Get the current version
    pub fn current_version(&self) -> &Version {
        &self.version
    }

    /// Bump version by specified amount
    pub fn bump_version(&mut self, bump: VersionBump) -> Result<Version> {
        let new_version = self.version.bump(bump)?;

        if !self.dry_run {
            new_version.write_to_cargo_toml(&self.project_root)?;
            self.version = new_version.clone();
        }

        Ok(new_version)
    }

    /// Run pre-release checks
    pub fn run_pre_checks(&self) -> Result<PreReleaseCheck> {
        let mut checks = PreReleaseCheck::new(&self.project_root);
        checks.run_all()?;
        Ok(checks)
    }

    /// Generate changelog for the release
    pub fn generate_changelog(&self, from_tag: Option<&str>) -> Result<Changelog> {
        let changelog = Changelog::generate(&self.project_root, from_tag)?;
        Ok(changelog)
    }

    /// Update CHANGELOG.md file
    pub fn update_changelog_file(&self, changelog: &Changelog, version: &Version) -> Result<()> {
        if self.dry_run {
            println!("[Dry run] Would update CHANGELOG.md with version {}", version);
            return Ok(());
        }

        changelog.update_file(&self.project_root, version)?;
        Ok(())
    }

    /// Create git tag for the release
    pub fn create_tag(&self, version: &Version, message: &str) -> Result<()> {
        let git = GitOperations::new(&self.project_root)?;

        if self.dry_run {
            println!("[Dry run] Would create tag v{} with message: {}", version, message);
            return Ok(());
        }

        git.create_tag(&format!("v{}", version), message)?;
        Ok(())
    }

    /// Perform complete release
    pub fn release(&mut self, bump: VersionBump, skip_checks: bool) -> Result<()> {
        // Step 1: Pre-release checks
        if !skip_checks {
            println!("Running pre-release checks...");
            let checks = self.run_pre_checks()?;

            if !checks.all_passed() {
                anyhow::bail!("Pre-release checks failed:\n{}", checks.summary());
            }
            println!("✓ All pre-release checks passed");
        }

        // Step 2: Bump version
        println!("Bumping version from {}...", self.version);
        let new_version = self.bump_version(bump)?;
        println!("✓ Version bumped to {}", new_version);

        // Step 3: Generate changelog
        println!("Generating changelog...");
        let last_tag = GitOperations::new(&self.project_root)?
            .get_last_tag()
            .ok()
            .flatten();
        let changelog = self.generate_changelog(last_tag.as_deref())?;
        println!("✓ Changelog generated ({} entries)", changelog.entry_count());

        // Step 4: Update changelog file
        self.update_changelog_file(&changelog, &new_version)?;
        println!("✓ CHANGELOG.md updated");

        // Step 5: Commit changes
        let git = GitOperations::new(&self.project_root)?;
        if !self.dry_run {
            git.commit_version_bump(&new_version)?;
            println!("✓ Changes committed");
        }

        // Step 6: Create tag
        let tag_message = format!("Release v{}", new_version);
        self.create_tag(&new_version, &tag_message)?;
        println!("✓ Tag v{} created", new_version);

        println!("\n🎉 Release v{} prepared successfully!", new_version);

        if self.dry_run {
            println!("\n[Dry run mode] No changes were made. Run without --dry-run to apply changes.");
        } else {
            println!("\nNext steps:");
            println!("  1. Review the changes");
            println!("  2. Push to remote: git push origin main --tags");
            println!("  3. Monitor CI/CD pipeline");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let version = Version::parse("1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_version_bump_patch() {
        let version = Version::parse("1.2.3").unwrap();
        let bumped = version.bump(VersionBump::Patch).unwrap();
        assert_eq!(bumped.to_string(), "1.2.4");
    }

    #[test]
    fn test_version_bump_minor() {
        let version = Version::parse("1.2.3").unwrap();
        let bumped = version.bump(VersionBump::Minor).unwrap();
        assert_eq!(bumped.to_string(), "1.3.0");
    }

    #[test]
    fn test_version_bump_major() {
        let version = Version::parse("1.2.3").unwrap();
        let bumped = version.bump(VersionBump::Major).unwrap();
        assert_eq!(bumped.to_string(), "2.0.0");
    }
}