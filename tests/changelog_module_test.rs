//! Tests for changelog module
//!
//! These tests verify the changelog generation and management functionality
//! following Keep a Changelog format.

use ltmatrix::release::{Changelog, ChangelogEntry, ChangelogSection};

#[test]
fn test_changelog_section_display() {
    // Verify all Keep a Changelog sections have correct display names
    assert_eq!(ChangelogSection::Added.to_string(), "Added");
    assert_eq!(ChangelogSection::Changed.to_string(), "Changed");
    assert_eq!(ChangelogSection::Deprecated.to_string(), "Deprecated");
    assert_eq!(ChangelogSection::Removed.to_string(), "Removed");
    assert_eq!(ChangelogSection::Fixed.to_string(), "Fixed");
    assert_eq!(ChangelogSection::Security.to_string(), "Security");
}

#[test]
fn test_changelog_section_from_str() {
    // Test parsing from string (case-insensitive)
    use std::str::FromStr;

    assert_eq!(
        ChangelogSection::from_str("added").unwrap(),
        ChangelogSection::Added
    );
    assert_eq!(
        ChangelogSection::from_str("ADDED").unwrap(),
        ChangelogSection::Added
    );
    assert_eq!(
        ChangelogSection::from_str("Changed").unwrap(),
        ChangelogSection::Changed
    );
    assert_eq!(
        ChangelogSection::from_str("deprecated").unwrap(),
        ChangelogSection::Deprecated
    );
    assert_eq!(
        ChangelogSection::from_str("removed").unwrap(),
        ChangelogSection::Removed
    );
    assert_eq!(
        ChangelogSection::from_str("fixed").unwrap(),
        ChangelogSection::Fixed
    );
    assert_eq!(
        ChangelogSection::from_str("security").unwrap(),
        ChangelogSection::Security
    );
}

#[test]
fn test_changelog_section_invalid() {
    use std::str::FromStr;

    // Invalid section should error
    assert!(ChangelogSection::from_str("invalid").is_err());
    assert!(ChangelogSection::from_str("unknown").is_err());
}

#[test]
fn test_changelog_entry_display_simple() {
    let entry = ChangelogEntry {
        section: ChangelogSection::Added,
        scope: None,
        description: "Add new feature".to_string(),
        pr_number: None,
        breaking: false,
    };

    assert_eq!(entry.to_string(), "- Add new feature");
}

#[test]
fn test_changelog_entry_with_scope() {
    let entry = ChangelogEntry {
        section: ChangelogSection::Added,
        scope: Some("cli".to_string()),
        description: "Add new flag".to_string(),
        pr_number: None,
        breaking: false,
    };

    assert_eq!(entry.to_string(), "- **cli**: Add new flag");
}

#[test]
fn test_changelog_entry_with_pr_number() {
    let entry = ChangelogEntry {
        section: ChangelogSection::Fixed,
        scope: None,
        description: "Fix bug in pipeline".to_string(),
        pr_number: Some(42),
        breaking: false,
    };

    assert_eq!(entry.to_string(), "- Fix bug in pipeline (#42)");
}

#[test]
fn test_changelog_entry_breaking_change() {
    let entry = ChangelogEntry {
        section: ChangelogSection::Changed,
        scope: None,
        description: "API endpoint changed".to_string(),
        pr_number: None,
        breaking: true,
    };

    assert_eq!(entry.to_string(), "- **BREAKING** API endpoint changed");
}

#[test]
fn test_changelog_entry_full_format() {
    let entry = ChangelogEntry {
        section: ChangelogSection::Removed,
        scope: Some("config".to_string()),
        description: "Remove deprecated option".to_string(),
        pr_number: Some(123),
        breaking: true,
    };

    assert_eq!(
        entry.to_string(),
        "- **BREAKING** **config**: Remove deprecated option (#123)"
    );
}

#[test]
fn test_changelog_new() {
    let changelog = Changelog::new();
    assert!(changelog.is_empty());
    assert_eq!(changelog.entry_count(), 0);
}

#[test]
fn test_changelog_add_entries() {
    let mut changelog = Changelog::new();

    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Added,
        scope: Some("cli".to_string()),
        description: "Add --parallel flag".to_string(),
        pr_number: Some(42),
        breaking: false,
    });

    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Fixed,
        scope: None,
        description: "Fix crash on empty input".to_string(),
        pr_number: Some(43),
        breaking: false,
    });

    assert!(!changelog.is_empty());
    assert_eq!(changelog.entry_count(), 2);
}

#[test]
fn test_changelog_entries_by_section() {
    let mut changelog = Changelog::new();

    // Add various entries
    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Added,
        scope: None,
        description: "Feature 1".to_string(),
        pr_number: None,
        breaking: false,
    });

    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Added,
        scope: None,
        description: "Feature 2".to_string(),
        pr_number: None,
        breaking: false,
    });

    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Fixed,
        scope: None,
        description: "Bug fix".to_string(),
        pr_number: None,
        breaking: false,
    });

    changelog.entries.push(ChangelogEntry {
        section: ChangelogSection::Security,
        scope: None,
        description: "Security patch".to_string(),
        pr_number: None,
        breaking: false,
    });

    // Verify filtering by section
    let added = changelog.entries_by_section(ChangelogSection::Added);
    assert_eq!(added.len(), 2);

    let fixed = changelog.entries_by_section(ChangelogSection::Fixed);
    assert_eq!(fixed.len(), 1);

    let security = changelog.entries_by_section(ChangelogSection::Security);
    assert_eq!(security.len(), 1);

    let deprecated = changelog.entries_by_section(ChangelogSection::Deprecated);
    assert_eq!(deprecated.len(), 0);
}

#[test]
fn test_changelog_all_sections_covered() {
    // Verify that all Keep a Changelog sections are supported
    let sections = [
        ChangelogSection::Added,
        ChangelogSection::Changed,
        ChangelogSection::Deprecated,
        ChangelogSection::Removed,
        ChangelogSection::Fixed,
        ChangelogSection::Security,
    ];

    // Each section should have a valid display representation
    for section in sections {
        let display = section.to_string();
        assert!(!display.is_empty());
        assert!(display.chars().next().unwrap().is_uppercase());
    }
}
