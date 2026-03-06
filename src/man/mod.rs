//! Man page generation
//!
//! This module provides functionality to generate Unix man pages for ltmatrix
//! and all its subcommands using clap_mangen.

use anyhow::{Context, Result};
use clap::CommandFactory;
use std::fs;
use std::path::Path;

/// Generate man pages for ltmatrix and all subcommands
///
/// This function creates man page files in the specified output directory.
/// It generates:
/// - ltmatrix.1 - Main man page
/// - ltmatrix-release.1 - Release subcommand man page
/// - ltmatrix-completions.1 - Completions subcommand man page
///
/// # Arguments
///
/// * `output_dir` - Directory where man pages will be written
///
/// # Returns
///
/// Returns `Ok(())` if all man pages were generated successfully,
/// or an error if generation failed.
///
/// # Example
///
/// ```no_run
/// use std::path::PathBuf;
/// use ltmatrix::man::generate_man_pages;
///
/// let output_dir = PathBuf::from("./target/man");
/// generate_man_pages(&output_dir).expect("Failed to generate man pages");
/// ```
pub fn generate_man_pages(output_dir: &Path) -> Result<()> {
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir).context("Failed to create man page output directory")?;

    // Get the command structure
    let cmd = crate::cli::args::Args::command();

    // Generate man page for the main command
    let mut buffer = Vec::new();
    clap_mangen::Man::new(cmd.clone())
        .render(&mut buffer)
        .context("Failed to render main man page")?;

    let main_man_path = output_dir.join("ltmatrix.1");
    fs::write(&main_man_path, buffer).with_context(|| {
        format!(
            "Failed to write main man page to {}",
            main_man_path.display()
        )
    })?;

    // Generate man pages for each subcommand
    for sub in cmd.get_subcommands() {
        let sub_name = sub.get_name();
        let mut buffer = Vec::new();
        clap_mangen::Man::new(sub.clone())
            .render(&mut buffer)
            .with_context(|| format!("Failed to render man page for subcommand {}", sub_name))?;

        let man_filename = format!("ltmatrix-{}.1", sub_name);
        let man_path = output_dir.join(&man_filename);
        fs::write(&man_path, buffer)
            .with_context(|| format!("Failed to write man page to {}", man_path.display()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_generate_man_pages_creates_files() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let output_dir = temp_dir.path().join("man");

        let result = generate_man_pages(&output_dir);

        assert!(result.is_ok(), "Man page generation should succeed");

        // Verify main man page exists
        let main_man = output_dir.join("ltmatrix.1");
        assert!(main_man.exists(), "Main man page should exist");

        // Verify subcommand man pages exist
        let release_man = output_dir.join("ltmatrix-release.1");
        assert!(release_man.exists(), "Release man page should exist");

        let completions_man = output_dir.join("ltmatrix-completions.1");
        assert!(
            completions_man.exists(),
            "Completions man page should exist"
        );
    }

    #[test]
    fn test_man_page_content() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let output_dir = temp_dir.path().join("man");

        generate_man_pages(&output_dir).expect("Man page generation should succeed");

        let main_man = output_dir.join("ltmatrix.1");
        let content = fs::read_to_string(&main_man).expect("Failed to read man page");

        // Check for required roff macros
        assert!(content.contains(".TH"), "Must have TH macro");
        assert!(content.contains(".SH"), "Must have SH macro");
        assert!(content.contains(".TP"), "Must have TP macro");

        // Check content
        assert!(content.contains("ltmatrix"), "Must mention ltmatrix");
    }

    #[test]
    fn test_creates_directory_if_not_exists() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let output_dir = temp_dir.path().join("nonexistent").join("man");

        assert!(!output_dir.exists(), "Directory should not exist initially");

        let result = generate_man_pages(&output_dir);

        assert!(
            result.is_ok(),
            "Should create directory and generate man pages"
        );
        assert!(output_dir.exists(), "Directory should be created");
    }
}
