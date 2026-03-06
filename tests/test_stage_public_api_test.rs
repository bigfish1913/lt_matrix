//! Public API Tests for Test Stage
//!
//! Tests the public interface of the test stage module, ensuring that
//! all public functions and types are accessible and work correctly.

use ltmatrix::pipeline::test::{
    TestFramework, detect_test_framework,
    file_exists_and_readable, directory_exists_and_accessible,
    read_file_lines, parse_toml_section,
};
use std::path::Path;
use tempfile::TempDir;

// ==================== TestFramework Public API Tests ====================

#[test]
fn test_test_framework_is_public_and_exported() {
    // Verify all framework variants are accessible
    let _ = TestFramework::Pytest;
    let _ = TestFramework::Npm;
    let _ = TestFramework::Go;
    let _ = TestFramework::Cargo;
    let _ = TestFramework::None;
}

#[test]
fn test_framework_test_command_returns_string() {
    let commands = vec![
        TestFramework::Pytest.test_command(),
        TestFramework::Npm.test_command(),
        TestFramework::Go.test_command(),
        TestFramework::Cargo.test_command(),
    ];

    for cmd in commands {
        assert!(!cmd.is_empty());
        assert!(cmd.len() > 0);
    }
}

#[test]
fn test_framework_test_command_none_returns_empty() {
    assert_eq!(TestFramework::None.test_command(), "");
}

#[test]
fn test_framework_display_name_returns_string() {
    let names = vec![
        TestFramework::Pytest.display_name(),
        TestFramework::Npm.display_name(),
        TestFramework::Go.display_name(),
        TestFramework::Cargo.display_name(),
        TestFramework::None.display_name(),
    ];

    for name in names {
        assert!(!name.is_empty());
        assert!(name.len() > 0);
    }
}

#[test]
fn test_framework_has_config_returns_bool() {
    let has_config_results = vec![
        (TestFramework::Pytest, true),
        (TestFramework::Npm, true),
        (TestFramework::Go, false),
        (TestFramework::Cargo, true),
        (TestFramework::None, false),
    ];

    for (framework, expected) in has_config_results {
        assert_eq!(framework.has_config(), expected);
    }
}

// ==================== detect_test_framework Public API Tests ====================

#[test]
fn test_detect_test_framework_is_public() {
    let dir = TempDir::new().unwrap();
    let result = detect_test_framework(dir.path());
    assert!(result.is_ok());
}

#[test]
fn test_detect_test_framework_returns_result() {
    let dir = TempDir::new().unwrap();
    let detection = detect_test_framework(dir.path()).unwrap();

    // Can access the framework field
    let _framework = detection.framework;
    let _config_files = detection.config_files.clone();
    let _test_paths = detection.test_paths.clone();
    let _confidence = detection.confidence;
}

#[test]
fn test_detect_empty_directory() {
    let dir = TempDir::new().unwrap();
    let detection = detect_test_framework(dir.path()).unwrap();

    assert_eq!(detection.framework, TestFramework::None);
    assert!(detection.config_files.is_empty());
    assert!(detection.test_paths.is_empty());
    assert_eq!(detection.confidence, 0.0);
}

#[test]
fn test_detect_cargo_project() {
    let dir = TempDir::new().unwrap();
    let cargo_toml = dir.path().join("Cargo.toml");
    std::fs::write(
        &cargo_toml,
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Cargo);
}

#[test]
fn test_detect_pytest_project() {
    let dir = TempDir::new().unwrap();
    let pytest_ini = dir.path().join("pytest.ini");
    std::fs::write(&pytest_ini, "[pytest]").unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Pytest);
}

#[test]
fn test_detect_npm_project() {
    let dir = TempDir::new().unwrap();
    let package_json = dir.path().join("package.json");
    std::fs::write(
        &package_json,
        r#"{
  "scripts": {
    "test": "jest"
  }
}
"#,
    ).unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Npm);
}

#[test]
fn test_detect_go_project() {
    let dir = TempDir::new().unwrap();
    let go_mod = dir.path().join("go.mod");
    std::fs::write(&go_mod, "module test\ngo 1.21\n").unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Go);
}

// ==================== File Utility Public API Tests ====================

#[test]
fn test_file_exists_returns_false_for_nonexistent() {
    let result = file_exists_and_readable(Path::new("this_file_does_not_exist_12345.txt"));
    assert!(!result);
}

#[test]
fn test_file_exists_returns_true_for_existing_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.txt");
    std::fs::write(&file_path, "content").unwrap();

    assert!(file_exists_and_readable(&file_path));
}

#[test]
fn test_directory_exists_returns_false_for_nonexistent() {
    let result = directory_exists_and_accessible(Path::new("this_dir_does_not_exist_12345"));
    assert!(!result);
}

#[test]
fn test_directory_exists_returns_true_for_existing_directory() {
    let dir = TempDir::new().unwrap();
    assert!(directory_exists_and_accessible(dir.path()));
}

#[test]
fn test_directory_exists_returns_false_for_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.txt");
    std::fs::write(&file_path, "content").unwrap();

    assert!(!directory_exists_and_accessible(&file_path));
}

#[test]
fn test_file_exists_returns_false_for_directory() {
    let dir = TempDir::new().unwrap();
    assert!(!file_exists_and_readable(dir.path()));
}

#[test]
fn test_read_file_lines_returns_result() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.txt");
    std::fs::write(&file_path, "line1\nline2\nline3").unwrap();

    let result = read_file_lines(&file_path, 10);
    assert!(result.is_ok());
}

#[test]
fn test_read_file_lines_returns_correct_lines() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.txt");
    std::fs::write(&file_path, "line1\nline2\nline3").unwrap();

    let lines = read_file_lines(&file_path, 10).unwrap();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "line1");
    assert_eq!(lines[1], "line2");
    assert_eq!(lines[2], "line3");
}

#[test]
fn test_read_file_lines_respects_limit() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.txt");
    std::fs::write(&file_path, "line1\nline2\nline3\nline4\nline5").unwrap();

    let lines = read_file_lines(&file_path, 3).unwrap();
    assert_eq!(lines.len(), 3);
}

#[test]
fn test_read_file_lines_zero_limit() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.txt");
    std::fs::write(&file_path, "line1\nline2\nline3").unwrap();

    let lines = read_file_lines(&file_path, 0).unwrap();
    assert_eq!(lines.len(), 0);
}

#[test]
fn test_read_file_lines_nonexistent_file() {
    let result = read_file_lines(Path::new("nonexistent.txt"), 5);
    assert!(result.is_err());
}

#[test]
fn test_parse_toml_section_returns_result() {
    let dir = TempDir::new().unwrap();
    let toml_path = dir.path().join("test.toml");
    std::fs::write(
        &toml_path,
        r#"[section]
key = "value"
"#,
    ).unwrap();

    let result = parse_toml_section(&toml_path, "section");
    assert!(result.is_ok());
}

#[test]
fn test_parse_toml_section_returns_some_for_existing_section() {
    let dir = TempDir::new().unwrap();
    let toml_path = dir.path().join("test.toml");
    std::fs::write(
        &toml_path,
        r#"[section]
key = "value"
"#,
    ).unwrap();

    let result = parse_toml_section(&toml_path, "section").unwrap();
    assert!(result.is_some());
}

#[test]
fn test_parse_toml_section_returns_none_for_missing_section() {
    let dir = TempDir::new().unwrap();
    let toml_path = dir.path().join("test.toml");
    std::fs::write(
        &toml_path,
        r#"[section]
key = "value"
"#,
    ).unwrap();

    let result = parse_toml_section(&toml_path, "nonexistent").unwrap();
    assert!(result.is_none());
}

#[test]
fn test_parse_toml_section_nonexistent_file() {
    let result = parse_toml_section(Path::new("nonexistent.toml"), "section");
    assert!(result.is_err());
}

// ==================== Integration API Tests ====================

#[test]
fn test_full_detection_workflow_cargo() {
    let dir = TempDir::new().unwrap();

    // Create Cargo.toml
    let cargo_toml = dir.path().join("Cargo.toml");
    std::fs::write(
        &cargo_toml,
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    // Create tests directory
    let tests_dir = dir.path().join("tests");
    std::fs::create_dir_all(&tests_dir).unwrap();
    std::fs::write(tests_dir.join("test.rs"), "#[test] fn test_it() {}").unwrap();

    // Run detection
    let detection = detect_test_framework(dir.path()).unwrap();

    // Verify results
    assert_eq!(detection.framework, TestFramework::Cargo);
    assert!(!detection.config_files.is_empty());
    assert!(!detection.test_paths.is_empty());
    assert!(detection.confidence > 0.9);
}

#[test]
fn test_detection_mixed_project() {
    let dir = TempDir::new().unwrap();

    // Create mixed project files
    std::fs::write(dir.path().join("package.json"), r#"{"name": "test"}"#).unwrap();
    std::fs::write(dir.path().join("go.mod"), "module test\n").unwrap();
    std::fs::write(dir.path().join("pytest.ini"), "[pytest]").unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    ).unwrap();

    // Cargo should take priority
    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Cargo);
}

#[test]
fn test_detection_with_test_files() {
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
    std::fs::write(tests_dir.join("integration_test.rs"), "#[test] fn test() {}").unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Cargo);
    assert!(!detection.test_paths.is_empty());
}

// ==================== Type Trait Tests ====================

#[test]
fn test_framework_implements_clone() {
    let framework = TestFramework::Cargo;
    let cloned = framework.clone();
    assert_eq!(framework, cloned);
}

#[test]
fn test_framework_implements_partial_eq() {
    assert_eq!(TestFramework::Pytest, TestFramework::Pytest);
    assert_ne!(TestFramework::Pytest, TestFramework::Npm);
}

#[test]
fn test_framework_debug_format() {
    let framework = TestFramework::Cargo;
    let debug_str = format!("{:?}", framework);
    assert!(debug_str.contains("Cargo"));
}

#[test]
fn test_detection_implements_clone() {
    let dir = TempDir::new().unwrap();
    let detection = detect_test_framework(dir.path()).unwrap();

    let cloned = detection.clone();
    assert_eq!(cloned.framework, detection.framework);
    assert_eq!(cloned.config_files, detection.config_files);
    assert_eq!(cloned.test_paths, detection.test_paths);
    assert_eq!(cloned.confidence, detection.confidence);
}

#[test]
fn test_detection_debug_format() {
    let dir = TempDir::new().unwrap();
    let detection = detect_test_framework(dir.path()).unwrap();

    let debug_str = format!("{:?}", detection);
    // Debug output should contain the framework name
    assert!(!debug_str.is_empty());
}

// ==================== Real-World Scenario Tests ====================

#[test]
fn test_detect_current_workspace() {
    // Try to detect framework in the current workspace
    let detection = detect_test_framework(Path::new("."));
    // Should not crash, should return something
    assert!(detection.is_ok());
}

#[test]
fn test_file_utilities_with_cargo_toml() {
    assert!(file_exists_and_readable(Path::new("Cargo.toml")));
    assert!(directory_exists_and_accessible(Path::new("src")));

    let lines = read_file_lines(Path::new("Cargo.toml"), 5);
    assert!(lines.is_ok());
    assert!(!lines.unwrap().is_empty());
}
