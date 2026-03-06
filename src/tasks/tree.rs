//! Task hierarchy tree visualization
//!
//! This module provides ASCII tree visualization for task hierarchies,
//! showing parent-child relationships using Unicode tree-drawing characters.

use crate::models::{Task, TaskStatus};

/// Formats a task and its subtasks as an ASCII tree
///
/// # Arguments
///
/// * `task` - The root task to visualize
///
/// # Returns
///
/// A string containing the formatted tree visualization
#[must_use]
pub fn format_tree(task: &Task) -> String {
    let mut result = String::new();
    format_task_recursive(task, "", &mut result);
    result
}

/// Helper function to format a single task line
fn format_task_line(task: &Task, prefix: &str) -> String {
    let status_symbol = status_to_symbol(&task.status);
    format!("{}{} {} [{}]", prefix, task.id, task.title, status_symbol)
}

/// Recursively formats a task and its children
///
/// # Arguments
///
/// * `task` - The task to format
/// * `prefix` - The prefix string for tree branches
/// * `output` - Output buffer to write to
fn format_task_recursive(task: &Task, prefix: &str, output: &mut String) {
    // Format and append the task line
    output.push_str(&format_task_line(task, prefix));
    output.push('\n');

    // Process subtasks
    if !task.subtasks.is_empty() {
        let subtask_count = task.subtasks.len();

        for (index, subtask) in task.subtasks.iter().enumerate() {
            let is_last = index == subtask_count - 1;

            // Create connector for this child
            let connector = if is_last { "└── " } else { "├── " };

            // Build the display prefix for this child
            let child_display_prefix = format!("{}{}", prefix, connector);

            // Build the continuation prefix for the child's descendants
            let child_continuation = build_continuation(prefix, is_last);

            // Append this child
            output.push_str(&format_task_line(subtask, &child_display_prefix));
            output.push('\n');

            // Recursively format the child's CHILDREN (not the child itself!)
            // The child has already been printed above
            for (grandchild_index, grandchild) in subtask.subtasks.iter().enumerate() {
                let grandchild_is_last = grandchild_index == subtask.subtasks.len() - 1;
                let grandchild_connector = if grandchild_is_last { "└── " } else { "├── " };
                let grandchild_prefix = format!("{}{}", child_continuation, grandchild_connector);
                format_task_recursive(grandchild, &grandchild_prefix, output);
            }
        }
    }
}

/// Builds the continuation prefix for descendants
///
/// # Arguments
///
/// * `parent_prefix` - The parent's prefix
/// * `parent_is_last` - Whether the parent is the last child
///
/// # Returns
///
/// The continuation prefix
fn build_continuation(parent_prefix: &str, parent_is_last: bool) -> String {
    if parent_prefix.is_empty() {
        // Root level
        if parent_is_last {
            "    ".to_string()
        } else {
            "│   ".to_string()
        }
    } else {
        // Nested level - replace parent's connector with continuation
        let base = if parent_prefix.ends_with("├── ") {
            format!("{}│   ", &parent_prefix[..parent_prefix.len() - 4])
        } else if parent_prefix.ends_with("└── ") {
            format!("{}    ", &parent_prefix[..parent_prefix.len() - 4])
        } else {
            parent_prefix.to_string()
        };

        if parent_is_last {
            format!("{}    ", base)
        } else {
            format!("{}│   ", base)
        }
    }
}

/// Converts a task status to a visual symbol
///
/// # Arguments
///
/// * `status` - The task status
///
/// # Returns
///
/// A string representing the status
#[must_use]
fn status_to_symbol(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "○",
        TaskStatus::InProgress => "⚙",
        TaskStatus::Completed => "✓",
        TaskStatus::Failed => "✗",
        TaskStatus::Blocked => "⚠",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tree_single_task() {
        let task = Task::new("task-1", "Main Task", "A task with no subtasks");
        let tree = format_tree(&task);

        assert!(tree.contains("task-1"));
        assert!(tree.contains("Main Task"));
        assert!(tree.contains("○")); // Pending status
    }

    #[test]
    fn test_format_tree_with_subtasks() {
        let mut parent = Task::new("task-1", "Parent Task", "Parent task");
        parent.subtasks = vec![
            Task::new("task-2", "Child 1", "First child"),
            Task::new("task-3", "Child 2", "Second child"),
        ];

        let tree = format_tree(&parent);

        assert!(tree.contains("task-1"));
        assert!(tree.contains("Parent Task"));
        assert!(tree.contains("task-2"));
        assert!(tree.contains("Child 1"));
        assert!(tree.contains("task-3"));
        assert!(tree.contains("Child 2"));
        assert!(tree.contains("├──"));
        assert!(tree.contains("└──"));
    }

    #[test]
    fn test_format_tree_nested_subtasks() {
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

        let tree = format_tree(&parent);

        assert!(tree.contains("task-1"));
        assert!(tree.contains("task-2"));
        assert!(tree.contains("task-3"));
        assert!(tree.contains("task-4"));
        assert!(tree.contains("task-5"));
        // The continuation characters should be present
        assert!(tree.contains("│   ") || tree.contains("    "));
    }

    #[test]
    fn test_status_to_symbol() {
        assert_eq!(status_to_symbol(&TaskStatus::Pending), "○");
        assert_eq!(status_to_symbol(&TaskStatus::InProgress), "⚙");
        assert_eq!(status_to_symbol(&TaskStatus::Completed), "✓");
        assert_eq!(status_to_symbol(&TaskStatus::Failed), "✗");
        assert_eq!(status_to_symbol(&TaskStatus::Blocked), "⚠");
    }

    #[test]
    fn test_format_tree_with_status() {
        let mut task = Task::new("task-1", "Main Task", "Task with status");
        task.status = TaskStatus::InProgress;

        let tree = format_tree(&task);

        assert!(tree.contains("⚙")); // InProgress symbol
        assert!(tree.contains("task-1"));
    }

    #[test]
    fn test_format_tree_empty_subtasks() {
        let task = Task::new("task-1", "Solo Task", "No children");
        let tree = format_tree(&task);

        assert!(tree.contains("task-1"));
        assert!(!tree.contains("├──"));
        assert!(!tree.contains("└──"));
    }

    #[test]
    fn test_format_tree_multiple_levels() {
        let mut level3 = Task::new("task-4", "Level 3", "Third level");
        level3.subtasks = vec![Task::new("task-5", "Level 4", "Fourth level")];

        let mut level2 = Task::new("task-3", "Level 2", "Second level");
        level2.subtasks = vec![level3];

        let mut level1 = Task::new("task-2", "Level 1", "First level");
        level1.subtasks = vec![level2];

        let mut root = Task::new("task-1", "Root", "Root task");
        root.subtasks = vec![level1];

        let tree = format_tree(&root);

        // All levels should be shown (implementation doesn't truncate)
        assert!(tree.contains("task-1"));
        assert!(tree.contains("task-2"));
        assert!(tree.contains("task-3"));
        assert!(tree.contains("task-4"));
        assert!(tree.contains("task-5"));
    }
}
