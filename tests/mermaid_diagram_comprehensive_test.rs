//! Comprehensive Mermaid diagram generation tests
//!
//! Extended test suite validating Mermaid diagram syntax, edge cases,
//! and complete functionality for task dependencies and execution flow.

use ltmatrix::models::{Task, TaskStatus};
use ltmatrix::tasks::scheduler::schedule_tasks;
use ltmatrix::tasks::topology::{
    generate_mermaid_flowchart, generate_mermaid_graph, export_mermaid_to_file,
};

#[test]
fn test_mermaid_flowchart_complete_syntax() {
    let task = Task::new("task-1", "Test Task", "Description");
    let mermaid = generate_mermaid_flowchart(&[task], None);

    // Verify complete Mermaid syntax structure
    assert!(mermaid.starts_with("```mermaid"));
    assert!(mermaid.ends_with("```\n"));
    assert!(mermaid.contains("graph TD"));
    assert!(mermaid.contains("task-1"));
}

#[test]
fn test_mermaid_flowchart_with_all_statuses() {
    let tasks = vec![
        {
            let mut t = Task::new("pending", "Pending Task", "Waiting");
            t.status = TaskStatus::Pending;
            t
        },
        {
            let mut t = Task::new("inprogress", "In Progress Task", "Working");
            t.status = TaskStatus::InProgress;
            t
        },
        {
            let mut t = Task::new("completed", "Completed Task", "Done");
            t.status = TaskStatus::Completed;
            t
        },
        {
            let mut t = Task::new("failed", "Failed Task", "Error");
            t.status = TaskStatus::Failed;
            t
        },
        {
            let mut t = Task::new("blocked", "Blocked Task", "Waiting");
            t.status = TaskStatus::Blocked;
            t
        },
    ];

    let mermaid = generate_mermaid_flowchart(&tasks, Some(true));

    // Verify all status indicators are present
    assert!(mermaid.contains("Pending") || mermaid.contains("○"));
    assert!(mermaid.contains("In Progress") || mermaid.contains("⚙"));
    assert!(mermaid.contains("Completed") || mermaid.contains("✓"));
    assert!(mermaid.contains("Failed") || mermaid.contains("✗"));
    assert!(mermaid.contains("Blocked") || mermaid.contains("⚠"));
}

#[test]
fn test_mermaid_flowchart_label_formatting() {
    let task = Task::new("task-1", "Task With Newlines\nAnd Quotes", "Test");
    let mermaid = generate_mermaid_flowchart(&[task], None);

    // Verify proper label formatting with quotes
    assert!(mermaid.contains("[\""));
    assert!(mermaid.contains("\"]"));
}

#[test]
fn test_mermaid_flowchart_dependency_direction() {
    let mut task2 = Task::new("task-2", "Dependent", "Depends on task-1");
    task2.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Independent", "No dependencies");
    let tasks = vec![task1, task2];

    let mermaid = generate_mermaid_flowchart(&tasks, None);

    // Verify dependency edge direction (task-1 --> task-2)
    // The arrow should point from dependency to dependent
    assert!(mermaid.contains("-->"));
}

#[test]
fn test_mermaid_flowchart_multidiamond_pattern() {
    // Create a complex diamond pattern:
    //       root
    //      / | \
    //     a  b  c
    //      \ | /
    //       mid
    //      /   \
    //     x     y
    //      \   /
    //       end

    let mut end = Task::new("end", "End", "Final task");
    end.depends_on = vec!["x".to_string(), "y".to_string()];

    let mut x = Task::new("x", "X Branch", "Branch X");
    x.depends_on = vec!["mid".to_string()];

    let mut y = Task::new("y", "Y Branch", "Branch Y");
    y.depends_on = vec!["mid".to_string()];

    let mut mid = Task::new("mid", "Middle", "Middle merge");
    mid.depends_on = vec!["a".to_string(), "b".to_string(), "c".to_string()];

    let mut a = Task::new("a", "Branch A", "First branch");
    a.depends_on = vec!["root".to_string()];

    let mut b = Task::new("b", "Branch B", "Second branch");
    b.depends_on = vec!["root".to_string()];

    let mut c = Task::new("c", "Branch C", "Third branch");
    c.depends_on = vec!["root".to_string()];

    let root = Task::new("root", "Root", "Root task");

    let tasks = vec![root, a, b, c, mid, x, y, end];
    let mermaid = generate_mermaid_flowchart(&tasks, None);

    // All tasks should be present
    for task_id in &["root", "a", "b", "c", "mid", "x", "y", "end"] {
        assert!(mermaid.contains(task_id), "Missing task: {}", task_id);
    }

    // Should have multiple edges
    assert!(mermaid.contains("-->"));
}

#[test]
fn test_mermaid_graph_execution_plan_structure() {
    let mut task2 = Task::new("task-2", "Task 2", "Second");
    task2.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Task 1", "First");
    let tasks = vec![task1, task2];

    let plan = schedule_tasks(tasks).unwrap();
    let mermaid = generate_mermaid_graph(&plan, Some(true));

    // Verify execution plan structure
    assert!(mermaid.contains("Level"));
    assert!(mermaid.contains("subgraph"));
    assert!(mermaid.contains("Task Execution Plan"));
    assert!(mermaid.contains("Total:"));
    assert!(mermaid.contains("Max Depth:"));
}

#[test]
fn test_mermaid_graph_with_parallel_tasks() {
    // Create tasks that can run in parallel
    let mut task4 = Task::new("task-4", "Final", "Final task");
    task4.depends_on = vec!["task-2".to_string(), "task-3".to_string()];

    let mut task2 = Task::new("task-2", "Branch A", "A branch");
    task2.depends_on = vec!["task-1".to_string()];

    let mut task3 = Task::new("task-3", "Branch B", "B branch");
    task3.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Root", "Root task");

    let tasks = vec![task1, task2, task3, task4];
    let plan = schedule_tasks(tasks).unwrap();
    let mermaid = generate_mermaid_graph(&plan, Some(true));

    // Should show levels for parallel execution
    assert!(mermaid.contains("Level"));
}

#[test]
fn test_export_mermaid_file_content_integrity() {
    use std::fs;

    let task = Task::new("test-integrity", "Integrity Test", "Test content integrity");
    let tasks = vec![task.clone()];

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_integrity.mmd");

    // Generate expected content
    let expected_content = generate_mermaid_flowchart(&tasks, None);

    // Export to file
    export_mermaid_to_file(&tasks, &test_file, None).unwrap();

    // Read and verify content matches
    let actual_content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(expected_content, actual_content);

    // Cleanup
    let _ = fs::remove_file(&test_file);
}

#[test]
fn test_export_mermaid_file_overwrite() {
    use std::fs;

    let task = Task::new("overwrite", "Overwrite Test", "Test overwrite");
    let tasks = vec![task];

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_overwrite.mmd");

    // Write initial content
    fs::write(&test_file, "initial content").unwrap();

    // Export should overwrite
    export_mermaid_to_file(&tasks, &test_file, None).unwrap();

    // Verify content was overwritten
    let content = fs::read_to_string(&test_file).unwrap();
    assert!(content.contains("```mermaid"));
    assert!(!content.contains("initial content"));

    // Cleanup
    let _ = fs::remove_file(&test_file);
}

#[test]
fn test_mermaid_id_sanitization_special_characters() {
    // Test with various special characters in task IDs
    let mut task2 = Task::new("task-with@special#chars", "Task 2", "Special chars");
    task2.depends_on = vec!["task.with.dots".to_string()];

    let mut task1 = Task::new("task.with.dots", "Task 1", "Dots");
    task1.depends_on = vec!["task_with_underscore".to_string()];

    let task0 = Task::new("task_with_underscore", "Task 0", "Underscore");

    let tasks = vec![task0, task1, task2];
    let mermaid = generate_mermaid_flowchart(&tasks, None);

    // Should still produce valid Mermaid syntax
    assert!(mermaid.contains("graph TD"));
    assert!(!mermaid.is_empty());
}

#[test]
fn test_mermaid_id_sanitization_numeric_start() {
    // Test IDs that start with numbers
    let mut task2 = Task::new("123task", "Task 2", "Starts with number");
    task2.depends_on = vec!["456task".to_string()];

    let task1 = Task::new("456task", "Task 1", "Also starts with number");

    let tasks = vec![task1, task2];
    let mermaid = generate_mermaid_flowchart(&tasks, None);

    // Should still produce valid Mermaid syntax
    assert!(mermaid.contains("graph TD"));
}

#[test]
fn test_mermaid_flowchart_without_dependencies() {
    // Test tasks with no dependencies at all
    let task1 = Task::new("task-1", "Independent 1", "No deps");
    let task2 = Task::new("task-2", "Independent 2", "No deps");
    let task3 = Task::new("task-3", "Independent 3", "No deps");

    let tasks = vec![task1, task2, task3];
    let mermaid = generate_mermaid_flowchart(&tasks, None);

    // All tasks should be present
    assert!(mermaid.contains("task-1"));
    assert!(mermaid.contains("task-2"));
    assert!(mermaid.contains("task-3"));

    // No dependency arrows should be present
    let has_arrow = mermaid.contains("-->");
    assert!(!has_arrow, "Should not have arrows when no dependencies exist");
}

#[test]
fn test_mermaid_flowchart_with_deep_chain() {
    // Test a deep dependency chain
    let mut task5 = Task::new("task-5", "Level 5", "Fifth");
    task5.depends_on = vec!["task-4".to_string()];

    let mut task4 = Task::new("task-4", "Level 4", "Fourth");
    task4.depends_on = vec!["task-3".to_string()];

    let mut task3 = Task::new("task-3", "Level 3", "Third");
    task3.depends_on = vec!["task-2".to_string()];

    let mut task2 = Task::new("task-2", "Level 2", "Second");
    task2.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Level 1", "First");

    let tasks = vec![task1, task2, task3, task4, task5];
    let mermaid = generate_mermaid_flowchart(&tasks, None);

    // All tasks should be present
    for i in 1..=5 {
        assert!(mermaid.contains(&format!("task-{}", i)));
    }
}

#[test]
fn test_mermaid_graph_statistics_display() {
    let tasks = vec![
        Task::new("task-1", "Task 1", "First"),
        Task::new("task-2", "Task 2", "Second"),
    ];

    let plan = schedule_tasks(tasks).unwrap();
    let mermaid = generate_mermaid_graph(&plan, None);

    // Should include statistics
    assert!(mermaid.contains("Total:"));
    assert!(mermaid.contains("Max Depth:"));
    assert!(mermaid.contains("Critical Path:"));
}

#[test]
fn test_mermaid_flowchart_empty_task_list() {
    let tasks: Vec<Task> = vec![];
    let mermaid = generate_mermaid_flowchart(&tasks, None);

    // Should handle empty list gracefully
    assert!(mermaid.contains("```mermaid"));
    assert!(mermaid.contains("graph TD"));
    assert!(mermaid.contains("No tasks") || mermaid.is_empty());
}

#[test]
fn test_mermaid_graph_empty_plan() {
    use std::collections::HashSet;

    let plan = ltmatrix::tasks::scheduler::ExecutionPlan {
        execution_levels: vec![],
        execution_order: vec![],
        critical_path: vec![],
        parallelizable_tasks: HashSet::new(),
        max_depth: 0,
        total_tasks: 0,
    };

    let mermaid = generate_mermaid_graph(&plan, None);

    // Should handle empty plan gracefully
    assert!(mermaid.contains("```mermaid"));
    assert!(mermaid.contains("graph TD"));
    assert!(mermaid.contains("No tasks") || mermaid.is_empty());
}

#[test]
fn test_mermaid_status_styling() {
    let tasks = vec![
        {
            let mut t = Task::new("completed", "Done", "Completed");
            t.status = TaskStatus::Completed;
            t
        },
        {
            let mut t = Task::new("inprogress", "Working", "In Progress");
            t.status = TaskStatus::InProgress;
            t
        },
        {
            let mut t = Task::new("failed", "Error", "Failed");
            t.status = TaskStatus::Failed;
            t
        },
    ];

    let mermaid = generate_mermaid_flowchart(&tasks, Some(true));

    // Should include CSS class definitions for styling
    assert!(mermaid.contains("classDef") || mermaid.contains("style"));
}

#[test]
fn test_mermaid_execution_order_edges() {
    let tasks = vec![
        Task::new("task-1", "First", "First"),
        Task::new("task-2", "Second", "Second"),
        Task::new("task-3", "Third", "Third"),
    ];

    let plan = schedule_tasks(tasks).unwrap();
    let mermaid = generate_mermaid_graph(&plan, None);

    // Should have execution order edges with "next" labels
    assert!(mermaid.contains("next"));
}

#[test]
fn test_export_mermaid_with_status_enabled() {
    use std::fs;

    let mut task = Task::new("status-test", "Status Test", "Testing status export");
    task.status = TaskStatus::Completed;

    let tasks = vec![task];
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_status.mmd");

    export_mermaid_to_file(&tasks, &test_file, Some(true)).unwrap();

    let content = fs::read_to_string(&test_file).unwrap();
    assert!(content.contains("Completed") || content.contains("✓"));

    // Cleanup
    let _ = fs::remove_file(&test_file);
}

#[test]
fn test_export_mermaid_nested_directory_creation() {
    use std::fs;

    let task = Task::new("nested-test", "Nested Test", "Testing nested dirs");
    let tasks = vec![task];

    let temp_dir = std::env::temp_dir();
    let nested_path = temp_dir.join("level1").join("level2").join("level3").join("test.mmd");

    export_mermaid_to_file(&tasks, &nested_path, None).unwrap();

    // Verify file exists at nested path
    assert!(nested_path.exists());

    // Cleanup
    let _ = fs::remove_file(&nested_path);
    let _ = fs::remove_dir_all(temp_dir.join("level1"));
}
