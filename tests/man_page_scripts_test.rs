//! Tests for man page generation scripts
//!
//! These tests verify that the shell/batch scripts for generating man pages work correctly.

use std::fs;
use std::path::PathBuf;

/// Test that the man generation example works
#[test]
fn test_generate_man_pages_example() {
    // This test verifies that the example code for generating man pages works
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("example_test");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Simulate what the example does
    let result = ltmatrix::man::generate_man_pages(&output_dir);

    assert!(result.is_ok(), "Example code should work");

    // Verify expected files
    assert!(output_dir.join("ltmatrix.1").exists());
    assert!(output_dir.join("ltmatrix-release.1").exists());
    assert!(output_dir.join("ltmatrix-completions.1").exists());
    assert!(output_dir.join("ltmatrix-man.1").exists());
}

/// Test that man pages can be generated to a specified output directory
#[test]
fn test_man_page_output_directory() {
    let custom_output = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("custom_output")
        .join("man");

    let result = ltmatrix::man::generate_man_pages(&custom_output);

    assert!(result.is_ok(), "Should generate to custom directory");

    // Verify files are in the custom location
    assert!(
        custom_output.join("ltmatrix.1").exists(),
        "Main man page should be in custom directory"
    );
}

/// Test that existing man pages are overwritten
#[test]
fn test_man_page_overwrite() {
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("overwrite_test");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Generate man pages first time
    ltmatrix::man::generate_man_pages(&output_dir).expect("First generation should succeed");

    let main_man = output_dir.join("ltmatrix.1");
    let first_content = fs::read_to_string(&main_man).expect("Failed to read first man page");

    // Modify the file
    fs::write(&main_man, "modified content").expect("Failed to modify man page");

    // Generate again
    ltmatrix::man::generate_man_pages(&output_dir).expect("Second generation should succeed");

    let second_content = fs::read_to_string(&main_man).expect("Failed to read second man page");

    // Content should be back to original (not "modified content")
    assert_eq!(
        first_content, second_content,
        "Man page should be overwritten with correct content"
    );

    assert_ne!(
        second_content, "modified content",
        "Man page should not contain modified content"
    );
}

/// Test that man page generation is idempotent
#[test]
fn test_man_page_generation_idempotent() {
    let output_dir1 = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("idempotent1");
    let output_dir2 = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("idempotent2");

    fs::create_dir_all(&output_dir1).expect("Failed to create output directory 1");
    fs::create_dir_all(&output_dir2).expect("Failed to create output directory 2");

    // Generate twice
    ltmatrix::man::generate_man_pages(&output_dir1).expect("First generation should succeed");
    ltmatrix::man::generate_man_pages(&output_dir2).expect("Second generation should succeed");

    // Compare main man pages
    let man1 =
        fs::read_to_string(output_dir1.join("ltmatrix.1")).expect("Failed to read first man page");
    let man2 =
        fs::read_to_string(output_dir2.join("ltmatrix.1")).expect("Failed to read second man page");

    assert_eq!(man1, man2, "Man page generation should be idempotent");
}

/// Test that all required sections are present in man pages
#[test]
fn test_man_page_required_sections() {
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sections_test");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    ltmatrix::man::generate_man_pages(&output_dir).expect("Man page generation should succeed");

    let main_man = output_dir.join("ltmatrix.1");
    let content = fs::read_to_string(&main_man).expect("Failed to read man page");

    // Required sections for a valid man page
    let required_sections = vec!["NAME", "SYNOPSIS", "DESCRIPTION"];

    for section in required_sections {
        assert!(
            content.contains(section),
            "Man page must have {} section",
            section
        );
    }
}

/// Test that man pages include proper command documentation
#[test]
fn test_man_page_command_documentation() {
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("command_docs");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    ltmatrix::man::generate_man_pages(&output_dir).expect("Man page generation should succeed");

    // Check that main man page documents the command
    let main_man = output_dir.join("ltmatrix.1");
    let content = fs::read_to_string(&main_man).expect("Failed to read main man page");

    // Should have description
    assert!(
        content.contains("DESCRIPTION"),
        "Main man page should have DESCRIPTION section"
    );

    // Should mention key concepts (case-insensitive)
    let content_lower = content.to_lowercase();
    let key_terms = vec!["agent", "orchestrator"];
    for term in key_terms {
        assert!(
            content_lower.contains(term),
            "Main man page should mention {}",
            term
        );
    }
}
