// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! CLI command handling
//!
//! This module handles the execution of different CLI commands and subcommands.

use super::args::{Args, CleanupArgs, Command, MemoryArgs};
use crate::interactive::{ClarificationRunner, ClarificationSession, NonInteractiveRunner};
use crate::workspace::WorkspaceState;
use anyhow::{Context, Result};
use ltmatrix_config::settings::{self, CliOverrides};
use std::path::PathBuf;
use tracing::{info, warn};

/// Execute the command specified in the arguments
pub fn execute_command(args: Args) -> Result<()> {
    match args.command {
        Some(Command::Release(ref release_args)) => execute_release(&args, release_args),
        Some(Command::Completions(ref completions_args)) => execute_completions(completions_args),
        Some(Command::Man(ref man_args)) => execute_man(man_args),
        Some(Command::Cleanup(ref cleanup_args)) => execute_cleanup(cleanup_args),
        Some(Command::Memory(ref memory_args)) => execute_memory(memory_args),
        None => {
            // Default to run command
            execute_run(&args)
        }
    }
}

/// Execute the main run logic
fn execute_run(args: &Args) -> Result<()> {
    use std::env;

    println!("ltmatrix - Long-Time Agent Orchestrator");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    // Handle resume mode
    if args.resume {
        return execute_resume(args);
    }

    // Regular execution requires a goal
    if let Some(goal) = &args.goal {
        // Load configuration with CLI overrides
        let overrides = args.to_overrides();
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

/// Execute resume mode - load and continue from previous workspace state
fn execute_resume(args: &Args) -> Result<()> {
    use std::env;

    let project_root = env::current_dir().context("Failed to get current directory")?;

    println!("Resume Mode: Continuing from previous workspace state");
    println!();

    // Check if workspace state exists
    if !WorkspaceState::exists(&project_root) {
        println!("No workspace state found in current directory.");
        println!();
        println!(
            "Expected file: {}",
            project_root
                .join(".ltmatrix")
                .join("tasks-manifest.json")
                .display()
        );
        println!();
        println!("To start a new task, run: ltmatrix \"your goal\"");
        println!("To clean up any residual state, run: ltmatrix cleanup --remove --force");
        return Ok(());
    }

    // Load workspace state with transformation (resets in_progress/blocked to pending)
    let state = WorkspaceState::load_with_transform(project_root.clone())
        .context("Failed to load workspace state")?;

    // Display current workspace status
    let summary = state.status_summary();
    println!("Workspace Status:");
    println!("  Total tasks: {}", summary.total());
    println!("  Completed: {}", summary.completed);
    println!("  In Progress: {}", summary.in_progress);
    println!("  Pending: {}", summary.pending);
    println!("  Failed: {}", summary.failed);
    println!("  Blocked: {}", summary.blocked);
    println!("  Progress: {:.1}%", summary.completion_percentage());
    println!();

    // Check if all tasks are completed
    if summary.pending == 0 && summary.in_progress == 0 && summary.failed == 0 {
        println!("All tasks are already completed!");
        println!();
        println!("To start fresh, run: ltmatrix cleanup --remove --force");
        return Ok(());
    }

    // Display pending/failed tasks
    println!("Remaining work:");
    if summary.pending > 0 {
        println!("  - {} task(s) pending", summary.pending);
    }
    if summary.failed > 0 {
        println!("  - {} task(s) failed (will retry)", summary.failed);
    }
    println!();

    // Load configuration
    let overrides = args.to_overrides();
    let config = settings::load_config_with_overrides(Some(overrides))
        .context("Failed to load configuration")?;

    println!("Configuration:");
    println!("  - Mode: {}", args.get_execution_mode());
    if let Some(ref agent) = config.default {
        println!("  - Agent: {}", agent);
    }
    println!();

    if args.dry_run {
        println!("DRY RUN - Would resume execution with the above configuration");
        println!("Remove --dry-run to actually execute");
    } else {
        println!("Resuming execution...");
        // TODO: Continue with pipeline execution using loaded state
        // This is where the actual pipeline execution would continue
        info!("Loaded workspace state with {} tasks", state.tasks.len());
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
            println!(
                "\n[Dry run] Would ask {} clarification questions",
                session.questions.len()
            );
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
pub fn execute_release(_args: &Args, release_args: &super::args::ReleaseArgs) -> Result<()> {
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
pub fn execute_completions(completions_args: &super::args::CompletionsArgs) -> Result<()> {
    use clap::CommandFactory;

    let shell = match completions_args.shell {
        super::args::Shell::Bash => crate::completions::ShellType::Bash,
        super::args::Shell::Zsh => crate::completions::ShellType::Zsh,
        super::args::Shell::Fish => crate::completions::ShellType::Fish,
        super::args::Shell::PowerShell => crate::completions::ShellType::PowerShell,
        super::args::Shell::Elvish => crate::completions::ShellType::Elvish,
    };

    // If --install flag is provided, print installation instructions
    if completions_args.install {
        println!("ltmatrix - Shell Completion Installation");
        println!();
        println!("Installing completions for: {}", shell.name());
        println!();

        crate::completions::print_install_instructions(shell);
        println!();
        println!("---");
        println!();
        println!("Quick install command:");
        println!(
            "  ltmatrix completions {} > {}",
            shell.name(),
            shell.default_install_path()
        );
        println!();
    } else {
        // Generate completions to stdout
        let mut cmd = Args::command();
        crate::completions::generate_completions(shell, &mut cmd)?;

        // Print installation instructions after completion
        if console::user_attended() {
            eprintln!();
            eprintln!("✓ Completion script generated successfully");
            eprintln!();
            eprintln!("To install completions for {}, run:", shell.name());
            eprintln!("  ltmatrix completions {} --install", shell.name());
            eprintln!();
        }
    }

    Ok(())
}

/// Execute the man command
pub fn execute_man(man_args: &super::args::ManArgs) -> Result<()> {
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

/// Execute the cleanup command
pub fn execute_cleanup(args: &CleanupArgs) -> Result<()> {
    use std::env;

    println!("ltmatrix - Workspace Cleanup");
    println!();

    // Get current directory as project root
    let project_root = env::current_dir().context("Failed to get current directory")?;

    // Check if workspace state exists
    if !WorkspaceState::exists(&project_root) {
        println!("No workspace state found in current directory.");
        println!(
            "Expected file: {}",
            project_root
                .join(".ltmatrix")
                .join("tasks-manifest.json")
                .display()
        );
        return Ok(());
    }

    // Show current state summary
    match WorkspaceState::load(project_root.clone()) {
        Ok(state) => {
            let summary = state.status_summary();
            println!("Current workspace state:");
            println!("  Total tasks: {}", summary.total());
            println!("  Completed: {}", summary.completed);
            println!("  In Progress: {}", summary.in_progress);
            println!("  Pending: {}", summary.pending);
            println!("  Failed: {}", summary.failed);
            println!("  Blocked: {}", summary.blocked);
            println!("  Progress: {:.1}%", summary.completion_percentage());
            println!();
        }
        Err(e) => {
            println!("Warning: Could not load workspace state: {}", e);
            println!();
        }
    }

    // Handle different cleanup modes
    if args.remove {
        // Remove all workspace state
        println!("Action: Remove all workspace state files");
        println!();

        if args.dry_run {
            println!("DRY RUN - Would remove:");
            println!("  {}", project_root.join(".ltmatrix").display());
            println!();
            println!("Use --force to actually perform the cleanup");
        } else if !args.force {
            println!("This will remove all workspace state files including:");
            println!(
                "  - {}",
                project_root
                    .join(".ltmatrix")
                    .join("tasks-manifest.json")
                    .display()
            );
            println!("  - Any other files in .ltmatrix directory");
            println!();
            println!("Are you sure you want to continue? (yes/no): ");

            // In a real implementation, you'd read user input here
            // For now, we'll require --force flag
            println!("Please use --force flag to confirm cleanup");
            return Ok(());
        } else {
            WorkspaceState::cleanup(&project_root).context("Failed to cleanup workspace state")?;
            println!("✓ Workspace state removed successfully");
        }
    } else if args.reset_all {
        // Reset all tasks to pending
        println!("Action: Reset all tasks to pending status");
        println!();

        if args.dry_run {
            println!("DRY RUN - Would reset all tasks to pending");
            println!("Use --force to actually perform the reset");
        } else {
            let mut state =
                WorkspaceState::load(project_root).context("Failed to load workspace state")?;
            state.reset_all().context("Failed to reset tasks")?;
            state.save().context("Failed to save workspace state")?;
            println!("✓ All tasks reset to pending status");
        }
    } else if args.reset_failed {
        // Reset only failed tasks
        println!("Action: Reset failed tasks to pending status");
        println!();

        if args.dry_run {
            println!("DRY RUN - Would reset failed tasks");
            println!("Use --force to actually perform the reset");
        } else {
            let mut state =
                WorkspaceState::load(project_root).context("Failed to load workspace state")?;
            let count = state
                .reset_failed()
                .context("Failed to reset failed tasks")?;
            state.save().context("Failed to save workspace state")?;
            println!("✓ Reset {} failed task(s) to pending status", count);
        }
    } else {
        // Default: just show status
        println!("No cleanup action specified. Use one of:");
        println!("  --remove       Remove all workspace state files");
        println!("  --reset-all    Reset all tasks to pending");
        println!("  --reset-failed Reset only failed tasks to pending");
        println!();
        println!("Use --force to confirm the action");
        println!("Use --dry-run to preview changes");
    }

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
    println!("  cleanup       Clean up workspace state");
    println!();
    println!("EXAMPLES:");
    println!("  ltmatrix \"build a REST API\"");
    println!("  ltmatrix --fast \"add error handling\"");
    println!("  ltmatrix --resume");
    println!("  ltmatrix cleanup --remove --force");
    println!("  ltmatrix cleanup --reset-failed");
    println!("  ltmatrix completions bash");
    println!("  ltmatrix man --output ./man");
    println!();
    println!("MAN PAGES:");
    println!("  ltmatrix(1)           Main ltmatrix command");
    println!("  ltmatrix-release(1)   Release subcommand");
    println!("  ltmatrix-completions(1) Completions subcommand");
    println!("  ltmatrix-man(1)       Man page generation subcommand");
    println!("  ltmatrix-cleanup(1)   Cleanup subcommand");
    println!();
    println!("For more information, visit: https://github.com/bigfish/ltmatrix");
}

/// Execute the memory subcommand
pub fn execute_memory(args: &MemoryArgs) -> Result<()> {
    use super::args::MemoryAction;

    match &args.action {
        MemoryAction::Summarize(summarize_args) => execute_memory_summarize(summarize_args),
        MemoryAction::Status(status_args) => execute_memory_status(status_args),
        MemoryAction::Clear(clear_args) => execute_memory_clear(clear_args),
    }
}

/// Execute memory summarize subcommand
fn execute_memory_summarize(args: &super::args::MemorySummarizeArgs) -> Result<()> {
    use crate::memory::MemoryStore;
    use ltmatrix_config::settings::MemoryConfig;
    use std::env;

    println!("ltmatrix - Memory Summarization");
    println!();

    // Get project root
    let project_root = args
        .project
        .clone()
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    println!("Project root: {}", project_root.display());
    println!();

    // Create memory store with custom config
    let mut config = MemoryConfig::default();
    if let Some(keep_fraction) = args.keep_fraction {
        if keep_fraction <= 0.0 || keep_fraction > 1.0 {
            anyhow::bail!("keep-fraction must be between 0.0 (exclusive) and 1.0 (inclusive)");
        }
        config.keep_fraction = keep_fraction;
    }

    let store = MemoryStore::with_config(&project_root, config.clone())
        .context("Failed to create memory store")?;

    let entry_count = store.entry_count();
    println!("Current entries: {}", entry_count);

    if entry_count == 0 {
        println!("No memory entries to summarize.");
        return Ok(());
    }

    // Check if summarization is needed
    let memory_file = project_root.join(".claude/memory.md");
    let file_size = if memory_file.exists() {
        std::fs::metadata(&memory_file)
            .map(|m| m.len() as usize)
            .unwrap_or(0)
    } else {
        0
    };

    println!("Memory file size: {} bytes", file_size);
    println!();

    // Determine if we should summarize
    let needs_summarization =
        args.force || file_size > config.max_file_size || entry_count > config.max_entries;

    if !needs_summarization {
        println!("Memory is within configured thresholds.");
        println!("  Max file size: {} bytes", config.max_file_size);
        println!("  Max entries: {}", config.max_entries);
        println!();
        if !args.force {
            println!("Use --force to summarize anyway.");
            return Ok(());
        }
    }

    if args.dry_run {
        println!("DRY RUN - Would summarize memory:");
        println!("  Entries to process: {}", entry_count);
        println!("  Keep fraction: {:.0}%", config.keep_fraction * 100.0);
        println!(
            "  Estimated entries to keep: {}",
            (entry_count as f64 * config.keep_fraction) as usize
        );
    } else {
        println!("Summarizing memory...");
        println!("  Keep fraction: {:.0}%", config.keep_fraction * 100.0);

        // Force summarization by triggering it with a modified config
        let mut force_config = MemoryConfig::default();
        force_config.max_file_size = 1;
        force_config.max_entries = 1;
        force_config.min_entries_for_summarization = 1;

        let _ = MemoryStore::with_config(&project_root, force_config)
            .context("Failed to summarize memory")?;

        println!();
        println!("✓ Memory summarized successfully");

        // Show new state
        let new_store = MemoryStore::new(&project_root)?;
        println!("  New entry count: {}", new_store.entry_count());
    }

    Ok(())
}

/// Execute memory status subcommand
fn execute_memory_status(args: &super::args::MemoryStatusArgs) -> Result<()> {
    use crate::memory::MemoryStore;
    use ltmatrix_config::settings::MemoryConfig;
    use std::env;

    println!("ltmatrix - Memory Status");
    println!();

    // Get project root
    let project_root = args
        .project
        .clone()
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let memory_file = project_root.join(".claude/memory.md");

    // Check if memory file exists
    if !memory_file.exists() {
        if args.json {
            println!(
                "{}",
                serde_json::json!({
                    "exists": false,
                    "project_root": project_root.display().to_string()
                })
            );
        } else {
            println!("No memory file found at: {}", memory_file.display());
            println!();
            println!("Memory will be created automatically when tasks complete.");
        }
        return Ok(());
    }

    // Load memory store
    let store = MemoryStore::new(&project_root).context("Failed to load memory store")?;

    let entries = store.get_entries();
    let entry_count = entries.len();

    // Get file size
    let file_size = std::fs::metadata(&memory_file)
        .map(|m| m.len())
        .unwrap_or(0);

    // Count by category
    let mut by_category: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for entry in &entries {
        *by_category.entry(entry.category.to_string()).or_insert(0) += 1;
    }

    // Count by priority
    let mut by_priority: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for entry in &entries {
        *by_priority
            .entry(format!("{:?}", entry.priority))
            .or_insert(0) += 1;
    }

    // Count deprecated entries
    let deprecated_count = entries.iter().filter(|e| e.deprecated).count();

    if args.json {
        let status = serde_json::json!({
            "exists": true,
            "project_root": project_root.display().to_string(),
            "memory_file": memory_file.display().to_string(),
            "file_size_bytes": file_size,
            "entry_count": entry_count,
            "by_category": by_category,
            "by_priority": by_priority,
            "deprecated_count": deprecated_count,
            "config": {
                "max_file_size": MemoryConfig::default().max_file_size,
                "max_entries": MemoryConfig::default().max_entries,
            }
        });
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        println!("Memory file: {}", memory_file.display());
        println!();
        println!("Statistics:");
        println!("  Total entries: {}", entry_count);
        println!("  File size: {} bytes", file_size);
        println!("  Deprecated entries: {}", deprecated_count);
        println!();

        if !by_category.is_empty() {
            println!("By category:");
            let mut categories: Vec<_> = by_category.iter().collect();
            categories.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
            for (category, count) in categories {
                println!("  {}: {}", category, count);
            }
            println!();
        }

        if !by_priority.is_empty() {
            println!("By priority:");
            let mut priorities: Vec<_> = by_priority.iter().collect();
            priorities.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
            for (priority, count) in priorities {
                println!("  {}: {}", priority, count);
            }
            println!();
        }

        // Show configuration thresholds
        let config = MemoryConfig::default();
        println!("Configuration:");
        println!("  Max file size: {} bytes", config.max_file_size);
        println!("  Max entries: {}", config.max_entries);
        println!();

        // Show summarization status
        let needs_summarization =
            file_size as usize > config.max_file_size || entry_count > config.max_entries;

        if needs_summarization {
            println!("⚠ Memory exceeds configured thresholds.");
            println!("  Run 'ltmatrix memory summarize' to reduce size.");
        } else {
            println!("✓ Memory is within configured thresholds.");
        }
    }

    Ok(())
}

/// Execute memory clear subcommand
fn execute_memory_clear(args: &super::args::MemoryClearArgs) -> Result<()> {
    use crate::memory::MemoryStore;
    use std::env;

    println!("ltmatrix - Memory Clear");
    println!();

    // Get project root
    let project_root = args
        .project
        .clone()
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let memory_file = project_root.join(".claude/memory.md");

    // Check if memory file exists
    if !memory_file.exists() {
        println!("No memory file found at: {}", memory_file.display());
        return Ok(());
    }

    // Load store to get entry count
    let store = MemoryStore::new(&project_root).context("Failed to load memory store")?;

    let entry_count = store.entry_count();
    println!("Memory file: {}", memory_file.display());
    println!("Entries to clear: {}", entry_count);
    println!();

    // Confirm unless --force
    if !args.force {
        if console::user_attended() {
            use dialoguer::Confirm;
            let confirm = Confirm::new()
                .with_prompt("Are you sure you want to clear all memory entries?")
                .default(false)
                .interact()?;

            if !confirm {
                println!("Cancelled.");
                return Ok(());
            }
        } else {
            println!("Use --force to clear memory without confirmation.");
            return Ok(());
        }
    }

    // Remove the memory file
    std::fs::remove_file(&memory_file).context("Failed to remove memory file")?;

    // Remove the .claude directory if empty
    if let Some(parent) = memory_file.parent() {
        let _ = std::fs::remove_dir(parent); // Ignore error if not empty
    }

    println!(
        "✓ Memory cleared successfully ({} entries removed)",
        entry_count
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::{CompletionsArgs, ReleaseArgs, Shell};
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn test_execute_run_command() {
        let args = Args::try_parse_from(["ltmatrix", "test goal"]).unwrap();
        assert!(execute_command(args).is_ok());
    }

    #[test]
    fn test_execute_completions() {
        let completions_args = CompletionsArgs {
            shell: Shell::Bash,
            install: false,
        };

        let result = execute_completions(&completions_args);
        assert!(
            result.is_ok(),
            "Should execute completions command successfully"
        );
    }

    #[test]
    fn test_execute_completions_with_install() {
        let completions_args = CompletionsArgs {
            shell: Shell::Bash,
            install: true,
        };

        let result = execute_completions(&completions_args);
        assert!(
            result.is_ok(),
            "Should execute completions with install flag successfully"
        );
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
            file: None,
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
            telemetry: false,
            command: Some(Command::Release(release_args)),
        };

        assert!(execute_command(args).is_ok());
    }
}
