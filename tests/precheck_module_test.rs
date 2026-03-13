//! Tests for pre-release check module
//!
//! These tests verify the pre-release validation checks.

use ltmatrix::release::{CheckResult, CheckStatus};

#[test]
fn test_check_status_display() {
    assert_eq!(CheckStatus::Passed.to_string(), "✓");
    assert_eq!(CheckStatus::Failed.to_string(), "✗");
    assert_eq!(CheckStatus::Skipped.to_string(), "○");
    assert_eq!(CheckStatus::Warning.to_string(), "⚠");
}

#[test]
fn test_check_result_passed() {
    let result = CheckResult::passed("Test check");

    assert_eq!(result.name, "Test check");
    assert_eq!(result.status, CheckStatus::Passed);
    assert!(result.message.is_none());
}

#[test]
fn test_check_result_failed() {
    let result = CheckResult::failed("Test check", "Error message");

    assert_eq!(result.name, "Test check");
    assert_eq!(result.status, CheckStatus::Failed);
    assert_eq!(result.message, Some("Error message".to_string()));
}

#[test]
fn test_check_result_skipped() {
    let result = CheckResult::skipped("Test check", "Not applicable");

    assert_eq!(result.name, "Test check");
    assert_eq!(result.status, CheckStatus::Skipped);
    assert_eq!(result.message, Some("Not applicable".to_string()));
}

#[test]
fn test_check_result_warning() {
    let result = CheckResult::warning("Test check", "Potential issue");

    assert_eq!(result.name, "Test check");
    assert_eq!(result.status, CheckStatus::Warning);
    assert_eq!(result.message, Some("Potential issue".to_string()));
}

#[test]
fn test_check_result_equality() {
    let r1 = CheckResult::passed("Test");
    let r2 = CheckResult::passed("Test");

    assert_eq!(r1.name, r2.name);
    assert_eq!(r1.status, r2.status);
}

#[test]
fn test_check_result_different_status() {
    let passed = CheckResult::passed("Test");
    let failed = CheckResult::failed("Test", "Error");

    assert_ne!(passed.status, failed.status);
}
