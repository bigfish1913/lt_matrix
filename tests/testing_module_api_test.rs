//! Tests for the testing module public API
//!
//! This test suite verifies that the testing module exports
//! the correct public API and that the detect_framework function
//! has the correct signature and behavior.

use ltmatrix::testing::{detect_framework, Framework, TestCommand};

#[test]
fn test_module_exports_framework() {
    // Test that Framework is accessible from the testing module
    let _fw = Framework::Cargo;
    let _fw = Framework::Pytest;
    let _fw = Framework::Npm;
    let _fw = Framework::Go;
    let _fw = Framework::None;
}

#[test]
fn test_module_exports_test_command() {
    // Test that TestCommand is accessible from the testing module
    let _cmd = TestCommand::new(Framework::Cargo);
    let _cmd = TestCommand::for_framework(&Framework::Pytest);
}

#[test]
fn test_module_exports_detect_framework() {
    // Test that detect_framework is accessible from the testing module
    // This is a compile-time check - if it compiles, the export works
    // We'll test the actual function behavior in async tests below
    let _ = detect_framework(".");
}

#[test]
fn test_detect_framework_returns_result() {
    // Test that detect_framework returns a Result type
    // This is a compile-time test

    // Create a simple future that uses detect_framework
    async fn check_return_type() {
        let result: anyhow::Result<Framework> = detect_framework(".").await;
        let _ = result;
    }

    // If this compiles, the return type is correct
    let _ = check_return_type();
}

#[tokio::test]
async fn test_detect_framework_accepts_path_str() {
    // Test that detect_framework accepts a string path
    let result = detect_framework(".").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_framework_accepts_path_buf() {
    // Test that detect_framework accepts a PathBuf
    use std::path::PathBuf;

    let path = PathBuf::from(".");
    let result = detect_framework(path).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_framework_accepts_path_slice() {
    // Test that detect_framework accepts a path slice (&Path)
    use std::path::Path;

    let path = Path::new(".");
    let result = detect_framework(path).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_framework_current_stub_returns_none() {
    // Test the current stub implementation
    // The current implementation always returns Framework::None
    let result = detect_framework(".").await.unwrap();
    assert_eq!(result, Framework::None);
}

#[tokio::test]
async fn test_detect_framework_with_nonexistent_path() {
    // Test behavior with a non-existent path
    // The current stub doesn't check paths, but this verifies
    // the signature accepts any path reference
    let result = detect_framework("/nonexistent/path/that/does/not/exist").await;
    // Stub returns Ok(None) for any path
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Framework::None);
}

#[tokio::test]
async fn test_detect_framework_with_empty_string() {
    // Test behavior with an empty string path
    let result = detect_framework("").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_framework_signature_is_async() {
    // Test that detect_framework is async by using it with tokio::test
    // If this test compiles and runs, the function is properly async

    async fn call_detect() -> anyhow::Result<Framework> {
        detect_framework(".").await
    }

    let result = call_detect().await;
    assert!(result.is_ok());
}

#[test]
fn test_detect_framework_documentation_example_compiles() {
    // Test that the example from the documentation compiles
    // This is a compile-time check

    // The example from the doc comment:
    // ```no_run
    // use ltmatrix::testing::detect_framework;
    //
    // # async fn example() -> anyhow::Result<()> {
    // let framework = detect_framework(".").await?;
    // println!("Detected framework: {}", framework);
    // # Ok(())
    // # }
    // ```

    async fn example() -> anyhow::Result<()> {
        let framework = detect_framework(".").await?;
        let _ = format!("Detected framework: {}", framework);
        Ok(())
    }

    // If this compiles, the example code is valid
    let _ = example();
}

#[test]
fn test_framework_and_test_command_integration() {
    // Test that Framework and TestCommand work together

    // Get command from framework
    let fw = Framework::Cargo;
    let cmd = TestCommand::for_framework(&fw);

    assert_eq!(cmd.framework, fw);
    assert_eq!(cmd.program, "cargo");
}

#[test]
fn test_all_frameworks_have_test_commands() {
    // Test that all framework variants can create TestCommand instances

    let frameworks = vec![
        Framework::Pytest,
        Framework::Npm,
        Framework::Go,
        Framework::Cargo,
        Framework::None,
    ];

    for fw in frameworks {
        let cmd = TestCommand::new(fw);
        assert_eq!(cmd.framework, fw);
    }
}

#[tokio::test]
async fn test_detect_framework_result_matches_framework_type() {
    // Test that detect_framework returns the correct Framework enum type

    let result = detect_framework(".").await.unwrap();

    // Verify it's actually a Framework variant
    match result {
        Framework::Pytest => {}
        Framework::Npm => {}
        Framework::Go => {}
        Framework::Cargo => {}
        Framework::None => {}
    }
}

#[test]
fn test_module_api_comprehensive_coverage() {
    // Comprehensive test of all public API surface

    // Framework enum
    let fw1 = Framework::Pytest;
    let fw2 = Framework::Npm;
    let _name1 = fw1.name();
    let _cmd1 = fw1.command();
    let _ = format!("{}", fw1);

    // TestCommand
    let cmd1 = TestCommand::new(fw1);
    let cmd2 = TestCommand::for_framework(&fw2);
    let _cmd3 = cmd1.clone().with_arg("--test").with_env("KEY", "value");
    let _cmd_line = cmd2.to_command_line();

    // Verify traits are implemented
    let _fw_clone = fw1;
    let _cmd_clone = cmd1.clone();
    let _ = format!("{:?}", fw1);
    let _ = format!("{:?}", cmd1);

    // Test equality
    assert_eq!(fw1, Framework::Pytest);
    assert_ne!(fw1, fw2);
    assert_eq!(cmd1, cmd1);
    assert_ne!(cmd1, cmd2);
}

#[tokio::test]
async fn test_detect_framework_with_absolute_path() {
    // Test that detect_framework accepts absolute paths
    // Use the current directory as an absolute path
    let current_dir = std::env::current_dir().unwrap();
    let result = detect_framework(&current_dir).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_framework_with_relative_path() {
    // Test that detect_framework accepts relative paths
    let result = detect_framework("../").await;
    assert!(result.is_ok());
}

#[test]
fn test_framework_is_send_and_sync() {
    // Test that Framework implements Send and Sync
    // This is important for async contexts

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<Framework>();
    assert_sync::<Framework>();
}

#[test]
fn test_test_command_is_send_and_sync() {
    // Test that TestCommand implements Send and Sync
    // This is important for async contexts

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<TestCommand>();
    assert_sync::<TestCommand>();
}

#[tokio::test]
async fn test_detect_framework_can_be_used_in_async_context() {
    // Test that detect_framework works properly in async contexts
    // with other async operations

    async fn check_multiple_paths() -> anyhow::Result<Vec<Framework>> {
        let paths = [".", "..", "../.."];

        let mut frameworks = Vec::new();
        for path in paths {
            let fw = detect_framework(path).await?;
            frameworks.push(fw);
        }

        Ok(frameworks)
    }

    let result = check_multiple_paths().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 3);
}

#[tokio::test]
async fn test_detect_framework_error_handling() {
    // Test that detect_framework properly handles errors
    // Currently the stub doesn't error, but this verifies
    // the Result type is correct

    let result = detect_framework(".").await;

    match result {
        Ok(fw) => {
            // Should always get Ok with current stub
            let _ = fw;
        }
        Err(e) => {
            // If it does error, verify error type
            let _: anyhow::Error = e;
        }
    }
}

#[test]
fn test_module_public_api_completeness() {
    // Verify all expected public API items are accessible

    // Types
    let _: Framework = Framework::None;
    let _: TestCommand = TestCommand::new(Framework::None);

    // Functions
    // detect_framework is async, tested in async tests above

    // Associated functions
    let _ = Framework::Pytest.name();
    let _ = Framework::Pytest.command();
    let _ = TestCommand::for_framework(&Framework::Pytest);

    // Builder methods
    let _ = TestCommand::new(Framework::None)
        .with_arg("test")
        .with_args(&["a", "b"])
        .with_env("k", "v")
        .with_work_dir(".");
}

#[test]
fn test_framework_command_returns_static_slice() {
    // Test that command() returns a static slice with 'static lifetime
    let cmd = Framework::Cargo.command();
    // The returned reference should be valid for 'static
    let _static_ref: &'static [&'static str] = cmd;
}

#[test]
fn test_test_command_builder_returns_self() {
    // Test that builder methods return Self for chaining
    let cmd = TestCommand::new(Framework::Cargo);

    // Each method should return Self
    let cmd2 = cmd.with_arg("test");
    let cmd3 = cmd2.with_args(&["a"]);
    let cmd4 = cmd3.with_env("k", "v");
    let cmd5 = cmd4.with_work_dir(".");

    // All should be TestCommand instances
    let _: TestCommand = cmd5;
}

#[tokio::test]
async fn test_detect_framework_anyhow_result() {
    // Test that detect_framework returns anyhow::Result
    // This is important for error handling consistency

    let result: anyhow::Result<Framework> = detect_framework(".").await;

    match result {
        Ok(_) => {}
        Err(e) => {
            // Verify error provides context
            let _error_msg = format!("{:?}", e);
        }
    }
}

#[test]
fn test_test_command_to_command_line_is_deterministic() {
    // Test that to_command_line() returns the same result for
    // identical commands

    let cmd1 = TestCommand::new(Framework::Cargo)
        .with_arg("--release")
        .with_work_dir("/test");

    let cmd2 = TestCommand::new(Framework::Cargo)
        .with_arg("--release")
        .with_work_dir("/test");

    assert_eq!(cmd1.to_command_line(), cmd2.to_command_line());
}
