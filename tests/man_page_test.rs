//! Tests for man page generation
//!
//! These tests verify that man pages can be generated for ltmatrix and all subcommands.

use std::fs;
use std::path::PathBuf;

/// Test that man pages can be generated for the main command
#[test]
fn test_main_man_page_generation() {
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("man_pages");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // This should generate the main man page
    let result = std::panic::catch_unwind(|| {
        ltmatrix::man::generate_man_pages(&output_dir).expect("Man page generation should succeed");
    });

    assert!(result.is_ok(), "Man page generation should succeed");

    // Verify main man page exists
    let main_man_page = output_dir.join("ltmatrix.1");
    assert!(main_man_page.exists(), "Main man page should be generated");

    // Verify man page contains expected content
    let content = fs::read_to_string(&main_man_page).expect("Failed to read man page");
    assert!(
        content.contains(".TH ltmatrix"),
        "Man page should have TH macro"
    );
    assert!(
        content.contains("ltmatrix"),
        "Man page should mention ltmatrix"
    );
    assert!(
        content.contains("Automate software development tasks"),
        "Man page should have description"
    );
}

/// Test that man pages are generated for all subcommands
#[test]
fn test_subcommand_man_pages() {
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("man_pages_subcommands");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    ltmatrix::man::generate_man_pages(&output_dir).expect("Man page generation should succeed");

    // Check for release subcommand man page
    let release_man_page = output_dir.join("ltmatrix-release.1");
    assert!(
        release_man_page.exists(),
        "Release subcommand man page should exist"
    );

    // Check for completions subcommand man page
    let completions_man_page = output_dir.join("ltmatrix-completions.1");
    assert!(
        completions_man_page.exists(),
        "Completions subcommand man page should exist"
    );

    // Verify release man page content
    let release_content =
        fs::read_to_string(&release_man_page).expect("Failed to read release man page");
    assert!(
        release_content.contains("release"),
        "Release man page should mention release"
    );
    assert!(
        release_content.contains(".TH"),
        "Release man page should have TH macro"
    );
}

/// Test that generated man pages are valid roff format
#[test]
fn test_man_page_valid_roff() {
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("man_pages_valid");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    ltmatrix::man::generate_man_pages(&output_dir).expect("Man page generation should succeed");

    let main_man_page = output_dir.join("ltmatrix.1");
    let content = fs::read_to_string(&main_man_page).expect("Failed to read man page");

    // Basic roff validation - check for required macros
    assert!(content.contains(".TH"), "Must have TH macro (title header)");
    assert!(
        content.contains(".SH"),
        "Must have SH macro (section header)"
    );
    assert!(
        content.contains(".TP"),
        "Must have TP macro (tagged paragraph)"
    );

    // Check that .TH appears early in the file (within first 5 lines)
    let lines: Vec<&str> = content.lines().collect();
    let th_found = lines.iter().take(5).any(|line| line.contains(".TH"));
    assert!(th_found, "Man page must have .TH macro in first 5 lines");
}

/// Test that man pages include proper sections
#[test]
fn test_man_page_sections() {
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("man_pages_sections");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    ltmatrix::man::generate_man_pages(&output_dir).expect("Man page generation should succeed");

    let main_man_page = output_dir.join("ltmatrix.1");
    let content = fs::read_to_string(&main_man_page).expect("Failed to read man page");

    // Check for standard man page sections
    assert!(content.contains("NAME"), "Must have NAME section");
    assert!(content.contains("SYNOPSIS"), "Must have SYNOPSIS section");
    assert!(
        content.contains("DESCRIPTION"),
        "Must have DESCRIPTION section"
    );
    assert!(content.contains("OPTIONS"), "Must have OPTIONS section");
}
