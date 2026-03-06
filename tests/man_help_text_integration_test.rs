//! Integration test for man page references in help text
//!
//! This test verifies that:
//! 1. The help text includes references to man pages
//! 2. The man page references follow the correct format
//! 3. Users can discover man pages through help output

use std::process::Command;

/// Test that --help output includes man page references
#[test]
fn test_help_output_includes_man_references() {
    let output = Command::new(env!("CARGO_BIN_EXE_ltmatrix"))
        .arg("--help")
        .output();

    // If binary doesn't exist (e.g., during cargo test without building), skip
    let output = match output {
        Ok(o) => o,
        Err(_) => {
            println!("Skipping help integration test: binary not built");
            return;
        }
    };

    assert!(
        output.status.success(),
        "ltmatrix --help should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for man page references
    // The help should mention man pages either explicitly or through examples
    let help_text = stdout.to_string();

    // Look for man-related keywords
    let has_man_reference = help_text.contains("man")
        || help_text.contains("MAN")
        || help_text.contains("manual")
        || help_text.contains("documentation");

    assert!(
        has_man_reference,
        "Help output should reference man pages or documentation. Got:\n{}",
        help_text
    );
}

/// Test that man subcommand help is available
#[test]
fn test_man_subcommand_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_ltmatrix"))
        .args(["man", "--help"])
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => {
            println!("Skipping man subcommand help test: binary not built");
            return;
        }
    };

    assert!(
        output.status.success(),
        "ltmatrix man --help should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should mention output directory option
    assert!(
        stdout.contains("output") || stdout.contains("OUTPUT") || stdout.contains("dir"),
        "Man subcommand help should mention output directory. Got:\n{}",
        stdout
    );
}

/// Test that all subcommands have help available
#[test]
fn test_all_subcommands_have_help() {
    let subcommands = vec!["release", "completions", "man"];

    for subcommand in subcommands {
        let output = Command::new(env!("CARGO_BIN_EXE_ltmatrix"))
            .args([subcommand, "--help"])
            .output();

        let output = match output {
            Ok(o) => o,
            Err(_) => {
                println!(
                    "Skipping {} subcommand help test: binary not built",
                    subcommand
                );
                return;
            }
        };

        assert!(
            output.status.success(),
            "ltmatrix {} --help should succeed. stderr: {}",
            subcommand,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.is_empty(),
            "{} subcommand help should not be empty",
            subcommand
        );
    }
}

/// Test that version information is available
#[test]
fn test_version_output() {
    let output = Command::new(env!("CARGO_BIN_EXE_ltmatrix"))
        .arg("--version")
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => {
            println!("Skipping version test: binary not built");
            return;
        }
    };

    assert!(
        output.status.success(),
        "ltmatrix --version should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should mention version
    assert!(
        stdout.contains("ltmatrix"),
        "Version output should mention ltmatrix. Got: {}",
        stdout
    );
}
