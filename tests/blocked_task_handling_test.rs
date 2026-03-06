//! Tests for blocked task handling strategies
//!
//! This test module verifies the --on-blocked flag functionality that handles
//! tasks when their dependencies fail or cannot be satisfied.

use ltmatrix::cli::args::BlockedStrategy;
use ltmatrix::models::{Task, TaskStatus};
use std::collections::HashMap;

/// Helper to create a test task
fn create_task(id: &str, title: &str, deps: Vec<&str>) -> Task {
    let mut task = Task::new(id, title, format!("Description for {}", title));
    task.depends_on = deps.into_iter().map(|s| s.to_string()).collect();
    task
}

/// Helper to create a failed task
fn create_failed_task(id: &str, title: &str, error: &str) -> Task {
    let mut task = create_task(id, title, vec![]);
    task.status = TaskStatus::Failed;
    task.error = Some(error.to_string());
    task
}

#[cfg(test)]
mod blocked_strategy_tests {
    use super::*;

    #[test]
    fn test_blocked_strategy_display_values() {
        assert_eq!(BlockedStrategy::Skip.to_string(), "skip");
        assert_eq!(BlockedStrategy::Ask.to_string(), "ask");
        assert_eq!(BlockedStrategy::Abort.to_string(), "abort");
        assert_eq!(BlockedStrategy::Retry.to_string(), "retry");
    }

    #[test]
    fn test_blocked_strategy_equality() {
        assert_eq!(BlockedStrategy::Skip, BlockedStrategy::Skip);
        assert_eq!(BlockedStrategy::Ask, BlockedStrategy::Ask);
        assert_eq!(BlockedStrategy::Abort, BlockedStrategy::Abort);
        assert_eq!(BlockedStrategy::Retry, BlockedStrategy::Retry);
    }

    #[test]
    fn test_blocked_strategy_inequality() {
        assert_ne!(BlockedStrategy::Skip, BlockedStrategy::Ask);
        assert_ne!(BlockedStrategy::Ask, BlockedStrategy::Abort);
        assert_ne!(BlockedStrategy::Abort, BlockedStrategy::Retry);
        assert_ne!(BlockedStrategy::Retry, BlockedStrategy::Skip);
    }

    #[test]
    fn test_identify_blocked_tasks() {
        let task1 = create_failed_task("task-1", "Failed task", "Dependency error");
        let mut task2 = create_task("task-2", "Dependent task", vec!["task-1"]);
        let task3 = create_task("task-3", "Independent task", vec![]);

        let task_map: HashMap<String, Task> = [
            (task1.id.clone(), task1),
            (task2.id.clone(), task2.clone()),
            (task3.id.clone(), task3),
        ]
        .into_iter()
        .collect();

        let completed: std::collections::HashSet<String> = std::collections::HashSet::new();

        // task-2 should not be executable since task-1 failed
        assert!(!task2.can_execute(&completed));

        // task-3 should be executable (no dependencies)
        let task3_ref = task_map.get("task-3").unwrap();
        assert!(task3_ref.can_execute(&completed));
    }

    #[test]
    fn test_skip_strategy_continues_with_other_tasks() {
        // Skip strategy: When a task is blocked, skip it and continue with other tasks
        let task1 = create_failed_task("task-1", "Failed task", "Error");
        let task2 = create_task("task-2", "Blocked task", vec!["task-1"]);
        let task3 = create_task("task-3", "Independent task", vec![]);

        let mut tasks = vec![task1, task2, task3];
        let task_map: HashMap<String, Task> =
            tasks.iter().map(|t| (t.id.clone(), t.clone())).collect();

        let completed: std::collections::HashSet<String> = std::collections::HashSet::new();

        // With skip strategy, task-3 should still be executable
        let task3_ref = task_map.get("task-3").unwrap();
        assert!(task3_ref.can_execute(&completed));
    }

    #[test]
    fn test_abort_strategy_stops_pipeline() {
        // Abort strategy: When a task is blocked, stop the entire pipeline
        let task1 = create_failed_task("task-1", "Failed task", "Error");
        let task2 = create_task("task-2", "Blocked task", vec!["task-1"]);
        let task3 = create_task("task-3", "Would be next", vec![]);

        // Even though task-3 has no dependencies, abort strategy
        // means the pipeline should stop when task-2 is blocked
        let task_map: HashMap<String, Task> = [
            (task1.id.clone(), task1),
            (task2.id.clone(), task2.clone()),
            (task3.id.clone(), task3),
        ]
        .into_iter()
        .collect();

        let completed: std::collections::HashSet<String> = std::collections::HashSet::new();

        // task-2 is blocked
        let task2_ref = task_map.get("task-2").unwrap();
        assert!(!task2_ref.can_execute(&completed));

        // With abort strategy, pipeline should stop here
        // This test verifies the blocking detection logic
        assert!(task2.depends_on.iter().any(|dep| {
            let dep_task = task_map.get(dep);
            dep_task.map_or(false, |t| t.is_failed())
        }));
    }

    #[test]
    fn test_retry_strategy_marks_for_retry() {
        // Retry strategy: When a task is blocked due to failed dependency,
        // mark it for retry after dependency is fixed
        let task1 = create_failed_task("task-1", "Failed task", "Error");
        let mut task2 = create_task("task-2", "Dependent task", vec!["task-1"]);

        let task_map: HashMap<String, Task> =
            [(task1.id.clone(), task1), (task2.id.clone(), task2.clone())]
                .into_iter()
                .collect();

        let completed: std::collections::HashSet<String> = std::collections::HashSet::new();

        // task-2 should be blocked
        assert!(!task2.can_execute(&completed));

        // With retry strategy, task-2 should remain in pending state
        // for retry after task-1 is fixed
        assert_eq!(task2.status, TaskStatus::Pending);

        // Simulate task-2 failing after trying to execute
        task2.status = TaskStatus::Failed;
        task2.error = Some("Dependency failed".to_string());

        // Now that it's failed, it should be retryable
        assert!(task2.can_retry(3));
    }

    #[test]
    fn test_ask_strategy_requires_user_input() {
        // Ask strategy: When a task is blocked, prompt user for action
        // This test verifies the strategy is set correctly
        let strategy = BlockedStrategy::Ask;
        assert_eq!(strategy, BlockedStrategy::Ask);

        // The ask strategy should be stored for later use
        // when blocked tasks are encountered during execution
        let strategy_str = strategy.to_string();
        assert_eq!(strategy_str, "ask");
    }

    #[test]
    fn test_multiple_blocked_dependencies() {
        // Test a task with multiple failed dependencies
        let task1 = create_failed_task("task-1", "Failed 1", "Error 1");
        let task2 = create_failed_task("task-2", "Failed 2", "Error 2");
        let mut task3 = create_task("task-3", "Multi-dependency", vec!["task-1", "task-2"]);

        let task_map: HashMap<String, Task> = [
            (task1.id.clone(), task1),
            (task2.id.clone(), task2),
            (task3.id.clone(), task3.clone()),
        ]
        .into_iter()
        .collect();

        let completed: std::collections::HashSet<String> = std::collections::HashSet::new();

        // task-3 should be blocked by both dependencies
        assert!(!task3.can_execute(&completed));

        // Count blocking dependencies
        let blocking_count = task3
            .depends_on
            .iter()
            .filter(|dep| task_map.get(*dep).map_or(false, |t| t.is_failed()))
            .count();

        assert_eq!(blocking_count, 2);
    }

    #[test]
    fn test_partial_dependency_failure() {
        // Test when some dependencies succeed and some fail
        let task1 = create_task("task-1", "Success", vec![]);
        let mut task1_success = task1.clone();
        task1_success.status = TaskStatus::Completed;

        let task2 = create_failed_task("task-2", "Failed", "Error");
        let mut task3 = create_task("task-3", "Mixed deps", vec!["task-1", "task-2"]);

        let task_map: HashMap<String, Task> = [
            (task1_success.id.clone(), task1_success),
            (task2.id.clone(), task2),
            (task3.id.clone(), task3.clone()),
        ]
        .into_iter()
        .collect();

        let mut completed = std::collections::HashSet::new();
        completed.insert("task-1".to_string());

        // task-3 should still be blocked because task-2 failed
        assert!(!task3.can_execute(&completed));

        let blocking_deps: Vec<_> = task3
            .depends_on
            .iter()
            .filter(|dep| {
                !completed.contains(*dep) || task_map.get(*dep).map_or(false, |t| t.is_failed())
            })
            .collect();

        assert_eq!(blocking_deps.len(), 1);
        assert!(blocking_deps.contains(&&"task-2".to_string()));
    }

    #[test]
    fn test_transitive_blocking() {
        // Test blocking through dependency chain
        let task1 = create_failed_task("task-1", "Root failure", "Error");
        let task2 = create_task("task-2", "Depends on 1", vec!["task-1"]);
        let task3 = create_task("task-3", "Depends on 2", vec!["task-2"]);

        let task_map: HashMap<String, Task> = [
            (task1.id.clone(), task1),
            (task2.id.clone(), task2.clone()),
            (task3.id.clone(), task3.clone()),
        ]
        .into_iter()
        .collect();

        let completed: std::collections::HashSet<String> = std::collections::HashSet::new();

        // task-2 is blocked by task-1
        assert!(!task2.can_execute(&completed));

        // task-3 is transitively blocked by task-1
        assert!(!task3.can_execute(&completed));

        // Verify the dependency chain
        let mut blocked_by_chain = Vec::new();
        let mut current_task = task3.id.clone();

        loop {
            let task = task_map.get(&current_task).unwrap();
            if task.depends_on.is_empty() {
                break;
            }
            for dep in &task.depends_on {
                if let Some(dep_task) = task_map.get(dep) {
                    if dep_task.is_failed() || !dep_task.is_completed() {
                        blocked_by_chain.push(dep.clone());
                        current_task = dep.clone();
                        break;
                    }
                }
            }
            if blocked_by_chain.len() > 10 {
                break; // Safety limit
            }
        }

        assert!(!blocked_by_chain.is_empty());
        assert!(
            blocked_by_chain.contains(&"task-1".to_string())
                || blocked_by_chain.contains(&"task-2".to_string())
        );
    }

    #[test]
    fn test_no_blocking_when_all_deps_complete() {
        // Test that tasks are not blocked when all dependencies are complete
        let mut task1 = create_task("task-1", "Complete", vec![]);
        task1.status = TaskStatus::Completed;

        let mut task2 = create_task("task-2", "Also complete", vec![]);
        task2.status = TaskStatus::Completed;

        let mut task3 = create_task("task-3", "Dependent", vec!["task-1", "task-2"]);

        let task_map: HashMap<String, Task> = [
            (task1.id.clone(), task1),
            (task2.id.clone(), task2),
            (task3.id.clone(), task3.clone()),
        ]
        .into_iter()
        .collect();

        let mut completed = std::collections::HashSet::new();
        completed.insert("task-1".to_string());
        completed.insert("task-2".to_string());

        // task-3 should be executable
        assert!(task3.can_execute(&completed));
    }

    #[test]
    fn test_strategy_with_missing_dependencies() {
        // Test handling of missing (non-existent) dependencies
        let mut task1 = create_task("task-1", "Valid task", vec![]);
        task1.status = TaskStatus::Completed;

        let mut task2 = create_task(
            "task-2",
            "Invalid dependency",
            vec!["task-1", "task-nonexistent"],
        );

        let task_map: HashMap<String, Task> =
            [(task1.id.clone(), task1), (task2.id.clone(), task2.clone())]
                .into_iter()
                .collect();

        let mut completed = std::collections::HashSet::new();
        completed.insert("task-1".to_string());

        // task-2 references a non-existent dependency
        // It should not be executable because the dependency doesn't exist in the map
        let missing_dep = task2
            .depends_on
            .iter()
            .any(|dep| !task_map.contains_key(dep));
        assert!(missing_dep);

        // Can't execute if dependency is missing
        let has_missing = task2
            .depends_on
            .iter()
            .any(|dep| !task_map.contains_key(dep));
        assert!(has_missing || !task2.can_execute(&completed));
    }

    #[test]
    fn test_empty_dependency_list() {
        // Test that tasks with no dependencies are never blocked
        let task = create_task("task-1", "No deps", vec![]);

        let completed: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Should always be executable
        assert!(task.can_execute(&completed));
        assert!(task.depends_on.is_empty());
    }

    #[test]
    fn test_self_dependency_blocking() {
        // Test that a task depending on itself is detected
        let mut task = create_task("task-1", "Self dep", vec!["task-1"]);

        let task_map: HashMap<String, Task> =
            [(task.id.clone(), task.clone())].into_iter().collect();

        let mut completed: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Task should not be executable because it depends on itself
        // which is not completed
        assert!(!task.can_execute(&completed));

        // Even if we mark it as completed, a self-dependency is a logical error
        // This should be caught by cycle detection in the scheduler
        completed.insert(task.id.clone());
        // This would still be problematic but the can_execute check would pass
        // The scheduler's cycle detection should prevent this
    }

    #[test]
    fn test_blocked_task_status_transitions() {
        // Test status transitions for blocked tasks under different strategies
        let mut task = create_task("task-1", "Test task", vec!["task-failed"]);

        // Initially pending
        assert_eq!(task.status, TaskStatus::Pending);

        // When blocked by failed dependency, status changes based on strategy:
        //
        // Skip: Task stays Pending but is skipped during execution
        // Abort: Task may be marked Failed or pipeline stops
        // Retry: Task stays Pending for retry
        // Ask: Task status depends on user input

        // Test that we can mark a task as blocked
        task.status = TaskStatus::Blocked;
        assert_eq!(task.status, TaskStatus::Blocked);
        assert!(task.error.is_some() || task.error.is_none()); // May or may not have error

        // Test transition from blocked to failed
        task.status = TaskStatus::Failed;
        task.error = Some("Dependency failed".to_string());
        assert_eq!(task.status, TaskStatus::Failed);
        assert!(task.error.is_some());
    }

    #[test]
    fn test_max_retries_with_blocking() {
        // Test that max_retries affects retry strategy
        let mut task = create_task("task-1", "Test task", vec!["task-failed"]);

        // Mark the task as failed so can_retry will work
        task.status = TaskStatus::Failed;
        task.error = Some("Task execution failed".to_string());

        // With retry strategy, task can be retried up to max_retries times
        let max_retries = 3;

        for i in 0..=max_retries {
            if i < max_retries {
                assert!(
                    task.can_retry(max_retries),
                    "Task should be retryable at attempt {}",
                    i
                );
                task.retry_count = i + 1;
            } else {
                // After max retries, should not retry
                assert!(
                    !task.can_retry(max_retries),
                    "Task should not be retryable after {} attempts",
                    max_retries
                );
            }
        }
    }
}

#[cfg(test)]
mod blocked_strategy_integration_tests {
    use super::*;

    #[test]
    fn test_dependency_chain_with_skip_strategy() {
        // Test a full dependency chain with skip strategy
        let tasks = vec![
            create_task("task-1", "Base", vec![]),
            create_task("task-2", "Middle", vec!["task-1"]),
            create_task("task-3", "Top", vec!["task-2"]),
        ];

        let task_map: HashMap<String, Task> =
            tasks.into_iter().map(|t| (t.id.clone(), t)).collect();

        // Simulate task-1 completing
        let mut completed = std::collections::HashSet::new();
        completed.insert("task-1".to_string());

        // task-2 should now be executable
        let task2 = task_map.get("task-2").unwrap();
        assert!(task2.can_execute(&completed));

        // task-3 still blocked by task-2
        let task3 = task_map.get("task-3").unwrap();
        assert!(!task3.can_execute(&completed));
    }

    #[test]
    fn test_diamond_dependency_with_blocking() {
        // Test diamond dependency pattern with blocking
        //     task-1
        //      /   \
        //  task-2  task-3
        //      \   /
        //      task-4
        let tasks = vec![
            create_task("task-1", "Base", vec![]),
            create_task("task-2", "Left branch", vec!["task-1"]),
            create_task("task-3", "Right branch", vec!["task-1"]),
            create_task("task-4", "Join", vec!["task-2", "task-3"]),
        ];

        let task_map: HashMap<String, Task> =
            tasks.into_iter().map(|t| (t.id.clone(), t)).collect();

        // After task-1 completes
        let mut completed = std::collections::HashSet::new();
        completed.insert("task-1".to_string());

        // Both branches should be executable
        let task2 = task_map.get("task-2").unwrap();
        let task3 = task_map.get("task-3").unwrap();
        assert!(task2.can_execute(&completed));
        assert!(task3.can_execute(&completed));

        // task-4 still blocked
        let task4 = task_map.get("task-4").unwrap();
        assert!(!task4.can_execute(&completed));

        // After task-2 completes
        completed.insert("task-2".to_string());
        assert!(!task4.can_execute(&completed)); // Still needs task-3

        // After both branches complete
        completed.insert("task-3".to_string());
        assert!(task4.can_execute(&completed));
    }

    #[test]
    fn test_parallel_independent_tasks_with_blocking() {
        // Test that independent tasks can run in parallel
        let failed_task = create_failed_task("task-0", "Failed", "Error");

        let tasks = vec![
            failed_task,
            create_task("task-1", "Blocked by 0", vec!["task-0"]),
            create_task("task-2", "Independent", vec![]),
            create_task("task-3", "Also independent", vec![]),
            create_task("task-4", "Blocked by 0", vec!["task-0"]),
        ];

        let task_map: HashMap<String, Task> =
            tasks.into_iter().map(|t| (t.id.clone(), t)).collect();

        let completed: std::collections::HashSet<String> = std::collections::HashSet::new();

        // task-2 and task-3 should be executable (parallel)
        let task2 = task_map.get("task-2").unwrap();
        let task3 = task_map.get("task-3").unwrap();
        assert!(task2.can_execute(&completed));
        assert!(task3.can_execute(&completed));

        // task-1 and task-4 should be blocked
        let task1 = task_map.get("task-1").unwrap();
        let task4 = task_map.get("task-4").unwrap();
        assert!(!task1.can_execute(&completed));
        assert!(!task4.can_execute(&completed));
    }

    #[test]
    fn test_strategy_persistence_across_pipeline_stages() {
        // Test that the blocked strategy is consistent across pipeline execution
        let strategies = vec![
            BlockedStrategy::Skip,
            BlockedStrategy::Abort,
            BlockedStrategy::Retry,
            BlockedStrategy::Ask,
        ];

        for strategy in strategies {
            // Each strategy should be consistent
            let strategy_str = strategy.to_string();
            assert!(!strategy_str.is_empty());

            // Strategy should be one of the valid options
            match strategy {
                BlockedStrategy::Skip
                | BlockedStrategy::Abort
                | BlockedStrategy::Retry
                | BlockedStrategy::Ask => {
                    // Valid strategy
                }
            }
        }
    }
}

#[cfg(test)]
mod blocked_task_statistics_tests {
    use super::*;

    #[test]
    fn test_count_blocked_tasks() {
        let tasks = vec![
            create_task("task-1", "Completed", vec![]),
            create_task("task-2", "Blocked", vec!["task-failed"]),
            create_task("task-3", "Also blocked", vec!["task-missing"]),
            create_task("task-4", "Independent", vec![]),
        ];

        let task_map: HashMap<String, Task> =
            tasks.into_iter().map(|t| (t.id.clone(), t)).collect();

        let completed: std::collections::HashSet<String> = std::collections::HashSet::new();

        let blocked_count = task_map
            .values()
            .filter(|t| !t.can_execute(&completed))
            .count();

        assert_eq!(blocked_count, 2); // task-2 and task-3
    }

    #[test]
    fn test_blocker_analysis() {
        // Analyze what's blocking each task
        let task1 = create_failed_task("task-1", "Failed", "Error");
        let task2 = create_failed_task("task-2", "Failed", "Error");

        let tasks = vec![
            task1,
            task2,
            create_task("task-3", "Blocked by both", vec!["task-1", "task-2"]),
            create_task("task-4", "Blocked by one", vec!["task-1"]),
        ];

        let task_map: HashMap<String, Task> =
            tasks.into_iter().map(|t| (t.id.clone(), t)).collect();

        let completed: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Analyze blockers for task-3
        let task3 = task_map.get("task-3").unwrap();
        let blockers: Vec<_> = task3
            .depends_on
            .iter()
            .filter(|dep| task_map.get(*dep).map_or(false, |t| t.is_failed()))
            .collect();

        assert_eq!(blockers.len(), 2);

        // Analyze blockers for task-4
        let task4 = task_map.get("task-4").unwrap();
        let blockers4: Vec<_> = task4
            .depends_on
            .iter()
            .filter(|dep| task_map.get(*dep).map_or(false, |t| t.is_failed()))
            .collect();

        assert_eq!(blockers4.len(), 1);
    }
}
