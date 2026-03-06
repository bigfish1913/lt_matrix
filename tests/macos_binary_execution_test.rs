// Integration tests for macOS binary execution verification
//
// This test module verifies that ltmatrix binaries execute correctly on macOS
// for both Intel (x86_64) and Apple Silicon (aarch64) architectures.
//
// These tests require macOS to run and will be automatically skipped on other platforms.
//
// Prerequisites:
// - macOS system (Intel or Apple Silicon)
// - Built binaries for both architectures
// - Xcode Command Line Tools (for codesign, otool, etc.)
//
// Usage:
//   cargo test --test macos_binary_execution_test
//
// Or build binaries first:
//   cargo build --release --target x86_64-apple-darwin
//   cargo build --release --target aarch64-apple-darwin
//   cargo test --test macos_binary_execution_test

#![cfg(target_os = "macos")]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Test configuration
struct TestConfig {
    intel_binary: PathBuf,
    arm_binary: PathBuf,
    project_root: PathBuf,
}

impl TestConfig {
    /// Load test configuration from environment or defaults
    fn load() -> Self {
        let project_root = Self::find_project_root();

        // Allow override via environment variables
        let intel_binary = env::var("LTMATRIX_INTEL_BINARY")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| project_root.join("target/x86_64-apple-darwin/release/ltmatrix"));

        let arm_binary = env::var("LTMATRIX_ARM_BINARY")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| project_root.join("target/aarch64-apple-darwin/release/ltmatrix"));

        Self {
            intel_binary,
            arm_binary,
            project_root,
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

    /// Check if a binary exists and is executable
    fn binary_exists(&self, binary: &Path) -> bool {
        binary.exists() && binary.is_file()
    }

    /// Get architecture name for a binary path
    fn get_arch_name(&self, binary: &Path) -> &str {
        if binary == self.intel_binary {
            "Intel (x86_64)"
        } else if binary == self.arm_binary {
            "Apple Silicon (aarch64)"
        } else {
            "Unknown"
        }
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

/// Helper function to run otool -L on a binary
fn get_dependencies(binary: &Path) -> Result<String, String> {
    let output = Command::new("otool")
        .arg("-L")
        .arg(binary)
        .output()
        .map_err(|e| format!("Failed to run otool: {}", e))?;

    if !output.status.success() {
        return Err(format!("otool failed: {:?}", output.status.code()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Helper function to check code signing status
fn check_code_signing(binary: &Path) -> Result<CodeSigningInfo, String> {
    let output = Command::new("codesign").arg("-dvv").arg(binary).output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            let is_signed = output.status.success() || stderr.contains("Signature");
            let signature_type = if stdout.contains("adhoc") || stderr.contains("adhoc") {
                "ad-hoc"
            } else if stdout.contains("Mac Developer") || stdout.contains("Developer ID") {
                "developer"
            } else {
                "unknown"
            };

            Ok(CodeSigningInfo {
                is_signed,
                signature_type: signature_type.to_string(),
                details: format!("{}\n{}", stdout, stderr),
            })
        }
        Err(_) => Ok(CodeSigningInfo {
            is_signed: false,
            signature_type: "none".to_string(),
            details: "codesign command failed".to_string(),
        }),
    }
}

struct CodeSigningInfo {
    is_signed: bool,
    signature_type: String,
    details: String,
}

// ============================================================================
// Test Suite: Both Architectures
// ============================================================================

#[cfg(test)]
mod both_architectures_tests {
    use super::*;

    fn get_config() -> TestConfig {
        TestConfig::load()
    }

    /// Test that both binaries exist (skip if missing)
    #[test]
    fn test_both_binaries_exist() {
        let config = get_config();

        let intel_exists = config.binary_exists(&config.intel_binary);
        let arm_exists = config.binary_exists(&config.arm_binary);

        if !intel_exists {
            eprintln!("⚠ Intel binary not found at: {:?}", config.intel_binary);
            eprintln!("  Build with: cargo build --release --target x86_64-apple-darwin");
        }

        if !arm_exists {
            eprintln!("⚠ ARM binary not found at: {:?}", config.arm_binary);
            eprintln!("  Build with: cargo build --release --target aarch64-apple-darwin");
        }

        // At least one should exist for meaningful testing
        assert!(
            intel_exists || arm_exists,
            "Neither Intel nor ARM binary found. Build both architectures first."
        );
    }

    /// Test --version command on both binaries
    #[test]
    fn test_version_both_architectures() {
        let config = get_config();
        let binaries = vec![
            (&config.intel_binary, "Intel"),
            (&config.arm_binary, "Apple Silicon"),
        ];

        for (binary, arch_name) in binaries {
            if !config.binary_exists(binary) {
                eprintln!("⚠ Skipping {} binary: not found", arch_name);
                continue;
            }

            match run_command(binary, &["--version"]) {
                Ok(output) => {
                    assert!(
                        output.contains("ltmatrix"),
                        "{}: --version output doesn't contain 'ltmatrix': {}",
                        arch_name,
                        output
                    );
                    println!("✓ {} --version: {}", arch_name, output.trim());
                }
                Err(e) => {
                    panic!("{}: --version command failed: {}", arch_name, e);
                }
            }
        }
    }

    /// Test --help command on both binaries
    #[test]
    fn test_help_both_architectures() {
        let config = get_config();
        let binaries = vec![
            (&config.intel_binary, "Intel"),
            (&config.arm_binary, "Apple Silicon"),
        ];

        for (binary, arch_name) in binaries {
            if !config.binary_exists(binary) {
                eprintln!("⚠ Skipping {} binary: not found", arch_name);
                continue;
            }

            match run_command(binary, &["--help"]) {
                Ok(output) => {
                    assert!(
                        output.contains("Usage:")
                            || output.contains("USAGE:")
                            || output.contains("usage:"),
                        "{}: --help output doesn't contain usage information",
                        arch_name
                    );
                    println!("✓ {} --help works", arch_name);
                }
                Err(e) => {
                    panic!("{}: --help command failed: {}", arch_name, e);
                }
            }
        }
    }

    /// Test dependencies with otool on both binaries
    #[test]
    fn test_dependencies_both_architectures() {
        let config = get_config();
        let binaries = vec![
            (&config.intel_binary, "Intel"),
            (&config.arm_binary, "Apple Silicon"),
        ];

        for (binary, arch_name) in binaries {
            if !config.binary_exists(binary) {
                eprintln!("⚠ Skipping {} binary: not found", arch_name);
                continue;
            }

            match get_dependencies(binary) {
                Ok(deps) => {
                    println!("✓ {} dependencies:\n{}", arch_name, deps);

                    // Check for expected system frameworks
                    let has_core_foundation = deps.contains("CoreFoundation");
                    let has_libsystem = deps.contains("libSystem");

                    if has_core_foundation {
                        println!("  ✓ Links to CoreFoundation (expected)");
                    }
                    if has_libsystem {
                        println!("  ✓ Links to libSystem (expected)");
                    }

                    // Check for suspicious dependencies
                    let suspicious = vec!["/usr/local/lib", "/opt/homebrew", "/homebrew/lib"];

                    for pattern in suspicious {
                        if deps.contains(pattern) {
                            eprintln!("  ⚠ Warning: Found suspicious dependency: {}", pattern);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("⚠ {}: Failed to check dependencies: {}", arch_name, e);
                }
            }
        }
    }

    /// Test code signing on both binaries
    #[test]
    fn test_code_signing_both_architectures() {
        let config = get_config();
        let binaries = vec![
            (&config.intel_binary, "Intel"),
            (&config.arm_binary, "Apple Silicon"),
        ];

        for (binary, arch_name) in binaries {
            if !config.binary_exists(binary) {
                eprintln!("⚠ Skipping {} binary: not found", arch_name);
                continue;
            }

            match check_code_signing(binary) {
                Ok(info) => {
                    if info.is_signed {
                        println!("✓ {} code signing: {}", arch_name, info.signature_type);
                    } else {
                        eprintln!(
                            "⚠ {}: Not code signed (acceptable for development)",
                            arch_name
                        );
                    }
                }
                Err(e) => {
                    eprintln!("⚠ {}: Could not check code signing: {}", arch_name, e);
                }
            }
        }
    }

    /// Test that both binaries don't crash on basic commands
    #[test]
    fn test_no_crash_both_architectures() {
        let config = get_config();
        let test_args = vec![vec!["--version"], vec!["--help"]];

        let binaries = vec![
            (&config.intel_binary, "Intel"),
            (&config.arm_binary, "Apple Silicon"),
        ];

        for (binary, arch_name) in binaries {
            if !config.binary_exists(binary) {
                continue;
            }

            for args in &test_args {
                let output = Command::new(binary).args(args).output();

                match output {
                    Ok(output) => {
                        let status = output.status.code().unwrap_or(-1);
                        assert!(
                            status >= 0 && status <= 1,
                            "{}: Command {:?} crashed with exit code {:?}",
                            arch_name,
                            args,
                            status
                        );
                    }
                    Err(e) => {
                        panic!("{}: Failed to execute {:?}: {}", arch_name, args, e);
                    }
                }
            }

            println!("✓ {} basic commands don't crash", arch_name);
        }
    }

    /// Test binary sizes are reasonable
    #[test]
    fn test_binary_sizes_both_architectures() {
        let config = get_config();
        let binaries = vec![
            (&config.intel_binary, "Intel"),
            (&config.arm_binary, "Apple Silicon"),
        ];

        for (binary, arch_name) in binaries {
            if !config.binary_exists(binary) {
                continue;
            }

            let metadata = fs::metadata(binary)
                .expect(&format!("Failed to read {} binary metadata", arch_name));

            let size_bytes = metadata.len();
            let size_mb = size_bytes as f64 / (1024.0 * 1024.0);

            println!("✓ {} binary size: {:.2} MB", arch_name, size_mb);

            assert!(
                size_mb >= 1.0 && size_mb <= 200.0,
                "{}: Binary size {} MB is outside expected range [1, 200] MB",
                arch_name,
                size_mb
            );
        }
    }
}

// ============================================================================
// Test Suite: Architecture-Specific Tests
// ============================================================================

#[cfg(test)]
mod intel_specific_tests {
    use super::*;

    #[test]
    fn test_intel_binary_format() {
        let config = TestConfig::load();

        if !config.binary_exists(&config.intel_binary) {
            eprintln!("⚠ Skipping: Intel binary not found");
            return;
        }

        let output = Command::new("file")
            .arg(&config.intel_binary)
            .output()
            .expect("Failed to run file command");

        let file_output = String::from_utf8_lossy(&output.stdout);

        assert!(
            file_output.contains("Mach-O 64-bit executable") && file_output.contains("x86_64"),
            "Intel binary format unexpected: {}",
            file_output
        );

        println!("✓ Intel binary format: {}", file_output.trim());
    }
}

#[cfg(test)]
mod arm_specific_tests {
    use super::*;

    #[test]
    fn test_arm_binary_format() {
        let config = TestConfig::load();

        if !config.binary_exists(&config.arm_binary) {
            eprintln!("⚠ Skipping: ARM binary not found");
            return;
        }

        let output = Command::new("file")
            .arg(&config.arm_binary)
            .output()
            .expect("Failed to run file command");

        let file_output = String::from_utf8_lossy(&output.stdout);

        assert!(
            file_output.contains("Mach-O 64-bit executable")
                && (file_output.contains("arm64") || file_output.contains("aarch64")),
            "ARM binary format unexpected: {}",
            file_output
        );

        println!("✓ ARM binary format: {}", file_output.trim());
    }
}

// ============================================================================
// Test Suite: Cross-Architecture Compatibility
// ============================================================================

#[cfg(test)]
mod cross_arch_compatibility {
    use super::*;

    /// Test that both binaries produce the same version output
    #[test]
    fn test_consistent_version_output() {
        let config = TestConfig::load();

        let intel_exists = config.binary_exists(&config.intel_binary);
        let arm_exists = config.binary_exists(&config.arm_binary);

        if !intel_exists || !arm_exists {
            eprintln!("⚠ Skipping: Need both binaries");
            return;
        }

        let intel_version =
            run_command(&config.intel_binary, &["--version"]).expect("Intel --version failed");

        let arm_version =
            run_command(&config.arm_binary, &["--version"]).expect("ARM --version failed");

        // Version output should be identical
        assert_eq!(
            intel_version, arm_version,
            "Version output differs between architectures:\nIntel: {}\nARM: {}",
            intel_version, arm_version
        );

        println!("✓ Version output consistent across architectures");
    }

    /// Test that both binaries have similar dependencies
    #[test]
    fn test_similar_dependencies() {
        let config = TestConfig::load();

        let intel_exists = config.binary_exists(&config.intel_binary);
        let arm_exists = config.binary_exists(&config.arm_binary);

        if !intel_exists || !arm_exists {
            eprintln!("⚠ Skipping: Need both binaries");
            return;
        }

        let intel_deps =
            get_dependencies(&config.intel_binary).expect("Failed to get Intel dependencies");
        let arm_deps =
            get_dependencies(&config.arm_binary).expect("Failed to get ARM dependencies");

        // Both should link to CoreFoundation
        let intel_has_cf = intel_deps.contains("CoreFoundation");
        let arm_has_cf = arm_deps.contains("CoreFoundation");

        assert!(
            intel_has_cf && arm_has_cf,
            "Both architectures should link to CoreFoundation\nIntel: {}\nARM: {}",
            intel_has_cf,
            arm_has_cf
        );

        println!("✓ Both architectures link to CoreFoundation");
    }
}

// ============================================================================
// Test Suite: Execution on Current Architecture
// ============================================================================

#[cfg(test)]
mod current_architecture_execution {
    use super::*;

    /// Get the binary for the current architecture
    fn get_current_binary() -> PathBuf {
        let config = TestConfig::load();

        if cfg!(target_arch = "x86_64") {
            config.intel_binary
        } else if cfg!(target_arch = "aarch64") {
            config.arm_binary
        } else {
            panic!("Unknown architecture");
        }
    }

    /// Test that the current architecture binary can execute test commands
    #[test]
    fn test_current_architecture_execution() {
        let binary = get_current_binary();

        if !binary.exists() {
            eprintln!(
                "⚠ Skipping: Current architecture binary not found at {:?}",
                binary
            );
            return;
        }

        // Test execution
        let output = run_command(&binary, &["--version"]).expect("Failed to execute binary");

        assert!(
            output.contains("ltmatrix"),
            "Output doesn't contain 'ltmatrix': {}",
            output
        );

        println!("✓ Current architecture binary executes successfully");
    }

    /// Test ad-hoc signing on current architecture binary
    #[test]
    fn test_adhoc_signing_current_binary() {
        let binary = get_current_binary();

        if !binary.exists() {
            eprintln!("⚠ Skipping: Current architecture binary not found");
            return;
        }

        // Apply ad-hoc signing
        let sign_output = Command::new("codesign")
            .args(&["--force", "--deep", "--sign", "-"])
            .arg(&binary)
            .output();

        match sign_output {
            Ok(output) => {
                if output.status.success() {
                    println!("✓ Ad-hoc signing applied");

                    // Verify the signature
                    let verify_output = Command::new("codesign").arg("-v").arg(&binary).output();

                    if let Ok(verify) = verify_output {
                        if verify.status.success() {
                            println!("✓ Signature verified");
                        } else {
                            eprintln!("⚠ Signature verification failed");
                        }
                    }
                } else {
                    eprintln!("⚠ Ad-hoc signing failed");
                }
            }
            Err(_) => {
                eprintln!("⚠ codesign command not available");
            }
        }
    }
}

// ============================================================================
// Test Suite: Universal Binary (if present)
// ============================================================================

#[cfg(test)]
mod universal_binary_tests {
    use super::*;

    /// Find universal binary if it exists
    fn find_universal_binary() -> Option<PathBuf> {
        let config = TestConfig::load();
        let universal_path = config.project_root.join("release/ltmatrix");

        if universal_path.exists() {
            Some(universal_path)
        } else {
            None
        }
    }

    /// Test that universal binary contains both architectures
    #[test]
    fn test_universal_binary_contains_both() {
        let universal_binary = match find_universal_binary() {
            Some(path) => path,
            None => {
                eprintln!("⚠ Skipping: Universal binary not found");
                return;
            }
        };

        let output = Command::new("lipo")
            .args(&["-info", universal_binary.to_str().unwrap()])
            .output();

        match output {
            Ok(output) => {
                let lipo_output = String::from_utf8_lossy(&output.stdout);
                println!("✓ Universal binary info: {}", lipo_output);

                assert!(
                    lipo_output.contains("x86_64")
                        && (lipo_output.contains("arm64") || lipo_output.contains("aarch64")),
                    "Universal binary should contain both architectures"
                );
            }
            Err(_) => {
                eprintln!("⚠ lipo command not available");
            }
        }
    }

    /// Test that universal binary executes
    #[test]
    fn test_universal_binary_executes() {
        let universal_binary = match find_universal_binary() {
            Some(path) => path,
            None => {
                eprintln!("⚠ Skipping: Universal binary not found");
                return;
            }
        };

        match run_command(&universal_binary, &["--version"]) {
            Ok(output) => {
                assert!(
                    output.contains("ltmatrix"),
                    "Universal binary --version doesn't contain 'ltmatrix': {}",
                    output
                );
                println!("✓ Universal binary executes");
            }
            Err(e) => {
                panic!("Universal binary failed: {}", e);
            }
        }
    }
}
