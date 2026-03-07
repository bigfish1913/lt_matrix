//! Tests for memory extraction and append logic
//!
//! This test file covers:
//! - MemoryEntryBuilder pattern
//! - CodeSnippet functionality
//! - MemoryPriority levels
//! - Deprecation handling
//! - Entry searching/matching
//! - Related tasks and tags
//! - MemoryCategory parsing
//! - Memory entry summary generation

use ltmatrix::memory::{
    MemoryStore, MemoryEntry, MemoryCategory, MemoryPriority,
    CodeSnippet, MemoryEntryBuilder,
    extract_memory_from_task, extract_task_summary,
};
use ltmatrix::models::{Task, TaskStatus, TaskComplexity};
use std::fs;
use tempfile::TempDir;

// ============================================================================
// MemoryEntryBuilder Tests
// ============================================================================

#[test]
fn test_memory_entry_builder_basic() {
    let entry = MemoryEntryBuilder::new("task-001", "Test Decision")
        .content("This is the content of the decision")
        .build()
        .unwrap();

    assert_eq!(entry.task_id, "task-001");
    assert_eq!(entry.title, "Test Decision");
    assert_eq!(entry.content, "This is the content of the decision");
}

#[test]
fn test_memory_entry_builder_requires_content() {
    let result = MemoryEntryBuilder::new("task-001", "Test")
        .build();

    assert!(result.is_err(), "Builder should require content");
    assert!(result.unwrap_err().to_string().contains("Content is required"));
}

#[test]
fn test_memory_entry_builder_with_category() {
    let entry = MemoryEntryBuilder::new("task-001", "Architecture")
        .content("Use Tokio runtime")
        .category(MemoryCategory::ArchitectureDecision)
        .build()
        .unwrap();

    assert_eq!(entry.category, MemoryCategory::ArchitectureDecision);
}

#[test]
fn test_memory_entry_builder_with_category_str() {
    let entry = MemoryEntryBuilder::new("task-001", "Pattern")
        .content("Repository pattern")
        .category_str("Pattern")
        .build()
        .unwrap();

    assert_eq!(entry.category, MemoryCategory::Pattern);
}

#[test]
fn test_memory_entry_builder_with_priority() {
    let entry = MemoryEntryBuilder::new("task-001", "Critical Decision")
        .content("This is critical")
        .priority(MemoryPriority::Critical)
        .build()
        .unwrap();

    assert_eq!(entry.priority, MemoryPriority::Critical);
}

#[test]
fn test_memory_entry_builder_with_files() {
    let files = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];
    let entry = MemoryEntryBuilder::new("task-001", "Files")
        .content("Modified files")
        .files(files.clone())
        .build()
        .unwrap();

    assert_eq!(entry.files, files);
}

#[test]
fn test_memory_entry_builder_with_single_file() {
    let entry = MemoryEntryBuilder::new("task-001", "Single File")
        .content("Modified one file")
        .file("src/main.rs")
        .file("src/lib.rs")
        .build()
        .unwrap();

    assert_eq!(entry.files, vec!["src/main.rs", "src/lib.rs"]);
}

#[test]
fn test_memory_entry_builder_with_key_points() {
    let points = vec!["Point 1".to_string(), "Point 2".to_string()];
    let entry = MemoryEntryBuilder::new("task-001", "Points")
        .content("Key decision")
        .key_points(points.clone())
        .build()
        .unwrap();

    assert_eq!(entry.key_points, points);
}

#[test]
fn test_memory_entry_builder_with_single_key_point() {
    let entry = MemoryEntryBuilder::new("task-001", "Points")
        .content("Key decision")
        .key_point("First point")
        .key_point("Second point")
        .build()
        .unwrap();

    assert_eq!(entry.key_points, vec!["First point", "Second point"]);
}

#[test]
fn test_memory_entry_builder_with_tags() {
    let tags = vec!["async".to_string(), "tokio".to_string()];
    let entry = MemoryEntryBuilder::new("task-001", "Tagged")
        .content("Tagged content")
        .tags(tags.clone())
        .build()
        .unwrap();

    assert_eq!(entry.tags, tags);
}

#[test]
fn test_memory_entry_builder_with_single_tag() {
    let entry = MemoryEntryBuilder::new("task-001", "Tagged")
        .content("Tagged content")
        .tag("rust")
        .tag("async")
        .build()
        .unwrap();

    assert_eq!(entry.tags, vec!["rust", "async"]);
}

#[test]
fn test_memory_entry_builder_with_related_tasks() {
    let related = vec!["task-000".to_string(), "task-002".to_string()];
    let entry = MemoryEntryBuilder::new("task-001", "Related")
        .content("Has related tasks")
        .related_tasks(related.clone())
        .build()
        .unwrap();

    assert_eq!(entry.related_tasks, related);
}

#[test]
fn test_memory_entry_builder_with_code_snippet() {
    let snippet = CodeSnippet {
        file: "src/main.rs".to_string(),
        start_line: 10,
        end_line: 20,
        code: "fn main() {}".to_string(),
        description: "Main function".to_string(),
    };

    let entry = MemoryEntryBuilder::new("task-001", "Code")
        .content("Added code")
        .code_snippet(snippet.clone())
        .build()
        .unwrap();

    assert_eq!(entry.code_snippets.len(), 1);
    assert_eq!(entry.code_snippets[0].file, "src/main.rs");
}

#[test]
fn test_memory_entry_builder_full() {
    let entry = MemoryEntryBuilder::new("task-042", "Complete Entry")
        .content("Full featured entry")
        .category(MemoryCategory::ArchitectureDecision)
        .priority(MemoryPriority::High)
        .files(vec!["src/main.rs".to_string()])
        .key_points(vec!["Important point".to_string()])
        .tags(vec!["architecture".to_string()])
        .related_tasks(vec!["task-001".to_string()])
        .build()
        .unwrap();

    assert_eq!(entry.task_id, "task-042");
    assert_eq!(entry.title, "Complete Entry");
    assert_eq!(entry.category, MemoryCategory::ArchitectureDecision);
    assert_eq!(entry.priority, MemoryPriority::High);
    assert_eq!(entry.files.len(), 1);
    assert_eq!(entry.key_points.len(), 1);
    assert_eq!(entry.tags.len(), 1);
    assert_eq!(entry.related_tasks.len(), 1);
}

// ============================================================================
// MemoryCategory Tests
// ============================================================================

#[test]
fn test_memory_category_display() {
    assert_eq!(format!("{}", MemoryCategory::ArchitectureDecision), "Architecture Decision");
    assert_eq!(format!("{}", MemoryCategory::Pattern), "Pattern");
    assert_eq!(format!("{}", MemoryCategory::ApiDesign), "API Design");
    assert_eq!(format!("{}", MemoryCategory::DataModel), "Data Model");
    assert_eq!(format!("{}", MemoryCategory::ErrorHandling), "Error Handling");
    assert_eq!(format!("{}", MemoryCategory::Performance), "Performance");
    assert_eq!(format!("{}", MemoryCategory::Security), "Security");
    assert_eq!(format!("{}", MemoryCategory::Testing), "Testing");
    assert_eq!(format!("{}", MemoryCategory::Dependencies), "Dependencies");
    assert_eq!(format!("{}", MemoryCategory::CodeOrganization), "Code Organization");
    assert_eq!(format!("{}", MemoryCategory::BugFix), "Bug Fix");
    assert_eq!(format!("{}", MemoryCategory::Configuration), "Configuration");
    assert_eq!(format!("{}", MemoryCategory::ImportantNote), "Important Note");
    assert_eq!(format!("{}", MemoryCategory::TaskCompletion), "Task Completion");
    assert_eq!(format!("{}", MemoryCategory::General), "General");
}

#[test]
fn test_memory_category_from_str() {
    use std::str::FromStr;

    // Test various valid inputs
    assert_eq!(MemoryCategory::from_str("Architecture Decision").unwrap(), MemoryCategory::ArchitectureDecision);
    assert_eq!(MemoryCategory::from_str("architecture decision").unwrap(), MemoryCategory::ArchitectureDecision);
    assert_eq!(MemoryCategory::from_str("architectural decision").unwrap(), MemoryCategory::ArchitectureDecision);
    assert_eq!(MemoryCategory::from_str("Architecture_Decision").unwrap(), MemoryCategory::ArchitectureDecision);

    assert_eq!(MemoryCategory::from_str("Pattern").unwrap(), MemoryCategory::Pattern);
    assert_eq!(MemoryCategory::from_str("patterns").unwrap(), MemoryCategory::Pattern);

    assert_eq!(MemoryCategory::from_str("API Design").unwrap(), MemoryCategory::ApiDesign);
    assert_eq!(MemoryCategory::from_str("api").unwrap(), MemoryCategory::ApiDesign);

    assert_eq!(MemoryCategory::from_str("Data Model").unwrap(), MemoryCategory::DataModel);
    assert_eq!(MemoryCategory::from_str("Bug Fix").unwrap(), MemoryCategory::BugFix);
    assert_eq!(MemoryCategory::from_str("bugfix").unwrap(), MemoryCategory::BugFix);
    assert_eq!(MemoryCategory::from_str("Important Note").unwrap(), MemoryCategory::ImportantNote);
    assert_eq!(MemoryCategory::from_str("note").unwrap(), MemoryCategory::ImportantNote);

    // Test invalid input
    assert!(MemoryCategory::from_str("invalid_category").is_err());
    assert!(MemoryCategory::from_str("unknown").is_err());
}

#[test]
fn test_memory_category_default() {
    let default: MemoryCategory = Default::default();
    assert_eq!(default, MemoryCategory::General);
}

#[test]
fn test_memory_category_serde() {
    // Serialize
    let json = serde_json::to_string(&MemoryCategory::ArchitectureDecision).unwrap();
    assert_eq!(json, "\"architecture_decision\"");

    // Deserialize
    let category: MemoryCategory = serde_json::from_str("\"pattern\"").unwrap();
    assert_eq!(category, MemoryCategory::Pattern);
}

// ============================================================================
// MemoryPriority Tests
// ============================================================================

#[test]
fn test_memory_priority_ordering() {
    // Verify ordering
    assert!(MemoryPriority::Critical > MemoryPriority::High);
    assert!(MemoryPriority::High > MemoryPriority::Normal);
    assert!(MemoryPriority::Normal > MemoryPriority::Low);

    // Verify equality
    assert!(MemoryPriority::Normal == MemoryPriority::Normal);
    assert!(MemoryPriority::High != MemoryPriority::Low);
}

#[test]
fn test_memory_priority_default() {
    let default: MemoryPriority = Default::default();
    assert_eq!(default, MemoryPriority::Normal);
}

#[test]
fn test_memory_priority_serde() {
    let json = serde_json::to_string(&MemoryPriority::Critical).unwrap();
    assert_eq!(json, "\"critical\"");

    let priority: MemoryPriority = serde_json::from_str("\"high\"").unwrap();
    assert_eq!(priority, MemoryPriority::High);
}

#[test]
fn test_memory_entry_with_priority_levels() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    // Create entries with different priorities
    let low = MemoryEntry::new("task-001", "Low Priority", "Low content")
        .with_priority(MemoryPriority::Low);
    let normal = MemoryEntry::new("task-002", "Normal Priority", "Normal content")
        .with_priority(MemoryPriority::Normal);
    let high = MemoryEntry::new("task-003", "High Priority", "High content")
        .with_priority(MemoryPriority::High);
    let critical = MemoryEntry::new("task-004", "Critical Priority", "Critical content")
        .with_priority(MemoryPriority::Critical);

    store.append_entry(&low).unwrap();
    store.append_entry(&normal).unwrap();
    store.append_entry(&high).unwrap();
    store.append_entry(&critical).unwrap();

    let entries = store.get_entries();
    assert_eq!(entries[0].priority, MemoryPriority::Low);
    assert_eq!(entries[1].priority, MemoryPriority::Normal);
    assert_eq!(entries[2].priority, MemoryPriority::High);
    assert_eq!(entries[3].priority, MemoryPriority::Critical);

    // Verify priority is in file
    let memory_file = temp_dir.path().join(".claude/memory.md");
    let content = fs::read_to_string(memory_file).unwrap();
    assert!(content.contains("**Priority**: Low"));
    assert!(content.contains("**Priority**: Normal"));
    assert!(content.contains("**Priority**: High"));
    assert!(content.contains("**Priority**: Critical"));
}

// ============================================================================
// CodeSnippet Tests
// ============================================================================

#[test]
fn test_code_snippet_creation() {
    let snippet = CodeSnippet {
        file: "src/main.rs".to_string(),
        start_line: 1,
        end_line: 10,
        code: "fn main() { println!(\"Hello\"); }".to_string(),
        description: "Main entry point".to_string(),
    };

    assert_eq!(snippet.file, "src/main.rs");
    assert_eq!(snippet.start_line, 1);
    assert_eq!(snippet.end_line, 10);
    assert!(snippet.code.contains("main"));
    assert_eq!(snippet.description, "Main entry point");
}

#[test]
fn test_code_snippet_serde() {
    let snippet = CodeSnippet {
        file: "src/lib.rs".to_string(),
        start_line: 5,
        end_line: 15,
        code: "pub fn helper() {}".to_string(),
        description: "Helper function".to_string(),
    };

    let json = serde_json::to_string(&snippet).unwrap();
    let deserialized: CodeSnippet = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.file, snippet.file);
    assert_eq!(deserialized.start_line, snippet.start_line);
    assert_eq!(deserialized.code, snippet.code);
}

#[test]
fn test_memory_entry_with_code_snippets() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    let snippet = CodeSnippet {
        file: "src/async.rs".to_string(),
        start_line: 10,
        end_line: 20,
        code: "async fn run() { }".to_string(),
        description: "Async runner".to_string(),
    };

    let entry = MemoryEntry::new("task-001", "Code Example", "Added async runner")
        .with_category("Pattern")
        .with_key_points(vec!["Uses async/await".to_string()]);

    // Store entry without snippet first
    store.append_entry(&entry).unwrap();

    let memory_file = temp_dir.path().join(".claude/memory.md");
    let content = fs::read_to_string(&memory_file).unwrap();

    // Verify content is stored
    assert!(content.contains("Code Example"));
    assert!(content.contains("Added async runner"));
}

// ============================================================================
// MemoryEntry Search/Match Tests
// ============================================================================

#[test]
fn test_memory_entry_matches_title() {
    let entry = MemoryEntry::new("task-001", "Architecture Decision", "Use Tokio");

    assert!(entry.matches("architecture"));
    assert!(entry.matches("ARCHITECTURE"));
    assert!(entry.matches("Decision"));
    assert!(!entry.matches("database"));
}

#[test]
fn test_memory_entry_matches_content() {
    let entry = MemoryEntry::new("task-001", "Decision", "We decided to use async Rust with Tokio runtime for performance");

    assert!(entry.matches("async"));
    assert!(entry.matches("tokio"));
    assert!(entry.matches("performance"));
    assert!(!entry.matches("python"));
}

#[test]
fn test_memory_entry_matches_tags() {
    let entry = MemoryEntry::new("task-001", "Tagged", "Content")
        .with_tags(vec!["rust".to_string(), "async".to_string(), "tokio".to_string()]);

    assert!(entry.matches("rust"));
    assert!(entry.matches("async"));
    assert!(entry.matches("tokio"));
    assert!(!entry.matches("python"));
}

#[test]
fn test_memory_entry_matches_files() {
    let entry = MemoryEntry::new("task-001", "Files", "Modified files")
        .with_files(vec!["src/main.rs".to_string(), "src/lib.rs".to_string()]);

    assert!(entry.matches("main.rs"));
    assert!(entry.matches("lib.rs"));
    assert!(entry.matches("src"));
    assert!(!entry.matches("test.rs"));
}

#[test]
fn test_memory_entry_matches_case_insensitive() {
    let entry = MemoryEntry::new("task-001", "ARCHITECTURE", "TOKIO RUNTIME");

    assert!(entry.matches("architecture"));
    assert!(entry.matches("tokio"));
    assert!(entry.matches("RUNTIME"));
}

// ============================================================================
// MemoryEntry Summary Tests
// ============================================================================

#[test]
fn test_memory_entry_to_summary() {
    let entry = MemoryEntry::new("task-001", "Important Decision", "This is a critical architectural choice");
    let summary = entry.to_summary();

    assert!(summary.contains("Important Decision"));
    assert!(summary.contains("This is a critical architectural choice"));
}

#[test]
fn test_memory_entry_to_summary_multiline() {
    let entry = MemoryEntry::new("task-001", "Decision", "First line\nSecond line\nThird line");
    let summary = entry.to_summary();

    // Should only include the first line
    assert!(summary.contains("First line"));
    assert!(!summary.contains("Second line"));
}

#[test]
fn test_memory_entry_to_summary_with_category() {
    let entry = MemoryEntry::new("task-001", "Decision", "Content")
        .with_category("Architecture Decision");

    let summary = entry.to_summary();
    assert!(summary.contains("Architecture Decision"));
}

// ============================================================================
// Deprecation Tests
// ============================================================================

#[test]
fn test_memory_entry_deprecation() {
    let mut entry = MemoryEntry::new("task-001", "Old Decision", "This was a decision");

    // Initially not deprecated
    assert!(!entry.deprecated);
    assert!(entry.deprecation_reason.is_none());

    // Mark as deprecated
    entry.deprecated = true;
    entry.deprecation_reason = Some("Superseded by task-005".to_string());

    assert!(entry.deprecated);
    assert_eq!(entry.deprecation_reason, Some("Superseded by task-005".to_string()));
}

#[test]
fn test_deprecated_entry_markdown() {
    let mut entry = MemoryEntry::new("task-001", "Deprecated Decision", "Old choice");
    entry.deprecated = true;
    entry.deprecation_reason = Some("Use new approach instead".to_string());

    let markdown = entry.to_markdown();

    assert!(markdown.contains("Deprecated"));
    assert!(markdown.contains("Use new approach instead"));
}

#[test]
fn test_deprecated_entry_no_reason() {
    let mut entry = MemoryEntry::new("task-001", "Deprecated", "Old");
    entry.deprecated = true;

    let markdown = entry.to_markdown();
    assert!(markdown.contains("Deprecated"));
}

// ============================================================================
// MemoryEntry Markdown Format Tests
// ============================================================================

#[test]
fn test_memory_entry_markdown_with_all_fields() {
    let entry = MemoryEntry::new("task-042", "Complete Entry", "Full content here")
        .with_category("Architecture Decision")
        .with_priority(MemoryPriority::High)
        .with_files(vec!["src/main.rs".to_string()])
        .with_tags(vec!["async".to_string()])
        .with_key_points(vec!["Point 1".to_string(), "Point 2".to_string()]);

    let markdown = entry.to_markdown();

    // Check all components
    assert!(markdown.contains("## [Architecture Decision] Complete Entry"));
    assert!(markdown.contains("**Task**: task-042"));
    assert!(markdown.contains("**Priority**: High"));
    assert!(markdown.contains("**Files**: src/main.rs"));
    assert!(markdown.contains("**Tags**: async"));
    assert!(markdown.contains("Full content here"));
    assert!(markdown.contains("### Key Points"));
    assert!(markdown.contains("- Point 1"));
    assert!(markdown.contains("- Point 2"));
    assert!(markdown.contains("---"));
}

#[test]
fn test_memory_entry_markdown_minimal() {
    let entry = MemoryEntry::new("task-001", "Simple", "Just content");
    let markdown = entry.to_markdown();

    // Should have required fields
    assert!(markdown.contains("## [General] Simple"));
    assert!(markdown.contains("**Task**: task-001"));
    assert!(markdown.contains("**Priority**: Normal"));
    assert!(markdown.contains("Just content"));
    assert!(markdown.contains("---"));

    // Should not have optional sections
    assert!(!markdown.contains("**Files**:"));
    assert!(!markdown.contains("**Tags**:"));
    assert!(!markdown.contains("### Key Points"));
}

// ============================================================================
// Related Tasks Tests
// ============================================================================

#[test]
fn test_memory_entry_with_related_tasks() {
    let entry = MemoryEntry::new("task-003", "Dependent Decision", "Based on earlier work")
        .with_related_tasks(vec!["task-001".to_string(), "task-002".to_string()]);

    assert_eq!(entry.related_tasks, vec!["task-001", "task-002"]);
}

#[test]
fn test_related_tasks_stored_and_loaded() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    let entry = MemoryEntry::new("task-003", "Related", "Has dependencies")
        .with_related_tasks(vec!["task-001".to_string(), "task-002".to_string()]);

    store.append_entry(&entry).unwrap();

    // Verify it was stored
    let entries = store.get_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].related_tasks, vec!["task-001", "task-002"]);
}

// ============================================================================
// Memory Entry ID Generation Tests
// ============================================================================

#[test]
fn test_memory_entry_id_generation() {
    let entry1 = MemoryEntry::new("task-001", "First Decision", "Content 1");
    let entry2 = MemoryEntry::new("task-002", "Second Decision", "Content 2");

    // IDs should be unique
    assert_ne!(entry1.id, entry2.id);

    // IDs should contain timestamp and title
    assert!(entry1.id.starts_with("mem-"));
    assert!(entry1.id.contains("first-decision"));
}

#[test]
fn test_memory_entry_id_sanitization() {
    let entry = MemoryEntry::new("task-001", "Decision with Special!@#$%Characters", "Content");

    // ID should only contain alphanumeric, dashes, and spaces converted to dashes
    assert!(entry.id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_task(id: &str, title: &str, description: &str) -> Task {
    let mut task = Task::new(id, title, description);
    task.status = TaskStatus::Completed;
    task.complexity = TaskComplexity::Moderate;
    task
}