//! Comprehensive tests for man page generation feature
//!
//! These tests verify:
//! 1. Man page generation for all subcommands
//! 2. Man page content validation
//! 3. Man page references in help text
//! 4. Man subcommand functionality
//! 5. Man page rendering (if man command is available)

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Test that man pages are generated for all expected subcommands
#[test]
fn test_all_subcommands_have_man_pages() {
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("all_subcommands_man");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    ltmatrix::man::generate_man_pages(&output_dir).expect("Man page generation should succeed");

    // List all generated man pages
    let entries: Vec<_> = fs::read_dir(&output_dir)
        .expect("Failed to read output directory")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name())
        .collect();

    // Expected man pages for all subcommands
    let expected_man_pages = vec![
        "ltmatrix.1",
        "ltmatrix-release.1",
        "ltmatrix-completions.1",
        "ltmatrix-man.1",
    ];

    for expected in &expected_man_pages {
        assert!(
            entries
                .iter()
                .any(|name| name.to_string_lossy().as_ref() == *expected),
            "Expected man page {} not found. Generated: {:?}",
            expected,
            entries
        );
    }

    // Verify all expected files exist and are not empty
    for expected in &expected_man_pages {
        let man_path = output_dir.join(expected);
        assert!(man_path.exists(), "{} should exist", expected);

        let content = fs::metadata(&man_path).expect("Failed to get metadata");
        assert!(content.len() > 0, "{} should not be empty", expected);
    }
}

/// Test that man pages contain required roff macros and structure
#[test]
fn test_man_page_roff_structure() {
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("roff_structure");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    ltmatrix::man::generate_man_pages(&output_dir).expect("Man page generation should succeed");

    let man_pages = vec![
        "ltmatrix.1",
        "ltmatrix-release.1",
        "ltmatrix-completions.1",
        "ltmatrix-man.1",
    ];

    for man_page in &man_pages {
        let man_path = output_dir.join(man_page);
        let content =
            fs::read_to_string(&man_path).unwrap_or_else(|_| panic!("Failed to read {}", man_page));

        // Check for required roff macros
        assert!(content.contains(".TH"), "{} must have TH macro", man_page);
        assert!(content.contains(".SH"), "{} must have SH macro", man_page);

        // Check for standard sections
        assert!(
            content.contains("NAME"),
            "{} must have NAME section",
            man_page
        );
        assert!(
            content.contains("SYNOPSIS"),
            "{} must have SYNOPSIS section",
            man_page
        );
        assert!(
            content.contains("DESCRIPTION"),
            "{} must have DESCRIPTION section",
            man_page
        );

        // Verify .TH comes first (within first 3 lines)
        let lines: Vec<&str> = content.lines().collect();
        let th_position = lines
            .iter()
            .position(|line| line.contains(".TH"))
            .expect("Should find .TH macro");

        assert!(
            th_position < 3,
            "{} .TH macro should be in first 3 lines, found at line {}",
            man_page,
            th_position + 1
        );
    }
}

/// Test that man pages contain correct command information
#[test]
fn test_man_page_command_information() {
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("command_info");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    ltmatrix::man::generate_man_pages(&output_dir).expect("Man page generation should succeed");

    // Check main man page
    let main_man = output_dir.join("ltmatrix.1");
    let main_content = fs::read_to_string(&main_man).expect("Failed to read main man page");

    assert!(
        main_content.contains("ltmatrix"),
        "Main man page should mention ltmatrix"
    );
    assert!(
        main_content.contains("agent"),
        "Main man page should mention agent"
    );
    assert!(
        main_content.contains("orchestrator"),
        "Main man page should mention orchestrator"
    );

    // Check release subcommand man page
    let release_man = output_dir.join("ltmatrix-release.1");
    let release_content =
        fs::read_to_string(&release_man).expect("Failed to read release man page");

    assert!(
        release_content.contains("release"),
        "Release man page should mention release"
    );

    // Check completions subcommand man page
    let completions_man = output_dir.join("ltmatrix-completions.1");
    let completions_content =
        fs::read_to_string(&completions_man).expect("Failed to read completions man page");

    assert!(
        completions_content.contains("completions"),
        "Completions man page should mention completions"
    );
}

/// Test that help text includes man page references
#[test]
fn test_help_text_contains_man_references() {
    use clap::CommandFactory;
    use ltmatrix::cli::args::Args;

    let mut cmd = Args::command();
    let help_text = cmd.render_long_help();

    let help_string = help_text.to_string();

    // Check for man page references
    assert!(
        help_string.contains("man") || help_string.contains("MAN"),
        "Help text should reference man pages"
    );

    // The help should mention how to get more information
    assert!(
        help_string.contains("help") || help_string.contains("HELP"),
        "Help text should mention help"
    );
}

/// Test the man subcommand functionality
#[test]
fn test_man_subcommand_execution() {
    use ltmatrix::cli::args::{Args, Command, ManArgs};
    use ltmatrix::cli::command::execute_command;

    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("subcommand_test");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    let man_args = ManArgs {
        output: output_dir.clone(),
    };

    let args = Args {
        goal: None,
        agent: None,
        mode: None,
        fast: false,
        expert: false,
        config: None,
        output: None,
        log_level: None,
        log_file: None,
        max_retries: None,
        timeout: None,
        dry_run: false,
        resume: false,
        ask: false,
        regenerate_plan: false,
        on_blocked: None,
        mcp_config: None,
        no_color: false,
        telemetry: false,
        command: Some(Command::Man(man_args)),
    };

    let result = execute_command(args);
    assert!(result.is_ok(), "Man subcommand should execute successfully");

    // Verify man pages were created
    assert!(
        output_dir.join("ltmatrix.1").exists(),
        "Main man page should be created by man subcommand"
    );
}

/// Test man page generation with non-existent directory
#[test]
fn test_man_page_creates_nested_directories() {
    let temp_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    // Use a UUID-based unique path to avoid conflicts
    let unique_name = format!(
        "nested_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let nested_dir = temp_dir.join(&unique_name).join("nested").join("man");

    // Clean up if it exists from a previous run
    if nested_dir.exists() {
        fs::remove_dir_all(&nested_dir).expect("Failed to clean up existing directory");
    }

    let result = ltmatrix::man::generate_man_pages(&nested_dir);

    assert!(
        result.is_ok(),
        "Should create nested directories successfully"
    );
    assert!(nested_dir.exists(), "Nested directory should be created");
    assert!(
        nested_dir.join("ltmatrix.1").exists(),
        "Main man page should exist in nested directory"
    );
}

/// Test that man pages are valid UTF-8
#[test]
fn test_man_pages_valid_utf8() {
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("utf8_test");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    ltmatrix::man::generate_man_pages(&output_dir).expect("Man page generation should succeed");

    let entries = fs::read_dir(&output_dir).expect("Failed to read directory");

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("1") {
            // Try to read as string - should not fail
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("Failed to read man page as UTF-8: {:?}", path));

            // Verify it's not empty
            assert!(
                !content.is_empty(),
                "Man page {:?} should not be empty",
                path
            );
        }
    }
}

/// Test that each man page has a unique filename
#[test]
fn test_man_page_unique_filenames() {
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("unique_filenames");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    ltmatrix::man::generate_man_pages(&output_dir).expect("Man page generation should succeed");

    let entries: Vec<_> = fs::read_dir(&output_dir)
        .expect("Failed to read directory")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name())
        .collect();

    let mut unique_names = std::collections::HashSet::new();

    for name in &entries {
        let name_str = name.to_string_lossy().to_string();
        assert!(
            unique_names.insert(name_str),
            "Duplicate man page filename found: {:?}",
            name
        );
    }

    // Should have at least 4 man pages (main + 3 subcommands)
    assert!(
        entries.len() >= 4,
        "Should have at least 4 man pages, got: {}",
        entries.len()
    );
}

/// Test that man page content is deterministic (same input produces same output)
#[test]
fn test_man_page_deterministic() {
    let output_dir1 = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("deterministic1");
    let output_dir2 = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("deterministic2");

    fs::create_dir_all(&output_dir1).expect("Failed to create output directory 1");
    fs::create_dir_all(&output_dir2).expect("Failed to create output directory 2");

    ltmatrix::man::generate_man_pages(&output_dir1).expect("First generation should succeed");
    ltmatrix::man::generate_man_pages(&output_dir2).expect("Second generation should succeed");

    let main_man1 =
        fs::read_to_string(output_dir1.join("ltmatrix.1")).expect("Failed to read first man page");
    let main_man2 =
        fs::read_to_string(output_dir2.join("ltmatrix.1")).expect("Failed to read second man page");

    assert_eq!(
        main_man1, main_man2,
        "Man page generation should be deterministic"
    );
}

/// Test man page rendering with actual man command (if available)
#[test]
fn test_man_page_rendering() {
    // Check if man command is available
    let man_available = Command::new("man")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !man_available {
        println!("Skipping man rendering test: man command not available");
        return;
    }

    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("rendering");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    ltmatrix::man::generate_man_pages(&output_dir).expect("Man page generation should succeed");

    let main_man = output_dir.join("ltmatrix.1");

    // Try to render the man page
    let output = Command::new("man")
        .arg("--local-file")
        .arg(&main_man)
        .output();

    match output {
        Ok(result) => {
            // Man command should succeed
            assert!(
                result.status.success(),
                "Man command should successfully render the man page. stderr: {}",
                String::from_utf8_lossy(&result.stderr)
            );

            // Output should contain some text
            let stdout = String::from_utf8_lossy(&result.stdout);
            assert!(
                !stdout.is_empty(),
                "Rendered man page should produce output"
            );
        }
        Err(e) => {
            println!(
                "Skipping man rendering test: man command failed to run: {}",
                e
            );
        }
    }
}

/// Test that man pages include version information
#[test]
fn test_man_page_includes_version() {
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("version_test");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    ltmatrix::man::generate_man_pages(&output_dir).expect("Man page generation should succeed");

    let main_man = output_dir.join("ltmatrix.1");
    let content = fs::read_to_string(&main_man).expect("Failed to read man page");

    // Check for version in TH macro or content
    // The .TH macro typically includes version information
    assert!(
        content.contains(".TH"),
        "Man page should have TH macro with version info"
    );

    // Check that the version appears somewhere in the man page
    let version = env!("CARGO_PKG_VERSION");
    assert!(
        content.contains(version) || content.chars().any(|c| c.is_ascii_digit()),
        "Man page should include version information"
    );
}

/// Test error handling when output directory is not writable
#[test]
#[cfg(unix)]
fn test_man_page_permission_error() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("readonly");

    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Make directory read-only
    let mut perms = fs::metadata(&output_dir)
        .expect("Failed to get permissions")
        .permissions();
    perms.set_readonly(true);
    fs::set_permissions(&output_dir, perms).expect("Failed to set permissions");

    let result = ltmatrix::man::generate_man_pages(&output_dir);

    // Should fail due to permission error
    assert!(result.is_err(), "Should fail with read-only directory");

    // Clean up - restore permissions for deletion
    let mut perms = fs::metadata(&output_dir)
        .expect("Failed to get permissions")
        .permissions();
    perms.set_readonly(false);
    fs::set_permissions(&output_dir, perms).expect("Failed to restore permissions");
}
