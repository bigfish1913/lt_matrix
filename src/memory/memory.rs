//! Project memory management
//!
//! This module provides persistent memory storage for architectural decisions,
//! key insights, and patterns learned during task execution. It maintains a
//! `.claude/memory.md` file that accumulates knowledge across task runs.
//!
//! # Features
//!
//! - **Persistent storage**: Memory is stored in `.claude/memory.md`
//! - **Timestamped entries**: Each memory entry includes timestamp and task reference
//! - **Automatic summarization**: Large memory files are automatically summarized
//! - **Context injection**: Memory can be injected into agent prompts for context
//! - **Thread-safe operations**: All operations are safe for concurrent access
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::memory::{MemoryStore, MemoryEntry};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Load memory store
//! let mut store = MemoryStore::new("./my-project")?;
//!
//! // Add a memory entry
//! let entry = MemoryEntry::new(
//!     "task-042",
//!     "Architecture Decision",
//!     "Decided to use async Rust with Tokio for all I/O operations"
//! );
//! store.append_entry(&entry)?;
//!
//! // Get memory for context injection
//! let context = store.get_memory_context()?;
//! println!("Current memory:\n{}", context);
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

/// Maximum memory file size before triggering summarization (in bytes)
const MAX_MEMORY_SIZE: usize = 50 * 1024; // 50KB

/// Maximum number of entries before triggering summarization
const MAX_ENTRIES: usize = 100;

/// Default memory file path relative to project root
const MEMORY_FILE_PATH: &str = ".claude/memory.md";

/// A single memory entry representing a key decision or insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Task ID that generated this memory
    pub task_id: String,

    /// Title/subject of the memory
    pub title: String,

    /// Content of the memory (can be multi-line markdown)
    pub content: String,

    /// Timestamp when the memory was created
    pub timestamp: DateTime<Utc>,

    /// Category of the memory (e.g., "Architecture Decision", "Pattern", "Bug Fix")
    pub category: String,
}

impl MemoryEntry {
    /// Create a new memory entry
    pub fn new(task_id: impl Into<String>, title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            title: title.into(),
            content: content.into(),
            timestamp: Utc::now(),
            category: "General".to_string(),
        }
    }

    /// Set the category of the memory
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    /// Format the memory entry as markdown
    pub fn to_markdown(&self) -> String {
        format!(
            "## {}\n\n**Task**: {}\n**Category**: {}\n**Date**: {}\n\n{}\n\n---\n",
            self.title,
            self.task_id,
            self.category,
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            self.content
        )
    }
}

/// Memory store for managing persistent project memory
#[derive(Debug)]
pub struct MemoryStore {
    /// Project root directory
    project_root: PathBuf,

    /// Path to the memory file
    memory_file: PathBuf,

    /// In-memory cache of entries (for faster access)
    entries: Arc<RwLock<Vec<MemoryEntry>>>,
}

impl MemoryStore {
    /// Create a new memory store for the given project root
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let project_root = project_root.as_ref().canonicalize()
            .context("Failed to canonicalize project root path")?;
        let memory_file = project_root.join(MEMORY_FILE_PATH);

        // Create .claude directory if it doesn't exist
        if let Some(parent) = memory_file.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .context("Failed to create .claude directory")?;
                info!("Created .claude directory at {}", parent.display());
            }
        }

        let store = Self {
            project_root,
            memory_file,
            entries: Arc::new(RwLock::new(Vec::new())),
        };

        // Load existing entries
        store.load_entries()?;

        Ok(store)
    }

    /// Load existing entries from the memory file
    fn load_entries(&self) -> Result<()> {
        if !self.memory_file.exists() {
            debug!("Memory file does not exist yet: {}", self.memory_file.display());
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.memory_file)
            .context("Failed to read memory file")?;

        // Parse entries from the markdown file
        let entries = parse_memory_file(&content)?;

        let mut entries_lock = self.entries.write().unwrap();
        *entries_lock = entries;

        debug!("Loaded {} memory entries", entries_lock.len());

        Ok(())
    }

    /// Append a new memory entry to the store
    pub fn append_entry(&self, entry: &MemoryEntry) -> Result<()> {
        // Add to in-memory cache
        {
            let mut entries = self.entries.write().unwrap();
            entries.push(entry.clone());
        }

        // Append to file
        let markdown = entry.to_markdown();

        // Create file if it doesn't exist, or append if it does
        let file_exists = self.memory_file.exists();

        let content = if file_exists {
            // Read existing header
            let existing = std::fs::read_to_string(&self.memory_file)
                .context("Failed to read existing memory file")?;

            // Find insertion point (after header, before first entry)
            if let Some(pos) = existing.find("\n---\n") {
                let before = &existing[..pos + 5];
                let after = &existing[pos + 5..];
                format!("{}{}{}", before, markdown, after)
            } else {
                // No existing entries, just append
                format!("{}{}", existing, markdown)
            }
        } else {
            // Create new file with header
            format!(
                "# Project Memory\n\nThis file contains key decisions, patterns, and insights \
                accumulated during task execution.\n\n---\n{}\n",
                markdown
            )
        };

        std::fs::write(&self.memory_file, content)
            .context("Failed to write memory file")?;

        info!("Appended memory entry for task {}", entry.task_id);

        // Check if summarization is needed
        self.check_and_summarize()?;

        Ok(())
    }

    /// Get the current memory context for injection into prompts
    pub fn get_memory_context(&self) -> Result<String> {
        let entries = self.entries.read().unwrap();

        if entries.is_empty() {
            return Ok("No project memory available yet.".to_string());
        }

        // Format entries for context injection
        let mut context = String::from("# Project Memory Context\n\n");
        context.push_str(&format!("Total entries: {}\n\n", entries.len()));

        // Show the most recent entries (limit to prevent context explosion)
        let recent_entries: Vec<_> = entries.iter().rev().take(20).collect();

        for entry in recent_entries {
            context.push_str(&format!(
                "## {}\n- Task: {}\n- Category: {}\n- Date: {}\n\n{}\n\n",
                entry.title,
                entry.task_id,
                entry.category,
                entry.timestamp.format("%Y-%m-%d"),
                entry.content
            ));
        }

        if entries.len() > 20 {
            context.push_str(&format!(
                "\n... and {} older entries\n",
                entries.len() - 20
            ));
        }

        Ok(context)
    }

    /// Check if summarization is needed and perform it
    fn check_and_summarize(&self) -> Result<()> {
        // Check file size
        if !self.memory_file.exists() {
            return Ok(());
        }

        let metadata = std::fs::metadata(&self.memory_file)
            .context("Failed to get memory file metadata")?;

        let entries = self.entries.read().unwrap();
        let needs_summarization = metadata.len() as usize > MAX_MEMORY_SIZE
            || entries.len() > MAX_ENTRIES;

        if needs_summarization {
            info!("Memory file exceeds threshold, triggering summarization");
            drop(entries); // Release lock before summarization
            self.summarize()?;
        }

        Ok(())
    }

    /// Summarize old memory entries to keep the file size manageable
    fn summarize(&self) -> Result<()> {
        let entries = self.entries.read().unwrap();

        if entries.len() < 10 {
            return Ok(()); // Not enough entries to summarize
        }

        // Keep recent entries and summarize old ones
        let keep_count = entries.len() / 2; // Keep half of the entries
        let (old_entries, recent_entries) = entries.split_at(entries.len() - keep_count);

        // Group old entries by category
        let mut categorized: std::collections::HashMap<String, Vec<&MemoryEntry>> =
            std::collections::HashMap::new();

        for entry in old_entries {
            categorized
                .entry(entry.category.clone())
                .or_insert_with(Vec::new)
                .push(entry);
        }

        // Create summary for each category
        let mut summary = String::new();
        summary.push_str("# Project Memory (Summarized)\n\n");
        summary.push_str("This file contains key decisions, patterns, and insights. ");
        summary.push_str("Older entries have been summarized for brevity.\n\n");
        summary.push_str("## Summary of Earlier Work\n\n");

        for (category, category_entries) in categorized.iter() {
            summary.push_str(&format!("### {} ({} entries)\n\n", category, category_entries.len()));

            for entry in category_entries {
                summary.push_str(&format!(
                    "- **[{}] {}**: {}\n",
                    entry.task_id,
                    entry.title,
                    entry.content.lines().next().unwrap_or("")
                ));
            }
            summary.push_str("\n");
        }

        summary.push_str("## Recent Detailed Entries\n\n---\n");

        // Add recent entries in full detail
        for entry in recent_entries {
            summary.push_str(&entry.to_markdown());
        }

        drop(entries); // Release lock before writing

        // Write summarized memory
        std::fs::write(&self.memory_file, summary)
            .context("Failed to write summarized memory file")?;

        info!(
            "Summarized memory: kept {} recent entries",
            keep_count
        );

        // Reload entries
        self.load_entries()?;

        Ok(())
    }

    /// Get the number of memory entries
    pub fn entry_count(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    /// Get all entries (for testing/debugging)
    pub fn get_entries(&self) -> Vec<MemoryEntry> {
        self.entries.read().unwrap().clone()
    }

    /// Clear all memory entries (for testing)
    #[cfg(test)]
    pub fn clear(&self) -> Result<()> {
        let mut entries = self.entries.write().unwrap();
        entries.clear();

        if self.memory_file.exists() {
            std::fs::remove_file(&self.memory_file)
                .context("Failed to remove memory file")?;
        }

        Ok(())
    }
}

/// Parse memory entries from markdown file content
fn parse_memory_file(content: &str) -> Result<Vec<MemoryEntry>> {
    let mut entries = Vec::new();

    // Split by "---" separators (markdown horizontal rules)
    let sections: Vec<&str> = content.split("\n---\n").collect();

    for section in sections {
        // Skip the header section
        if section.starts_with("# Project Memory") {
            continue;
        }

        // Try to parse as an entry
        if let Some(entry) = parse_memory_section(section) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

/// Parse a single memory section
fn parse_memory_section(section: &str) -> Option<MemoryEntry> {
    let lines: Vec<&str> = section.lines().collect();

    // Extract title (first ## heading)
    let title = lines
        .iter()
        .find(|line| line.starts_with("## "))
        .map(|line| line.trim_start_matches("## ").to_string())?;

    // Extract metadata
    let mut task_id = String::new();
    let mut category = "General".to_string();
    let mut timestamp = Utc::now();

    for line in &lines {
        if line.starts_with("**Task**: ") {
            task_id = line.trim_start_matches("**Task**: ").to_string();
        } else if line.starts_with("**Category**: ") {
            category = line.trim_start_matches("**Category**: ").to_string();
        } else if line.starts_with("**Date**: ") {
            let date_str = line.trim_start_matches("**Date**: ");
            if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(
                date_str,
                "%Y-%m-%d %H:%M:%S UTC"
            ) {
                timestamp = DateTime::from_naive_utc_and_offset(parsed, Utc);
            }
        }
    }

    // Extract content (everything after metadata, before next section)
    let content_start = lines
        .iter()
        .position(|line| line.is_empty() && !task_id.is_empty())
        .unwrap_or(0);

    let content = lines
        .iter()
        .skip(content_start + 1)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    if task_id.is_empty() {
        return None;
    }

    Some(MemoryEntry {
        task_id,
        title,
        content,
        timestamp,
        category,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_memory_entry_creation() {
        let entry = MemoryEntry::new(
            "task-001",
            "Test Decision",
            "This is a test decision"
        );

        assert_eq!(entry.task_id, "task-001");
        assert_eq!(entry.title, "Test Decision");
        assert_eq!(entry.content, "This is a test decision");
        assert_eq!(entry.category, "General");
    }

    #[test]
    fn test_memory_entry_with_category() {
        let entry = MemoryEntry::new("task-001", "Test", "Content")
            .with_category("Architecture Decision");

        assert_eq!(entry.category, "Architecture Decision");
    }

    #[test]
    fn test_memory_entry_markdown_format() {
        let entry = MemoryEntry::new("task-042", "Test Title", "Test content line 1\nTest content line 2");

        let markdown = entry.to_markdown();

        assert!(markdown.contains("## Test Title"));
        assert!(markdown.contains("**Task**: task-042"));
        assert!(markdown.contains("Test content line 1"));
        assert!(markdown.contains("---"));
    }

    #[test]
    fn test_memory_store_creation() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        assert_eq!(store.entry_count(), 0);
    }

    #[test]
    fn test_memory_store_append() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        let entry = MemoryEntry::new("task-001", "First Decision", "Use async Rust");
        store.append_entry(&entry).unwrap();

        assert_eq!(store.entry_count(), 1);

        // Verify file was created
        let memory_file = temp_dir.path().join(".claude/memory.md");
        assert!(memory_file.exists());

        let content = std::fs::read_to_string(memory_file).unwrap();
        assert!(content.contains("# Project Memory"));
        assert!(content.contains("First Decision"));
    }

    #[test]
    fn test_memory_store_multiple_entries() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        for i in 1..=5 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content for decision {}", i)
            );
            store.append_entry(&entry).unwrap();
        }

        assert_eq!(store.entry_count(), 5);
    }

    #[test]
    fn test_memory_context_injection() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // Empty store
        let context = store.get_memory_context().unwrap();
        assert!(context.contains("No project memory available"));

        // Add entry
        let entry = MemoryEntry::new("task-001", "Architecture", "Use Tokio runtime");
        store.append_entry(&entry).unwrap();

        let context = store.get_memory_context().unwrap();
        assert!(context.contains("Project Memory Context"));
        assert!(context.contains("Architecture"));
        assert!(context.contains("Use Tokio runtime"));
    }

    #[test]
    fn test_parse_memory_section() {
        let section = r#"## Test Decision

**Task**: task-042
**Category**: Architecture Decision
**Date**: 2026-03-07 12:00:00 UTC

This is the content of the decision.

It can span multiple lines.
"#;

        let entry = parse_memory_section(section).unwrap();

        assert_eq!(entry.title, "Test Decision");
        assert_eq!(entry.task_id, "task-042");
        assert_eq!(entry.category, "Architecture Decision");
        assert!(entry.content.contains("content of the decision"));
    }

    #[test]
    fn test_memory_persistence() {
        let temp_dir = TempDir::new().unwrap();

        // Create store and add entry
        {
            let store = MemoryStore::new(temp_dir.path()).unwrap();
            let entry = MemoryEntry::new("task-001", "Decision", "Content");
            store.append_entry(&entry).unwrap();
        }

        // Create new store and verify entry persists
        {
            let store = MemoryStore::new(temp_dir.path()).unwrap();
            assert_eq!(store.entry_count(), 1);

            let entries = store.get_entries();
            assert_eq!(entries[0].task_id, "task-001");
        }
    }
}