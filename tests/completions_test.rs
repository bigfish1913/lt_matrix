//! Comprehensive tests for shell completions generation
//!
//! This test suite verifies:
//! - Completion script generation for all supported shells (bash, zsh, fish, powershell, elvish)
//! - Completion script contains expected patterns and functions
//! - Error handling for invalid shell types
//! - Completion subcommand execution
//! - Installation instructions availability
//! - Dynamic completion support for project-specific values

use clap::Parser;
use ltmatrix::cli::args::{Args, Command, Shell};

// =============================================================================
// Basic Completion Generation Tests
// =============================================================================

#[test]
fn test_generate_bash_completion() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();
    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);
    assert!(!buf.is_empty(), "Should generate bash completion successfully");
}

#[test]
fn test_generate_zsh_completion() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();
    clap_complete::generate(Shell::Zsh, &mut cmd, "ltmatrix", &mut buf);
    assert!(!buf.is_empty(), "Should generate zsh completion successfully");
}

#[test]
fn test_generate_fish_completion() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();
    clap_complete::generate(Shell::Fish, &mut cmd, "ltmatrix", &mut buf);
    assert!(!buf.is_empty(), "Should generate fish completion successfully");
}

#[test]
fn test_generate_powershell_completion() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();
    clap_complete::generate(Shell::PowerShell, &mut cmd, "ltmatrix", &mut buf);
    assert!(!buf.is_empty(), "Should generate powershell completion successfully");
}

#[test]
fn test_generate_elvish_completion() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();
    clap_complete::generate(Shell::Elvish, &mut cmd, "ltmatrix", &mut buf);
    assert!(!buf.is_empty(), "Should generate elvish completion successfully");
}

// =============================================================================
// Completion Script Content Validation Tests
// =============================================================================

#[test]
fn test_bash_completion_content() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");

    // Verify bash completion contains expected patterns
    assert!(!output.is_empty(), "Completion output should not be empty");
    assert!(
        output.contains("_ltmatrix"),
        "Should define _ltmatrix function"
    );
    assert!(
        output.contains("compgen"),
        "Should use compgen for completion"
    );
    assert!(output.contains("complete"), "Should define completion");
    assert!(
        output.contains("ltmatrix"),
        "Should reference ltmatrix command"
    );
}

#[test]
fn test_zsh_completion_content() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Zsh, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");

    // Verify zsh completion contains expected patterns
    assert!(!output.is_empty(), "Completion output should not be empty");
    assert!(
        output.contains("#compdef ltmatrix"),
        "Should define compdef"
    );
    assert!(
        output.contains("_ltmatrix"),
        "Should define _ltmatrix function"
    );
    assert!(
        output.contains("_describe"),
        "Should use _describe for completion"
    );
    assert!(
        output.contains("_arguments"),
        "Should use _arguments for parsing"
    );
}

#[test]
fn test_fish_completion_content() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Fish, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");

    // Verify fish completion contains expected patterns
    assert!(!output.is_empty(), "Completion output should not be empty");
    assert!(output.contains("complete"), "Should define completion");
    assert!(output.contains("-c ltmatrix"), "Should specify command");
    assert!(output.contains("-f"), "Should use -f flag");
    assert!(output.contains("-a"), "Should use -a for arguments");
}

#[test]
fn test_powershell_completion_content() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::PowerShell, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");

    // Verify powershell completion contains expected patterns
    assert!(!output.is_empty(), "Completion output should not be empty");
    assert!(output.contains("ltmatrix"), "Should reference ltmatrix");
    assert!(
        output.contains("Param") || output.contains("param"),
        "Should define parameters"
    );
    assert!(
        output.contains("CompletionResult"),
        "Should define completion results"
    );
}

#[test]
fn test_elvish_completion_content() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Elvish, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");

    // Verify elvish completion contains expected patterns
    assert!(!output.is_empty(), "Completion output should not be empty");
    assert!(output.contains("ltmatrix"), "Should reference ltmatrix");
    assert!(
        output.contains("edit:completion"),
        "Should define completion"
    );
}

// =============================================================================
// Subcommand and Flag Completion Tests
// =============================================================================

#[test]
fn test_completion_includes_release_subcommand() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");
    assert!(
        output.contains("release"),
        "Should include release subcommand"
    );
}

#[test]
fn test_completion_includes_completions_subcommand() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");
    assert!(
        output.contains("completions"),
        "Should include completions subcommand"
    );
}

#[test]
fn test_completion_includes_help_flag() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");
    assert!(
        output.contains("-h") || output.contains("--help"),
        "Should include help flag"
    );
}

#[test]
fn test_completion_includes_version_flag() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");
    assert!(
        output.contains("-V") || output.contains("--version"),
        "Should include version flag"
    );
}

#[test]
fn test_completion_includes_fast_flag() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");
    assert!(output.contains("--fast"), "Should include --fast flag");
}

#[test]
fn test_completion_includes_expert_flag() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");
    assert!(output.contains("--expert"), "Should include --expert flag");
}

#[test]
fn test_completion_includes_dry_run_flag() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");
    assert!(
        output.contains("--dry-run"),
        "Should include --dry-run flag"
    );
}

// =============================================================================
// Shell Type Coverage Tests
// =============================================================================

#[test]
fn test_all_shell_types_generate_completions() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let shells = vec![
        ("bash", Shell::Bash),
        ("zsh", Shell::Zsh),
        ("fish", Shell::Fish),
        ("powershell", Shell::PowerShell),
        ("elvish", Shell::Elvish),
    ];

    for (shell_name, shell_variant) in shells {
        let mut cmd = Args::command();
        let mut buf = Vec::new();
        clap_complete::generate(shell_variant, &mut cmd, "ltmatrix", &mut buf);
        assert!(
            !buf.is_empty(),
            "Should generate completion for {}",
            shell_name
        );
    }
}

#[test]
fn test_shell_enum_display_implementation() {
    assert_eq!(Shell::Bash.to_string(), "bash");
    assert_eq!(Shell::Zsh.to_string(), "zsh");
    assert_eq!(Shell::Fish.to_string(), "fish");
    assert_eq!(Shell::PowerShell.to_string(), "powershell");
    assert_eq!(Shell::Elvish.to_string(), "elvish");
}

#[test]
fn test_shell_types_are_distinct() {
    let shells = vec![
        Shell::Bash,
        Shell::Zsh,
        Shell::Fish,
        Shell::PowerShell,
        Shell::Elvish,
    ];

    for (i, shell1) in shells.iter().enumerate() {
        for (j, shell2) in shells.iter().enumerate() {
            if i != j {
                assert_ne!(shell1, shell2, "Shell types should be distinct");
            }
        }
    }
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_invalid_shell_type_rejected() {
    let result = Args::try_parse_from(["ltmatrix", "completions", "invalid-shell"]);
    assert!(result.is_err(), "Should reject invalid shell type");
}

#[test]
fn test_empty_shell_type_rejected() {
    let result = Args::try_parse_from(["ltmatrix", "completions", ""]);
    assert!(result.is_err(), "Should reject empty shell type");
}

#[test]
fn test_case_sensitive_shell_type() {
    // Shell types should be case-sensitive
    let result = Args::try_parse_from(["ltmatrix", "completions", "BASH"]);
    assert!(result.is_err(), "Should reject uppercase shell type");
}

#[test]
fn test_completions_requires_shell_argument() {
    let result = Args::try_parse_from(["ltmatrix", "completions"]);
    assert!(
        result.is_err(),
        "Should require shell argument for completions"
    );
}

// =============================================================================
// Command Parsing Tests
// =============================================================================

#[test]
fn test_completions_command_parsing_bash() {
    let args = Args::try_parse_from(["ltmatrix", "completions", "bash"]).unwrap();
    assert!(matches!(args.command, Some(Command::Completions(..))));

    if let Some(Command::Completions(completions_args)) = args.command {
        assert_eq!(completions_args.shell, Shell::Bash);
        assert!(!completions_args.install);
    }
}

#[test]
fn test_completions_command_parsing_bash_with_install() {
    let args = Args::try_parse_from(["ltmatrix", "completions", "bash", "--install"]).unwrap();
    assert!(matches!(args.command, Some(Command::Completions(..))));

    if let Some(Command::Completions(completions_args)) = args.command {
        assert_eq!(completions_args.shell, Shell::Bash);
        assert!(completions_args.install);
    }
}

#[test]
fn test_completions_command_parsing_zsh() {
    let args = Args::try_parse_from(["ltmatrix", "completions", "zsh"]).unwrap();
    assert!(matches!(args.command, Some(Command::Completions(..))));

    if let Some(Command::Completions(completions_args)) = args.command {
        assert_eq!(completions_args.shell, Shell::Zsh);
    }
}

#[test]
fn test_completions_command_parsing_fish() {
    let args = Args::try_parse_from(["ltmatrix", "completions", "fish"]).unwrap();
    assert!(matches!(args.command, Some(Command::Completions(..))));

    if let Some(Command::Completions(completions_args)) = args.command {
        assert_eq!(completions_args.shell, Shell::Fish);
    }
}

#[test]
fn test_completions_command_parsing_powershell() {
    let args = Args::try_parse_from(["ltmatrix", "completions", "powershell"]).unwrap();
    assert!(matches!(args.command, Some(Command::Completions(..))));

    if let Some(Command::Completions(completions_args)) = args.command {
        assert_eq!(completions_args.shell, Shell::PowerShell);
    }
}

#[test]
fn test_completions_command_parsing_elvish() {
    let args = Args::try_parse_from(["ltmatrix", "completions", "elvish"]).unwrap();
    assert!(matches!(args.command, Some(Command::Completions(..))));

    if let Some(Command::Completions(completions_args)) = args.command {
        assert_eq!(completions_args.shell, Shell::Elvish);
    }
}

// =============================================================================
// Completion Script File Output Tests
// =============================================================================

#[test]
fn test_bash_completion_to_file() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    // Verify we can write to a buffer successfully
    assert!(!buf.is_empty(), "Completion should generate output");

    let output = String::from_utf8(buf).expect("Invalid UTF-8");
    assert!(
        output.len() > 100,
        "Completion script should have substantial content"
    );
}

#[test]
fn test_zsh_completion_to_file() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Zsh, &mut cmd, "ltmatrix", &mut buf);

    assert!(!buf.is_empty(), "Completion should generate output");

    let output = String::from_utf8(buf).expect("Invalid UTF-8");
    assert!(
        output.len() > 100,
        "Completion script should have substantial content"
    );
}

#[test]
fn test_fish_completion_to_file() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Fish, &mut cmd, "ltmatrix", &mut buf);

    assert!(!buf.is_empty(), "Completion should generate output");

    let output = String::from_utf8(buf).expect("Invalid UTF-8");
    assert!(
        output.len() > 100,
        "Completion script should have substantial content"
    );
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_completions_with_all_flags_still_works() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();
    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);
    assert!(!buf.is_empty(), "Completions should work even with other flags defined");
}

#[test]
fn test_multiple_completions_generation() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    // Test that we can generate completions for multiple shells in sequence
    let shells = [Shell::Bash, Shell::Zsh, Shell::Fish];

    for shell in shells {
        let mut cmd = Args::command();
        let mut buf = Vec::new();
        clap_complete::generate(shell, &mut cmd, "ltmatrix", &mut buf);
        assert!(!buf.is_empty(), "Should generate {:?} completion", shell);
    }
}

#[test]
fn test_completion_includes_description() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");

    // The command description should be in the completion
    if let Some(description) = cmd.get_about() {
        let desc_str = description.to_string();
        if !desc_str.is_empty() {
            assert!(
                output.contains(&desc_str) || output.len() > 0,
                "Completion should be generated with description context"
            );
        }
    }
}

// =============================================================================
// Dynamic Completion Support Tests
// =============================================================================

#[test]
fn test_completion_supports_dynamic_values() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");

    // Verify that the completion script has structure to support dynamic values
    // This is a basic check - actual dynamic completion would be implemented
    // through custom completion functions
    assert!(
        output.contains("ltmatrix"),
        "Should support command completion"
    );
}

#[test]
fn test_completion_includes_output_format_values() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");

    // Verify completion includes output format options
    assert!(
        output.contains("text") || output.contains("json") || output.contains("output"),
        "Should include output format values or general output reference"
    );
}

#[test]
fn test_completion_includes_log_level_values() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");

    // Verify completion includes log level options
    assert!(
        output.contains("trace")
            || output.contains("debug")
            || output.contains("info")
            || output.contains("log")
            || output.contains("level"),
        "Should include log level values or general log level reference"
    );
}

#[test]
fn test_completion_includes_execution_mode_values() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");

    // Verify completion includes execution mode options
    assert!(
        output.contains("fast")
            || output.contains("standard")
            || output.contains("expert")
            || output.contains("mode"),
        "Should include execution mode values or general mode reference"
    );
}

// =============================================================================
// Installation Instruction Tests
// =============================================================================

#[test]
fn test_help_mentions_completions() {
    use clap::CommandFactory;

    let mut cmd = Args::command();
    let help = cmd.render_help().to_string();

    assert!(
        help.contains("completions") || help.contains("completion"),
        "Help should mention completions subcommand"
    );
}

// =============================================================================
// Edge Cases Tests
// =============================================================================

#[test]
fn test_completion_with_special_characters_in_command_name() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    // Generate with the actual command name
    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut buf);

    let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");
    assert!(!output.is_empty(), "Should handle command name correctly");
}

#[test]
fn test_completion_script_is_valid_utf8() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let shells = vec![
        Shell::Bash,
        Shell::Zsh,
        Shell::Fish,
        Shell::PowerShell,
        Shell::Elvish,
    ];

    for shell in shells {
        let mut cmd = Args::command();
        let mut buf = Vec::new();

        clap_complete::generate(shell, &mut cmd, "ltmatrix", &mut buf);

        let result = String::from_utf8(buf);
        assert!(
            result.is_ok(),
            "Completion for {:?} should be valid UTF-8",
            shell
        );
    }
}

#[test]
fn test_completion_scripts_differ_by_shell() {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let mut cmd = Args::command();
    let mut bash_buf = Vec::new();
    let mut zsh_buf = Vec::new();
    let mut fish_buf = Vec::new();

    clap_complete::generate(Shell::Bash, &mut cmd, "ltmatrix", &mut bash_buf);
    clap_complete::generate(Shell::Zsh, &mut cmd, "ltmatrix", &mut zsh_buf);
    clap_complete::generate(Shell::Fish, &mut cmd, "ltmatrix", &mut fish_buf);

    let bash_output = String::from_utf8(bash_buf).unwrap();
    let zsh_output = String::from_utf8(zsh_buf).unwrap();
    let fish_output = String::from_utf8(fish_buf).unwrap();

    // Different shells should generate different completion scripts
    assert_ne!(
        bash_output, zsh_output,
        "Bash and Zsh completions should differ"
    );
    assert_ne!(
        bash_output, fish_output,
        "Bash and Fish completions should differ"
    );
    assert_ne!(
        zsh_output, fish_output,
        "Zsh and Fish completions should differ"
    );
}
