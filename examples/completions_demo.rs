//! Shell Completions Demo
//!
//! This example demonstrates how to generate and use shell completions for ltmatrix.
//!
//! # Usage
//!
//! Run this example to see how completions are generated:
//! ```bash
//! cargo run --example completions_demo
//! ```
//!
//! # Shell Completion Examples
//!
//! ## Bash
//! ```bash
//! ltmatrix completions bash > ~/.local/share/bash-completion/completions/ltmatrix
//! ```
//!
//! ## Zsh
//! ```bash
//! ltmatrix completions zsh > ~/.zsh/completion/_ltmatrix
//! ```
//!
//! ## Fish
//! ```bash
//! ltmatrix completions fish > ~/.config/fish/completions/ltmatrix.fish
//! ```
//!
//! ## PowerShell
//! ```powershell
//! ltmatrix completions powershell > ~/Documents/PowerShell/Completions/ltmatrix.ps1
//! ```
//!
//! ## Elvish
//! ```bash
//! ltmatrix completions elvish > ~/.elvish/lib/ltmatrix.elv
//! ```

use clap::CommandFactory;
use ltmatrix::cli::args::Args;
use ltmatrix::completions::{generate_completions, ShellType};

fn main() {
    println!("ltmatrix Shell Completions Demo");
    println!("================================\n");

    // Demonstrate completion generation for each shell
    let shells = [
        ShellType::Bash,
        ShellType::Zsh,
        ShellType::Fish,
        ShellType::PowerShell,
        ShellType::Elvish,
    ];

    println!("Supported shells:");
    for shell in &shells {
        println!("  - {}", shell.name());
    }
    println!();

    // Show installation instructions for each shell
    println!("Installation Instructions:\n");

    for shell in &shells {
        println!("--- {} ---", shell.name().to_uppercase());
        println!("Default path: {}", shell.default_install_path());
        println!();

        // Generate completion to stdout (for demo purposes, we'll just show success)
        let mut cmd = Args::command();
        match generate_completions(*shell, &mut cmd) {
            Ok(_) => println!("✓ Completion generated successfully"),
            Err(e) => println!("✗ Failed: {}", e),
        }
        println!();
    }

    println!("Usage Examples:");
    println!();
    println!("1. Generate completion for Bash:");
    println!(
        "   $ ltmatrix completions bash > ~/.local/share/bash-completion/completions/ltmatrix"
    );
    println!();
    println!("2. Generate completion with installation instructions:");
    println!("   $ ltmatrix completions zsh --install");
    println!();
    println!("3. Use completion in interactive shell:");
    println!("   $ ltmatrix --<TAB>  # Shows available options");
    println!("   $ ltmatrix c<TAB>  # Shows completions subcommand");
    println!("   $ ltmatrix completions <TAB>  # Shows available shells");
    println!();
}
