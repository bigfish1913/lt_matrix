//! Tests for the TestCommand struct and its builder pattern
//!
//! This test suite verifies the TestCommand struct's functionality,
//! including its builder methods, serialization, and command formatting.

use ltmatrix::testing::{Framework, TestCommand};
use serde_json::{from_str, to_string};

#[test]
fn test_test_command_new_for_all_frameworks() {
    // Test creating TestCommand for each framework variant

    // Pytest
    let pytest_cmd = TestCommand::new(Framework::Pytest);
    assert_eq!(pytest_cmd.framework, Framework::Pytest);
    assert_eq!(pytest_cmd.program, "pytest");
    assert_eq!(pytest_cmd.args, vec!["-v"]);
    assert!(pytest_cmd.env_vars.is_empty());
    assert!(pytest_cmd.work_dir.is_none());

    // Npm
    let npm_cmd = TestCommand::new(Framework::Npm);
    assert_eq!(npm_cmd.framework, Framework::Npm);
    assert_eq!(npm_cmd.program, "npm");
    assert_eq!(npm_cmd.args, vec!["test"]);
    assert!(npm_cmd.env_vars.is_empty());
    assert!(npm_cmd.work_dir.is_none());

    // Go
    let go_cmd = TestCommand::new(Framework::Go);
    assert_eq!(go_cmd.framework, Framework::Go);
    assert_eq!(go_cmd.program, "go");
    assert_eq!(go_cmd.args, vec!["test", "./..."]);
    assert!(go_cmd.env_vars.is_empty());
    assert!(go_cmd.work_dir.is_none());

    // Cargo
    let cargo_cmd = TestCommand::new(Framework::Cargo);
    assert_eq!(cargo_cmd.framework, Framework::Cargo);
    assert_eq!(cargo_cmd.program, "cargo");
    assert_eq!(cargo_cmd.args, vec!["test"]);
    assert!(cargo_cmd.env_vars.is_empty());
    assert!(cargo_cmd.work_dir.is_none());

    // None
    let none_cmd = TestCommand::new(Framework::None);
    assert_eq!(none_cmd.framework, Framework::None);
    assert_eq!(none_cmd.program, "echo");
    assert_eq!(none_cmd.args, vec!["No tests configured"]);
    assert!(none_cmd.env_vars.is_empty());
    assert!(none_cmd.work_dir.is_none());
}

#[test]
fn test_test_command_for_framework() {
    // Test the for_framework factory method
    let cmd = TestCommand::for_framework(&Framework::Cargo);

    assert_eq!(cmd.framework, Framework::Cargo);
    assert_eq!(cmd.program, "cargo");
    assert_eq!(cmd.args, vec!["test"]);
}

#[test]
fn test_test_command_with_arg_single() {
    // Test adding a single argument using builder pattern
    let cmd = TestCommand::new(Framework::Cargo).with_arg("--release");

    assert_eq!(cmd.args, vec!["test", "--release"]);
}

#[test]
fn test_test_command_with_arg_chaining() {
    // Test chaining multiple with_arg calls
    let cmd = TestCommand::new(Framework::Cargo)
        .with_arg("--release")
        .with_arg("--verbose")
        .with_arg("--no-fail-fast");

    assert_eq!(
        cmd.args,
        vec!["test", "--release", "--verbose", "--no-fail-fast"]
    );
}

#[test]
fn test_test_command_with_args_multiple() {
    // Test adding multiple arguments at once
    let cmd = TestCommand::new(Framework::Pytest).with_args(&["--tb=short", "-x", "--timeout=10"]);

    assert_eq!(cmd.args, vec!["-v", "--tb=short", "-x", "--timeout=10"]);
}

#[test]
fn test_test_command_with_args_mixed() {
    // Test mixing with_arg and with_args
    let cmd = TestCommand::new(Framework::Cargo)
        .with_arg("--release")
        .with_args(&["--verbose", "--no-fail-fast"])
        .with_arg("--quiet");

    assert_eq!(
        cmd.args,
        vec![
            "test",
            "--release",
            "--verbose",
            "--no-fail-fast",
            "--quiet"
        ]
    );
}

#[test]
fn test_test_command_with_env_single() {
    // Test adding a single environment variable
    let cmd = TestCommand::new(Framework::Cargo).with_env("RUST_LOG", "debug");

    assert_eq!(cmd.env_vars.len(), 1);
    assert_eq!(
        cmd.env_vars[0],
        ("RUST_LOG".to_string(), "debug".to_string())
    );
}

#[test]
fn test_test_command_with_env_multiple() {
    // Test adding multiple environment variables
    let cmd = TestCommand::new(Framework::Cargo)
        .with_env("RUST_LOG", "debug")
        .with_env("RUST_BACKTRACE", "1")
        .with_env("TEST_VAR", "test_value");

    assert_eq!(cmd.env_vars.len(), 3);
    assert_eq!(
        cmd.env_vars[0],
        ("RUST_LOG".to_string(), "debug".to_string())
    );
    assert_eq!(
        cmd.env_vars[1],
        ("RUST_BACKTRACE".to_string(), "1".to_string())
    );
    assert_eq!(
        cmd.env_vars[2],
        ("TEST_VAR".to_string(), "test_value".to_string())
    );
}

#[test]
fn test_test_command_with_env_string_types() {
    // Test that with_env accepts different string-like types
    let cmd = TestCommand::new(Framework::Cargo)
        .with_env("KEY1", "value1")
        .with_env(String::from("KEY2"), String::from("value2"))
        .with_env(&String::from("KEY3"), &String::from("value3"));

    assert_eq!(cmd.env_vars.len(), 3);
}

#[test]
fn test_test_command_with_work_dir() {
    // Test setting the working directory
    let cmd = TestCommand::new(Framework::Cargo).with_work_dir("/my/project");

    assert_eq!(cmd.work_dir, Some("/my/project".to_string()));
}

#[test]
fn test_test_command_with_work_dir_types() {
    // Test that with_work_dir accepts different string-like types
    let cmd1 = TestCommand::new(Framework::Cargo).with_work_dir("/path1");

    let cmd2 = TestCommand::new(Framework::Cargo).with_work_dir(String::from("/path2"));

    let cmd3 = TestCommand::new(Framework::Cargo).with_work_dir(&String::from("/path3"));

    assert_eq!(cmd1.work_dir, Some("/path1".to_string()));
    assert_eq!(cmd2.work_dir, Some("/path2".to_string()));
    assert_eq!(cmd3.work_dir, Some("/path3".to_string()));
}

#[test]
fn test_test_command_full_builder_pattern() {
    // Test using all builder methods together
    let cmd = TestCommand::new(Framework::Cargo)
        .with_arg("--release")
        .with_args(&["--verbose", "--no-fail-fast"])
        .with_env("RUST_LOG", "debug")
        .with_env("RUST_BACKTRACE", "1")
        .with_work_dir("/tmp/test");

    assert_eq!(cmd.framework, Framework::Cargo);
    assert_eq!(cmd.program, "cargo");
    assert_eq!(
        cmd.args,
        vec!["test", "--release", "--verbose", "--no-fail-fast"]
    );
    assert_eq!(cmd.env_vars.len(), 2);
    assert_eq!(cmd.work_dir, Some("/tmp/test".to_string()));
}

#[test]
fn test_test_command_to_command_line_simple() {
    // Test converting to command line string (simple case)
    let cmd = TestCommand::new(Framework::Pytest);
    assert_eq!(cmd.to_command_line(), "pytest -v");
}

#[test]
fn test_test_command_to_command_line_with_args() {
    // Test converting to command line string with arguments
    let cmd = TestCommand::new(Framework::Cargo)
        .with_arg("--release")
        .with_arg("--verbose");

    assert_eq!(cmd.to_command_line(), "cargo test --release --verbose");
}

#[test]
fn test_test_command_to_command_line_with_work_dir() {
    // Test converting to command line string with working directory
    let cmd = TestCommand::new(Framework::Cargo).with_work_dir("/my/project");

    assert_eq!(cmd.to_command_line(), "(cd /my/project && cargo test)");
}

#[test]
fn test_test_command_to_command_line_with_all() {
    // Test converting to command line string with everything
    let cmd = TestCommand::new(Framework::Go)
        .with_args(&["-v", "-race"])
        .with_work_dir("/home/user/project");

    // Note: env_vars are not included in to_command_line()
    assert_eq!(
        cmd.to_command_line(),
        "(cd /home/user/project && go test ./... -v -race)"
    );
}

#[test]
fn test_test_command_to_command_line_npm() {
    // Test npm command formatting
    let cmd = TestCommand::new(Framework::Npm);
    assert_eq!(cmd.to_command_line(), "npm test");
}

#[test]
fn test_test_command_to_command_line_go() {
    // Test go command formatting
    let cmd = TestCommand::new(Framework::Go);
    assert_eq!(cmd.to_command_line(), "go test ./...");
}

#[test]
fn test_test_command_to_command_line_none() {
    // Test None framework command formatting
    let cmd = TestCommand::new(Framework::None);
    assert_eq!(cmd.to_command_line(), "echo No tests configured");
}

#[test]
fn test_test_command_equality() {
    // Test PartialEq implementation
    let cmd1 = TestCommand::new(Framework::Cargo);
    let cmd2 = TestCommand::new(Framework::Cargo);
    assert_eq!(cmd1, cmd2);

    let cmd3 = TestCommand::new(Framework::Pytest);
    assert_ne!(cmd1, cmd3);

    let cmd4 = TestCommand::new(Framework::Cargo).with_arg("--release");
    assert_ne!(cmd1, cmd4);
}

#[test]
fn test_test_command_clone() {
    // Test Clone implementation
    let original = TestCommand::new(Framework::Cargo)
        .with_arg("--release")
        .with_env("RUST_LOG", "debug")
        .with_work_dir("/tmp");

    let cloned = original.clone();

    assert_eq!(original, cloned);

    // Verify independence
    assert_eq!(original.framework, cloned.framework);
    assert_eq!(original.program, cloned.program);
    assert_eq!(original.args, cloned.args);
    assert_eq!(original.env_vars, cloned.env_vars);
    assert_eq!(original.work_dir, cloned.work_dir);
}

#[test]
fn test_test_command_debug_format() {
    // Test Debug trait implementation
    let cmd = TestCommand::new(Framework::Cargo);
    let debug_str = format!("{:?}", cmd);

    assert!(debug_str.contains("Cargo"));
    assert!(debug_str.contains("cargo"));
}

#[test]
fn test_test_command_serialization() {
    // Test that TestCommand can be serialized to JSON
    let cmd = TestCommand::new(Framework::Cargo)
        .with_arg("--release")
        .with_env("RUST_LOG", "debug");

    let json = to_string(&cmd).expect("Failed to serialize TestCommand");

    // Verify JSON contains expected fields
    assert!(json.contains("\"Cargo\""));
    assert!(json.contains("\"cargo\""));
    assert!(json.contains("--release"));
    assert!(json.contains("RUST_LOG"));
}

#[test]
fn test_test_command_deserialization() {
    // Test that TestCommand can be deserialized from JSON
    let json = r#"{
        "framework": "Cargo",
        "program": "cargo",
        "args": ["test", "--release"],
        "env_vars": [["RUST_LOG", "debug"]],
        "work_dir": "/tmp"
    }"#;

    let cmd: TestCommand = from_str(json).expect("Failed to deserialize TestCommand");

    assert_eq!(cmd.framework, Framework::Cargo);
    assert_eq!(cmd.program, "cargo");
    assert_eq!(cmd.args, vec!["test", "--release"]);
    assert_eq!(
        cmd.env_vars,
        vec![("RUST_LOG".to_string(), "debug".to_string())]
    );
    assert_eq!(cmd.work_dir, Some("/tmp".to_string()));
}

#[test]
fn test_test_command_roundtrip_serialization() {
    // Test that serialization and deserialization are symmetric
    let original = TestCommand::new(Framework::Pytest)
        .with_args(&["--tb=short", "-x"])
        .with_env("PYTEST_TIMEOUT", "10")
        .with_work_dir("/test/path");

    let json = to_string(&original).expect("Failed to serialize");
    let deserialized: TestCommand = from_str(&json).expect("Failed to deserialize");

    assert_eq!(original, deserialized);
}

#[test]
fn test_test_command_env_vars_order_preserved() {
    // Test that environment variables maintain their insertion order
    let cmd = TestCommand::new(Framework::Cargo)
        .with_env("VAR1", "value1")
        .with_env("VAR2", "value2")
        .with_env("VAR3", "value3");

    assert_eq!(cmd.env_vars[0].0, "VAR1");
    assert_eq!(cmd.env_vars[1].0, "VAR2");
    assert_eq!(cmd.env_vars[2].0, "VAR3");
}

#[test]
fn test_test_command_args_order_preserved() {
    // Test that arguments maintain their insertion order
    let cmd = TestCommand::new(Framework::Cargo)
        .with_arg("--arg1")
        .with_arg("--arg2")
        .with_arg("--arg3");

    // First arg is "test" from the constructor
    assert_eq!(cmd.args[0], "test");
    assert_eq!(cmd.args[1], "--arg1");
    assert_eq!(cmd.args[2], "--arg2");
    assert_eq!(cmd.args[3], "--arg3");
}

#[test]
fn test_test_command_builder_pattern_consumes_self() {
    // Test that builder methods consume and return self
    let cmd = TestCommand::new(Framework::Cargo);
    let cmd = cmd.with_arg("--release");

    // Original cmd is moved, but we still have the new one
    assert_eq!(cmd.args, vec!["test", "--release"]);
}

#[test]
fn test_test_command_empty_args_allowed() {
    // Test that commands can have no additional args beyond defaults
    let cmd = TestCommand::new(Framework::Cargo);
    assert_eq!(cmd.args.len(), 1); // Just "test"
    assert_eq!(cmd.args[0], "test");
}

#[test]
fn test_test_command_none_framework_special_handling() {
    // Test that Framework::None gets special echo command
    let cmd = TestCommand::new(Framework::None);

    assert_eq!(cmd.program, "echo");
    assert_eq!(cmd.args, vec!["No tests configured"]);
    assert_eq!(cmd.framework, Framework::None);
}

#[test]
fn test_test_command_work_dir_none_by_default() {
    // Test that work_dir is None by default
    let cmd = TestCommand::new(Framework::Cargo);
    assert!(cmd.work_dir.is_none());
}

#[test]
fn test_test_command_env_vars_empty_by_default() {
    // Test that env_vars is empty by default
    let cmd = TestCommand::new(Framework::Cargo);
    assert!(cmd.env_vars.is_empty());
}

#[test]
fn test_test_command_with_overwrite_work_dir() {
    // Test that with_work_dir overwrites previous value
    let cmd = TestCommand::new(Framework::Cargo)
        .with_work_dir("/path1")
        .with_work_dir("/path2");

    assert_eq!(cmd.work_dir, Some("/path2".to_string()));
}

#[test]
fn test_test_command_duplicate_env_keys_allowed() {
    // Test that duplicate env keys are allowed (not deduplicated)
    let cmd = TestCommand::new(Framework::Cargo)
        .with_env("KEY", "value1")
        .with_env("KEY", "value2");

    assert_eq!(cmd.env_vars.len(), 2);
    assert_eq!(cmd.env_vars[0], ("KEY".to_string(), "value1".to_string()));
    assert_eq!(cmd.env_vars[1], ("KEY".to_string(), "value2".to_string()));
}

#[test]
fn test_test_command_complex_real_world_example() {
    // Test a complex real-world test command scenario
    let cmd = TestCommand::new(Framework::Cargo)
        .with_args(&["--release", "--no-fail-fast", "--verbose"])
        .with_env("RUST_LOG", "ltmatrix=debug")
        .with_env("RUST_BACKTRACE", "full")
        .with_env("TEST_ENV_VAR", "test_value")
        .with_work_dir("/home/user/projects/ltmatrix");

    assert_eq!(cmd.framework, Framework::Cargo);
    assert_eq!(cmd.program, "cargo");
    assert_eq!(
        cmd.args,
        vec!["test", "--release", "--no-fail-fast", "--verbose"]
    );
    assert_eq!(cmd.env_vars.len(), 3);
    assert_eq!(
        cmd.to_command_line(),
        "(cd /home/user/projects/ltmatrix && cargo test --release --no-fail-fast --verbose)"
    );
}
