//! Error Scenarios Tests for Test Stage
//!
//! Tests error handling and failure scenarios for framework detection
//! and test stage functionality.

use ltmatrix::pipeline::test::{
    detect_test_framework, read_file_lines, parse_toml_section, TestFramework,
};
use std::path::Path;
use tempfile::TempDir;

// ==================== File System Error Scenarios ====================

#[test]
fn test_detection_with_nonexistent_directory() {
    let nonexistent = Path::new("/this/path/does/not/exist/12345");
    let result = detect_test_framework(nonexistent);

    // Should handle gracefully - either Ok with None framework or Err
    match result {
        Ok(detection) => {
            assert_eq!(detection.framework, TestFramework::None);
        }
        Err(_) => {
            // Also acceptable to return an error
        }
    }
}

#[test]
fn test_read_file_lines_nonexistent_file() {
    let result = read_file_lines(Path::new("nonexistent_file_12345.txt"), 10);
    assert!(result.is_err());
}

#[test]
fn test_read_file_lines_permission_denied() {
    // This test is platform-dependent
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("unreadable.txt");
        std::fs::write(&file_path, "content").unwrap();

        // Remove read permissions
        let mut perms = std::fs::metadata(&file_path).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&file_path, perms).unwrap();

        // Should handle permission error
        let result = read_file_lines(&file_path, 10);
        assert!(result.is_err());
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems, just verify the function exists
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content").unwrap();

        let result = read_file_lines(&file_path, 10);
        assert!(result.is_ok());
    }
}

#[test]
fn test_parse_toml_nonexistent_file() {
    let result = parse_toml_section(Path::new("nonexistent_12345.toml"), "section");
    assert!(result.is_err());
}

#[test]
fn test_parse_toml_invalid_toml() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("invalid.toml");
    std::fs::write(&file_path, "this is not valid toml [[[ [[[").unwrap();

    let result = parse_toml_section(&file_path, "section");
    assert!(result.is_err());
}

#[test]
fn test_parse_toml_malformed_section() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("malformed.toml");
    std::fs::write(
        &file_path,
        r#"[section
key = "value"
"#,
    ).unwrap();

    let result = parse_toml_section(&file_path, "section");
    assert!(result.is_err());
}

// ==================== Configuration Error Scenarios ====================

#[test]
fn test_cargo_with_malformed_toml() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "not valid toml {{{",
    ).unwrap();

    // Should not crash
    let _detection = detect_test_framework(dir.path()).unwrap();
}

#[test]
fn test_package_json_with_malformed_json() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        "{ invalid json {{{ ",
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
fn test_pyproject_with_malformed_toml() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        "not valid toml",
    ).unwrap();

    // Should not crash
    let _detection = detect_test_framework(dir.path()).unwrap();
}

#[test]
fn test_go_mod_with_invalid_syntax() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("go.mod"),
        "this is not valid go.mod syntax {{{",
    ).unwrap();

    // Should not crash - may or may not detect Go
    let _detection = detect_test_framework(dir.path()).unwrap();
}

#[test]
fn test_pytest_ini_with_invalid_ini() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("pytest.ini"),
        "not valid ini {{{",
    ).unwrap();

    // Should not crash
    let _detection = detect_test_framework(dir.path()).unwrap();
}

// ==================== Special Path Error Scenarios ====================

#[test]
fn test_detection_with_very_long_path() {
    let dir = TempDir::new().unwrap();

    // Create a deeply nested directory
    let mut path = dir.path().to_path_buf();
    for i in 0..20 {
        path = path.join(format!("level_{}", i));
    }
    std::fs::create_dir_all(&path).unwrap();

    // Should handle without error
    let _detection = detect_test_framework(&path).unwrap();
}

#[test]
fn test_detection_with_special_characters_in_path() {
    let dir = TempDir::new().unwrap();

    let special_dir = dir.path().join("test with spaces & special-chars_123");
    std::fs::create_dir_all(&special_dir).unwrap();
    std::fs::write(
        special_dir.join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    // Should handle special characters
    let detection = detect_test_framework(&special_dir).unwrap();
    assert_eq!(detection.framework, TestFramework::Cargo);
}

#[test]
fn test_detection_with_unicode_in_path() {
    let dir = TempDir::new().unwrap();

    let unicode_dir = dir.path().join("test_测试_🧪");
    std::fs::create_dir_all(&unicode_dir).unwrap();
    std::fs::write(
        unicode_dir.join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    // Should handle unicode
    let detection = detect_test_framework(&unicode_dir).unwrap();
    assert_eq!(detection.framework, TestFramework::Cargo);
}

// ==================== Large File Error Scenarios ====================

#[test]
fn test_read_file_lines_from_large_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("large.txt");

    // Create a large file
    let mut content = String::new();
    for i in 0..1000 {
        content.push_str(&format!("Line {}\n", i));
    }
    std::fs::write(&file_path, content).unwrap();

    // Should handle large files
    let result = read_file_lines(&file_path, 10);
    assert!(result.is_ok());

    let lines = result.unwrap();
    assert_eq!(lines.len(), 10);
}

#[test]
fn test_read_file_lines_limit_exceeds_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("small.txt");
    std::fs::write(&file_path, "line1\nline2\nline3").unwrap();

    // Request more lines than exist
    let lines = read_file_lines(&file_path, 1000).unwrap();
    assert_eq!(lines.len(), 3);
}

// ==================== Concurrent Access Scenarios ====================

#[test]
fn test_multiple_detections_same_directory() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    // Multiple detections should be safe
    let detection1 = detect_test_framework(dir.path()).unwrap();
    let detection2 = detect_test_framework(dir.path()).unwrap();
    let detection3 = detect_test_framework(dir.path()).unwrap();

    // All should return the same result
    assert_eq!(detection1.framework, detection2.framework);
    assert_eq!(detection2.framework, detection3.framework);
}

#[test]
fn test_detection_while_directory_changes() {
    let dir = TempDir::new().unwrap();

    // Initial detection - no framework
    let detection1 = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection1.framework, TestFramework::None);

    // Add Cargo.toml
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    // New detection should find Cargo
    let detection2 = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection2.framework, TestFramework::Cargo);
}

// ==================== Resource Exhaustion Scenarios ====================

#[test]
fn test_detection_with_many_files() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    // Create many test files
    let tests_dir = dir.path().join("tests");
    std::fs::create_dir_all(&tests_dir).unwrap();

    for i in 0..100 {
        std::fs::write(
            tests_dir.join(format!("test_{}.rs", i)),
            "#[test] fn test_it() {}",
        ).unwrap();
    }

    // Should handle many files without issue
    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Cargo);
}

#[test]
fn test_detection_with_deeply_nested_structure() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    // Create deeply nested structure
    let mut path = dir.path().join("src").join("nested");
    for i in 0..10 {
        path = path.join(format!("level_{}", i));
    }
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(
        path.join("lib.rs"),
        r#"#[cfg(test)]
mod tests { #[test] fn test_it() {} }
"#,
    ).unwrap();

    // Should handle deeply nested structures
    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Cargo);
}

// ==================== Permission Edge Cases ====================

#[test]
fn test_file_exists_with_unreadable_file() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("unreadable.txt");
        std::fs::write(&file_path, "content").unwrap();

        // Remove read permissions
        let mut perms = std::fs::metadata(&file_path).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&file_path, perms).unwrap();

        // Should return false for unreadable file
        assert!(!file_exists_and_readable(&file_path));
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems, skip this test
        let _ = true;
    }
}

#[test]
fn test_directory_exists_with_inaccessible_directory() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let sub_dir = dir.path().join("inaccessible");
        std::fs::create_dir_all(&sub_dir).unwrap();

        // Remove read/execute permissions
        let mut perms = std::fs::metadata(&sub_dir).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&sub_dir, perms).unwrap();

        // May return false for inaccessible directory
        let result = directory_exists_and_accessible(&sub_dir);
        // Result depends on platform and implementation
        let _ = result;
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems, skip this test
        let _ = true;
    }
}

// ==================== Empty/Whitespace Content ====================

#[test]
fn test_parse_toml_empty_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("empty.toml");
    std::fs::write(&file_path, "").unwrap();

    let result = parse_toml_section(&file_path, "section");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_parse_toml_whitespace_only() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("whitespace.toml");
    std::fs::write(&file_path, "   \n\n\t\t\n").unwrap();

    let result = parse_toml_section(&file_path, "section");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_read_file_lines_whitespace_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("whitespace.txt");
    std::fs::write(&file_path, "   \n\n\t\t\n").unwrap();

    let lines = read_file_lines(&file_path, 10).unwrap();
    // Whitespace lines should be preserved
    assert!(!lines.is_empty());
}

// ==================== Mixed Framework Conflict ====================

#[test]
fn test_detection_priority_conflict() {
    let dir = TempDir::new().unwrap();

    // Create all framework indicators
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    std::fs::write(dir.path().join("go.mod"), "module test\n").unwrap();
    std::fs::write(dir.path().join("pytest.ini"), "[pytest]").unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"scripts": {"test": "jest"}}"#,
    ).unwrap();

    // Should prioritize Cargo
    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Cargo);
}
