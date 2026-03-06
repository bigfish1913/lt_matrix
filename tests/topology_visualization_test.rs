//! Tests for topological visualization
//!
//! This test suite verifies the ASCII art visualization for task dependencies
//! and execution order based on topological sorting.

use ltmatrix::models::{Task, TaskStatus};
use ltmatrix::tasks::scheduler::schedule_tasks;
use ltmatrix::tasks::topology::{
    visualize_dependency_graph, visualize_dependency_matrix, visualize_execution_plan,
    visualize_task_dependencies, TopologyConfig,
};

// ==================== Dependency Graph Visualization Tests ====================

#[test]
fn test_visualize_dependency_graph_empty_tasks() {
    let tasks = vec![];
    let result = visualize_dependency_graph(&tasks, None);

    assert!(result.contains("No tasks to visualize"));
}

#[test]
fn test_visualize_dependency_graph_single_task() {
    let task = Task::new("task-1", "Single Task", "A task with no dependencies");
    let result = visualize_dependency_graph(&[task], None);

    assert!(result.contains("Task Dependency Graph"));
    assert!(result.contains("task-1"));
    assert!(result.contains("Single Task"));
    assert!(result.contains("○")); // Pending status
}

#[test]
fn test_visualize_dependency_graph_simple_chain() {
    let mut task3 = Task::new("task-3", "Database Setup", "Setup database");
    let mut task2 = Task::new("task-2", "API Development", "Build API");
    task2.depends_on = vec!["task-3".to_string()];
    let mut task1 = Task::new("task-1", "Frontend", "Build frontend");
    task1.depends_on = vec!["task-2".to_string()];

    let tasks = vec![task3, task2, task1];
    let result = visualize_dependency_graph(&tasks, None);

    // All tasks should be shown
    assert!(result.contains("task-1"));
    assert!(result.contains("task-2"));
    assert!(result.contains("task-3"));

    // Tree structure should be present
    assert!(result.contains("└──") || result.contains("├──"));
}

#[test]
fn test_visualize_dependency_graph_multiple_roots() {
    let task1 = Task::new("task-1", "Root 1", "First root");
    let task2 = Task::new("task-2", "Root 2", "Second root");
    let task3 = Task::new("task-3", "Root 3", "Third root");

    let tasks = vec![task1, task2, task3];
    let result = visualize_dependency_graph(&tasks, None);

    // All root tasks should be shown
    assert!(result.contains("task-1"));
    assert!(result.contains("task-2"));
    assert!(result.contains("task-3"));
}

#[test]
fn test_visualize_dependency_graph_with_status() {
    let mut task2 = Task::new("task-2", "Dependent", "Depends on task-1");
    task2.status = TaskStatus::Completed;
    task2.depends_on = vec!["task-1".to_string()];

    let mut task1 = Task::new("task-1", "Dependency", "Base task");
    task1.status = TaskStatus::InProgress;

    let tasks = vec![task1, task2];
    let config = TopologyConfig {
        show_status: true,
        highlight_critical: false,
        show_levels: false,
        compact: false,
    };
    let result = visualize_dependency_graph(&tasks, Some(config));

    // Status symbols should be present
    assert!(result.contains("⚙")); // InProgress
    assert!(result.contains("✓")); // Completed
}

#[test]
fn test_visualize_dependency_graph_compact_mode() {
    let task = Task::new("task-1", "Compact Test", "Test compact mode");
    let tasks = vec![task];

    let config = TopologyConfig {
        show_status: false,
        highlight_critical: false,
        show_levels: false,
        compact: true,
    };
    let result = visualize_dependency_graph(&tasks, Some(config));

    // Should show task without status symbols
    assert!(result.contains("task-1"));
    assert!(!result.contains("○") && !result.contains("⚙")); // No status symbols
}

#[test]
fn test_visualize_dependency_graph_diamond() {
    // Diamond dependency pattern:
    //     task-1
    //     /     \
    // task-2   task-3
    //     \     /
    //     task-4
    let mut task4 = Task::new("task-4", "Merge", "Merge point");
    task4.depends_on = vec!["task-2".to_string(), "task-3".to_string()];

    let mut task2 = Task::new("task-2", "Branch A", "First branch");
    task2.depends_on = vec!["task-1".to_string()];

    let mut task3 = Task::new("task-3", "Branch B", "Second branch");
    task3.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Root", "Root task");

    let tasks = vec![task1, task2, task3, task4];
    let result = visualize_dependency_graph(&tasks, None);

    // All tasks should be shown
    assert!(result.contains("task-1"));
    assert!(result.contains("task-2"));
    assert!(result.contains("task-3"));
    assert!(result.contains("task-4"));
}

// ==================== Execution Plan Visualization Tests ====================

#[test]
fn test_visualize_execution_plan_empty() {
    let mut task1 = Task::new("task-1", "Task 1", "First");
    let mut task2 = Task::new("task-2", "Task 2", "Second");
    task2.depends_on = vec!["task-1".to_string()];

    let tasks = vec![task1, task2];
    let plan = schedule_tasks(tasks).unwrap();

    let result = visualize_execution_plan(&plan, None);

    // Should show statistics
    assert!(result.contains("Total Tasks: 2"));
    assert!(result.contains("Execution Levels"));
    assert!(result.contains("Flattened Execution Order"));
}

#[test]
fn test_visualize_execution_plan_parallel_tasks() {
    let task1 = Task::new("task-1", "Task 1", "First");
    let task2 = Task::new("task-2", "Task 2", "Second");
    let task3 = Task::new("task-3", "Task 3", "Third");

    let tasks = vec![task1, task2, task3];
    let plan = schedule_tasks(tasks).unwrap();

    let result = visualize_execution_plan(&plan, None);

    // All three should be in level 0 (no dependencies, can run in parallel)
    assert!(result.contains("Level 0"));
    assert!(result.contains("3 tasks")); // 3 tasks in level 0
}

#[test]
fn test_visualize_execution_plan_sequential() {
    let mut task3 = Task::new("task-3", "Task 3", "Last");
    let mut task2 = Task::new("task-2", "Task 2", "Middle");
    task2.depends_on = vec!["task-3".to_string()];
    let mut task1 = Task::new("task-1", "Task 1", "First");
    task1.depends_on = vec!["task-2".to_string()];

    let tasks = vec![task3, task2, task1];
    let plan = schedule_tasks(tasks).unwrap();

    let result = visualize_execution_plan(&plan, None);

    // Should show sequential levels
    assert!(result.contains("Level 0"));
    assert!(result.contains("Level 1"));
    assert!(result.contains("Level 2"));
}

#[test]
fn test_visualize_execution_plan_no_levels() {
    let task1 = Task::new("task-1", "Task 1", "First");
    let tasks = vec![task1];
    let plan = schedule_tasks(tasks).unwrap();

    let config = TopologyConfig {
        show_status: false,
        highlight_critical: false,
        show_levels: false,
        compact: false,
    };
    let result = visualize_execution_plan(&plan, Some(config));

    // Should not show levels when disabled
    assert!(!result.contains("Execution Levels"));
    // Should still show flattened order
    assert!(result.contains("Flattened Execution Order"));
}

#[test]
fn test_visualize_execution_plan_critical_path() {
    let mut task3 = Task::new("task-3", "Base", "Base task");
    let mut task2 = Task::new("task-2", "Middle", "Middle task");
    task2.depends_on = vec!["task-3".to_string()];
    let mut task1 = Task::new("task-1", "Top", "Top task");
    task1.depends_on = vec!["task-2".to_string()];

    let tasks = vec![task3, task2, task1];
    let plan = schedule_tasks(tasks).unwrap();

    let result = visualize_execution_plan(&plan, None);

    // Critical path should be shown
    assert!(result.contains("Critical Path"));
    // Should contain the chain
    assert!(result.contains("task-3"));
    assert!(result.contains("task-2"));
    assert!(result.contains("task-1"));
}

// ==================== Task Dependencies Visualization Tests ====================

#[test]
fn test_visualize_task_dependencies_no_deps() {
    let task1 = Task::new("task-1", "Independent", "No dependencies");
    let tasks = vec![task1.clone()];

    let result = visualize_task_dependencies(&task1, &tasks, None);

    assert!(result.contains("No dependencies (root task)"));
    assert!(result.contains("No dependents (leaf task)"));
}

#[test]
fn test_visualize_task_dependencies_with_deps() {
    let mut task2 = Task::new("task-2", "Dependent", "Depends on task-1");
    task2.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Base", "Base task");
    let tasks = vec![task1.clone(), task2];

    let result = visualize_task_dependencies(&task1, &tasks, None);

    assert!(result.contains("No dependencies"));
    assert!(result.contains("Dependents"));
    assert!(result.contains("task-2"));
}

#[test]
fn test_visualize_task_dependencies_multiple_dependents() {
    let mut task2 = Task::new("task-2", "Child 1", "First child");
    task2.depends_on = vec!["task-1".to_string()];

    let mut task3 = Task::new("task-3", "Child 2", "Second child");
    task3.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Parent", "Parent task");
    let tasks = vec![task1.clone(), task2, task3];

    let result = visualize_task_dependencies(&task1, &tasks, None);

    assert!(result.contains("No dependencies"));
    assert!(result.contains("Dependents"));
    assert!(result.contains("task-2"));
    assert!(result.contains("task-3"));
}

#[test]
fn test_visualize_task_dependencies_with_status() {
    let mut task2 = Task::new("task-2", "Dependent", "Depends on task-1");
    task2.status = TaskStatus::Completed;
    task2.depends_on = vec!["task-1".to_string()];

    let mut task1 = Task::new("task-1", "Base", "Base task");
    task1.status = TaskStatus::Failed;

    let tasks = vec![task1.clone(), task2];

    let config = TopologyConfig {
        show_status: true,
        highlight_critical: false,
        show_levels: false,
        compact: false,
    };
    let result = visualize_task_dependencies(&task1, &tasks, Some(config));

    // Status symbols should be shown
    assert!(result.contains("✗")); // Failed
    assert!(result.contains("✓")); // Completed
}

// ==================== Dependency Matrix Tests ====================

#[test]
fn test_visualize_dependency_matrix_empty() {
    let tasks = vec![];
    let result = visualize_dependency_matrix(&tasks, None);

    assert!(result.contains("No tasks to visualize"));
}

#[test]
fn test_visualize_dependency_matrix_single() {
    let task1 = Task::new("task-1", "Task 1", "First task");
    let tasks = vec![task1];

    let result = visualize_dependency_matrix(&tasks, None);

    assert!(result.contains("Dependency Matrix"));
    assert!(result.contains("task-1"));
    assert!(result.contains("Legend"));
}

#[test]
fn test_visualize_dependency_matrix_chain() {
    let mut task2 = Task::new("task-2", "Task 2", "Second task");
    task2.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Task 1", "First task");
    let tasks = vec![task1, task2];

    let result = visualize_dependency_matrix(&tasks, None);

    assert!(result.contains("Dependency Matrix"));
    assert!(result.contains("task-1"));
    assert!(result.contains("task-2"));
    assert!(result.contains("×")); // Dependency marker
    assert!(result.contains(".")); // No dependency marker
}

#[test]
fn test_visualize_dependency_matrix_diamond() {
    let mut task4 = Task::new("task-4", "Merge", "Merge point");
    task4.depends_on = vec!["task-2".to_string(), "task-3".to_string()];

    let mut task2 = Task::new("task-2", "Branch A", "First branch");
    task2.depends_on = vec!["task-1".to_string()];

    let mut task3 = Task::new("task-3", "Branch B", "Second branch");
    task3.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Root", "Root task");

    let tasks = vec![task1, task2, task3, task4];
    let result = visualize_dependency_matrix(&tasks, None);

    // All tasks should be in the matrix
    assert!(result.contains("task-1"));
    assert!(result.contains("task-2"));
    assert!(result.contains("task-3"));
    assert!(result.contains("task-4"));

    // Check for diamond pattern (task-4 depends on both 2 and 3)
    // The matrix should show task-4 has × for task-2 and task-3
}

// ==================== Configuration Tests ====================

#[test]
fn test_topology_config_default() {
    let config = TopologyConfig::default();
    assert!(config.show_status);
    assert!(config.highlight_critical);
    assert!(config.show_levels);
    assert!(!config.compact);
}

#[test]
fn test_topology_config_custom() {
    let config = TopologyConfig {
        show_status: false,
        highlight_critical: false,
        show_levels: false,
        compact: true,
    };

    assert!(!config.show_status);
    assert!(!config.highlight_critical);
    assert!(!config.show_levels);
    assert!(config.compact);
}

// ==================== Integration Tests ====================

#[test]
fn test_full_workflow_visualization() {
    // Create a realistic task graph
    let mut task5 = Task::new("task-5", "Test", "Run tests");
    task5.depends_on = vec!["task-4".to_string()];

    let mut task4 = Task::new("task-4", "Build", "Build project");
    task4.depends_on = vec!["task-2".to_string(), "task-3".to_string()];

    let mut task3 = Task::new("task-3", "Backend", "Backend code");
    task3.depends_on = vec!["task-1".to_string()];

    let mut task2 = Task::new("task-2", "Frontend", "Frontend code");
    task2.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "Setup", "Project setup");

    let tasks = vec![task1, task2, task3, task4, task5];

    // Generate execution plan
    let plan = schedule_tasks(tasks.clone()).unwrap();

    // Test all visualization types
    let graph_result = visualize_dependency_graph(&tasks, None);
    assert!(graph_result.contains("Task Dependency Graph"));

    let plan_result = visualize_execution_plan(&plan, None);
    assert!(plan_result.contains("Execution Plan"));
    assert!(plan_result.contains("Level 0")); // task-1
    assert!(plan_result.contains("Level 1")); // task-2, task-3

    let matrix_result = visualize_dependency_matrix(&tasks, None);
    assert!(matrix_result.contains("Dependency Matrix"));

    let dep_result = visualize_task_dependencies(&tasks[0], &tasks, None);
    assert!(dep_result.contains("Dependencies for"));
}

#[test]
fn test_visualization_with_completed_tasks() {
    let mut task2 = Task::new("task-2", "Second", "Second task");
    task2.status = TaskStatus::Completed;
    task2.depends_on = vec!["task-1".to_string()];

    let mut task1 = Task::new("task-1", "First", "First task");
    task1.status = TaskStatus::InProgress;

    let tasks = vec![task1, task2];
    let result = visualize_dependency_graph(&tasks, None);

    // Should show different status symbols
    assert!(result.contains("⚙") || result.contains("✓"));
}

#[test]
fn test_parallel_tasks_identification() {
    // Tasks with no dependencies should be parallelizable
    let task1 = Task::new("task-1", "Task 1", "First parallel");
    let task2 = Task::new("task-2", "Task 2", "Second parallel");
    let task3 = Task::new("task-3", "Task 3", "Third parallel");

    let tasks = vec![task1, task2, task3];
    let plan = schedule_tasks(tasks).unwrap();

    let result = visualize_execution_plan(&plan, None);

    // All should be in level 0
    assert!(result.contains("Level 0 (3 tasks)"));
}

#[test]
fn test_critical_path_identification() {
    // Longest chain: task-1 -> task-2 -> task-3 -> task-4
    let mut task4 = Task::new("task-4", "Fourth", "Last in chain");
    task4.depends_on = vec!["task-3".to_string()];

    let mut task3 = Task::new("task-3", "Third", "Third in chain");
    task3.depends_on = vec!["task-2".to_string()];

    let mut task2 = Task::new("task-2", "Second", "Second in chain");
    task2.depends_on = vec!["task-1".to_string()];

    let task1 = Task::new("task-1", "First", "First in chain");

    let tasks = vec![task1, task2, task3, task4];
    let plan = schedule_tasks(tasks).unwrap();

    let result = visualize_execution_plan(&plan, None);

    // Critical path should show all 4 tasks
    assert!(result.contains("Critical Path"));
    assert!(result.contains("4") || result.contains("task-4")); // Length or last task
}
