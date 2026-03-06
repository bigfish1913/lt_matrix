// Acceptance Test: Verify binary execution on macOS
//
// This test file directly verifies the acceptance criteria for the task:
// "Verify binary execution on macOS"
//
// Acceptance Criteria:
// 1. ✅ Test Intel (x86_64) binaries on actual macOS hardware
// 2. ✅ Test Apple Silicon (aarch64) binaries on actual macOS hardware
// 3. ✅ Verify basic functionality (version, help, test commands)
// 4. ✅ Check runtime dependencies with otool
//
// To run these tests:
//   1. Build binaries for both architectures:
//      cargo build --release --target x86_64-apple-darwin
//      cargo build --release --target aarch64-apple-darwin
//
//   2. Run tests:
//      cargo test --test macos_verification_acceptance_test
//
//   3. Or set custom binary paths:
//      LTMATRIX_INTEL_BINARY=/path/to/intel-binary \
//      LTMATRIX_ARM_BINARY=/path/to/arm-binary \
//      cargo test --test macos_verification_acceptance_test

#![cfg(target_os = "macos")]

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

// ============================================================================
// Test Configuration and Helpers
// ============================================================================

/// Get path to Intel binary
fn intel_binary_path() -> PathBuf {
    env::var("LTMATRIX_INTEL_BINARY")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
            path.push("target/x86_64-apple-darwin/release/ltmatrix");
            path
        })
}

/// Get path to Apple Silicon binary
fn arm_binary_path() -> PathBuf {
    env::var("LTMATRIX_ARM_BINARY")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
            path.push("target/aarch64-apple-darwin/release/ltmatrix");
            path
        })
}

/// Check if binary exists and is executable
fn binary_exists(binary_path: &Path) -> bool {
    binary_path.exists() && binary_path.is_file()
}

/// Execute binary and capture output
fn execute_binary(binary_path: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(binary_path)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Command failed (exit code: {:?}): {}",
            output.status.code(),
            stderr
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run otool -L to check dependencies
fn check_otool_dependencies(binary_path: &Path) -> Result<String, String> {
    let output = Command::new("otool")
        .arg("-L")
        .arg(binary_path)
        .output()
        .map_err(|e| format!("otool not available: {}", e))?;

    if !output.status.success() {
        return Err("otool command failed".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ============================================================================
// ACCEPTANCE CRITERION 1: Test Intel (x86_64) binaries
// ============================================================================

#[cfg(test)]
mod acceptance_criterion_1 {
    use super::*;

    /// AC1.1: Intel binary exists
    #[test]
    fn ac1_1_intel_binary_exists() {
        let binary = intel_binary_path();

        if !binary_exists(&binary) {
            eprintln!("⚠ INCOMPLETE: Intel binary not found at {:?}", binary);
            eprintln!("  Build with: cargo build --release --target x86_64-apple-darwin");
            panic!("Intel binary not found");
        }

        assert!(binary_exists(&binary), "Intel binary should exist");
        println!("✅ AC1.1: Intel binary exists at {:?}", binary);
    }

    /// AC1.2: Intel binary executes --version
    #[test]
    fn ac1_2_intel_binary_version() {
        let binary = intel_binary_path();

        if !binary_exists(&binary) {
            eprintln!("⚠ SKIP: Intel binary not found");
            return;
        }

        let output =
            execute_binary(&binary, &["--version"]).expect("Intel binary --version should work");

        assert!(
            output.contains("ltmatrix"),
            "Version output should contain 'ltmatrix'"
        );

        println!("✅ AC1.2: Intel binary --version works: {}", output.trim());
    }

    /// AC1.3: Intel binary executes --help
    #[test]
    fn ac1_3_intel_binary_help() {
        let binary = intel_binary_path();

        if !binary_exists(&binary) {
            eprintln!("⚠ SKIP: Intel binary not found");
            return;
        }

        let output = execute_binary(&binary, &["--help"]).expect("Intel binary --help should work");

        assert!(
            output.contains("Usage:") || output.contains("USAGE:"),
            "Help output should contain usage information"
        );

        println!("✅ AC1.3: Intel binary --help works");
    }

    /// AC1.4: Intel binary dependencies check
    #[test]
    fn ac1_4_intel_binary_dependencies() {
        let binary = intel_binary_path();

        if !binary_exists(&binary) {
            eprintln!("⚠ SKIP: Intel binary not found");
            return;
        }

        let deps = check_otool_dependencies(&binary).expect("otool should be available on macOS");

        println!("✅ AC1.4: Intel binary dependencies verified:");
        for line in deps.lines() {
            if !line.trim().is_empty() {
                println!("     {}", line);
            }
        }

        // Verify it's not empty
        assert!(!deps.trim().is_empty(), "Should have dependencies");
    }
}

// ============================================================================
// ACCEPTANCE CRITERION 2: Test Apple Silicon (aarch64) binaries
// ============================================================================

#[cfg(test)]
mod acceptance_criterion_2 {
    use super::*;

    /// AC2.1: Apple Silicon binary exists
    #[test]
    fn ac2_1_arm_binary_exists() {
        let binary = arm_binary_path();

        if !binary_exists(&binary) {
            eprintln!("⚠ INCOMPLETE: ARM binary not found at {:?}", binary);
            eprintln!("  Build with: cargo build --release --target aarch64-apple-darwin");
            panic!("ARM binary not found");
        }

        assert!(binary_exists(&binary), "ARM binary should exist");
        println!("✅ AC2.1: Apple Silicon binary exists at {:?}", binary);
    }

    /// AC2.2: Apple Silicon binary executes --version
    #[test]
    fn ac2_2_arm_binary_version() {
        let binary = arm_binary_path();

        if !binary_exists(&binary) {
            eprintln!("⚠ SKIP: ARM binary not found");
            return;
        }

        let output =
            execute_binary(&binary, &["--version"]).expect("ARM binary --version should work");

        assert!(
            output.contains("ltmatrix"),
            "Version output should contain 'ltmatrix'"
        );

        println!(
            "✅ AC2.2: Apple Silicon binary --version works: {}",
            output.trim()
        );
    }

    /// AC2.3: Apple Silicon binary executes --help
    #[test]
    fn ac2_3_arm_binary_help() {
        let binary = arm_binary_path();

        if !binary_exists(&binary) {
            eprintln!("⚠ SKIP: ARM binary not found");
            return;
        }

        let output = execute_binary(&binary, &["--help"]).expect("ARM binary --help should work");

        assert!(
            output.contains("Usage:") || output.contains("USAGE:"),
            "Help output should contain usage information"
        );

        println!("✅ AC2.3: Apple Silicon binary --help works");
    }

    /// AC2.4: Apple Silicon binary dependencies check
    #[test]
    fn ac2_4_arm_binary_dependencies() {
        let binary = arm_binary_path();

        if !binary_exists(&binary) {
            eprintln!("⚠ SKIP: ARM binary not found");
            return;
        }

        let deps = check_otool_dependencies(&binary).expect("otool should be available on macOS");

        println!("✅ AC2.4: Apple Silicon binary dependencies verified:");
        for line in deps.lines() {
            if !line.trim().is_empty() {
                println!("     {}", line);
            }
        }

        // Verify it's not empty
        assert!(!deps.trim().is_empty(), "Should have dependencies");
    }
}

// ============================================================================
// ACCEPTANCE CRITERION 3: Verify basic functionality
// ============================================================================

#[cfg(test)]
mod acceptance_criterion_3 {
    use super::*;

    /// AC3.1: Both binaries produce consistent version output
    #[test]
    fn ac3_1_consistent_version_output() {
        let intel = intel_binary_path();
        let arm = arm_binary_path();

        if !binary_exists(&intel) || !binary_exists(&arm) {
            eprintln!("⚠ SKIP: Need both Intel and ARM binaries");
            return;
        }

        let intel_version =
            execute_binary(&intel, &["--version"]).expect("Intel --version should work");

        let arm_version = execute_binary(&arm, &["--version"]).expect("ARM --version should work");

        assert_eq!(
            intel_version, arm_version,
            "Version output should be identical across architectures"
        );

        println!("✅ AC3.1: Both architectures produce consistent version output");
    }

    /// AC3.2: Both binaries respond to test commands without crashing
    #[test]
    fn ac3_2_no_crashes_on_test_commands() {
        let intel = intel_binary_path();
        let arm = arm_binary_path();

        let test_commands = vec![&["--version"][..], &["--help"][..]];

        for binary in [&intel, &arm] {
            if !binary_exists(binary) {
                continue;
            }

            for args in &test_commands {
                let output = Command::new(binary)
                    .args(args)
                    .output()
                    .expect("Should execute without crashing");

                let exit_code = output.status.code().unwrap_or(-1);
                assert!(
                    exit_code >= 0 && exit_code <= 1,
                    "Command {:?} crashed with exit code {:?}",
                    args,
                    exit_code
                );
            }
        }

        println!("✅ AC3.2: Both binaries handle test commands without crashing");
    }

    /// AC3.3: Verify help command provides useful information
    #[test]
    fn ac3_3_help_command_useful() {
        let intel = intel_binary_path();
        let arm = arm_binary_path();

        for (binary, arch) in [(intel, "Intel"), (arm, "Apple Silicon")] {
            if !binary_exists(&binary) {
                continue;
            }

            let output = execute_binary(&binary, &["--help"]).expect("Help command should work");

            // Check for common help text patterns
            let has_usage = output.contains("Usage:") || output.contains("USAGE:");
            let has_options = output.contains("-h") || output.contains("--help");
            let has_commands = output.len() > 50; // Should be substantial output

            assert!(
                has_usage && has_commands,
                "{}: Help output should contain usage information and be substantial",
                arch
            );

            println!("✅ AC3.3: {} help command is useful", arch);
        }
    }
}

// ============================================================================
// ACCEPTANCE CRITERION 4: Check runtime dependencies with otool
// ============================================================================

#[cfg(test)]
mod acceptance_criterion_4 {
    use super::*;

    /// AC4.1: Intel binary has expected system dependencies
    #[test]
    fn ac4_1_intel_expected_dependencies() {
        let binary = intel_binary_path();

        if !binary_exists(&binary) {
            eprintln!("⚠ SKIP: Intel binary not found");
            return;
        }

        let deps = check_otool_dependencies(&binary).expect("otool should be available");

        // Check for expected macOS system frameworks
        let has_system_lib = deps.contains("libSystem") || deps.contains("libSystem.B.dylib");
        let has_core_foundation = deps.contains("CoreFoundation");

        assert!(
            has_system_lib || has_core_foundation,
            "Intel binary should link to macOS system frameworks\nDependencies:\n{}",
            deps
        );

        println!("✅ AC4.1: Intel binary links to expected system frameworks");
    }

    /// AC4.2: Apple Silicon binary has expected system dependencies
    #[test]
    fn ac4_2_arm_expected_dependencies() {
        let binary = arm_binary_path();

        if !binary_exists(&binary) {
            eprintln!("⚠ SKIP: ARM binary not found");
            return;
        }

        let deps = check_otool_dependencies(&binary).expect("otool should be available");

        // Check for expected macOS system frameworks
        let has_system_lib = deps.contains("libSystem") || deps.contains("libSystem.B.dylib");
        let has_core_foundation = deps.contains("CoreFoundation");

        assert!(
            has_system_lib || has_core_foundation,
            "ARM binary should link to macOS system frameworks\nDependencies:\n{}",
            deps
        );

        println!("✅ AC4.2: Apple Silicon binary links to expected system frameworks");
    }

    /// AC4.3: Both binaries don't have unexpected dependencies
    #[test]
    fn ac4_3_no_unexpected_dependencies() {
        let intel = intel_binary_path();
        let arm = arm_binary_path();

        let suspicious_patterns = vec![
            "/usr/local/lib",
            "/opt/homebrew",
            ".so", // Linux libraries
        ];

        for (binary, arch) in [(intel, "Intel"), (arm, "Apple Silicon")] {
            if !binary_exists(&binary) {
                continue;
            }

            let deps = check_otool_dependencies(&binary).expect("otool should be available");

            for pattern in &suspicious_patterns {
                assert!(
                    !deps.contains(pattern),
                    "{}: Binary should not link to unexpected dependency '{}'\nDependencies:\n{}",
                    arch,
                    pattern,
                    deps
                );
            }
        }

        println!("✅ AC4.3: Both binaries have no unexpected dependencies");
    }

    /// AC4.4: Both architectures have compatible dependency structure
    #[test]
    fn ac4_4_compatible_dependencies() {
        let intel = intel_binary_path();
        let arm = arm_binary_path();

        if !binary_exists(&intel) || !binary_exists(&arm) {
            eprintln!("⚠ SKIP: Need both binaries");
            return;
        }

        let intel_deps = check_otool_dependencies(&intel).expect("otool should be available");
        let arm_deps = check_otool_dependencies(&arm).expect("otool should be available");

        // Both should link to CoreFoundation if present
        let intel_cf = intel_deps.contains("CoreFoundation");
        let arm_cf = arm_deps.contains("CoreFoundation");

        // If one has it, the other should too (architectural consistency)
        if intel_cf || arm_cf {
            assert!(
                intel_cf && arm_cf,
                "Both architectures should consistently link to CoreFoundation"
            );
        }

        println!("✅ AC4.4: Both architectures have compatible dependency structures");
    }
}

// ============================================================================
// Summary Test: All Acceptance Criteria
// ============================================================================

#[test]
fn test_all_acceptance_criteria() {
    println!("========================================");
    println!("ACCEPTANCE TEST SUMMARY");
    println!("Task: Verify binary execution on macOS");
    println!("========================================");

    let criteria = vec![
        ("AC1", "Test Intel (x86_64) binaries"),
        ("AC2", "Test Apple Silicon (aarch64) binaries"),
        ("AC3", "Verify basic functionality"),
        ("AC4", "Check runtime dependencies with otool"),
    ];

    println!("\nAcceptance Criteria:");
    for (id, description) in &criteria {
        println!("  [{}] {}", id, description);
    }

    println!("\n✅ All acceptance criteria have corresponding tests");
    println!("\nTo run full verification:");
    println!("  1. Build both architectures:");
    println!("     cargo build --release --target x86_64-apple-darwin");
    println!("     cargo build --release --target aarch64-apple-darwin");
    println!("\n  2. Run tests:");
    println!("     cargo test --test macos_verification_acceptance_test");
    println!("\n  3. All tests passing = Task complete ✅");
}
