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
//! - Memory extraction from task results
//! - Timestamp and task reference handling
//! - Memory entry format validation

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

// ============================================================================
// Memory Extraction Integration Tests
// ============================================================================

#[test]
fn test_extract_memory_from_task_architecture_decision() {
    let task = create_test_task("task-001", "Setup Project", "Initialize Rust project");
    let result = "Architecture decision: Using Tokio runtime for async operations";

    let entries = extract_memory_from_task(&task, result).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].category, MemoryCategory::ArchitectureDecision);
    assert!(entries[0].content.contains("Tokio"));
    assert!(entries[0].tags.contains(&"architecture".to_string()));
}

#[test]
fn test_extract_memory_from_task_multiple_types() {
    let task = create_test_task("task-002", "Implement Feature", "Add user authentication");
    let result = r#"
    Architecture decision: Using JWT for authentication
    Pattern: Repository pattern for user data
    Important: Store refresh tokens securely
    API design: RESTful endpoints for auth flow
    Performance: Using bcrypt with cost factor 12
    Security: Passwords hashed before storage
    Error handling: Graceful fallback for auth failures
    "#;

    let entries = extract_memory_from_task(&task, result).unwrap();

    // Should extract multiple entries of different types
    assert!(entries.len() >= 5, "Expected at least 5 entries, got {}", entries.len());

    let categories: Vec<_> = entries.iter().map(|e| e.category).collect();
    assert!(categories.contains(&MemoryCategory::ArchitectureDecision));
    assert!(categories.contains(&MemoryCategory::Pattern));
    assert!(categories.contains(&MemoryCategory::ImportantNote));
}

#[test]
fn test_extract_memory_from_task_pattern() {
    let task = create_test_task("task-003", "Refactor Code", "Apply design patterns");
    let result = "Pattern: Using the Builder pattern for configuration objects";

    let entries = extract_memory_from_task(&task, result).unwrap();

    assert!(!entries.is_empty());
    assert_eq!(entries[0].category, MemoryCategory::Pattern);
    assert!(entries[0].content.contains("Builder"));
}

#[test]
fn test_extract_memory_from_task_important_note() {
    let task = create_test_task("task-004", "Fix Bug", "Resolve memory leak");
    let result = "Important: Always call .close() on connections to prevent leaks";

    let entries = extract_memory_from_task(&task, result).unwrap();

    assert!(!entries.is_empty());
    assert_eq!(entries[0].category, MemoryCategory::ImportantNote);
    assert!(entries[0].content.contains("close()"));
}

#[test]
fn test_extract_memory_from_task_performance() {
    let task = create_test_task("task-005", "Optimize", "Improve query performance");
    let result = "Performance: Added database connection pooling for 10x throughput improvement";

    let entries = extract_memory_from_task(&task, result).unwrap();

    assert!(!entries.is_empty());
    assert_eq!(entries[0].category, MemoryCategory::Performance);
}

#[test]
fn test_extract_memory_from_task_security() {
    let task = create_test_task("task-006", "Security Audit", "Fix vulnerabilities");
    let result = "Security: Implemented input sanitization to prevent SQL injection attacks";

    let entries = extract_memory_from_task(&task, result).unwrap();

    assert!(!entries.is_empty());
    assert_eq!(entries[0].category, MemoryCategory::Security);
}

#[test]
fn test_extract_memory_from_task_api_design() {
    let task = create_test_task("task-007", "API Design", "Create REST endpoints");
    let result = "API design: RESTful endpoints with versioned routes /api/v1/";

    let entries = extract_memory_from_task(&task, result).unwrap();

    assert!(!entries.is_empty());
    assert_eq!(entries[0].category, MemoryCategory::ApiDesign);
}

#[test]
fn test_extract_memory_from_task_error_handling() {
    let task = create_test_task("task-008", "Error Handling", "Improve error messages");
    let result = "Error handling: Using anyhow for error propagation with context";

    let entries = extract_memory_from_task(&task, result).unwrap();

    assert!(!entries.is_empty());
    assert_eq!(entries[0].category, MemoryCategory::ErrorHandling);
}

#[test]
fn test_extract_memory_from_task_empty_result() {
    let task = create_test_task("task-009", "Minor Fix", "Fixed typo");
    let result = "Fixed a typo in the README";

    let entries = extract_memory_from_task(&task, result).unwrap();

    // Should return empty when no patterns match
    assert!(entries.is_empty());
}

#[test]
fn test_extract_memory_from_task_case_insensitive() {
    let task = create_test_task("task-010", "Test Case", "Test case sensitivity");
    let result = "ARCHITECTURE DECISION: Using PostgreSQL for persistence";

    let entries = extract_memory_from_task(&task, result).unwrap();

    assert!(!entries.is_empty());
    assert_eq!(entries[0].category, MemoryCategory::ArchitectureDecision);
}

// ============================================================================
// Task Summary Extraction Tests
// ============================================================================

#[test]
fn test_extract_task_summary_basic() {
    let task = create_test_task("task-001", "Implement Feature", "Add user authentication");
    let files = vec!["src/auth.rs".to_string(), "src/user.rs".to_string()];

    let entry = extract_task_summary(&task, &files).unwrap();

    assert_eq!(entry.task_id, "task-001");
    assert!(entry.title.contains("Implement Feature"));
    assert!(entry.content.contains("Add user authentication"));
    assert!(entry.content.contains("src/auth.rs"));
    assert!(entry.content.contains("src/user.rs"));
    assert_eq!(entry.category, MemoryCategory::TaskCompletion);
    assert_eq!(entry.files, files);
}

#[test]
fn test_extract_task_summary_no_files() {
    let task = create_test_task("task-002", "Documentation", "Update README");

    let entry = extract_task_summary(&task, &[]).unwrap();

    assert_eq!(entry.task_id, "task-002");
    assert!(entry.content.contains("Update README"));
    assert!(entry.files.is_empty());
}

#[test]
fn test_extract_task_summary_includes_task_metadata() {
    let mut task = Task::new("task-042", "Complex Task", "Multi-step implementation");
    task.status = TaskStatus::Completed;
    task.complexity = TaskComplexity::Complex;

    let entry = extract_task_summary(&task, &["src/main.rs".to_string()]).unwrap();

    assert_eq!(entry.task_id, "task-042");
    assert!(entry.title.contains("Completed"));
}

// ============================================================================
// Memory Entry Append with Timestamp Tests
// ============================================================================

#[test]
fn test_append_entry_includes_timestamp() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    let entry = MemoryEntry::new("task-001", "Decision", "Use async Rust");

    // Store timestamp before append
    let before = chrono::Utc::now();

    store.append_entry(&entry).unwrap();

    // Verify entry has timestamp
    let entries = store.get_entries();
    assert_eq!(entries.len(), 1);

    // Timestamp should be recent (within 1 second)
    let diff = (entries[0].timestamp - before).num_seconds().abs();
    assert!(diff < 2, "Timestamp should be within 2 seconds of append time");

    // Verify timestamp is in file
    let memory_file = temp_dir.path().join(".claude/memory.md");
    let content = fs::read_to_string(memory_file).unwrap();
    assert!(content.contains("**Date**:"));
}

#[test]
fn test_append_entry_includes_task_reference() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    let entry = MemoryEntry::new("task-042", "Architecture", "Use PostgreSQL");

    store.append_entry(&entry).unwrap();

    // Verify task reference in entry
    let entries = store.get_entries();
    assert_eq!(entries[0].task_id, "task-042");

    // Verify task reference is in file
    let memory_file = temp_dir.path().join(".claude/memory.md");
    let content = fs::read_to_string(memory_file).unwrap();
    assert!(content.contains("**Task**: task-042"));
}

#[test]
fn test_append_entry_creates_memory_file() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    let entry = MemoryEntry::new("task-001", "First Entry", "Initial decision");
    store.append_entry(&entry).unwrap();

    let memory_file = temp_dir.path().join(".claude/memory.md");
    assert!(memory_file.exists());

    let content = fs::read_to_string(memory_file).unwrap();
    assert!(content.starts_with("# Project Memory"));
    assert!(content.contains("First Entry"));
}

#[test]
fn test_append_multiple_entries_with_timestamps() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    // Append multiple entries
    for i in 1..=5 {
        let entry = MemoryEntry::new(
            format!("task-{:03}", i),
            format!("Decision {}", i),
            format!("Content for decision {}", i)
        );
        store.append_entry(&entry).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let entries = store.get_entries();
    assert_eq!(entries.len(), 5);

    // Verify timestamps are in order
    for i in 0..4 {
        assert!(entries[i].timestamp <= entries[i + 1].timestamp);
    }
}

// ============================================================================
// Memory Entry Format Tests
// ============================================================================

#[test]
fn test_memory_entry_format_markdown_structure() {
    let entry = MemoryEntry::new("task-001", "Architecture Decision", "Use Tokio runtime")
        .with_category_enum(MemoryCategory::ArchitectureDecision)
        .with_priority(MemoryPriority::High)
        .with_files(vec!["src/main.rs".to_string()])
        .with_key_points(vec!["All I/O is async".to_string()])
        .with_tags(vec!["async".to_string(), "tokio".to_string()]);

    let markdown = entry.to_markdown();

    // Check structure
    assert!(markdown.starts_with("## [Architecture Decision] Architecture Decision"));
    assert!(markdown.contains("**Task**: task-001"));
    assert!(markdown.contains("**Date**:"));
    assert!(markdown.contains("**Priority**: High"));
    assert!(markdown.contains("**Files**: src/main.rs"));
    assert!(markdown.contains("**Tags**: async, tokio"));
    assert!(markdown.contains("Use Tokio runtime"));
    assert!(markdown.contains("### Key Points"));
    assert!(markdown.contains("- All I/O is async"));
    assert!(markdown.ends_with("---\n"));
}

#[test]
fn test_memory_entry_format_minimal() {
    let entry = MemoryEntry::new("task-001", "Simple Note", "Just a note");

    let markdown = entry.to_markdown();

    // Should have required fields
    assert!(markdown.contains("## [General] Simple Note"));
    assert!(markdown.contains("**Task**: task-001"));
    assert!(markdown.contains("**Date**:"));
    assert!(markdown.contains("**Priority**: Normal"));
    assert!(markdown.contains("Just a note"));

    // Should not have optional fields
    assert!(!markdown.contains("**Files**:"));
    assert!(!markdown.contains("**Tags**:"));
    assert!(!markdown.contains("### Key Points"));
}

#[test]
fn test_memory_entry_format_deprecated() {
    let mut entry = MemoryEntry::new("task-001", "Old Approach", "Use sync I/O");
    entry.deprecated = true;
    entry.deprecation_reason = Some("Superseded by async approach in task-005".to_string());

    let markdown = entry.to_markdown();

    assert!(markdown.contains("Deprecated"));
    assert!(markdown.contains("Superseded by async approach"));
}

// ============================================================================
// Memory Store Integration Tests
// ============================================================================

#[test]
fn test_memory_store_persists_entries() {
    let temp_dir = TempDir::new().unwrap();

    // Create store and add entry
    {
        let store = MemoryStore::new(temp_dir.path()).unwrap();
        let entry = MemoryEntry::new("task-001", "Decision", "Use async Rust");
        store.append_entry(&entry).unwrap();
    }

    // Create new store and verify persistence
    {
        let store = MemoryStore::new(temp_dir.path()).unwrap();
        assert_eq!(store.entry_count(), 1);

        let entries = store.get_entries();
        assert_eq!(entries[0].task_id, "task-001");
        assert_eq!(entries[0].title, "Decision");
    }
}

#[test]
fn test_memory_store_get_memory_context() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    // Empty context
    let context = store.get_memory_context().unwrap();
    assert!(context.contains("No project memory available"));

    // Add entry
    let entry = MemoryEntry::new("task-001", "Architecture", "Use Tokio")
        .with_category_enum(MemoryCategory::ArchitectureDecision);
    store.append_entry(&entry).unwrap();

    // Get context
    let context = store.get_memory_context().unwrap();
    assert!(context.contains("Project Memory Context"));
    assert!(context.contains("Architecture"));
    assert!(context.contains("Use Tokio"));
    assert!(context.contains("Total entries: 1"));
}

#[test]
fn test_memory_store_append_extracted_entries() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    // Create task and extract memories
    let task = create_test_task("task-001", "Feature Implementation", "Add authentication");
    let result = r#"
    Architecture decision: Using JWT for authentication
    Pattern: Repository pattern for data access
    Important: Store tokens securely
    "#;

    let entries = extract_memory_from_task(&task, result).unwrap();

    // Append all extracted entries
    for entry in &entries {
        store.append_entry(entry).unwrap();
    }

    assert_eq!(store.entry_count(), entries.len());

    // Verify file contains all entries
    let memory_file = temp_dir.path().join(".claude/memory.md");
    let content = fs::read_to_string(memory_file).unwrap();

    assert!(content.contains("JWT"));
    assert!(content.contains("Repository pattern"));
    assert!(content.contains("Store tokens securely"));
}

#[test]
fn test_extraction_and_append_end_to_end() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path()).unwrap();

    // Simulate task completion
    let task = create_test_task("task-042", "Database Setup", "Configure PostgreSQL");
    let task_result = r#"
    Architecture decision: Using PostgreSQL with connection pooling
    Pattern: Repository pattern for data access
    API design: Exposed /api/users endpoint
    Performance: Added connection pool with max 10 connections
    Security: Encrypted connection strings in config
    Important: Connection strings must be in environment variables
    "#;

    // Extract memories from task result
    let extracted = extract_memory_from_task(&task, task_result).unwrap();

    // Also create task summary
    let files = vec!["src/db.rs".to_string(), "src/config.rs".to_string()];
    let summary = extract_task_summary(&task, &files).unwrap();

    // Append all entries
    for entry in &extracted {
        store.append_entry(entry).unwrap();
    }
    store.append_entry(&summary).unwrap();

    // Verify all entries were stored
    let entries = store.get_entries();
    assert!(entries.len() >= 6, "Expected at least 6 entries, got {}", entries.len());

    // Verify memory file format
    let memory_file = temp_dir.path().join(".claude/memory.md");
    let content = fs::read_to_string(memory_file).unwrap();

    // Check for timestamps and task references
    assert!(content.contains("**Date**:"));
    assert!(content.contains("**Task**: task-042"));

    // Check for various categories
    assert!(content.contains("[Architecture Decision]"));
    assert!(content.contains("[Pattern]"));
    assert!(content.contains("[API Design]"));
    assert!(content.contains("[Performance]"));
    assert!(content.contains("[Security]"));
    assert!(content.contains("[Important Note]"));
    assert!(content.contains("[Task Completion]"));
}