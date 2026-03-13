// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Changelog generation and management
//!
//! This module provides changelog parsing, generation, and updating following
//! the Keep a Changelog format (https://keepachangelog.com/).

use anyhow::{Context, Result};
use std::fmt;
use std::path::Path;
use std::process::Command;

/// Changelog sections following Keep a Changelog format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangelogSection {
    Added,
    Changed,
    Deprecated,
    Removed,
    Fixed,
    Security,
}

impl fmt::Display for ChangelogSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChangelogSection::Added => write!(f, "Added"),
            ChangelogSection::Changed => write!(f, "Changed"),
            ChangelogSection::Deprecated => write!(f, "Deprecated"),
            ChangelogSection::Removed => write!(f, "Removed"),
            ChangelogSection::Fixed => write!(f, "Fixed"),
            ChangelogSection::Security => write!(f, "Security"),
        }
    }
}

impl std::str::FromStr for ChangelogSection {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "added" => Ok(ChangelogSection::Added),
            "changed" => Ok(ChangelogSection::Changed),
            "deprecated" => Ok(ChangelogSection::Deprecated),
            "removed" => Ok(ChangelogSection::Removed),
            "fixed" => Ok(ChangelogSection::Fixed),
            "security" => Ok(ChangelogSection::Security),
            _ => anyhow::bail!("Unknown changelog section: {}", s),
        }
    }
}

/// A single changelog entry
#[derive(Debug, Clone)]
pub struct ChangelogEntry {
    pub section: ChangelogSection,
    pub scope: Option<String>,
    pub description: String,
    pub pr_number: Option<u64>,
    pub breaking: bool,
}

impl fmt::Display for ChangelogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scope = self
            .scope
            .as_ref()
            .map(|s| format!("**{}**: ", s))
            .unwrap_or_default();

        let pr = self
            .pr_number
            .map(|n| format!(" (#{})", n))
            .unwrap_or_default();

        let breaking = if self.breaking { "**BREAKING** " } else { "" };

        write!(f, "- {}{}{}{}", breaking, scope, self.description, pr)
    }
}

/// Changelog representation
#[derive(Debug, Clone, Default)]
pub struct Changelog {
    pub entries: Vec<ChangelogEntry>,
}

impl Changelog {
    /// Create empty changelog
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Generate changelog from git commits since a tag
    pub fn generate(project_root: impl AsRef<Path>, from_tag: Option<&str>) -> Result<Self> {
        let mut changelog = Self::new();

        // Get commits since the tag
        let range = match from_tag {
            Some(tag) => format!("{}..HEAD", tag),
            None => {
                // Try to get the last tag
                let output = Command::new("git")
                    .args(["describe", "--tags", "--abbrev=0"])
                    .current_dir(project_root.as_ref())
                    .output()?;

                if output.status.success() {
                    let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    format!("{}..HEAD", tag)
                } else {
                    // No tags, use all commits
                    "HEAD".to_string()
                }
            }
        };

        // Get commit log
        let output = Command::new("git")
            .args(["log", "--pretty=format:%H|%s|%b", &range])
            .current_dir(project_root.as_ref())
            .output()
            .context("Failed to get git log")?;

        if !output.status.success() {
            return Ok(changelog);
        }

        let log = String::from_utf8_lossy(&output.stdout);

        // Parse commits
        for line in log.lines() {
            if let Some(entry) = parse_commit_line(line) {
                changelog.entries.push(entry);
            }
        }

        Ok(changelog)
    }

    /// Get total entry count
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get entries by section
    pub fn entries_by_section(&self, section: ChangelogSection) -> Vec<&ChangelogEntry> {
        self.entries
            .iter()
            .filter(|e| e.section == section)
            .collect()
    }

    /// Check if changelog has any entries
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Update CHANGELOG.md file
    pub fn update_file(
        &self,
        project_root: impl AsRef<Path>,
        version: &super::version::Version,
    ) -> Result<()> {
        let changelog_path = project_root.as_ref().join("CHANGELOG.md");

        if !changelog_path.exists() {
            // Create new changelog file
            let content = self.generate_new_changelog(version);
            std::fs::write(&changelog_path, content)?;
            return Ok(());
        }

        let content = std::fs::read_to_string(&changelog_path)?;
        let updated = self.insert_version_section(&content, version)?;
        std::fs::write(&changelog_path, updated)?;

        Ok(())
    }

    /// Generate a new CHANGELOG.md content
    fn generate_new_changelog(&self, version: &super::version::Version) -> String {
        let date = chrono::Local::now().format("%Y-%m-%d");

        let mut content = String::new();
        content.push_str("# Changelog\n\n");
        content
            .push_str("All notable changes to this project will be documented in this file.\n\n");
        content.push_str(
            "The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),\n",
        );
        content.push_str("and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n");
        content.push_str("## [Unreleased]\n\n");
        content.push_str(&self.format_version_section(version, &date.to_string()));

        content
    }

    /// Insert version section into existing changelog
    fn insert_version_section(
        &self,
        content: &str,
        version: &super::version::Version,
    ) -> Result<String> {
        let date = chrono::Local::now().format("%Y-%m-%d");
        let new_section = self.format_version_section(version, &date.to_string());

        // Find [Unreleased] section and insert after it
        let unreleased_pattern = "## [Unreleased]";

        if let Some(pos) = content.find(unreleased_pattern) {
            // Find the next ## heading after Unreleased
            let rest = &content[pos + unreleased_pattern.len()..];
            let next_section_pos = rest
                .find("\n## ")
                .map(|p| p + unreleased_pattern.len() + pos);

            let (before, after) = match next_section_pos {
                Some(p) => (
                    content[..pos + unreleased_pattern.len()].to_string(),
                    content[p..].to_string(),
                ),
                None => (content.to_string(), String::new()),
            };

            // Clear the Unreleased section and add new version
            let mut result = before;
            result.push_str("\n\n");
            result.push_str(&new_section);
            result.push_str(&after);

            Ok(result)
        } else {
            // No Unreleased section, prepend new section
            let mut result = String::new();
            result.push_str(&new_section);
            result.push_str("\n\n");
            result.push_str(content);
            Ok(result)
        }
    }

    /// Format version section as markdown
    fn format_version_section(&self, version: &super::version::Version, date: &str) -> String {
        let mut section = String::new();
        section.push_str(&format!("## [{}] - {}\n\n", version, date));

        // Group entries by section
        for section_type in [
            ChangelogSection::Added,
            ChangelogSection::Changed,
            ChangelogSection::Deprecated,
            ChangelogSection::Removed,
            ChangelogSection::Fixed,
            ChangelogSection::Security,
        ] {
            let entries = self.entries_by_section(section_type);
            if !entries.is_empty() {
                section.push_str(&format!("### {}\n\n", section_type));

                // Group by scope if there are many entries
                if entries.len() > 5 {
                    let grouped = self.group_entries_by_scope(&entries);
                    for (scope, scope_entries) in grouped {
                        if let Some(scope_name) = scope {
                            section.push_str(&format!("#### {}\n\n", capitalize(&scope_name)));
                        }
                        for entry in scope_entries {
                            section.push_str(&format!("{}\n", entry));
                        }
                        section.push('\n');
                    }
                } else {
                    for entry in entries {
                        section.push_str(&format!("{}\n", entry));
                    }
                    section.push('\n');
                }
            }
        }

        section
    }

    /// Group entries by scope
    fn group_entries_by_scope(
        &self,
        entries: &[&ChangelogEntry],
    ) -> Vec<(Option<String>, Vec<ChangelogEntry>)> {
        let mut groups: std::collections::HashMap<Option<String>, Vec<ChangelogEntry>> =
            std::collections::HashMap::new();

        for entry in entries {
            groups
                .entry(entry.scope.clone())
                .or_default()
                .push((*entry).clone());
        }

        let mut result: Vec<_> = groups.into_iter().collect();
        // Sort: entries without scope last
        result.sort_by(|a, b| match (&a.0, &b.0) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(a), Some(b)) => a.cmp(b),
        });

        result
    }
}

/// Parse a commit line in format: HASH|SUBJECT|BODY
fn parse_commit_line(line: &str) -> Option<ChangelogEntry> {
    let parts: Vec<&str> = line.splitn(3, '|').collect();
    if parts.len() < 2 {
        return None;
    }

    let subject = parts[1];

    // Parse conventional commit
    let (commit_type, scope, description, breaking) = parse_conventional_commit(subject)?;

    // Skip certain commit types
    if matches!(
        commit_type,
        "refactor" | "test" | "chore" | "style" | "ci" | "build" | "perf"
    ) {
        return None;
    }

    // Map commit type to changelog section
    let section = match commit_type {
        "feat" | "feature" => ChangelogSection::Added,
        "fix" | "bugfix" => ChangelogSection::Fixed,
        "change" | "changed" => ChangelogSection::Changed,
        "deprecate" | "deprecated" => ChangelogSection::Deprecated,
        "remove" | "removed" => ChangelogSection::Removed,
        "security" => ChangelogSection::Security,
        "docs" | "documentation" => ChangelogSection::Changed,
        _ => ChangelogSection::Changed,
    };

    // Extract PR number from body
    let pr_number = parts.get(2).and_then(|body| {
        let body = *body;
        // Look for PR number in body
        let re = regex::Regex::new(r"#(\d+)").ok()?;
        re.captures(body)?.get(1)?.as_str().parse().ok()
    });

    Some(ChangelogEntry {
        section,
        scope,
        description: description.to_string(),
        pr_number,
        breaking,
    })
}

/// Parse conventional commit format: type(scope)!: description
fn parse_conventional_commit(subject: &str) -> Option<(&str, Option<String>, &str, bool)> {
    // Pattern: type(scope)!: description or type!: description
    let re = regex::Regex::new(r"^([a-z]+)(?:\(([a-z-]+)\))?(!)?:\s*(.+)$").ok()?;

    let caps = re.captures(subject)?;

    let commit_type = caps.get(1)?.as_str();
    let scope = caps.get(2).map(|m| m.as_str().to_string());
    let breaking = caps.get(3).is_some();
    let description = caps.get(4)?.as_str();

    Some((commit_type, scope, description, breaking))
}

/// Capitalize first letter
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_conventional_commit() {
        let result = parse_conventional_commit("feat(cli): add new flag");
        assert_eq!(
            result,
            Some(("feat", Some("cli".to_string()), "add new flag", false))
        );

        let result = parse_conventional_commit("fix: correct bug");
        assert_eq!(result, Some(("fix", None, "correct bug", false)));

        let result = parse_conventional_commit("feat(api)!: breaking change");
        assert_eq!(
            result,
            Some(("feat", Some("api".to_string()), "breaking change", true))
        );
    }

    #[test]
    fn test_changelog_section_display() {
        assert_eq!(ChangelogSection::Added.to_string(), "Added");
        assert_eq!(ChangelogSection::Fixed.to_string(), "Fixed");
    }

    #[test]
    fn test_changelog_entry_display() {
        let entry = ChangelogEntry {
            section: ChangelogSection::Added,
            scope: Some("cli".to_string()),
            description: "Add new flag".to_string(),
            pr_number: Some(42),
            breaking: false,
        };

        assert_eq!(entry.to_string(), "- **cli**: Add new flag (#42)");
    }

    #[test]
    fn test_changelog_entry_breaking() {
        let entry = ChangelogEntry {
            section: ChangelogSection::Changed,
            scope: None,
            description: "API change".to_string(),
            pr_number: None,
            breaking: true,
        };

        assert_eq!(entry.to_string(), "- **BREAKING** API change");
    }
}
