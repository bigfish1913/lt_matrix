//! Integration tests for test framework detection
//!
//! These tests create temporary project structures to verify
//! framework detection logic for pytest, npm, Go, and Cargo.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;
use ltmatrix::pipeline::test::{
    detect_test_framework, TestFramework, file_exists_and_readable,
    directory_exists_and_accessible, parse_toml_section, read_file_lines,
};

/// Creates a temporary directory with a file
fn create_temp_file(dir: &Path, name: &str, content: &str) -> std::io::Result<()> {
    let file_path = dir.join(name);
    let mut file = File::create(&file_path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

#[test]
fn test_detect_cargo_project() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Create a minimal Cargo.toml
    create_temp_file(project_dir, "Cargo.toml", r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#).unwrap();

    // Create src directory with a test
    fs::create_dir_all(project_dir.join("src")).unwrap();
    create_temp_file(&project_dir.join("src"), "lib.rs", r#"
#[cfg(test)]
mod tests {
    #[test]
    fn test_example() {
        assert!(true);
    }
}
"#).unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    assert_eq!(detection.framework, TestFramework::Cargo);
    assert!(detection.config_files.iter().any(|p| p.ends_with("Cargo.toml")));
    assert_eq!(detection.confidence, 1.0);
}

#[test]
fn test_detect_cargo_with_tests_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "Cargo.toml", "[package]\nname = \"test\"\n").unwrap();
    fs::create_dir_all(project_dir.join("tests")).unwrap();
    create_temp_file(&project_dir.join("tests"), "integration_test.rs", "// test file").unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    assert_eq!(detection.framework, TestFramework::Cargo);
    assert!(detection.test_paths.iter().any(|p| p.ends_with("tests")));
}

#[test]
fn test_detect_go_project() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "go.mod", "module example\n\ngo 1.21\n").unwrap();
    create_temp_file(project_dir, "main_test.go", "package main\n\nimport \"testing\"\n\nfunc TestExample(t *testing.T) {}\n").unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    assert_eq!(detection.framework, TestFramework::Go);
    assert!(detection.config_files.iter().any(|p| p.ends_with("go.mod")));
    // Check that test files were found (check file name, not full path)
    assert!(detection.test_paths.iter().any(|p| p.file_name().unwrap().to_str().unwrap().ends_with("_test.go")));
    assert_eq!(detection.confidence, 1.0);
}

#[test]
fn test_detect_go_with_only_test_files() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "utils_test.go", "package main\n\nfunc TestUtils(t *testing.T) {}\n").unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    assert_eq!(detection.framework, TestFramework::Go);
    assert!((detection.confidence - 0.7).abs() < 0.01); // Should be 0.7 with only test files
}

#[test]
fn test_detect_pytest_with_pytest_ini() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "pytest.ini", "[pytest]\nminversion = 7.0\n").unwrap();
    fs::create_dir_all(project_dir.join("tests")).unwrap();
    create_temp_file(&project_dir.join("tests"), "test_example.py", "def test_example(): pass\n").unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    assert_eq!(detection.framework, TestFramework::Pytest);
    assert!(detection.config_files.iter().any(|p| p.ends_with("pytest.ini")));
    assert!(detection.test_paths.iter().any(|p| p.to_str().unwrap().contains("test_example")));
}

#[test]
fn test_detect_pytest_with_pyproject_toml() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "pyproject.toml", r#"
[tool.pytest.ini_options]
minversion = "7.0"
"#).unwrap();

    create_temp_file(project_dir, "test_main.py", "def test_main(): pass\n").unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    assert_eq!(detection.framework, TestFramework::Pytest);
    assert!(detection.config_files.iter().any(|p| p.ends_with("pyproject.toml")));
}

#[test]
fn test_detect_pytest_with_test_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::create_dir_all(project_dir.join("tests")).unwrap();
    create_temp_file(&project_dir.join("tests"), "test_foo.py", "def test_foo(): pass\n").unwrap();
    create_temp_file(&project_dir.join("tests"), "test_bar.py", "def test_bar(): pass\n").unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    assert_eq!(detection.framework, TestFramework::Pytest);
    assert_eq!(detection.test_paths.len(), 2);
}

#[test]
fn test_detect_npm_with_test_script() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "package.json", r#"
{
  "name": "test-project",
  "version": "1.0.0",
  "scripts": {
    "test": "jest"
  }
}
"#).unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    assert_eq!(detection.framework, TestFramework::Npm);
    assert!(detection.config_files.iter().any(|p| p.ends_with("package.json")));
    assert!(detection.confidence >= 0.5);
}

#[test]
fn test_detect_npm_with_test_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "package.json", r#"
{
  "name": "test-project",
  "scripts": {
    "test": "mocha"
  },
  "devDependencies": {
    "jest": "^29.0.0"
  }
}
"#).unwrap();

    fs::create_dir_all(project_dir.join("__tests__")).unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    assert_eq!(detection.framework, TestFramework::Npm);
    assert!(detection.test_paths.iter().any(|p| p.ends_with("__tests__")));
    assert_eq!(detection.confidence, 1.0);
}

#[test]
fn test_detect_npm_with_multiple_test_scripts() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "package.json", r#"
{
  "name": "test-project",
  "scripts": {
    "test": "jest",
    "test:watch": "jest --watch",
    "test:coverage": "jest --coverage"
  }
}
"#).unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    assert_eq!(detection.framework, TestFramework::Npm);
    assert!(detection.confidence >= 0.8); // Should be higher with multiple scripts
}

#[test]
fn test_detect_npm_with_jest_dependency() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "package.json", r#"
{
  "name": "test-project",
  "devDependencies": {
    "jest": "^29.0.0",
    "@types/jest": "^29.0.0"
  }
}
"#).unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    assert_eq!(detection.framework, TestFramework::Npm);
}

#[test]
fn test_detect_no_framework() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Empty directory with no framework indicators
    let detection = detect_test_framework(project_dir).unwrap();

    assert_eq!(detection.framework, TestFramework::None);
    assert_eq!(detection.confidence, 0.0);
    assert!(detection.config_files.is_empty());
    assert!(detection.test_paths.is_empty());
}

#[test]
fn test_framework_priority_cargo_over_others() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Create indicators for multiple frameworks
    create_temp_file(project_dir, "Cargo.toml", "[package]\nname = \"test\"\n").unwrap();
    create_temp_file(project_dir, "package.json", "{\"scripts\": {\"test\": \"jest\"}}\n").unwrap();
    create_temp_file(project_dir, "go.mod", "module test\n").unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    // Cargo should be detected first (highest priority)
    assert_eq!(detection.framework, TestFramework::Cargo);
}

#[test]
fn test_framework_priority_go_over_pytest_and_npm() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "go.mod", "module test\n").unwrap();
    create_temp_file(project_dir, "package.json", "{\"scripts\": {\"test\": \"jest\"}}\n").unwrap();
    create_temp_file(project_dir, "pytest.ini", "[pytest]\n").unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    // Go should be detected before pytest and npm
    assert_eq!(detection.framework, TestFramework::Go);
}

#[test]
fn test_framework_priority_pytest_over_npm() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "pytest.ini", "[pytest]\n").unwrap();
    create_temp_file(project_dir, "package.json", "{\"scripts\": {\"test\": \"jest\"}}\n").unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    // pytest should be detected before npm
    assert_eq!(detection.framework, TestFramework::Pytest);
}

#[test]
fn test_parse_toml_section_valid() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "test.toml", r#"
[package]
name = "test"

[dependencies]
serde = "1.0"

[tool.pytest.ini_options]
minversion = "7.0"
"#).unwrap();

    let result = parse_toml_section(&project_dir.join("test.toml"), "tool.pytest.ini_options");
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

#[test]
fn test_parse_toml_section_missing() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "test.toml", r#"
[package]
name = "test"
"#).unwrap();

    let result = parse_toml_section(&project_dir.join("test.toml"), "missing.section");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_parse_toml_section_invalid_file() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    let result = parse_toml_section(&project_dir.join("nonexistent.toml"), "section");
    assert!(result.is_err());
}

#[test]
fn test_file_exists_and_readable_file() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "test.txt", "content").unwrap();

    assert!(file_exists_and_readable(&project_dir.join("test.txt")));
    assert!(!file_exists_and_readable(&project_dir.join("nonexistent.txt")));
}

#[test]
fn test_file_exists_and_readable_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::create_dir_all(project_dir.join("test_dir")).unwrap();

    // Should return false for directories
    assert!(!file_exists_and_readable(&project_dir.join("test_dir")));
}

#[test]
fn test_directory_exists_and_accessible() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::create_dir_all(project_dir.join("test_dir")).unwrap();
    create_temp_file(project_dir, "test.txt", "content").unwrap();

    assert!(directory_exists_and_accessible(&project_dir.join("test_dir")));
    assert!(!directory_exists_and_accessible(&project_dir.join("nonexistent_dir")));
    assert!(!directory_exists_and_accessible(&project_dir.join("test.txt")));
}

#[test]
fn test_read_file_lines() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "test.txt", "line1\nline2\nline3\nline4\n").unwrap();

    let lines = read_file_lines(&project_dir.join("test.txt"), 2).unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "line1");
    assert_eq!(lines[1], "line2");
}

#[test]
fn test_read_file_lines_more_than_available() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "test.txt", "line1\nline2\n").unwrap();

    let lines = read_file_lines(&project_dir.join("test.txt"), 10).unwrap();
    assert_eq!(lines.len(), 2);
}

#[test]
fn test_read_file_lines_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "empty.txt", "").unwrap();

    let lines = read_file_lines(&project_dir.join("empty.txt"), 5).unwrap();
    assert!(lines.is_empty());
}

#[test]
fn test_read_file_lines_nonexistent() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    let result = read_file_lines(&project_dir.join("nonexistent.txt"), 5);
    assert!(result.is_err());
}

#[test]
fn test_detect_cargo_recursive_test_scanning() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    create_temp_file(project_dir, "Cargo.toml", "[package]\nname = \"test\"\n").unwrap();

    // Create nested directory structure with tests
    fs::create_dir_all(project_dir.join("src/utils")).unwrap();
    create_temp_file(&project_dir.join("src/utils"), "helpers.rs", r#"
#[cfg(test)]
mod tests {
    #[test]
    fn test_helper() {
        assert!(true);
    }
}
"#).unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    assert_eq!(detection.framework, TestFramework::Cargo);
    // Should find test paths in nested structure
    assert!(!detection.test_paths.is_empty());
}

#[test]
fn test_detect_pytest_confidence_levels() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Test with 1 indicator (pytest.ini only)
    create_temp_file(project_dir, "pytest.ini", "[pytest]\n").unwrap();
    let detection = detect_test_framework(project_dir).unwrap();
    assert_eq!(detection.framework, TestFramework::Pytest);
    assert!((detection.confidence - 0.6).abs() < 0.1);

    // Clean up and test with 2 indicators
    let temp_dir2 = TempDir::new().unwrap();
    let project_dir2 = temp_dir2.path();
    create_temp_file(project_dir2, "pytest.ini", "[pytest]\n").unwrap();
    create_temp_file(project_dir2, "test_main.py", "def test(): pass\n").unwrap();

    let detection2 = detect_test_framework(project_dir2).unwrap();
    assert_eq!(detection2.framework, TestFramework::Pytest);
    assert!((detection2.confidence - 0.9).abs() < 0.1);
}

#[test]
fn test_detect_go_without_go_mod() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Only test files, no go.mod
    create_temp_file(project_dir, "handler_test.go", "package main\n\nfunc TestHandler(t *testing.T) {}\n").unwrap();
    create_temp_file(project_dir, "utils_test.go", "package main\n\nfunc TestUtils(t *testing.T) {}\n").unwrap();

    let detection = detect_test_framework(project_dir).unwrap();

    assert_eq!(detection.framework, TestFramework::Go);
    // Check that we have multiple test files (by checking file names)
    let test_count = detection.test_paths.iter()
        .filter(|p| p.file_name().unwrap().to_str().unwrap().ends_with("_test.go"))
        .count();
    assert!(test_count >= 2);
}
