//! Edge Cases Tests for Test Stage
//!
//! Tests edge cases and boundary conditions for framework detection
//! and test stage functionality.

use ltmatrix::pipeline::test::{
    detect_test_framework, file_exists_and_readable, directory_exists_and_accessible,
    read_file_lines, parse_toml_section, TestFramework,
};
use std::path::PathBuf;
use tempfile::TempDir;

// ==================== Empty/Minimal Project Edge Cases ====================

#[test]
fn test_detection_with_empty_directory() {
    let dir = TempDir::new().unwrap();
    let detection = detect_test_framework(dir.path()).unwrap();

    assert_eq!(detection.framework, TestFramework::None);
    assert!(detection.config_files.is_empty());
    assert!(detection.test_paths.is_empty());
    assert_eq!(detection.confidence, 0.0);
}

#[test]
fn test_detection_with_only_readme() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("README.md"), "# My Project").unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::None);
}

#[test]
fn test_detection_with_only_git_directory() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".git")).unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::None);
}

// ==================== Malformed Configuration Edge Cases ====================

#[test]
fn test_cargo_with_invalid_toml() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "this is not valid toml [[[",
    ).unwrap();

    // Should not panic, should handle gracefully
    let _detection = detect_test_framework(dir.path()).unwrap();
    // May or may not detect Cargo depending on error handling
    // Just verify it doesn't crash
}

#[test]
fn test_package_json_with_invalid_json() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        "{ invalid json }",
    ).unwrap();

    // May return error or detect no framework
    let result = detect_test_framework(dir.path());
    match result {
        Ok(detection) => {
            // Should not detect npm with invalid package.json
            assert_ne!(detection.framework, TestFramework::Npm);
        }
        Err(_) => {
            // Also acceptable to return an error for invalid JSON
        }
    }
}

#[test]
fn test_pyproject_with_invalid_toml() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        "not valid toml",
    ).unwrap();

    // Should handle gracefully
    let _detection = detect_test_framework(dir.path()).unwrap();
    // Just verify it doesn't crash
}

// ==================== Empty/Missing Test Files Edge Cases ====================

#[test]
fn test_cargo_with_empty_tests_directory() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    let tests_dir = dir.path().join("tests");
    std::fs::create_dir_all(&tests_dir).unwrap();
    // Don't add any test files

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Cargo);
    // Should still detect Cargo even without test files
}

#[test]
fn test_pytest_with_empty_tests_directory() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("pytest.ini"), "[pytest]").unwrap();

    let tests_dir = dir.path().join("tests");
    std::fs::create_dir_all(&tests_dir).unwrap();
    // Don't add any test files

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Pytest);
    // Lower confidence without test files
    assert!(detection.confidence < 1.0);
}

#[test]
fn test_npm_with_package_json_no_test_scripts() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{
  "name": "test-project",
  "version": "1.0.0"
}
"#,
    ).unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    // May still detect npm but with low confidence
    // or may not detect at all
    if detection.framework == TestFramework::Npm {
        assert!(detection.confidence < 0.5);
    } else {
        assert_eq!(detection.framework, TestFramework::None);
    }
}

// ==================== Mixed Projects Edge Cases ====================

#[test]
fn test_monorepo_with_multiple_frameworks() {
    let dir = TempDir::new().unwrap();

    // Add multiple framework indicators
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "monorepo"
version = "0.1.0"
"#,
    ).unwrap();

    std::fs::write(
        dir.path().join("package.json"),
        r#"{"name": "monorepo"}"#,
    ).unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    // Cargo should take priority
    assert_eq!(detection.framework, TestFramework::Cargo);
}

#[test]
fn test_project_with_test_file_in_root() {
    let dir = TempDir::new().unwrap();

    // Create a test file in root (not in tests/ directory)
    std::fs::write(
        dir.path().join("test_utils.py"),
        "def test_helper(): pass",
    ).unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    // May detect pytest due to test_*.py pattern in root
    // This is acceptable behavior
    if detection.framework == TestFramework::Pytest {
        // If detected, confidence should be low without proper config
        assert!(detection.confidence < 1.0);
    } else {
        assert_eq!(detection.framework, TestFramework::None);
    }
}

#[test]
fn test_nested_test_directories() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    // Create deeply nested test directory
    let nested_tests = dir.path().join("a/b/c/tests");
    std::fs::create_dir_all(&nested_tests).unwrap();
    std::fs::write(nested_tests.join("test.rs"), "#[test] fn test_it() {}").unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Cargo);
    // May or may not detect nested tests depending on implementation
}

// ==================== File System Edge Cases ====================

#[test]
fn test_file_exists_with_symlink() {
    let dir = TempDir::new().unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let file_path = dir.path().join("original.txt");
        std::fs::write(&file_path, "content").unwrap();

        let link_path = dir.path().join("link.txt");
        symlink(&file_path, &link_path).unwrap();

        // Should work with symlinks
        assert!(file_exists_and_readable(&link_path));
    }

    #[cfg(windows)]
    {
        // On Windows, just verify the file exists
        let file_path = dir.path().join("original.txt");
        std::fs::write(&file_path, "content").unwrap();
        assert!(file_exists_and_readable(&file_path));
    }
}

#[test]
fn test_directory_exists_with_symlink() {
    let dir = TempDir::new().unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let original_dir = dir.path().join("original");
        std::fs::create_dir_all(&original_dir).unwrap();

        let link_dir = dir.path().join("link");
        symlink(&original_dir, &link_dir).unwrap();

        // Should work with directory symlinks
        assert!(directory_exists_and_accessible(&link_dir));
    }

    #[cfg(windows)]
    {
        // On Windows, just verify the directory exists
        let original_dir = dir.path().join("original");
        std::fs::create_dir_all(&original_dir).unwrap();
        assert!(directory_exists_and_accessible(&original_dir));
    }
}

#[test]
fn test_read_file_lines_with_empty_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("empty.txt");
    std::fs::write(&file_path, "").unwrap();

    let lines = read_file_lines(&file_path, 10).unwrap();
    assert!(lines.is_empty());
}

#[test]
fn test_read_file_lines_with_single_line() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("single.txt");
    std::fs::write(&file_path, "only one line").unwrap();

    let lines = read_file_lines(&file_path, 10).unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], "only one line");
}

#[test]
fn test_read_file_lines_with_no_newline_at_end() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("no_newline.txt");
    std::fs::write(&file_path, "line1\nline2\nline3").unwrap();

    let lines = read_file_lines(&file_path, 10).unwrap();
    assert_eq!(lines.len(), 3);
}

#[test]
fn test_read_file_lines_with_blank_lines() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("blank.txt");
    std::fs::write(&file_path, "line1\n\nline3\n\n").unwrap();

    let lines = read_file_lines(&file_path, 10).unwrap();
    // Blank lines should be preserved
    assert_eq!(lines.len(), 4);
}

// ==================== TOML Parsing Edge Cases ====================

#[test]
fn test_parse_toml_section_empty_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("empty.toml");
    std::fs::write(&file_path, "").unwrap();

    let result = parse_toml_section(&file_path, "section");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_parse_toml_section_no_matching_section() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("config.toml");
    std::fs::write(
        &file_path,
        r#"[other_section]
key = "value"
"#,
    ).unwrap();

    let result = parse_toml_section(&file_path, "missing_section");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_parse_toml_section_nested_key() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("config.toml");
    std::fs::write(
        &file_path,
        r#"[parent]
[parent.child]
key = "value"
"#,
    ).unwrap();

    let result = parse_toml_section(&file_path, "parent.child");
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

#[test]
fn test_parse_toml_section_special_characters() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("config.toml");
    std::fs::write(
        &file_path,
        r#"[section_name-with.special]
key = "value"
"#,
    ).unwrap();

    let result = parse_toml_section(&file_path, "section_name-with.special");
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

// ==================== Permission Edge Cases ====================

#[test]
fn test_detection_with_unreadable_files() {
    let dir = TempDir::new().unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let file_path = dir.path().join("unreadable.txt");
        std::fs::write(&file_path, "content").unwrap();

        // Remove read permissions
        let mut perms = std::fs::metadata(&file_path).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&file_path, perms).unwrap();

        // Should handle gracefully without crashing
        assert!(!file_exists_and_readable(&file_path));
    }

    #[cfg(windows)]
    {
        // On Windows, skip this test or use alternative approach
        // Just verify the function exists
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content").unwrap();
        assert!(file_exists_and_readable(&file_path));
    }
}

// ==================== Confidence Boundary Cases ====================

#[test]
fn test_confidence_zero() {
    let dir = TempDir::new().unwrap();
    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.confidence, 0.0);
}

#[test]
fn test_confidence_maximum() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    let src_dir = dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        r#"#[cfg(test)]
mod tests { #[test] fn test_it() {} }
"#,
    ).unwrap();

    let tests_dir = dir.path().join("tests");
    std::fs::create_dir_all(&tests_dir).unwrap();
    std::fs::write(
        tests_dir.join("test.rs"),
        "#[test] fn test_it() {}",
    ).unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.confidence, 1.0);
}

#[test]
fn test_confidence_intermediate() {
    let dir = TempDir::new().unwrap();

    // Only go.mod, no test files
    std::fs::write(
        dir.path().join("go.mod"),
        "module test\ngo 1.21\n",
    ).unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    if detection.framework == TestFramework::Go {
        // Should have intermediate confidence
        assert!(detection.confidence > 0.0);
        assert!(detection.confidence < 1.0);
    }
}

// ==================== Path Handling Edge Cases ====================

#[test]
fn test_detection_with_absolute_path() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    // Use absolute path
    let abs_path = std::fs::canonicalize(dir.path()).unwrap();
    let detection = detect_test_framework(&abs_path).unwrap();

    assert_eq!(detection.framework, TestFramework::Cargo);
}

#[test]
fn test_detection_with_relative_path() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    // Use relative path
    let rel_path = PathBuf::from(".").join(dir.path().file_name().unwrap());
    let detection = detect_test_framework(&rel_path);

    // May fail with relative path depending on working directory
    // Just verify it doesn't crash
    let _ = detection;
}

#[test]
fn test_detection_with_trailing_slash() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    let path_with_slash = dir.path().join("");
    let detection = detect_test_framework(&path_with_slash).unwrap();

    assert_eq!(detection.framework, TestFramework::Cargo);
}
