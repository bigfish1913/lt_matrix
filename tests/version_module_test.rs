//! Tests for version module
//!
//! These tests verify the semantic versioning implementation
//! following Semantic Versioning 2.0.0 specification.

use ltmatrix::release::{BumpError, Version, VersionBump};
use std::str::FromStr;

#[test]
fn test_version_parse_simple() {
    let v = Version::parse("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
    assert!(!v.is_pre_release());
}

#[test]
fn test_version_parse_with_v_prefix() {
    // Version with 'v' prefix should be accepted
    let v = Version::parse("v1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
    assert_eq!(v.to_string(), "1.2.3"); // Display should not include 'v'
}

#[test]
fn test_version_parse_pre_release() {
    let v = Version::parse("1.2.3-alpha.1").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
    assert_eq!(v.pre_release, Some("alpha.1".to_string()));
    assert!(v.is_pre_release());
}

#[test]
fn test_version_parse_pre_release_beta() {
    let v = Version::parse("2.0.0-beta.2").unwrap();
    assert_eq!(v.pre_release, Some("beta.2".to_string()));
    assert!(v.is_pre_release());
}

#[test]
fn test_version_parse_pre_release_rc() {
    let v = Version::parse("3.0.0-rc.1").unwrap();
    assert_eq!(v.pre_release, Some("rc.1".to_string()));
    assert!(v.is_pre_release());
}

#[test]
fn test_version_parse_build_metadata() {
    let v = Version::parse("1.2.3+build.123").unwrap();
    assert_eq!(v.build, Some("build.123".to_string()));
    assert!(!v.is_pre_release());
}

#[test]
fn test_version_parse_full() {
    // Full semver: MAJOR.MINOR.PATCH-PRERELEASE+BUILD
    let v = Version::parse("1.2.3-alpha.1+build.123").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
    assert_eq!(v.pre_release, Some("alpha.1".to_string()));
    assert_eq!(v.build, Some("build.123".to_string()));
    assert!(v.is_pre_release());
    assert_eq!(v.to_string(), "1.2.3-alpha.1+build.123");
}

#[test]
fn test_version_invalid_formats() {
    // Missing parts
    assert!(matches!(
        Version::parse("1.2"),
        Err(BumpError::InvalidFormat(_))
    ));
    assert!(matches!(
        Version::parse("1"),
        Err(BumpError::InvalidFormat(_))
    ));
    assert!(matches!(
        Version::parse("1.2.3.4"),
        Err(BumpError::InvalidFormat(_))
    ));

    // Non-numeric parts
    assert!(matches!(
        Version::parse("a.b.c"),
        Err(BumpError::InvalidFormat(_))
    ));
    assert!(matches!(
        Version::parse("1.a.3"),
        Err(BumpError::InvalidFormat(_))
    ));

    // Empty string
    assert!(matches!(
        Version::parse(""),
        Err(BumpError::InvalidFormat(_))
    ));
}

#[test]
fn test_version_display() {
    let v = Version::parse("1.2.3").unwrap();
    assert_eq!(v.to_string(), "1.2.3");

    let v = Version::parse("2.0.0-alpha.1").unwrap();
    assert_eq!(v.to_string(), "2.0.0-alpha.1");
}

#[test]
fn test_version_default() {
    let v = Version::default();
    assert_eq!(v.major, 0);
    assert_eq!(v.minor, 1);
    assert_eq!(v.patch, 0);
    assert!(!v.is_pre_release());
}

#[test]
fn test_version_from_str() {
    let v = Version::from_str("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
}

#[test]
fn test_version_bump_patch() {
    let v = Version::parse("1.2.3").unwrap();
    let bumped = v.bump(VersionBump::Patch).unwrap();

    assert_eq!(bumped.major, 1);
    assert_eq!(bumped.minor, 2);
    assert_eq!(bumped.patch, 4);
    assert!(!bumped.is_pre_release());
    assert_eq!(bumped.to_string(), "1.2.4");
}

#[test]
fn test_version_bump_minor() {
    let v = Version::parse("1.2.3").unwrap();
    let bumped = v.bump(VersionBump::Minor).unwrap();

    assert_eq!(bumped.major, 1);
    assert_eq!(bumped.minor, 3);
    assert_eq!(bumped.patch, 0); // Patch resets to 0
    assert!(!bumped.is_pre_release());
    assert_eq!(bumped.to_string(), "1.3.0");
}

#[test]
fn test_version_bump_major() {
    let v = Version::parse("1.2.3").unwrap();
    let bumped = v.bump(VersionBump::Major).unwrap();

    assert_eq!(bumped.major, 2);
    assert_eq!(bumped.minor, 0); // Minor resets to 0
    assert_eq!(bumped.patch, 0); // Patch resets to 0
    assert!(!bumped.is_pre_release());
    assert_eq!(bumped.to_string(), "2.0.0");
}

#[test]
fn test_version_bump_clears_pre_release() {
    let v = Version::parse("1.2.3-alpha.1").unwrap();
    let bumped = v.bump(VersionBump::Patch).unwrap();

    // Bumping should clear pre-release
    assert!(!bumped.is_pre_release());
    assert_eq!(bumped.to_string(), "1.2.4");
}

#[test]
fn test_version_bump_pre_release_new() {
    let v = Version::parse("1.2.3").unwrap();
    let bumped = v
        .bump(VersionBump::PreRelease("alpha".to_string()))
        .unwrap();

    assert!(bumped.is_pre_release());
    assert_eq!(bumped.pre_release, Some("alpha.1".to_string()));
    assert_eq!(bumped.to_string(), "1.2.3-alpha.1");
}

#[test]
fn test_version_bump_pre_release_increment() {
    let v = Version::parse("1.2.3-alpha.1").unwrap();
    let bumped = v
        .bump(VersionBump::PreRelease("alpha".to_string()))
        .unwrap();

    assert_eq!(bumped.pre_release, Some("alpha.2".to_string()));
    assert_eq!(bumped.to_string(), "1.2.3-alpha.2");
}

#[test]
fn test_version_comparison() {
    let v1 = Version::parse("1.2.3").unwrap();
    let v2 = Version::parse("1.2.4").unwrap();
    let v3 = Version::parse("1.3.0").unwrap();
    let v4 = Version::parse("2.0.0").unwrap();

    // Verify ordering
    assert!(v1 < v2);
    assert!(v2 < v3);
    assert!(v3 < v4);
    assert!(v1 < v4);
}

#[test]
fn test_version_equality() {
    let v1 = Version::parse("1.2.3").unwrap();
    let v2 = Version::parse("1.2.3").unwrap();
    let v3 = Version::parse("1.2.4").unwrap();

    assert_eq!(v1, v2);
    assert_ne!(v1, v3);
}

#[test]
fn test_version_base_version() {
    let v = Version::parse("1.2.3-alpha.1+build.123").unwrap();
    let base = v.base_version();

    assert_eq!(base.major, 1);
    assert_eq!(base.minor, 2);
    assert_eq!(base.patch, 3);
    assert!(base.pre_release.is_none());
    assert!(base.build.is_none());
    assert_eq!(base.to_string(), "1.2.3");
}

#[test]
fn test_version_bump_display() {
    assert_eq!(VersionBump::Major.to_string(), "major");
    assert_eq!(VersionBump::Minor.to_string(), "minor");
    assert_eq!(VersionBump::Patch.to_string(), "patch");
    assert_eq!(
        VersionBump::PreRelease("alpha".to_string()).to_string(),
        "pre-release (alpha)"
    );
}

#[test]
fn test_semver_compliance_zero_version() {
    // Per semver, version 0.x.x is for initial development
    // and anything may change at any time
    let v = Version::parse("0.1.0").unwrap();
    assert_eq!(v.major, 0);

    let v = Version::parse("0.0.1").unwrap();
    assert_eq!(v.major, 0);
    assert_eq!(v.minor, 0);
}

#[test]
fn test_semver_compliance_stable_version() {
    // Version 1.0.0 marks the public API
    let v = Version::parse("1.0.0").unwrap();
    assert!(v.major >= 1);
}

#[test]
fn test_version_bump_chain() {
    // Test a realistic version progression
    let v = Version::parse("0.1.0").unwrap();

    // Bug fix
    let v = v.bump(VersionBump::Patch).unwrap();
    assert_eq!(v.to_string(), "0.1.1");

    // Another bug fix
    let v = v.bump(VersionBump::Patch).unwrap();
    assert_eq!(v.to_string(), "0.1.2");

    // New feature (minor)
    let v = v.bump(VersionBump::Minor).unwrap();
    assert_eq!(v.to_string(), "0.2.0");

    // Bug fix on new minor
    let v = v.bump(VersionBump::Patch).unwrap();
    assert_eq!(v.to_string(), "0.2.1");

    // Breaking change (major)
    let v = v.bump(VersionBump::Major).unwrap();
    assert_eq!(v.to_string(), "1.0.0");
}

#[test]
fn test_version_pre_release_progression() {
    // Typical pre-release progression
    let v = Version::parse("1.0.0").unwrap();

    let v = v
        .bump(VersionBump::PreRelease("alpha".to_string()))
        .unwrap();
    assert_eq!(v.to_string(), "1.0.0-alpha.1");

    let v = v
        .bump(VersionBump::PreRelease("alpha".to_string()))
        .unwrap();
    assert_eq!(v.to_string(), "1.0.0-alpha.2");

    // Switch to beta
    let v = Version::parse("1.0.0").unwrap();
    let v = v.bump(VersionBump::PreRelease("beta".to_string())).unwrap();
    assert_eq!(v.to_string(), "1.0.0-beta.1");

    // Switch to rc
    let v = Version::parse("1.0.0").unwrap();
    let v = v.bump(VersionBump::PreRelease("rc".to_string())).unwrap();
    assert_eq!(v.to_string(), "1.0.0-rc.1");
}
