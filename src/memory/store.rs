// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! High-level memory store integration
//!
//! This module provides integration between the memory system and the pipeline,
//! making it easy to inject memory into agent prompts and extract memories
//! from task completions.

use anyhow::Result;
use std::path::Path;
use tracing::{debug, info};

use crate::models::Task;
use super::memory::{MemoryEntry, MemoryStore};
use super::extractor::{extract_memory_from_task, extract_task_summary};

/// Memory integration for pipeline stages
pub struct MemoryIntegration {
    store: MemoryStore,
}

impl MemoryIntegration {
    /// Create a new memory integration for the given project
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let store = MemoryStore::new(project_root)?;
        Ok(Self { store })
    }

    /// Get memory context for injection into agent prompts
    pub fn get_context_for_prompt(&self) -> Result<String> {
        self.store.get_memory_context()
    }

    /// Extract and store memories from a completed task
    pub fn extract_and_store(&self, task: &Task, task_result: &str) -> Result<usize> {
        // Extract memories from task result
        let entries = extract_memory_from_task(task, task_result)?;

        let count = entries.len();

        // Store all extracted entries
        for entry in &entries {
            self.store.append_entry(entry)?;
        }

        if count > 0 {
            info!("Extracted and stored {} memory entries from task {}", count, task.id);
        }

        Ok(count)
    }

    /// Create and store a summary memory for a task
    pub fn store_task_summary(&self, task: &Task, files_changed: &[String]) -> Result<()> {
        let entry = extract_task_summary(task, files_changed)?;
        self.store.append_entry(&entry)?;
        debug!("Stored task summary for {}", task.id);
        Ok(())
    }

    /// Get the number of stored memory entries
    pub fn entry_count(&self) -> usize {
        self.store.entry_count()
    }

    /// Get all memory entries
    pub fn get_entries(&self) -> Vec<MemoryEntry> {
        self.store.get_entries()
    }
}

/// Format memory for injection into an agent prompt
pub fn format_memory_for_prompt(memory_context: &str) -> String {
    format!(
        "# Project Memory Context\n\n\
        The following context from previous tasks may be relevant:\n\n\
        {}\n\n\
        Consider this information when making decisions and implementing solutions.",
        memory_context
    )
}

/// Check if memory injection is beneficial for a given prompt
pub fn should_inject_memory(prompt: &str) -> bool {
    // Don't inject memory for very short prompts
    if prompt.len() < 100 {
        return false;
    }

    // Keywords that suggest memory would be helpful
    let memory_keywords = [
        "architecture",
        "design",
        "pattern",
        "best practice",
        "previous",
        "existing",
        "current",
        "maintain",
        "extend",
        "refactor",
        "integrate",
    ];

    let prompt_lower = prompt.to_lowercase();

    // Inject if any keyword is found
    memory_keywords.iter().any(|keyword| prompt_lower.contains(keyword))
}

/// Calculate the maximum memory size that can be injected given remaining context
pub fn calculate_max_memory_size(available_context: usize) -> usize {
    // Reserve 20% of available context for memory
    let max_size = (available_context as f64 * 0.2) as usize;

    // But cap at 5KB to avoid overwhelming the prompt
    max_size.min(5 * 1024)
}

/// Truncate memory context to fit within size limits
pub fn truncate_memory_context(context: &str, max_size: usize) -> String {
    if context.len() <= max_size {
        return context.to_string();
    }

    // Try to truncate at a natural boundary
    let truncated = &context[..max_size];

    // Find the last newline before the cutoff
    if let Some(last_newline) = truncated.rfind('\n') {
        let result = &context[..last_newline];
        format!(
            "{}\n\n... (memory truncated for context limits)",
            result
        )
    } else {
        format!(
            "{}...\n\n... (memory truncated for context limits)",
            truncated
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Task, TaskStatus, TaskComplexity};
    use tempfile::TempDir;

    fn create_test_task() -> Task {
        let mut task = Task::new("task-001", "Test Task", "Test description");
        task.status = TaskStatus::Completed;
        task.complexity = TaskComplexity::Moderate;
        task
    }

    #[test]
    fn test_memory_integration_creation() {
        let temp_dir = TempDir::new().unwrap();
        let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

        assert_eq!(integration.entry_count(), 0);
    }

    #[test]
    fn test_extract_and_store() {
        let temp_dir = TempDir::new().unwrap();
        let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

        let task = create_test_task();
        let result = "Architecture decision: Using Tokio runtime for async operations";

        let count = integration.extract_and_store(&task, result).unwrap();

        assert!(count > 0);
        assert_eq!(integration.entry_count(), count);
    }

    #[test]
    fn test_store_task_summary() {
        let temp_dir = TempDir::new().unwrap();
        let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

        let task = create_test_task();
        let files = vec!["src/main.rs".to_string()];

        integration.store_task_summary(&task, &files).unwrap();

        assert_eq!(integration.entry_count(), 1);
    }

    #[test]
    fn test_format_memory_for_prompt() {
        let memory = "## Architecture Decision\nUse async Rust with Tokio";

        let formatted = format_memory_for_prompt(memory);

        assert!(formatted.contains("Project Memory Context"));
        assert!(formatted.contains("Tokio"));
        assert!(formatted.contains("Consider this information"));
    }

    #[test]
    fn test_should_inject_memory() {
        // Short prompt - no injection
        assert!(!should_inject_memory("Fix bug"));

        // Long prompt without keywords - no injection
        let long_no_keywords = "x".repeat(200);
        assert!(!should_inject_memory(&long_no_keywords));

        // Prompts with keywords - inject (need to be > 100 chars)
        let prompt1 = "Refactor the architecture to use better patterns throughout the entire codebase and improve overall system design";
        assert!(should_inject_memory(prompt1));

        let prompt2 = "Extend the existing functionality to support more features and maintain backward compatibility with previous versions";
        assert!(should_inject_memory(prompt2));

        let prompt3 = "Follow best practices for error handling and ensure proper integration with all the components in the system";
        assert!(should_inject_memory(prompt3));
    }

    #[test]
    fn test_calculate_max_memory_size() {
        assert_eq!(calculate_max_memory_size(10000), 2000); // 20% of 10KB
        assert_eq!(calculate_max_memory_size(25000), 5000); // 25000 * 0.2 = 5000
        assert_eq!(calculate_max_memory_size(25600), 5120); // 25600 * 0.2 = 5120
        assert_eq!(calculate_max_memory_size(100000), 5120); // Capped at 5KB (5 * 1024)
    }

    #[test]
    fn test_truncate_memory_context() {
        let context = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";

        // No truncation needed
        let result = truncate_memory_context(context, 100);
        assert_eq!(result, context);

        // Truncation needed
        let result = truncate_memory_context(context, 15);
        assert!(result.contains("truncated"));
        assert!(result.len() <= 15 + 50); // Allow room for truncation message
    }

    #[test]
    fn test_get_context_for_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

        // Empty memory
        let context = integration.get_context_for_prompt().unwrap();
        assert!(context.contains("No project memory available"));

        // Add entry using a pattern that the extractor will recognize
        let task = create_test_task();
        integration.extract_and_store(&task, "Architecture decision: Use Tokio runtime for async operations").unwrap();

        let context = integration.get_context_for_prompt().unwrap();
        assert!(context.contains("Project Memory Context"));
    }
}