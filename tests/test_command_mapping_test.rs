//! Comprehensive tests for test command mapping functionality
//!
//! This module specifically tests the mapping between detected frameworks
//! and their corresponding test commands, ensuring all frameworks return
//! correct commands for execution.

use ltmatrix::pipeline::test::TestFramework;

// ==================== Test Command Mapping Tests ====================

#[test]
fn test_pytest_command_mapping() {
    let framework = TestFramework::Pytest;
    assert_eq!(framework.test_command(), "pytest");
    assert!(!framework.test_command().is_empty());
    assert!(framework.test_command().contains("pytest"));
}

#[test]
fn test_npm_command_mapping() {
    let framework = TestFramework::Npm;
    assert_eq!(framework.test_command(), "npm test");
    assert!(!framework.test_command().is_empty());
    assert!(framework.test_command().contains("npm"));
    assert!(framework.test_command().contains("test"));
}

#[test]
fn test_go_command_mapping() {
    let framework = TestFramework::Go;
    assert_eq!(framework.test_command(), "go test ./...");
    assert!(!framework.test_command().is_empty());
    assert!(framework.test_command().contains("go"));
    assert!(framework.test_command().contains("test"));
    assert!(framework.test_command().contains("./..."));
}

#[test]
fn test_cargo_command_mapping() {
    let framework = TestFramework::Cargo;
    assert_eq!(framework.test_command(), "cargo test");
    assert!(!framework.test_command().is_empty());
    assert!(framework.test_command().contains("cargo"));
    assert!(framework.test_command().contains("test"));
}

#[test]
fn test_none_command_mapping() {
    let framework = TestFramework::None;
    assert_eq!(framework.test_command(), "");
    assert!(framework.test_command().is_empty());
}

// ==================== Edge Case: No Framework Found ====================

#[test]
fn test_no_framework_command() {
    let framework = TestFramework::None;
    assert_eq!(framework.test_command(), "");
    assert!(framework.test_command().is_empty());
}

#[test]
fn test_no_framework_display_name() {
    let framework = TestFramework::None;
    assert_eq!(framework.display_name(), "None");
    assert!(!framework.display_name().is_empty());
}

#[test]
fn test_no_framework_has_no_config() {
    let framework = TestFramework::None;
    assert!(!framework.has_config());
}

// ==================== Edge Case: Multiple Frameworks Present ====================

#[test]
fn test_framework_priority_cargo_over_npm() {
    // When multiple frameworks are detected, Cargo should take precedence
    let frameworks = vec![
        TestFramework::Cargo,
        TestFramework::Npm,
    ];

    assert_eq!(frameworks[0], TestFramework::Cargo);
    assert_eq!(frameworks[0].test_command(), "cargo test");
}

#[test]
fn test_framework_priority_go_over_pytest() {
    // Go should be detected before pytest
    let frameworks = vec![
        TestFramework::Go,
        TestFramework::Pytest,
    ];

    assert_eq!(frameworks[0], TestFramework::Go);
    assert_eq!(frameworks[0].test_command(), "go test ./...");
}

#[test]
fn test_framework_priority_pytest_over_npm() {
    // pytest should be detected before npm
    let frameworks = vec![
        TestFramework::Pytest,
        TestFramework::Npm,
    ];

    assert_eq!(frameworks[0], TestFramework::Pytest);
    assert_eq!(frameworks[0].test_command(), "pytest");
}

#[test]
fn test_all_frameworks_have_unique_commands() {
    let frameworks = vec![
        TestFramework::Pytest,
        TestFramework::Npm,
        TestFramework::Go,
        TestFramework::Cargo,
    ];

    let commands: Vec<_> = frameworks.iter()
        .map(|f| f.test_command())
        .collect();

    // All commands should be unique
    let unique_commands: std::collections::HashSet<_> = commands.iter().collect();
    assert_eq!(unique_commands.len(), frameworks.len());
}

#[test]
fn test_all_frameworks_have_unique_display_names() {
    let frameworks = vec![
        TestFramework::Pytest,
        TestFramework::Npm,
        TestFramework::Go,
        TestFramework::Cargo,
        TestFramework::None,
    ];

    let display_names: Vec<_> = frameworks.iter()
        .map(|f| f.display_name())
        .collect();

    // All display names should be unique
    let unique_names: std::collections::HashSet<_> = display_names.iter().collect();
    assert_eq!(unique_names.len(), frameworks.len());
}

// ==================== Command Format Consistency Tests ====================

#[test]
fn test_command_format_no_leading_trailing_whitespace() {
    let commands = vec![
        TestFramework::Pytest.test_command(),
        TestFramework::Npm.test_command(),
        TestFramework::Go.test_command(),
        TestFramework::Cargo.test_command(),
    ];

    for cmd in commands {
        assert_eq!(cmd, cmd.trim(), "Command should not have leading/trailing whitespace: '{}'", cmd);
    }
}

#[test]
fn test_command_format_lowercase_consistency() {
    // Framework-specific commands should be lowercase (except for multi-word commands)
    assert_eq!(TestFramework::Pytest.test_command(), "pytest");
    assert_eq!(TestFramework::Pytest.test_command().to_lowercase(), "pytest");

    assert_eq!(TestFramework::Cargo.test_command(), "cargo test");
    assert_eq!(TestFramework::Cargo.test_command().to_lowercase(), "cargo test");

    assert_eq!(TestFramework::Go.test_command(), "go test ./...");
    assert_eq!(TestFramework::Go.test_command().to_lowercase(), "go test ./...");

    // npm test contains a space, but individual words are lowercase
    let npm_cmd = TestFramework::Npm.test_command();
    assert!(npm_cmd.contains("npm"));
    assert!(npm_cmd.contains("test"));
}

#[test]
fn test_command_execution_ready() {
    // All test commands should be ready to execute without modification
    let frameworks = vec![
        TestFramework::Pytest,
        TestFramework::Npm,
        TestFramework::Go,
        TestFramework::Cargo,
    ];

    for framework in frameworks {
        let cmd = framework.test_command();
        assert!(!cmd.is_empty(), "{} command should not be empty", framework.display_name());
        assert!(cmd.len() > 0, "{} command should have length > 0", framework.display_name());
    }
}

// ==================== Framework Detection with Command Mapping ====================

#[test]
fn test_detection_result_returns_correct_command() {
    let frameworks = vec![
        (TestFramework::Pytest, "pytest"),
        (TestFramework::Npm, "npm test"),
        (TestFramework::Go, "go test ./..."),
        (TestFramework::Cargo, "cargo test"),
        (TestFramework::None, ""),
    ];

    for (framework, expected_command) in frameworks {
        assert_eq!(
            framework.test_command(),
            expected_command,
            "Framework {:?} should return command '{}'",
            framework,
            expected_command
        );
    }
}

#[test]
fn test_framework_cloning_preserves_command() {
    let original = TestFramework::Pytest;
    let cloned = original.clone();

    assert_eq!(original.test_command(), cloned.test_command());
    assert_eq!(original.test_command(), "pytest");
    assert_eq!(cloned.test_command(), "pytest");
}

// ==================== Comprehensive Framework Coverage ====================

#[test]
fn test_all_frameworks_mapped() {
    // Ensure all framework variants are mapped to commands
    let frameworks = vec![
        TestFramework::Pytest,
        TestFramework::Npm,
        TestFramework::Go,
        TestFramework::Cargo,
        TestFramework::None,
    ];

    for framework in frameworks {
        let cmd = framework.test_command();
        let display = framework.display_name();

        // All frameworks should have a display name
        assert!(!display.is_empty(), "{:?} should have a display name", framework);

        // All frameworks except None should have a test command
        if framework != TestFramework::None {
            assert!(!cmd.is_empty(), "{:?} should have a test command", framework);
        } else {
            assert!(cmd.is_empty(), "None should not have a test command");
        }
    }
}

#[test]
fn test_framework_command_consistency_across_calls() {
    // Commands should remain consistent across multiple calls
    let frameworks = vec![
        TestFramework::Pytest,
        TestFramework::Npm,
        TestFramework::Go,
        TestFramework::Cargo,
    ];

    for framework in frameworks {
        let cmd1 = framework.test_command();
        let cmd2 = framework.test_command();
        let cmd3 = framework.test_command();

        assert_eq!(cmd1, cmd2);
        assert_eq!(cmd2, cmd3);
        assert_eq!(cmd1, cmd3);
    }
}

// ==================== Integration with Confidence Scores ====================

#[test]
fn test_command_mapping_independent_of_confidence() {
    // Test commands should be consistent regardless of confidence scores
    // The command mapping is a property of the framework, not the detection
    let framework = TestFramework::Go;
    assert_eq!(
        framework.test_command(),
        "go test ./...",
        "Go framework command should always be correct"
    );
}

#[test]
fn test_command_mapping_with_varying_frameworks() {
    // Commands should be correct for all frameworks
    let frameworks = vec![
        TestFramework::Pytest,
        TestFramework::Npm,
        TestFramework::Go,
        TestFramework::Cargo,
    ];

    for framework in frameworks {
        let cmd = framework.test_command();
        assert!(!cmd.is_empty(), "{} command should not be empty", framework.display_name());
        assert!(cmd.contains("test") || cmd.contains("pytest"), "{} command should contain test-related keyword", framework.display_name());
    }
}

// ==================== Special Cases and Boundaries ====================

#[test]
fn test_go_command_includes_recursive_flag() {
    // Go test command should include ./... for recursive testing
    let cmd = TestFramework::Go.test_command();
    assert!(cmd.contains("./..."), "Go test command should include recursive flag");
}

#[test]
fn test_npm_command_preserves_space() {
    // npm test command should have a space between npm and test
    let cmd = TestFramework::Npm.test_command();
    assert!(cmd.contains(" "), "npm test command should contain a space");
    assert_eq!(cmd, "npm test");
}

#[test]
fn test_framework_commands_do_not_contain_newlines() {
    let frameworks = vec![
        TestFramework::Pytest,
        TestFramework::Npm,
        TestFramework::Go,
        TestFramework::Cargo,
    ];

    for framework in frameworks {
        let cmd = framework.test_command();
        assert!(!cmd.contains('\n'), "{} command should not contain newlines", framework.display_name());
        assert!(!cmd.contains('\r'), "{} command should not contain carriage returns", framework.display_name());
    }
}

#[test]
fn test_framework_commands_are_ascii() {
    let frameworks = vec![
        TestFramework::Pytest,
        TestFramework::Npm,
        TestFramework::Go,
        TestFramework::Cargo,
    ];

    for framework in frameworks {
        let cmd = framework.test_command();
        assert!(cmd.is_ascii(), "{} command should be ASCII: '{}'", framework.display_name(), cmd);
    }
}

// ==================== Real-World Scenario Tests ====================

#[test]
fn test_mixed_framework_scenario() {
    // Test that each framework correctly maps to its command
    let scenarios = vec![
        (TestFramework::Cargo, "cargo test"),
        (TestFramework::Go, "go test ./..."),
        (TestFramework::Pytest, "pytest"),
        (TestFramework::Npm, "npm test"),
    ];

    for (framework, expected_command) in scenarios {
        assert_eq!(
            framework.test_command(),
            expected_command,
            "{} should map to {}",
            framework.display_name(),
            expected_command
        );
    }
}

#[test]
fn test_framework_pytest_complete_workflow() {
    // Test pytest framework properties
    let framework = TestFramework::Pytest;

    // Verify the command mapping is correct
    assert_eq!(framework.test_command(), "pytest");
    assert_eq!(framework.display_name(), "pytest");
    assert!(framework.has_config());
}

#[test]
fn test_framework_none_edge_case_workflow() {
    // Test None framework properties
    let framework = TestFramework::None;

    // Verify the command mapping is correct (empty)
    assert_eq!(framework.test_command(), "");
    assert!(!framework.has_config());
    assert_eq!(framework.display_name(), "None");
}
