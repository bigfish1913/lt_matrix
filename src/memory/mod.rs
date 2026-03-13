// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Project memory management
//!
//! This module handles persistent memory storage and retrieval for architectural decisions,
//! patterns, and insights accumulated during task execution.
//!
//! # Overview
//!
//! The memory system maintains multiple levels of memory:
//!
//! - **Project Memory**: Long-term memory stored at `.ltmatrix/memory/project.json`
//!   - Project structure and architecture
//!   - Technology stack and conventions
//!   - Completed tasks history
//!   - Architecture decisions
//!
//! - **Run Memory**: Per-execution memory stored at `.ltmatrix/memory/run-{id}.json`
//!   - Agent sessions and their states
//!   - Context decisions made during execution
//!   - Task execution history
//!
//! - **Memory.md**: Legacy memory file at `.claude/memory.md`
//!   - Timestamped entries
//!   - Automatic summarization
//!   - Context injection
//!
//! # Modules
//!
//! - [`memory`]: Core memory storage and retrieval
//! - [`extractor`]: Extract memories from task results
//! - [`store`]: High-level integration for the pipeline
//! - [`project`]: Project-level memory management
//! - [`run_memory`]: Run-scoped memory management
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::memory::{MemoryIntegration, MemoryEntry, ProjectMemory, RunMemory};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create memory integration
//! let memory = MemoryIntegration::new("./my-project")?;
//!
//! // Get context for agent prompt
//! let context = memory.get_context_for_prompt()?;
//!
//! // Project memory
//! let mut project = ProjectMemory::new("my-project");
//! project.save(&std::path::Path::new(".ltmatrix/memory/project.json")).await?;
//!
//! // Run memory
//! let run = RunMemory::with_mode("standard");
//! # Ok(())
//! # }
//! ```

pub mod extractor;
pub mod memory;
pub mod project;
pub mod run_memory;
pub mod store;

// Re-export main types for convenience
pub use extractor::{extract_files_affected, extract_memory_from_task, extract_task_summary};
pub use memory::{
    CodeSnippet, MemoryCategory, MemoryEntry, MemoryEntryBuilder, MemoryPriority, MemoryStore,
};
pub use project::{
    get_project_memory_path, ArchitectureDecision, CodingConventions, CompletedTask, ProjectMemory,
    ProjectStructure, TechStack,
};
pub use run_memory::{
    cleanup_old_run_memories, get_current_run_memory_path, get_run_memory_path, AgentSessionInfo,
    ContextDecision, RunMemory, RunStatus, SessionStats, TaskExecutionRecord,
};
pub use store::{
    calculate_max_memory_size, format_memory_for_prompt, should_inject_memory,
    truncate_memory_context, MemoryIntegration,
};
