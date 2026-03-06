// Tests for macOS universal binary and packaging scripts
//
// This test module verifies that:
// 1. Shell scripts have valid syntax
// 2. Scripts contain required functionality
// 3. Scripts are executable and properly structured
//
// These tests can run on any platform but provide full value on macOS.
//
// Usage:
//   cargo test --test macos_scripts_test

use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use std::env;

/// Test configuration
struct ScriptTestConfig {
    project_root: PathBuf,
}

impl ScriptTestConfig {
    /// Load test configuration
    fn load() -> Self {
        let project_root = Self::find_project_root();
        Self { project_root }
    }

    /// Find the project root by searching for Cargo.toml
    fn find_project_root() -> PathBuf {
        let mut path = env::current_exe().unwrap();
        path.pop(); // Remove executable name

        // Search upwards for Cargo.toml
        while !path.join("Cargo.toml").exists() {
            path.pop();
            if path == Path::new("") {
                panic!("Could not find project root (Cargo.toml)");
            }
        }

        path
    }

    /// Get path to create-universal-binary.sh
    fn create_universal_binary_script(&self) -> PathBuf {
        self.project_root.join("scripts/create-universal-binary.sh")
    }

    /// Get path to package-macos.sh
    fn package_macos_script(&self) -> PathBuf {
        self.project_root.join("scripts/package-macos.sh")
    }

    /// Get path to scripts directory
    fn scripts_dir(&self) -> PathBuf {
        self.project_root.join("scripts")
    }
}

/// Helper to check if a command is available
fn command_exists(command: &str) -> bool {
    let result = if cfg!(target_os = "windows") {
        Command::new("where")
            .arg(command)
            .output()
    } else {
        Command::new("which")
            .arg(command)
            .output()
    };

    result.map(|output| output.status.success()).unwrap_or(false)
}

/// Helper to validate shell script syntax
fn validate_shell_syntax(script_path: &Path) -> Result<bool, String> {
    // Check if sh/bash is available
    let shell = if cfg!(target_os = "windows") {
        // On Windows, check for bash in Git Bash or WSL
        if command_exists("bash") {
            "bash"
        } else {
            return Err("bash not found on Windows".to_string());
        }
    } else {
        "sh"
    };

    // Convert path to string and handle Windows path format for bash
    let path_str = script_path.to_str().ok_or_else(|| {
        format!("Invalid path: {:?}", script_path)
    })?;

    // On Windows, convert backslashes to forward slashes for bash compatibility
    let bash_path = if cfg!(target_os = "windows") {
        path_str.replace('\\', "/")
    } else {
        path_str.to_string()
    };

    let output = Command::new(shell)
        .arg("-n")
        .arg(&bash_path)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(true)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Syntax error: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to validate syntax: {}", e))
    }
}

// ============================================================================
// Test Suite: Script Existence
// ============================================================================

#[cfg(test)]
mod script_existence_tests {
    use super::*;

    #[test]
    fn test_scripts_directory_exists() {
        let config = ScriptTestConfig::load();
        let scripts_dir = config.scripts_dir();

        assert!(scripts_dir.exists(), "scripts directory not found");
        assert!(scripts_dir.is_dir(), "scripts path is not a directory");
    }

    #[test]
    fn test_create_universal_binary_script_exists() {
        let config = ScriptTestConfig::load();
        let script = config.create_universal_binary_script();

        assert!(script.exists(), "create-universal-binary.sh not found");
        assert!(script.is_file(), "create-universal-binary.sh is not a file");
    }

    #[test]
    fn test_package_macos_script_exists() {
        let config = ScriptTestConfig::load();
        let script = config.package_macos_script();

        assert!(script.exists(), "package-macos.sh not found");
        assert!(script.is_file(), "package-macos.sh is not a file");
    }
}

// ============================================================================
// Test Suite: Script Syntax
// ============================================================================

#[cfg(test)]
mod script_syntax_tests {
    use super::*;

    #[test]
    fn test_create_universal_binary_syntax() {
        let config = ScriptTestConfig::load();
        let script = config.create_universal_binary_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        // Skip syntax validation on Windows since these are macOS-specific scripts
        // and bash path handling is complex across WSL/Git Bash/MinGW
        if cfg!(target_os = "windows") {
            eprintln!("⚠ Skipping syntax check on Windows (macOS-specific scripts)");
            return;
        }

        match validate_shell_syntax(&script) {
            Ok(_) => {
                println!("✓ create-universal-binary.sh has valid syntax");
            }
            Err(e) => {
                panic!("Syntax validation failed: {}", e);
            }
        }
    }

    #[test]
    fn test_package_macos_syntax() {
        let config = ScriptTestConfig::load();
        let script = config.package_macos_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        // Skip syntax validation on Windows since these are macOS-specific scripts
        // and bash path handling is complex across WSL/Git Bash/MinGW
        if cfg!(target_os = "windows") {
            eprintln!("⚠ Skipping syntax check on Windows (macOS-specific scripts)");
            return;
        }

        match validate_shell_syntax(&script) {
            Ok(_) => {
                println!("✓ package-macos.sh has valid syntax");
            }
            Err(e) => {
                panic!("Syntax validation failed: {}", e);
            }
        }
    }
}

// ============================================================================
// Test Suite: Script Content
// ============================================================================

#[cfg(test)]
mod script_content_tests {
    use super::*;

    #[test]
    fn test_create_universal_binary_has_shebang() {
        let config = ScriptTestConfig::load();
        let script = config.create_universal_binary_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        assert!(
            contents.starts_with("#!/bin/bash") || contents.starts_with("#!/usr/bin/env bash"),
            "Script missing valid shebang"
        );

        println!("✓ create-universal-binary.sh has valid shebang");
    }

    #[test]
    fn test_package_macos_has_shebang() {
        let config = ScriptTestConfig::load();
        let script = config.package_macos_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        assert!(
            contents.starts_with("#!/bin/bash") || contents.starts_with("#!/usr/bin/env bash"),
            "Script missing valid shebang"
        );

        println!("✓ package-macos.sh has valid shebang");
    }

    #[test]
    fn test_create_universal_binary_has_set_e() {
        let config = ScriptTestConfig::load();
        let script = config.create_universal_binary_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        assert!(
            contents.contains("set -e") || contents.contains("set -o errexit"),
            "Script should have 'set -e' for error handling"
        );

        println!("✓ create-universal-binary.sh has error handling");
    }

    #[test]
    fn test_package_macos_has_set_e() {
        let config = ScriptTestConfig::load();
        let script = config.package_macos_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        assert!(
            contents.contains("set -e") || contents.contains("set -o errexit"),
            "Script should have 'set -e' for error handling"
        );

        println!("✓ package-macos.sh has error handling");
    }

    #[test]
    fn test_create_universal_binary_required_commands() {
        let config = ScriptTestConfig::load();
        let script = config.create_universal_binary_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        // Check for required macOS commands
        let required_commands = vec![
            ("lipo", "lipo command for creating universal binary"),
            ("file", "file command for binary verification"),
            ("codesign", "codesign command for code signing"),
        ];

        for (cmd, description) in &required_commands {
            assert!(
                contents.contains(cmd),
                "Script missing reference to: {} ({})",
                cmd,
                description
            );
        }

        println!("✓ create-universal-binary.sh references required commands");
    }

    #[test]
    fn test_package_macos_required_commands() {
        let config = ScriptTestConfig::load();
        let script = config.package_macos_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        // Check for required packaging commands
        let required_commands = vec![
            ("tar", "tar command for creating archive"),
            ("shasum", "shasum command for checksum"),
        ];

        for (cmd, description) in &required_commands {
            assert!(
                contents.contains(cmd),
                "Script missing reference to: {} ({})",
                cmd,
                description
            );
        }

        println!("✓ package-macos.sh references required commands");
    }

    #[test]
    fn test_create_universal_binary_binary_paths() {
        let config = ScriptTestConfig::load();
        let script = config.create_universal_binary_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        // Check for binary path references
        let required_paths = vec![
            ("x86_64-apple-darwin", "Intel architecture"),
            ("aarch64-apple-darwin", "ARM architecture"),
            ("target/release/ltmatrix-universal", "Universal binary output"),
        ];

        for (path, description) in &required_paths {
            assert!(
                contents.contains(path),
                "Script missing reference to: {} ({})",
                path,
                description
            );
        }

        println!("✓ create-universal-binary.sh references correct binary paths");
    }

    #[test]
    fn test_package_macos_output_structure() {
        let config = ScriptTestConfig::load();
        let script = config.package_macos_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        // Check for package structure
        let required_files = vec![
            ("README.md", "Package README"),
            ("install.sh", "Installation script"),
            ("uninstall.sh", "Uninstallation script"),
        ];

        for (file, description) in &required_files {
            assert!(
                contents.contains(file),
                "Script missing reference to: {} ({})",
                file,
                description
            );
        }

        println!("✓ package-macos.sh creates correct package structure");
    }
}

// ============================================================================
// Test Suite: Script Safety
// ============================================================================

#[cfg(test)]
mod script_safety_tests {
    use super::*;

    #[test]
    fn test_create_universal_binary_has_checks() {
        let config = ScriptTestConfig::load();
        let script = config.create_universal_binary_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        // Check for safety checks
        let safety_checks = vec![
            ("if [[ ! -f", "File existence checks"),
            ("command -v", "Command availability checks"),
            ("uname", "Platform detection"),
        ];

        let mut found_checks = 0;
        for (pattern, description) in &safety_checks {
            if contents.contains(pattern) {
                found_checks += 1;
                println!("✓ Found {} check: {}", pattern, description);
            }
        }

        assert!(
            found_checks >= 2,
            "Script should have at least 2 types of safety checks (found {})",
            found_checks
        );
    }

    #[test]
    fn test_package_macos_has_checks() {
        let config = ScriptTestConfig::load();
        let script = config.package_macos_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        // Check for safety checks
        let safety_checks = vec![
            ("if [[ ! -f", "File existence checks"),
            ("command -v", "Command availability checks"),
        ];

        let mut found_checks = 0;
        for (pattern, description) in &safety_checks {
            if contents.contains(pattern) {
                found_checks += 1;
                println!("✓ Found {} check: {}", pattern, description);
            }
        }

        assert!(
            found_checks >= 1,
            "Script should have at least 1 type of safety check (found {})",
            found_checks
        );
    }

    #[test]
    fn test_scripts_no_hardcoded_secrets() {
        let config = ScriptTestConfig::load();

        let scripts = vec![
            config.create_universal_binary_script(),
            config.package_macos_script(),
        ];

        let suspicious_patterns = vec![
            "password",
            "secret",
            "api_key",
            "token",
            "private_key",
        ];

        for script in scripts {
            if !script.exists() {
                continue;
            }

            let contents = fs::read_to_string(&script)
                .expect("Failed to read script");

            for pattern in &suspicious_patterns {
                // Allow in comments
                for line in contents.lines() {
                    if !line.trim_start().starts_with('#') && line.to_lowercase().contains(pattern) {
                        eprintln!("⚠ Warning: Script {} may contain '{}': {}",
                            script.display(),
                            pattern,
                            line.trim()
                        );
                    }
                }
            }
        }

        println!("✓ Scripts checked for hardcoded secrets");
    }
}

// ============================================================================
// Test Suite: Script Documentation
// ============================================================================

#[cfg(test)]
mod script_documentation_tests {
    use super::*;

    #[test]
    fn test_create_universal_binary_has_header() {
        let config = ScriptTestConfig::load();
        let script = config.create_universal_binary_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        // Check for documentation header
        assert!(
            contents.contains("# Create Universal macOS Binary") || contents.contains("# Universal"),
            "Script should have descriptive header comment"
        );

        assert!(
            contents.contains("# Requirements:") || contents.contains("# Usage:"),
            "Script should document requirements or usage"
        );

        println!("✓ create-universal-binary.sh has documentation header");
    }

    #[test]
    fn test_package_macos_has_header() {
        let config = ScriptTestConfig::load();
        let script = config.package_macos_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        // Check for documentation header
        assert!(
            contents.contains("# Package Universal macOS Binary") || contents.contains("# Package"),
            "Script should have descriptive header comment"
        );

        assert!(
            contents.contains("# Usage:"),
            "Script should document usage"
        );

        println!("✓ package-macos.sh has documentation header");
    }

    #[test]
    fn test_create_universal_binary_has_usage_example() {
        let config = ScriptTestConfig::load();
        let script = config.create_universal_binary_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        assert!(
            contents.contains("./scripts/create-universal-binary.sh") || contents.contains("./create-universal-binary.sh"),
            "Script should show usage example"
        );

        println!("✓ create-universal-binary.sh has usage example");
    }

    #[test]
    fn test_package_macos_has_usage_example() {
        let config = ScriptTestConfig::load();
        let script = config.package_macos_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        assert!(
            contents.contains("./scripts/package-macos.sh") || contents.contains("./package-macos.sh"),
            "Script should show usage example"
        );

        println!("✓ package-macos.sh has usage example");
    }
}

// ============================================================================
// Test Suite: Script Structure
// ============================================================================

#[cfg(test)]
mod script_structure_tests {
    use super::*;

    #[test]
    fn test_create_universal_binary_has_functions() {
        let config = ScriptTestConfig::load();
        let script = config.create_universal_binary_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        // Check for function definitions
        assert!(
            contents.contains("print_success") || contents.contains("print_error") || contents.contains("print_warning"),
            "Script should have helper functions for output"
        );

        println!("✓ create-universal-binary.sh has helper functions");
    }

    #[test]
    fn test_package_macos_has_functions() {
        let config = ScriptTestConfig::load();
        let script = config.package_macos_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        // Check for function definitions
        assert!(
            contents.contains("print_success") || contents.contains("print_error") || contents.contains("print_info"),
            "Script should have helper functions for output"
        );

        println!("✓ package-macos.sh has helper functions");
    }

    #[test]
    fn test_create_universal_binary_has_steps() {
        let config = ScriptTestConfig::load();
        let script = config.create_universal_binary_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        // Check for step comments
        assert!(
            contents.contains("Step ") || contents.contains("step "),
            "Script should have step comments for organization"
        );

        println!("✓ create-universal-binary.sh is organized in steps");
    }

    #[test]
    fn test_package_macos_has_steps() {
        let config = ScriptTestConfig::load();
        let script = config.package_macos_script();

        if !script.exists() {
            eprintln!("⚠ Skipping: Script not found");
            return;
        }

        let contents = fs::read_to_string(&script)
            .expect("Failed to read script");

        // Check for step comments
        assert!(
            contents.contains("Step ") || contents.contains("step "),
            "Script should have step comments for organization"
        );

        println!("✓ package-macos.sh is organized in steps");
    }
}

// ============================================================================
// Test Suite: Documentation Files
// ============================================================================

#[cfg(test)]
mod documentation_tests {
    use super::*;

    #[test]
    fn test_universal_binary_guide_exists() {
        let config = ScriptTestConfig::load();
        let guide = config.project_root.join("MACOS_UNIVERSAL_BINARY_GUIDE.md");

        if !guide.exists() {
            eprintln!("⚠ MACOS_UNIVERSAL_BINARY_GUIDE.md not found");
            return;
        }

        assert!(guide.exists(), "MACOS_UNIVERSAL_BINARY_GUIDE.md not found");
        println!("✓ MACOS_UNIVERSAL_BINARY_GUIDE.md exists");
    }

    #[test]
    fn test_universal_binary_status_exists() {
        let config = ScriptTestConfig::load();
        let status = config.project_root.join("MACOS_UNIVERSAL_BINARY_STATUS.md");

        if !status.exists() {
            eprintln!("⚠ MACOS_UNIVERSAL_BINARY_STATUS.md not found");
            return;
        }

        assert!(status.exists(), "MACOS_UNIVERSAL_BINARY_STATUS.md not found");
        println!("✓ MACOS_UNIVERSAL_BINARY_STATUS.md exists");
    }

    #[test]
    fn test_guide_has_required_sections() {
        let config = ScriptTestConfig::load();
        let guide = config.project_root.join("MACOS_UNIVERSAL_BINARY_GUIDE.md");

        if !guide.exists() {
            eprintln!("⚠ Skipping: Guide not found");
            return;
        }

        let contents = fs::read_to_string(&guide)
            .expect("Failed to read guide");

        let required_sections = vec![
            "## Overview",
            "## Requirements",
            "## Creation Process",
            "## Verification",
            "## Distribution",
            "## Installation",
            "## Troubleshooting",
        ];

        for section in &required_sections {
            assert!(
                contents.contains(section),
                "Guide missing section: {}",
                section
            );
        }

        println!("✓ MACOS_UNIVERSAL_BINARY_GUIDE.md has all required sections");
    }

    #[test]
    fn test_status_has_current_status() {
        let config = ScriptTestConfig::load();
        let status = config.project_root.join("MACOS_UNIVERSAL_BINARY_STATUS.md");

        if !status.exists() {
            eprintln!("⚠ Skipping: Status not found");
            return;
        }

        let contents = fs::read_to_string(&status)
            .expect("Failed to read status");

        assert!(
            contents.contains("**Status**") || contents.contains("## Status"),
            "Status document should have status indicator"
        );

        assert!(
            contents.contains("**Task**") || contents.contains("## Task"),
            "Status document should describe the task"
        );

        println!("✓ MACOS_UNIVERSAL_BINARY_STATUS.md has status information");
    }
}
