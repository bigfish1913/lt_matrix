//! CLI command handling
//!
//! This module handles the execution of different CLI commands and subcommands.

use super::args::{Args, Command};
use crate::config::settings::{self, CliOverrides};
use crate::interactive::{analyze_goal_ambiguity, ClarificationRunner, ClarificationSession, NonInteractiveRunner};
use anyhow::{Context, Result};
use tracing::{info, warn};

/// Execute the command specified in the arguments
pub fn execute_command(args: Args) -> Result<()> {
    match args.command {
        Some(Command::Release(ref release_args)) => execute_release(&args, release_args),
        Some(Command::Completions(ref completions_args)) => execute_completions(completions_args),
        Some(Command::Man(ref man_args)) => execute_man(man_args),
        None => {
            // Default to run command
            execute_run(&args)
        }
    }
}

/// Execute the main run logic
fn execute_run(args: &Args) -> Result<()> {
    if let Some(goal) = &args.goal {
        println!("ltmatrix - Long-Time Agent Orchestrator");
        println!("Version: {}", env!("CARGO_PKG_VERSION"));
        println!();

        // Load configuration with CLI overrides
        let overrides = CliOverrides::from(args.clone());
        let config = settings::load_config_with_overrides(Some(overrides))
            .context("Failed to load configuration")?;

        // Display configuration info
        println!("Goal: {}", goal);
        println!("Mode: {}", args.get_execution_mode());

        if let Some(ref agent) = config.default {
            println!("Agent: {}", agent);
        }

        if args.dry_run {
            println!("Dry run: plan will be generated but not executed");
        }

        if args.resume {
            println!("Resume: will continue from last interrupted task");
        }

        // Handle interactive clarification if --ask flag is set
        let enhanced_goal = if args.ask {
            println!("\nInteractive clarification enabled (--ask flag)");
            let enhanced_goal = run_interactive_clarification(goal, args)?;
            enhanced_goal
        } else {
            goal.clone()
        };

        // Continue with actual execution
        println!("\nConfig loaded successfully:");
        println!("  - Default agent: {:?}", config.default);
        println!("  - Log level: {:?}", config.logging.level);
        println!("  - Output format: {:?}", config.output.format);

        if args.ask {
            println!("  - Enhanced goal: {}", enhanced_goal);
        }

        info!("Ready to execute with goal: {}", enhanced_goal);

        // TODO: Continue with pipeline execution using enhanced_goal
        // This is where the actual pipeline (Generate -> Assess -> Execute -> Test -> Verify -> Commit)
        // will be invoked with the clarified goal

    } else {
        print_help();
    }

    Ok(())
}

/// Run interactive clarification session
///
/// If --ask flag is set, this function will:
/// 1. Generate clarification questions based on goal ambiguity
/// 2. Present questions interactively to the user
/// 3. Collect and validate answers
/// 4. Generate an enhanced goal with clarifications
fn run_interactive_clarification(goal: &str, args: &Args) -> Result<String> {
    use crate::interactive::{analyze_goal_ambiguity, ClarificationSession};

    info!("Starting interactive clarification for goal: {}", goal);

    // Generate clarification questions
    let questions = analyze_goal_ambiguity(goal);

    if questions.is_empty() {
        info!("No clarification questions generated - goal appears clear");
        println!("\n✓ Goal is clear enough - no clarification needed");
        return Ok(goal.to_string());
    }

    // Create clarification session
    let mut session = ClarificationSession::new(goal);
    for question in questions {
        session.add_question(question);
    }

    // Run interactive session
    let processed_session = if !args.dry_run && console::user_attended() {
        // Interactive mode
        let runner = ClarificationRunner::new();
        runner.run_clarification(session)?
    } else {
        // Non-interactive mode (dry-run or no terminal)
        if args.dry_run {
            println!("\n[Dry run] Would ask {} clarification questions", session.questions.len());
        }
        NonInteractiveRunner::process_session(session)?
    };

    // Confirm before proceeding
    if !args.dry_run && console::user_attended() {
        let runner = ClarificationRunner::new();
        if !runner.confirm_proceed(&processed_session)? {
            warn!("User cancelled during clarification confirmation");
            anyhow::bail!("Clarification cancelled by user");
        }
    }

    // Generate enhanced goal with clarifications
    let enhanced_goal = enhance_goal_with_clarifications(&processed_session);
    Ok(enhanced_goal)
}

/// Enhance the original goal with clarification answers
fn enhance_goal_with_clarifications(session: &ClarificationSession) -> String {
    if session.answers.is_empty() {
        return session.goal.clone();
    }

    let mut enhanced = session.goal.clone();

    // Append clarifications to the goal
    let clarifications = session.generate_prompt_injection();
    enhanced.push_str(&clarifications);

    enhanced
}

/// Execute the release command
fn execute_release(_args: &Args, release_args: &super::args::ReleaseArgs) -> Result<()> {
    println!("ltmatrix - Release Build");
    println!();

    if release_args.all_targets {
        println!("Building for all target platforms...");
        // TODO: Implement multi-target build
    } else if let Some(target) = &release_args.target {
        println!("Building for target: {}", target);
        // TODO: Implement single target build
    } else {
        println!("Building for host platform...");
        // TODO: Implement host platform build
    }

    if release_args.archive {
        println!("Creating release archives...");
        // TODO: Implement archive creation
    }

    println!("\nOutput directory: {}", release_args.output.display());
    println!("\nTODO: Implement release logic");

    Ok(())
}

/// Execute the completions command
fn execute_completions(completions_args: &super::args::CompletionsArgs) -> Result<()> {
    use clap::CommandFactory;
    use std::io::Write;

    let shell = match completions_args.shell {
        super::args::Shell::Bash => clap_complete::Shell::Bash,
        super::args::Shell::Zsh => clap_complete::Shell::Zsh,
        super::args::Shell::Fish => clap_complete::Shell::Fish,
        super::args::Shell::PowerShell => clap_complete::Shell::PowerShell,
        super::args::Shell::Elvish => clap_complete::Shell::Elvish,
    };

    let mut cmd = Args::command();
    let mut buf = Vec::new();

    clap_complete::generate(shell, &mut cmd, "ltmatrix", &mut buf);

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(&buf)
        .context("Failed to write completions to stdout")?;

    Ok(())
}

/// Execute the man command
fn execute_man(man_args: &super::args::ManArgs) -> Result<()> {
    use std::fs;

    println!("ltmatrix - Man Page Generation");
    println!("Output directory: {}", man_args.output.display());
    println!();

    // Generate man pages
    crate::man::generate_man_pages(&man_args.output).context("Failed to generate man pages")?;

    // List generated files
    let man_entries: Vec<_> = fs::read_dir(&man_args.output)
        .context("Failed to read output directory")?
        .filter_map(|entry| entry.ok())
        .collect();

    if man_entries.is_empty() {
        println!("No man pages were generated!");
    } else {
        println!("Generated man pages:");
        for entry in &man_entries {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            println!("  - {}", name_str);
        }
    }

    println!();
    println!("To view a man page, run:");
    println!("  man {}", man_args.output.join("ltmatrix.1").display());
    println!();
    println!("To install man pages system-wide:");
    println!(
        "  cp {}/*.1 /usr/local/share/man/man1/",
        man_args.output.display()
    );

    Ok(())
}

/// Print help information
fn print_help() {
    println!("ltmatrix - Long-Time Agent Orchestrator");
    println!();
    println!("USAGE:");
    println!("  ltmatrix [OPTIONS] <GOAL>");
    println!("  ltmatrix [SUBCOMMAND]");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help       Print help information");
    println!("  -V, --version    Print version information");
    println!("  --fast           Fast execution mode");
    println!("  --expert         Expert execution mode");
    println!("  --dry-run        Generate plan without execution");
    println!("  --resume         Resume interrupted work");
    println!("  --output <FORMAT> Output format (text, json)");
    println!("  --log-level <LEVEL> Log level (trace, debug, info, warn, error)");
    println!();
    println!("SUBCOMMANDS:");
    println!("  release       Create a release build");
    println!("  completions   Generate shell completions");
    println!("  man           Generate man pages");
    println!();
    println!("EXAMPLES:");
    println!("  ltmatrix \"build a REST API\"");
    println!("  ltmatrix --fast \"add error handling\"");
    println!("  ltmatrix completions bash");
    println!("  ltmatrix man --output ./man");
    println!();
    println!("MAN PAGES:");
    println!("  ltmatrix(1)           Main ltmatrix command");
    println!("  ltmatrix-release(1)   Release subcommand");
    println!("  ltmatrix-completions(1) Completions subcommand");
    println!("  ltmatrix-man(1)       Man page generation subcommand");
    println!();
    println!("For more information, visit: https://github.com/bigfish/ltmatrix");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::ReleaseArgs;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn test_execute_run_command() {
        let args = Args::try_parse_from(["ltmatrix", "test goal"]).unwrap();
        assert!(execute_command(args).is_ok());
    }

    #[test]
    fn test_execute_completions() {
        use clap::CommandFactory;
        use clap_complete::Shell;

        let shell = Shell::Bash;
        let mut cmd = Args::command();
        let mut buf = Vec::new();

        clap_complete::generate(shell, &mut cmd, "ltmatrix", &mut buf);

        // Verify completion script was generated
        let output = String::from_utf8(buf).expect("Invalid UTF-8 in completion output");
        assert!(!output.is_empty(), "Completion output should not be empty");
        assert!(
            output.contains("_ltmatrix"),
            "Completion should define _ltmatrix function"
        );
        assert!(output.contains("compgen"), "Completion should use compgen");
    }

    #[test]
    fn test_execute_release() {
        let release_args = ReleaseArgs {
            target: Some("x86_64-unknown-linux-musl".to_string()),
            output: PathBuf::from("./dist"),
            archive: true,
            all_targets: false,
        };

        let args = Args {
            goal: None,
            agent: None,
            mode: None,
            fast: false,
            expert: false,
            config: None,
            output: None,
            log_level: None,
            log_file: None,
            max_retries: None,
            timeout: None,
            dry_run: false,
            resume: false,
            ask: false,
            regenerate_plan: false,
            on_blocked: None,
            mcp_config: None,
            no_color: false,
            command: Some(Command::Release(release_args)),
        };

        assert!(execute_command(args).is_ok());
    }
}
