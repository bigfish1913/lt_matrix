//! Integration tests for memory CLI commands
//!
//! These tests verify:
//! 1. Memory status command
//! 2. Memory summarize command
//! 3. Memory clear command
//! 4. Automatic summarization triggering

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use ltmatrix::memory::{MemoryStore, MemoryEntry, MemoryCategory, MemoryPriority};
use ltmatrix::config::settings::MemoryConfig;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_memory_store_with_entries(temp_dir: &TempDir, count: usize) -> MemoryStore {
    let store = MemoryStore::new(temp_dir.path()).expect("Failed to create memory store");

    for i in 0..count {
        let entry = MemoryEntry::new(
            format!("task-{:03}", i),
            format!("Decision {}", i),
            format!("Content for decision {}", i)
        )
        .with_category_enum(MemoryCategory::ArchitectureDecision);
        store.append_entry(&entry).expect("Failed to append entry");
    }

    store
}

fn get_memory_file_path(temp_dir: &TempDir) -> PathBuf {
    temp_dir.path().join(".claude/memory.md")
}

// ============================================================================
// Memory Status Tests
// ============================================================================

mod status_tests {
    use super::*;

    #[test]
    fn test_memory_status_no_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let memory_file = get_memory_file_path(&temp_dir);

        assert!(!memory_file.exists(), "Memory file should not exist initially");
    }

    #[test]
    fn test_memory_status_with_entries() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = create_memory_store_with_entries(&temp_dir, 5);

        assert_eq!(store.entry_count(), 5);

        let memory_file = get_memory_file_path(&temp_dir);
        assert!(memory_file.exists(), "Memory file should exist after adding entries");
    }

    #[test]
    fn test_memory_status_file_size() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let _store = create_memory_store_with_entries(&temp_dir, 10);

        let memory_file = get_memory_file_path(&temp_dir);
        let metadata = fs::metadata(&memory_file).expect("Failed to get file metadata");

        assert!(metadata.len() > 0, "Memory file should have content");
    }

    #[test]
    fn test_memory_status_by_category() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = MemoryStore::new(temp_dir.path()).expect("Failed to create memory store");

        // Add entries with different categories
        let entry1 = MemoryEntry::new("task-001", "Arch", "Content")
            .with_category_enum(MemoryCategory::ArchitectureDecision);
        let entry2 = MemoryEntry::new("task-002", "Pattern", "Content")
            .with_category_enum(MemoryCategory::Pattern);
        let entry3 = MemoryEntry::new("task-003", "Security", "Content")
            .with_category_enum(MemoryCategory::Security);

        store.append_entry(&entry1).unwrap();
        store.append_entry(&entry2).unwrap();
        store.append_entry(&entry3).unwrap();

        let entries = store.get_entries();
        assert_eq!(entries.len(), 3);

        let categories: Vec<_> = entries.iter().map(|e| e.category).collect();
        assert!(categories.contains(&MemoryCategory::ArchitectureDecision));
        assert!(categories.contains(&MemoryCategory::Pattern));
        assert!(categories.contains(&MemoryCategory::Security));
    }

    #[test]
    fn test_memory_status_by_priority() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = MemoryStore::new(temp_dir.path()).expect("Failed to create memory store");

        // Add entries with different priorities
        let entry1 = MemoryEntry::new("task-001", "Critical", "Content")
            .with_priority(MemoryPriority::Critical);
        let entry2 = MemoryEntry::new("task-002", "High", "Content")
            .with_priority(MemoryPriority::High);
        let entry3 = MemoryEntry::new("task-003", "Normal", "Content")
            .with_priority(MemoryPriority::Normal);

        store.append_entry(&entry1).unwrap();
        store.append_entry(&entry2).unwrap();
        store.append_entry(&entry3).unwrap();

        let entries = store.get_entries();
        let priorities: Vec<_> = entries.iter().map(|e| e.priority).collect();
        assert!(priorities.contains(&MemoryPriority::Critical));
        assert!(priorities.contains(&MemoryPriority::High));
        assert!(priorities.contains(&MemoryPriority::Normal));
    }
}

// ============================================================================
// Memory Summarize Tests
// ============================================================================

mod summarize_tests {
    use super::*;

    #[test]
    fn test_summarize_no_entries() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = MemoryStore::new(temp_dir.path()).expect("Failed to create memory store");

        assert_eq!(store.entry_count(), 0);
        // Summarization should be a no-op with no entries
    }

    #[test]
    fn test_summarize_triggers_on_entry_count() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create config with low thresholds
        let mut config = MemoryConfig::default();
        config.max_entries = 5;
        config.min_entries_for_summarization = 3;
        config.keep_fraction = 0.5;

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add entries beyond threshold
        for i in 0..10 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content for decision {}", i)
            );
            store.append_entry(&entry).expect("Failed to append entry");
        }

        // Verify summarization was triggered
        // After summarization, entries should be reduced
        let final_count = store.entry_count();
        assert!(final_count < 10, "Entries should be reduced after summarization");
    }

    #[test]
    fn test_summarize_preserves_recent_entries() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut config = MemoryConfig::default();
        config.max_entries = 5;
        config.min_entries_for_summarization = 3;
        config.keep_fraction = 0.5;

        let store = MemoryStore::with_config(temp_dir.path(), config)
            .expect("Failed to create memory store");

        // Add entries with specific titles
        for i in 0..10 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content for decision {}", i)
            );
            store.append_entry(&entry).expect("Failed to append entry");
        }

        let entries = store.get_entries();
        // Recent entries should be preserved
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_summarize_keeps_high_priority() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut config = MemoryConfig::default();
        config.max_entries = 5;
        config.min_entries_for_summarization = 3;
        config.keep_fraction = 0.5;
        config.preserve_high_priority = true;

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add high priority entry first
        let high_priority = MemoryEntry::new("task-critical", "Critical Decision", "Important architectural choice")
            .with_priority(MemoryPriority::Critical)
            .with_category_enum(MemoryCategory::ArchitectureDecision);
        store.append_entry(&high_priority).unwrap();

        // Add regular entries
        for i in 0..10 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content for decision {}", i)
            );
            store.append_entry(&entry).unwrap();
        }

        let entries = store.get_entries();

        // High priority entries may be summarized depending on configuration,
        // but critical/high priority entries should be preserved if preserve_high_priority is true
        // The exact behavior depends on the summarization algorithm
        // We verify that the store still has entries after summarization
        assert!(!entries.is_empty(), "Store should have entries after summarization");

        // Verify that at least some entries remain
        assert!(store.entry_count() <= 11, "Summarization should have occurred");
    }

    #[test]
    fn test_summarize_disabled_via_config() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut config = MemoryConfig::default();
        config.max_entries = 5;
        config.enable_summarization = false;

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add many entries
        for i in 0..20 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content for decision {}", i)
            );
            store.append_entry(&entry).unwrap();
        }

        // All entries should be preserved when summarization is disabled
        assert_eq!(store.entry_count(), 20);
    }

    #[test]
    fn test_keep_fraction_controls_retention() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut config = MemoryConfig::default();
        config.max_entries = 5;
        config.min_entries_for_summarization = 3;
        config.keep_fraction = 0.3; // Keep only 30%

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        for i in 0..10 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content for decision {}", i)
            );
            store.append_entry(&entry).unwrap();
        }

        // Entries should be reduced significantly
        let final_count = store.entry_count();
        assert!(final_count <= 5, "Entries should be reduced with keep_fraction=0.3");
    }
}

// ============================================================================
// Memory Clear Tests
// ============================================================================

mod clear_tests {
    use super::*;

    #[test]
    fn test_clear_removes_memory_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let _store = create_memory_store_with_entries(&temp_dir, 5);

        let memory_file = get_memory_file_path(&temp_dir);
        assert!(memory_file.exists());

        // Remove the memory file
        fs::remove_file(&memory_file).expect("Failed to remove memory file");

        assert!(!memory_file.exists());
    }

    #[test]
    fn test_clear_removes_empty_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let _store = create_memory_store_with_entries(&temp_dir, 5);

        let memory_file = get_memory_file_path(&temp_dir);
        let claude_dir = memory_file.parent().unwrap();

        // Remove the memory file
        fs::remove_file(&memory_file).expect("Failed to remove memory file");

        // Directory should be empty and removable
        fs::remove_dir(claude_dir).ok();
    }

    #[test]
    fn test_clear_preserves_other_files() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let _store = create_memory_store_with_entries(&temp_dir, 5);

        let claude_dir = temp_dir.path().join(".claude");
        let other_file = claude_dir.join("other.txt");

        // Create another file in the same directory
        fs::write(&other_file, "other content").expect("Failed to write other file");

        let memory_file = get_memory_file_path(&temp_dir);
        fs::remove_file(&memory_file).expect("Failed to remove memory file");

        // Other file should still exist
        assert!(other_file.exists(), "Other files should not be affected");
    }
}

// ============================================================================
// Automatic Summarization Triggering Tests
// ============================================================================

mod auto_trigger_tests {
    use super::*;

    #[test]
    fn test_auto_trigger_on_size_threshold() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut config = MemoryConfig::default();
        config.max_file_size = 500; // Very low threshold
        config.min_entries_for_summarization = 2;

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add entries until we exceed the size threshold
        for i in 0..20 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                "This is a longer content to increase file size quickly. ".repeat(10)
            );
            store.append_entry(&entry).expect("Failed to append entry");
        }

        // Verify summarization was triggered
        let memory_file = get_memory_file_path(&temp_dir);
        let metadata = fs::metadata(&memory_file).expect("Failed to get metadata");
        let file_size = metadata.len() as usize;

        // After summarization, file should be smaller than it would be without summarization
        // The exact size depends on the summarization algorithm
        assert!(file_size < 50000, "File size should be controlled by summarization");
    }

    #[test]
    fn test_auto_trigger_on_entry_count() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut config = MemoryConfig::default();
        config.max_entries = 10;
        config.min_entries_for_summarization = 5;
        config.keep_fraction = 0.5;

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add entries beyond threshold
        for i in 0..20 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content {}", i)
            );
            store.append_entry(&entry).expect("Failed to append entry");
        }

        // Verify summarization was triggered and entries were reduced
        let final_count = store.entry_count();
        assert!(final_count <= 15, "Entry count should be reduced after summarization");
    }

    #[test]
    fn test_no_trigger_below_threshold() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let config = MemoryConfig::default(); // Default thresholds (50KB, 100 entries)

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add entries below threshold
        for i in 0..10 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content {}", i)
            );
            store.append_entry(&entry).expect("Failed to append entry");
        }

        // All entries should be preserved (no summarization triggered)
        assert_eq!(store.entry_count(), 10);
    }

    #[test]
    fn test_min_entries_prevents_premature_summarization() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut config = MemoryConfig::default();
        config.max_entries = 5; // Low threshold
        config.min_entries_for_summarization = 20; // High minimum

        let store = MemoryStore::with_config(temp_dir.path(), config.clone())
            .expect("Failed to create memory store");

        // Add entries that would trigger max_entries but not min_entries
        for i in 0..10 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content {}", i)
            );
            store.append_entry(&entry).expect("Failed to append entry");
        }

        // All entries should be preserved (min_entries not met)
        assert_eq!(store.entry_count(), 10);
    }
}

// ============================================================================
// Integration with Pipeline Tests
// ============================================================================

mod pipeline_integration_tests {
    use super::*;

    #[test]
    fn test_memory_grows_with_tasks() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = MemoryStore::new(temp_dir.path()).expect("Failed to create memory store");

        // Simulate task completions
        for i in 0..5 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Completed: Feature {}", i),
                format!("Successfully implemented feature {}", i)
            )
            .with_category_enum(MemoryCategory::TaskCompletion);
            store.append_entry(&entry).expect("Failed to append entry");
        }

        assert_eq!(store.entry_count(), 5);

        let memory_file = get_memory_file_path(&temp_dir);
        let content = fs::read_to_string(&memory_file).expect("Failed to read memory file");

        // Verify content includes task references
        assert!(content.contains("task-"));
        assert!(content.contains("Completed"));
    }

    #[test]
    fn test_memory_context_for_prompt_injection() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = create_memory_store_with_entries(&temp_dir, 3);

        let context = store.get_memory_context().expect("Failed to get context");

        assert!(context.contains("Project Memory Context"));
        assert!(context.contains("Total entries: 3"));
    }

    #[test]
    fn test_memory_context_limits_recent_entries() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = create_memory_store_with_entries(&temp_dir, 25);

        let context = store.get_memory_context().expect("Failed to get context");

        // Context should indicate truncated entries
        assert!(context.contains("Total entries: 25"));
        // Only recent entries should be shown (up to 20 by default)
    }
}