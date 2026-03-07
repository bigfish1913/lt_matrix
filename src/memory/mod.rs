//! Project memory management
//!
//! This module handles persistent memory storage and retrieval for architectural decisions,
//! patterns, and insights accumulated during task execution.
//!
//! # Overview
//!
//! The memory system maintains a `.claude/memory.md` file that accumulates knowledge across
//! task runs. Key features include:
//!
//! - **Persistent storage**: Memory is stored in `.claude/memory.md`
//! - **Timestamped entries**: Each memory entry includes timestamp and task reference
//! - **Automatic summarization**: Large memory files are automatically summarized
//! - **Context injection**: Memory can be injected into agent prompts for context
//! - **Memory extraction**: Automatically extracts decisions, patterns, and notes from task results
//!
//! # Modules
//!
//! - [`memory`]: Core memory storage and retrieval
//! - [`extractor`]: Extract memories from task results
//! - [`store`]: High-level integration for the pipeline
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::memory::{MemoryIntegration, MemoryEntry};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create memory integration
//! let memory = MemoryIntegration::new("./my-project")?;
//!
//! // Get context for agent prompt
//! let context = memory.get_context_for_prompt()?;
//!
//! // Extract and store memories from a completed task
//! // let task = /* ... */;
//! // let result = "Architecture decision: Using Tokio for async";
//! // memory.extract_and_store(&task, result)?;
//! # Ok(())
//! # }
//! ```

pub mod extractor;
pub mod memory;
pub mod store;

// Re-export main types for convenience
pub use memory::{
    MemoryEntry, MemoryStore, MemoryCategory, MemoryPriority,
    CodeSnippet, MemoryEntryBuilder,
};
pub use extractor::{
    extract_memory_from_task, extract_task_summary,
    extract_files_affected,
};
pub use store::{
    MemoryIntegration,
    format_memory_for_prompt,
    should_inject_memory,
    calculate_max_memory_size,
    truncate_memory_context,
};
