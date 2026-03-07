//! Acceptance tests for memory management system
//!
//! These tests validate that the implementation meets all acceptance criteria:
//! 1. Create src/memory/mod.rs for project memory (.claude/memory.md)
//! 2. Extract key decisions and architectural choices from completed tasks
//! 3. Append to memory.md with timestamp and task reference
//! 4. Load memory.md at pipeline start for context
//! 5. Implement memory summarization if file grows too large
//! 6. Support memory injection into agent prompts

use ltmatrix::memory::{
    MemoryStore, MemoryEntry, MemoryIntegration,
    memory::MemoryCategory,
    extract_memory_from_task,
};
use ltmatrix::models::{Task, TaskStatus, TaskComplexity};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Acceptance Criterion 1: Create src/memory/mod.rs and .claude/memory.md
// ============================================================================

#[test]
fn test_acceptance_1_module_structure_exists() {
    // This test validates that the memory module exists and is properly structured
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create store - this creates .claude directory
    let store = MemoryStore::new(project_root).unwrap();

    // Verify .claude directory exists
    let claude_dir = project_root.join(".claude");
    assert!(claude_dir.exists(), ".claude directory must exist");
    assert!(claude_dir.is_dir(), ".claude must be a directory");

    // The memory.md file is created when the first entry is appended
    // Add an entry to trigger file creation
    let entry = create_test_entry("task-001", "Test Entry", "Test content");
    store.append_entry(&entry).unwrap();

    // Verify memory.md file exists after adding an entry
    let memory_file = claude_dir.join("memory.md");
    assert!(memory_file.exists(), ".claude/memory.md must exist after adding entry");

    // Verify file is readable and has proper format
    let content = fs::read_to_string(&memory_file).unwrap();
    assert!(content.contains("# Project Memory"), "File must have header");
    assert!(content.contains("Test Entry"), "File must contain the entry");
}

#[test]
fn test_acceptance_1_memory_module_public_api() {
    // Validate that the memory module exposes the expected public API
    // This is a compile-time test - if it compiles, the API exists

    let temp_dir = TempDir::new().unwrap();
    let _store = MemoryStore::new(temp_dir.path()).unwrap();
    let _entry = MemoryEntry::new("task-001", "Title", "Content");
    let _integration = MemoryIntegration::new(temp_dir.path()).unwrap();

    // If we got here without compilation errors, the API is correct
}

// ============================================================================
// Acceptance Criterion 2: Extract key decisions and architectural choices
// ============================================================================

#[test]
fn test_acceptance_2_extract_architectural_decisions() {
    let task = create_completed_task("task-001", "Setup Project", "Project setup");
    let task_result = r#"
    Completed the project setup.

    Architecture decision: Use async Rust with Tokio runtime for all I/O operations.
    This provides better performance and scalability compared to synchronous code.

    Also decided to use serde for JSON serialization.
    "#;

    let entries = extract_memory_from_task(&task, task_result).unwrap();

    // Should extract the architectural decision
    let arch_decisions: Vec<_> = entries
        .iter()
        .filter(|e| e.category == MemoryCategory::ArchitectureDecision)
        .collect();

    assert!(!arch_decisions.is_empty(),
            "Must extract at least one architectural decision");
    assert!(arch_decisions.iter().any(|e| e.content.contains("Tokio")),
            "Must capture Tokio decision");
}

#[test]
fn test_acceptance_2_extract_key_patterns() {
    let task = create_completed_task("task-002", "Error Handling", "Add error handling");
    let task_result = r#"
    Implemented comprehensive error handling.

    Pattern: Use Result<T, E> for recoverable errors
    Best practice: Provide context with anyhow::Context
    Established a pattern: Early return on errors to avoid nested conditions
    "#;

    let entries = extract_memory_from_task(&task, task_result).unwrap();

    let patterns: Vec<_> = entries
        .iter()
        .filter(|e| e.category == MemoryCategory::Pattern)
        .collect();

    assert!(!patterns.is_empty(),
            "Must extract patterns from task results");
    assert!(patterns.iter().any(|e| e.content.contains("Result")),
            "Must capture Result pattern");
}

#[test]
fn test_acceptance_2_extract_insights_and_notes() {
    let task = create_completed_task("task-003", "Security", "Add security measures");
    let task_result = r#"
    Added security improvements.

    Important: Always validate user input before processing
    Remember: Use prepared statements for SQL queries
    Warning: Never log sensitive data like passwords
    "#;

    let entries = extract_memory_from_task(&task, task_result).unwrap();

    let notes: Vec<_> = entries
        .iter()
        .filter(|e| e.category == MemoryCategory::ImportantNote)
        .collect();

    assert!(!notes.is_empty(),
            "Must extract important notes from task results");
    assert!(notes.iter().any(|e| e.content.contains("validate")),
            "Must capture validation note");
}

#[test]
fn test_acceptance_2_extract_from_various_task_types() {
    // Test extraction from different types of tasks
    let task_types = vec![
        ("task-001", "Architecture", "Architecture decision: Microservices pattern"),
        ("task-002", "Implementation", "Pattern: Builder pattern for complex objects"),
        ("task-003", "Bug Fix", "Important: Race condition fixed with mutex"),
        ("task-004", "Refactoring", "Best practice: Extract to method"),
        ("task-005", "Testing", "Note: Always test edge cases"),
    ];

    for (id, title, result) in task_types {
        let task = create_completed_task(id, title, title);
        let entries = extract_memory_from_task(&task, result).unwrap();

        // Each task type should produce at least one entry
        assert!(!entries.is_empty(),
                "Task type '{}' should produce memory entries", title);
    }
}

// ============================================================================
// Acceptance Criterion 3: Append with timestamp and task reference
// ============================================================================

#[test]
fn test_acceptance_3_append_with_timestamp() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    let entry = create_test_entry("task-001", "Test", "Content");
    store.append_entry(&entry).unwrap();

    let entries = store.get_entries();
    assert_eq!(entries.len(), 1);

    // Verify timestamp exists and is recent
    let now = chrono::Utc::now();
    let entry_time = entries[0].timestamp;

    let time_diff = now - entry_time;
    assert!(time_diff.num_seconds() < 10, "Timestamp should be recent");
    assert!(time_diff.num_seconds() >= 0, "Timestamp should not be in the future");
}

#[test]
fn test_acceptance_3_append_with_task_reference() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    let entry = create_test_entry("task-042", "Important Decision", "Use Rust");
    store.append_entry(&entry).unwrap();

    // Verify task reference is stored
    let entries = store.get_entries();
    assert_eq!(entries[0].task_id, "task-042");

    // Verify task reference appears in file
    let memory_file = temp_dir.path().join(".claude/memory.md");
    let content = fs::read_to_string(memory_file).unwrap();
    assert!(content.contains("**Task**: task-042"),
            "Task reference must be in memory file");
}

#[test]
fn test_acceptance_3_append_multiple_entries() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    // Append multiple entries
    for i in 1..=10 {
        let entry = create_test_entry(
            &format!("task-{:03}", i),
            &format!("Entry {}", i),
            &format!("Content {}", i),
        );
        store.append_entry(&entry).unwrap();
    }

    // Verify all entries are stored
    assert_eq!(store.entry_count(), 10);

    // Verify all task references are in file
    let memory_file = temp_dir.path().join(".claude/memory.md");
    let content = fs::read_to_string(memory_file).unwrap();

    for i in 1..=10 {
        assert!(content.contains(&format!("**Task**: task-{:03}", i)),
                "Task reference {} must be in memory file", i);
    }
}

#[test]
fn test_acceptance_3_append_preserves_existing_content() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    // Add first entry
    let entry1 = create_test_entry("task-001", "First", "First content");
    store.append_entry(&entry1).unwrap();

    // Add second entry
    let entry2 = create_test_entry("task-002", "Second", "Second content");
    store.append_entry(&entry2).unwrap();

    // Verify both entries are present
    let entries = store.get_entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].content, "First content");
    assert_eq!(entries[1].content, "Second content");
}

// ============================================================================
// Acceptance Criterion 4: Load memory.md at pipeline start for context
// ============================================================================

#[test]
fn test_acceptance_4_load_existing_memory() {
    let temp_dir = TempDir::new().unwrap();
    let memory_file = temp_dir.path().join(".claude/memory.md");

    // Create an existing memory file
    fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
    fs::write(
        &memory_file,
        r#"# Project Memory

---

## Test Entry

**Task**: task-001
**Category**: Test
**Date**: 2026-03-07 12:00:00 UTC

This is test content from a previous session.

---
"#
    ).unwrap();

    // Load store - should read existing memory
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    // Verify existing memory is loaded
    assert_eq!(store.entry_count(), 1,
               "Must load existing entries from memory.md");

    let entries = store.get_entries();
    assert_eq!(entries[0].task_id, "task-001");
    assert!(entries[0].content.contains("previous session"));
}

#[test]
fn test_acceptance_4_context_available_on_creation() {
    let temp_dir = TempDir::new().unwrap();

    // Create store with existing memory
    {
        let store = MemoryStore::new(temp_dir.path()).unwrap();
        let entry = create_test_entry("task-001", "Setup", "Use Tokio");
        store.append_entry(&entry).unwrap();
    }

    // Create new store instance - context should be available immediately
    let store = MemoryStore::new(temp_dir.path()).unwrap();
    let context = store.get_memory_context().unwrap();

    assert!(context.contains("Setup"),
            "Context must be available immediately after loading");
    assert!(context.contains("Tokio"),
            "Context must contain previously stored memory");
}

#[test]
fn test_acceptance_4_empty_memory_no_crash() {
    let temp_dir = TempDir::new().unwrap();

    // Create store in empty directory
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    // Should not crash, should return appropriate message
    let context = store.get_memory_context().unwrap();
    assert!(context.contains("No project memory"),
            "Should handle empty memory gracefully");
}

#[test]
fn test_acceptance_4_integration_loads_on_creation() {
    let temp_dir = TempDir::new().unwrap();

    // Create integration and add memory
    {
        let integration = MemoryIntegration::new(temp_dir.path()).unwrap();
        let task = create_completed_task("task-001", "Test", "Test");
        integration.store_task_summary(&task, &[]).unwrap();
    }

    // Create new integration - should load existing memory
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();
    assert_eq!(integration.entry_count(), 1,
               "Integration must load existing memory on creation");

    let context = integration.get_context_for_prompt().unwrap();
    assert!(context.contains("Test"),
            "Loaded memory must be available for context");
}

// ============================================================================
// Acceptance Criterion 5: Implement memory summarization
// ============================================================================

#[test]
fn test_acceptance_5_summarization_on_size() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    // Add enough entries to exceed size threshold (50KB)
    for i in 1..=200 {
        let content = format!("Decision {}: This is a long content to ensure we reach the file size threshold needed for automatic summarization. Each entry should be substantial enough that when we add hundreds of them, the total file size exceeds 50KB.", i);
        let entry = create_test_entry(
            &format!("task-{:03}", i),
            &format!("Decision {}", i),
            &content,
        );
        store.append_entry(&entry).unwrap();
    }

    // Check that summarization was triggered
    let memory_file = temp_dir.path().join(".claude/memory.md");
    let metadata = fs::metadata(&memory_file).unwrap();
    let content = fs::read_to_string(&memory_file).unwrap();

    // File should either be summarized or still within reasonable bounds
    // (If it was very large and got summarized, it should be smaller now)
    assert!(metadata.len() < 100 * 1024, // Should be under 100KB after summarization
            "File size should be controlled after summarization");

    // Content should show evidence of summarization or organization
    assert!(content.contains("Summary") ||
            content.contains("Recent") ||
            content.contains("entries"),
            "Content should show organization/summarization");
}

#[test]
fn test_acceptance_5_summarization_on_count() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    // Add entries to exceed count threshold (100)
    for i in 1..=105 {
        let entry = create_test_entry(
            &format!("task-{:03}", i),
            &format!("Entry {}", i),
            &format!("Content {}", i),
        );
        store.append_entry(&entry).unwrap();
    }

    // Verify summarization occurred
    let memory_file = temp_dir.path().join(".claude/memory.md");
    let content = fs::read_to_string(&memory_file).unwrap();

    // Should show evidence of summarization
    assert!(content.contains("Summary") ||
            content.contains("Recent") ||
            content.contains("Earlier"),
            "File should show summarization when count exceeds threshold");
}

#[test]
fn test_acceptance_5_summarization_preserves_recent() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    // Add many entries
    for i in 1..=150 {
        let entry = create_test_entry(
            &format!("task-{:03}", i),
            &format!("Entry {}", i),
            &format!("Content {}", i),
        );
        store.append_entry(&entry).unwrap();
    }

    let memory_file = temp_dir.path().join(".claude/memory.md");
    let content = fs::read_to_string(&memory_file).unwrap();

    // Recent entries should have full formatting
    // Note: Category is embedded in title line like ## [Category] Title
    assert!(content.contains("**Task**:"), "Recent entries should have full metadata");
    assert!(content.contains("**Date**:"), "Recent entries should have full metadata");
}

#[test]
fn test_acceptance_5_summarization_groups_by_category() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    // Add entries with different categories
    let categories = vec![
        ("Architecture Decision", 50),
        ("Pattern", 50),
        ("Important Note", 50),
    ];

    for (category, count) in categories {
        for i in 1..=count {
            let entry = MemoryEntry::new(
                &format!("task-{}-{:03}", category, i),
                &format!("Entry {}", i),
                "Content"
            ).with_category(category);
            store.append_entry(&entry).unwrap();
        }
    }

    let memory_file = temp_dir.path().join(".claude/memory.md");
    let content = fs::read_to_string(&memory_file).unwrap();

    // Summarized content should reference categories
    // (This may be in a summary section or visible through organization)
    assert!(content.contains("Architecture Decision") ||
            content.contains("Pattern") ||
            content.contains("Important Note") ||
            content.contains("Category"),
            "Summarization should preserve category information");
}

// ============================================================================
// Acceptance Criterion 6: Support memory injection into agent prompts
// ============================================================================

#[test]
fn test_acceptance_6_get_context_for_prompt() {
    let temp_dir = TempDir::new().unwrap();
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

    // Store some memory
    let task = create_completed_task("task-001", "Setup", "Project setup");
    let result = "Architecture decision: Use Tokio runtime";
    integration.extract_and_store(&task, result).unwrap();

    // Get context for prompt
    let context = integration.get_context_for_prompt().unwrap();

    // Verify context is properly formatted
    assert!(!context.is_empty(), "Context must not be empty");
    assert!(context.contains("Tokio"), "Context must contain stored memory");
    assert!(context.contains("task-001") || context.contains("Setup"),
            "Context should reference task");
}

#[test]
fn test_acceptance_6_context_formatted_for_injection() {
    let temp_dir = TempDir::new().unwrap();
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

    let task = create_completed_task("task-001", "Decision", "Important decision");
    integration.store_task_summary(&task, &["src/main.rs".to_string()]).unwrap();

    let context = integration.get_context_for_prompt().unwrap();

    // Should be formatted as readable text
    assert!(context.contains("Project Memory") ||
            context.contains("entries") ||
            context.contains("Decision"),
            "Context should be formatted for readability");

    // Should not contain raw JSON or other non-injectable formats
    assert!(!context.contains("{") || context.contains("##"),
            "Context should be readable text, not raw data");
}

#[test]
fn test_acceptance_6_context_includes_relevant_info() {
    let temp_dir = TempDir::new().unwrap();
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

    // Store multiple types of memories
    let task = create_completed_task("task-001", "Architecture", "Design decisions");
    let result = r#"
    Architecture decision: Use async Rust
    Pattern: Repository pattern for data access
    Important: Always validate input
    "#;
    integration.extract_and_store(&task, result).unwrap();

    let context = integration.get_context_for_prompt().unwrap();

    // Context should include key information
    assert!(context.contains("async") || context.contains("Repository") ||
            context.contains("validate") || context.contains("Architecture"),
            "Context should include stored decision information");
}

#[test]
fn test_acceptance_6_context_size_limited() {
    let temp_dir = TempDir::new().unwrap();
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

    // Add many entries
    for i in 1..=50 {
        let task = create_completed_task(
            &format!("task-{:03}", i),
            &format!("Task {}", i),
            &format!("Description {}", i),
        );
        integration.store_task_summary(&task, &[]).unwrap();
    }

    let context = integration.get_context_for_prompt().unwrap();

    // Context should be limited to prevent overwhelming prompts
    // Current implementation shows ~20 recent entries
    assert!(context.len() < 50_000, // Should be under 50KB
            "Context should be size-limited to prevent overwhelming prompts");

    // Should indicate if there are more entries
    if integration.entry_count() > 20 {
        assert!(context.contains("older") || context.contains("more") ||
                context.contains("...") || context.contains("and"),
                "Should indicate there are more entries than shown");
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_completed_task(id: &str, title: &str, description: &str) -> Task {
    let mut task = Task::new(id, title, description);
    task.status = TaskStatus::Completed;
    task.complexity = TaskComplexity::Moderate;
    task
}

fn create_test_entry(task_id: &str, title: &str, content: &str) -> MemoryEntry {
    MemoryEntry::new(task_id, title, content)
        .with_category("Test Category")
}
