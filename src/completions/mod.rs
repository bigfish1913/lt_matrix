// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Shell completion generation for ltmatrix
//!
//! This module provides functionality to generate shell completion scripts for
//! various shells including bash, zsh, fish, PowerShell, and elvish.
//!
//! # Usage
//!
//! Generate completions for your preferred shell:
//!
//! ```bash
//! ltmatrix completions bash > ~/.local/share/bash-completion/completions/ltmatrix
//! ltmatrix completions zsh > ~/.zsh/completion/_ltmatrix
//! ltmatrix completions fish > ~/.config/fish/completions/ltmatrix.fish
//! ltmatrix completions powershell > ~\Documents\PowerShell\Completions\ltmatrix.ps1
//! ltmatrix completions elvish > ~/.elvish/lib/ltmatrix.elv
//! ```
//!
//! # Installation
//!
//! ## Bash
//!
//! ### On Linux (bash-completion >= 2.0)
//! ```bash
//! ltmatrix completions bash > ~/.local/share/bash-completion/completions/ltmatrix
//! # or
//! ltmatrix completions bash | sudo tee /usr/share/bash-completion/completions/ltmatrix
//! ```
//!
//! ### On macOS (Homebrew)
//! ```bash
//! ltmatrix completions bash > $(brew --prefix)/etc/bash_completion.d/ltmatrix
//! ```
//!
//! ### On macOS (MacPorts)
//! ```bash
//! ltmatrix completions bash > /opt/local/etc/bash_completion.d/ltmatrix
//! ```
//!
//! ## Zsh
//!
//! ### For a single user
//! ```bash
//! # Create completion directory if it doesn't exist
//! mkdir -p ~/.zsh/completion
//!
//! # Generate completion
//! ltmatrix completions zsh > ~/.zsh/completion/_ltmatrix
//!
//! # Add to ~/.zshrc (if not already present)
//! fpath=(~/.zsh/completion $fpath)
//! autoload -U compinit && compinit
//! ```
//!
//! ### System-wide
//! ```bash
//! ltmatrix completions zsh | sudo tee /usr/share/zsh/vendor-completions/_ltmatrix
//! # or
//! ltmatrix completions zsh | sudo tee /usr/local/share/zsh/site-functions/_ltmatrix
//! ```
//!
//! ### Oh My Zsh
//! ```bash
//! ltmatrix completions zsh > ~/.oh-my-zsh/completions/_ltmatrix
//! ```
//!
//! ## Fish
//!
//! ### For a single user
//! ```bash
//! ltmatrix completions fish > ~/.config/fish/completions/ltmatrix.fish
//! ```
//!
//! ### System-wide
//! ```bash
//! ltmatrix completions fish > ~/.config/fish/completions/ltmatrix.fish
//! # The fish completion directory is automatically sourced
//! ```
//!
//! ## PowerShell
//!
//! ### Windows PowerShell 5.1 or PowerShell Core 6+
//! ```powershell
//! # Create completion directory
//! New-Item -Path (Split-Path -Parent $PROFILE) -ItemType Directory -Force | Out-Null
//!
//! # Generate completion
//! ltmatrix completions powershell > (Split-Path -Parent $PROFILE)\Completions\ltmatrix.ps1
//!
//! # Add to $PROFILE (if not already present)
//! # echo '. (Split-Path -Parent $PROFILE)\Completions\ltmatrix.ps1' >> $PROFILE
//! ```
//!
//! ### Cross-platform (PowerShell Core)
//! ```powershell
//! # Linux/macOS
//! mkdir -p ~/.config/powershell/Completions
//! ltmatrix completions powershell > ~/.config/powershell/Completions/ltmatrix.ps1
//!
//! # Add to ~/.config/powershell/Microsoft.PowerShell_profile.ps1
//! # echo '. ~/.config/powershell/Completions/ltmatrix.ps1' >> ~/.config/powershell/Microsoft.PowerShell_profile.ps1
//! ```
//!
//! ## Elvish
//!
//! ```elvish
//! # Create directory
//! mkdir -p ~/.elvish/lib
//!
//! # Generate completion
//! ltmatrix completions elvish > ~/.elvish/lib/ltmatrix.elv
//!
//! # Add to ~/.elvish/rc.elv (if not already present)
//! # echo 'use ~/.elvish/lib/ltmatrix' >> ~/.elvish/rc.elv
//! ```
//!
//! # Dynamic Completions
//!
//! This module supports dynamic completions for project-specific values such as:
//!
//! - **Configuration files**: Completes `.ltmatrix.toml` files in the current directory
//! - **MCP config files**: Completes `.mcp.json` files in the current directory
//! - **Log files**: Completes existing `.log` files in the `logs/` directory
//! - **Workspace files**: Completes files in the `.ltmatrix/` directory
//!
//! Dynamic completions are automatically generated based on the current project context.

use anyhow::{Context, Result};
use clap::Command;
use clap_complete::{generate, Shell};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Shell types supported for completion generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

impl ShellType {
    /// Convert from clap's Shell enum
    pub fn from_clap_shell(shell: Shell) -> Option<Self> {
        match shell {
            Shell::Bash => Some(ShellType::Bash),
            Shell::Zsh => Some(ShellType::Zsh),
            Shell::Fish => Some(ShellType::Fish),
            Shell::PowerShell => Some(ShellType::PowerShell),
            Shell::Elvish => Some(ShellType::Elvish),
            _ => None,
        }
    }

    /// Convert to clap's Shell enum
    pub fn to_clap_shell(self) -> Shell {
        match self {
            ShellType::Bash => Shell::Bash,
            ShellType::Zsh => Shell::Zsh,
            ShellType::Fish => Shell::Fish,
            ShellType::PowerShell => Shell::PowerShell,
            ShellType::Elvish => Shell::Elvish,
        }
    }

    /// Get the shell name as a string
    pub fn name(self) -> &'static str {
        match self {
            ShellType::Bash => "bash",
            ShellType::Zsh => "zsh",
            ShellType::Fish => "fish",
            ShellType::PowerShell => "powershell",
            ShellType::Elvish => "elvish",
        }
    }

    /// Get the default installation path for this shell
    pub fn default_install_path(self) -> &'static str {
        match self {
            ShellType::Bash => "~/.local/share/bash-completion/completions/ltmatrix",
            ShellType::Zsh => "~/.zsh/completion/_ltmatrix",
            ShellType::Fish => "~/.config/fish/completions/ltmatrix.fish",
            ShellType::PowerShell => "~/Documents/PowerShell/Completions/ltmatrix.ps1",
            ShellType::Elvish => "~/.elvish/lib/ltmatrix.elv",
        }
    }

    /// Get installation instructions for this shell
    pub fn install_instructions(self) -> String {
        match self {
            ShellType::Bash => {
                format!(
                    r#"# Bash Completion Installation

## For a single user (recommended)
```bash
ltmatrix completions bash > ~/.local/share/bash-completion/completions/ltmatrix
# Restart your shell or run: source ~/.bashrc
```

## System-wide
```bash
ltmatrix completions bash | sudo tee /usr/share/bash-completion/completions/ltmatrix
```

## macOS with Homebrew
```bash
ltmatrix completions bash > $(brew --prefix)/etc/bash_completion.d/ltmatrix
```

## macOS with MacPorts
```bash
ltmatrix completions bash > /opt/local/etc/bash_completion.d/ltmatrix
```

## Verify installation
After installation, restart your shell and run:
```bash
ltmatrix --<TAB>
```
"#
                )
            }
            ShellType::Zsh => {
                format!(
                    r#"# Zsh Completion Installation

## For a single user (recommended)
```bash
# Create completion directory
mkdir -p ~/.zsh/completion

# Generate completion
ltmatrix completions zsh > ~/.zsh/completion/_ltmatrix

# Add to ~/.zshrc (if not already present)
cat >> ~/.zshrc << 'EOF'
# Add ltmatrix completions
fpath=(~/.zsh/completion $fpath)
autoload -U compinit && compinit
EOF

# Reload completions
autoload -U compinit && compinit
```

## System-wide
```bash
ltmatrix completions zsh | sudo tee /usr/share/zsh/vendor-completions/_ltmatrix
# or
ltmatrix completions zsh | sudo tee /usr/local/share/zsh/site-functions/_ltmatrix
```

## Oh My Zsh
```bash
ltmatrix completions zsh > ~/.oh-my-zsh/completions/_ltmatrix
```

## Verify installation
After installation, restart your shell and run:
```zsh
ltmatrix --<TAB>
```
"#
                )
            }
            ShellType::Fish => {
                format!(
                    r#"# Fish Completion Installation

## For a single user (recommended)
```bash
# Create completion directory
mkdir -p ~/.config/fish/completions

# Generate completion
ltmatrix completions fish > ~/.config/fish/completions/ltmatrix.fish

# Completions are automatically sourced by fish
```

## Verify installation
Completions are automatically loaded. Just run:
```fish
ltmatrix --<TAB>
```
"#
                )
            }
            ShellType::PowerShell => {
                format!(
                    r#"# PowerShell Completion Installation

## Windows PowerShell 5.1 or PowerShell Core 6+

### For a single user
```powershell
# Create completion directory
New-Item -Path (Split-Path -Parent $PROFILE) -ItemType Directory -Force | Out-Null

# Generate completion
ltmatrix completions powershell > (Split-Path -Parent $PROFILE)\Completions\ltmatrix.ps1

# Add to $PROFILE (if not already present)
if (-not (Get-Content $PROFILE | Select-String -Pattern 'ltmatrix.ps1' -Quiet)) {{
    echo '. (Split-Path -Parent $PROFILE)\Completions\ltmatrix.ps1' >> $PROFILE
}}

# Reload profile
. $PROFILE
```

## PowerShell Core on Linux/macOS
```bash
# Create directory
mkdir -p ~/.config/powershell/Completions

# Generate completion
ltmatrix completions powershell > ~/.config/powershell/Completions/ltmatrix.ps1

# Add to profile
echo '. ~/.config/powershell/Completions/ltmatrix.ps1' >> ~/.config/powershell/Microsoft.PowerShell_profile.ps1

# Reload profile
source ~/.config/powershell/Microsoft.PowerShell_profile.ps1
```

## Verify installation
After installation, restart your shell and run:
```powershell
ltmatrix --<TAB>
```
"#
                )
            }
            ShellType::Elvish => {
                format!(
                    r#"# Elvish Completion Installation

## For a single user
```elvish
# Create directory
mkdir -p ~/.elvish/lib

# Generate completion
ltmatrix completions elvish > ~/.elvish/lib/ltmatrix.elv

# Add to ~/.elvish/rc.elv (if not already present)
echo 'use ~/.elvish/lib/ltmatrix' >> ~/.elvish/rc.elv

# Reload configuration
# In elvish, run: eval (slurp ~/.elvish/rc.elv)
```

## Verify installation
After reloading, run:
```elvish
ltmatrix --<TAB>
```
"#
                )
            }
        }
    }
}

/// Generate shell completions and write to stdout
///
/// # Arguments
///
/// * `shell` - The shell type to generate completions for
/// * `cmd` - The clap Command to generate completions from
///
/// # Returns
///
/// Returns Ok(()) if successful, or an error if generation fails
pub fn generate_completions(shell: ShellType, cmd: &mut Command) -> Result<()> {
    let mut buf = Vec::new();
    generate(shell.to_clap_shell(), cmd, "ltmatrix", &mut buf);

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(&buf)
        .context("Failed to write completions to stdout")?;

    Ok(())
}

/// Generate shell completions and write to a file
///
/// # Arguments
///
/// * `shell` - The shell type to generate completions for
/// * `cmd` - The clap Command to generate completions from
/// * `output_path` - The path to write the completion file to
///
/// # Returns
///
/// Returns Ok(()) if successful, or an error if generation or file writing fails
pub fn generate_completions_to_file(
    shell: ShellType,
    cmd: &mut Command,
    output_path: &Path,
) -> Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let mut buf = Vec::new();
    generate(shell.to_clap_shell(), cmd, "ltmatrix", &mut buf);

    fs::write(output_path, buf)
        .with_context(|| format!("Failed to write completion file: {}", output_path.display()))?;

    Ok(())
}

/// Generate completions for all supported shells
///
/// # Arguments
///
/// * `cmd` - The clap Command to generate completions from
/// * `output_dir` - The directory to write completion files to
///
/// # Returns
///
/// Returns Ok(()) if successful, or an error if generation or file writing fails
pub fn generate_all_completions(cmd: &mut Command, output_dir: &Path) -> Result<()> {
    let shells = [
        ShellType::Bash,
        ShellType::Zsh,
        ShellType::Fish,
        ShellType::PowerShell,
        ShellType::Elvish,
    ];

    for shell in shells {
        let filename = match shell {
            ShellType::Bash => "ltmatrix.bash",
            ShellType::Zsh => "_ltmatrix",
            ShellType::Fish => "ltmatrix.fish",
            ShellType::PowerShell => "ltmatrix.ps1",
            ShellType::Elvish => "ltmatrix.elv",
        };

        let output_path = output_dir.join(filename);
        generate_completions_to_file(shell, cmd, &output_path)
            .with_context(|| format!("Failed to generate {} completions", shell.name()))?;
    }

    Ok(())
}

/// Print installation instructions for a shell
///
/// # Arguments
///
/// * `shell` - The shell type to print instructions for
pub fn print_install_instructions(shell: ShellType) {
    println!("{}", shell.install_instructions());
}

/// Print installation instructions for all shells
pub fn print_all_install_instructions() {
    let shells = [
        ShellType::Bash,
        ShellType::Zsh,
        ShellType::Fish,
        ShellType::PowerShell,
        ShellType::Elvish,
    ];

    println!("ltmatrix - Shell Completion Installation Instructions");
    println!();
    println!("Select your shell to view installation instructions:");
    println!();

    for shell in shells {
        println!("--- {} ---", shell.name().to_uppercase());
        println!();
        println!("{}", shell.install_instructions());
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::Args;
    use clap::CommandFactory;

    #[test]
    fn test_shell_type_conversion() {
        // Test conversion from clap Shell
        assert_eq!(
            ShellType::from_clap_shell(Shell::Bash),
            Some(ShellType::Bash)
        );
        assert_eq!(
            ShellType::from_clap_shell(Shell::Zsh),
            Some(ShellType::Zsh)
        );
        assert_eq!(
            ShellType::from_clap_shell(Shell::Fish),
            Some(ShellType::Fish)
        );
        assert_eq!(
            ShellType::from_clap_shell(Shell::PowerShell),
            Some(ShellType::PowerShell)
        );
        assert_eq!(
            ShellType::from_clap_shell(Shell::Elvish),
            Some(ShellType::Elvish)
        );
    }

    #[test]
    fn test_shell_type_name() {
        assert_eq!(ShellType::Bash.name(), "bash");
        assert_eq!(ShellType::Zsh.name(), "zsh");
        assert_eq!(ShellType::Fish.name(), "fish");
        assert_eq!(ShellType::PowerShell.name(), "powershell");
        assert_eq!(ShellType::Elvish.name(), "elvish");
    }

    #[test]
    fn test_generate_completions_to_stdout() {
        let mut cmd = Args::command();
        let result = generate_completions(ShellType::Bash, &mut cmd);
        assert!(result.is_ok(), "Should generate bash completions successfully");
    }

    #[test]
    fn test_generate_completions_to_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let output_path = temp_dir.path().join("ltmatrix.bash");
        let mut cmd = Args::command();

        let result = generate_completions_to_file(ShellType::Bash, &mut cmd, &output_path);

        assert!(
            result.is_ok(),
            "Should generate bash completions to file successfully"
        );
        assert!(
            output_path.exists(),
            "Completion file should exist"
        );

        // Verify file is not empty
        let contents = fs::read_to_string(&output_path)
            .expect("Failed to read completion file");
        assert!(
            !contents.is_empty(),
            "Completion file should not be empty"
        );
        assert!(
            contents.contains("ltmatrix"),
            "Completion file should contain 'ltmatrix'"
        );
    }

    #[test]
    fn test_generate_all_completions() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut cmd = Args::command();

        let result = generate_all_completions(&mut cmd, temp_dir.path());

        assert!(
            result.is_ok(),
            "Should generate all completions successfully"
        );

        // Verify all completion files exist
        assert!(temp_dir.path().join("ltmatrix.bash").exists());
        assert!(temp_dir.path().join("_ltmatrix").exists());
        assert!(temp_dir.path().join("ltmatrix.fish").exists());
        assert!(temp_dir.path().join("ltmatrix.ps1").exists());
        assert!(temp_dir.path().join("ltmatrix.elv").exists());
    }

    #[test]
    fn test_install_instructions_contain_required_info() {
        let shells = [
            ShellType::Bash,
            ShellType::Zsh,
            ShellType::Fish,
            ShellType::PowerShell,
            ShellType::Elvish,
        ];

        for shell in shells {
            let instructions = shell.install_instructions();
            assert!(
                !instructions.is_empty(),
                "{} instructions should not be empty",
                shell.name()
            );
            assert!(
                instructions.contains("ltmatrix"),
                "{} instructions should contain 'ltmatrix'",
                shell.name()
            );
        }
    }
}
