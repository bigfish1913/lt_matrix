//! Edge case and stress tests for the Generate stage
//!
//! These tests verify robustness under unusual inputs, boundary conditions,
//! and edge cases that might occur in production.
//!
//! Note: Many low-level edge cases are tested in the module's internal tests.
//! These tests focus on edge cases accessible through the public API.

use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::pipeline::generate::{
    calculate_generation_stats, GenerateConfig, GenerationResult, ValidationError,
};

/// Edge case: Very long task ID
#[test]
fn test_edge_case_very_long_task_id() {
    let long_id = "task-".repeat(100);

    let task = Task::new(&long_id, "Task with long ID", "Description");

    assert_eq!(task.id, long_id);
}

/// Edge case: Very long task title
#[test]
fn test_edge_case_very_long_task_title() {
    let long_title = "A".repeat(10000);

    let task = Task::new("task-1", &long_title, "Description");

    assert_eq!(task.title, long_title);
}

/// Edge case: Very long task description
#[test]
fn test_edge_case_very_long_task_description() {
    let long_description = "This is a very long description. ".repeat(1000);

    let task = Task::new("task-1", "Task", &long_description);

    assert_eq!(task.description, long_description);
}

/// Edge case: Special characters in task fields
#[test]
fn test_edge_case_special_characters_in_fields() {
    let task_with_special = Task::new(
        "task-with-special-<>&\"'-chars",
        "Task with emojis: 🚀 🎯 ⚡",
        "Description with special chars: <>&\"' and unicode: 你好世界",
    );

    assert!(!task_with_special.id.is_empty());
    assert!(!task_with_special.title.is_empty());
    assert!(!task_with_special.description.is_empty());
}

/// Edge case: Tasks with many dependencies
#[test]
fn test_edge_case_many_dependencies() {
    let mut deps = vec![];
    for i in 1..=50 {
        deps.push(format!("task-{}", i));
    }

    let mut task = Task::new("final", "Final Task", "Depends on many tasks");
    task.depends_on = deps;

    assert_eq!(task.depends_on.len(), 50);
}

/// Edge case: Deep dependency chain
#[test]
fn test_edge_case_deep_dependency_chain() {
    let mut tasks = vec![];

    // Create a chain of 100 tasks
    for i in 1..=100 {
        let mut t = Task::new(
            &format!("task-{}", i),
            &format!("Task {}", i),
            "Description",
        );

        if i > 1 {
            t.depends_on = vec![format!("task-{}", i - 1)];
        }

        tasks.push(t);
    }

    // Verify the chain is correctly structured
    assert_eq!(tasks.len(), 100);
    assert!(tasks[0].depends_on.is_empty());
    assert_eq!(tasks[99].depends_on.len(), 1);
}

/// Edge case: Wide dependency graph (many independent tasks)
#[test]
fn test_edge_case_wide_dependency_graph() {
    let tasks: Vec<Task> = (1..=1000)
        .map(|i| {
            let mut t = Task::new(
                &format!("task-{}", i),
                &format!("Task {}", i),
                "Description",
            );
            t.depends_on = vec![];
            t
        })
        .collect();

    assert_eq!(tasks.len(), 1000);
    assert!(tasks.iter().all(|t| t.depends_on.is_empty()));
}

/// Edge case: Diamond dependency pattern
#[test]
fn test_edge_case_diamond_dependencies() {
    let tasks = vec![
        {
            let mut t = Task::new("base", "Base", "Foundation");
            t.depends_on = vec![];
            t
        },
        {
            let mut t = Task::new("left", "Left branch", "Branch A");
            t.depends_on = vec!["base".to_string()];
            t
        },
        {
            let mut t = Task::new("right", "Right branch", "Branch B");
            t.depends_on = vec!["base".to_string()];
            t
        },
        {
            let mut t = Task::new("final", "Final", "Merge point");
            t.depends_on = vec!["left".to_string(), "right".to_string()];
            t
        },
    ];

    // Verify diamond structure
    assert_eq!(tasks[0].depends_on.len(), 0); // base
    assert_eq!(tasks[1].depends_on, vec!["base"]); // left
    assert_eq!(tasks[2].depends_on, vec!["base"]); // right
    assert_eq!(tasks[3].depends_on.len(), 2); // final
}

/// Edge case: Single character task IDs
#[test]
fn test_edge_case_single_character_ids() {
    let task_a = Task::new("a", "Task A", "Single char ID");
    let mut task_b = Task::new("b", "Task B", "Also single char");
    task_b.depends_on = vec!["a".to_string()];

    assert_eq!(task_a.id, "a");
    assert_eq!(task_b.id, "b");
    assert_eq!(task_b.depends_on[0], "a");
}

/// Edge case: Numeric task IDs
#[test]
fn test_edge_case_numeric_task_ids() {
    let task_1 = Task::new("1", "Task 1", "Numeric ID");
    let mut task_2 = Task::new("2", "Task 2", "Also numeric");
    task_2.depends_on = vec!["1".to_string()];

    assert_eq!(task_1.id, "1");
    assert_eq!(task_2.id, "2");
    assert_eq!(task_2.depends_on[0], "1");
}

/// Edge case: Self-dependency (circular with single task)
#[test]
fn test_edge_case_self_dependency_validation_error() {
    // Self-dependency would be caught by validation
    let error = ValidationError::CircularDependency {
        chain: vec!["task-1".to_string(), "task-1".to_string()],
    };

    let message = format!("{}", error);
    assert!(message.contains("Circular"));
}

/// Edge case: Very large validation errors list
#[test]
fn test_edge_case_many_validation_errors() {
    let mut validation_errors = vec![];

    // Create many duplicate ID errors
    for i in 1..=1000 {
        validation_errors.push(ValidationError::DuplicateTaskId {
            id: format!("task-{}", i),
        });
    }

    // Should handle large error list without panicking
    assert_eq!(validation_errors.len(), 1000);

    for error in &validation_errors {
        let _ = format!("{}", error); // Should be able to format all
    }
}

/// Edge case: Task with empty depends_on array
#[test]
fn test_edge_case_empty_depends_on_array() {
    let mut task = Task::new("task-1", "Task", "With empty array");
    task.depends_on = vec![];

    assert!(task.depends_on.is_empty());
}

/// Edge case: Task without depends_on set
#[test]
fn test_edge_case_no_depends_on_field() {
    let task = Task::new("task-1", "Task", "Without depends_on");

    // Default depends_on is empty
    assert!(task.depends_on.is_empty());
}

/// Edge case: Unicode in task IDs
#[test]
fn test_edge_case_unicode_task_ids() {
    let task_chinese = Task::new("任务-1", "Chinese Task", "Task with Chinese ID");
    let task_emoji = Task::new("🚀-task", "Emoji Task", "Task with emoji in ID");

    assert_eq!(task_chinese.id, "任务-1");
    assert_eq!(task_emoji.id, "🚀-task");
}

/// Edge case: Maximum configuration values
#[test]
fn test_edge_case_max_configuration_values() {
    let config = GenerateConfig {
        generation_model: "x".repeat(10000),
        timeout: u64::MAX,
        max_retries: u32::MAX,
        max_tasks: usize::MAX,
        enable_validation: true,
        execution_mode: ltmatrix::pipeline::generate::ExecutionMode::Expert,
    };

    // Should not panic with extreme values
    assert_eq!(config.timeout, u64::MAX);
    assert_eq!(config.max_retries, u32::MAX);
    assert_eq!(config.max_tasks, usize::MAX);
}

/// Edge case: Zero configuration values
#[test]
fn test_edge_case_zero_configuration_values() {
    let config = GenerateConfig {
        generation_model: String::new(),
        timeout: 0,
        max_retries: 0,
        max_tasks: 0,
        enable_validation: false,
        execution_mode: ltmatrix::pipeline::generate::ExecutionMode::Fast,
    };

    // Should not panic with zero values
    assert_eq!(config.timeout, 0);
    assert_eq!(config.max_retries, 0);
    assert_eq!(config.max_tasks, 0);
}

/// Edge case: Complex validation error with multiple circular dependencies
#[test]
fn test_edge_case_multiple_circular_dependencies() {
    let errors = vec![
        ValidationError::CircularDependency {
            chain: vec!["a".to_string(), "b".to_string(), "a".to_string()],
        },
        ValidationError::CircularDependency {
            chain: vec!["c".to_string(), "d".to_string(), "c".to_string()],
        },
    ];

    assert_eq!(errors.len(), 2);

    for error in &errors {
        let message = format!("{}", error);
        assert!(message.contains("Circular"));
    }
}

/// Edge case: Task ID that's just whitespace
#[test]
fn test_edge_case_whitespace_task_id() {
    let task = Task::new("   ", "Task", "Whitespace ID");

    assert_eq!(task.id, "   ");
}

/// Edge case: Mix of different complexity values
#[test]
fn test_edge_case_all_complexity_combinations() {
    let tasks: Vec<Task> = ["Simple", "Moderate", "Complex"]
        .iter()
        .enumerate()
        .map(|(i, complexity)| {
            let mut t = Task::new(&format!("task-{}", i), "Task", "Description");
            t.complexity = match *complexity {
                "Simple" => TaskComplexity::Simple,
                "Moderate" => TaskComplexity::Moderate,
                "Complex" => TaskComplexity::Complex,
                _ => TaskComplexity::Moderate,
            };
            t.depends_on = vec![];
            t
        })
        .collect();

    let result = GenerationResult {
        tasks,
        task_count: 3,
        dependency_depth: 0,
        validation_errors: vec![],
        generation_log: None,
    };

    let stats = calculate_generation_stats(&result);

    assert_eq!(stats.simple, 1);
    assert_eq!(stats.moderate, 1);
    assert_eq!(stats.complex, 1);
}

/// Edge case: Statistics with empty result
#[test]
fn test_edge_case_statistics_with_empty_result() {
    let result = GenerationResult {
        tasks: vec![],
        task_count: 0,
        dependency_depth: 0,
        validation_errors: vec![],
        generation_log: None,
    };

    let stats = calculate_generation_stats(&result);

    assert_eq!(stats.total_tasks, 0);
    assert_eq!(stats.simple, 0);
    assert_eq!(stats.moderate, 0);
    assert_eq!(stats.complex, 0);
    assert_eq!(stats.tasks_with_dependencies, 0);
    assert_eq!(stats.total_dependencies, 0);
    assert_eq!(stats.dependency_depth, 0);
    assert_eq!(stats.validation_errors, 0);

    // Should be able to display without panicking
    let _display = format!("{}", stats);
}

/// Edge case: Duplicate dependencies
#[test]
fn test_edge_case_duplicate_dependencies() {
    let mut task = Task::new("task-1", "Task", "Duplicate dependencies");
    task.depends_on = vec![
        "dep-1".to_string(),
        "dep-1".to_string(),
        "dep-1".to_string(),
    ];

    // All three should be present (validation happens separately)
    assert_eq!(task.depends_on.len(), 3);
}

/// Edge case: Empty task ID
#[test]
fn test_edge_case_empty_task_id() {
    let task = Task::new("", "Task", "Empty ID");

    assert_eq!(task.id, "");
}

/// Edge case: Empty task title
#[test]
fn test_edge_case_empty_task_title() {
    let task = Task::new("task-1", "", "Empty title");

    assert_eq!(task.title, "");
}

/// Edge case: Empty task description
#[test]
fn test_edge_case_empty_task_description() {
    let task = Task::new("task-1", "Task", "");

    assert_eq!(task.description, "");
}

/// Stress test: Maximum task count limit
#[test]
fn test_stress_max_task_limit() {
    let config = GenerateConfig {
        max_tasks: 1,
        ..Default::default()
    };

    // Even if more tasks are generated, should respect limit
    // (This is a unit test of the config, actual truncation happens in generate_tasks)
    assert_eq!(config.max_tasks, 1);
}

/// Edge case: All validation error types can be created
#[test]
fn test_edge_case_all_validation_error_types() {
    let errors = vec![
        ValidationError::MissingDependency {
            task: "t1".to_string(),
            dependency: "missing".to_string(),
        },
        ValidationError::CircularDependency {
            chain: vec!["a".to_string(), "b".to_string()],
        },
        ValidationError::DuplicateTaskId {
            id: "dup".to_string(),
        },
        ValidationError::InvalidStructure {
            task: "bad".to_string(),
            reason: "empty".to_string(),
        },
    ];

    assert_eq!(errors.len(), 4);

    // Verify all can be formatted
    for error in &errors {
        let _message = format!("{}", error);
    }
}

/// Edge case: Result with all tasks having dependencies
#[test]
fn test_edge_case_all_tasks_with_dependencies() {
    let tasks = vec![
        {
            let mut t = Task::new("task-1", "Task 1", "First");
            t.depends_on = vec![];
            t
        },
        {
            let mut t = Task::new("task-2", "Task 2", "Second");
            t.depends_on = vec!["task-1".to_string()];
            t
        },
        {
            let mut t = Task::new("task-3", "Task 3", "Third");
            t.depends_on = vec!["task-2".to_string()];
            t
        },
    ];

    let result = GenerationResult {
        tasks,
        task_count: 3,
        dependency_depth: 2,
        validation_errors: vec![],
        generation_log: None,
    };

    let stats = calculate_generation_stats(&result);
    assert_eq!(stats.tasks_with_dependencies, 2);
    assert_eq!(stats.total_dependencies, 2);
}

/// Edge case: Very deep dependency chain in statistics
#[test]
fn test_edge_case_deep_dependency_in_statistics() {
    let tasks: Vec<Task> = (1..=100)
        .map(|i| {
            let mut t = Task::new(&format!("task-{}", i), "Task", "Description");
            if i > 1 {
                t.depends_on = vec![format!("task-{}", i - 1)];
            }
            t
        })
        .collect();

    let result = GenerationResult {
        tasks,
        task_count: 100,
        dependency_depth: 99,
        validation_errors: vec![],
        generation_log: None,
    };

    let stats = calculate_generation_stats(&result);
    assert_eq!(stats.dependency_depth, 99);
    assert_eq!(stats.tasks_with_dependencies, 99);
    assert_eq!(stats.total_dependencies, 99);
}

/// Edge case: Configuration with extreme timeout values
#[test]
fn test_edge_case_extreme_timeout_values() {
    let config_zero = GenerateConfig {
        timeout: 0,
        ..Default::default()
    };

    let config_max = GenerateConfig {
        timeout: u64::MAX,
        ..Default::default()
    };

    assert_eq!(config_zero.timeout, 0);
    assert_eq!(config_max.timeout, u64::MAX);
}

/// Edge case: All complexity types represented in result
#[test]
fn test_edge_case_all_complexity_types_in_result() {
    let tasks = vec![
        {
            let mut t = Task::new("simple", "Simple", "Simple task");
            t.complexity = TaskComplexity::Simple;
            t
        },
        {
            let mut t = Task::new("moderate", "Moderate", "Moderate task");
            t.complexity = TaskComplexity::Moderate;
            t
        },
        {
            let mut t = Task::new("complex", "Complex", "Complex task");
            t.complexity = TaskComplexity::Complex;
            t
        },
    ];

    let result = GenerationResult {
        tasks,
        task_count: 3,
        dependency_depth: 0,
        validation_errors: vec![],
        generation_log: None,
    };

    let stats = calculate_generation_stats(&result);

    assert_eq!(stats.simple, 1);
    assert_eq!(stats.moderate, 1);
    assert_eq!(stats.complex, 1);
}

/// Edge case: Validation errors with all error types
#[test]
fn test_edge_case_all_validation_errors_in_result() {
    let result = GenerationResult {
        tasks: vec![],
        task_count: 0,
        dependency_depth: 0,
        validation_errors: vec![
            ValidationError::MissingDependency {
                task: "task-1".to_string(),
                dependency: "missing".to_string(),
            },
            ValidationError::CircularDependency {
                chain: vec!["a".to_string(), "b".to_string()],
            },
            ValidationError::DuplicateTaskId {
                id: "dup".to_string(),
            },
            ValidationError::InvalidStructure {
                task: "bad".to_string(),
                reason: "empty".to_string(),
            },
        ],
        generation_log: None,
    };

    assert_eq!(result.validation_errors.len(), 4);
}

/// Edge case: Task status variations
#[test]
fn test_edge_case_task_status_variations() {
    let mut pending = Task::new("pending", "Pending", "Pending task");
    pending.status = TaskStatus::Pending;

    let mut in_progress = Task::new("in-progress", "In Progress", "In progress task");
    in_progress.status = TaskStatus::InProgress;

    let mut completed = Task::new("completed", "Completed", "Completed task");
    completed.status = TaskStatus::Completed;

    assert!(matches!(pending.status, TaskStatus::Pending));
    assert!(matches!(in_progress.status, TaskStatus::InProgress));
    assert!(matches!(completed.status, TaskStatus::Completed));
}

/// Edge case: Very long validation error message
#[test]
fn test_edge_case_very_long_validation_error_message() {
    let long_reason = "r".repeat(10000);

    let error = ValidationError::InvalidStructure {
        task: "task-1".to_string(),
        reason: long_reason,
    };

    let message = format!("{}", error);
    assert!(message.len() > 10000);
}
