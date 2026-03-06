//! Test stage of the pipeline
//!
//! This module handles automatic detection and execution of tests across
//! multiple frameworks: pytest, npm, Go, and Cargo.

use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

/// Supported testing frameworks
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestFramework {
    /// Python pytest framework
    Pytest,

    /// Node.js/npm test framework
    Npm,

    /// Go testing framework
    Go,

    /// Rust/Cargo testing framework
    Cargo,

    /// No testing framework detected
    None,
}

impl TestFramework {
    /// Returns the command to run tests for this framework
    pub fn test_command(&self) -> &'static str {
        match self {
            TestFramework::Pytest => "pytest",
            TestFramework::Npm => "npm test",
            TestFramework::Go => "go test ./...",
            TestFramework::Cargo => "cargo test",
            TestFramework::None => "",
        }
    }

    /// Returns the display name of this framework
    pub fn display_name(&self) -> &'static str {
        match self {
            TestFramework::Pytest => "pytest",
            TestFramework::Npm => "npm",
            TestFramework::Go => "Go",
            TestFramework::Cargo => "Cargo",
            TestFramework::None => "None",
        }
    }

    /// Returns true if this framework has configuration files
    pub fn has_config(&self) -> bool {
        matches!(self, TestFramework::Pytest | TestFramework::Npm | TestFramework::Cargo)
    }
}

/// Result of framework detection
#[derive(Debug, Clone)]
pub struct FrameworkDetection {
    /// The detected framework
    pub framework: TestFramework,

    /// Configuration files found
    pub config_files: Vec<PathBuf>,

    /// Test files/directories found
    pub test_paths: Vec<PathBuf>,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
}

impl FrameworkDetection {
    /// Creates a new detection result
    fn new(framework: TestFramework) -> Self {
        FrameworkDetection {
            framework,
            config_files: Vec::new(),
            test_paths: Vec::new(),
            confidence: 0.0,
        }
    }

    /// Adds a configuration file to the detection
    fn with_config(mut self, path: PathBuf) -> Self {
        self.config_files.push(path);
        self
    }

    /// Adds a test path to the detection
    fn with_test_path(mut self, path: PathBuf) -> Self {
        self.test_paths.push(path);
        self
    }

    /// Sets the confidence score
    fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }
}

/// Detects the testing framework in use for the current project
pub fn detect_test_framework(project_dir: &Path) -> Result<FrameworkDetection> {
    // Check in order of specificity: Cargo, Go, pytest, npm
    if let Some(detection) = detect_cargo(project_dir)? {
        return Ok(detection);
    }

    if let Some(detection) = detect_go(project_dir)? {
        return Ok(detection);
    }

    if let Some(detection) = detect_pytest(project_dir)? {
        return Ok(detection);
    }

    if let Some(detection) = detect_npm(project_dir)? {
        return Ok(detection);
    }

    // No framework detected
    Ok(FrameworkDetection::new(TestFramework::None))
}

/// Detects Cargo/Rust testing framework
fn detect_cargo(project_dir: &Path) -> Result<Option<FrameworkDetection>> {
    let cargo_toml = project_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(None);
    }

    let mut detection = FrameworkDetection::new(TestFramework::Cargo)
        .with_config(cargo_toml)
        .with_confidence(1.0);

    // Check for tests/ directory
    let tests_dir = project_dir.join("tests");
    if tests_dir.exists() {
        detection = detection.with_test_path(tests_dir);
    }

    // Check for #[test] attributes in src/
    let src_dir = project_dir.join("src");
    if let Ok(has_tests) = scan_directory_for_test_attributes(&src_dir) {
        if has_tests {
            detection.test_paths.push(src_dir.clone());
        }
    }

    // Check for #[cfg(test)] modules
    if let Ok(has_test_modules) = scan_directory_for_test_modules(&src_dir) {
        if has_test_modules && !detection.test_paths.contains(&src_dir) {
            detection.test_paths.push(src_dir);
        }
    }

    Ok(Some(detection))
}

/// Detects Go testing framework
fn detect_go(project_dir: &Path) -> Result<Option<FrameworkDetection>> {
    let mut detection = FrameworkDetection::new(TestFramework::Go);
    let mut found_indicators = 0;

    // Check for go.mod
    let go_mod = project_dir.join("go.mod");
    if go_mod.exists() {
        detection = detection.with_config(go_mod);
        found_indicators += 1;
    }

    // Scan for _test.go files
    if let Ok(test_files) = find_files_with_suffix(project_dir, "_test.go") {
        if !test_files.is_empty() {
            for file in test_files {
                detection = detection.with_test_path(file);
            }
            found_indicators += 1;
        }
    }

    // Set confidence based on indicators found
    let confidence = match found_indicators {
        0 => 0.0,
        1 => 0.7,
        2 => 1.0,
        _ => 1.0,
    };

    detection = detection.with_confidence(confidence);

    if found_indicators > 0 {
        Ok(Some(detection))
    } else {
        Ok(None)
    }
}

/// Detects Python pytest framework
fn detect_pytest(project_dir: &Path) -> Result<Option<FrameworkDetection>> {
    let mut detection = FrameworkDetection::new(TestFramework::Pytest);
    let mut found_indicators = 0;

    // Check for pytest.ini
    let pytest_ini = project_dir.join("pytest.ini");
    if pytest_ini.exists() {
        detection = detection.with_config(pytest_ini);
        found_indicators += 1;
    }

    // Check for pyproject.toml with [tool.pytest] section
    let pyproject_toml = project_dir.join("pyproject.toml");
    if pyproject_toml.exists() {
        if let Ok(content) = fs::read_to_string(&pyproject_toml) {
            if content.contains("[tool.pytest]") || content.contains("[tool.pytest.ini_options]") {
                detection = detection.with_config(pyproject_toml);
                found_indicators += 1;
            }
        }
    }

    // Check for test_*.py files in common test directories
    for test_dir in &["tests", "test"] {
        let test_path = project_dir.join(test_dir);
        if test_path.exists() {
            if let Ok(test_files) = find_files_with_prefix(&test_path, "test_", ".py") {
                if !test_files.is_empty() {
                    for file in test_files {
                        detection = detection.with_test_path(file);
                    }
                    found_indicators += 1;
                    break;
                }
            }
        }
    }

    // Also check for test_*.py in project root
    if let Ok(test_files) = find_files_with_prefix(project_dir, "test_", ".py") {
        if !test_files.is_empty() {
            for file in test_files {
                if !detection.test_paths.contains(&file) {
                    detection = detection.with_test_path(file);
                }
            }
            found_indicators += 1;
        }
    }

    // Set confidence based on indicators found
    let confidence = match found_indicators {
        0 => 0.0,
        1 => 0.6,
        2 => 0.9,
        3 => 1.0,
        _ => 1.0,
    };

    detection = detection.with_confidence(confidence);

    if found_indicators > 0 {
        Ok(Some(detection))
    } else {
        Ok(None)
    }
}

/// Detects npm/Node.js testing framework
fn detect_npm(project_dir: &Path) -> Result<Option<FrameworkDetection>> {
    let package_json = project_dir.join("package.json");
    if !package_json.exists() {
        return Ok(None);
    }

    // Parse package.json to check for test scripts
    let content = fs::read_to_string(&package_json)
        .context("Failed to read package.json")?;

    let parsed: serde_json::Value = serde_json::from_str(&content)
        .context("Failed to parse package.json")?;

    let mut detection = FrameworkDetection::new(TestFramework::Npm)
        .with_config(package_json);

    let mut found_indicators = 0;

    // Check for "scripts" section with "test" key
    if let Some(scripts) = parsed.get("scripts").and_then(|s| s.as_object()) {
        if scripts.contains_key("test") {
            found_indicators += 1;
        }

        // Also check for related test scripts
        for key in ["test:watch", "test:coverage", "test:unit", "test:integration"] {
            if scripts.contains_key(key) {
                found_indicators += 1;
                break;
            }
        }
    }

    // Check for devDependencies with test frameworks
    if let Some(dev_deps) = parsed.get("devDependencies").and_then(|d| d.as_object()) {
        let test_frameworks = [
            "jest", "mocha", "jasmine", "karma", "ava", "vitest",
            "@jest/globals", "ts-jest", "babel-jest",
        ];

        for framework in test_frameworks {
            if dev_deps.contains_key(framework) {
                found_indicators += 1;
                break;
            }
        }
    }

    // Check for common test directories
    for test_dir in &["tests", "test", "__tests__", "spec"] {
        let test_path = project_dir.join(test_dir);
        if test_path.exists() {
            detection = detection.with_test_path(test_path);
            found_indicators += 1;
            break;
        }
    }

    // Set confidence based on indicators found
    let confidence = match found_indicators {
        0 => 0.0,
        1 => 0.5,
        2 => 0.8,
        3 => 1.0,
        _ => 1.0,
    };

    detection = detection.with_confidence(confidence);

    if found_indicators > 0 {
        Ok(Some(detection))
    } else {
        Ok(None)
    }
}

// ============================================================================
// Helper Functions for File System Checks
// ============================================================================

/// Checks if a directory contains files with #[test] attributes (Rust)
fn scan_directory_for_test_attributes(dir: &Path) -> Result<bool> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(false);
    }

    let entries = fs::read_dir(dir)
        .context("Failed to read directory")?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if scan_directory_for_test_attributes(&path)? {
                return Ok(true);
            }
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = fs::read_to_string(&path) {
                if content.contains("#[test]") || content.contains("#[tokio::test]") {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

/// Checks if a directory contains #[cfg(test)] module declarations (Rust)
fn scan_directory_for_test_modules(dir: &Path) -> Result<bool> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(false);
    }

    let entries = fs::read_dir(dir)
        .context("Failed to read directory")?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if scan_directory_for_test_modules(&path)? {
                return Ok(true);
            }
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = fs::read_to_string(&path) {
                if content.contains("#[cfg(test)]") {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

/// Finds all files with a specific suffix in a directory (recursive)
fn find_files_with_suffix(dir: &Path, suffix: &str) -> Result<Vec<PathBuf>> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    let entries = fs::read_dir(dir)
        .context("Failed to read directory")?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            results.extend(find_files_with_suffix(&path, suffix)?);
        } else if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.ends_with(suffix) {
                results.push(path);
            }
        }
    }

    Ok(results)
}

/// Finds all files with a specific prefix and extension in a directory (recursive)
fn find_files_with_prefix(dir: &Path, prefix: &str, extension: &str) -> Result<Vec<PathBuf>> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    let entries = fs::read_dir(dir)
        .context("Failed to read directory")?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            results.extend(find_files_with_prefix(&path, prefix, extension)?);
        } else if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.starts_with(prefix) && file_name.ends_with(extension) {
                results.push(path);
            }
        }
    }

    Ok(results)
}

/// Parses a TOML configuration file and extracts specific sections
pub fn parse_toml_section(file_path: &Path, section: &str) -> Result<Option<toml::Table>> {
    let content = fs::read_to_string(file_path)
        .context("Failed to read TOML file")?;

    let parsed: toml::Table = toml::from_str(&content)
        .context("Failed to parse TOML file")?;

    // Navigate to the specified section
    let keys: Vec<&str> = section.split('.').collect();
    let mut current = Some(&parsed);

    for key in keys {
        match current {
            Some(table) => {
                current = table.get(key).and_then(|v| v.as_table());
            }
            None => return Ok(None),
        }
    }

    Ok(current.cloned())
}

/// Checks if a file exists and is readable
pub fn file_exists_and_readable(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// Checks if a directory exists and is accessible
pub fn directory_exists_and_accessible(path: &Path) -> bool {
    path.exists() && path.is_dir()
}

/// Reads the first N lines of a file (useful for checking shebangs, etc.)
pub fn read_file_lines(path: &Path, max_lines: usize) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)
        .context("Failed to read file")?;

    Ok(content.lines().take(max_lines).map(String::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_display_names() {
        assert_eq!(TestFramework::Pytest.display_name(), "pytest");
        assert_eq!(TestFramework::Npm.display_name(), "npm");
        assert_eq!(TestFramework::Go.display_name(), "Go");
        assert_eq!(TestFramework::Cargo.display_name(), "Cargo");
        assert_eq!(TestFramework::None.display_name(), "None");
    }

    #[test]
    fn test_framework_test_commands() {
        assert_eq!(TestFramework::Pytest.test_command(), "pytest");
        assert_eq!(TestFramework::Npm.test_command(), "npm test");
        assert_eq!(TestFramework::Go.test_command(), "go test ./...");
        assert_eq!(TestFramework::Cargo.test_command(), "cargo test");
    }

    #[test]
    fn test_framework_has_config() {
        assert!(TestFramework::Pytest.has_config());
        assert!(TestFramework::Npm.has_config());
        assert!(TestFramework::Cargo.has_config());
        assert!(!TestFramework::Go.has_config()); // Go doesn't require config
        assert!(!TestFramework::None.has_config());
    }

    #[test]
    fn test_detection_builder() {
        let detection = FrameworkDetection::new(TestFramework::Pytest)
            .with_config(PathBuf::from("pytest.ini"))
            .with_test_path(PathBuf::from("tests/test_foo.py"))
            .with_confidence(0.9);

        assert_eq!(detection.framework, TestFramework::Pytest);
        assert_eq!(detection.config_files.len(), 1);
        assert_eq!(detection.test_paths.len(), 1);
        assert!((detection.confidence - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn test_file_exists_checks() {
        assert!(file_exists_and_readable(Path::new("Cargo.toml")));
        assert!(!file_exists_and_readable(Path::new("nonexistent.txt")));
    }

    #[test]
    fn test_directory_checks() {
        assert!(directory_exists_and_accessible(Path::new("src")));
        assert!(!directory_exists_and_accessible(Path::new("nonexistent_dir")));
    }

    #[test]
    fn test_read_file_lines() {
        let lines = read_file_lines(Path::new("Cargo.toml"), 3).unwrap();
        assert!(!lines.is_empty());
        assert!(lines.len() <= 3);
    }
}
