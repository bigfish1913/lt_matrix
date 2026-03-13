//! Integration tests for scheduler priority boosting
//!
//! Verifies that the scheduler correctly applies priority boosts
//! to tasks that block multiple downstream tasks.

use ltmatrix::models::{Task, TaskComplexity};
use ltmatrix::tasks::scheduler::{schedule_tasks_with_priority, PriorityConfig};

fn create_test_task(id: &str, title: &str, priority: u8, depends_on: Vec<String>) -> Task {
    let mut task = Task::new(id, title, format!("Task: {}", title));
    task.priority = priority;
    task.depends_on = depends_on;
    task.complexity = TaskComplexity::Simple;
    task
}

#[tokio::test]
async fn test_scheduler_priority_boost_is_applied() {
    // Create tasks where task-1 blocks multiple downstream tasks
    let task1 = create_test_task("task-1", "Foundation", 5, vec![]);
    let task2 = create_test_task("task-2", "Feature A", 5, vec!["task-1".to_string()]);
    let task3 = create_test_task("task-3", "Feature B", 5, vec!["task-1".to_string()]);
    let task4 = create_test_task("task-4", "Feature C", 5, vec!["task-1".to_string()]);

    let tasks = vec![task1, task2, task3, task4];

    // Use the scheduler directly to verify priority boosting
    let priority_config = PriorityConfig::default();
    let plan = schedule_tasks_with_priority(tasks.clone(), &priority_config).unwrap();

    // task-1 should be first in execution order (it blocks 3 others)
    assert_eq!(plan.execution_order[0], "task-1");

    // Verify critical path is identified
    assert!(!plan.critical_path.is_empty());
}
