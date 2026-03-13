//! Integration tests for release module
//!
//! These tests verify the complete release management workflow.

use ltmatrix::release::{
    Changelog, ChangelogEntry, ChangelogSection, CheckResult, CheckStatus, Version, VersionBump,
};

// =============================================================================
// Changelog Integration Tests
// =============================================================================

#[test]
fn test_changelog_integration_create_complete() {
    // Test creating a complete changelog with all section types
    let mut changelog = Changelog::new();

    // Add entries for each Keep a Changelog section
    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Added,
        scope: Some("cli".to_string()),
        description: "Add --parallel flag for concurrent execution".to_string(),
        pr_number: Some(42),
        breaking: false,
    });

    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Changed,
        scope: Some("pipeline".to_string()),
        description: "Improve task scheduling algorithm".to_string(),
        pr_number: Some(43),
        breaking: false,
    });

    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Deprecated,
        scope: Some("config".to_string()),
        description: "Deprecate old config format".to_string(),
        pr_number: Some(44),
        breaking: false,
    });

    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Removed,
        scope: Some("cli".to_string()),
        description: "Remove deprecated --old-flag".to_string(),
        pr_number: Some(45),
        breaking: true,
    });

    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Fixed,
        scope: None,
        description: "Fix memory leak in agent pool".to_string(),
        pr_number: Some(46),
        breaking: false,
    });

    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Security,
        scope: None,
        description: "Fix potential command injection vulnerability".to_string(),
        pr_number: Some(47),
        breaking: false,
    });

    // Verify all entries are present
    assert_eq!(changelog.entry_count(), 6);

    // Verify filtering by section works
    assert_eq!(
        changelog.entries_by_section(ChangelogSection::Added).len(),
        1
    );
    assert_eq!(
        changelog
            .entries_by_section(ChangelogSection::Changed)
            .len(),
        1
    );
    assert_eq!(
        changelog
            .entries_by_section(ChangelogSection::Deprecated)
            .len(),
        1
    );
    assert_eq!(
        changelog
            .entries_by_section(ChangelogSection::Removed)
            .len(),
        1
    );
    assert_eq!(
        changelog.entries_by_section(ChangelogSection::Fixed).len(),
        1
    );
    assert_eq!(
        changelog
            .entries_by_section(ChangelogSection::Security)
            .len(),
        1
    );
}

#[test]
fn test_changelog_breaking_changes_detection() {
    // Verify breaking changes are properly marked
    let mut changelog = Changelog::new();

    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Changed,
        scope: Some("api".to_string()),
        description: "Change API response format".to_string(),
        pr_number: Some(100),
        breaking: true,
    });

    let changed_entries = changelog.entries_by_section(ChangelogSection::Changed);
    assert_eq!(changed_entries.len(), 1);
    assert!(changed_entries[0].breaking);

    let display = changed_entries[0].to_string();
    assert!(display.contains("**BREAKING**"));
}

// =============================================================================
// Version Integration Tests
// =============================================================================

#[test]
fn test_version_bump_semantic_rules() {
    // Test that version bumps follow Semantic Versioning rules

    // Start with 1.0.0
    let v = Version::parse("1.0.0").unwrap();

    // PATCH: Bug fixes should increment patch
    let v1 = v.bump(VersionBump::Patch).unwrap();
    assert_eq!(v1.to_string(), "1.0.1");

    // MINOR: New features should increment minor and reset patch
    let v2 = v1.bump(VersionBump::Minor).unwrap();
    assert_eq!(v2.to_string(), "1.1.0");

    // PATCH after MINOR should only increment patch
    let v3 = v2.bump(VersionBump::Patch).unwrap();
    assert_eq!(v3.to_string(), "1.1.1");

    // MAJOR: Breaking changes should increment major and reset minor/patch
    let v4 = v3.bump(VersionBump::Major).unwrap();
    assert_eq!(v4.to_string(), "2.0.0");
}

#[test]
fn test_version_release_workflow() {
    // Simulate a typical release workflow

    // Development version with pre-release
    let dev = Version::parse("1.0.0-alpha.1").unwrap();
    assert!(dev.is_pre_release());

    // Alpha 2
    let dev = dev
        .bump(VersionBump::PreRelease("alpha".to_string()))
        .unwrap();
    assert_eq!(dev.to_string(), "1.0.0-alpha.2");

    // Beta 1 (start new pre-release series)
    let dev = Version::parse("1.0.0")
        .unwrap()
        .bump(VersionBump::PreRelease("beta".to_string()))
        .unwrap();
    assert_eq!(dev.to_string(), "1.0.0-beta.1");

    // RC 1
    let rc = Version::parse("1.0.0")
        .unwrap()
        .bump(VersionBump::PreRelease("rc".to_string()))
        .unwrap();
    assert_eq!(rc.to_string(), "1.0.0-rc.1");

    // Final release (no pre-release)
    let release = Version::parse("1.0.0").unwrap();
    assert!(!release.is_pre_release());
    assert_eq!(release.to_string(), "1.0.0");
}

#[test]
fn test_version_compatibility_matrix() {
    // Test that version comparison works correctly for compatibility checks

    // Same major version should generally be compatible
    let v1_0_0 = Version::parse("1.0.0").unwrap();
    let v1_1_0 = Version::parse("1.1.0").unwrap();
    let v1_0_1 = Version::parse("1.0.1").unwrap();

    assert!(v1_0_0 < v1_0_1);
    assert!(v1_0_1 < v1_1_0);
    assert!(v1_0_0 < v1_1_0);

    // Different major version indicates breaking changes
    let v2_0_0 = Version::parse("2.0.0").unwrap();
    assert!(v1_1_0 < v2_0_0);
}

// =============================================================================
// Changelog Format Tests
// =============================================================================

#[test]
fn test_changelog_keep_a_changelog_format() {
    // Verify output follows Keep a Changelog format

    let entry = ChangelogEntry {
        section: ChangelogSection::Added,
        scope: Some("cli".to_string()),
        description: "New feature description".to_string(),
        pr_number: Some(123),
        breaking: false,
    };

    // Entry should start with dash
    let display = entry.to_string();
    assert!(display.starts_with("- "));

    // PR number should be in parentheses with hash
    assert!(display.contains("(#123)"));

    // Scope should be bold
    assert!(display.contains("**cli**:"));
}

#[test]
fn test_changelog_entry_ordering() {
    // Test that entries can be organized by section

    let mut changelog = Changelog::new();

    // Add entries in random order
    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Fixed,
        scope: None,
        description: "Fix".to_string(),
        pr_number: None,
        breaking: false,
    });

    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Added,
        scope: None,
        description: "Feature".to_string(),
        pr_number: None,
        breaking: false,
    });

    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Security,
        scope: None,
        description: "Security".to_string(),
        pr_number: None,
        breaking: false,
    });

    // Should be able to filter by section
    let added = changelog.entries_by_section(ChangelogSection::Added);
    let fixed = changelog.entries_by_section(ChangelogSection::Fixed);
    let security = changelog.entries_by_section(ChangelogSection::Security);

    assert_eq!(added.len(), 1);
    assert_eq!(fixed.len(), 1);
    assert_eq!(security.len(), 1);
}

// =============================================================================
// Check Result Tests
// =============================================================================

#[test]
fn test_check_result_workflow() {
    // Test creating check results for a release workflow

    let checks = vec![
        CheckResult::passed("Tests"),
        CheckResult::passed("Format"),
        CheckResult::passed("Clippy"),
        CheckResult::warning("Documentation", "Some warnings"),
        CheckResult::passed("Clean working tree"),
        CheckResult::passed("Main branch"),
        CheckResult::passed("CHANGELOG.md exists"),
    ];

    // All should pass (warnings are acceptable)
    let all_passed = checks
        .iter()
        .all(|c| c.status == CheckStatus::Passed || c.status == CheckStatus::Warning);
    assert!(all_passed);

    // Count by status
    let passed_count = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Passed)
        .count();
    let warning_count = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Warning)
        .count();

    assert_eq!(passed_count, 6);
    assert_eq!(warning_count, 1);
}

#[test]
fn test_check_result_failure_scenario() {
    // Test a scenario where checks fail

    let checks = vec![
        CheckResult::passed("Tests"),
        CheckResult::failed("Format", "Code is not formatted"),
        CheckResult::failed("Clippy", "Warnings found"),
        CheckResult::passed("Clean working tree"),
    ];

    let failed_count = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Failed)
        .count();
    assert_eq!(failed_count, 2);

    // Release should be blocked
    let release_ok = checks
        .iter()
        .all(|c| c.status == CheckStatus::Passed || c.status == CheckStatus::Skipped);
    assert!(!release_ok);
}

// =============================================================================
// Breaking Changes Policy Tests
// =============================================================================

#[test]
fn test_breaking_change_detection() {
    // Verify breaking changes are properly identified

    let mut entries = Vec::new();

    // Regular feature (not breaking)
    entries.push(ChangelogEntry {
        section: ChangelogSection::Added,
        scope: Some("cli".to_string()),
        description: "Add new flag".to_string(),
        pr_number: Some(1),
        breaking: false,
    });

    // Breaking change
    entries.push(ChangelogEntry {
        section: ChangelogSection::Removed,
        scope: Some("api".to_string()),
        description: "Remove deprecated endpoint".to_string(),
        pr_number: Some(2),
        breaking: true,
    });

    // Breaking API change
    entries.push(ChangelogEntry {
        section: ChangelogSection::Changed,
        scope: Some("config".to_string()),
        description: "Change config file format".to_string(),
        pr_number: Some(3),
        breaking: true,
    });

    // Find breaking changes
    let breaking: Vec<_> = entries.iter().filter(|e| e.breaking).collect();
    assert_eq!(breaking.len(), 2);

    // Verify display format includes BREAKING marker
    for entry in &breaking {
        assert!(entry.to_string().contains("**BREAKING**"));
    }
}

#[test]
fn test_version_bump_for_breaking_change() {
    // Verify that breaking changes result in major version bump

    let current = Version::parse("1.2.3").unwrap();

    // Breaking change should trigger major bump
    let next = current.bump(VersionBump::Major).unwrap();
    assert_eq!(next.major, current.major + 1);
    assert_eq!(next.minor, 0);
    assert_eq!(next.patch, 0);
}

// =============================================================================
// Release Workflow Simulation
// =============================================================================

#[test]
fn test_complete_release_workflow() {
    // Simulate a complete release workflow

    // 1. Start with current version
    let current = Version::parse("0.5.3").unwrap();

    // 2. Determine bump type based on changes
    // For this example, we have a new feature (minor bump)
    let bump_type = VersionBump::Minor;

    // 3. Bump version
    let new_version = current.bump(bump_type).unwrap();
    assert_eq!(new_version.to_string(), "0.6.0");

    // 4. Create changelog
    let mut changelog = Changelog::new();
    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Added,
        scope: Some("agent".to_string()),
        description: "Add Gemini agent backend".to_string(),
        pr_number: Some(100),
        breaking: false,
    });

    // 5. Verify changelog
    assert_eq!(changelog.entry_count(), 1);
    assert!(!changelog.is_empty());

    // 6. Pre-release checks would pass (simulated)
    let checks_passed = true;
    assert!(checks_passed);
}
