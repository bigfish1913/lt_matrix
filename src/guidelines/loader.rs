// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Guidelines loader
//!
//! Provides functionality for loading guidelines from the filesystem.

use anyhow::{bail, Result};
use ltmatrix_core::AgentType;
use std::fs;
use std::path::PathBuf;

/// Loaded guidelines content for each agent type
#[derive(Debug, Clone, Default)]
pub struct Guidelines {
    /// Common guidelines shared by all agent types
    pub common: Option<String>,

    /// Guidelines for Plan agents
    pub plan: Option<String>,

    /// Guidelines for Dev agents
    pub dev: Option<String>,

    /// Guidelines for Test agents
    pub test: Option<String>,

    /// Guidelines for Review agents
    pub review: Option<String>,
}

impl Guidelines {
    /// Creates an empty guidelines structure
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets guidelines for a specific agent type
    pub fn get_for_agent_type(&self, agent_type: AgentType) -> Option<String> {
        let specific = match agent_type {
            AgentType::Plan => self.plan.as_ref(),
            AgentType::Dev => self.dev.as_ref(),
            AgentType::Test => self.test.as_ref(),
            AgentType::Review => self.review.as_ref(),
        };

        // Combine common + specific guidelines
        match (&self.common, specific) {
            (Some(common), Some(specific)) => Some(format!("{}\n\n{}", common, specific)),
            (Some(common), None) => Some(common.clone()),
            (None, Some(specific)) => Some(specific.clone()),
            (None, None) => None,
        }
    }

    /// Checks if any guidelines are loaded
    pub fn is_empty(&self) -> bool {
        self.common.is_none()
            && self.plan.is_none()
            && self.dev.is_none()
            && self.test.is_none()
            && self.review.is_none()
    }
}

/// Loader for project guidelines
///
/// This struct handles loading guidelines from the filesystem.
/// It supports both:
/// - Directory structure: `.ltmatrix/guidelines/{_common,plan,dev,test,review}.md`
/// - Single file: `.ltmatrix/guidelines.md`
#[derive(Debug, Clone)]
pub struct GuidelinesLoader {
    /// Path to the guidelines directory or file
    guidelines_path: PathBuf,

    /// Whether to use single file mode
    single_file: bool,
}

impl GuidelinesLoader {
    /// Creates a new guidelines loader
    ///
    /// # Arguments
    ///
    /// * `guidelines_path` - Path to the guidelines directory or single file
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::guidelines::GuidelinesLoader;
    /// use std::path::PathBuf;
    ///
    /// let loader = GuidelinesLoader::new(PathBuf::from(".ltmatrix/guidelines"));
    /// ```
    pub fn new(guidelines_path: PathBuf) -> Self {
        // Check if the path points to a file (single file mode)
        let single_file = guidelines_path
            .extension()
            .map(|ext| ext == "md")
            .unwrap_or(false);

        GuidelinesLoader {
            guidelines_path,
            single_file,
        }
    }

    /// Creates a loader with default path
    ///
    /// Uses `.ltmatrix/guidelines` as the default guidelines directory.
    pub fn default_loader(project_root: &PathBuf) -> Self {
        Self::new(project_root.join(".ltmatrix").join("guidelines"))
    }

    /// Loads all guidelines
    ///
    /// # Returns
    ///
    /// Returns the loaded guidelines, or an empty structure if no guidelines exist.
    pub fn load(&self) -> Result<Guidelines> {
        if self.single_file {
            self.load_single_file()
        } else {
            self.load_directory()
        }
    }

    /// Loads guidelines from a single file
    ///
    /// The single file is expected to have sections marked with headers:
    /// - `# Common` or `# General` for shared guidelines
    /// - `# Plan` for plan agent guidelines
    /// - `# Dev` or `# Development` for dev agent guidelines
    /// - `# Test` or `# Testing` for test agent guidelines
    /// - `# Review` for review agent guidelines
    fn load_single_file(&self) -> Result<Guidelines> {
        if !self.guidelines_path.exists() {
            return Ok(Guidelines::new());
        }

        let content = fs::read_to_string(&self.guidelines_path)?;
        let mut guidelines = Guidelines::new();

        // Parse sections from the single file
        let sections = parse_sections(&content);

        for (section_name, section_content) in sections {
            let content = section_content.trim().to_string();
            if content.is_empty() {
                continue;
            }

            match section_name.to_lowercase().as_str() {
                "common" | "general" | "_common" => {
                    guidelines.common = Some(content);
                }
                "plan" | "planning" => {
                    guidelines.plan = Some(content);
                }
                "dev" | "development" => {
                    guidelines.dev = Some(content);
                }
                "test" | "testing" => {
                    guidelines.test = Some(content);
                }
                "review" => {
                    guidelines.review = Some(content);
                }
                _ => {}
            }
        }

        Ok(guidelines)
    }

    /// Loads guidelines from a directory structure
    fn load_directory(&self) -> Result<Guidelines> {
        let mut guidelines = Guidelines::new();

        if !self.guidelines_path.exists() {
            return Ok(guidelines);
        }

        if !self.guidelines_path.is_dir() {
            // If it's not a directory, try loading as single file
            return self.load_single_file();
        }

        // Load each guideline file
        guidelines.common = self.load_file("_common.md");
        guidelines.plan = self.load_file("plan.md");
        guidelines.dev = self.load_file("dev.md");
        guidelines.test = self.load_file("test.md");
        guidelines.review = self.load_file("review.md");

        Ok(guidelines)
    }

    /// Loads a single guideline file
    fn load_file(&self, filename: &str) -> Option<String> {
        let path = self.guidelines_path.join(filename);

        if !path.exists() {
            return None;
        }

        match fs::read_to_string(&path) {
            Ok(content) => {
                let trimmed = content.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            }
            Err(e) => {
                tracing::warn!("Failed to load guideline file {:?}: {}", path, e);
                None
            }
        }
    }

    /// Loads guidelines for a specific agent type
    ///
    /// This is a convenience method that loads all guidelines and
    /// returns only the relevant ones for the given agent type.
    pub fn load_for_agent_type(&self, agent_type: AgentType) -> Result<Option<String>> {
        let guidelines = self.load()?;
        Ok(guidelines.get_for_agent_type(agent_type))
    }

    /// Checks if guidelines exist
    pub fn exists(&self) -> bool {
        if self.single_file {
            self.guidelines_path.exists()
        } else {
            self.guidelines_path.exists() && self.guidelines_path.is_dir()
        }
    }
}

/// Parses sections from a single guidelines file
///
/// Sections are identified by markdown headers (e.g., `# Plan`, `# Dev`).
fn parse_sections(content: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut current_section: Option<String> = None;
    let mut current_content = String::new();

    for line in content.lines() {
        // Check for section headers
        if line.starts_with("# ") {
            // Save previous section
            if let Some(section) = current_section.take() {
                sections.push((section, current_content.trim().to_string()));
                current_content = String::new();
            }

            // Start new section
            current_section = Some(line[2..].trim().to_string());
        } else if current_section.is_some() {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
        }
    }

    // Save last section
    if let Some(section) = current_section {
        sections.push((section, current_content.trim().to_string()));
    }

    sections
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_guidelines_get_for_agent_type() {
        let mut guidelines = Guidelines::new();
        guidelines.common = Some("Common guideline".to_string());
        guidelines.dev = Some("Dev guideline".to_string());

        let dev_guidelines = guidelines.get_for_agent_type(AgentType::Dev);
        assert!(dev_guidelines.is_some());
        let content = dev_guidelines.unwrap();
        assert!(content.contains("Common guideline"));
        assert!(content.contains("Dev guideline"));

        let plan_guidelines = guidelines.get_for_agent_type(AgentType::Plan);
        assert!(plan_guidelines.is_some());
        assert!(plan_guidelines.unwrap().contains("Common guideline"));
    }

    #[test]
    fn test_guidelines_is_empty() {
        let guidelines = Guidelines::new();
        assert!(guidelines.is_empty());

        let mut guidelines_with_content = Guidelines::new();
        guidelines_with_content.common = Some("Test".to_string());
        assert!(!guidelines_with_content.is_empty());
    }

    #[test]
    fn test_parse_sections() {
        let content = r#"# Common
Use UTF-8 encoding

# Dev
Use snake_case for functions

# Test
Test coverage should be at least 80%
"#;

        let sections = parse_sections(content);
        assert_eq!(sections.len(), 3);

        assert_eq!(sections[0].0, "Common");
        assert!(sections[0].1.contains("UTF-8"));

        assert_eq!(sections[1].0, "Dev");
        assert!(sections[1].1.contains("snake_case"));

        assert_eq!(sections[2].0, "Test");
        assert!(sections[2].1.contains("80%"));
    }

    #[test]
    fn test_loader_nonexistent_path() {
        let loader = GuidelinesLoader::new(PathBuf::from("/nonexistent/path"));
        let guidelines = loader.load().unwrap();
        assert!(guidelines.is_empty());
    }

    #[test]
    fn test_loader_directory() {
        let temp_dir = TempDir::new().unwrap();
        let guidelines_dir = temp_dir.path().join("guidelines");
        fs::create_dir_all(&guidelines_dir).unwrap();

        // Create test files
        fs::write(
            guidelines_dir.join("_common.md"),
            "# Common\nUse UTF-8 encoding",
        )
        .unwrap();
        fs::write(
            guidelines_dir.join("dev.md"),
            "# Dev\nUse snake_case for functions",
        )
        .unwrap();

        let loader = GuidelinesLoader::new(guidelines_dir);
        let guidelines = loader.load().unwrap();

        assert!(guidelines.common.is_some());
        assert!(guidelines.dev.is_some());
        assert!(guidelines.plan.is_none());

        let dev_content = guidelines.get_for_agent_type(AgentType::Dev).unwrap();
        assert!(dev_content.contains("UTF-8"));
        assert!(dev_content.contains("snake_case"));
    }

    #[test]
    fn test_loader_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let guidelines_file = temp_dir.path().join("guidelines.md");

        let content = r#"# Common
Use UTF-8 encoding

# Dev
Use snake_case for functions
"#;
        fs::write(&guidelines_file, content).unwrap();

        let loader = GuidelinesLoader::new(guidelines_file);
        assert!(loader.single_file);

        let guidelines = loader.load().unwrap();
        assert!(guidelines.common.is_some());
        assert!(guidelines.dev.is_some());
    }

    #[test]
    fn test_load_for_agent_type() {
        let temp_dir = TempDir::new().unwrap();
        let guidelines_dir = temp_dir.path().join("guidelines");
        fs::create_dir_all(&guidelines_dir).unwrap();

        fs::write(guidelines_dir.join("dev.md"), "Use snake_case").unwrap();

        let loader = GuidelinesLoader::new(guidelines_dir);
        let dev_guidelines = loader.load_for_agent_type(AgentType::Dev).unwrap();

        assert!(dev_guidelines.is_some());
        assert!(dev_guidelines.unwrap().contains("snake_case"));
    }
}
