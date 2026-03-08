// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Memory stage of the pipeline
//!
//! This module handles updating project memory with completed task information,
//! architectural decisions, and key insights learned during task execution.

use anyhow::Result;
use std::path::PathBuf;
use tracing::{debug, info};

use ltmatrix_core::Task;

/// Configuration for the memory stage
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Whether memory update is enabled
    pub enabled: bool,

    /// Project root directory for memory storage
    pub project_root: Option<PathBuf>,

    /// Memory file path (default: .claude/memory.md)
    pub memory_file: PathBuf,

    /// Whether to extract memories from task results
    pub extract_memories: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        MemoryConfig {
            enabled: true,
            project_root: None,
            memory_file: PathBuf::from(".claude/memory.md"),
            extract_memories: true,
        }
    }
}

impl MemoryConfig {
    /// Create config for fast mode
    pub fn fast_mode() -> Self {
        MemoryConfig {
            enabled: true,
            project_root: None,
            memory_file: PathBuf::from(".claude/memory.md"),
            extract_memories: false, // Don't extract in fast mode
        }
    }

    /// Create config for expert mode
    pub fn expert_mode() -> Self {
        MemoryConfig {
            enabled: true,
            project_root: None,
            memory_file: PathBuf::from(".claude/memory.md"),
            extract_memories: true,
        }
    }
}

/// Update project memory with completed task information
///
/// This function extracts key insights, architectural decisions, and patterns
/// from completed tasks and updates the project memory file.
pub async fn update_memory(tasks: &[Task], config: &MemoryConfig) -> Result<()> {
    if !config.enabled {
        debug!("Memory update disabled by config");
        return Ok(());
    }

    if tasks.is_empty() {
        debug!("No tasks to process for memory update");
        return Ok(());
    }

    info!("Updating project memory for {} tasks", tasks.len());

    // Get completed tasks
    let completed_tasks: Vec<_> = tasks.iter().filter(|t| t.is_completed()).collect();

    if completed_tasks.is_empty() {
        debug!("No completed tasks to process for memory update");
        return Ok(());
    }

    // For now, just log the memory updates
    // In a full implementation, this would:
    // 1. Load existing memory file
    // 2. Extract memories from task results
    // 3. Append new memory entries
    // 4. Summarize if file is too large

    for task in &completed_tasks {
        debug!("Processing memory for task: {}", task.id);

        if config.extract_memories {
            // Extract memory from task description and result
            let memory_entry = format!(
                "## Task: {}\n**Title**: {}\n**Status**: Completed\n\n",
                task.id, task.title
            );

            debug!("Memory entry: {}", memory_entry);
        }
    }

    info!("Memory update completed for {} tasks", completed_tasks.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ltmatrix_core::{Task, TaskStatus};

    #[test]
    fn test_memory_config_default() {
        let config = MemoryConfig::default();
        assert!(config.enabled);
        assert!(config.extract_memories);
    }

    #[test]
    fn test_memory_config_fast_mode() {
        let config = MemoryConfig::fast_mode();
        assert!(!config.extract_memories);
    }

    #[test]
    fn test_memory_config_expert_mode() {
        let config = MemoryConfig::expert_mode();
        assert!(config.extract_memories);
    }

    #[tokio::test]
    async fn test_update_memory_disabled() {
        let config = MemoryConfig {
            enabled: false,
            ..Default::default()
        };

        let tasks = vec![Task::new("task-1", "Test", "Description")];
        let result = update_memory(&tasks, &config).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_memory_empty_tasks() {
        let config = MemoryConfig::default();
        let tasks = vec![];
        let result = update_memory(&tasks, &config).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_memory_with_completed_task() {
        let config = MemoryConfig::default();

        let mut task = Task::new("task-1", "Test Task", "Test description");
        task.status = TaskStatus::Completed;

        let tasks = vec![task];
        let result = update_memory(&tasks, &config).await;

        assert!(result.is_ok());
    }
}
