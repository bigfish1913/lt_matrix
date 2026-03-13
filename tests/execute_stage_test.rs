//! Tests for the execute stage

use ltmatrix::models::{Task, TaskComplexity};
use ltmatrix::pipeline::execute::{build_execution_prompt, build_task_context, ExecuteConfig};
use std::collections::{HashMap, HashSet};

#[test]
fn test_build_execution_prompt_structure() {
    let task = Task::new("task-1", "Implement feature X", "Add feature X with tests");
    let context = "Project: Web App\nStack: Rust";

    let prompt = build_execution_prompt(&task, context);

    assert!(prompt.contains("Implement feature X"));
    assert!(prompt.contains("Add feature X with tests"));
    assert!(prompt.contains("Project: Web App"));
    assert!(prompt.contains("Stack: Rust"));
    assert!(prompt.contains("Begin your implementation now"));
}

#[test]
fn test_build_task_context_with_memory() {
    let task = Task::new("task-1", "Test Task", "Implementation task");
    let task_map: HashMap<String, Task> = [(task.id.clone(), task.clone())].into_iter().collect();
    let completed_tasks: HashSet<String> = HashSet::new();
    let project_memory = "# Architecture\nWe use MVC pattern";

    let context = build_task_context(&task, &task_map, &completed_tasks, project_memory).unwrap();

    assert!(context.contains("Project Memory"));
    assert!(context.contains("Architecture"));
    assert!(context.contains("MVC pattern"));
    assert!(context.contains("Task: Test Task"));
    assert!(context.contains("Complexity:"));
}

#[test]
fn test_build_task_context_with_dependencies() {
    let task1 = Task::new("task-1", "Setup", "Initial setup");
    let mut task2 = Task::new("task-2", "Feature", "Main feature");
    task2.depends_on = vec!["task-1".to_string()];

    let task_map: HashMap<String, Task> = [
        (task1.id.clone(), task1.clone()),
        (task2.id.clone(), task2.clone()),
    ]
    .into_iter()
    .collect();

    let completed_tasks: HashSet<String> = ["task-1".to_string()].into_iter().collect();
    let project_memory = "";

    let context = build_task_context(&task2, &task_map, &completed_tasks, project_memory).unwrap();

    assert!(context.contains("Dependencies"));
    assert!(context.contains("- Setup (completed)"));
}

#[test]
fn test_execute_config_defaults() {
    let config = ExecuteConfig::default();

    assert_eq!(config.max_retries, 3);
    assert_eq!(config.timeout, 3600);
    assert!(config.enable_sessions);
    assert_eq!(
        config.memory_file,
        std::path::PathBuf::from(".claude/memory.md")
    );
}

#[test]
fn test_execute_config_fast_mode() {
    let config = ExecuteConfig::fast_mode();

    assert_eq!(config.max_retries, 1);
    assert_eq!(config.timeout, 1800);
    assert!(config.enable_sessions);
}

#[test]
fn test_execute_config_expert_mode() {
    let config = ExecuteConfig::expert_mode();

    assert_eq!(config.max_retries, 3);
    assert_eq!(config.timeout, 7200);
    assert!(config.enable_sessions);
}

#[test]
fn test_execution_statistics_initialization() {
    use ltmatrix::pipeline::execute::ExecutionStatistics;

    let stats = ExecutionStatistics {
        total_tasks: 10,
        completed_tasks: 8,
        failed_tasks: 1,
        skipped_tasks: 1,
        total_retries: 3,
        total_time: 300,
        simple_tasks: 4,
        moderate_tasks: 4,
        complex_tasks: 2,
        sessions_reused: 5,
    };

    assert_eq!(stats.total_tasks, 10);
    assert_eq!(stats.completed_tasks, 8);
    assert_eq!(stats.failed_tasks, 1);
    assert_eq!(stats.total_retries, 3);
    assert_eq!(stats.total_time, 300);
}

#[test]
fn test_task_complexity_preservation() {
    let mut simple_task = Task::new("simple-1", "Simple Task", "Easy implementation");
    let mut moderate_task = Task::new("moderate-1", "Moderate Task", "Medium complexity");
    let mut complex_task = Task::new("complex-1", "Complex Task", "Hard implementation");

    simple_task.complexity = TaskComplexity::Simple;
    moderate_task.complexity = TaskComplexity::Moderate;
    complex_task.complexity = TaskComplexity::Complex;

    let task_map: HashMap<String, Task> = [
        (simple_task.id.clone(), simple_task),
        (moderate_task.id.clone(), moderate_task),
        (complex_task.id.clone(), complex_task),
    ]
    .into_iter()
    .collect();

    // Test that complexity is preserved in the task map
    let retrieved_simple = task_map.get("simple-1").unwrap();
    assert_eq!(retrieved_simple.complexity, TaskComplexity::Simple);

    let retrieved_moderate = task_map.get("moderate-1").unwrap();
    assert_eq!(retrieved_moderate.complexity, TaskComplexity::Moderate);

    let retrieved_complex = task_map.get("complex-1").unwrap();
    assert_eq!(retrieved_complex.complexity, TaskComplexity::Complex);
}
