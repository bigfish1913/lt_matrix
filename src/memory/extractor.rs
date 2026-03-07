//! Memory extraction from task results
//!
//! This module provides functionality to extract key decisions, patterns,
//! and insights from completed tasks for storage in project memory.
//!
//! # Extraction Patterns
//!
//! The module supports extracting several types of information:
//! - **Architecture Decisions**: Statements about technology choices and design patterns
//! - **Patterns**: Code patterns and best practices established
//! - **Important Notes**: Warnings, reminders, and important observations
//! - **API Decisions**: API design choices
//! - **Performance Insights**: Performance-related decisions
//! - **Security Decisions**: Security-related choices
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::memory::extractor::extract_memory_from_task;
//! use ltmatrix::models::Task;
//!
//! # fn example() -> anyhow::Result<()> {
//! let task = Task::new("task-001", "Setup Database", "Configure PostgreSQL");
//! let result = r#"
//!     Architecture decision: Using PostgreSQL with connection pooling
//!     Pattern: Repository pattern for data access
//!     Important: Connection strings should be stored in environment variables
//! "#;
//!
//! let entries = extract_memory_from_task(&task, result)?;
//! println!("Extracted {} memory entries", entries.len());
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use regex::Regex;
use tracing::debug;

use crate::models::Task;
use super::memory::{MemoryEntry, MemoryCategory};

/// Extract memory entries from a completed task
///
/// This function analyzes task output for various patterns that indicate
/// important decisions, patterns, or notes that should be preserved in
/// project memory.
///
/// # Arguments
///
/// * `task` - The completed task to extract memories from
/// * `task_result` - The output/result from the task execution
///
/// # Returns
///
/// A vector of memory entries extracted from the task result
pub fn extract_memory_from_task(task: &Task, task_result: &str) -> Result<Vec<MemoryEntry>> {
    let mut entries = Vec::new();

    // Extract architectural decisions
    entries.extend(extract_architectural_decisions(task, task_result)?);

    // Extract patterns
    entries.extend(extract_patterns(task, task_result)?);

    // Extract important notes
    entries.extend(extract_important_notes(task, task_result)?);

    // Extract API decisions
    entries.extend(extract_api_decisions(task, task_result)?);

    // Extract performance decisions
    entries.extend(extract_performance_decisions(task, task_result)?);

    // Extract security decisions
    entries.extend(extract_security_decisions(task, task_result)?);

    // Extract error handling patterns
    entries.extend(extract_error_handling_patterns(task, task_result)?);

    debug!("Extracted {} memory entries from task {}", entries.len(), task.id);

    Ok(entries)
}

/// Extract architectural decisions from task result
fn extract_architectural_decisions(task: &Task, result: &str) -> Result<Vec<MemoryEntry>> {
    let mut entries = Vec::new();

    // Look for decision markers
    let patterns = [
        r"(?i)architecture decision[:\s]+([^\n]+)",
        r"(?i)architectural decision[:\s]+([^\n]+)",
        r"(?i)decided to[:\s]+use\s+([^\n]+)",
        r"(?i)decided to[:\s]+([^\n]+)",
        r"(?i)using\s+(\w+)\s+for\s+([^\n]+)",
        r"(?i)adopted\s+(\w+)\s+([^\n]+)",
        r"(?i)chose\s+(\w+)\s+([^\n]+)",
    ];

    for pattern_str in &patterns {
        if let Ok(re) = Regex::new(pattern_str) {
            for cap in re.captures_iter(result) {
                // Get the first capture group (decision text)
                if let Some(matched) = cap.get(1) {
                    let decision = matched.as_str().trim();

                    // Skip if too short or too long
                    if decision.len() > 10 && decision.len() < 500 {
                        entries.push(
                            MemoryEntry::new(
                                &task.id,
                                format!("Architecture Decision from {}", task.title),
                                decision.to_string(),
                            )
                            .with_category_enum(MemoryCategory::ArchitectureDecision)
                            .with_tags(vec!["architecture".to_string()])
                        );
                    }
                }
            }
        }
    }

    Ok(entries)
}

/// Extract patterns from task result
fn extract_patterns(task: &Task, result: &str) -> Result<Vec<MemoryEntry>> {
    let mut entries = Vec::new();

    // Look for pattern markers
    let patterns = [
        r"(?i)pattern[:\s]+([^\n]+)",
        r"(?i)best practice[:\s]+([^\n]+)",
        r"(?i)established\s+(?:a\s+)?pattern[:\s]+([^\n]+)",
        r"(?i)follow(?:ing)?\s+(?:the\s+)?(\w+)\s+pattern",
    ];

    for pattern_str in &patterns {
        if let Ok(re) = Regex::new(pattern_str) {
            for cap in re.captures_iter(result) {
                if let Some(matched) = cap.get(1) {
                    let pattern_text = matched.as_str().trim();

                    if pattern_text.len() > 10 && pattern_text.len() < 500 {
                        entries.push(
                            MemoryEntry::new(
                                &task.id,
                                format!("Pattern from {}", task.title),
                                pattern_text.to_string(),
                            )
                            .with_category_enum(MemoryCategory::Pattern)
                            .with_tags(vec!["pattern".to_string()])
                        );
                    }
                }
            }
        }
    }

    Ok(entries)
}

/// Extract important notes from task result
fn extract_important_notes(task: &Task, result: &str) -> Result<Vec<MemoryEntry>> {
    let mut entries = Vec::new();

    // Look for note markers
    let patterns = [
        r"(?i)important[:\s]+([^\n]+)",
        r"(?i)note[:\s]+([^\n]+)",
        r"(?i)remember[:\s]+([^\n]+)",
        r"(?i)warning[:\s]+([^\n]+)",
        r"(?i)caution[:\s]+([^\n]+)",
        r"(?i)be careful[:\s]+([^\n]+)",
    ];

    for pattern_str in &patterns {
        if let Ok(re) = Regex::new(pattern_str) {
            for cap in re.captures_iter(result) {
                if let Some(matched) = cap.get(1) {
                    let note = matched.as_str().trim();

                    if note.len() > 10 && note.len() < 500 {
                        entries.push(
                            MemoryEntry::new(
                                &task.id,
                                format!("Important Note from {}", task.title),
                                note.to_string(),
                            )
                            .with_category_enum(MemoryCategory::ImportantNote)
                            .with_tags(vec!["important".to_string()])
                        );
                    }
                }
            }
        }
    }

    Ok(entries)
}

/// Extract API design decisions from task result
fn extract_api_decisions(task: &Task, result: &str) -> Result<Vec<MemoryEntry>> {
    let mut entries = Vec::new();

    let patterns = [
        r"(?i)api\s+(?:design|decision)[:\s]+([^\n]+)",
        r"(?i)endpoint[:\s]+([^\n]+)",
        r"(?i)exposed\s+(?:api|endpoint)[:\s]+([^\n]+)",
    ];

    for pattern_str in &patterns {
        if let Ok(re) = Regex::new(pattern_str) {
            for cap in re.captures_iter(result) {
                if let Some(matched) = cap.get(1) {
                    let api_decision = matched.as_str().trim();

                    if api_decision.len() > 10 && api_decision.len() < 500 {
                        entries.push(
                            MemoryEntry::new(
                                &task.id,
                                format!("API Design from {}", task.title),
                                api_decision.to_string(),
                            )
                            .with_category_enum(MemoryCategory::ApiDesign)
                            .with_tags(vec!["api".to_string()])
                        );
                    }
                }
            }
        }
    }

    Ok(entries)
}

/// Extract performance-related decisions from task result
fn extract_performance_decisions(task: &Task, result: &str) -> Result<Vec<MemoryEntry>> {
    let mut entries = Vec::new();

    let patterns = [
        r"(?i)performance[:\s]+([^\n]+)",
        r"(?i)optimized[:\s]+([^\n]+)",
        r"(?i)optimization[:\s]+([^\n]+)",
        r"(?i)for (?:better|improved) performance[:\s]+([^\n]+)",
    ];

    for pattern_str in &patterns {
        if let Ok(re) = Regex::new(pattern_str) {
            for cap in re.captures_iter(result) {
                if let Some(matched) = cap.get(1) {
                    let perf_decision = matched.as_str().trim();

                    if perf_decision.len() > 10 && perf_decision.len() < 500 {
                        entries.push(
                            MemoryEntry::new(
                                &task.id,
                                format!("Performance Decision from {}", task.title),
                                perf_decision.to_string(),
                            )
                            .with_category_enum(MemoryCategory::Performance)
                            .with_tags(vec!["performance".to_string()])
                        );
                    }
                }
            }
        }
    }

    Ok(entries)
}

/// Extract security-related decisions from task result
fn extract_security_decisions(task: &Task, result: &str) -> Result<Vec<MemoryEntry>> {
    let mut entries = Vec::new();

    let patterns = [
        r"(?i)security[:\s]+([^\n]+)",
        r"(?i)secured[:\s]+([^\n]+)",
        r"(?i)authentication[:\s]+([^\n]+)",
        r"(?i)authorization[:\s]+([^\n]+)",
        r"(?i)encrypted[:\s]+([^\n]+)",
    ];

    for pattern_str in &patterns {
        if let Ok(re) = Regex::new(pattern_str) {
            for cap in re.captures_iter(result) {
                if let Some(matched) = cap.get(1) {
                    let security_decision = matched.as_str().trim();

                    if security_decision.len() > 10 && security_decision.len() < 500 {
                        entries.push(
                            MemoryEntry::new(
                                &task.id,
                                format!("Security Decision from {}", task.title),
                                security_decision.to_string(),
                            )
                            .with_category_enum(MemoryCategory::Security)
                            .with_tags(vec!["security".to_string()])
                        );
                    }
                }
            }
        }
    }

    Ok(entries)
}

/// Extract error handling patterns from task result
fn extract_error_handling_patterns(task: &Task, result: &str) -> Result<Vec<MemoryEntry>> {
    let mut entries = Vec::new();

    let patterns = [
        r"(?i)error handling[:\s]+([^\n]+)",
        r"(?i)handles? errors?[:\s]+([^\n]+)",
        r"(?i)error recovery[:\s]+([^\n]+)",
    ];

    for pattern_str in &patterns {
        if let Ok(re) = Regex::new(pattern_str) {
            for cap in re.captures_iter(result) {
                if let Some(matched) = cap.get(1) {
                    let error_pattern = matched.as_str().trim();

                    if error_pattern.len() > 10 && error_pattern.len() < 500 {
                        entries.push(
                            MemoryEntry::new(
                                &task.id,
                                format!("Error Handling from {}", task.title),
                                error_pattern.to_string(),
                            )
                            .with_category_enum(MemoryCategory::ErrorHandling)
                            .with_tags(vec!["error-handling".to_string()])
                        );
                    }
                }
            }
        }
    }

    Ok(entries)
}

/// Extract a summary memory from a complex task
///
/// Creates a memory entry summarizing the task completion including
/// files modified and key outcomes.
///
/// # Arguments
///
/// * `task` - The completed task
/// * `files_changed` - List of files that were modified during the task
///
/// # Returns
///
/// A memory entry summarizing the task completion
pub fn extract_task_summary(task: &Task, files_changed: &[String]) -> Result<MemoryEntry> {
    let mut content = String::new();

    content.push_str(&format!("Task: {}\n\n", task.title));
    content.push_str(&format!("Description: {}\n\n", task.description));

    if !files_changed.is_empty() {
        content.push_str("Files modified:\n");
        for file in files_changed {
            content.push_str(&format!("- {}\n", file));
        }
    }

    Ok(
        MemoryEntry::new(
            &task.id,
            format!("Completed: {}", task.title),
            content,
        )
        .with_category_enum(MemoryCategory::TaskCompletion)
        .with_files(files_changed.to_vec())
    )
}

/// Extract files affected from task output
///
/// Parses task output to identify files that were created or modified.
pub fn extract_files_affected(result: &str) -> Vec<String> {
    let mut files = Vec::new();

    // Common patterns for file references
    let patterns = [
        r"(?i)created[:\s]+`([^`]+)`",
        r"(?i)modified[:\s]+`([^`]+)`",
        r"(?i)updated[:\s]+`([^`]+)`",
        r"(?i)file[:\s]+`([^`]+)`",
        r"src/[\w/]+\.rs",
        r"tests/[\w/]+\.rs",
        r"[\w/]+\.toml",
    ];

    for pattern_str in &patterns {
        if let Ok(re) = Regex::new(pattern_str) {
            for cap in re.captures_iter(result) {
                if let Some(matched) = cap.get(1).or_else(|| cap.get(0)) {
                    let file = matched.as_str().trim();
                    if !files.contains(&file.to_string()) {
                        files.push(file.to_string());
                    }
                }
            }
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Task, TaskStatus, TaskComplexity};

    fn create_test_task() -> Task {
        let mut task = Task::new("task-001", "Test Task", "Test description");
        task.status = TaskStatus::Completed;
        task.complexity = TaskComplexity::Moderate;
        task
    }

    #[test]
    fn test_extract_architectural_decisions() {
        let task = create_test_task();
        let result = "Architecture decision: Using Tokio runtime for async operations";

        let entries = extract_architectural_decisions(&task, result).unwrap();

        assert!(!entries.is_empty());
        assert!(entries[0].title.contains("Architecture Decision"));
        assert!(entries[0].content.contains("Tokio"));
        assert_eq!(entries[0].category, MemoryCategory::ArchitectureDecision);
    }

    #[test]
    fn test_extract_patterns() {
        let task = create_test_task();
        let result = "Pattern: Repository pattern for data access layer";

        let entries = extract_patterns(&task, result).unwrap();

        assert!(!entries.is_empty());
        assert_eq!(entries[0].category, MemoryCategory::Pattern);
        assert!(entries[0].content.contains("Repository"));
    }

    #[test]
    fn test_extract_important_notes() {
        let task = create_test_task();
        let result = "Important: Always validate user input before processing";

        let entries = extract_important_notes(&task, result).unwrap();

        assert!(!entries.is_empty());
        assert_eq!(entries[0].category, MemoryCategory::ImportantNote);
    }

    #[test]
    fn test_extract_memory_from_task() {
        let task = create_test_task();
        let result = r#"
        Architecture decision: Using async/await with Tokio
        Pattern: Error handling with anyhow
        Important: Remember to handle panics gracefully
        "#;

        let entries = extract_memory_from_task(&task, result).unwrap();

        assert!(entries.len() >= 3);
    }

    #[test]
    fn test_extract_task_summary() {
        let task = create_test_task();
        let files = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
        ];

        let entry = extract_task_summary(&task, &files).unwrap();

        assert_eq!(entry.task_id, "task-001");
        assert!(entry.content.contains("Test Task"));
        assert!(entry.content.contains("src/main.rs"));
        assert_eq!(entry.category, MemoryCategory::TaskCompletion);
    }

    #[test]
    fn test_extract_api_decisions() {
        let task = create_test_task();
        let result = "API design: RESTful endpoints with JSON responses";

        let entries = extract_api_decisions(&task, result).unwrap();

        assert!(!entries.is_empty());
        assert_eq!(entries[0].category, MemoryCategory::ApiDesign);
    }

    #[test]
    fn test_extract_performance_decisions() {
        let task = create_test_task();
        let result = "Performance: Added connection pooling for database queries";

        let entries = extract_performance_decisions(&task, result).unwrap();

        assert!(!entries.is_empty());
        assert_eq!(entries[0].category, MemoryCategory::Performance);
    }

    #[test]
    fn test_extract_security_decisions() {
        let task = create_test_task();
        let result = "Security: Implemented JWT-based authentication";

        let entries = extract_security_decisions(&task, result).unwrap();

        assert!(!entries.is_empty());
        assert_eq!(entries[0].category, MemoryCategory::Security);
    }

    #[test]
    fn test_extract_error_handling_patterns() {
        let task = create_test_task();
        let result = "Error handling: Using anyhow for error propagation";

        let entries = extract_error_handling_patterns(&task, result).unwrap();

        assert!(!entries.is_empty());
        assert_eq!(entries[0].category, MemoryCategory::ErrorHandling);
    }

    #[test]
    fn test_extract_files_affected() {
        let result = r#"
        Created: src/new_module.rs
        Modified: src/main.rs
        File: `Cargo.toml` updated with new dependencies
        "#;

        let files = extract_files_affected(result);

        assert!(!files.is_empty());
        assert!(files.contains(&"src/new_module.rs".to_string()));
    }

    #[test]
    fn test_memory_entry_has_tags() {
        let task = create_test_task();
        let result = "Architecture decision: Using PostgreSQL for persistence";

        let entries = extract_architectural_decisions(&task, result).unwrap();

        assert!(!entries.is_empty());
        assert!(entries[0].tags.contains(&"architecture".to_string()));
    }
}