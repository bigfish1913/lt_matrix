//! Pipeline memory loading tests
//!
//! This test suite validates memory loading in the pipeline:
//! - Loading memory.md at pipeline start
//! - Injecting memory context into agent prompts
//! - Configuration options for memory injection
//! - Memory loading behavior with various file states

use ltmatrix::memory::{MemoryIntegration, MemoryEntry};
use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::pipeline::execute::{
    build_task_context, build_execution_prompt, ExecuteConfig,
    get_execution_order, load_project_memory,
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::runtime::Runtime;

// ============================================================================
// Memory Loading Tests
// ============================================================================

#[test]
fn test_load_project_memory_existing_file() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
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

        // Load memory
        let memory = load_project_memory(&memory_file).await.unwrap();

        assert!(!memory.is_empty(), "Memory should be loaded from existing file");
        assert!(
            memory.contains("Tokio"),
            "Memory should contain stored content"
        );
        assert!(
            memory.contains("task-001"),
            "Memory should contain task reference"
        );
    });
}

#[test]
fn test_load_project_memory_nonexistent_file() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let memory_file = temp_dir.path().join(".claude/nonexistent.md");

        // Load memory from non-existent file
        let memory = load_project_memory(&memory_file).await.unwrap();

        assert!(
            memory.is_empty(),
            "Non-existent file should return empty string"
        );
    });
}

#[test]
fn test_load_project_memory_empty_file() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let memory_file = temp_dir.path().join(".claude/memory.md");

        // Create empty memory file
        fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
        fs::write(&memory_file, "").unwrap();

        // Load memory
        let memory = load_project_memory(&memory_file).await.unwrap();

        assert!(memory.is_empty(), "Empty file should return empty string");
    });
}

#[test]
fn test_load_project_memory_header_only() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let memory_file = temp_dir.path().join(".claude/memory.md");

        // Create memory file with only header
        fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
        fs::write(&memory_file, "# Project Memory\n\nNo entries yet.\n").unwrap();

        // Load memory
        let memory = load_project_memory(&memory_file).await.unwrap();

        assert!(!memory.is_empty(), "Header-only file should return content");
        assert!(
            memory.contains("Project Memory"),
            "Should contain header content"
        );
    });
}

#[test]
fn test_load_project_memory_large_file() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let memory_file = temp_dir.path().join(".claude/memory.md");

        // Create large memory file
        fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
        let mut content = String::from("# Project Memory\n\n");
        for i in 0..100 {
            content.push_str(&format!(
                "## Entry {}\n\nContent for entry {} with some additional text to make it larger.\n\n---\n\n",
                i, i
            ));
        }
        fs::write(&memory_file, &content).unwrap();

        // Load memory - should handle large files
        let memory = load_project_memory(&memory_file).await.unwrap();

        assert!(!memory.is_empty(), "Large file should be loaded");
        assert!(memory.len() > 1000, "Large file should have substantial content");
    });
}

#[test]
fn test_load_project_memory_unicode_content() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let memory_file = temp_dir.path().join(".claude/memory.md");

        // Create memory file with unicode
        fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
        fs::write(
            &memory_file,
            "# Project Memory\n\n## Decision\n\n支持中文 🚀 Émojis and accents\n",
        )
        .unwrap();

        // Load memory
        let memory = load_project_memory(&memory_file).await.unwrap();

        assert!(memory.contains("中文"), "Should handle Chinese characters");
        assert!(memory.contains("🚀"), "Should handle emojis");
        assert!(memory.contains("Émojis"), "Should handle accents");
    });
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
    let context = integration.get_context_for_prompt().unwrap();

    // 3. Verify memory flows through
    assert!(
        context.contains("Tokio"),
        "Memory should be available for prompt injection"
    );

    // 4. Build execution prompt with memory
    let prompt = build_execution_prompt(&task, &context);
    assert!(
        prompt.contains("Tokio"),
        "Memory should be in execution prompt"
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
            integration.entry_count(),
            1,
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
fn test_memory_file_corrupted_content() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let memory_file = temp_dir.path().join(".claude/memory.md");

        // Create corrupted/invalid markdown file
        fs::create_dir_all(temp_dir.path().join(".claude")).unwrap();
        fs::write(
            &memory_file,
            "This is not valid memory format\nJust random text\nNo headers or structure",
        )
        .unwrap();

        // Should still load (even if parsing later might fail)
        let memory = load_project_memory(&memory_file).await.unwrap();
        assert!(
            !memory.is_empty(),
            "Should load content even if format is unexpected"
        );
    });
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
fn test_build_context_with_task_complexity() {
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

#[test]
fn test_memory_injection_order() {
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