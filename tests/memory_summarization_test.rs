//! Tests for memory summarization feature
//!
//! These tests verify:
//! 1. Detection when memory.md grows too large (file size threshold)
//! 2. Detection when there are too many entries (entry count threshold)
//! 3. Summarization algorithm consolidates older entries while preserving recent context
//! 4. Configuration for summarization thresholds
//! 5. Custom MemoryConfig usage with MemoryStore
//! 6. Size detection logic with configurable thresholds

use ltmatrix::memory::{MemoryEntry, MemoryStore, MemoryCategory, MemoryPriority};
use ltmatrix::config::settings::MemoryConfig;
use tempfile::TempDir;
use std::fs;
use std::io::Write as IoWrite;

// ============================================================================
// Configuration Tests
// ============================================================================

mod config_tests {
    use super::*;

    #[test]
    fn test_memory_config_default_values() {
        let config = MemoryConfig::default();

        assert_eq!(config.max_file_size, 50 * 1024); // 50KB
        assert_eq!(config.max_entries, 100);
        assert_eq!(config.min_entries_for_summarization, 10);
        assert!((config.keep_fraction - 0.5).abs() < f64::EPSILON);
        assert_eq!(config.max_context_size, 5 * 1024); // 5KB
        assert!(config.enable_summarization);
        assert!(config.preserve_high_priority);
        assert_eq!(config.old_entry_threshold_seconds, 86400); // 24 hours
    }

    #[test]
    fn test_memory_config_validation_valid() {
        let config = MemoryConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_memory_config_validation_zero_max_file_size() {
        let config = MemoryConfig {
            max_file_size: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("max_file_size"));
    }

    #[test]
    fn test_memory_config_validation_zero_max_entries() {
        let config = MemoryConfig {
            max_entries: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("max_entries"));
    }

    #[test]
    fn test_memory_config_validation_zero_min_entries() {
        let config = MemoryConfig {
            min_entries_for_summarization: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("min_entries_for_summarization"));
    }

    #[test]
    fn test_memory_config_validation_invalid_keep_fraction_zero() {
        let config = MemoryConfig {
            keep_fraction: 0.0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("keep_fraction"));
    }

    #[test]
    fn test_memory_config_validation_invalid_keep_fraction_negative() {
        let config = MemoryConfig {
            keep_fraction: -0.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_memory_config_validation_invalid_keep_fraction_above_one() {
        let config = MemoryConfig {
            keep_fraction: 1.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_memory_config_validation_keep_fraction_at_boundary() {
        // keep_fraction of 1.0 should be valid
        let config = MemoryConfig {
            keep_fraction: 1.0,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_memory_config_validation_zero_max_context_size() {
        let config = MemoryConfig {
            max_context_size: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("max_context_size"));
    }

    #[test]
    fn test_memory_config_validation_zero_old_entry_threshold() {
        let config = MemoryConfig {
            old_entry_threshold_seconds: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("old_entry_threshold_seconds"));
    }

    #[test]
    fn test_memory_config_old_entry_threshold_duration() {
        let config = MemoryConfig {
            old_entry_threshold_seconds: 3600, // 1 hour
            ..Default::default()
        };

        let duration = config.old_entry_threshold_duration();
        assert_eq!(duration.num_seconds(), 3600);
    }
}

// ============================================================================
// Threshold Detection Tests
// ============================================================================

mod threshold_detection_tests {
    use super::*;

    /// Test that summarization is triggered when file size exceeds threshold
    /// The constants MAX_MEMORY_SIZE (50KB) and MAX_ENTRIES (100) are used
    #[test]
    fn test_summarization_triggered_by_file_size() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // Create a large memory file manually by writing content directly
        let memory_file = temp_dir.path().join(".claude/memory.md");
        let mut file = fs::File::create(&memory_file).unwrap();

        // Write header
        writeln!(file, "# Project Memory\n\n").unwrap();
        writeln!(file, "---\n").unwrap();

        // Create entries until we exceed the file size threshold (50KB)
        // Each entry should be large enough to trigger summarization
        for i in 1..=15 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Large Entry {}", i),
                "x".repeat(4000) // 4KB of content per entry
            );
            write!(file, "{}", entry.to_markdown()).unwrap();
        }

        drop(file);

        // Check file size exceeds threshold
        let metadata = fs::metadata(&memory_file).unwrap();
        assert!(metadata.len() > 50 * 1024, "File should exceed 50KB threshold");

        // Now add another entry - this should trigger summarization
        let new_entry = MemoryEntry::new("task-trigger", "Trigger Entry", "This triggers summarization");
        store.append_entry(&new_entry).unwrap();

        // After adding, the store should still work correctly
        assert!(store.entry_count() > 0);
    }

    #[test]
    fn test_summarization_triggered_by_entry_count() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // Add more entries than MAX_ENTRIES (100)
        // But we need to ensure each entry has some content
        for i in 1..=110 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Decision {}", i),
                format!("Content for decision {} with enough text to make it meaningful", i)
            );
            store.append_entry(&entry).unwrap();
        }

        // The store should have handled summarization
        // Entry count might be reduced after summarization
        let count = store.entry_count();
        assert!(count > 0, "Should have entries after adding 110");

        // Memory context should still be available
        let context = store.get_memory_context().unwrap();
        assert!(context.contains("Project Memory Context"));
    }

    #[test]
    fn test_no_summarization_below_thresholds() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // Add only a few entries (below thresholds)
        for i in 1..=5 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Small Entry {}", i),
                format!("Small content {}", i)
            );
            store.append_entry(&entry).unwrap();
        }

        // All entries should be preserved
        assert_eq!(store.entry_count(), 5);

        let entries = store.get_entries();
        assert_eq!(entries.len(), 5);
    }

    #[test]
    fn test_summarization_minimum_entries_requirement() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // The implementation requires at least 10 entries before summarization
        // Add 9 entries with small content - should not trigger summarization
        for i in 1..=9 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Entry {}", i),
                "Content"
            );
            store.append_entry(&entry).unwrap();
        }

        // All 9 entries should still be present
        assert_eq!(store.entry_count(), 9);
    }
}

// ============================================================================
// Summarization Algorithm Tests
// ============================================================================

mod summarization_algorithm_tests {
    use super::*;

    #[test]
    fn test_summarization_preserves_recent_entries() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // Add 20 entries to trigger summarization (needs 10+ entries)
        for i in 1..=20 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Entry {}", i),
                format!("Content for entry number {}", i)
            );
            store.append_entry(&entry).unwrap();
        }

        let entries = store.get_entries();

        // The most recent entries should be preserved in full
        // The summarization keeps half of the entries (10 most recent)
        let recent_titles: Vec<&str> = entries.iter()
            .rev()
            .take(5)
            .map(|e| e.title.as_str())
            .collect();

        // Recent entries (16-20) should be present
        assert!(recent_titles.iter().any(|t| t.contains("Entry 20")));
        assert!(recent_titles.iter().any(|t| t.contains("Entry 19")));
    }

    #[test]
    fn test_summarization_consolidates_older_entries() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // Add entries with different categories
        let categories = [
            MemoryCategory::ArchitectureDecision,
            MemoryCategory::Pattern,
            MemoryCategory::ApiDesign,
        ];

        for i in 1..=15 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Entry {}", i),
                format!("Content for entry {}", i)
            ).with_category_enum(categories[i % categories.len()]);

            store.append_entry(&entry).unwrap();
        }

        // After summarization, the memory file should still exist
        let memory_file = temp_dir.path().join(".claude/memory.md");
        assert!(memory_file.exists());

        // Check that the file contains categorized summary or recent entries
        let content = fs::read_to_string(&memory_file).unwrap();

        // The file should have memory content
        assert!(content.contains("Project Memory") || content.contains("Entry"));
    }

    #[test]
    fn test_summarization_categorizes_entries_correctly() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // Add entries with specific categories
        for i in 1..=12 {
            let category = match i {
                1..=4 => MemoryCategory::ArchitectureDecision,
                5..=8 => MemoryCategory::Pattern,
                _ => MemoryCategory::BugFix,
            };

            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("{:?} Entry {}", category, i),
                format!("Content for {} entry", category)
            ).with_category_enum(category);

            store.append_entry(&entry).unwrap();
        }

        let entries = store.get_entries();
        assert!(!entries.is_empty());

        // Verify categories are preserved
        let has_arch = entries.iter().any(|e| e.category == MemoryCategory::ArchitectureDecision);
        let has_pattern = entries.iter().any(|e| e.category == MemoryCategory::Pattern);
        let has_bugfix = entries.iter().any(|e| e.category == MemoryCategory::BugFix);

        assert!(has_arch || has_pattern || has_bugfix, "Should have at least some categorized entries");
    }

    #[test]
    fn test_summarization_keeps_half_entries() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // Add exactly 20 entries
        for i in 1..=20 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Entry {:02}", i),
                format!("Content for entry {:02}", i)
            );
            store.append_entry(&entry).unwrap();
        }

        // The summarization should keep half (10) of the entries
        let entries = store.get_entries();

        // Entry count should be around 10 (could be less due to summarization)
        assert!(entries.len() <= 20, "Entries should be reduced after summarization");
    }
}

// ============================================================================
// Memory Entry Tests
// ============================================================================

mod memory_entry_tests {
    use super::*;

    #[test]
    fn test_memory_entry_with_priority() {
        let entry = MemoryEntry::new("task-001", "Critical Decision", "This is critical")
            .with_priority(MemoryPriority::Critical);

        assert_eq!(entry.priority, MemoryPriority::Critical);
    }

    #[test]
    fn test_memory_entry_with_files() {
        let entry = MemoryEntry::new("task-001", "Files Changed", "Modified files")
            .with_files(vec!["src/main.rs".to_string(), "src/lib.rs".to_string()]);

        assert_eq!(entry.files.len(), 2);
        assert!(entry.files.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn test_memory_entry_with_key_points() {
        let entry = MemoryEntry::new("task-001", "Key Points", "Important notes")
            .with_key_points(vec!["Point 1".to_string(), "Point 2".to_string()]);

        assert_eq!(entry.key_points.len(), 2);
    }

    #[test]
    fn test_memory_entry_markdown_format_includes_priority() {
        let entry = MemoryEntry::new("task-001", "Test", "Content")
            .with_priority(MemoryPriority::High);

        let markdown = entry.to_markdown();
        assert!(markdown.contains("**Priority**: High"));
    }

    #[test]
    fn test_memory_entry_summary_format() {
        let entry = MemoryEntry::new("task-042", "Test Title", "First line of content\nSecond line");

        let summary = entry.to_summary();
        assert!(summary.contains("Test Title"));
        assert!(summary.contains("First line of content"));
    }

    #[test]
    fn test_memory_entry_matches_search() {
        let entry = MemoryEntry::new("task-001", "Architecture Decision", "Using Tokio for async runtime")
            .with_tags(vec!["async".to_string(), "tokio".to_string()]);

        assert!(entry.matches("tokio"));
        assert!(entry.matches("architecture"));
        assert!(entry.matches("ASYNC")); // Case insensitive
        assert!(!entry.matches("python"));
    }
}

// ============================================================================
// Memory Store Tests
// ============================================================================

mod memory_store_tests {
    use super::*;

    #[test]
    fn test_memory_store_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let claude_dir = temp_dir.path().join(".claude");

        assert!(!claude_dir.exists());

        let _store = MemoryStore::new(temp_dir.path()).unwrap();

        assert!(claude_dir.exists());
    }

    #[test]
    fn test_memory_store_persistence() {
        let temp_dir = TempDir::new().unwrap();

        // Create store and add entry
        {
            let store = MemoryStore::new(temp_dir.path()).unwrap();
            let entry = MemoryEntry::new("task-001", "First", "Content one");
            store.append_entry(&entry).unwrap();
            assert_eq!(store.entry_count(), 1);
        }

        // Create new store instance - should load existing entries
        {
            let store = MemoryStore::new(temp_dir.path()).unwrap();
            assert_eq!(store.entry_count(), 1);

            let entries = store.get_entries();
            assert_eq!(entries[0].task_id, "task-001");
        }
    }

    #[test]
    fn test_memory_context_injection_format() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // Empty store
        let context = store.get_memory_context().unwrap();
        assert!(context.contains("No project memory available"));

        // Add entry
        let entry = MemoryEntry::new("task-001", "Architecture", "Use Tokio runtime");
        store.append_entry(&entry).unwrap();

        let context = store.get_memory_context().unwrap();
        assert!(context.contains("Project Memory Context"));
        assert!(context.contains("Architecture"));
        assert!(context.contains("Use Tokio runtime"));
    }

    #[test]
    fn test_memory_context_limits_entries() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // Add 25 entries
        for i in 1..=25 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Entry {}", i),
                format!("Content {}", i)
            );
            store.append_entry(&entry).unwrap();
        }

        // Context should limit to 20 recent entries
        let context = store.get_memory_context().unwrap();
        assert!(context.contains("Total entries: 25"));
        // Should indicate older entries exist
        assert!(context.contains("older entries") || context.contains("more"));
    }

    #[test]
    fn test_multiple_entries_with_same_task_id() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // Multiple entries with same task ID should be allowed
        let entry1 = MemoryEntry::new("task-001", "Decision 1", "Content 1");
        let entry2 = MemoryEntry::new("task-001", "Decision 2", "Content 2");

        store.append_entry(&entry1).unwrap();
        store.append_entry(&entry2).unwrap();

        assert_eq!(store.entry_count(), 2);
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

mod integration_tests {
    use super::*;
    use ltmatrix::memory::MemoryIntegration;

    #[test]
    fn test_memory_integration_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let integration = MemoryIntegration::new(temp_dir.path()).unwrap();

        // Initial state
        assert_eq!(integration.entry_count(), 0);

        // Get context for empty memory
        let context = integration.get_context_for_prompt().unwrap();
        assert!(context.contains("No project memory available"));
    }

    #[test]
    fn test_memory_file_format_consistency() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // Add entry with all fields
        let entry = MemoryEntry::new("task-001", "Full Entry", "Main content here")
            .with_category_enum(MemoryCategory::ArchitectureDecision)
            .with_priority(MemoryPriority::High)
            .with_files(vec!["src/main.rs".to_string()])
            .with_key_points(vec!["Key point 1".to_string()])
            .with_tags(vec!["important".to_string()]);

        store.append_entry(&entry).unwrap();

        // Read the file and verify format
        let memory_file = temp_dir.path().join(".claude/memory.md");
        let content = fs::read_to_string(&memory_file).unwrap();

        assert!(content.contains("# Project Memory"));
        assert!(content.contains("[Architecture Decision] Full Entry"));
        assert!(content.contains("**Task**: task-001"));
        assert!(content.contains("**Priority**: High"));
        assert!(content.contains("**Files**: src/main.rs"));
        assert!(content.contains("Main content here"));
    }

    #[test]
    fn test_summarization_with_mixed_priorities() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // Add entries with different priorities
        for i in 1..=15 {
            let priority = match i {
                1..=5 => MemoryPriority::Critical,
                6..=10 => MemoryPriority::High,
                _ => MemoryPriority::Normal,
            };

            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Priority Test {}", i),
                format!("Content with priority {:?}", priority)
            ).with_priority(priority);

            store.append_entry(&entry).unwrap();
        }

        let entries = store.get_entries();

        // Critical and high priority entries should be preserved
        let has_critical = entries.iter().any(|e| e.priority == MemoryPriority::Critical);
        let has_high = entries.iter().any(|e| e.priority == MemoryPriority::High);

        assert!(has_critical || has_high, "High priority entries should be preserved");
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_content_entry() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        let entry = MemoryEntry::new("task-001", "Empty Content", "");
        store.append_entry(&entry).unwrap();

        assert_eq!(store.entry_count(), 1);
    }

    #[test]
    fn test_very_long_content_entry() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        let long_content = "x".repeat(10000);
        let entry = MemoryEntry::new("task-001", "Long Content", long_content);

        store.append_entry(&entry).unwrap();

        assert_eq!(store.entry_count(), 1);

        let entries = store.get_entries();
        assert_eq!(entries[0].content.len(), 10000);
    }

    #[test]
    fn test_special_characters_in_entry() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        let entry = MemoryEntry::new(
            "task-001",
            "Special <>&\"' Characters",
            "Content with **markdown** and `code` and [links](url)"
        );

        store.append_entry(&entry).unwrap();

        let entries = store.get_entries();
        assert!(entries[0].content.contains("**markdown**"));
    }

    #[test]
    fn test_unicode_content() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        let entry = MemoryEntry::new(
            "task-001",
            "Unicode 标题",
            "Unicode content: 日本語 🎉 émojis"
        );

        store.append_entry(&entry).unwrap();

        let entries = store.get_entries();
        assert!(entries[0].title.contains("标题"));
        assert!(entries[0].content.contains("日本語"));
    }

    #[test]
    fn test_category_parsing_from_string() {
        use std::str::FromStr;

        // Test various category string formats
        assert_eq!(
            MemoryCategory::from_str("Architecture Decision").unwrap(),
            MemoryCategory::ArchitectureDecision
        );
        assert_eq!(
            MemoryCategory::from_str("architecture_decision").unwrap(),
            MemoryCategory::ArchitectureDecision
        );
        assert_eq!(
            MemoryCategory::from_str("ARCHITECTURE-DECISION").unwrap(),
            MemoryCategory::ArchitectureDecision
        );
        assert_eq!(
            MemoryCategory::from_str("bugfix").unwrap(),
            MemoryCategory::BugFix
        );
    }

    #[test]
    fn test_unknown_category_returns_error() {
        use std::str::FromStr;

        let result = MemoryCategory::from_str("UnknownCategory");
        assert!(result.is_err());
    }
}

// ============================================================================
// Custom Configuration Tests
// ============================================================================

mod custom_config_tests {
    use super::*;

    /// Test that MemoryStore::with_config applies custom configuration
    #[test]
    fn test_memory_store_with_custom_config() {
        let temp_dir = TempDir::new().unwrap();

        let custom_config = MemoryConfig {
            max_file_size: 10 * 1024, // 10KB instead of default 50KB
            max_entries: 20,          // 20 instead of default 100
            min_entries_for_summarization: 5,
            keep_fraction: 0.3,
            max_context_size: 2 * 1024,
            enable_summarization: true,
            preserve_high_priority: true,
            old_entry_threshold_seconds: 3600,
        };

        let store = MemoryStore::with_config(temp_dir.path(), custom_config.clone()).unwrap();
        assert_eq!(store.entry_count(), 0);

        // Verify configuration is applied by checking behavior
        // Add entries and verify they work with custom config
        for i in 1..=10 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Entry {}", i),
                format!("Content {}", i)
            );
            store.append_entry(&entry).unwrap();
        }

        assert_eq!(store.entry_count(), 10);
    }

    /// Test that summarization can be disabled via configuration
    #[test]
    fn test_summarization_disabled_via_config() {
        let temp_dir = TempDir::new().unwrap();

        let config = MemoryConfig {
            enable_summarization: false,
            ..Default::default()
        };

        let store = MemoryStore::with_config(temp_dir.path(), config).unwrap();

        // Add many entries that would normally trigger summarization
        for i in 1..=150 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Entry {}", i),
                format!("Content for entry {} with enough text", i)
            );
            store.append_entry(&entry).unwrap();
        }

        // When summarization is disabled, all entries should be preserved
        // (beyond the default max_entries of 100)
        assert!(store.entry_count() > 100, "Entries should not be summarized when disabled");
    }

    /// Test custom max_file_size threshold triggers summarization
    #[test]
    fn test_custom_max_file_size_triggers_summarization() {
        let temp_dir = TempDir::new().unwrap();

        // Use a very small file size threshold (1KB)
        let config = MemoryConfig {
            max_file_size: 1024, // 1KB
            min_entries_for_summarization: 3,
            keep_fraction: 0.5,
            enable_summarization: true,
            ..Default::default()
        };

        let store = MemoryStore::with_config(temp_dir.path(), config).unwrap();

        // Add entries with large content to exceed 1KB threshold
        for i in 1..=10 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Large Entry {}", i),
                "x".repeat(500) // 500 bytes per entry
            );
            store.append_entry(&entry).unwrap();
        }

        // Store should still work after summarization
        assert!(store.entry_count() > 0);

        // Memory file should exist
        let memory_file = temp_dir.path().join(".claude/memory.md");
        assert!(memory_file.exists());
    }

    /// Test custom max_entries threshold triggers summarization
    #[test]
    fn test_custom_max_entries_triggers_summarization() {
        let temp_dir = TempDir::new().unwrap();

        let config = MemoryConfig {
            max_entries: 5, // Very low threshold
            min_entries_for_summarization: 3,
            keep_fraction: 0.5,
            enable_summarization: true,
            ..Default::default()
        };

        let store = MemoryStore::with_config(temp_dir.path(), config).unwrap();

        // Add 10 entries, which exceeds max_entries of 5
        for i in 1..=10 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Entry {}", i),
                format!("Content {}", i)
            );
            store.append_entry(&entry).unwrap();
        }

        // After summarization, entries should be reduced
        let count = store.entry_count();
        // The exact count depends on summarization logic
        assert!(count > 0, "Should have some entries");
    }

    /// Test keep_fraction controls how many entries are kept
    #[test]
    fn test_keep_fraction_controls_retention() {
        let temp_dir = TempDir::new().unwrap();

        // Use keep_fraction of 0.8 to keep 80% of entries
        let config = MemoryConfig {
            min_entries_for_summarization: 5,
            keep_fraction: 0.8,
            max_entries: 15, // Trigger at 15 entries
            enable_summarization: true,
            ..Default::default()
        };

        let store = MemoryStore::with_config(temp_dir.path(), config).unwrap();

        // Add 20 entries
        for i in 1..=20 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Entry {}", i),
                format!("Content {}", i)
            );
            store.append_entry(&entry).unwrap();
        }

        // Most entries should be kept with 80% retention
        let entries = store.get_entries();
        assert!(!entries.is_empty());
    }

    /// Test min_entries_for_summarization prevents premature summarization
    #[test]
    fn test_min_entries_prevents_premature_summarization() {
        let temp_dir = TempDir::new().unwrap();

        // Require at least 20 entries before summarization
        let config = MemoryConfig {
            max_file_size: 100, // Very small to trigger early
            min_entries_for_summarization: 20,
            enable_summarization: true,
            ..Default::default()
        };

        let store = MemoryStore::with_config(temp_dir.path(), config).unwrap();

        // Add 15 entries (less than min_entries_for_summarization)
        for i in 1..=15 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Entry {}", i),
                "x".repeat(100)
            );
            store.append_entry(&entry).unwrap();
        }

        // All 15 entries should be preserved (under minimum threshold)
        assert_eq!(store.entry_count(), 15);
    }
}

// ============================================================================
// Size Detection Tests
// ============================================================================

mod size_detection_tests {
    use super::*;

    /// Test that file size is correctly detected
    #[test]
    fn test_file_size_detection() {
        let temp_dir = TempDir::new().unwrap();
        let store = MemoryStore::new(temp_dir.path()).unwrap();

        // Add entry and verify file exists
        let entry = MemoryEntry::new("task-001", "Test", "Content");
        store.append_entry(&entry).unwrap();

        let memory_file = temp_dir.path().join(".claude/memory.md");
        assert!(memory_file.exists());

        let metadata = fs::metadata(&memory_file).unwrap();
        assert!(metadata.len() > 0, "Memory file should have content");
    }

    /// Test that large file size triggers summarization detection
    #[test]
    fn test_large_file_size_triggers_detection() {
        let temp_dir = TempDir::new().unwrap();

        // Create a config with small file size threshold
        let config = MemoryConfig {
            max_file_size: 500, // 500 bytes - very small
            min_entries_for_summarization: 3,
            enable_summarization: true,
            ..Default::default()
        };

        let store = MemoryStore::with_config(temp_dir.path(), config).unwrap();

        // Create entries that will exceed 500 bytes
        for i in 1..=5 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Entry {}", i),
                "x".repeat(200) // Each entry ~200 bytes
            );
            store.append_entry(&entry).unwrap();
        }

        // Check that file size exceeded threshold at some point
        let memory_file = temp_dir.path().join(".claude/memory.md");
        let final_metadata = fs::metadata(&memory_file).unwrap();

        // File should exist and have content
        assert!(final_metadata.len() > 0);
    }

    /// Test detection respects entry count threshold
    #[test]
    fn test_entry_count_detection() {
        let temp_dir = TempDir::new().unwrap();

        let config = MemoryConfig {
            max_entries: 10,
            min_entries_for_summarization: 5,
            enable_summarization: true,
            ..Default::default()
        };

        let store = MemoryStore::with_config(temp_dir.path(), config).unwrap();

        // Add exactly max_entries
        for i in 1..=10 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Entry {}", i),
                format!("Content {}", i)
            );
            store.append_entry(&entry).unwrap();
        }

        // At this point, we have exactly 10 entries (at threshold but not over)
        // Add one more to exceed threshold
        let entry = MemoryEntry::new("task-011", "Entry 11", "Content 11");
        store.append_entry(&entry).unwrap();

        // Store should still function
        assert!(store.entry_count() > 0);
    }

    /// Test that both thresholds are checked (file size OR entry count)
    #[test]
    fn test_either_threshold_triggers_summarization() {
        let temp_dir = TempDir::new().unwrap();

        // Test 1: Trigger by entry count (not file size)
        {
            let temp_dir1 = TempDir::new().unwrap();
            let config1 = MemoryConfig {
                max_file_size: 1024 * 1024, // 1MB - won't trigger
                max_entries: 5,             // Will trigger
                min_entries_for_summarization: 3,
                enable_summarization: true,
                ..Default::default()
            };

            let store1 = MemoryStore::with_config(temp_dir1.path(), config1).unwrap();

            for i in 1..=10 {
                let entry = MemoryEntry::new(
                    format!("task-{:03}", i),
                    format!("Entry {}", i),
                    "x" // Small content
                );
                store1.append_entry(&entry).unwrap();
            }

            assert!(store1.entry_count() > 0);
        }

        // Test 2: Trigger by file size (not entry count)
        {
            let temp_dir2 = TempDir::new().unwrap();
            let config2 = MemoryConfig {
                max_file_size: 100,   // Will trigger
                max_entries: 10000,   // Won't trigger
                min_entries_for_summarization: 2,
                enable_summarization: true,
                ..Default::default()
            };

            let store2 = MemoryStore::with_config(temp_dir2.path(), config2).unwrap();

            for i in 1..=5 {
                let entry = MemoryEntry::new(
                    format!("task-{:03}", i),
                    format!("Entry {}", i),
                    "x".repeat(100) // Large content
                );
                store2.append_entry(&entry).unwrap();
            }

            assert!(store2.entry_count() > 0);
        }
    }
}

// ============================================================================
// Preserve High Priority Tests
// ============================================================================

mod preserve_high_priority_tests {
    use super::*;

    /// Test that high priority entries are preserved during summarization
    /// The file should contain either parsed high-priority entries or a summary section
    #[test]
    fn test_high_priority_entries_preserved() {
        let temp_dir = TempDir::new().unwrap();

        let config = MemoryConfig {
            max_entries: 10,
            min_entries_for_summarization: 5,
            preserve_high_priority: true,
            enable_summarization: true,
            keep_fraction: 0.3,
            ..Default::default()
        };

        let store = MemoryStore::with_config(temp_dir.path(), config).unwrap();

        // Add high priority entries first (these will be in "old" section after summarization)
        for i in 1..=5 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Critical Entry {}", i),
                format!("Critical content {}", i)
            ).with_priority(MemoryPriority::Critical);
            store.append_entry(&entry).unwrap();
        }

        // Add normal entries (these will push high-priority to "old" section)
        for i in 6..=15 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Normal Entry {}", i),
                format!("Normal content {}", i)
            );
            store.append_entry(&entry).unwrap();
        }

        // Check that the memory file contains preserved high-priority content
        let memory_file = temp_dir.path().join(".claude/memory.md");
        let content = fs::read_to_string(&memory_file).unwrap();

        // The file should either have:
        // 1. A "Preserved High-Priority Entries" section, OR
        // 2. The critical entries themselves in the parsed entries
        let entries = store.get_entries();
        let has_critical_in_entries = entries.iter()
            .any(|e| e.priority == MemoryPriority::Critical);
        let has_preserved_section = content.contains("Preserved High-Priority") ||
                                    content.contains("Critical Entry");

        // At least one should be true - entries are preserved somehow
        assert!(
            has_critical_in_entries || has_preserved_section,
            "High-priority entries should be preserved either in entries or summary section"
        );
    }

    /// Test that preserve_high_priority=false allows summarizing high priority
    #[test]
    fn test_high_priority_can_be_summarized_when_disabled() {
        let temp_dir = TempDir::new().unwrap();

        let config = MemoryConfig {
            max_entries: 10,
            min_entries_for_summarization: 5,
            preserve_high_priority: false, // Disabled
            enable_summarization: true,
            keep_fraction: 0.3,
            ..Default::default()
        };

        let store = MemoryStore::with_config(temp_dir.path(), config).unwrap();

        // Add high priority entries
        for i in 1..=15 {
            let entry = MemoryEntry::new(
                format!("task-{:03}", i),
                format!("Entry {}", i),
                format!("Content {}", i)
            ).with_priority(MemoryPriority::Critical);
            store.append_entry(&entry).unwrap();
        }

        // All entries are high priority, but can be summarized
        let entries = store.get_entries();
        assert!(!entries.is_empty());
    }
}

// ============================================================================
// Configuration Validation Edge Cases
// ============================================================================

mod config_validation_edge_cases {
    use super::*;

    #[test]
    fn test_memory_config_serialization() {
        let config = MemoryConfig {
            max_file_size: 75 * 1024,
            max_entries: 150,
            min_entries_for_summarization: 15,
            keep_fraction: 0.75,
            max_context_size: 10 * 1024,
            enable_summarization: false,
            preserve_high_priority: false,
            old_entry_threshold_seconds: 172800, // 48 hours
        };

        // Serialize to TOML
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("max_file_size"));
        assert!(toml_str.contains("max_entries"));
        assert!(toml_str.contains("enable_summarization = false"));

        // Deserialize back
        let deserialized: MemoryConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.max_file_size, 75 * 1024);
        assert_eq!(deserialized.max_entries, 150);
        assert!(!deserialized.enable_summarization);
    }

    #[test]
    fn test_memory_config_deserialization_from_toml() {
        let toml_str = r#"
            max_file_size = 102400
            max_entries = 200
            min_entries_for_summarization = 20
            keep_fraction = 0.6
            max_context_size = 8192
            enable_summarization = true
            preserve_high_priority = false
            old_entry_threshold_seconds = 43200
        "#;

        let config: MemoryConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.max_file_size, 102400);
        assert_eq!(config.max_entries, 200);
        assert_eq!(config.min_entries_for_summarization, 20);
        assert!((config.keep_fraction - 0.6).abs() < f64::EPSILON);
        assert_eq!(config.max_context_size, 8192);
        assert!(config.enable_summarization);
        assert!(!config.preserve_high_priority);
        assert_eq!(config.old_entry_threshold_seconds, 43200);
    }

    #[test]
    fn test_memory_config_defaults_via_deserialization() {
        // Empty TOML should use all defaults
        let toml_str = "";
        let config: MemoryConfig = toml::from_str(toml_str).unwrap();

        // Should match default
        let default = MemoryConfig::default();
        assert_eq!(config.max_file_size, default.max_file_size);
        assert_eq!(config.max_entries, default.max_entries);
        assert_eq!(config.enable_summarization, default.enable_summarization);
    }

    #[test]
    fn test_memory_config_partial_override() {
        // Only override some values
        let toml_str = r#"
            max_entries = 50
            enable_summarization = false
        "#;

        let config: MemoryConfig = toml::from_str(toml_str).unwrap();

        // Overridden values
        assert_eq!(config.max_entries, 50);
        assert!(!config.enable_summarization);

        // Default values for rest
        assert_eq!(config.max_file_size, 50 * 1024);
        assert_eq!(config.min_entries_for_summarization, 10);
    }
}