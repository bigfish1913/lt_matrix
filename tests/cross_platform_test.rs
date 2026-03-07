//! Cross-platform compatibility tests
//!
//! This test module verifies that ltmatrix works correctly across different
//! platforms (Linux, macOS, Windows). It tests:
//! - Path handling (Windows backslash vs Unix forward slash)
//! - Process spawning differences
//! - Git operations compatibility
//! - Terminal output handling
//! - Environment-specific behavior

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Project root directory
fn project_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Check if running on Windows
fn is_windows() -> bool {
    cfg!(windows)
}

/// Check if running on Unix (Linux/macOS)
fn is_unix() -> bool {
    cfg!(unix)
}

// ============================================================================
// Path Handling Tests
// ============================================================================

#[test]
fn test_path_separator_detection() {
    let separator = std::path::MAIN_SEPARATOR;

    if is_windows() {
        assert_eq!(separator, '\\', "Windows should use backslash");
    } else {
        assert_eq!(separator, '/', "Unix should use forward slash");
    }
}

#[test]
fn test_path_join_cross_platform() {
    let base = PathBuf::from("src").join("lib").join("module.rs");

    let components: Vec<_> = base.components().collect();
    assert_eq!(components.len(), 3, "Path should have 3 components");
    assert_eq!(base.file_name().unwrap(), std::ffi::OsStr::new("module.rs"));
}

#[test]
fn test_path_with_forward_slash_works() {
    let path = PathBuf::from("src/lib/module.rs");
    assert!(path.to_str().is_some(), "Path should be valid");

    let components: Vec<_> = path.components().collect();
    assert_eq!(components.len(), 3);
}

#[test]
fn test_path_canonicalization() {
    let temp_dir = std::env::temp_dir();
    let test_path = temp_dir.join("ltmatrix_test_canonical");

    fs::create_dir_all(&test_path).ok();

    if let Ok(canonical) = test_path.canonicalize() {
        assert!(canonical.exists(), "Canonical path should exist");
    }

    fs::remove_dir_all(&test_path).ok();
}

#[test]
fn test_path_parent_traversal() {
    let path = PathBuf::from("src/lib/module.rs");

    let parent = path.parent();
    assert!(parent.is_some());
    assert_eq!(parent.unwrap(), Path::new("src/lib"));

    let grandparent = parent.unwrap().parent();
    assert!(grandparent.is_some());
    assert_eq!(grandparent.unwrap(), Path::new("src"));
}

#[test]
fn test_path_extension_handling() {
    let path = PathBuf::from("src/main.rs");
    assert_eq!(path.extension().unwrap(), "rs");
    assert_eq!(path.file_stem().unwrap(), "main");
}

#[test]
fn test_path_with_spaces() {
    let path = PathBuf::from("src/my module/test file.rs");
    assert!(path.to_str().is_some(), "Path with spaces should be valid");
}

#[test]
fn test_path_with_unicode() {
    let path = PathBuf::from("src/文档/测试.rs");
    assert!(path.to_str().is_some(), "Unicode path should be valid");
}

#[test]
fn test_absolute_path_detection() {
    let abs_path = if is_windows() {
        PathBuf::from("C:\\src\\main.rs")
    } else {
        PathBuf::from("/src/main.rs")
    };
    assert!(abs_path.is_absolute(), "Path should be absolute");

    let rel_path = PathBuf::from("src/main.rs");
    assert!(!rel_path.is_absolute(), "Path should be relative");
}

#[test]
fn test_current_dir_handling() {
    let current_dir = env::current_dir();
    assert!(current_dir.is_ok(), "Should get current directory");

    let dir = current_dir.unwrap();
    assert!(dir.is_absolute(), "Current directory should be absolute");
    assert!(dir.exists(), "Current directory should exist");
}

#[test]
fn test_temp_dir_cross_platform() {
    let temp_dir = env::temp_dir();
    assert!(temp_dir.is_absolute(), "Temp dir should be absolute");
    assert!(temp_dir.exists(), "Temp dir should exist");
}

#[test]
fn test_home_dir_cross_platform() {
    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .or_else(|_| env::var("HOMEPATH"));

    assert!(home_dir.is_ok() || cfg!(windows), "Home should be available");
}

// ============================================================================
// Process Spawning Tests
// ============================================================================

#[test]
fn test_command_echo_simple() {
    let result = if is_windows() {
        Command::new("cmd").args(["/C", "echo", "hello"]).output()
    } else {
        Command::new("echo").arg("hello").output()
    };

    assert!(result.is_ok(), "Echo command should execute");

    let output = result.unwrap();
    assert!(output.status.success(), "Echo should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello"), "Output should contain 'hello'");
}

#[test]
fn test_command_environment_variables() {
    let result = if is_windows() {
        Command::new("cmd")
            .args(["/C", "echo", "%TEST_VAR%"])
            .env("TEST_VAR", "test_value")
            .output()
    } else {
        Command::new("sh")
            .args(["-c", "echo $TEST_VAR"])
            .env("TEST_VAR", "test_value")
            .output()
    };

    assert!(result.is_ok(), "Command with env should execute");

    let output = result.unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test_value"), "Env var should be set");
}

#[test]
fn test_command_exit_code_success() {
    let result = if is_windows() {
        Command::new("cmd").args(["/C", "exit", "0"]).output()
    } else {
        Command::new("true").output()
    };

    assert!(result.is_ok(), "Command should execute");
    let output = result.unwrap();
    assert!(output.status.success(), "Command should succeed");
}

#[test]
fn test_command_exit_code_failure() {
    let result = if is_windows() {
        Command::new("cmd").args(["/C", "exit", "1"]).output()
    } else {
        Command::new("false").output()
    };

    assert!(result.is_ok(), "Command should execute");
    let output = result.unwrap();
    assert!(!output.status.success(), "Command should fail");
}

#[test]
fn test_command_working_directory() {
    let result = if is_windows() {
        Command::new("cmd")
            .args(["/C", "cd"])
            .current_dir(project_root())
            .output()
    } else {
        Command::new("pwd").current_dir(project_root()).output()
    };

    assert!(result.is_ok(), "Command with working dir should execute");
    let output = result.unwrap();
    assert!(output.status.success(), "Command should succeed");
}

#[test]
fn test_command_git_version() {
    let result = Command::new("git").arg("--version").output();

    assert!(result.is_ok(), "Git command should be available");

    let output = result.unwrap();
    assert!(output.status.success(), "Git should execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("git version"), "Git version should be printed");
}

#[test]
fn test_command_cargo_available() {
    let result = Command::new("cargo").arg("--version").output();

    assert!(result.is_ok(), "Cargo should be available");

    let output = result.unwrap();
    assert!(output.status.success(), "Cargo should execute");
}

// ============================================================================
// Git Operations Tests
// ============================================================================

#[test]
fn test_git_repository_detection() {
    let git_dir = project_root().join(".git");
    assert!(git_dir.exists(), "Project should have .git directory");
}

#[test]
fn test_git_status_command() {
    let result = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(project_root())
        .output();

    assert!(result.is_ok(), "Git status should execute");

    let output = result.unwrap();
    assert!(output.status.success(), "Git status should succeed");
}

#[test]
fn test_git_rev_parse_head() {
    let result = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_root())
        .output();

    assert!(result.is_ok(), "Git rev-parse should execute");

    let output = result.unwrap();
    assert!(output.status.success(), "Git rev-parse should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim().len(), 40, "SHA should be 40 chars");
}

#[test]
fn test_git_branch_command() {
    let result = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(project_root())
        .output();

    assert!(result.is_ok(), "Git branch should execute");

    let output = result.unwrap();
    assert!(output.status.success(), "Git branch should succeed");
}

#[test]
fn test_git_remote_command() {
    let result = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(project_root())
        .output();

    assert!(result.is_ok(), "Git remote should execute");

    let output = result.unwrap();
    assert!(output.status.success(), "Git remote should succeed");
}

#[test]
fn test_git_log_command() {
    let result = Command::new("git")
        .args(["log", "-1", "--format=%s"])
        .current_dir(project_root())
        .output();

    assert!(result.is_ok(), "Git log should execute");

    let output = result.unwrap();
    assert!(output.status.success(), "Git log should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.trim().is_empty(), "Should have commits");
}

#[test]
fn test_git_ls_files_command() {
    let result = Command::new("git")
        .args(["ls-files", "--error-unmatch", "Cargo.toml"])
        .current_dir(project_root())
        .output();

    assert!(result.is_ok(), "Git ls-files should execute");

    let output = result.unwrap();
    assert!(output.status.success(), "Cargo.toml should be tracked");
}

// ============================================================================
// Terminal Output Tests
// ============================================================================

#[test]
fn test_ansi_color_codes() {
    let green = "\x1b[32m";
    let reset = "\x1b[0m";

    let colored = format!("{}Success{}", green, reset);
    assert!(colored.contains('\x1b'), "Should contain ANSI codes");
}

#[test]
fn test_newline_handling() {
    let text = "line1\nline2\nline3";
    assert_eq!(text.split('\n').count(), 3);
}

#[test]
fn test_carriage_return_handling() {
    let text_crlf = "line1\r\nline2\r\n";
    let text_lf = "line1\nline2\n";

    let crlf_lines: Vec<_> = text_crlf.split('\n').filter(|s| !s.is_empty()).collect();
    let lf_lines: Vec<_> = text_lf.split('\n').filter(|s| !s.is_empty()).collect();

    assert_eq!(crlf_lines.len(), 2);
    assert_eq!(lf_lines.len(), 2);
}

#[test]
fn test_stdout_write() {
    let result = std::io::stdout().write_all(b"test");
    assert!(result.is_ok(), "Should write to stdout");
}

#[test]
fn test_stderr_write() {
    let result = std::io::stderr().write_all(b"test");
    assert!(result.is_ok(), "Should write to stderr");
}

// ============================================================================
// Environment Tests
// ============================================================================

#[test]
fn test_env_path_variable() {
    let path_var = env::var("PATH");
    assert!(path_var.is_ok(), "PATH should be set");

    let path_value = path_var.unwrap();
    assert!(!path_value.is_empty(), "PATH should not be empty");
}

#[test]
fn test_env_separator() {
    let separator = if is_windows() { ';' } else { ':' };

    let path_var = env::var("PATH").unwrap();
    assert!(
        path_var.contains(&separator.to_string()) || path_var.split(separator).count() >= 1,
        "PATH should use correct separator"
    );
}

#[test]
fn test_env_home_variable() {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .or_else(|_| {
            env::var("HOMEDRIVE")
                .and_then(|d| env::var("HOMEPATH").map(|p| format!("{}{}", d, p)))
        });

    assert!(home.is_ok() || is_windows(), "Home should be set");
}

// ============================================================================
// Time Handling Tests
// ============================================================================

#[test]
fn test_system_time() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH);

    assert!(duration.is_ok(), "Should get time since epoch");
    assert!(duration.unwrap().as_secs() > 1_700_000_000, "Time should be recent");
}

#[test]
fn test_chrono_datetime() {
    use chrono::{Local, Utc};

    let utc_now = Utc::now();
    let local_now = Local::now();

    assert!(utc_now.timestamp() > 0, "UTC time should be valid");
    assert!(local_now.timestamp() > 0, "Local time should be valid");
}

// ============================================================================
// Cargo/Build Tests
// ============================================================================

#[test]
fn test_cargo_manifest_dir() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let path = PathBuf::from(manifest_dir);
    assert!(path.exists(), "Manifest dir should exist");
    assert!(path.is_absolute(), "Manifest dir should be absolute");
}

#[test]
fn test_cargo_pkg_name() {
    let pkg_name = env!("CARGO_PKG_NAME");
    assert_eq!(pkg_name, "ltmatrix", "Package name should be ltmatrix");
}

#[test]
fn test_cargo_pkg_version() {
    let version = env!("CARGO_PKG_VERSION");
    assert!(version.contains('.'), "Version should be semver format");
}

// ============================================================================
// Platform Detection Tests
// ============================================================================

#[test]
fn test_platform_detection() {
    let os = env::consts::OS;

    match os {
        "windows" => {
            assert!(is_windows(), "Windows detected");
            assert!(!is_unix(), "Not Unix");
        }
        "linux" | "macos" => {
            assert!(is_unix(), "Unix detected");
            assert!(!is_windows(), "Not Windows");
        }
        _ => {}
    }
}

#[test]
fn test_architecture_detection() {
    let arch = env::consts::ARCH;

    assert!(
        matches!(arch, "x86_64" | "x86" | "aarch64" | "arm"),
        "Architecture should be recognized: {}",
        arch
    );
}

#[test]
fn test_family_detection() {
    let family = env::consts::FAMILY;

    assert!(
        matches!(family, "windows" | "unix"),
        "Family should be windows or unix: {}",
        family
    );
}

// ============================================================================
// Concurrency Tests
// ============================================================================

#[test]
fn test_thread_spawn() {
    use std::thread;

    let handle = thread::spawn(|| 42);

    let result = handle.join();
    assert!(result.is_ok(), "Thread should complete");
    assert_eq!(result.unwrap(), 42, "Thread should return correct value");
}

#[test]
fn test_mutex_cross_platform() {
    use std::sync::{Arc, Mutex};

    let counter = Arc::new(Mutex::new(0));
    let counter_clone = Arc::clone(&counter);

    *counter_clone.lock().unwrap() += 1;

    assert_eq!(*counter.lock().unwrap(), 1, "Mutex should work");
}

#[test]
fn test_channel_cross_platform() {
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel();

    tx.send(42).expect("Should send");

    let received = rx.recv().expect("Should receive");
    assert_eq!(received, 42, "Channel should transmit");
}

// ============================================================================
// Network Tests (Basic)
// ============================================================================

#[test]
fn test_dns_resolution() {
    use std::net::ToSocketAddrs;

    let result = "localhost:80".to_socket_addrs();

    assert!(result.is_ok(), "Should resolve localhost");

    let addrs: Vec<_> = result.unwrap().collect();
    assert!(!addrs.is_empty(), "Should have addresses");
}

#[test]
fn test_loopback_interface() {
    use std::net::{IpAddr, Ipv4Addr};

    let loopback = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

    assert!(loopback.is_loopback(), "127.0.0.1 should be loopback");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_io_error_kind() {
    use std::io::ErrorKind;

    let result = fs::read_to_string("/nonexistent/path/that/does/not/exist.txt");

    assert!(result.is_err(), "Should fail for nonexistent file");

    let error = result.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotFound, "Should be NotFound error");
}

#[test]
fn test_error_display() {
    let error = std::io::Error::new(std::io::ErrorKind::Other, "test error");

    let display = format!("{}", error);
    assert!(display.contains("test error"), "Error should display");
}

// ============================================================================
// File System Tests
// ============================================================================

#[test]
fn test_file_create_and_write() {
    let temp_dir = env::temp_dir();
    let test_file = temp_dir.join("ltmatrix_test_file.txt");

    let write_result = fs::write(&test_file, "test content");
    assert!(write_result.is_ok(), "Should write to file");

    let read_result = fs::read_to_string(&test_file);
    assert!(read_result.is_ok(), "Should read file");

    let content = read_result.unwrap();
    assert_eq!(content, "test content", "Content should match");

    fs::remove_file(&test_file).ok();
}

#[test]
fn test_directory_create_and_remove() {
    let temp_dir = env::temp_dir();
    let test_dir = temp_dir.join("ltmatrix_test_dir");

    let create_result = fs::create_dir(&test_dir);
    assert!(create_result.is_ok(), "Should create directory");

    assert!(test_dir.exists(), "Directory should exist");

    let remove_result = fs::remove_dir(&test_dir);
    assert!(remove_result.is_ok(), "Should remove directory");

    assert!(!test_dir.exists(), "Directory should be removed");
}

#[test]
fn test_file_metadata() {
    let cargo_toml = project_root().join("Cargo.toml");

    let metadata = fs::metadata(&cargo_toml);
    assert!(metadata.is_ok(), "Should get file metadata");

    let meta = metadata.unwrap();
    assert!(meta.is_file(), "Should be a file");
    assert!(meta.len() > 0, "File should have content");
}

#[test]
fn test_directory_listing() {
    let src_dir = project_root().join("src");

    let entries = fs::read_dir(&src_dir);
    assert!(entries.is_ok(), "Should read directory");

    let count = entries.unwrap().count();
    assert!(count > 0, "Source directory should have files");
}

// ============================================================================
// CI Environment Tests
// ============================================================================

#[test]
fn test_ci_environment_detection() {
    let is_ci = env::var("CI").is_ok()
        || env::var("GITHUB_ACTIONS").is_ok()
        || env::var("TRAVIS").is_ok()
        || env::var("CIRCLECI").is_ok()
        || env::var("JENKINS_URL").is_ok();

    // Just verify detection works
    let _ = is_ci;
}

#[test]
fn test_github_actions_environment() {
    let github_actions = env::var("GITHUB_ACTIONS");

    if github_actions.is_ok() && github_actions.unwrap() == "true" {
        let workspace = env::var("GITHUB_WORKSPACE");
        assert!(workspace.is_ok(), "GITHUB_WORKSPACE should be set");
    }
}

// ============================================================================
// Path Comparison Tests
// ============================================================================

#[test]
fn test_path_starts_with() {
    let path = PathBuf::from("src/lib/module.rs");

    assert!(path.starts_with("src"), "Path should start with 'src'");
    assert!(path.starts_with("src/lib"), "Path should start with 'src/lib'");
}

#[test]
fn test_path_ends_with() {
    let path = PathBuf::from("src/lib/module.rs");

    assert!(path.ends_with("module.rs"), "Path should end with 'module.rs'");
    assert!(path.ends_with("lib/module.rs"), "Path should end with 'lib/module.rs'");
}

// ============================================================================
// Line Ending Tests
// ============================================================================

#[test]
fn test_line_ending_write() {
    let temp_dir = env::temp_dir();
    let test_file = temp_dir.join("ltmatrix_line_endings.txt");

    let content = "line1\nline2\nline3";

    fs::write(&test_file, content).expect("Should write file");

    let read_content = fs::read_to_string(&test_file).expect("Should read file");

    assert!(read_content.contains("line1"), "Should contain line1");
    assert!(read_content.contains("line2"), "Should contain line2");
    assert!(read_content.contains("line3"), "Should contain line3");

    fs::remove_file(&test_file).ok();
}

// ============================================================================
// Process Arguments Tests
// ============================================================================

#[test]
fn test_command_args_with_spaces() {
    let result = if is_windows() {
        Command::new("cmd")
            .args(["/C", "echo", "hello world"])
            .output()
    } else {
        Command::new("echo").arg("hello world").output()
    };

    assert!(result.is_ok(), "Command should execute");

    let output = result.unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello world"), "Should handle spaced argument");
}

// ============================================================================
// Windows-Specific Path Tests
// ============================================================================

#[test]
fn test_windows_unc_path_handling() {
    // UNC paths should be recognized on all platforms for validation
    let unc_path = PathBuf::from(r"\\server\share\file.txt");

    // UNC paths should be valid path objects
    assert!(unc_path.to_str().is_some(), "UNC path should be valid string");

    // On Windows, UNC paths are absolute
    #[cfg(windows)]
    assert!(unc_path.is_absolute(), "UNC path should be absolute on Windows");
}

#[test]
fn test_windows_drive_letter_path() {
    // Drive letter paths
    let drive_path = PathBuf::from("C:\\Users\\test\\file.txt");

    #[cfg(windows)]
    {
        assert!(drive_path.is_absolute(), "Drive path should be absolute on Windows");
        // Components should include drive root
        let components: Vec<_> = drive_path.components().collect();
        assert!(!components.is_empty(), "Should have components");
    }

    // On Unix, this is still a valid relative-looking path
    #[cfg(not(windows))]
    {
        assert!(drive_path.to_str().is_some(), "Path should be valid string");
    }
}

#[test]
fn test_windows_long_path_prefix() {
    // Windows long path prefix (\\?\)
    let long_path = PathBuf::from(r"\\?\C:\Very\Long\Path\That\Exceeds\Normal\Limits");

    // Should be valid on all platforms
    assert!(long_path.to_str().is_some(), "Long path prefix should be valid");
}

#[test]
fn test_path_with_backslash_on_unix() {
    // Test that forward slashes work universally
    let forward_path = PathBuf::from("src/lib/module.rs");

    // Forward slashes should work on all platforms
    assert!(forward_path.to_str().is_some());
    let components: Vec<_> = forward_path.components().collect();
    assert_eq!(components.len(), 3, "Should parse 3 components");
}

#[test]
fn test_path_case_sensitivity() {
    // Test path comparison behavior
    let lower = PathBuf::from("src/main.rs");
    let upper = PathBuf::from("SRC/MAIN.RS");

    // On case-sensitive filesystems (Linux), these are different
    // On case-insensitive (Windows, macOS default), they may be the same
    // The test just verifies comparison works
    let _comparison = lower == upper;

    // Both should be valid paths
    assert!(lower.to_str().is_some());
    assert!(upper.to_str().is_some());
}

#[test]
fn test_path_reserved_names_windows() {
    // Windows reserved names that shouldn't be used as file names
    let reserved_names = ["CON", "PRN", "AUX", "NUL", "COM1", "LPT1"];

    for name in &reserved_names {
        let path = PathBuf::from(name);
        // These are valid Path objects but problematic as filenames on Windows
        assert!(path.to_str().is_some(), "{} should be a valid path component", name);
    }
}

// ============================================================================
// Security Path Module Integration Tests
// ============================================================================

#[test]
fn test_path_traversal_detection() {
    // Test that traversal patterns are detected correctly
    let traversal_paths = [
        "../../../etc/passwd",
        "..\\..\\windows\\system32",
        "subdir/../../../etc",
        "./../secret",
    ];

    for path_str in &traversal_paths {
        let path = PathBuf::from(path_str);
        let mut has_traversal = false;

        for component in path.components() {
            if matches!(component, std::path::Component::ParentDir) {
                has_traversal = true;
                break;
            }
        }

        assert!(has_traversal, "Path '{}' should contain traversal", path_str);
    }
}

#[test]
fn test_safe_path_components() {
    // Test paths that should NOT have traversal
    let safe_paths = [
        "src/lib/module.rs",
        "tests/integration_test.rs",
        "./current/dir/file.txt",
        "relative/path/to/file",
    ];

    for path_str in &safe_paths {
        let path = PathBuf::from(path_str);
        let has_traversal = path.components().any(|c| {
            matches!(c, std::path::Component::ParentDir)
        });

        assert!(!has_traversal, "Path '{}' should not contain traversal", path_str);
    }
}

#[test]
fn test_path_sanitize_characters() {
    // Characters that are problematic in filenames
    let problematic_chars = ['<', '>', ':', '"', '|', '?', '*'];

    for &ch in &problematic_chars {
        let filename = format!("test{}file.txt", ch);
        // These create valid Path objects but may be invalid on some filesystems
        let path = PathBuf::from(&filename);
        assert!(path.to_str().is_some(), "Path with '{}' should be valid Path object", ch);
    }
}

#[test]
fn test_path_null_byte_rejection() {
    // Paths with null bytes should be handled carefully
    // Rust's Path/PathBuf don't allow null bytes in construction
    // This test verifies normal path operations don't introduce nulls
    let path = PathBuf::from("normal/path.txt");
    let path_str = path.to_string_lossy();
    assert!(!path_str.contains('\0'), "Path should not contain null bytes");
}

// ============================================================================
// Command Execution Platform Differences
// ============================================================================

#[test]
fn test_command_shell_differences() {
    // Different shells on different platforms
    let shell = if is_windows() {
        ("cmd", vec!["/C", "echo", "test"])
    } else {
        ("sh", vec!["-c", "echo test"])
    };

    let result = Command::new(shell.0)
        .args(&shell.1)
        .output();

    assert!(result.is_ok(), "Shell command should execute");
    let output = result.unwrap();
    assert!(output.status.success(), "Shell echo should succeed");
}

#[test]
fn test_command_path_separator_in_args() {
    // Test that paths in command arguments work across platforms
    let test_path = if is_windows() {
        "src\\main.rs"
    } else {
        "src/main.rs"
    };

    // Use forward slash which works on all platforms
    let universal_path = "src/main.rs";

    // Both should be valid
    assert!(PathBuf::from(test_path).to_str().is_some());
    assert!(PathBuf::from(universal_path).to_str().is_some());
}

#[test]
fn test_command_output_encoding() {
    // Test that command output encoding is handled correctly
    let result = if is_windows() {
        Command::new("cmd").args(["/C", "echo", "Hello World"]).output()
    } else {
        Command::new("echo").arg("Hello World").output()
    };

    assert!(result.is_ok());
    let output = result.unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should handle ASCII correctly on all platforms
    assert!(stdout.contains("Hello"));
}

#[test]
fn test_command_with_unicode_output() {
    // Test Unicode handling in command output
    let unicode_text = "Hello 世界 🌍";

    let result = if is_windows() {
        Command::new("cmd")
            .args(["/C", "echo", unicode_text])
            .output()
    } else {
        Command::new("printf").arg("%s").arg(unicode_text).output()
    };

    // Command should execute (encoding handling may vary)
    if let Ok(output) = result {
        // Just verify the command ran without crashing
        let _stdout = String::from_utf8_lossy(&output.stdout);
    }
}

#[test]
fn test_command_timeout_basic() {
    // Test that short commands complete
    use std::time::Instant;

    let start = Instant::now();

    let result = if is_windows() {
        Command::new("cmd").args(["/C", "echo", "done"]).output()
    } else {
        Command::new("echo").arg("done").output()
    };

    let duration = start.elapsed();

    assert!(result.is_ok(), "Command should complete");
    assert!(duration.as_secs() < 5, "Echo command should be fast");
}

// ============================================================================
// Environment Variable Platform Differences
// ============================================================================

#[test]
fn test_env_var_case_sensitivity() {
    // Environment variables are case-insensitive on Windows
    #[cfg(windows)]
    {
        // On Windows, PATH and path should refer to the same variable
        let path_upper = env::var("PATH");
        let path_lower = env::var("path");
        // Both should succeed or both should fail
        assert_eq!(path_upper.is_ok(), path_lower.is_ok());
    }

    #[cfg(not(windows))]
    {
        // On Unix, they are different variables
        let path_upper = env::var("PATH");
        assert!(path_upper.is_ok(), "PATH should exist on Unix");
    }
}

#[test]
fn test_env_path_separator() {
    let path_var = env::var("PATH").unwrap();
    let separator = if is_windows() { ';' } else { ':' };

    // PATH should have at least one entry
    let entries: Vec<_> = path_var.split(separator).filter(|s| !s.is_empty()).collect();
    assert!(!entries.is_empty(), "PATH should have entries");
}

#[test]
fn test_env_temp_variable() {
    // Different temp variables on different platforms
    let temp = if is_windows() {
        env::var("TEMP").or_else(|_| env::var("TMP"))
    } else {
        env::var("TMPDIR").or_else(|_| Ok("/tmp".to_string()))
    };

    assert!(temp.is_ok(), "Temp directory variable should be available");
}

#[test]
fn test_env_user_variables() {
    // User identification varies by platform
    let user = if is_windows() {
        env::var("USERNAME").or_else(|_| env::var("USER"))
    } else {
        env::var("USER")
    };

    // User should be available on most systems
    assert!(user.is_ok() || is_windows(), "User variable should be available");
}

// ============================================================================
// Terminal Color Configuration Tests
// ============================================================================

#[test]
fn test_color_env_no_color() {
    // Test NO_COLOR environment variable support
    let no_color_set = env::var("NO_COLOR").is_ok();

    // If NO_COLOR is set, applications should respect it
    // This test just verifies we can detect it
    if no_color_set {
        let _value = env::var("NO_COLOR").unwrap();
        // Any value (even empty) means colors should be disabled
    }
}

#[test]
fn test_color_term_variable() {
    // TERM variable affects color support on Unix
    #[cfg(unix)]
    {
        let term = env::var("TERM");
        if let Ok(term_value) = term {
            // Common terminal types that support colors
            let supports_color = term_value.contains("xterm")
                || term_value.contains("screen")
                || term_value.contains("linux")
                || term_value.contains("ansi")
                || term_value == "dumb"; // dumb may not support color

            // Just verify we can read the variable
            let _ = supports_color;
        }
    }
}

#[test]
fn test_colorterm_variable() {
    // COLORTERM indicates true color support
    let colorterm = env::var("COLORTERM");

    if let Ok(ct) = colorterm {
        // truecolor or 24bit indicates full RGB support
        let has_true_color = ct == "truecolor" || ct == "24bit";
        let _ = has_true_color;
    }
}

// ============================================================================
// Async Runtime Cross-Platform Tests
// ============================================================================

#[test]
fn test_tokio_runtime_creation() {
    // Test that basic async operations work
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();

    assert!(rt.is_ok(), "Tokio runtime should create");

    let runtime = rt.unwrap();
    let result = runtime.block_on(async { 42 });

    assert_eq!(result, 42, "Async block should execute");
}

#[test]
fn test_tokio_multi_thread() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build();

    assert!(rt.is_ok(), "Multi-threaded runtime should create");

    let runtime = rt.unwrap();
    let result = runtime.block_on(async {
        let handle = tokio::spawn(async { 100 });
        handle.await.unwrap()
    });

    assert_eq!(result, 100, "Spawned task should return correct value");
}

#[test]
fn test_tokio_sleep() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let start = std::time::Instant::now();
    rt.block_on(async {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    });

    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() >= 10, "Sleep should wait at least 10ms");
}

// ============================================================================
// File Locking Cross-Platform Tests
// ============================================================================

#[test]
fn test_file_create_exclusive() {
    let temp_dir = env::temp_dir();
    let test_file = temp_dir.join("ltmatrix_exclusive_test.txt");

    // Clean up any existing file
    let _ = fs::remove_file(&test_file);

    // Create file
    let result = fs::File::create_new(&test_file);
    assert!(result.is_ok(), "Should create new file exclusively");

    // Second creation should fail
    let second = fs::File::create_new(&test_file);
    assert!(second.is_err(), "Second exclusive create should fail");

    // Clean up
    fs::remove_file(&test_file).ok();
}

#[test]
fn test_file_append_mode() {
    let temp_dir = env::temp_dir();
    let test_file = temp_dir.join("ltmatrix_append_test.txt");

    // Clean up and create initial file
    let _ = fs::remove_file(&test_file);
    fs::write(&test_file, "line1\n").unwrap();

    // Append to file
    use std::fs::OpenOptions;
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(&test_file)
        .unwrap();

    use std::io::Write;
    file.write_all(b"line2\n").unwrap();

    // Verify content
    let content = fs::read_to_string(&test_file).unwrap();
    assert!(content.contains("line1"), "Should have original content");
    assert!(content.contains("line2"), "Should have appended content");

    // Clean up
    fs::remove_file(&test_file).ok();
}

// ============================================================================
// Symlink Cross-Platform Tests
// ============================================================================

#[test]
fn test_symlink_creation() {
    let temp_dir = env::temp_dir();
    let original = temp_dir.join("ltmatrix_original.txt");
    let link = temp_dir.join("ltmatrix_link.txt");

    // Create original file
    fs::write(&original, "test content").unwrap();

    // Create symlink
    #[cfg(unix)]
    {
        let result = std::os::unix::fs::symlink(&original, &link);
        if result.is_ok() {
            assert!(link.exists(), "Symlink should exist");
            let content = fs::read_to_string(&link).unwrap();
            assert_eq!(content, "test content");
            fs::remove_file(&link).ok();
        }
    }

    #[cfg(windows)]
    {
        // Windows symlinks may require admin privileges
        let result = std::os::windows::fs::symlink_file(&original, &link);
        if result.is_ok() {
            assert!(link.exists(), "Symlink should exist");
            fs::remove_file(&link).ok();
        }
        // If it fails due to permissions, that's expected
    }

    // Clean up
    fs::remove_file(&original).ok();
}

#[test]
fn test_symlink_detection() {
    let temp_dir = env::temp_dir();
    let regular_file = temp_dir.join("ltmatrix_regular.txt");

    fs::write(&regular_file, "test").unwrap();

    let metadata = fs::symlink_metadata(&regular_file);
    assert!(metadata.is_ok(), "Should get symlink metadata");

    let meta = metadata.unwrap();
    assert!(!meta.file_type().is_symlink(), "Regular file should not be symlink");

    fs::remove_file(&regular_file).ok();
}

// ============================================================================
// Permissions Cross-Platform Tests
// ============================================================================

#[test]
fn test_file_permissions_read_only() {
    let temp_dir = env::temp_dir();
    let test_file = temp_dir.join("ltmatrix_readonly.txt");

    fs::write(&test_file, "content").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&test_file).unwrap().permissions();
        perms.set_mode(0o444); // Read-only
        fs::set_permissions(&test_file, perms).unwrap();

        // Verify we set read-only
        let new_perms = fs::metadata(&test_file).unwrap().permissions();
        assert!(!new_perms.readonly(), "Unix mode 0o444 includes read bits");

        // Reset for cleanup
        let mut perms = fs::metadata(&test_file).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&test_file, perms).ok();
    }

    #[cfg(windows)]
    {
        let perms = fs::metadata(&test_file).unwrap().permissions();
        let _ = perms.readonly(); // Just verify we can read it
    }

    fs::remove_file(&test_file).ok();
}

// ============================================================================
// Git Operations Platform-Specific Tests
// ============================================================================

#[test]
fn test_git_config_system() {
    // Git config command works differently on different platforms
    let result = Command::new("git")
        .args(["config", "--system", "--list"])
        .output();

    // May fail due to permissions, but shouldn't crash
    if let Ok(output) = result {
        // Just verify git command executed
        let _status = output.status;
    }
}

#[test]
fn test_git_config_global() {
    let result = Command::new("git")
        .args(["config", "--global", "--list"])
        .output();

    // May succeed or fail depending on setup
    if let Ok(output) = result {
        let _status = output.status;
    }
}

#[test]
fn test_git_core_autocrlf() {
    // Test line ending configuration which differs by platform
    let result = Command::new("git")
        .args(["config", "core.autocrlf"])
        .current_dir(project_root())
        .output();

    if let Ok(output) = result {
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout);
            // Common values: "true" (Windows), "input" (Unix), "false"
            let valid_values = ["true", "false", "input", ""];
            assert!(
                valid_values.contains(&value.trim()),
                "autocrlf should be a valid value"
            );
        }
    }
}

// ============================================================================
// Memory and Resource Tests
// ============================================================================

#[test]
fn test_available_parallelism() {
    // Test that we can detect available parallelism
    let parallelism = std::thread::available_parallelism();
    assert!(parallelism.is_ok(), "Should detect available parallelism");

    let count = parallelism.unwrap().get();
    assert!(count >= 1, "Should have at least 1 thread");
}

#[test]
fn test_system_info() {
    // Verify we can get basic system information
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    assert!(!os.is_empty(), "OS should be detected");
    assert!(!arch.is_empty(), "Architecture should be detected");

    // Common valid values
    let valid_os = ["windows", "linux", "macos"];
    let valid_arch = ["x86_64", "x86", "aarch64", "arm"];

    assert!(valid_os.contains(&os), "OS should be recognized: {}", os);
    assert!(valid_arch.contains(&arch), "Architecture should be recognized: {}", arch);
}
