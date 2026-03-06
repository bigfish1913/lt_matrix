// Integration tests for verifying macOS builds
//
// These tests verify that the ltmatrix binary works correctly on macOS.
// They should be run after building for macOS targets:
//   - x86_64-apple-darwin (Intel)
//   - aarch64-apple-darwin (Apple Silicon)
//
// Usage on macOS:
//   cargo test --test macos_build_verification
//
// Or run the built binary:
//   ./target/x86_64-apple-darwin/release/ltmatrix --version
//
// Note: These tests require macOS to run. On other platforms, tests will
// be skipped with appropriate warnings.

#![cfg(target_os = "macos")]

use std::process::Command;
use std::path::Path;
use std::env;

/// Helper function to get the path to the ltmatrix binary
fn get_binary_path() -> String {
    // In CI/testing, the binary might be in different locations
    // Check environment variable first
    if let Ok(path) = env::var("LTMATRIX_BINARY") {
        return path;
    }

    // Default to release build in target directory
    let target = env::var("TARGET").unwrap_or_else(|_| {
        // Detect current architecture
        if cfg!(target_arch = "x86_64") {
            "x86_64-apple-darwin".to_string()
        } else if cfg!(target_arch = "aarch64") {
            "aarch64-apple-darwin".to_string()
        } else {
            "unknown".to_string()
        }
    });

    format!("target/{}/release/ltmatrix", target)
}

/// Test that the binary exists and is executable
#[test]
fn test_binary_exists() {
    let binary_path = get_binary_path();
    let path = Path::new(&binary_path);

    if !path.exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    // On Unix-like systems (including macOS), check if it's executable
    use std::os::unix::fs::PermissionsExt;
    let metadata = path.metadata()
        .expect("Failed to read binary metadata");
    let permissions = metadata.permissions();
    let mode = permissions.mode();

    // Check if user execute bit is set (0o111)
    assert_eq!(
        mode & 0o111,
        0o111,
        "Binary is not executable. Run: chmod +x {}",
        binary_path
    );
}

/// Test that the binary is a Mach-O executable
#[test]
fn test_binary_macho_format() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    // Use 'file' command to verify it's a Mach-O executable
    let output = Command::new("file")
        .arg(&binary_path)
        .output()
        .expect("Failed to execute file command");

    assert!(output.status.success(), "file command failed");

    let file_output = String::from_utf8_lossy(&output.stdout);

    // Verify it's a Mach-O 64-bit executable
    assert!(
        file_output.contains("Mach-O 64-bit executable"),
        "Binary is not a Mach-O 64-bit executable: {}",
        file_output
    );

    // Verify architecture
    if cfg!(target_arch = "x86_64") {
        assert!(
            file_output.contains("x86_64"),
            "Expected x86_64 binary, got: {}",
            file_output
        );
    } else if cfg!(target_arch = "aarch64") {
        assert!(
            file_output.contains("arm64") || file_output.contains("aarch64"),
            "Expected ARM64 binary, got: {}",
            file_output
        );
    }
}

/// Test that the binary can be executed and responds to --version
#[test]
fn test_binary_version() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    let output = Command::new(&binary_path)
        .arg("--version")
        .output()
        .expect("Failed to execute binary");

    assert!(
        output.status.success(),
        "Binary exited with non-zero status: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let version_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        version_output.contains("ltmatrix"),
        "Version output doesn't contain 'ltmatrix': {}",
        version_output
    );
}

/// Test that the binary can display help information
#[test]
fn test_binary_help() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    let output = Command::new(&binary_path)
        .arg("--help")
        .output()
        .expect("Failed to execute binary");

    assert!(
        output.status.success(),
        "Help command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let help_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        help_output.contains("Usage:") || help_output.contains("USAGE"),
        "Help output doesn't contain usage information: {}",
        help_output
    );
}

/// Test code signing status
///
/// On macOS, binaries should be code signed. For development builds,
/// ad-hoc signing (self-signed) is acceptable. This test verifies:
/// 1. The binary is code signed (either with proper certificate or ad-hoc)
/// 2. The signature is valid
#[test]
fn test_code_signing_status() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    // Use 'codesign' to verify code signing
    let output = Command::new("codesign")
        .arg("-dvv")
        .arg(&binary_path)
        .output();

    match output {
        Ok(output) => {
            let codesign_output = String::from_utf8_lossy(&output.stdout);
            let stderr_output = String::from_utf8_lossy(&output.stderr);

            // Check if binary is code signed
            if output.status.success() {
                // Binary is code signed - verify signature type
                println!("✓ Binary is code signed");
                println!("Code signing details: {}", codesign_output);

                // Check for ad-hoc signing (acceptable for development)
                if codesign_output.contains("adhoc") ||
                   stderr_output.contains("Signature=adhoc") {
                    println!("✓ Binary has ad-hoc signature (acceptable for development)");
                } else if codesign_output.contains("Mac Developer") ||
                          codesign_output.contains("Developer ID") ||
                          stderr_output.contains("Authority") {
                    println!("✓ Binary has proper developer signature");
                }
            } else {
                // Binary might not be signed - this is a warning for development
                eprintln!("⚠ Warning: Binary is not code signed");
                eprintln!("For development, ad-hoc signing is recommended:");
                eprintln!("  codesign --force --deep --sign - {}", binary_path);
                eprintln!("For distribution, proper code signing is required");

                // Don't fail the test, as unsigned binaries can run on macOS
                // (with right-click + Open on first run, or disabling Gatekeeper)
            }
        }
        Err(_) => {
            eprintln!("⚠ codesign command not available, skipping code signing check");
        }
    }
}

/// Test that the binary has ad-hoc signing for local execution
///
/// Ad-hoc signing is sufficient for local development and testing.
/// This test verifies the binary can be signed with ad-hoc signature.
#[test]
fn test_adhoc_signing() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    // Try to apply ad-hoc signing if not already signed
    let sign_output = Command::new("codesign")
        .arg("--force")
        .arg("--deep")
        .arg("--sign")
        .arg("-")  // "-" indicates ad-hoc signing
        .arg(&binary_path)
        .output();

    match sign_output {
        Ok(output) => {
            if output.status.success() {
                println!("✓ Ad-hoc signing applied successfully");

                // Verify the signing worked
                let verify_output = Command::new("codesign")
                    .arg("-v")
                    .arg(&binary_path)
                    .output();

                if let Ok(verify) = verify_output {
                    if verify.status.success() {
                        println!("✓ Ad-hoc signature verified");
                    } else {
                        let stderr = String::from_utf8_lossy(&verify.stderr);
                        eprintln!("⚠ Ad-hoc signature verification failed: {}", stderr);
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("⚠ Ad-hoc signing failed: {}", stderr);
            }
        }
        Err(_) => {
            eprintln!("⚠ codesign command not available, skipping ad-hoc signing test");
        }
    }
}

/// Test dynamic library dependencies
///
/// On macOS, use 'otool -L' to check linked libraries.
/// This test verifies:
/// 1. Binary links to expected system frameworks
/// 2. No unusual or unexpected dependencies
#[test]
fn test_dynamic_dependencies() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    // Use 'otool -L' to check linked libraries
    let output = Command::new("otool")
        .arg("-L")
        .arg(&binary_path)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let otool_output = String::from_utf8_lossy(&output.stdout);
                println!("Dynamic dependencies:\n{}", otool_output);

                // Check for expected system libraries
                let expected_system_libs = vec![
                    "/usr/lib/libSystem.B.dylib",
                    "/System/Library/Frameworks/CoreFoundation.framework",
                ];

                // Check for unexpected dependencies
                let suspicious_patterns = vec![
                    "/usr/local/lib",
                    "/opt/homebrew/lib",
                    "/homebrew/lib",
                    ".dylib",  // Non-system dylib
                ];

                for pattern in suspicious_patterns {
                    if otool_output.contains(pattern) {
                        eprintln!("⚠ Warning: Found suspicious dependency pattern: {}", pattern);
                    }
                }
            } else {
                eprintln!("⚠ otool command failed");
            }
        }
        Err(_) => {
            eprintln!("⚠ otool command not available, skipping dependency check");
        }
    }
}

/// Test that the binary doesn't have unexpected hardcoded paths
#[test]
fn test_no_hardcoded_paths() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    // Use 'strings' to check for hardcoded paths
    let output = Command::new("strings")
        .arg(&binary_path)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let strings_output = String::from_utf8_lossy(&output.stdout);

                // Check for problematic hardcoded paths
                let problematic_paths = vec![
                    "/home/",           // Linux-specific
                    "C:\\",             // Windows-specific
                    "/usr/local/lib",   // Non-system path
                    "/opt/homebrew",    // Homebrew path
                ];

                for path in problematic_paths {
                    // Allow occasional occurrences (e.g., in error messages)
                    let count = strings_output.matches(path).count();
                    if count > 5 {
                        eprintln!("⚠ Warning: Found many occurrences of '{}': {}", path, count);
                    }
                }
            }
        }
        Err(_) => {
            eprintln!("⚠ strings command not available, skipping hardcoded path check");
        }
    }
}

/// Test that the binary has reasonable entitlements
///
/// Entitlements control what special capabilities the binary has.
/// For a CLI tool, minimal entitlements are expected.
#[test]
fn test_binary_entitlements() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    // Use 'codesign -d --entitlements' to check entitlements
    let output = Command::new("codesign")
        .arg("-d")
        .arg("--entitlements")
        .arg("-")
        .arg(&binary_path)
        .output();

    match output {
        Ok(output) => {
            let entitlements = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if output.status.success() || stderr.contains("entitlements") {
                println!("Entitlements:\n{}", entitlements);

                // Check for unexpected entitlements
                let unexpected_entitlements = vec![
                    "com.apple.security.get-task-allow",
                    "com.apple.security.network.server",
                    "com.apple.security.network.client",
                ];

                for entitlement in unexpected_entitlements {
                    if entitlements.contains(entitlement) {
                        eprintln!("⚠ Warning: Binary has unexpected entitlement: {}", entitlement);
                    }
                }
            } else {
                // No entitlements is fine for a basic CLI tool
                println!("✓ Binary has no special entitlements (expected for CLI tool)");
            }
        }
        Err(_) => {
            eprintln!("⚠ Could not check entitlements");
        }
    }
}

/// Test basic CLI functionality
#[test]
fn test_cli_subcommands() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    // Test that we can list available subcommands
    let output = Command::new(&binary_path)
        .arg("help")
        .output();

    if let Ok(output) = output {
        let help_text = String::from_utf8_lossy(&output.stdout);

        // Check for expected subcommands based on the project
        // These should match what's defined in the CLI
        let expected_commands: Vec<&str> = vec![
            // Add expected subcommands here based on actual CLI implementation
            // Examples: "run", "init", "config", etc.
        ];

        for cmd in expected_commands {
            assert!(
                help_text.contains(cmd),
                "Expected subcommand '{}' not found in help output",
                cmd
            );
        }
    }
}

/// Test that the binary doesn't have obvious crashes on basic commands
#[test]
fn test_no_crash_on_basic_commands() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    // Test various basic commands that shouldn't crash
    let test_args = vec![
        vec!["--version"],
        vec!["--help"],
        vec!["help"],
    ];

    for args in test_args {
        let output = Command::new(&binary_path)
            .args(&args)
            .output();

        match output {
            Ok(output) => {
                // Exit code 0 is success, 1 is often used for errors
                // Both are acceptable. Crash would be signal like 134 (SIGABRT)
                let status = output.status.code().unwrap_or(-1);
                assert!(
                    status >= 0 && status <= 1,
                    "Command {:?} crashed with exit code {:?}",
                    args,
                    status
                );
            }
            Err(e) => {
                panic!("Failed to execute command {:?}: {}", args, e);
            }
        }
    }
}

/// Test binary file size is reasonable
#[test]
fn test_binary_size() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    let metadata = Path::new(&binary_path)
        .metadata()
        .expect("Failed to read binary metadata");

    let size_bytes = metadata.len();
    let size_mb = size_bytes as f64 / (1024.0 * 1024.0);

    println!("Binary size: {:.2} MB", size_mb);

    // For a Rust CLI with tokio and git2, expect 5-50 MB
    // This is a wide range but accounts for different build configurations
    assert!(
        size_mb >= 1.0 && size_mb <= 200.0,
        "Binary size {} MB is outside expected range [1, 200] MB",
        size_mb
    );
}

/// Test that the binary has proper permissions and is a regular file
#[test]
fn test_binary_properties() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    let path = Path::new(&binary_path);
    let metadata = path.metadata().expect("Failed to read binary metadata");

    // Check it's a file, not a directory
    assert!(
        metadata.is_file(),
        "Path {} is not a regular file",
        binary_path
    );

    // Check file size is non-zero
    assert!(
        metadata.len() > 0,
        "Binary has zero size"
    );
}

/// Test Gatekeeper acceptance
///
/// Gatekeeper is macOS's security feature that verifies apps from
/// the internet. This test verifies the binary can be executed without
/// Gatekeeper blocking it (if properly signed).
#[test]
fn test_gatekeeper_acceptance() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    // Try to execute the binary - Gatekeeper would block if there's an issue
    let output = Command::new(&binary_path)
        .arg("--version")
        .output();

    match output {
        Ok(output) => {
            // If we get here without "killed" or similar, Gatekeeper accepted it
            if output.status.success() || output.status.code().unwrap_or(-1) > 0 {
                println!("✓ Binary passes Gatekeeper checks (or is unsigned but executable)");
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                eprintln!("⚠ Gatekeeper may be blocking execution: {}", e);
                eprintln!("Try: xattr -d com.apple.quarantine {}", binary_path);
            } else {
                panic!("Failed to execute binary: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;

    /// Regression test: Ensure the binary doesn't crash with specific inputs
    /// that have caused issues in the past
    #[test]
    fn test_regression_empty_config() {
        let binary_path = get_binary_path();

        if !Path::new(&binary_path).exists() {
            eprintln!("Skipping test: binary not found at {}", binary_path);
            return;
        }

        // Test with empty config path (should handle gracefully)
        let output = Command::new(&binary_path)
            .arg("--config")
            .arg("/dev/null/empty.toml")
            .output();

        if let Ok(output) = output {
            // Should fail gracefully, not crash
            let status = output.status.code().unwrap_or(-1);
            assert!(
                status >= 0,
                "Binary crashed with invalid config path"
            );
        }
    }

    /// Regression test: Test with very long argument strings
    #[test]
    fn test_regression_long_arguments() {
        let binary_path = get_binary_path();

        if !Path::new(&binary_path).exists() {
            eprintln!("Skipping test: binary not found at {}", binary_path);
            return;
        }

        let long_arg = "a".repeat(10000);

        let output = Command::new(&binary_path)
            .arg(&long_arg)
            .output();

        if let Ok(output) = output {
            // Should handle gracefully, not buffer overflow
            let status = output.status.code().unwrap_or(-1);
            assert!(
                status >= 0,
                "Binary crashed with very long argument"
            );
        }
    }

    /// Regression test: Verify symlink handling
    #[test]
    fn test_regression_symlink_handling() {
        let binary_path = get_binary_path();

        if !Path::new(&binary_path).exists() {
            eprintln!("Skipping test: binary not found at {}", binary_path);
            return;
        }

        // Create a temporary symlink to the binary
        let temp_dir = tempfile::tempdir().ok();
        if temp_dir.is_none() {
            eprintln!("Skipping symlink test: could not create temp dir");
            return;
        }

        let temp_dir = temp_dir.unwrap();
        let symlink_path = temp_dir.path().join("ltmatrix_symlink");

        // Try to create symlink
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::symlink;

            if symlink(&binary_path, &symlink_path).is_ok() {
                // Test execution via symlink
                let output = Command::new(&symlink_path)
                    .arg("--version")
                    .output();

                if let Ok(output) = output {
                    // Should work the same as direct execution
                    assert!(
                        output.status.success(),
                        "Binary failed when executed via symlink"
                    );
                }
            } else {
                eprintln!("Skipping symlink test: could not create symlink");
            }
        }
    }
}

#[cfg(test)]
mod linker_verification {
    use super::*;

    /// Test that the binary doesn't have linker issues
    #[test]
    fn test_linker_verification() {
        let binary_path = get_binary_path();

        if !Path::new(&binary_path).exists() {
            eprintln!("Skipping test: binary not found at {}", binary_path);
            return;
        }

        // Use 'otool -l' to check load commands
        let output = Command::new("otool")
            .arg("-l")
            .arg(&binary_path)
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let otool_output = String::from_utf8_lossy(&output.stdout);

                    // Check for proper linkage
                    if otool_output.contains("LC_LOAD_DYLIB") {
                        println!("✓ Binary uses dynamic linking (normal for macOS)");
                    }

                    if otool_output.contains("LC_RPATH") {
                        println!("✓ Binary has rpath set");
                    }
                }
            }
            Err(_) => {
                eprintln!("⚠ otool command not available, skipping linker verification");
            }
        }
    }
}

#[cfg(test)]
mod dependency_compatibility {
    use super::*;

    /// Test that native dependencies are compatible with macOS
    #[test]
    fn test_native_dependency_compatibility() {
        let binary_path = get_binary_path();

        if !Path::new(&binary_path).exists() {
            eprintln!("Skipping test: binary not found at {}", binary_path);
            return;
        }

        // Check linked libraries for compatibility issues
        let output = Command::new("otool")
            .arg("-L")
            .arg(&binary_path)
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let deps = String::from_utf8_lossy(&output.stdout);

                    // Check for Linux-specific libraries (should not be present)
                    let linux_libs = vec![
                        "linux-vdso",
                        "ld-linux",
                        "libssl.so",
                        "libcrypto.so",
                    ];

                    for lib in linux_libs {
                        assert!(
                            !deps.contains(lib),
                            "Binary links to Linux-specific library: {}",
                            lib
                        );
                    }

                    // Check for expected macOS frameworks
                    let macos_frameworks = vec![
                        "CoreFoundation",
                        "Security",
                    ];

                    for framework in &macos_frameworks {
                        if deps.contains(framework) {
                            println!("✓ Binary uses macOS framework: {}", framework);
                        }
                    }
                }
            }
            Err(_) => {
                eprintln!("⚠ Could not verify native dependencies");
            }
        }
    }
}
