//! Framework Detection Tests
//!
//! Comprehensive tests for test framework detection functionality across
//! multiple programming languages and testing frameworks.

use ltmatrix::pipeline::test::{
    detect_test_framework, file_exists_and_readable, directory_exists_and_accessible,
    TestFramework,
};
use std::path::Path;
use tempfile::TempDir;

/// Creates a temporary Cargo project with tests
fn create_cargo_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    let cargo_toml = dir.path().join("Cargo.toml");
    std::fs::write(
        &cargo_toml,
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    ).unwrap();

    let src_dir = dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        r#"pub fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(1, 2), 3);
    }
}
"#,
    ).unwrap();

    let tests_dir = dir.path().join("tests");
    std::fs::create_dir_all(&tests_dir).unwrap();
    std::fs::write(
        tests_dir.join("integration_test.rs"),
        r#"#[test]
fn test_integration() {
    assert!(true);
}
"#,
    ).unwrap();

    dir
}

/// Creates a temporary Python project with pytest
fn create_pytest_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    let pytest_ini = dir.path().join("pytest.ini");
    std::fs::write(
        &pytest_ini,
        r#"[pytest]
testpaths = tests
"#,
    ).unwrap();

    let tests_dir = dir.path().join("tests");
    std::fs::create_dir_all(&tests_dir).unwrap();
    std::fs::write(
        tests_dir.join("test_math.py"),
        r#"def test_addition():
    assert 1 + 1 == 2
"#,
    ).unwrap();

    dir
}

/// Creates a temporary npm project with tests
fn create_npm_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    let package_json = dir.path().join("package.json");
    std::fs::write(
        &package_json,
        r#"{
  "name": "test-project",
  "version": "1.0.0",
  "scripts": {
    "test": "jest"
  },
  "devDependencies": {
    "jest": "^29.0.0"
  }
}
"#,
    ).unwrap();

    let tests_dir = dir.path().join("tests");
    std::fs::create_dir_all(&tests_dir).unwrap();

    dir
}

/// Creates a temporary Go project with tests
fn create_go_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    let go_mod = dir.path().join("go.mod");
    std::fs::write(
        &go_mod,
        r#"module test-project

go 1.21
"#,
    ).unwrap();

    std::fs::write(
        dir.path().join("math_test.go"),
        r#"package testproject

import "testing"

func TestAdd(t *testing.T) {
    if 1 + 1 != 2 {
        t.Error("Expected 1 + 1 to equal 2")
    }
}
"#,
    ).unwrap();

    dir
}

#[test]
fn test_detect_cargo_framework() {
    let project = create_cargo_project();
    let detection = detect_test_framework(project.path()).unwrap();

    assert_eq!(detection.framework, TestFramework::Cargo);
    assert!(!detection.config_files.is_empty());
    assert!(detection.confidence > 0.9);

    // Verify Cargo.toml is detected
    let cargo_toml = project.path().join("Cargo.toml");
    assert!(detection.config_files.iter().any(|p| p == &cargo_toml));
}

#[test]
fn test_detect_cargo_with_tests_directory() {
    let project = create_cargo_project();
    let detection = detect_test_framework(project.path()).unwrap();

    // Should detect tests/ directory
    let tests_dir = project.path().join("tests");
    assert!(detection.test_paths.iter().any(|p| p == &tests_dir));
}

#[test]
fn test_detect_cargo_with_inline_tests() {
    let project = create_cargo_project();
    let detection = detect_test_framework(project.path()).unwrap();

    // Should detect src/ directory with #[test] attributes
    let src_dir = project.path().join("src");
    assert!(detection.test_paths.iter().any(|p| p == &src_dir));
}

#[test]
fn test_detect_pytest_framework() {
    let project = create_pytest_project();
    let detection = detect_test_framework(project.path()).unwrap();

    assert_eq!(detection.framework, TestFramework::Pytest);
    assert!(!detection.config_files.is_empty());
    assert!(detection.confidence > 0.8);
}

#[test]
fn test_detect_pytest_with_pytest_ini() {
    let project = create_pytest_project();
    let detection = detect_test_framework(project.path()).unwrap();

    let pytest_ini = project.path().join("pytest.ini");
    assert!(detection.config_files.iter().any(|p| p == &pytest_ini));
}

#[test]
fn test_detect_pytest_with_test_files() {
    let project = create_pytest_project();
    let detection = detect_test_framework(project.path()).unwrap();

    // Should detect test_*.py files
    assert!(!detection.test_paths.is_empty());
    assert!(detection.test_paths.iter().any(|p| {
        p.to_string_lossy().contains("test_math.py")
    }));
}

#[test]
fn test_detect_npm_framework() {
    let project = create_npm_project();
    let detection = detect_test_framework(project.path()).unwrap();

    assert_eq!(detection.framework, TestFramework::Npm);
    assert!(!detection.config_files.is_empty());
    assert!(detection.confidence > 0.7);
}

#[test]
fn test_detect_npm_with_package_json() {
    let project = create_npm_project();
    let detection = detect_test_framework(project.path()).unwrap();

    let package_json = project.path().join("package.json");
    assert!(detection.config_files.iter().any(|p| p == &package_json));
}

#[test]
fn test_detect_npm_with_test_script() {
    let project = create_npm_project();
    let detection = detect_test_framework(project.path()).unwrap();

    // Should detect test script in package.json
    assert!(detection.confidence > 0.5);
}

#[test]
fn test_detect_go_framework() {
    let project = create_go_project();
    let detection = detect_test_framework(project.path()).unwrap();

    assert_eq!(detection.framework, TestFramework::Go);
    assert!(!detection.config_files.is_empty());
}

#[test]
fn test_detect_go_with_go_mod() {
    let project = create_go_project();
    let detection = detect_test_framework(project.path()).unwrap();

    let go_mod = project.path().join("go.mod");
    assert!(detection.config_files.iter().any(|p| p == &go_mod));
}

#[test]
fn test_detect_go_with_test_files() {
    let project = create_go_project();
    let detection = detect_test_framework(project.path()).unwrap();

    // Should detect _test.go files
    assert!(!detection.test_paths.is_empty());
    assert!(detection.test_paths.iter().any(|p| {
        p.to_string_lossy().ends_with("_test.go")
    }));
}

#[test]
fn test_detect_no_framework() {
    let dir = TempDir::new().unwrap();
    let detection = detect_test_framework(dir.path()).unwrap();

    assert_eq!(detection.framework, TestFramework::None);
    assert!(detection.config_files.is_empty());
    assert!(detection.test_paths.is_empty());
    assert_eq!(detection.confidence, 0.0);
}

#[test]
fn test_framework_priority_cargo_first() {
    let cargo_project = create_cargo_project();

    // Even if we add a package.json, Cargo should be detected first
    let package_json = cargo_project.path().join("package.json");
    std::fs::write(&package_json, r#"{"name": "mixed-project"}"#).unwrap();

    let detection = detect_test_framework(cargo_project.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Cargo);
}

#[test]
fn test_framework_priority_go_second() {
    let go_project = create_go_project();

    // Add a pytest.ini - Go should still be detected
    let pytest_ini = go_project.path().join("pytest.ini");
    std::fs::write(&pytest_ini, r#"[pytest]"#).unwrap();

    let detection = detect_test_framework(go_project.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Go);
}

#[test]
fn test_framework_priority_pytest_third() {
    let pytest_project = create_pytest_project();

    // Add a package.json - pytest should still be detected
    let package_json = pytest_project.path().join("package.json");
    std::fs::write(&package_json, r#"{"name": "mixed-project"}"#).unwrap();

    let detection = detect_test_framework(pytest_project.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Pytest);
}

#[test]
fn test_file_exists_and_readable_valid_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.txt");
    std::fs::write(&file_path, "content").unwrap();

    assert!(file_exists_and_readable(&file_path));
}

#[test]
fn test_file_exists_and_readable_nonexistent_file() {
    assert!(!file_exists_and_readable(Path::new("nonexistent.txt")));
}

#[test]
fn test_file_exists_and_readable_directory() {
    let dir = TempDir::new().unwrap();
    assert!(!file_exists_and_readable(dir.path()));
}

#[test]
fn test_directory_exists_and_accessible_valid_directory() {
    let dir = TempDir::new().unwrap();
    assert!(directory_exists_and_accessible(dir.path()));
}

#[test]
fn test_directory_exists_and_accessible_nonexistent_directory() {
    assert!(!directory_exists_and_accessible(Path::new("nonexistent_dir")));
}

#[test]
fn test_directory_exists_and_accessible_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.txt");
    std::fs::write(&file_path, "content").unwrap();

    assert!(!directory_exists_and_accessible(&file_path));
}

#[test]
fn test_cargo_detection_high_confidence() {
    let project = create_cargo_project();
    let detection = detect_test_framework(project.path()).unwrap();

    // Cargo.toml presence + tests directory should give high confidence
    assert!(detection.confidence >= 0.9);
}

#[test]
fn test_pytest_detection_with_pyproject_toml() {
    let dir = TempDir::new().unwrap();
    let pyproject_toml = dir.path().join("pyproject.toml");
    std::fs::write(
        &pyproject_toml,
        r#"[tool.pytest.ini_options]
testpaths = ["tests"]
"#,
    ).unwrap();

    let tests_dir = dir.path().join("tests");
    std::fs::create_dir_all(&tests_dir).unwrap();
    std::fs::write(
        tests_dir.join("test_example.py"),
        "def test_example(): pass",
    ).unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Pytest);
    assert!(detection.confidence > 0.8);
}

#[test]
fn test_npm_detection_with_multiple_test_scripts() {
    let dir = TempDir::new().unwrap();
    let package_json = dir.path().join("package.json");
    std::fs::write(
        &package_json,
        r#"{
  "scripts": {
    "test": "jest",
    "test:watch": "jest --watch",
    "test:coverage": "jest --coverage"
  }
}
"#,
    ).unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Npm);
    // Multiple test scripts should increase confidence
    assert!(detection.confidence > 0.7);
}

#[test]
fn test_npm_detection_with_test_framework_dev_dependency() {
    let dir = TempDir::new().unwrap();
    let package_json = dir.path().join("package.json");
    std::fs::write(
        &package_json,
        r#"{
  "devDependencies": {
    "jest": "^29.0.0",
    "typescript": "^5.0.0"
  }
}
"#,
    ).unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Npm);
}

#[test]
fn test_go_detection_without_go_mod() {
    let dir = TempDir::new().unwrap();

    // Create only _test.go file, no go.mod
    std::fs::write(
        dir.path().join("handler_test.go"),
        r#"package main

import "testing"

func TestHandler(t *testing.T) {}
"#,
    ).unwrap();

    let detection = detect_test_framework(dir.path()).unwrap();
    assert_eq!(detection.framework, TestFramework::Go);
    // Lower confidence without go.mod
    assert!(detection.confidence < 1.0);
    assert!(detection.confidence > 0.0);
}
