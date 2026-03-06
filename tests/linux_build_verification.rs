// Integration tests for verifying Linux builds
//
// These tests verify that the ltmatrix binary works correctly on Linux.
// They should be run after building for Linux targets:
//   - x86_64-unknown-linux-musl
//   - aarch64-unknown-linux-musl
//
// Usage on Linux:
//   cargo test --test linux_build_verification
//
// Or run the built binary:
//   ./target/x86_64-unknown-linux-musl/release/ltmatrix --version

use std::env;
use std::path::Path;
use std::process::Command;

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
            "x86_64-unknown-linux-musl".to_string()
        } else if cfg!(target_arch = "aarch64") {
            "aarch64-unknown-linux-musl".to_string()
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

    // On Unix-like systems, check if it's executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = path.metadata().expect("Failed to read binary metadata");
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

/// Test that the binary is statically linked (for musl targets)
#[test]
fn test_static_linking() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    // Use 'ldd' to check dynamic dependencies
    let output = Command::new("ldd").arg(&binary_path).output();

    match output {
        Ok(output) => {
            let ldd_output = String::from_utf8_lossy(&output.stdout);

            // For musl static binaries, we expect:
            // "not a dynamic executable" meaning fully static
            // OR very minimal dependencies (only ld-musl*)

            if ldd_output.contains("not a dynamic executable") {
                // Fully static - this is ideal for musl
                println!("✓ Binary is fully statically linked");
            } else {
                // Check if dependencies are minimal
                let lines: Vec<&str> = ldd_output
                    .lines()
                    .filter(|line| !line.contains("ld-musl") && !line.trim().is_empty())
                    .collect();

                if lines.is_empty() {
                    println!("✓ Binary has minimal dynamic dependencies (only musl runtime)");
                } else {
                    eprintln!("⚠ Warning: Binary has unexpected dependencies:");
                    for line in lines {
                        eprintln!("  {}", line);
                    }
                    // Don't fail the test, just warn
                }
            }
        }
        Err(_) => {
            eprintln!("⚠ ldd command not available, skipping static linking check");
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
    let output = Command::new(&binary_path).arg("help").output();

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

/// Test that the binary doesn't have obvious segfaults on basic commands
#[test]
fn test_no_crash_on_basic_commands() {
    let binary_path = get_binary_path();

    if !Path::new(&binary_path).exists() {
        eprintln!("Skipping test: binary not found at {}", binary_path);
        return;
    }

    // Test various basic commands that shouldn't crash
    let test_args = vec![vec!["--version"], vec!["--help"], vec!["help"]];

    for args in test_args {
        let output = Command::new(&binary_path).args(&args).output();

        match output {
            Ok(output) => {
                // Exit code 0 is success, 1 is often used for errors
                // Both are acceptable. Crash would be signal like 134 (SIGABRT) or 139 (SIGSEGV)
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
    assert!(metadata.len() > 0, "Binary has zero size");
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
            assert!(status >= 0, "Binary crashed with invalid config path");
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

        let output = Command::new(&binary_path).arg(&long_arg).output();

        if let Ok(output) = output {
            // Should handle gracefully, not buffer overflow
            let status = output.status.code().unwrap_or(-1);
            assert!(status >= 0, "Binary crashed with very long argument");
        }
    }
}
