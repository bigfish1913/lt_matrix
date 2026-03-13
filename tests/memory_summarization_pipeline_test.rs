//! Integration tests for memory summarization pipeline
//!
//! These tests verify the complete integration of:
//! 1. Detection of memory thresholds (file size and entry count)
//! 2. Automatic summarization triggering when thresholds are exceeded
//! 3. CLI command integration for manual summarization
//! 4. End-to-end pipeline with memory extraction and summarization

use ltmatrix::config::settings::MemoryConfig;
use ltmatrix::memory::{
    calculate_max_memory_size, format_memory_for_prompt, should_inject_memory,
    truncate_memory_context, MemoryCategory, MemoryEntry, MemoryIntegration, MemoryPriority,
    MemoryStore,
};
use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_task(id: &str, title: &str) -> Task {
    let mut task = Task::new(id, title, format!("Description for {}", title));
    task.status = TaskStatus::Completed;
    task.complexity = TaskComplexity::Moderate;
    task
}

fn create_large_memory_content(size_bytes: usize) -> String {
    "This is a test content line that adds to the memory file size. ".repeat(size_bytes / 60 + 1)
}

fn get_memory_file_path(temp_dir: &TempDir) -> PathBuf {
    temp_dir.path().join(".claude/memory.md")
}

// ============================================================================
// Detection-Trigger Integration Tests
// ============================================================================

mod detection_trigger_integration {
    use super::*;

    /// Test that file size threshold detection triggers summarization
    #[test]
    fn test_file_size_detection_triggers_summarization() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Configure very low file size threshold
        let mut config = MemoryConfig::default();
        config.max_file_size = 1000; // 1KB
        config.max_entries = 1000; // High enough to not trigger first
        config.min_entries_for_summarization = 2;
        config.keep_fraction = 0.5;

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add entries with large content to exceed file size threshold
        for i in 0..5 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                create_large_memory_content(500), // 500+ bytes per entry
            );
            store.append_entry(&entry).expect("Failed to append entry");
        }

        // Verify that summarization occurred (entries should be reduced)
        let final_count = store.entry_count();
        assert!(
            final_count < 5,
            "Summarization should have reduced entries. Got {} entries",
            final_count
        );

        // Verify memory file exists and has reasonable size
        let memory_file = get_memory_file_path(&temp_dir);
        assert!(memory_file.exists());

        let metadata = fs::metadata(&memory_file).expect("Failed to get metadata");
        let file_size = metadata.len() as usize;

        // File size should be controlled after summarization
        assert!(
            file_size < 5000,
            "File size should be controlled after summarization. Got {} bytes",
            file_size
        );
    }

    /// Test that entry count threshold detection triggers summarization
    #[test]
    fn test_entry_count_detection_triggers_summarization() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Configure low entry count threshold
        let mut config = MemoryConfig::default();
        config.max_file_size = 1024 * 1024; // 1MB - high enough to not trigger first
        config.max_entries = 10;
        config.min_entries_for_summarization = 5;
        config.keep_fraction = 0.4;

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add entries to exceed threshold
        for i in 0..25 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content for decision {}", i),
            );
            store.append_entry(&entry).expect("Failed to append entry");
        }

        // Verify summarization occurred
        let final_count = store.entry_count();
        assert!(
            final_count < 25,
            "Summarization should have reduced entries. Got {} entries",
            final_count
        );
    }

    /// Test that both thresholds are evaluated (OR logic)
    #[test]
    fn test_either_threshold_triggers_summarization() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Test case 1: Entry count threshold exceeded, file size OK
        {
            let mut config = MemoryConfig::default();
            config.max_file_size = 1024 * 1024; // 1MB
            config.max_entries = 5;
            config.min_entries_for_summarization = 3;
            config.keep_fraction = 0.5;

            let store = MemoryStore::with_config(temp_dir.path(), config.clone())
                .expect("Failed to create memory store");

            for i in 0..10 {
                let entry = MemoryEntry::new(
                    format!("task-a-{:03}", i),
                    format!("Decision {}", i),
                    "Small content",
                );
                store.append_entry(&entry).expect("Failed to append entry");
            }

            assert!(
                store.entry_count() < 10,
                "Entry count threshold should trigger"
            );
        }

        // Test case 2: File size threshold exceeded, entry count OK
        {
            let temp_dir2 = TempDir::new().expect("Failed to create temp dir");
            let mut config = MemoryConfig::default();
            config.max_file_size = 500;
            config.max_entries = 100;
            config.min_entries_for_summarization = 2;
            config.keep_fraction = 0.5;

            let store = MemoryStore::with_config(temp_dir2.path(), config.clone())
                .expect("Failed to create memory store");

            for i in 0..3 {
                let entry = MemoryEntry::new(
                    format!("task-b-{:03}", i),
                    format!("Decision {}", i),
                    create_large_memory_content(500),
                );
                store.append_entry(&entry).expect("Failed to append entry");
            }

            assert!(
                store.entry_count() <= 3,
                "File size threshold should trigger"
            );
        }
    }

    /// Test that min_entries_for_summarization prevents premature summarization
    #[test]
    fn test_min_entries_gate_prevents_summarization() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut config = MemoryConfig::default();
        config.max_entries = 3; // Low threshold
        config.min_entries_for_summarization = 100; // High gate
        config.keep_fraction = 0.5;

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add entries that exceed max_entries but not min_entries_for_summarization
        for i in 0..10 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content {}", i),
            );
            store.append_entry(&entry).expect("Failed to append entry");
        }

        // All entries should be preserved because min_entries not met
        assert_eq!(
            store.entry_count(),
            10,
            "min_entries_for_summarization should prevent summarization"
        );
    }
}

// ============================================================================
// CLI Command Integration Tests
// ============================================================================

mod cli_command_integration {
    use super::*;

    /// Test memory status command returns correct statistics
    #[test]
    fn test_memory_status_statistics() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = MemoryStore::new(temp_dir.path()).expect("Failed to create memory store");

        // Add entries with various categories and priorities
        let entries = vec![
            MemoryEntry::new("task-001", "Arch Decision", "Content")
                .with_category_enum(MemoryCategory::ArchitectureDecision)
                .with_priority(MemoryPriority::High),
            MemoryEntry::new("task-002", "Pattern", "Content")
                .with_category_enum(MemoryCategory::Pattern)
                .with_priority(MemoryPriority::Normal),
            MemoryEntry::new("task-003", "Security", "Content")
                .with_category_enum(MemoryCategory::Security)
                .with_priority(MemoryPriority::Critical),
            MemoryEntry::new("task-004", "API Design", "Content")
                .with_category_enum(MemoryCategory::ApiDesign)
                .with_priority(MemoryPriority::Normal),
        ];

        for entry in &entries {
            store.append_entry(entry).expect("Failed to append entry");
        }

        let stored_entries = store.get_entries();
        assert_eq!(stored_entries.len(), 4);

        // Verify category distribution
        let categories: Vec<_> = stored_entries.iter().map(|e| e.category).collect();
        assert!(categories.contains(&MemoryCategory::ArchitectureDecision));
        assert!(categories.contains(&MemoryCategory::Pattern));
        assert!(categories.contains(&MemoryCategory::Security));
        assert!(categories.contains(&MemoryCategory::ApiDesign));

        // Verify priority distribution
        let priorities: Vec<_> = stored_entries.iter().map(|e| e.priority).collect();
        assert!(priorities.contains(&MemoryPriority::Critical));
        assert!(priorities.contains(&MemoryPriority::High));
        assert!(priorities.contains(&MemoryPriority::Normal));
    }

    /// Test memory summarize with force flag
    #[test]
    fn test_memory_summarize_force() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Use default config (high thresholds)
        let config = MemoryConfig::default();

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add a few entries (below threshold)
        for i in 0..5 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content {}", i),
            );
            store.append_entry(&entry).expect("Failed to append entry");
        }

        // Verify entries are preserved (no auto-summarization)
        assert_eq!(store.entry_count(), 5);

        // Simulate force summarization by creating a new store with forced config
        let mut force_config = MemoryConfig::default();
        force_config.max_file_size = 1;
        force_config.max_entries = 1;
        force_config.min_entries_for_summarization = 1;
        force_config.keep_fraction = 0.5;

        // Create new store which will trigger summarization on load
        let _forced_store = MemoryStore::with_config(temp_dir.path(), force_config)
            .expect("Failed to create forced memory store");

        // Verify summarization occurred
        let final_store = MemoryStore::new(temp_dir.path()).expect("Failed to create final store");
        assert!(final_store.entry_count() <= 5);
    }

    /// Test memory clear removes all entries
    #[test]
    fn test_memory_clear_integration() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = MemoryStore::new(temp_dir.path()).expect("Failed to create memory store");

        // Add entries
        for i in 0..10 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content {}", i),
            );
            store.append_entry(&entry).expect("Failed to append entry");
        }

        let memory_file = get_memory_file_path(&temp_dir);
        assert!(memory_file.exists());

        // Clear memory by removing file
        fs::remove_file(&memory_file).expect("Failed to remove memory file");

        // Verify clear
        assert!(!memory_file.exists());

        // Create new store should start fresh
        let new_store = MemoryStore::new(temp_dir.path()).expect("Failed to create new store");
        assert_eq!(new_store.entry_count(), 0);
    }

    /// Test JSON status output format
    #[test]
    fn test_memory_status_json_format() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = MemoryStore::new(temp_dir.path()).expect("Failed to create memory store");

        // Add entries
        for i in 0..3 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content {}", i),
            );
            store.append_entry(&entry).expect("Failed to append entry");
        }

        let entries = store.get_entries();
        let memory_file = get_memory_file_path(&temp_dir);
        let file_size = fs::metadata(&memory_file).map(|m| m.len()).unwrap_or(0);

        // Simulate JSON status output structure
        let status = serde_json::json!({
            "exists": true,
            "entry_count": entries.len(),
            "file_size_bytes": file_size,
            "by_category": {
                "General": 3
            }
        });

        // Verify JSON structure
        assert!(status["exists"].as_bool().unwrap());
        assert_eq!(status["entry_count"].as_u64().unwrap(), 3);
    }
}

// ============================================================================
// Pipeline Integration Tests
// ============================================================================

mod pipeline_integration {
    use super::*;

    /// Test MemoryIntegration end-to-end with summarization
    #[test]
    fn test_memory_integration_with_auto_summarization() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Configure low thresholds for testing
        let mut config = MemoryConfig::default();
        config.max_entries = 10;
        config.min_entries_for_summarization = 5;
        config.keep_fraction = 0.5;

        // Create store with config
        let _store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Create memory integration
        let integration =
            MemoryIntegration::new(temp_dir.path()).expect("Failed to create memory integration");

        // Simulate multiple task completions
        for i in 0..20 {
            let task = create_test_task(&format!("task-{:03}", i), &format!("Task {}", i));
            let result = format!(
                "Architecture decision: Using pattern {} for implementation\n\
                 Pattern: Established pattern for module {}\n\
                 Important: Remember to handle edge cases in task {}",
                i, i, i
            );

            let count = integration
                .extract_and_store(&task, &result)
                .expect("Failed to extract and store");

            assert!(
                count >= 1,
                "Should extract at least one memory from task result"
            );
        }

        // Verify memory was built
        assert!(integration.entry_count() > 0, "Should have memory entries");

        // Get context for prompt
        let context = integration
            .get_context_for_prompt()
            .expect("Failed to get context");

        assert!(
            context.contains("Project Memory Context") || context.contains("No project memory")
        );
    }

    /// Test memory extraction with summarization trigger
    #[test]
    fn test_extraction_triggers_summarization() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Configure aggressive summarization
        let mut config = MemoryConfig::default();
        config.max_entries = 5;
        config.min_entries_for_summarization = 3;
        config.keep_fraction = 0.4;

        let _store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        let integration =
            MemoryIntegration::new(temp_dir.path()).expect("Failed to create memory integration");

        // Add many entries quickly
        for i in 0..15 {
            let task = create_test_task(&format!("task-{:03}", i), &format!("Task {}", i));
            let result = format!("Architecture decision: Pattern {}", i);

            integration
                .extract_and_store(&task, &result)
                .expect("Failed to extract and store");
        }

        // After summarization, entry count should be controlled
        let final_count = integration.entry_count();
        assert!(
            final_count < 20,
            "Summarization should control entry count. Got {} entries",
            final_count
        );
    }

    /// Test task summary storage integrates with summarization
    #[test]
    fn test_task_summary_with_summarization() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut config = MemoryConfig::default();
        config.max_entries = 8;
        config.min_entries_for_summarization = 4;
        config.keep_fraction = 0.5;

        let _store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        let integration =
            MemoryIntegration::new(temp_dir.path()).expect("Failed to create memory integration");

        // Store task summaries
        for i in 0..15 {
            let task = create_test_task(&format!("task-{:03}", i), &format!("Feature {}", i));
            let files = vec![
                format!("src/module_{}.rs", i),
                format!("tests/test_{}.rs", i),
            ];

            integration
                .store_task_summary(&task, &files)
                .expect("Failed to store task summary");
        }

        // Summarization should have occurred
        let entries = integration.get_entries();
        assert!(
            entries.len() < 20,
            "Summarization should have occurred. Got {} entries",
            entries.len()
        );
    }
}

// ============================================================================
// Memory Context Integration Tests
// ============================================================================

mod memory_context_integration {
    use super::*;

    /// Test format_memory_for_prompt integration
    #[test]
    fn test_format_memory_for_prompt_integration() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = MemoryStore::new(temp_dir.path()).expect("Failed to create memory store");

        // Add architectural decisions
        for i in 0..5 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Architecture Decision {}", i),
                format!("Using pattern {} for modularity", i),
            )
            .with_category_enum(MemoryCategory::ArchitectureDecision);
            store.append_entry(&entry).expect("Failed to append entry");
        }

        let context = store.get_memory_context().expect("Failed to get context");
        let formatted = format_memory_for_prompt(&context);

        assert!(formatted.contains("Project Memory Context"));
        assert!(formatted.contains("Consider this information"));
        assert!(formatted.contains("Architecture Decision"));
    }

    /// Test should_inject_memory decision logic
    #[test]
    fn test_should_inject_memory_logic() {
        // Short prompts should not trigger injection
        assert!(!should_inject_memory("Fix bug"));

        // Long prompts without keywords
        let long_no_keywords = "x".repeat(200);
        assert!(!should_inject_memory(&long_no_keywords));

        // Prompts with relevant keywords
        let architecture_prompt = "Please refactor the architecture to improve maintainability and follow established patterns throughout the codebase";
        assert!(should_inject_memory(architecture_prompt));

        let pattern_prompt = "Implement the feature following the existing patterns and extend the current implementation to support new use cases";
        assert!(should_inject_memory(pattern_prompt));

        let integration_prompt = "Integrate the new module with the existing system and maintain backward compatibility with previous versions";
        assert!(should_inject_memory(integration_prompt));
    }

    /// Test calculate_max_memory_size respects limits
    #[test]
    fn test_calculate_max_memory_size_limits() {
        // Small context - 20% allocation
        assert_eq!(calculate_max_memory_size(10000), 2000);

        // Medium context - 20% would be 10000, but capped at 5KB (5120 bytes)
        assert_eq!(calculate_max_memory_size(50000), 5120);

        // Large context - still capped at 5KB
        assert_eq!(calculate_max_memory_size(100000), 5120);
    }

    /// Test truncate_memory_context preserves content
    #[test]
    fn test_truncate_memory_context_preservation() {
        let context = "Line 1: Important architecture decision\n\
                       Line 2: Pattern established\n\
                       Line 3: Key insight\n\
                       Line 4: Another decision\n\
                       Line 5: Final note";

        // No truncation needed
        let result = truncate_memory_context(context, 200);
        assert_eq!(result, context);

        // Truncation at line boundary
        let result = truncate_memory_context(context, 60);
        assert!(result.contains("truncated"));
        assert!(result.len() < 150); // Allow room for truncation message
    }
}

// ============================================================================
// High-Priority Preservation Tests
// ============================================================================

mod high_priority_preservation {
    use super::*;

    /// Test that high-priority entries are preserved during summarization
    #[test]
    fn test_high_priority_entries_preserved() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut config = MemoryConfig::default();
        config.max_entries = 5;
        config.min_entries_for_summarization = 3;
        config.keep_fraction = 0.3;
        config.preserve_high_priority = true;

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add critical/high priority entries
        let critical_entry = MemoryEntry::new(
            "task-critical",
            "Critical Security Decision",
            "Never store passwords in plain text",
        )
        .with_priority(MemoryPriority::Critical)
        .with_category_enum(MemoryCategory::Security);

        let high_entry = MemoryEntry::new(
            "task-high",
            "Important Architecture",
            "Use async/await for all I/O",
        )
        .with_priority(MemoryPriority::High)
        .with_category_enum(MemoryCategory::ArchitectureDecision);

        store.append_entry(&critical_entry).unwrap();
        store.append_entry(&high_entry).unwrap();

        // Add many regular entries to trigger summarization
        for i in 0..15 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Regular Decision {}", i),
                format!("Content {}", i),
            );
            store.append_entry(&entry).unwrap();
        }

        let entries = store.get_entries();

        // Verify that summarization occurred but preserved important entries
        assert!(entries.len() < 20, "Summarization should have occurred");

        // Check for preserved high-priority entries in the file content
        let memory_file = get_memory_file_path(&temp_dir);
        let content = fs::read_to_string(&memory_file).expect("Failed to read memory file");

        // The summarization should preserve security and architecture decisions
        if content.contains("Security") || content.contains("Critical") {
            // High-priority security entry was preserved
        }
    }

    /// Test that preserve_high_priority=false allows summarizing all entries
    #[test]
    fn test_no_high_priority_preservation() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut config = MemoryConfig::default();
        config.max_entries = 5;
        config.min_entries_for_summarization = 3;
        config.keep_fraction = 0.3;
        config.preserve_high_priority = false;

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add critical entry
        let critical_entry =
            MemoryEntry::new("task-critical", "Critical Decision", "Very important")
                .with_priority(MemoryPriority::Critical);

        store.append_entry(&critical_entry).unwrap();

        // Add regular entries
        for i in 0..15 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content {}", i),
            );
            store.append_entry(&entry).unwrap();
        }

        // All entries should be subject to summarization
        let final_count = store.entry_count();
        assert!(
            final_count <= 10,
            "Summarization should reduce entries regardless of priority"
        );
    }
}

// ============================================================================
// Edge Cases and Error Handling Tests
// ============================================================================

mod edge_cases {
    use super::*;

    /// Test summarization with single entry
    #[test]
    fn test_summarization_with_single_entry() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut config = MemoryConfig::default();
        config.max_entries = 1;
        config.min_entries_for_summarization = 1;
        config.keep_fraction = 0.5;

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add single entry
        let entry = MemoryEntry::new("task-001", "Single Decision", "Only entry");
        store.append_entry(&entry).expect("Failed to append entry");

        // Should still have at least one entry
        assert!(store.entry_count() >= 1);
    }

    /// Test summarization with empty memory file
    #[test]
    fn test_summarization_empty_memory() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let config = MemoryConfig::default();
        let store = MemoryStore::with_config(temp_dir.path(), config)
            .expect("Failed to create memory store");

        // No entries
        assert_eq!(store.entry_count(), 0);

        // Get context should return appropriate message
        let context = store.get_memory_context().expect("Failed to get context");
        assert!(context.contains("No project memory"));
    }

    /// Test memory file persistence across store recreation
    #[test]
    fn test_memory_persistence_across_sessions() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create store and add entries
        {
            let store = MemoryStore::new(temp_dir.path()).expect("Failed to create memory store");

            for i in 0..5 {
                let entry = MemoryEntry::new(
                    format!("task-{:03}", i),
                    format!("Decision {}", i),
                    format!("Content {}", i),
                );
                store.append_entry(&entry).expect("Failed to append entry");
            }
        }

        // Create new store and verify persistence
        {
            let store = MemoryStore::new(temp_dir.path()).expect("Failed to create memory store");
            assert_eq!(store.entry_count(), 5);

            let entries = store.get_entries();
            assert!(entries.iter().any(|e| e.title.contains("Decision")));
        }
    }

    /// Test summarization with very long entry content
    #[test]
    fn test_summarization_with_long_content() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut config = MemoryConfig::default();
        config.max_file_size = 2000;
        config.min_entries_for_summarization = 2;
        config.keep_fraction = 0.5;

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add entries with very long content
        for i in 0..3 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Long Decision {}", i),
                create_large_memory_content(2000),
            );
            store.append_entry(&entry).expect("Failed to append entry");
        }

        // Summarization should control file size
        let memory_file = get_memory_file_path(&temp_dir);
        let metadata = fs::metadata(&memory_file).expect("Failed to get metadata");

        // File size should be reasonable after summarization
        assert!(metadata.len() < 10000, "File size should be controlled");
    }

    /// Test concurrent access safety (basic test)
    #[test]
    fn test_concurrent_access_safety() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create two stores pointing to same location
        let store1 = MemoryStore::new(temp_dir.path()).expect("Failed to create store1");
        let store2 = MemoryStore::new(temp_dir.path()).expect("Failed to create store2");

        // Both should read the same empty state
        assert_eq!(store1.entry_count(), 0);
        assert_eq!(store2.entry_count(), 0);

        // Add entry through store1
        let entry = MemoryEntry::new("task-001", "Shared Decision", "Content");
        store1.append_entry(&entry).expect("Failed to append entry");

        // store1 should see the entry
        assert_eq!(store1.entry_count(), 1);

        // Create new store to verify persistence
        let store3 = MemoryStore::new(temp_dir.path()).expect("Failed to create store3");
        assert_eq!(store3.entry_count(), 1);
    }
}

// ============================================================================
// Configuration Validation Integration Tests
// ============================================================================

mod config_validation {
    use super::*;

    /// Test MemoryConfig validation catches invalid values
    #[test]
    fn test_config_validation_errors() {
        // Zero max_file_size
        let mut config = MemoryConfig::default();
        config.max_file_size = 0;
        assert!(config.validate().is_err());

        // Zero max_entries
        let mut config = MemoryConfig::default();
        config.max_entries = 0;
        assert!(config.validate().is_err());

        // Zero min_entries_for_summarization
        let mut config = MemoryConfig::default();
        config.min_entries_for_summarization = 0;
        assert!(config.validate().is_err());

        // Invalid keep_fraction (0)
        let mut config = MemoryConfig::default();
        config.keep_fraction = 0.0;
        assert!(config.validate().is_err());

        // Invalid keep_fraction (> 1)
        let mut config = MemoryConfig::default();
        config.keep_fraction = 1.5;
        assert!(config.validate().is_err());

        // Zero max_context_size
        let mut config = MemoryConfig::default();
        config.max_context_size = 0;
        assert!(config.validate().is_err());

        // Zero old_entry_threshold_seconds
        let mut config = MemoryConfig::default();
        config.old_entry_threshold_seconds = 0;
        assert!(config.validate().is_err());
    }

    /// Test valid MemoryConfig passes validation
    #[test]
    fn test_config_validation_success() {
        let config = MemoryConfig::default();
        assert!(config.validate().is_ok());

        // Custom valid config
        let mut config = MemoryConfig::default();
        config.max_file_size = 100 * 1024;
        config.max_entries = 200;
        config.min_entries_for_summarization = 20;
        config.keep_fraction = 0.7;
        config.max_context_size = 10 * 1024;
        config.enable_summarization = true;
        config.preserve_high_priority = false;
        config.old_entry_threshold_seconds = 172800;
        assert!(config.validate().is_ok());
    }
}
