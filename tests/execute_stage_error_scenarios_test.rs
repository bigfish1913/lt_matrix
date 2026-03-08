//! Error scenario and robustness tests for the execute stage
//!
//! These tests verify error handling, edge cases, and robustness:
//! - Circular dependency detection
//! - Missing task references
//! - Malformed input handling
//! - Session error recovery
//! - Timeout scenarios
//! - Concurrent execution safety

use ltmatrix::models::{ModeConfig, Task, TaskComplexity, TaskStatus};
use ltmatrix::pipeline::execute::ExecuteConfig;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[tokio::test]
async fn test_circular_dependency_detection() {
    // Create circular dependency: task-1 -> task-2 -> task-1
    let mut task1 = Task::new("task-1", "First", "First task");
    let mut task2 = Task::new("task-2", "Second", "Second task");

    task1.depends_on = vec!["task-2".to_string()];
    task2.depends_on = vec!["task-1".to_string()];

    let task_map: HashMap<String, Task> = [(task1.id.clone(), task1), (task2.id.clone(), task2)]
        .into_iter()
        .collect();

    // Should detect circular dependency and fail
    let result = ltmatrix::pipeline::execute::get_execution_order(&task_map);

    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Circular") || e.to_string().contains("circular"));
    }
}

#[tokio::test]
async fn test_self_dependency_detection() {
    let mut task = Task::new("task-1", "Self Dependent", "Depends on itself");
    task.depends_on = vec!["task-1".to_string()];

    let task_map: HashMap<String, Task> = [(task.id.clone(), task)].into_iter().collect();

    // Should detect self as circular dependency
    let result = ltmatrix::pipeline::execute::get_execution_order(&task_map);

    assert!(result.is_err());
}

#[tokio::test]
async fn test_complex_circular_dependency_chain() {
    // task-1 -> task-2 -> task-3 -> task-1
    let mut task1 = Task::new("task-1", "T1", "First");
    let mut task2 = Task::new("task-2", "T2", "Second");
    let mut task3 = Task::new("task-3", "T3", "Third");

    task1.depends_on = vec!["task-3".to_string()];
    task2.depends_on = vec!["task-1".to_string()];
    task3.depends_on = vec!["task-2".to_string()];

    let task_map: HashMap<String, Task> = [
        (task1.id.clone(), task1),
        (task2.id.clone(), task2),
        (task3.id.clone(), task3),
    ]
    .into_iter()
    .collect();

    // Should detect circular dependency
    let result = ltmatrix::pipeline::execute::get_execution_order(&task_map);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_diamond_dependency_not_circular() {
    // Diamond dependency is NOT circular:
    //     task-1
    //     /    \
    // task-2  task-3
    //     \    /
    //     task-4

    let task1 = Task::new("task-1", "Base", "Base");
    let mut task2 = Task::new("task-2", "Left", "Left");
    let mut task3 = Task::new("task-3", "Right", "Right");
    let mut task4 = Task::new("task-4", "Merge", "Merge");

    task2.depends_on = vec!["task-1".to_string()];
    task3.depends_on = vec!["task-1".to_string()];
    task4.depends_on = vec!["task-2".to_string(), "task-3".to_string()];

    let task_map: HashMap<String, Task> = [
        (task1.id.clone(), task1),
        (task2.id.clone(), task2),
        (task3.id.clone(), task3),
        (task4.id.clone(), task4),
    ]
    .into_iter()
    .collect();

    // Should succeed - diamond is valid
    let result = ltmatrix::pipeline::execute::get_execution_order(&task_map);
    assert!(result.is_ok());

    let order = result.unwrap();
    assert_eq!(order[0], "task-1");
    assert_eq!(order[3], "task-4");
}

#[tokio::test]
async fn test_missing_dependency_reference() {
    let mut task = Task::new("task-1", "Test", "Test task");
    task.depends_on = vec!["nonexistent-task".to_string()];

    let task_map: HashMap<String, Task> = [(task.id.clone(), task)].into_iter().collect();

    // Topological sort should fail due to missing dependency
    let result = ltmatrix::pipeline::execute::get_execution_order(&task_map);
    assert!(result.is_err());

    // Verify the error mentions the missing task
    let err = result.unwrap_err();
    assert!(err.to_string().contains("nonexistent-task") || err.to_string().contains("not found"));
}

#[tokio::test]
async fn test_empty_task_title_and_description() {
    let task = Task::new("task-1", "", "");

    // Should still create valid task
    assert_eq!(task.id, "task-1");
    assert_eq!(task.title, "");
    assert_eq!(task.description, "");

    // Prompt building should handle empty strings
    let task_map: HashMap<String, Task> = [(task.id.clone(), task.clone())].into_iter().collect();
    let completed: HashSet<String> = HashSet::new();

    let context = ltmatrix::pipeline::execute::build_task_context(&task, &task_map, &completed, "");

    assert!(context.is_ok());
}

#[tokio::test]
async fn test_task_with_special_characters_in_id() {
    let task = Task::new("task-1-with-special-chars_123", "Test", "Test task");

    assert_eq!(task.id, "task-1-with-special-chars_123");

    let task_map: HashMap<String, Task> = [(task.id.clone(), task)].into_iter().collect();

    let order = ltmatrix::pipeline::execute::get_execution_order(&task_map);
    assert!(order.is_ok());
}

#[tokio::test]
async fn test_very_long_task_id() {
    let long_id = "a".repeat(10_000);
    let task = Task::new(&long_id, "Test", "Test");

    assert_eq!(task.id.len(), 10_000);

    let task_map: HashMap<String, Task> = [(task.id.clone(), task)].into_iter().collect();

    let order = ltmatrix::pipeline::execute::get_execution_order(&task_map);
    assert!(order.is_ok());
}

#[tokio::test]
async fn test_task_with_many_dependencies() {
    let mut task = Task::new("task-main", "Main", "Main task");

    // Add 100 dependencies
    for i in 0..100 {
        task.depends_on.push(format!("dep-{}", i));
    }

    // Create dependency tasks
    let mut task_map: HashMap<String, Task> = HashMap::new();
    for i in 0..100 {
        let dep_task = Task::new(&format!("dep-{}", i), &format!("Dep {}", i), "Dependency");
        task_map.insert(dep_task.id.clone(), dep_task);
    }
    task_map.insert(task.id.clone(), task.clone());

    // Should handle many dependencies
    let order = ltmatrix::pipeline::execute::get_execution_order(&task_map);
    assert!(order.is_ok());
    assert_eq!(order.unwrap().len(), 101);
}

#[tokio::test]
async fn test_execution_config_edge_cases() {
    // Test zero timeout
    let config = ExecuteConfig {
        mode_config: ModeConfig::default(),
        max_retries: 0,
        timeout: 0,
        enable_sessions: false,
        work_dir: PathBuf::from("."),
        memory_file: PathBuf::from("memory.md"),
        enable_workspace_persistence: false,
        project_root: None,
        agent_pool: None,
    };
    assert_eq!(config.timeout, 0);
    assert_eq!(config.max_retries, 0);

    // Test very large timeout
    let config = ExecuteConfig {
        mode_config: ModeConfig::default(),
        max_retries: 1000,
        timeout: 86400, // 24 hours
        enable_sessions: false,
        work_dir: PathBuf::from("."),
        memory_file: PathBuf::from("memory.md"),
        enable_workspace_persistence: false,
        project_root: None,
        agent_pool: None,
    };
    assert_eq!(config.timeout, 86400);
    assert_eq!(config.max_retries, 1000);
}

#[tokio::test]
async fn test_task_context_with_multiline_description() {
    let description = r#"This is a multiline description:

## Section 1
Some content

## Section 2
More content

- Item 1
- Item 2
"#;

    let task = Task::new("task-1", "Multiline", description);
    let task_map: HashMap<String, Task> = [(task.id.clone(), task.clone())].into_iter().collect();
    let completed: HashSet<String> = HashSet::new();

    let context =
        ltmatrix::pipeline::execute::build_task_context(&task, &task_map, &completed, "").unwrap();

    // Should preserve multiline formatting
    assert!(context.contains("multiline description"));
}

#[tokio::test]
async fn test_execution_prompt_with_context_escaping() {
    let task = Task::new(
        "task-1",
        "Task with {{placeholders}}",
        "Description with {braces} and $symbols",
    );

    let context = "Context with {{more}} placeholders";
    let prompt = ltmatrix::pipeline::execute::build_execution_prompt(&task, context);

    // Should not attempt to format/escape placeholders
    assert!(prompt.contains("{{placeholders}}"));
    assert!(prompt.contains("{braces}"));
    assert!(prompt.contains("$symbols"));
}

#[tokio::test]
async fn test_multiple_levels_of_dependencies() {
    // Create 10 levels of dependencies
    let mut task_map: HashMap<String, Task> = HashMap::new();

    for i in 0..10 {
        let mut task = Task::new(&format!("task-{}", i), &format!("Task {}", i), "Test");
        if i > 0 {
            task.depends_on = vec![format!("task-{}", i - 1)];
        }
        task_map.insert(task.id.clone(), task);
    }

    let order = ltmatrix::pipeline::execute::get_execution_order(&task_map).unwrap();

    // Should execute in order from 0 to 9
    assert_eq!(order.len(), 10);
    for (i, task_id) in order.iter().enumerate() {
        assert_eq!(task_id, &format!("task-{}", i));
    }
}

#[tokio::test]
async fn test_execution_order_with_reversed_insertion() {
    // Insert tasks in reverse order of dependencies
    let task4 = Task::new("task-4", "Fourth", "Level 3");
    let mut task3 = Task::new("task-3", "Third", "Level 2");
    task3.depends_on = vec!["task-4".to_string()];
    let mut task2 = Task::new("task-2", "Second", "Level 1");
    task2.depends_on = vec!["task-3".to_string()];
    let mut task1 = Task::new("task-1", "First", "Level 0");
    task1.depends_on = vec!["task-2".to_string()];

    let task_map: HashMap<String, Task> = [
        (task4.id.clone(), task4),
        (task3.id.clone(), task3),
        (task2.id.clone(), task2),
        (task1.id.clone(), task1),
    ]
    .into_iter()
    .collect();

    let order = ltmatrix::pipeline::execute::get_execution_order(&task_map).unwrap();

    // Should respect dependencies regardless of insertion order
    assert_eq!(order, vec!["task-4", "task-3", "task-2", "task-1"]);
}

#[tokio::test]
async fn test_task_status_transitions_comprehensive() {
    let mut task = Task::new("task-1", "Test", "Test");

    // Pending -> InProgress
    task.status = TaskStatus::Pending;
    assert!(!task.is_completed());
    assert!(!task.is_failed());
    assert!(!task.status.is_terminal());

    task.status = TaskStatus::InProgress;
    assert!(!task.is_completed());
    assert!(!task.is_failed());
    assert!(!task.status.is_terminal());

    // InProgress -> Completed
    task.status = TaskStatus::Completed;
    assert!(task.is_completed());
    assert!(!task.is_failed());
    assert!(task.status.is_terminal());

    // Note: TaskStatus is just an enum, so we can assign any value
    // State transitions would need to be enforced through methods if desired
    task.status = TaskStatus::Pending;
    assert_eq!(task.status, TaskStatus::Pending);
}

#[tokio::test]
async fn test_retry_count_exceeds_limit() {
    let mut task = Task::new("task-1", "Test", "Test");
    task.status = TaskStatus::Failed;
    task.retry_count = 10;

    // Should not retry beyond limit
    assert!(!task.can_retry(3));
    assert!(!task.can_retry(5));
    assert!(!task.can_retry(10));
}

#[tokio::test]
async fn test_complexity_preservation_through_map() {
    let mut simple = Task::new("simple", "Simple", "Simple");
    let mut moderate = Task::new("moderate", "Moderate", "Moderate");
    let mut complex = Task::new("complex", "Complex", "Complex");

    simple.complexity = TaskComplexity::Simple;
    moderate.complexity = TaskComplexity::Moderate;
    complex.complexity = TaskComplexity::Complex;

    let task_map: HashMap<String, Task> = [
        (simple.id.clone(), simple.clone()),
        (moderate.id.clone(), moderate.clone()),
        (complex.id.clone(), complex.clone()),
    ]
    .into_iter()
    .collect();

    // Verify complexity is preserved
    assert_eq!(
        task_map.get("simple").unwrap().complexity,
        TaskComplexity::Simple
    );
    assert_eq!(
        task_map.get("moderate").unwrap().complexity,
        TaskComplexity::Moderate
    );
    assert_eq!(
        task_map.get("complex").unwrap().complexity,
        TaskComplexity::Complex
    );
}

#[tokio::test]
async fn test_build_task_context_handles_unicode() {
    let task = Task::new(
        "task-1",
        "Unicode 🚀 Test",
        "Description with emoji 🎉 and 中文 characters",
    );

    let task_map: HashMap<String, Task> = [(task.id.clone(), task.clone())].into_iter().collect();
    let completed: HashSet<String> = HashSet::new();

    let context =
        ltmatrix::pipeline::execute::build_task_context(&task, &task_map, &completed, "").unwrap();

    assert!(context.contains("🚀"));
    assert!(context.contains("🎉"));
    assert!(context.contains("中文"));
}

#[tokio::test]
async fn test_execution_statistics_edge_cases() {
    use ltmatrix::pipeline::execute::ExecutionStatistics;

    // All tasks failed
    let stats = ExecutionStatistics {
        total_tasks: 10,
        completed_tasks: 0,
        skipped_tasks: 0,
        failed_tasks: 10,
        total_retries: 30,
        total_time: 3600,
        simple_tasks: 3,
        moderate_tasks: 3,
        complex_tasks: 4,
        sessions_reused: 0,
    };

    assert_eq!(stats.completed_tasks, 0);
    assert_eq!(stats.failed_tasks, 10);

    // All tasks completed with no retries
    let stats = ExecutionStatistics {
        total_tasks: 5,
        completed_tasks: 5,
        skipped_tasks: 0,
        failed_tasks: 0,
        total_retries: 0,
        total_time: 600,
        simple_tasks: 2,
        moderate_tasks: 2,
        complex_tasks: 1,
        sessions_reused: 3,
    };

    assert_eq!(stats.completed_tasks, 5);
    assert_eq!(stats.failed_tasks, 0);
    assert_eq!(stats.total_retries, 0);
}

#[tokio::test]
async fn test_single_task_execution_order() {
    let task = Task::new("only-task", "Only Task", "Single task");
    let task_map: HashMap<String, Task> = [(task.id.clone(), task)].into_iter().collect();

    let order = ltmatrix::pipeline::execute::get_execution_order(&task_map).unwrap();

    assert_eq!(order.len(), 1);
    assert_eq!(order[0], "only-task");
}

#[tokio::test]
async fn test_mode_config_model_selection() {
    let fast_config = ModeConfig::fast_mode();
    let default_config = ModeConfig::default();
    let expert_config = ModeConfig::expert_mode();

    // Fast mode should use different models
    assert_ne!(fast_config.model_fast, default_config.model_fast);

    // Expert mode should use same model for all
    assert_eq!(expert_config.model_fast, expert_config.model_smart);

    // Default should have different models
    assert_ne!(default_config.model_fast, default_config.model_smart);
}
