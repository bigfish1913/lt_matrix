// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

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
//! - **Rich metadata**: Entries include files affected, decisions, and code snippets
//!
//! # Memory Entry Format
//!
//! Each memory entry follows this markdown structure:
//!
//! ```markdown
//! ## [Category] Entry Title
//!
//! **Task**: task-xxx
//! **Date**: 2026-03-07 12:00:00 UTC
//! **Category**: Architecture Decision
//! **Files**: src/main.rs, src/lib.rs
//!
//! Content describing the decision or insight...
//!
//! ### Key Points
//! - Point 1
//! - Point 2
//!
//! ---
//! ```
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::memory::{MemoryStore, MemoryEntry, MemoryEntryBuilder, MemoryCategory};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Load memory store
//! let mut store = MemoryStore::new("./my-project")?;
//!
//! // Add a memory entry using the builder pattern
//! let entry = MemoryEntryBuilder::new("task-042", "Architecture Decision")
//!     .content("Decided to use async Rust with Tokio for all I/O operations")
//!     .category(MemoryCategory::ArchitectureDecision)
//!     .files(vec!["src/main.rs".to_string(), "src/lib.rs".to_string()])
//!     .key_points(vec!["All I/O is async".to_string(), "Use tokio runtime".to_string()])
//!     .build()?;
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
use tracing::{debug, info};

use ltmatrix_config::settings::MemoryConfig;

/// Maximum memory file size before triggering summarization (in bytes)
const MAX_MEMORY_SIZE: usize = 50 * 1024; // 50KB

/// Maximum number of entries before triggering summarization
const MAX_ENTRIES: usize = 100;

/// Default memory file path relative to project root
const MEMORY_FILE_PATH: &str = ".claude/memory.md";

/// Categories for memory entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    /// Architecture and design decisions
    ArchitectureDecision,

    /// Patterns established in the codebase
    Pattern,

    /// API design decisions
    ApiDesign,

    /// Data model changes
    DataModel,

    /// Error handling patterns
    ErrorHandling,

    /// Performance optimizations
    Performance,

    /// Security-related decisions
    Security,

    /// Testing strategies
    Testing,

    /// Dependencies and integrations
    Dependencies,

    /// Code organization decisions
    CodeOrganization,

    /// Bug fixes and their rationale
    BugFix,

    /// Configuration decisions
    Configuration,

    /// General important notes
    ImportantNote,

    /// Task completion summaries
    TaskCompletion,

    /// Other uncategorized memories
    General,
}

impl Default for MemoryCategory {
    fn default() -> Self {
        MemoryCategory::General
    }
}

impl std::fmt::Display for MemoryCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryCategory::ArchitectureDecision => write!(f, "Architecture Decision"),
            MemoryCategory::Pattern => write!(f, "Pattern"),
            MemoryCategory::ApiDesign => write!(f, "API Design"),
            MemoryCategory::DataModel => write!(f, "Data Model"),
            MemoryCategory::ErrorHandling => write!(f, "Error Handling"),
            MemoryCategory::Performance => write!(f, "Performance"),
            MemoryCategory::Security => write!(f, "Security"),
            MemoryCategory::Testing => write!(f, "Testing"),
            MemoryCategory::Dependencies => write!(f, "Dependencies"),
            MemoryCategory::CodeOrganization => write!(f, "Code Organization"),
            MemoryCategory::BugFix => write!(f, "Bug Fix"),
            MemoryCategory::Configuration => write!(f, "Configuration"),
            MemoryCategory::ImportantNote => write!(f, "Important Note"),
            MemoryCategory::TaskCompletion => write!(f, "Task Completion"),
            MemoryCategory::General => write!(f, "General"),
        }
    }
}

impl std::str::FromStr for MemoryCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase().replace('_', " ").replace('-', " ");
        match lower.as_str() {
            "architecture decision" | "architectural decision" => {
                Ok(MemoryCategory::ArchitectureDecision)
            }
            "pattern" | "patterns" => Ok(MemoryCategory::Pattern),
            "api design" | "api" => Ok(MemoryCategory::ApiDesign),
            "data model" | "data models" => Ok(MemoryCategory::DataModel),
            "error handling" => Ok(MemoryCategory::ErrorHandling),
            "performance" => Ok(MemoryCategory::Performance),
            "security" => Ok(MemoryCategory::Security),
            "testing" => Ok(MemoryCategory::Testing),
            "dependencies" | "dependency" => Ok(MemoryCategory::Dependencies),
            "code organization" => Ok(MemoryCategory::CodeOrganization),
            "bug fix" | "bugfix" => Ok(MemoryCategory::BugFix),
            "configuration" | "config" => Ok(MemoryCategory::Configuration),
            "important note" | "important" | "note" => Ok(MemoryCategory::ImportantNote),
            "task completion" | "completed" => Ok(MemoryCategory::TaskCompletion),
            "general" => Ok(MemoryCategory::General),
            _ => Err(format!("Unknown memory category: {}", s)),
        }
    }
}

/// Priority level for memory entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryPriority {
    /// Low priority - informational only
    Low,

    /// Normal priority - useful context
    Normal,

    /// High priority - critical architectural decisions
    High,

    /// Critical - must be considered in all future work
    Critical,
}

impl Default for MemoryPriority {
    fn default() -> Self {
        MemoryPriority::Normal
    }
}

/// A single memory entry representing a key decision or insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier for this entry
    pub id: String,

    /// Task ID that generated this memory
    pub task_id: String,

    /// Title/subject of the memory
    pub title: String,

    /// Content of the memory (can be multi-line markdown)
    pub content: String,

    /// Timestamp when the memory was created
    pub timestamp: DateTime<Utc>,

    /// Category of the memory
    pub category: MemoryCategory,

    /// Priority/importance of this memory
    pub priority: MemoryPriority,

    /// Files affected by this decision/insight
    pub files: Vec<String>,

    /// Key points extracted from the content
    pub key_points: Vec<String>,

    /// Related task IDs
    pub related_tasks: Vec<String>,

    /// Tags for searching and filtering
    pub tags: Vec<String>,

    /// Code snippets if relevant
    pub code_snippets: Vec<CodeSnippet>,

    /// Whether this entry is deprecated/obsolete
    pub deprecated: bool,

    /// Reason for deprecation if applicable
    pub deprecation_reason: Option<String>,
}

/// A code snippet associated with a memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSnippet {
    /// File path
    pub file: String,

    /// Starting line number
    pub start_line: usize,

    /// Ending line number
    pub end_line: usize,

    /// The code content
    pub code: String,

    /// Description of what this code does
    pub description: String,
}

impl MemoryEntry {
    /// Create a new memory entry with required fields
    pub fn new(
        task_id: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        let task_id = task_id.into();
        let title = title.into();

        // Generate unique ID
        let id = format!(
            "mem-{}-{}",
            now.timestamp(),
            title
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == ' ')
                .take(20)
                .collect::<String>()
                .replace(' ', "-")
                .to_lowercase()
        );

        Self {
            id,
            task_id,
            title,
            content: content.into(),
            timestamp: now,
            category: MemoryCategory::General,
            priority: MemoryPriority::Normal,
            files: Vec::new(),
            key_points: Vec::new(),
            related_tasks: Vec::new(),
            tags: Vec::new(),
            code_snippets: Vec::new(),
            deprecated: false,
            deprecation_reason: None,
        }
    }

    /// Set the category of the memory
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        let category_str = category.into();
        self.category = category_str.parse().unwrap_or(MemoryCategory::General);
        self
    }

    /// Set the category directly
    pub fn with_category_enum(mut self, category: MemoryCategory) -> Self {
        self.category = category;
        self
    }

    /// Set the priority
    pub fn with_priority(mut self, priority: MemoryPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Add affected files
    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.files = files;
        self
    }

    /// Add key points
    pub fn with_key_points(mut self, points: Vec<String>) -> Self {
        self.key_points = points;
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add related tasks
    pub fn with_related_tasks(mut self, tasks: Vec<String>) -> Self {
        self.related_tasks = tasks;
        self
    }

    /// Format the memory entry as markdown
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Title with category
        md.push_str(&format!("## [{}] {}\n\n", self.category, self.title));

        // Metadata
        md.push_str(&format!("**Task**: {}\n", self.task_id));
        md.push_str(&format!(
            "**Date**: {}\n",
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        md.push_str(&format!("**Priority**: {:?}\n", self.priority));

        if !self.files.is_empty() {
            md.push_str(&format!("**Files**: {}\n", self.files.join(", ")));
        }

        if !self.tags.is_empty() {
            md.push_str(&format!("**Tags**: {}\n", self.tags.join(", ")));
        }

        md.push('\n');

        // Main content
        md.push_str(&self.content);
        md.push_str("\n\n");

        // Key points if any
        if !self.key_points.is_empty() {
            md.push_str("### Key Points\n\n");
            for point in &self.key_points {
                md.push_str(&format!("- {}\n", point));
            }
            md.push('\n');
        }

        // Code snippets if any
        if !self.code_snippets.is_empty() {
            md.push_str("### Code Examples\n\n");
            for snippet in &self.code_snippets {
                md.push_str(&format!(
                    "**{}** ({}:{}-{})\n",
                    snippet.description, snippet.file, snippet.start_line, snippet.end_line
                ));
                md.push_str(&format!("```rust\n{}\n```\n\n", snippet.code));
            }
        }

        // Deprecation notice if applicable
        if self.deprecated {
            md.push_str("> ⚠️ **Deprecated**");
            if let Some(ref reason) = self.deprecation_reason {
                md.push_str(&format!(": {}", reason));
            }
            md.push_str("\n\n");
        }

        md.push_str("---\n");

        md
    }

    /// Create a summary line for this entry
    pub fn to_summary(&self) -> String {
        format!(
            "- [{}] **{}**: {}",
            self.category,
            self.title,
            self.content.lines().next().unwrap_or("")
        )
    }

    /// Check if this entry matches a search query
    pub fn matches(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();

        // Check title
        if self.title.to_lowercase().contains(&query_lower) {
            return true;
        }

        // Check content
        if self.content.to_lowercase().contains(&query_lower) {
            return true;
        }

        // Check tags
        if self
            .tags
            .iter()
            .any(|t| t.to_lowercase().contains(&query_lower))
        {
            return true;
        }

        // Check files
        if self
            .files
            .iter()
            .any(|f| f.to_lowercase().contains(&query_lower))
        {
            return true;
        }

        false
    }
}

/// Builder for creating memory entries with a fluent API
pub struct MemoryEntryBuilder {
    task_id: String,
    title: String,
    content: Option<String>,
    category: MemoryCategory,
    priority: MemoryPriority,
    files: Vec<String>,
    key_points: Vec<String>,
    related_tasks: Vec<String>,
    tags: Vec<String>,
    code_snippets: Vec<CodeSnippet>,
}

impl MemoryEntryBuilder {
    /// Create a new builder with required fields
    pub fn new(task_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            title: title.into(),
            content: None,
            category: MemoryCategory::General,
            priority: MemoryPriority::Normal,
            files: Vec::new(),
            key_points: Vec::new(),
            related_tasks: Vec::new(),
            tags: Vec::new(),
            code_snippets: Vec::new(),
        }
    }

    /// Set the content
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Set the category
    pub fn category(mut self, category: MemoryCategory) -> Self {
        self.category = category;
        self
    }

    /// Set the category from string
    pub fn category_str(mut self, category: impl Into<String>) -> Self {
        self.category = category.into().parse().unwrap_or(MemoryCategory::General);
        self
    }

    /// Set the priority
    pub fn priority(mut self, priority: MemoryPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Add files
    pub fn files(mut self, files: Vec<String>) -> Self {
        self.files = files;
        self
    }

    /// Add a single file
    pub fn file(mut self, file: impl Into<String>) -> Self {
        self.files.push(file.into());
        self
    }

    /// Add key points
    pub fn key_points(mut self, points: Vec<String>) -> Self {
        self.key_points = points;
        self
    }

    /// Add a single key point
    pub fn key_point(mut self, point: impl Into<String>) -> Self {
        self.key_points.push(point.into());
        self
    }

    /// Add tags
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add a single tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add related tasks
    pub fn related_tasks(mut self, tasks: Vec<String>) -> Self {
        self.related_tasks = tasks;
        self
    }

    /// Add a code snippet
    pub fn code_snippet(mut self, snippet: CodeSnippet) -> Self {
        self.code_snippets.push(snippet);
        self
    }

    /// Build the memory entry
    pub fn build(self) -> Result<MemoryEntry> {
        let content = self
            .content
            .ok_or_else(|| anyhow::anyhow!("Content is required for memory entry"))?;

        Ok(MemoryEntry {
            id: String::new(), // Will be generated in new()
            task_id: self.task_id,
            title: self.title,
            content,
            timestamp: Utc::now(),
            category: self.category,
            priority: self.priority,
            files: self.files,
            key_points: self.key_points,
            related_tasks: self.related_tasks,
            tags: self.tags,
            code_snippets: self.code_snippets,
            deprecated: false,
            deprecation_reason: None,
        })
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

    /// Configuration for memory management
    config: MemoryConfig,
}

impl MemoryStore {
    /// Create a new memory store for the given project root with default configuration
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        Self::with_config(project_root, MemoryConfig::default())
    }

    /// Create a new memory store with custom configuration
    pub fn with_config(project_root: impl AsRef<Path>, config: MemoryConfig) -> Result<Self> {
        let project_root = project_root
            .as_ref()
            .canonicalize()
            .context("Failed to canonicalize project root path")?;
        let memory_file = project_root.join(MEMORY_FILE_PATH);

        // Create .claude directory if it doesn't exist
        if let Some(parent) = memory_file.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).context("Failed to create .claude directory")?;
                info!("Created .claude directory at {}", parent.display());
            }
        }

        let store = Self {
            project_root,
            memory_file,
            entries: Arc::new(RwLock::new(Vec::new())),
            config,
        };

        // Load existing entries
        store.load_entries()?;

        Ok(store)
    }

    /// Load existing entries from the memory file
    fn load_entries(&self) -> Result<()> {
        if !self.memory_file.exists() {
            debug!(
                "Memory file does not exist yet: {}",
                self.memory_file.display()
            );
            return Ok(());
        }

        let content =
            std::fs::read_to_string(&self.memory_file).context("Failed to read memory file")?;

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

        std::fs::write(&self.memory_file, content).context("Failed to write memory file")?;

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
            context.push_str(&format!("\n... and {} older entries\n", entries.len() - 20));
        }

        Ok(context)
    }

    /// Check if summarization is needed and perform it
    fn check_and_summarize(&self) -> Result<()> {
        // Skip if summarization is disabled
        if !self.config.enable_summarization {
            return Ok(());
        }

        // Check file size
        if !self.memory_file.exists() {
            return Ok(());
        }

        let metadata =
            std::fs::metadata(&self.memory_file).context("Failed to get memory file metadata")?;

        let entries = self.entries.read().unwrap();

        // Use configurable thresholds
        let needs_summarization = metadata.len() as usize > self.config.max_file_size
            || entries.len() > self.config.max_entries;

        if needs_summarization && entries.len() >= self.config.min_entries_for_summarization {
            info!("Memory file exceeds threshold, triggering summarization");
            drop(entries); // Release lock before summarization
            self.summarize()?;
        }

        Ok(())
    }

    /// Summarize old memory entries to keep the file size manageable
    fn summarize(&self) -> Result<()> {
        let entries = self.entries.read().unwrap();

        if entries.len() < self.config.min_entries_for_summarization {
            return Ok(()); // Not enough entries to summarize
        }

        // Calculate how many entries to keep based on config
        let keep_count = ((entries.len() as f64) * self.config.keep_fraction) as usize;
        let keep_count = keep_count.max(1); // Keep at least 1 entry

        // Separate entries to keep vs summarize
        let (old_entries, recent_entries) = entries.split_at(entries.len() - keep_count);

        // Separate high-priority entries if preservation is enabled
        let (high_priority_old, regular_old): (Vec<_>, Vec<_>) =
            if self.config.preserve_high_priority {
                old_entries.iter().partition(|e| {
                    e.priority == MemoryPriority::High || e.priority == MemoryPriority::Critical
                })
            } else {
                (Vec::new(), old_entries.iter().collect())
            };

        // Group old entries by category (excluding high-priority if preserved)
        let mut categorized: std::collections::HashMap<String, Vec<&MemoryEntry>> =
            std::collections::HashMap::new();

        for entry in regular_old {
            categorized
                .entry(entry.category.to_string())
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
            summary.push_str(&format!(
                "### {} ({} entries)\n\n",
                category,
                category_entries.len()
            ));

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

        // Add preserved high-priority entries if any
        if !high_priority_old.is_empty() {
            summary.push_str("## Preserved High-Priority Entries\n\n");
            for entry in high_priority_old {
                summary.push_str(&entry.to_markdown());
            }
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
            "Summarized memory: kept {} recent entries ({} fraction)",
            keep_count, self.config.keep_fraction
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
            std::fs::remove_file(&self.memory_file).context("Failed to remove memory file")?;
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

    // Extract title (first ## heading, may have category prefix like "[Architecture Decision]")
    let title_line = lines.iter().find(|line| line.starts_with("## "))?;

    // Extract title, potentially removing category prefix
    let title_text = title_line.trim_start_matches("## ");
    let title = if title_text.starts_with('[') {
        // Extract title after "] " pattern
        title_text
            .split("] ")
            .nth(1)
            .unwrap_or(title_text)
            .to_string()
    } else {
        title_text.to_string()
    };

    // Extract metadata
    let mut task_id = String::new();
    let mut category_str = String::new();
    let mut timestamp = Utc::now();
    let mut files: Vec<String> = Vec::new();
    let mut tags: Vec<String> = Vec::new();
    let mut key_points: Vec<String> = Vec::new();

    for line in &lines {
        if line.starts_with("**Task**: ") {
            task_id = line.trim_start_matches("**Task**: ").to_string();
        } else if line.starts_with("**Category**: ") {
            category_str = line.trim_start_matches("**Category**: ").to_string();
        } else if line.starts_with("**Date**: ") {
            let date_str = line.trim_start_matches("**Date**: ");
            if let Ok(parsed) =
                chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S UTC")
            {
                timestamp = DateTime::from_naive_utc_and_offset(parsed, Utc);
            }
        } else if line.starts_with("**Files**: ") {
            let files_str = line.trim_start_matches("**Files**: ");
            files = files_str.split(", ").map(|s| s.to_string()).collect();
        } else if line.starts_with("**Tags**: ") {
            let tags_str = line.trim_start_matches("**Tags**: ");
            tags = tags_str.split(", ").map(|s| s.to_string()).collect();
        }
    }

    // Extract key points (lines starting with "- " after "### Key Points")
    let mut in_key_points = false;
    for line in &lines {
        if line.contains("### Key Points") {
            in_key_points = true;
        } else if line.starts_with("### ") && in_key_points {
            in_key_points = false;
        } else if in_key_points && line.starts_with("- ") {
            key_points.push(line.trim_start_matches("- ").to_string());
        }
    }

    // Extract content (everything after metadata, before Key Points or Code Examples)
    let content_start = lines
        .iter()
        .position(|line| line.is_empty() && !task_id.is_empty())
        .unwrap_or(0);

    let content_end = lines
        .iter()
        .position(|line| line.starts_with("### "))
        .unwrap_or(lines.len());

    let content = lines
        .iter()
        .skip(content_start + 1)
        .take(content_end.saturating_sub(content_start + 1))
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    if task_id.is_empty() {
        return None;
    }

    // Create entry using the constructor
    let mut entry = MemoryEntry::new(&task_id, &title, &content);

    // Set timestamp
    entry.timestamp = timestamp;

    // Set category
    entry.category = category_str.parse().unwrap_or(MemoryCategory::General);

    // Set additional fields
    if !files.is_empty() {
        entry.files = files;
    }
    if !tags.is_empty() {
        entry.tags = tags;
    }
    if !key_points.is_empty() {
        entry.key_points = key_points;
    }

    Some(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_memory_entry_creation() {
        let entry = MemoryEntry::new("task-001", "Test Decision", "This is a test decision");

        assert_eq!(entry.task_id, "task-001");
        assert_eq!(entry.title, "Test Decision");
        assert_eq!(entry.content, "This is a test decision");
        assert_eq!(entry.category, MemoryCategory::General);
    }

    #[test]
    fn test_memory_entry_with_category() {
        let entry = MemoryEntry::new("task-001", "Test", "Content")
            .with_category_enum(MemoryCategory::ArchitectureDecision);

        assert_eq!(entry.category, MemoryCategory::ArchitectureDecision);
    }

    #[test]
    fn test_memory_entry_markdown_format() {
        let entry = MemoryEntry::new(
            "task-042",
            "Test Title",
            "Test content line 1\nTest content line 2",
        );

        let markdown = entry.to_markdown();

        // Title includes category prefix: ## [General] Test Title
        assert!(markdown.contains("## [General] Test Title"));
        assert!(markdown.contains("**Task**: task-042"));
        assert!(markdown.contains("Test content line 1"));
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
                format!("Content for decision {}", i),
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
        assert_eq!(entry.category, MemoryCategory::ArchitectureDecision);
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
