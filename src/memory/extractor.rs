//! Memory extraction from task results
//!
//! This module provides functionality to extract key decisions, patterns,
//! and insights from completed tasks for storage in project memory.

use anyhow::Result;
use regex::Regex;
use tracing::debug;

use crate::models::Task;
use super::memory::MemoryEntry;

/// Extract memory entries from a completed task
pub fn extract_memory_from_task(task: &Task, task_result: &str) -> Result<Vec<MemoryEntry>> {
    let mut entries = Vec::new();

    // Extract architectural decisions
    entries.extend(extract_architectural_decisions(task, task_result)?);

    // Extract patterns
    entries.extend(extract_patterns(task, task_result)?);

    // Extract important notes
    entries.extend(extract_important_notes(task, task_result)?);

    debug!("Extracted {} memory entries from task {}", entries.len(), task.id);

    Ok(entries)
}

/// Extract architectural decisions from task result
fn extract_architectural_decisions(task: &Task, result: &str) -> Result<Vec<MemoryEntry>> {
    let mut entries = Vec::new();

    // Look for decision markers
    let patterns = [
        r"(?i)architecture decision[:\s]+([^\n]+)",
        r"(?i)decided to[:\s]+([^\n]+)",
        r"(?i)using\s+(\w+)\s+for\s+([^\n]+)",
        r"(?i)adopted\s+(\w+)\s+([^\n]+)",
    ];

    for pattern_str in &patterns {
        if let Ok(re) = Regex::new(pattern_str) {
            for cap in re.captures_iter(result) {
                if let Some(matched) = cap.get(1) {
                    let decision: &str = matched.as_str().trim();

                    // Skip if too short or too long
                    if decision.len() > 10 && decision.len() < 500 {
                        entries.push(
                            MemoryEntry::new(
                                &task.id,
                                format!("Architecture Decision from {}", task.title),
                                decision.to_string(),
                            )
                            .with_category("Architecture Decision")
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
    ];

    for pattern_str in &patterns {
        if let Ok(re) = Regex::new(pattern_str) {
            for cap in re.captures_iter(result) {
                if let Some(matched) = cap.get(1) {
                    let pattern_text: &str = matched.as_str().trim();

                    if pattern_text.len() > 10 && pattern_text.len() < 500 {
                        entries.push(
                            MemoryEntry::new(
                                &task.id,
                                format!("Pattern from {}", task.title),
                                pattern_text.to_string(),
                            )
                            .with_category("Pattern")
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
    ];

    for pattern_str in &patterns {
        if let Ok(re) = Regex::new(pattern_str) {
            for cap in re.captures_iter(result) {
                if let Some(matched) = cap.get(1) {
                    let note: &str = matched.as_str().trim();

                    if note.len() > 10 && note.len() < 500 {
                        entries.push(
                            MemoryEntry::new(
                                &task.id,
                                format!("Important Note from {}", task.title),
                                note.to_string(),
                            )
                            .with_category("Important Note")
                        );
                    }
                }
            }
        }
    }

    Ok(entries)
}

/// Extract a summary memory from a complex task
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
        .with_category("Task Completion")
    )
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
    }

    #[test]
    fn test_extract_patterns() {
        let task = create_test_task();
        let result = "Pattern: Repository pattern for data access layer";

        let entries = extract_patterns(&task, result).unwrap();

        assert!(!entries.is_empty());
        assert!(entries[0].category == "Pattern");
        assert!(entries[0].content.contains("Repository"));
    }

    #[test]
    fn test_extract_important_notes() {
        let task = create_test_task();
        let result = "Important: Always validate user input before processing";

        let entries = extract_important_notes(&task, result).unwrap();

        assert!(!entries.is_empty());
        assert!(entries[0].category == "Important Note");
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
        assert_eq!(entry.category, "Task Completion");
    }
}