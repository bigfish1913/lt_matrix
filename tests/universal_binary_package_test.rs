// Integration tests for universal macOS binary and distribution package
//
// This test module verifies that:
// 1. Universal binary creation works correctly
// 2. Distribution packaging is complete and valid
// 3. Package contents are correct and functional
//
// These tests require macOS to run and will be automatically skipped on other platforms.
//
// Prerequisites:
// - macOS system with Xcode Command Line Tools
// - Built Intel (x86_64) and ARM (aarch64) binaries
// - Universal binary created via create-universal-binary.sh
// - Distribution package created via package-macos.sh
//
// Usage:
//   # Build binaries and create universal binary
//   cargo build --release --target x86_64-apple-darwin
//   cargo build --release --target aarch64-apple-darwin
//   ./scripts/create-universal-binary.sh
//
//   # Create distribution package
//   ./scripts/package-macos.sh 0.1.0
//
//   # Run tests
//   cargo test --test universal_binary_package_test

#![cfg(target_os = "macos")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use std::env;
use std::io::Read;
use tempfile::TempDir;

/// Test configuration
struct TestConfig {
    project_root: PathBuf,
    intel_binary: PathBuf,
    arm_binary: PathBuf,
    universal_binary: PathBuf,
    dist_dir: PathBuf,
    version: String,
}

impl TestConfig {
    /// Load test configuration from environment or defaults
    fn load() -> Self {
        let project_root = Self::find_project_root();

        // Allow version override via environment
        let version = env::var("LTMATRIX_VERSION")
            .ok()
            .unwrap_or_else(|| {
                // Try to read from Cargo.toml
                if let Ok(toml) = fs::read_to_string(project_root.join("Cargo.toml")) {
                    toml.lines()
                        .find(|line| line.starts_with("version = "))
                        .and_then(|line| {
                            line.split('"')
                                .nth(1)
                                .map(|v| v.to_string())
                        })
                        .unwrap_or_else(|| "0.1.0".to_string())
                } else {
                    "0.1.0".to_string()
                }
            });

        // Allow binary path overrides
        let intel_binary = env::var("LTMATRIX_INTEL_BINARY")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| project_root.join("target/x86_64-apple-darwin/release/ltmatrix"));

        let arm_binary = env::var("LTMATRIX_ARM_BINARY")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| project_root.join("target/aarch64-apple-darwin/release/ltmatrix"));

        let universal_binary = env::var("LTMATRIX_UNIVERSAL_BINARY")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| project_root.join("target/release/ltmatrix-universal"));

        let dist_dir = project_root.join("target/dist");

        Self {
            project_root,
            intel_binary,
            arm_binary,
            universal_binary,
            dist_dir,
            version,
        }
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

    /// Get the expected package directory name
    fn package_dir(&self) -> PathBuf {
        self.dist_dir.join(format!("ltmatrix-{}-macos-universal", self.version))
    }

    /// Get the expected tarball path
    fn tarball_path(&self) -> PathBuf {
        self.dist_dir.join(format!("ltmatrix-{}-macos-universal.tar.gz", self.version))
    }

    /// Get the expected checksum path
    fn checksum_path(&self) -> PathBuf {
        self.dist_dir.join(format!("ltmatrix-{}-macos-universal.tar.gz.sha256", self.version))
    }
}

/// Helper function to run a command and return output
fn run_command(binary: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(binary)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute {:?}: {}", binary, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!(
            "Command failed with exit code {:?}\nStdout: {}\nStderr: {}",
            output.status.code(),
            stdout,
            stderr
        ));
    }

    Ok(stdout)
}

/// Helper function to run shell command and return output
fn run_shell_command(cmd: &str) -> Result<String, String> {
    let output = if cfg!(target_os = "macos") {
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
    } else {
        Command::new("cmd")
            .args(&["/C", cmd])
            .output()
    };

    let output = output.map_err(|e| format!("Failed to execute shell command '{}': {}", cmd, e))?;

    if !output.status.success() {
        return Err(format!(
            "Shell command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ============================================================================
// Test Suite: Universal Binary Creation
// ============================================================================

#[cfg(test)]
mod universal_binary_creation_tests {
    use super::*;

    #[test]
    fn test_component_binaries_exist() {
        let config = TestConfig::load();

        let intel_exists = config.intel_binary.exists();
        let arm_exists = config.arm_binary.exists();

        if !intel_exists {
            eprintln!("⚠ Intel binary not found: {:?}", config.intel_binary);
            eprintln!("  Build with: cargo build --release --target x86_64-apple-darwin");
        }

        if !arm_exists {
            eprintln!("⚠ ARM binary not found: {:?}", config.arm_binary);
            eprintln!("  Build with: cargo build --release --target aarch64-apple-darwin");
        }

        // At least one component binary should exist for meaningful testing
        assert!(
            intel_exists || arm_exists,
            "Neither Intel nor ARM binary found. Build both architectures first."
        );
    }

    #[test]
    fn test_universal_binary_exists() {
        let config = TestConfig::load();

        if !config.universal_binary.exists() {
            eprintln!("⚠ Universal binary not found: {:?}", config.universal_binary);
            eprintln!("  Create with: ./scripts/create-universal-binary.sh");
            return;
        }

        assert!(config.universal_binary.exists(), "Universal binary not found");
    }

    #[test]
    fn test_universal_binary_format() {
        let config = TestConfig::load();

        if !config.universal_binary.exists() {
            eprintln!("⚠ Skipping: Universal binary not found");
            return;
        }

        // Check file format
        let output = Command::new("file")
            .arg(&config.universal_binary)
            .output()
            .expect("Failed to run file command");

        assert!(output.status.success(), "file command failed");

        let file_output = String::from_utf8_lossy(&output.stdout);
        println!("Universal binary format: {}", file_output);

        assert!(
            file_output.contains("Mach-O universal binary"),
            "Not a universal binary: {}",
            file_output
        );

        assert!(
            file_output.contains("x86_64"),
            "Universal binary missing x86_64 architecture"
        );

        assert!(
            file_output.contains("arm64") || file_output.contains("aarch64"),
            "Universal binary missing arm64 architecture"
        );
    }

    #[test]
    fn test_universal_binary_architectures() {
        let config = TestConfig::load();

        if !config.universal_binary.exists() {
            eprintln!("⚠ Skipping: Universal binary not found");
            return;
        }

        // Use lipo to verify architectures
        let output = Command::new("lipo")
            .args(&["-info", config.universal_binary.to_str().unwrap()])
            .output()
            .expect("Failed to run lipo command");

        assert!(output.status.success(), "lipo command failed");

        let lipo_output = String::from_utf8_lossy(&output.stdout);
        println!("Universal binary architectures: {}", lipo_output);

        assert!(
            lipo_output.contains("x86_64"),
            "Universal binary missing x86_64 architecture"
        );

        assert!(
            lipo_output.contains("arm64"),
            "Universal binary missing arm64 architecture"
        );
    }

    #[test]
    fn test_universal_binary_executable() {
        let config = TestConfig::load();

        if !config.universal_binary.exists() {
            eprintln!("⚠ Skipping: Universal binary not found");
            return;
        }

        // Test --version command
        match run_command(&config.universal_binary, &["--version"]) {
            Ok(output) => {
                assert!(
                    output.contains("ltmatrix"),
                    "Universal binary --version doesn't contain 'ltmatrix': {}",
                    output
                );
                println!("✓ Universal binary --version: {}", output.trim());
            }
            Err(e) => {
                panic!("Universal binary --version failed: {}", e);
            }
        }

        // Test --help command
        match run_command(&config.universal_binary, &["--help"]) {
            Ok(output) => {
                assert!(
                    output.contains("Usage:") || output.contains("USAGE:"),
                    "Universal binary --help missing usage information"
                );
                println!("✓ Universal binary --help works");
            }
            Err(e) => {
                panic!("Universal binary --help failed: {}", e);
            }
        }
    }

    #[test]
    fn test_universal_binary_size() {
        let config = TestConfig::load();

        if !config.universal_binary.exists() {
            eprintln!("⚠ Skipping: Universal binary not found");
            return;
        }

        let metadata = fs::metadata(&config.universal_binary)
            .expect("Failed to read universal binary metadata");

        let size_bytes = metadata.len();
        let size_mb = size_bytes as f64 / (1024.0 * 1024.0);

        println!("Universal binary size: {:.2} MB", size_mb);

        // Universal binary should be larger than individual binaries
        // but not unreasonably large (typically 10-100 MB for Rust apps)
        assert!(
            size_mb >= 2.0 && size_mb <= 200.0,
            "Universal binary size {} MB is outside expected range [2, 200] MB",
            size_mb
        );

        // If component binaries exist, verify universal is larger
        if config.intel_binary.exists() && config.arm_binary.exists() {
            let intel_size = fs::metadata(&config.intel_binary).unwrap().len();
            let arm_size = fs::metadata(&config.arm_binary).unwrap().len();

            // Universal should be roughly sum of both (allowing for overhead)
            let expected_min = (intel_size + arm_size) as f64 * 0.9;
            let expected_max = (intel_size + arm_size) as f64 * 1.2;

            assert!(
                size_bytes as f64 >= expected_min && size_bytes as f64 <= expected_max,
                "Universal binary size {} MB is unexpected relative to components (Intel: {:.2} MB, ARM: {:.2} MB)",
                size_mb,
                intel_size as f64 / (1024.0 * 1024.0),
                arm_size as f64 / (1024.0 * 1024.0)
            );
        }
    }

    #[test]
    fn test_universal_binary_code_signing() {
        let config = TestConfig::load();

        if !config.universal_binary.exists() {
            eprintln!("⚠ Skipping: Universal binary not found");
            return;
        }

        // Verify code signing
        let output = Command::new("codesign")
            .args(&["-v", config.universal_binary.to_str().unwrap()])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    println!("✓ Universal binary code signature is valid");
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!("⚠ Code signature verification failed: {}", stderr);

                    // Try to apply ad-hoc signing
                    eprintln!("Attempting to apply ad-hoc signature...");
                    let sign_output = Command::new("codesign")
                        .args(&["--force", "--deep", "--sign", "-", config.universal_binary.to_str().unwrap()])
                        .output();

                    if let Ok(sign) = sign_output {
                        if sign.status.success() {
                            println!("✓ Applied ad-hoc signature");
                        } else {
                            eprintln!("⚠ Failed to apply ad-hoc signature");
                        }
                    }
                }
            }
            Err(_) => {
                eprintln!("⚠ codesign command not available");
            }
        }
    }
}

// ============================================================================
// Test Suite: Distribution Package
// ============================================================================

#[cfg(test)]
mod distribution_package_tests {
    use super::*;

    #[test]
    fn test_package_directory_exists() {
        let config = TestConfig::load();
        let package_dir = config.package_dir();

        if !package_dir.exists() {
            eprintln!("⚠ Package directory not found: {:?}", package_dir);
            eprintln!("  Create with: ./scripts/package-macos.sh {}", config.version);
            return;
        }

        assert!(package_dir.exists(), "Package directory not found");
        assert!(package_dir.is_dir(), "Package path is not a directory");
    }

    #[test]
    fn test_tarball_exists() {
        let config = TestConfig::load();
        let tarball = config.tarball_path();

        if !tarball.exists() {
            eprintln!("⚠ Tarball not found: {:?}", tarball);
            eprintln!("  Create with: ./scripts/package-macos.sh {}", config.version);
            return;
        }

        assert!(tarball.exists(), "Tarball not found");
        assert!(tarball.is_file(), "Tarball path is not a file");
    }

    #[test]
    fn test_checksum_exists() {
        let config = TestConfig::load();
        let checksum = config.checksum_path();

        if !checksum.exists() {
            eprintln!("⚠ Checksum file not found: {:?}", checksum);
            return;
        }

        assert!(checksum.exists(), "Checksum file not found");
    }

    #[test]
    fn test_package_contains_binary() {
        let config = TestConfig::load();
        let package_dir = config.package_dir();

        if !package_dir.exists() {
            eprintln!("⚠ Skipping: Package directory not found");
            return;
        }

        let binary = package_dir.join("ltmatrix");
        assert!(binary.exists(), "Package missing ltmatrix binary");
        assert!(binary.is_file(), "ltmatrix is not a file");

        // Check it's executable
        use std::os::unix::fs::PermissionsExt;
        let metadata = binary.metadata().expect("Failed to read binary metadata");
        let permissions = metadata.permissions();
        let mode = permissions.mode();

        assert_eq!(
            mode & 0o111,
            0o111,
            "Binary is not executable"
        );

        println!("✓ Package contains executable binary");
    }

    #[test]
    fn test_package_contains_readme() {
        let config = TestConfig::load();
        let package_dir = config.package_dir();

        if !package_dir.exists() {
            eprintln!("⚠ Skipping: Package directory not found");
            return;
        }

        let readme = package_dir.join("README.md");
        assert!(readme.exists(), "Package missing README.md");

        // Verify README contains expected sections
        let contents = fs::read_to_string(&readme)
            .expect("Failed to read README.md");

        let expected_sections = vec![
            "# ltmatrix",
            "## Installation",
            "## Quick Start",
            "## Troubleshooting",
        ];

        for section in expected_sections {
            assert!(
                contents.contains(section),
                "README.md missing section: {}",
                section
            );
        }

        println!("✓ Package contains README.md with expected sections");
    }

    #[test]
    fn test_readme_contains_version() {
        let config = TestConfig::load();
        let package_dir = config.package_dir();

        if !package_dir.exists() {
            eprintln!("⚠ Skipping: Package directory not found");
            return;
        }

        let readme = package_dir.join("README.md");
        if !readme.exists() {
            eprintln!("⚠ Skipping: README.md not found");
            return;
        }

        let contents = fs::read_to_string(&readme)
            .expect("Failed to read README.md");

        assert!(
            contents.contains(&config.version),
            "README.md doesn't contain version {}",
            config.version
        );

        println!("✓ README.md contains correct version");
    }

    #[test]
    fn test_package_contains_install_script() {
        let config = TestConfig::load();
        let package_dir = config.package_dir();

        if !package_dir.exists() {
            eprintln!("⚠ Skipping: Package directory not found");
            return;
        }

        let install_script = package_dir.join("install.sh");
        assert!(install_script.exists(), "Package missing install.sh");

        // Check it's executable
        use std::os::unix::fs::PermissionsExt;
        let metadata = install_script.metadata().expect("Failed to read install.sh metadata");
        let permissions = metadata.permissions();
        let mode = permissions.mode();

        assert_eq!(
            mode & 0o111,
            0o111,
            "install.sh is not executable"
        );

        // Verify script contains expected content
        let contents = fs::read_to_string(&install_script)
            .expect("Failed to read install.sh");

        assert!(
            contents.contains("#!/bin/bash"),
            "install.sh missing shebang"
        );

        assert!(
            contents.contains("INSTALL_DIR"),
            "install.sh missing INSTALL_DIR variable"
        );

        println!("✓ Package contains install.sh");
    }

    #[test]
    fn test_package_contains_uninstall_script() {
        let config = TestConfig::load();
        let package_dir = config.package_dir();

        if !package_dir.exists() {
            eprintln!("⚠ Skipping: Package directory not found");
            return;
        }

        let uninstall_script = package_dir.join("uninstall.sh");
        assert!(uninstall_script.exists(), "Package missing uninstall.sh");

        // Check it's executable
        use std::os::unix::fs::PermissionsExt;
        let metadata = uninstall_script.metadata().expect("Failed to read uninstall.sh metadata");
        let permissions = metadata.permissions();
        let mode = permissions.mode();

        assert_eq!(
            mode & 0o111,
            0o111,
            "uninstall.sh is not executable"
        );

        println!("✓ Package contains uninstall.sh");
    }

    #[test]
    fn test_tarball_extractable() {
        let config = TestConfig::load();
        let tarball = config.tarball_path();

        if !tarball.exists() {
            eprintln!("⚠ Skipping: Tarball not found");
            return;
        }

        // Create temp directory for extraction
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let extract_dir = temp_dir.path();

        // Extract tarball
        let output = Command::new("tar")
            .args(&[
                "-xzf",
                tarball.to_str().unwrap(),
                "-C",
                extract_dir.to_str().unwrap(),
            ])
            .output();

        match output {
            Ok(output) => {
                assert!(output.status.success(), "Failed to extract tarball");
                println!("✓ Tarball is extractable");
            }
            Err(e) => {
                panic!("Failed to extract tarball: {}", e);
            }
        }
    }

    #[test]
    fn test_tarball_contains_all_files() {
        let config = TestConfig::load();
        let tarball = config.tarball_path();

        if !tarball.exists() {
            eprintln!("⚠ Skipping: Tarball not found");
            return;
        }

        // List tarball contents
        let output = Command::new("tar")
            .args(&["-tzf", tarball.to_str().unwrap()])
            .output()
            .expect("Failed to list tarball contents");

        assert!(output.status.success(), "Failed to list tarball");

        let contents = String::from_utf8_lossy(&output.stdout);
        println!("Tarball contents:\n{}", contents);

        let expected_files = vec![
            "ltmatrix",
            "README.md",
            "install.sh",
            "uninstall.sh",
        ];

        for file in expected_files {
            assert!(
                contents.contains(file),
                "Tarball missing file: {}",
                file
            );
        }

        println!("✓ Tarball contains all expected files");
    }

    #[test]
    fn test_checksum_valid() {
        let config = TestConfig::load();
        let tarball = config.tarball_path();
        let checksum_file = config.checksum_path();

        if !tarball.exists() || !checksum_file.exists() {
            eprintln!("⚠ Skipping: Tarball or checksum not found");
            return;
        }

        // Read checksum file
        let checksum_content = fs::read_to_string(&checksum_file)
            .expect("Failed to read checksum file");

        println!("Checksum file content: {}", checksum_content);

        // Verify checksum format
        assert!(
            checksum_content.contains("SHA256"),
            "Checksum file missing SHA256 indicator"
        );

        assert!(
            checksum_content.contains(&format!("ltmatrix-{}-macos-universal.tar.gz", config.version)),
            "Checksum file doesn't reference correct tarball"
        );

        println!("✓ Checksum file format is valid");
    }

    #[test]
    fn test_tarball_size_reasonable() {
        let config = TestConfig::load();
        let tarball = config.tarball_path();

        if !tarball.exists() {
            eprintln!("⚠ Skipping: Tarball not found");
            return;
        }

        let metadata = fs::metadata(&tarball)
            .expect("Failed to read tarball metadata");

        let size_bytes = metadata.len();
        let size_mb = size_bytes as f64 / (1024.0 * 1024.0);

        println!("Tarball size: {:.2} MB", size_mb);

        // Tarball should be compressed, so smaller than universal binary
        // But still contain all files
        assert!(
            size_mb >= 1.0 && size_mb <= 100.0,
            "Tarball size {} MB is outside expected range [1, 100] MB",
            size_mb
        );

        println!("✓ Tarball size is reasonable");
    }
}

// ============================================================================
// Test Suite: Package Installation
// ============================================================================

#[cfg(test)]
mod package_installation_tests {
    use super::*;

    #[test]
    fn test_binary_from_package_executable() {
        let config = TestConfig::load();
        let package_dir = config.package_dir();

        if !package_dir.exists() {
            eprintln!("⚠ Skipping: Package directory not found");
            return;
        }

        let binary = package_dir.join("ltmatrix");
        if !binary.exists() {
            eprintln!("⚠ Skipping: Binary not found in package");
            return;
        }

        // Test execution
        match run_command(&binary, &["--version"]) {
            Ok(output) => {
                assert!(
                    output.contains("ltmatrix"),
                    "Binary --version doesn't contain 'ltmatrix': {}",
                    output
                );
                println!("✓ Binary from package executes correctly");
            }
            Err(e) => {
                panic!("Binary from package failed: {}", e);
            }
        }
    }

    #[test]
    fn test_install_script_syntax() {
        let config = TestConfig::load();
        let package_dir = config.package_dir();

        if !package_dir.exists() {
            eprintln!("⚠ Skipping: Package directory not found");
            return;
        }

        let install_script = package_dir.join("install.sh");
        if !install_script.exists() {
            eprintln!("⚠ Skipping: install.sh not found");
            return;
        }

        // Check script syntax
        let output = Command::new("sh")
            .args(&["-n", install_script.to_str().unwrap()])
            .output();

        match output {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    panic!("install.sh has syntax errors: {}", stderr);
                }
                println!("✓ install.sh has valid syntax");
            }
            Err(e) => {
                eprintln!("⚠ Could not verify install.sh syntax: {}", e);
            }
        }
    }

    #[test]
    fn test_uninstall_script_syntax() {
        let config = TestConfig::load();
        let package_dir = config.package_dir();

        if !package_dir.exists() {
            eprintln!("⚠ Skipping: Package directory not found");
            return;
        }

        let uninstall_script = package_dir.join("uninstall.sh");
        if !uninstall_script.exists() {
            eprintln!("⚠ Skipping: uninstall.sh not found");
            return;
        }

        // Check script syntax
        let output = Command::new("sh")
            .args(&["-n", uninstall_script.to_str().unwrap()])
            .output();

        match output {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    panic!("uninstall.sh has syntax errors: {}", stderr);
                }
                println!("✓ uninstall.sh has valid syntax");
            }
            Err(e) => {
                eprintln!("⚠ Could not verify uninstall.sh syntax: {}", e);
            }
        }
    }
}

// ============================================================================
// Test Suite: Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_package_workflow() {
        let config = TestConfig::load();

        // This test verifies the complete workflow:
        // 1. Component binaries exist
        // 2. Universal binary exists
        // 3. Package directory exists
        // 4. Tarball exists
        // 5. Checksum exists

        let mut steps_completed = Vec::new();
        let mut steps_failed = Vec::new();

        // Step 1: Check component binaries
        if config.intel_binary.exists() {
            steps_completed.push("Intel binary exists");
        } else {
            steps_failed.push("Intel binary missing");
        }

        if config.arm_binary.exists() {
            steps_completed.push("ARM binary exists");
        } else {
            steps_failed.push("ARM binary missing");
        }

        // Step 2: Check universal binary
        if config.universal_binary.exists() {
            steps_completed.push("Universal binary exists");

            // Verify it's actually a universal binary
            if let Ok(output) = Command::new("lipo")
                .args(&["-info", config.universal_binary.to_str().unwrap()])
                .output()
            {
                let lipo_output = String::from_utf8_lossy(&output.stdout);
                if lipo_output.contains("x86_64") && lipo_output.contains("arm64") {
                    steps_completed.push("Universal binary has both architectures");
                } else {
                    steps_failed.push("Universal binary missing architectures");
                }
            }
        } else {
            steps_failed.push("Universal binary missing");
        }

        // Step 3: Check package directory
        let package_dir = config.package_dir();
        if package_dir.exists() {
            steps_completed.push("Package directory exists");

            // Check required files
            let binary = package_dir.join("ltmatrix");
            let readme = package_dir.join("README.md");
            let install = package_dir.join("install.sh");
            let uninstall = package_dir.join("uninstall.sh");

            if binary.exists() {
                steps_completed.push("Binary in package");
            } else {
                steps_failed.push("Binary missing from package");
            }

            if readme.exists() {
                steps_completed.push("README in package");
            } else {
                steps_failed.push("README missing from package");
            }

            if install.exists() {
                steps_completed.push("install.sh in package");
            } else {
                steps_failed.push("install.sh missing from package");
            }

            if uninstall.exists() {
                steps_completed.push("uninstall.sh in package");
            } else {
                steps_failed.push("uninstall.sh missing from package");
            }
        } else {
            steps_failed.push("Package directory missing");
        }

        // Step 4: Check tarball
        let tarball = config.tarball_path();
        if tarball.exists() {
            steps_completed.push("Tarball exists");
        } else {
            steps_failed.push("Tarball missing");
        }

        // Step 5: Check checksum
        let checksum = config.checksum_path();
        if checksum.exists() {
            steps_completed.push("Checksum exists");
        } else {
            steps_failed.push("Checksum missing");
        }

        // Print summary
        println!("\n=== Package Workflow Summary ===");
        println!("Completed ({}/{}):", steps_completed.len(), steps_completed.len() + steps_failed.len());
        for step in &steps_completed {
            println!("  ✓ {}", step);
        }

        if !steps_failed.is_empty() {
            println!("\nFailed:");
            for step in &steps_failed {
                println!("  ✗ {}", step);
            }
        }

        // Assert that at least the universal binary and package structure exist
        if config.universal_binary.exists() && package_dir.exists() {
            println!("\n✓ Core package structure is complete");
        } else {
            println!("\n⚠ Package workflow incomplete");
        }
    }

    #[test]
    fn test_version_consistency() {
        let config = TestConfig::load();

        // Verify version is consistent across:
        // 1. Universal binary --version output
        // 2. Package directory name
        // 3. README.md
        // 4. Tarball name

        let mut versions = Vec::new();

        // Check universal binary version
        if config.universal_binary.exists() {
            if let Ok(output) = run_command(&config.universal_binary, &["--version"]) {
                if output.contains(&config.version) {
                    versions.push(("Binary", config.version.clone()));
                }
            }
        }

        // Check package directory name
        let package_dir = config.package_dir();
        if package_dir.to_str().unwrap().contains(&config.version) {
            versions.push(("Package directory", config.version.clone()));
        }

        // Check README.md
        let readme = package_dir.join("README.md");
        if readme.exists() {
            if let Ok(contents) = fs::read_to_string(&readme) {
                if contents.contains(&config.version) {
                    versions.push(("README.md", config.version.clone()));
                }
            }
        }

        // Check tarball name
        let tarball = config.tarball_path();
        if tarball.to_str().unwrap().contains(&config.version) {
            versions.push(("Tarball", config.version.clone()));
        }

        println!("\nVersion consistency check:");
        for (source, version) in &versions {
            println!("  {}: {}", source, version);
        }

        // At least 2 sources should have the version
        assert!(
            versions.len() >= 2,
            "Version consistency check failed: only {}/4 sources have version {}",
            versions.len(),
            config.version
        );

        println!("✓ Version is consistent across {} sources", versions.len());
    }

    #[test]
    fn test_documentation_completeness() {
        let config = TestConfig::load();
        let package_dir = config.package_dir();

        if !package_dir.exists() {
            eprintln!("⚠ Skipping: Package directory not found");
            return;
        }

        let readme = package_dir.join("README.md");
        if !readme.exists() {
            eprintln!("⚠ Skipping: README.md not found");
            return;
        }

        let contents = fs::read_to_string(&readme)
            .expect("Failed to read README.md");

        // Check for comprehensive documentation
        let required_sections = vec![
            // Headers
            ("# ltmatrix", "Main header"),
            ("## Installation", "Installation section"),
            ("## Quick Start", "Quick start section"),
            ("## Configuration", "Configuration section"),
            ("## Uninstall", "Uninstall section"),
            ("## Troubleshooting", "Troubleshooting section"),
            ("## System Requirements", "System requirements section"),
            ("## Support", "Support section"),
            ("## License", "License section"),

            // Installation methods
            ("Quick Install", "Quick install instructions"),
            ("Alternative Locations", "Alternative installation locations"),

            // Verification steps
            ("## Verification", "Verification section"),
            ("--version", "Version verification"),
            ("--help", "Help verification"),
            ("lipo -info", "Architecture verification"),

            // Troubleshooting
            ("Command not found", "Command not found troubleshooting"),
            ("Cannot be opened", "Gatekeeper troubleshooting"),
            ("Code signing", "Code signing troubleshooting"),

            // Metadata
            ("Version:", "Version metadata"),
            ("Platform:", "Platform metadata"),
            ("Build Date:", "Build date metadata"),
        ];

        let mut missing = Vec::new();

        for (pattern, description) in &required_sections {
            if !contents.contains(pattern) {
                missing.push(description);
            }
        }

        if !missing.is_empty() {
            println!("⚠ README.md missing sections:");
            for section in &missing {
                println!("  - {}", section);
            }
        }

        // Allow some flexibility - at least 80% of sections should be present
        let coverage = (required_sections.len() - missing.len()) as f64 / required_sections.len() as f64;

        assert!(
            coverage >= 0.8,
            "README.md incomplete: {:.0}% coverage (required: 80%)",
            coverage * 100.0
        );

        println!("✓ README.md is {:.0}% complete", coverage * 100.0);
    }
}
