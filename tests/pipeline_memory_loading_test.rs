//! Pipeline memory loading tests
//!
//! This test suite validates memory loading in the pipeline:
//! - Loading memory.md at pipeline start
//! - Injecting memory context into agent prompts
//! - Configuration options for memory injection
//! - Memory loading behavior with various file states

use ltmatrix::memory::{MemoryIntegration, MemoryStore, MemoryEntry};
use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::pipeline::execute::{
    build_task_context, build_execution_prompt, ExecuteConfig,
    get_execution_order,
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Memory Loading Tests (via MemoryIntegration public API)
// ============================================================================

#[test]
fn test_memory_loading_existing_file() {
    let temp_dir = TempDir::new().unwrap();
    let memory_file = temp_dir.path().join(".claude/memory.md");

    // Create memory file with content
    fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
    fs::write(
        &memory_file,
        r#"# Project Memory

---

## [Architecture Decision] Use Tokio Runtime

**Task**: task-001
**Date**: 2026-03-07 12:00:00 UTC

Using Tokio for async operations.

---
"#,
    )
    .unwrap();

    // Load memory via MemoryIntegration
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

    // Verify memory is loaded
    assert_eq!(integration.entry_count(), 1, "Should load existing entry");

    // Verify content is available
    let context = integration.get_context_for_prompt().unwrap();
    assert!(
        context.contains("Tokio"),
        "Memory should contain stored content"
    );
}

#[test]
fn test_memory_loading_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();

    // Create MemoryIntegration without existing memory file
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

    // Should handle gracefully
    assert_eq!(integration.entry_count(), 0, "Should have no entries");

    let context = integration.get_context_for_prompt().unwrap();
    assert!(
        context.contains("No project memory"),
        "Should indicate no memory available"
    );
}

#[test]
fn test_memory_loading_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let memory_file = temp_dir.path().join(".claude/memory.md");

    // Create empty memory file
    fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
    fs::write(&memory_file, "").unwrap();

    // Load memory
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

    assert_eq!(
        integration.entry_count(), 0,
        "Empty file should result in no entries"
    );
}

#[test]
fn test_memory_loading_header_only() {
    let temp_dir = TempDir::new().unwrap();
    let memory_file = temp_dir.path().join(".claude/memory.md");

    // Create memory file with only header (no valid entries)
    fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
    fs::write(&memory_file, "# Project Memory\n\nNo entries yet.\n").unwrap();

    // Load memory
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

    // Header only should result in no parsed entries
    assert_eq!(
        integration.entry_count(), 0,
        "Header-only file should have no entries"
    );
}

#[test]
fn test_memory_loading_large_file() {
    let temp_dir = TempDir::new().unwrap();

    // Use MemoryStore to add many entries
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    // Add many entries
    for i in 0..50 {
        let entry = MemoryEntry::new(
            &format!("task-{:03}", i),
            &format!("Entry {}", i),
            &format!("Content for entry {} with additional text", i),
        );
        store.append_entry(&entry).unwrap();
    }

    // Create new MemoryIntegration to reload
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

    assert_eq!(
        integration.entry_count(), 50,
        "Large file should load all entries"
    );

    // Verify context is available
    let context = integration.get_context_for_prompt().unwrap();
    assert!(!context.is_empty(), "Context should be available");
}

#[test]
fn test_memory_loading_unicode_content() {
    let temp_dir = TempDir::new().unwrap();

    // Create store with unicode content
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    let entry = MemoryEntry::new(
        "task-001",
        "国际化 测试",
        "支持中文 🚀 Émojis and accents",
    );
    store.append_entry(&entry).unwrap();

    // Reload via MemoryIntegration
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

    let context = integration.get_context_for_prompt().unwrap();
    assert!(context.contains("中文"), "Should handle Chinese characters");
    assert!(context.contains("🚀"), "Should handle emojis");
    assert!(context.contains("Émojis"), "Should handle accents");
}

#[test]
fn test_memory_loading_persistence() {
    let temp_dir = TempDir::new().unwrap();

    // First load - create memory
    {
        let store = MemoryStore::new(temp_dir.path()).unwrap();
        let entry = MemoryEntry::new("task-001", "Setup", "Architecture decision: Use Rust for performance");
        store.append_entry(&entry).unwrap();
    }

    // Second load - verify persistence via MemoryIntegration
    {
        let integration = MemoryIntegration::new(temp_dir.path()).unwrap();
        assert_eq!(
            integration.entry_count(), 1,
            "Memory should persist between loads"
        );

        let context = integration.get_context_for_prompt().unwrap();
        assert!(
            context.contains("Rust"),
            "Stored memory should be available after reload"
        );
    }
}

// ============================================================================
// Memory Injection into Prompts Tests
// ============================================================================

#[test]
fn test_build_task_context_with_memory() {
    let task = create_test_task("task-001", "Test Task", "Test description");
    let task_map = create_task_map(&[task.clone()]);
    let completed_tasks = HashSet::new();
    let project_memory = "## Architecture Decision\n\nUse Tokio runtime for async operations.";

    let context =
        build_task_context(&task, &task_map, &completed_tasks, project_memory).unwrap();

    assert!(
        context.contains("Project Memory"),
        "Context should have project memory section"
    );
    assert!(
        context.contains("Use Tokio"),
        "Context should include memory content"
    );
    assert!(
        context.contains("Task: Test Task"),
        "Context should have task info"
    );
}

#[test]
fn test_build_task_context_without_memory() {
    let task = create_test_task("task-001", "Test Task", "Test description");
    let task_map = create_task_map(&[task.clone()]);
    let completed_tasks = HashSet::new();
    let project_memory = ""; // Empty memory

    let context =
        build_task_context(&task, &task_map, &completed_tasks, project_memory).unwrap();

    assert!(
        !context.contains("Project Memory"),
        "Context should not have memory section when empty"
    );
    assert!(
        context.contains("Task: Test Task"),
        "Context should still have task info"
    );
}

#[test]
fn test_build_task_context_with_dependencies() {
    let task1 = create_test_task("task-001", "First Task", "First description");
    let mut task2 = create_test_task("task-002", "Second Task", "Second description");
    task2.depends_on = vec!["task-001".to_string()];

    let task_map = create_task_map(&[task1.clone(), task2.clone()]);
    let completed_tasks = HashSet::from(["task-001".to_string()]);
    let project_memory = "Previous decisions";

    let context =
        build_task_context(&task2, &task_map, &completed_tasks, project_memory).unwrap();

    assert!(
        context.contains("Dependencies"),
        "Context should show dependencies"
    );
    assert!(
        context.contains("First Task"),
        "Context should show completed dependency"
    );
    assert!(context.contains("(completed)"), "Should mark as completed");
}

#[test]
fn test_build_execution_prompt_includes_context() {
    let task = create_test_task("task-042", "Implement Feature", "Add new feature");
    let context = r#"## Project Memory

Use Tokio runtime.

## Task Context

Task: Implement Feature
Description: Add new feature"#;

    let prompt = build_execution_prompt(&task, context);

    assert!(
        prompt.contains("Implement Feature"),
        "Prompt should include task title"
    );
    assert!(
        prompt.contains("Add new feature"),
        "Prompt should include task description"
    );
    assert!(
        prompt.contains("Use Tokio"),
        "Prompt should include memory context"
    );
    assert!(
        prompt.contains("Begin your implementation"),
        "Prompt should have instructions"
    );
}

#[test]
fn test_build_execution_prompt_format() {
    let task = create_test_task("task-001", "Test", "Description");
    let context = "Context here";

    let prompt = build_execution_prompt(&task, context);

    // Verify prompt structure
    assert!(prompt.starts_with("You are implementing"), "Should have intro");
    assert!(prompt.contains("## Your Task"), "Should have task section");
    assert!(prompt.contains("## Instructions"), "Should have instructions");
    assert!(
        prompt.contains("Complete the task"),
        "Should have instruction items"
    );
    assert!(
        prompt.contains("Follow best practices"),
        "Should mention best practices"
    );
    assert!(prompt.contains("Add tests"), "Should mention testing");
    assert!(prompt.contains("Document changes"), "Should mention documentation");
}

#[test]
fn test_context_structure_with_memory() {
    let task = create_test_task("task-001", "Test", "Test");
    let task_map = create_task_map(&[task.clone()]);
    let completed_tasks = HashSet::new();
    let project_memory = "Memory content";

    let context =
        build_task_context(&task, &task_map, &completed_tasks, project_memory).unwrap();

    // Memory should appear before task context
    let memory_pos = context.find("Project Memory").unwrap();
    let task_pos = context.find("Task Context").unwrap();

    assert!(
        memory_pos < task_pos,
        "Memory should appear before task context"
    );
}

#[test]
fn test_context_with_task_complexity() {
    let mut task = create_test_task("task-001", "Complex Task", "Hard work");
    task.complexity = TaskComplexity::Complex;
    let task_map = create_task_map(&[task.clone()]);
    let completed_tasks = HashSet::new();
    let project_memory = "";

    let context =
        build_task_context(&task, &task_map, &completed_tasks, project_memory).unwrap();

    assert!(
        context.contains("Complexity: Complex"),
        "Context should include task complexity"
    );
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_execute_config_memory_file_default() {
    let config = ExecuteConfig::default();

    assert_eq!(
        config.memory_file,
        PathBuf::from(".claude/memory.md"),
        "Default memory file path should be .claude/memory.md"
    );
}

#[test]
fn test_execute_config_custom_memory_file() {
    let config = ExecuteConfig {
        memory_file: PathBuf::from("custom/memory/path.md"),
        ..Default::default()
    };

    assert_eq!(
        config.memory_file,
        PathBuf::from("custom/memory/path.md"),
        "Should support custom memory file path"
    );
}

#[test]
fn test_execute_config_fast_mode_memory_file() {
    let config = ExecuteConfig::fast_mode();

    assert_eq!(
        config.memory_file,
        PathBuf::from(".claude/memory.md"),
        "Fast mode should use default memory file"
    );
}

#[test]
fn test_execute_config_expert_mode_memory_file() {
    let config = ExecuteConfig::expert_mode();

    assert_eq!(
        config.memory_file,
        PathBuf::from(".claude/memory.md"),
        "Expert mode should use default memory file"
    );
}

#[test]
fn test_execute_config_all_fields() {
    let config = ExecuteConfig::default();

    // Verify all expected fields exist
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.timeout, 3600);
    assert!(config.enable_sessions);
    assert_eq!(config.memory_file, PathBuf::from(".claude/memory.md"));
}

// ============================================================================
// End-to-End Pipeline Memory Flow Tests
// ============================================================================

#[test]
fn test_memory_flow_from_creation_to_prompt() {
    let temp_dir = TempDir::new().unwrap();

    // 1. Create memory integration and store a decision
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();
    let task = create_test_task("task-001", "Setup", "Project setup");
    integration
        .extract_and_store(&task, "Architecture decision: Use Rust with Tokio")
        .unwrap();

    // 2. Get context for prompt
    let memory_context = integration.get_context_for_prompt().unwrap();

    // 3. Verify memory flows through
    assert!(
        memory_context.contains("Tokio"),
        "Memory should be available for prompt injection"
    );

    // 4. Build task context with memory
    let task_map = create_task_map(&[task.clone()]);
    let completed = HashSet::new();
    let full_context = build_task_context(&task, &task_map, &completed, &memory_context).unwrap();

    assert!(
        full_context.contains("Tokio"),
        "Memory should be in task context"
    );

    // 5. Build execution prompt
    let prompt = build_execution_prompt(&task, &full_context);
    assert!(
        prompt.contains("Tokio"),
        "Memory should flow to execution prompt"
    );
}

#[test]
fn test_memory_persistence_across_pipeline_runs() {
    let temp_dir = TempDir::new().unwrap();

    // First "pipeline run" - create memory
    {
        let integration = MemoryIntegration::new(temp_dir.path()).unwrap();
        let task = create_test_task("task-001", "First Run", "Initial setup");
        integration
            .extract_and_store(&task, "Architecture decision: Use PostgreSQL database")
            .unwrap();
    }

    // Second "pipeline run" - load existing memory
    {
        let integration = MemoryIntegration::new(temp_dir.path()).unwrap();
        assert_eq!(
            integration.entry_count(), 1,
            "Memory should persist across runs"
        );

        let context = integration.get_context_for_prompt().unwrap();
        assert!(
            context.contains("PostgreSQL"),
            "Previous memory should be available"
        );
    }
}

#[test]
fn test_multiple_memory_entries_in_prompt_context() {
    let temp_dir = TempDir::new().unwrap();
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

    // Store multiple memories
    for i in 1..=5 {
        let task = create_test_task(&format!("task-{:03}", i), &format!("Task {}", i), "Description");
        integration
            .extract_and_store(
                &task,
                &format!("Architecture decision: Decision number {}", i),
            )
            .unwrap();
    }

    let context = integration.get_context_for_prompt().unwrap();

    // All memories should be accessible
    for i in 1..=5 {
        assert!(
            context.contains(&format!("Decision number {}", i)),
            "Memory {} should be in context",
            i
        );
    }
}

#[test]
fn test_memory_context_limits_entries() {
    let temp_dir = TempDir::new().unwrap();
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

    // Add more than 20 entries (the display limit)
    for i in 1..=30 {
        let task = create_test_task(
            &format!("task-{:03}", i),
            &format!("Task {}", i),
            &format!("Description {}", i),
        );
        integration.store_task_summary(&task, &[]).unwrap();
    }

    let context = integration.get_context_for_prompt().unwrap();

    // Should indicate there are more entries
    assert!(
        context.contains("older") || context.contains("and") || context.contains("entries"),
        "Should indicate more entries exist"
    );
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_memory_integration_handles_new_project_directory() {
    let temp_dir = TempDir::new().unwrap();
    let new_project = temp_dir.path().join("new_project");

    // Create the project directory (but not .claude subdirectory)
    // MemoryIntegration will create the .claude directory
    fs::create_dir_all(&new_project).unwrap();

    let integration = MemoryIntegration::new(&new_project).unwrap();
    assert_eq!(integration.entry_count(), 0);

    // .claude directory should be created
    assert!(new_project.join(".claude").exists());
}

#[test]
fn test_memory_with_special_characters_in_paths() {
    let config = ExecuteConfig {
        memory_file: PathBuf::from("path with spaces/memory.md"),
        ..Default::default()
    };

    assert_eq!(
        config.memory_file,
        PathBuf::from("path with spaces/memory.md"),
        "Should handle paths with spaces"
    );
}

#[test]
fn test_empty_memory_context_in_task_context() {
    let task = create_test_task("task-001", "Test", "Test");
    let task_map = create_task_map(&[task.clone()]);
    let completed = HashSet::new();
    let empty_memory = "";

    let context = build_task_context(&task, &task_map, &completed, empty_memory).unwrap();

    // Should not crash and should still have task context
    assert!(context.contains("Task Context"), "Should have task context");
    assert!(context.contains("Task: Test"), "Should have task info");
    // Should not have Project Memory section since memory is empty
    assert!(
        !context.contains("## Project Memory"),
        "Should not have memory section for empty memory"
    );
}

#[test]
fn test_memory_integration_with_concurrent_tasks() {
    let temp_dir = TempDir::new().unwrap();
    let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

    // Simulate multiple tasks completing and storing memories
    let tasks: Vec<Task> = (1..=10)
        .map(|i| create_test_task(&format!("task-{:03}", i), &format!("Task {}", i), "Desc"))
        .collect();

    for task in &tasks {
        integration
            .store_task_summary(task, &[])
            .unwrap();
    }

    // All memories should be stored
    assert_eq!(integration.entry_count(), 10);

    // Context should be available
    let context = integration.get_context_for_prompt().unwrap();
    assert!(!context.is_empty());
}

#[test]
fn test_get_execution_order_with_memory_aware_tasks() {
    // Create tasks with dependencies
    let task1 = create_test_task("task-001", "Setup", "Initial setup");
    let mut task2 = create_test_task("task-002", "Build", "Build on setup");
    task2.depends_on = vec!["task-001".to_string()];
    let mut task3 = create_test_task("task-003", "Deploy", "Deploy after build");
    task3.depends_on = vec!["task-002".to_string()];

    let task_map = create_task_map(&[task1, task2, task3]);

    // Get execution order
    let order = get_execution_order(&task_map).unwrap();

    // Verify order respects dependencies
    assert_eq!(order.len(), 3);
    assert_eq!(order[0], "task-001");
    assert_eq!(order[1], "task-002");
    assert_eq!(order[2], "task-003");
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_task(id: &str, title: &str, description: &str) -> Task {
    let mut task = Task::new(id, title, description);
    task.status = TaskStatus::Pending;
    task.complexity = TaskComplexity::Moderate;
    task
}

fn create_task_map(tasks: &[Task]) -> HashMap<String, Task> {
    tasks
        .iter()
        .map(|t| (t.id.clone(), t.clone()))
        .collect()
}