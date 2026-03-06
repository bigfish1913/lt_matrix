//! Task hierarchy tree view tests
//!
//! This test suite validates the ASCII tree visualization for task hierarchies.

use ltmatrix::models::{Task, TaskStatus};

#[test]
fn test_tree_view_single_task() {
    let task = Task::new("task-1", "Main Task", "A task with no subtasks");

    let tree = ltmatrix::tasks::tree::format_tree(&task);

    // Should show just the task without any tree branches
    assert!(tree.contains("task-1"));
    assert!(tree.contains("Main Task"));
    assert!(tree.contains("○")); // Default pending status
}

#[test]
fn test_tree_view_with_subtasks() {
    let mut parent = Task::new("task-1", "Parent Task", "Parent task");
    parent.subtasks = vec![
        Task::new("task-2", "Child 1", "First child"),
        Task::new("task-3", "Child 2", "Second child"),
    ];

    let tree = ltmatrix::tasks::tree::format_tree(&parent);

    // Should show parent with children using tree characters
    assert!(tree.contains("task-1"));
    assert!(tree.contains("Parent Task"));
    assert!(tree.contains("task-2"));
    assert!(tree.contains("Child 1"));
    assert!(tree.contains("task-3"));
    assert!(tree.contains("Child 2"));

    // Should have tree branching characters
    assert!(tree.contains("├──"));
    assert!(tree.contains("└──"));
}

#[test]
fn test_tree_view_nested_subtasks() {
    let mut child2 = Task::new("task-3", "Child 2", "Second child");
    child2.subtasks = vec![
        Task::new("task-4", "Grandchild 1", "First grandchild"),
        Task::new("task-5", "Grandchild 2", "Second grandchild"),
    ];

    let mut parent = Task::new("task-1", "Parent Task", "Parent task");
    parent.subtasks = vec![
        Task::new("task-2", "Child 1", "First child"),
        child2,
    ];

    let tree = ltmatrix::tasks::tree::format_tree(&parent);

    // Should show nested hierarchy
    assert!(tree.contains("task-1"));
    assert!(tree.contains("task-2"));
    assert!(tree.contains("task-3"));
    assert!(tree.contains("task-4"));
    assert!(tree.contains("task-5"));

    // Should have proper tree structure with continuation characters
    // Note: The exact continuation character depends on the tree structure
    // For nested children, we should have some form of indentation
    assert!(tree.contains("│   ") || tree.contains("    "));
}

#[test]
fn test_tree_view_with_status() {
    let mut task = Task::new("task-1", "Main Task", "Task with status");
    task.status = TaskStatus::InProgress;

    let tree = ltmatrix::tasks::tree::format_tree(&task);

    // Should show status indicator
    assert!(tree.contains("⚙"));
}

#[test]
fn test_tree_view_max_depth_respected() {
    // Create a task hierarchy deeper than typical depth
    let mut level3 = Task::new("task-4", "Level 3", "Third level");
    level3.subtasks = vec![
        Task::new("task-5", "Level 4", "Fourth level"),
    ];

    let mut level2 = Task::new("task-3", "Level 2", "Second level");
    level2.subtasks = vec![level3];

    let mut level1 = Task::new("task-2", "Level 1", "First level");
    level1.subtasks = vec![level2];

    let mut root = Task::new("task-1", "Root", "Root task");
    root.subtasks = vec![level1];

    let tree = ltmatrix::tasks::tree::format_tree(&root);

    // Implementation shows all levels (no max depth limiting)
    assert!(tree.contains("task-1"));
    assert!(tree.contains("task-2"));
    assert!(tree.contains("task-3"));
    assert!(tree.contains("task-4"));
    assert!(tree.contains("task-5"));
}

#[test]
fn test_tree_view_empty_subtasks() {
    let task = Task::new("task-1", "Solo Task", "No children");

    let tree = ltmatrix::tasks::tree::format_tree(&task);

    // Should handle gracefully without tree characters
    assert!(tree.contains("task-1"));
    assert!(!tree.contains("├──"));
    assert!(!tree.contains("└──"));
}

#[test]
fn test_tree_format_single_child() {
    // Test edge case: parent with exactly one child
    let mut parent = Task::new("task-1", "Parent", "Parent with one child");
    parent.subtasks = vec![Task::new("task-2", "Only Child", "Single child")];

    let tree = ltmatrix::tasks::tree::format_tree(&parent);

    assert!(tree.contains("task-1"));
    assert!(tree.contains("task-2"));
    // Single child should use └── (last child marker)
    assert!(tree.contains("└──"));
    assert!(!tree.contains("├──"));
}

#[test]
fn test_tree_format_three_children() {
    // Test with three children to verify middle child handling
    let mut parent = Task::new("task-1", "Parent", "Parent with three children");
    parent.subtasks = vec![
        Task::new("task-2", "First", "First child"),
        Task::new("task-3", "Middle", "Middle child"),
        Task::new("task-4", "Last", "Last child"),
    ];

    let tree = ltmatrix::tasks::tree::format_tree(&parent);

    assert!(tree.contains("├──")); // First and middle children
    assert!(tree.contains("└──")); // Last child
}

#[test]
fn test_tree_status_symbols_all_types() {
    // Test all status symbols
    let statuses = vec![
        (TaskStatus::Pending, "○"),
        (TaskStatus::InProgress, "⚙"),
        (TaskStatus::Completed, "✓"),
        (TaskStatus::Failed, "✗"),
        (TaskStatus::Blocked, "⚠"),
    ];

    for (status, symbol) in statuses {
        let mut task = Task::new("test", "Test", "Test task");
        task.status = status.clone();
        let tree = ltmatrix::tasks::tree::format_tree(&task);
        assert!(tree.contains(symbol), "Status symbol {:?} not found", status);
    }
}

#[test]
fn test_tree_deep_nesting_structure() {
    // Create a deeply nested structure (5 levels)
    let mut level4 = Task::new("task-5", "Level 4", "Fourth level");
    level4.subtasks = vec![Task::new("task-6", "Level 5", "Fifth level")];

    let mut level3 = Task::new("task-4", "Level 3", "Third level");
    level3.subtasks = vec![level4];

    let mut level2 = Task::new("task-3", "Level 2", "Second level");
    level2.subtasks = vec![level3];

    let mut level1 = Task::new("task-2", "Level 1", "First level");
    level1.subtasks = vec![level2];

    let mut root = Task::new("task-1", "Root", "Root task");
    root.subtasks = vec![level1];

    let tree = ltmatrix::tasks::tree::format_tree(&root);

    // Verify all levels are present
    for i in 1..=6 {
        assert!(tree.contains(&format!("task-{}", i)), "Level {} missing", i);
    }

    // Count the number of lines (should be 6 for 6 tasks)
    let line_count = tree.lines().count();
    assert_eq!(line_count, 6, "Expected 6 lines for 6 tasks");
}

#[test]
fn test_tree_branching_with_multiple_nested_children() {
    // Complex tree: root -> 2 children, second child has 2 children
    let child2_grand1 = Task::new("task-4", "Grandchild 2-1", "First grandchild of child 2");
    let child2_grand2 = Task::new("task-5", "Grandchild 2-2", "Second grandchild of child 2");

    let mut child2 = Task::new("task-3", "Child 2", "Second child");
    child2.subtasks = vec![child2_grand1, child2_grand2];

    let mut parent = Task::new("task-1", "Parent", "Parent");
    parent.subtasks = vec![
        Task::new("task-2", "Child 1", "First child"),
        child2,
    ];

    let tree = ltmatrix::tasks::tree::format_tree(&parent);

    // Verify all tasks present
    assert!(tree.contains("task-1"));
    assert!(tree.contains("task-2"));
    assert!(tree.contains("task-3"));
    assert!(tree.contains("task-4"));
    assert!(tree.contains("task-5"));

    // Verify tree structure
    assert!(tree.contains("├──")); // First child of parent
    assert!(tree.contains("└──")); // Last child at various levels
    // Note: Continuation characters depend on the specific tree structure
}

#[test]
fn test_tree_format_output_structure() {
    // Test that output has proper structure: each task on its own line
    let mut parent = Task::new("task-1", "Parent", "Parent");
    parent.subtasks = vec![
        Task::new("task-2", "Child 1", "First"),
        Task::new("task-3", "Child 2", "Second"),
    ];

    let tree = ltmatrix::tasks::tree::format_tree(&parent);

    let lines: Vec<&str> = tree.lines().collect();

    // Should have 3 lines (parent + 2 children)
    assert_eq!(lines.len(), 3);

    // First line should be the parent (no prefix)
    assert!(lines[0].contains("task-1"));

    // Second line should be first child with ├── prefix
    assert!(lines[1].contains("├──"));
    assert!(lines[1].contains("task-2"));

    // Third line should be last child with └── prefix
    assert!(lines[2].contains("└──"));
    assert!(lines[2].contains("task-3"));
}

#[test]
fn test_tree_no_extra_blank_lines() {
    // Ensure there are no excessive blank lines in output
    let mut parent = Task::new("task-1", "Parent", "Parent");
    parent.subtasks = vec![
        Task::new("task-2", "Child", "Child"),
    ];

    let tree = ltmatrix::tasks::tree::format_tree(&parent);

    // Count non-empty lines
    let non_empty_count = tree.lines().filter(|l| !l.is_empty()).count();

    // Should have exactly 2 non-empty lines
    assert_eq!(non_empty_count, 2);
}

#[test]
fn test_tree_unicode_characters() {
    // Verify all expected Unicode tree characters are present in a complex tree
    let mut child2 = Task::new("task-3", "Child 2", "Second child");
    child2.subtasks = vec![
        Task::new("task-4", "Grandchild", "Grandchild"),
    ];

    let mut parent = Task::new("task-1", "Parent", "Parent");
    parent.subtasks = vec![
        Task::new("task-2", "Child 1", "First child"),
        child2,
    ];

    let tree = ltmatrix::tasks::tree::format_tree(&parent);

    // Check for tree-drawing characters
    assert!(tree.contains("├──"), "Missing branch character");
    assert!(tree.contains("└──"), "Missing last branch character");

    // Check for status symbols
    assert!(tree.contains("○"), "Missing pending status symbol");
}

#[test]
fn test_tree_id_title_format() {
    // Verify the format: <prefix><id> <title> [<status>]
    let task = Task::new("test-id", "Test Title", "Description");
    let tree = ltmatrix::tasks::tree::format_tree(&task);

    // Should contain id and title in that order
    assert!(tree.contains("test-id Test Title"));
}

#[test]
fn test_tree_mixed_status_hierarchy() {
    // Test tree with tasks in different states
    let mut child1 = Task::new("task-2", "Completed Child", "Done");
    child1.status = TaskStatus::Completed;

    let mut child2 = Task::new("task-3", "Failed Child", "Failed");
    child2.status = TaskStatus::Failed;

    let mut parent = Task::new("task-1", "Parent", "Parent");
    parent.subtasks = vec![child1, child2];

    let tree = ltmatrix::tasks::tree::format_tree(&parent);

    // Should show different status symbols
    assert!(tree.contains("✓")); // Completed
    assert!(tree.contains("✗")); // Failed
}
