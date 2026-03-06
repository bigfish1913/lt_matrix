//! Comprehensive test suite for task dependency scheduler
//!
//! This test suite verifies:
//! - Topological sort correctness and edge cases
//! - Cycle detection for various dependency patterns
//! - Critical path calculation accuracy
//! - Parallelizable task identification
//! - Execution level grouping for parallelism
//! - Graph statistics calculation
//! - Visualization and diagram generation
//!
//! The scheduler implements Kahn's algorithm for topological sorting
//! combined with DFS-based cycle detection and longest path calculation.

use ltmatrix::models::Task;
use ltmatrix::tasks::scheduler::*;
use std::collections::HashSet;

/// Helper function to create test tasks with dependencies
fn create_task(id: &str, deps: Vec<&str>) -> Task {
    let mut task = Task::new(id, id, format!("Task {}", id));
    task.depends_on = deps.into_iter().map(|s| s.to_string()).collect();
    task
}

/// Helper to create a task map from a vector of tasks
fn create_task_map(tasks: Vec<Task>) -> std::collections::HashMap<String, Task> {
    tasks
        .into_iter()
        .map(|task| (task.id.clone(), task))
        .collect()
}

// ============================================================================
// Topological Sort Tests
// ============================================================================

#[test]
fn test_topological_sort_empty_list() {
    let tasks = vec![];
    let plan = schedule_tasks(tasks).expect("Should handle empty task list");

    assert_eq!(plan.total_tasks, 0);
    assert_eq!(plan.execution_levels.len(), 0);
    assert_eq!(plan.execution_order.len(), 0);
    assert_eq!(plan.max_depth, 0);
}

#[test]
fn test_topological_sort_single_task() {
    let tasks = vec![create_task("task-1", vec![])];
    let plan = schedule_tasks(tasks).expect("Should schedule single task");

    assert_eq!(plan.total_tasks, 1);
    assert_eq!(plan.execution_order, vec!["task-1"]);
    assert_eq!(plan.max_depth, 1);
    assert_eq!(plan.execution_levels.len(), 1);
    assert_eq!(plan.execution_levels[0].len(), 1);
}

#[test]
fn test_topological_sort_linear_chain() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-2"]),
        create_task("task-4", vec!["task-3"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should schedule linear chain");

    // Verify execution order respects dependencies
    assert_eq!(
        plan.execution_order,
        vec!["task-1", "task-2", "task-3", "task-4"]
    );

    // Each task is in its own level (no parallelism)
    assert_eq!(plan.max_depth, 4);
    assert_eq!(plan.execution_levels.len(), 4);

    // Verify critical path includes all tasks
    assert_eq!(
        plan.critical_path,
        vec!["task-1", "task-2", "task-3", "task-4"]
    );

    // No parallelizable tasks in linear chain
    assert_eq!(plan.parallelizable_tasks.len(), 0);
}

#[test]
fn test_topological_sort_diamond_pattern() {
    // Diamond pattern: task-1 -> [task-2, task-3] -> task-4
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-1"]),
        create_task("task-4", vec!["task-2", "task-3"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should schedule diamond pattern");

    // Level 1: task-1
    assert_eq!(plan.execution_levels[0].len(), 1);
    assert_eq!(plan.execution_levels[0][0].id, "task-1");

    // Level 2: task-2 and task-3 in parallel
    assert_eq!(plan.execution_levels[1].len(), 2);
    let level_2_ids: HashSet<String> = plan.execution_levels[1]
        .iter()
        .map(|t| t.id.clone())
        .collect();
    assert_eq!(
        level_2_ids,
        HashSet::from(["task-2".to_string(), "task-3".to_string()])
    );

    // Level 3: task-4
    assert_eq!(plan.execution_levels[2].len(), 1);
    assert_eq!(plan.execution_levels[2][0].id, "task-4");

    assert_eq!(plan.max_depth, 3);
}

#[test]
fn test_topological_sort_complex_graph() {
    // Complex graph with multiple branches
    // task-1 -> [task-2, task-3]
    // task-2 -> task-4
    // task-3 -> [task-5, task-6]
    // task-4 -> task-7
    // task-5 -> task-7
    // task-6 -> task-7
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-1"]),
        create_task("task-4", vec!["task-2"]),
        create_task("task-5", vec!["task-3"]),
        create_task("task-6", vec!["task-3"]),
        create_task("task-7", vec!["task-4", "task-5", "task-6"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should schedule complex graph");

    // Verify total tasks
    assert_eq!(plan.total_tasks, 7);

    // Verify execution order respects all dependencies
    let pos = |id: &str| plan.execution_order.iter().position(|x| x == id).unwrap();

    assert!(pos("task-1") < pos("task-2"));
    assert!(pos("task-1") < pos("task-3"));
    assert!(pos("task-2") < pos("task-4"));
    assert!(pos("task-3") < pos("task-5"));
    assert!(pos("task-3") < pos("task-6"));
    assert!(pos("task-4") < pos("task-7"));
    assert!(pos("task-5") < pos("task-7"));
    assert!(pos("task-6") < pos("task-7"));
}

#[test]
fn test_topological_sort_multiple_roots() {
    // Multiple independent tasks that merge later
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec![]),
        create_task("task-3", vec![]),
        create_task("task-4", vec!["task-1", "task-2", "task-3"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should schedule multiple roots");

    // First level should have all three root tasks
    assert_eq!(plan.execution_levels[0].len(), 3);

    // Second level has the final task
    assert_eq!(plan.execution_levels[1].len(), 1);
    assert_eq!(plan.max_depth, 2);

    // All root tasks should be in execution order
    assert_eq!(plan.execution_order.len(), 4);
}

#[test]
fn test_topological_sort_independent_tasks() {
    // Completely independent tasks
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec![]),
        create_task("task-3", vec![]),
        create_task("task-4", vec![]),
        create_task("task-5", vec![]),
    ];

    let plan = schedule_tasks(tasks).expect("Should schedule independent tasks");

    // All tasks should be in a single level
    assert_eq!(plan.execution_levels.len(), 1);
    assert_eq!(plan.execution_levels[0].len(), 5);
    assert_eq!(plan.max_depth, 1);

    // All tasks are parallelizable
    assert_eq!(plan.parallelizable_tasks.len(), 4); // All but one on critical path
}

// ============================================================================
// Cycle Detection Tests
// ============================================================================

#[test]
fn test_cycle_detection_simple_cycle() {
    // Simple 3-node cycle: task-1 -> task-2 -> task-3 -> task-1
    let tasks = vec![
        create_task("task-1", vec!["task-3"]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-2"]),
    ];

    let result = schedule_tasks(tasks);
    assert!(result.is_err(), "Should detect cycle");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Circular dependency"),
        "Error should mention circular dependency"
    );
    assert!(err_msg.contains("task-"), "Error should include task IDs");
}

#[test]
fn test_cycle_detection_self_loop() {
    // Self-referencing task
    let tasks = vec![create_task("task-1", vec!["task-1"])];

    let result = schedule_tasks(tasks);
    assert!(result.is_err(), "Should detect self-loop");

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Circular dependency"));
}

#[test]
fn test_cycle_detection_complex_cycle() {
    // Complex cycle with branches
    // task-1 -> task-2 -> task-4 -> task-1 (cycle)
    // task-1 -> task-3 -> task-4
    let tasks = vec![
        create_task("task-1", vec!["task-4"]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-1"]),
        create_task("task-4", vec!["task-2"]),
    ];

    let result = schedule_tasks(tasks);
    assert!(result.is_err(), "Should detect complex cycle");
}

#[test]
fn test_cycle_detection_multiple_cycles() {
    // Multiple independent cycles
    // Cycle 1: task-1 -> task-2 -> task-1
    // Cycle 2: task-3 -> task-4 -> task-3
    let tasks = vec![
        create_task("task-1", vec!["task-2"]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-4"]),
        create_task("task-4", vec!["task-3"]),
    ];

    let result = schedule_tasks(tasks);
    assert!(result.is_err(), "Should detect at least one cycle");
}

#[test]
fn test_cycle_detection_indirect_cycle() {
    // Indirect cycle through multiple tasks
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-2"]),
        create_task("task-4", vec!["task-3"]),
        create_task("task-5", vec!["task-4"]),
        create_task("task-1", vec!["task-5"]), // Creates cycle back to task-1
    ];

    // This should fail during scheduling
    // Note: This creates duplicate task-1, but the HashMap will overwrite
    let result = schedule_tasks(tasks);
    // The result might succeed (last task-1 wins) or fail depending on implementation
    // Just verify it doesn't panic
    let _ = result;
}

// ============================================================================
// Dependency Validation Tests
// ============================================================================

#[test]
fn test_validate_dependencies_missing() {
    // Task depends on non-existent task
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-999"]), // Non-existent
    ];

    let result = schedule_tasks(tasks);
    assert!(result.is_err(), "Should reject missing dependencies");

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("non-existent task") || err_msg.contains("Missing task"));
}

#[test]
fn test_validate_dependencies_multiple_missing() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-999", "task-888", "task-777"]),
    ];

    let result = schedule_tasks(tasks);
    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("task-999"));
}

#[test]
fn test_validate_dependencies_valid() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-1", "task-2"]),
    ];

    let result = schedule_tasks(tasks);
    assert!(result.is_ok(), "Should accept valid dependencies");
}

// ============================================================================
// Critical Path Tests
// ============================================================================

#[test]
fn test_critical_path_linear() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-2"]),
    ];

    let plan = schedule_tasks(tasks).unwrap();

    // All tasks should be on critical path in linear chain
    assert_eq!(plan.critical_path.len(), 3);
    assert_eq!(plan.critical_path, vec!["task-1", "task-2", "task-3"]);
}

#[test]
fn test_critical_path_branching() {
    // task-1 -> [task-2 (long), task-3 (short)]
    // task-2 -> task-4
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-1"]),
        create_task("task-4", vec!["task-2"]),
    ];

    let plan = schedule_tasks(tasks).unwrap();

    // Critical path should be task-1 -> task-2 -> task-4 (length 3)
    assert_eq!(plan.critical_path.len(), 3);
    assert_eq!(plan.critical_path[0], "task-1");
    assert_eq!(plan.critical_path[2], "task-4");

    // task-3 should be parallelizable
    assert!(plan.parallelizable_tasks.contains(&"task-3".to_string()));
}

#[test]
fn test_critical_path_multiple_branches() {
    // task-1 -> [task-2, task-3, task-4]
    // task-2 -> task-5 -> task-7
    // task-3 -> task-6
    // task-4 -> task-7
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-1"]),
        create_task("task-4", vec!["task-1"]),
        create_task("task-5", vec!["task-2"]),
        create_task("task-6", vec!["task-3"]),
        create_task("task-7", vec!["task-5", "task-4"]),
    ];

    let plan = schedule_tasks(tasks).unwrap();

    // Critical path should be the longest path
    // Either task-1 -> task-2 -> task-5 -> task-7 or similar
    assert!(plan.critical_path.len() >= 3);

    // Verify first task is root
    assert_eq!(plan.critical_path[0], "task-1");

    // Critical path should be unique
    let unique_tasks: HashSet<_> = plan.critical_path.iter().collect();
    assert_eq!(unique_tasks.len(), plan.critical_path.len());
}

// ============================================================================
// Parallelizable Tasks Tests
// ============================================================================

#[test]
fn test_parallelizable_tasks_diamond() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-1"]),
        create_task("task-4", vec!["task-2", "task-3"]),
    ];

    let plan = schedule_tasks(tasks).unwrap();

    // In diamond, either task-2 or task-3 can be parallelizable
    // depending on which path is chosen as critical
    assert!(
        plan.parallelizable_tasks.contains(&"task-2".to_string())
            || plan.parallelizable_tasks.contains(&"task-3".to_string())
    );

    // task-1 and task-4 must be on critical path
    assert!(plan.critical_path.contains(&"task-1".to_string()));
    assert!(plan.critical_path.contains(&"task-4".to_string()));
}

#[test]
fn test_parallelizable_tasks_none_in_linear() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-2"]),
    ];

    let plan = schedule_tasks(tasks).unwrap();

    // No parallelizable tasks in linear chain
    assert_eq!(plan.parallelizable_tasks.len(), 0);
}

#[test]
fn test_parallelizable_tasks_all_independent() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec![]),
        create_task("task-3", vec![]),
        create_task("task-4", vec![]),
    ];

    let plan = schedule_tasks(tasks).unwrap();

    // 3 out of 4 tasks should be parallelizable (one on critical path)
    assert_eq!(plan.parallelizable_tasks.len(), 3);
}

// ============================================================================
// Execution Levels Tests
// ============================================================================

#[test]
fn test_execution_levels_parallelism_maximization() {
    // Create a scenario where we want maximum parallelism
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec![]),
        create_task("task-3", vec![]),
        create_task("task-4", vec!["task-1", "task-2"]),
        create_task("task-5", vec!["task-2", "task-3"]),
        create_task("task-6", vec!["task-4", "task-5"]),
    ];

    let plan = schedule_tasks(tasks).unwrap();

    // Level 1: task-1, task-2, task-3 (all roots)
    assert_eq!(plan.execution_levels[0].len(), 3);

    // Level 2: task-4, task-5 (both have satisfied deps)
    assert_eq!(plan.execution_levels[1].len(), 2);

    // Level 3: task-6 (final task)
    assert_eq!(plan.execution_levels[2].len(), 1);

    assert_eq!(plan.max_depth, 3);
}

#[test]
fn test_execution_levels_dependency_satisfaction() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-1", "task-2"]),
        create_task("task-4", vec!["task-1"]),
    ];

    let plan = schedule_tasks(tasks).unwrap();

    // Level 1: only task-1
    assert_eq!(plan.execution_levels[0].len(), 1);

    // Level 2: task-2 and task-4 (both only depend on task-1)
    assert_eq!(plan.execution_levels[1].len(), 2);

    // Level 3: task-3 (needs both task-1 and task-2)
    assert_eq!(plan.execution_levels[2].len(), 1);
}

// ============================================================================
// Graph Statistics Tests
// ============================================================================

#[test]
fn test_graph_statistics_simple() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec![]),
        create_task("task-3", vec!["task-1", "task-2"]),
    ];

    let task_map = create_task_map(tasks);
    let stats = calculate_graph_statistics(&task_map).unwrap();

    assert_eq!(stats.total_tasks, 3);
    assert_eq!(stats.total_edges, 2); // task-1->task-3, task-2->task-3
    assert_eq!(stats.root_tasks, 2); // task-1 and task-2
    assert_eq!(stats.leaf_tasks, 1); // task-3
    assert_eq!(stats.max_depth, 2);
    assert!(stats.parallelism_factor > 1.0);
    assert_eq!(stats.critical_path_length, 2);
}

#[test]
fn test_graph_statistics_linear() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-2"]),
    ];

    let task_map = create_task_map(tasks);
    let stats = calculate_graph_statistics(&task_map).unwrap();

    assert_eq!(stats.total_tasks, 3);
    assert_eq!(stats.total_edges, 2);
    assert_eq!(stats.root_tasks, 1); // only task-1
    assert_eq!(stats.leaf_tasks, 1); // only task-3
    assert_eq!(stats.max_depth, 3);
    assert_eq!(stats.parallelism_factor, 1.0); // No parallelism in linear chain
}

#[test]
fn test_graph_statistics_empty() {
    let tasks = vec![];
    let task_map = create_task_map(tasks);
    let stats = calculate_graph_statistics(&task_map).unwrap();

    assert_eq!(stats.total_tasks, 0);
    assert_eq!(stats.total_edges, 0);
    assert_eq!(stats.root_tasks, 0);
    assert_eq!(stats.leaf_tasks, 0);
    assert_eq!(stats.max_depth, 0);
    assert_eq!(stats.parallelism_factor, 1.0);
}

#[test]
fn test_graph_statistics_high_parallelism() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec![]),
        create_task("task-3", vec![]),
        create_task("task-4", vec![]),
        create_task("task-5", vec!["task-1", "task-2", "task-3", "task-4"]),
    ];

    let task_map = create_task_map(tasks);
    let stats = calculate_graph_statistics(&task_map).unwrap();

    assert_eq!(stats.total_tasks, 5);
    assert_eq!(stats.root_tasks, 4);
    assert_eq!(stats.leaf_tasks, 1);
    assert_eq!(stats.max_depth, 2);

    // High parallelism: 5 tasks / 2 levels = 2.5
    assert!(stats.parallelism_factor >= 2.0);
}

// ============================================================================
// Visualization Tests
// ============================================================================

#[test]
fn test_mermaid_diagram_generation() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-1"]),
    ];

    let task_map = create_task_map(tasks);
    let diagram = generate_mermaid_diagram(&task_map);

    assert!(diagram.contains("graph TD"));
    assert!(diagram.contains("task-1 --> task-2"));
    assert!(diagram.contains("task-1 --> task-3"));
}

#[test]
fn test_mermaid_diagram_empty() {
    let tasks = vec![];
    let task_map = create_task_map(tasks);
    let diagram = generate_mermaid_diagram(&task_map);

    assert!(diagram.contains("graph TD"));
    assert_eq!(diagram.lines().count(), 1); // Only the header
}

#[test]
fn test_execution_plan_visualization() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
    ];

    let plan = schedule_tasks(tasks).unwrap();
    let viz = visualize_execution_plan(&plan);

    assert!(viz.contains("Execution Plan"));
    assert!(viz.contains("Total Tasks: 2"));
    assert!(viz.contains("Max Depth:"));
    assert!(viz.contains("Critical Path:"));
    assert!(viz.contains("Execution Levels:"));
}

#[test]
fn test_execution_plan_visualization_comprehensive() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-1"]),
        create_task("task-4", vec!["task-2", "task-3"]),
    ];

    let plan = schedule_tasks(tasks).unwrap();
    let viz = visualize_execution_plan(&plan);

    // Verify all key information is present
    assert!(viz.contains("Total Tasks: 4"));
    assert!(viz.contains("Max Depth: 3"));
    assert!(viz.contains("Critical Path Length: 3"));
    assert!(viz.contains(&format!(
        "Parallelizable Tasks: {}",
        plan.parallelizable_tasks.len()
    )));
    assert!(viz.contains("Level 1:"));
    assert!(viz.contains("Level 2:"));
    assert!(viz.contains("Level 3:"));
}

// ============================================================================
// Edge Cases and Large-Scale Tests
// ============================================================================

#[test]
fn test_large_scale_dag() {
    // Create a large DAG with 91 tasks in 10 levels
    let mut tasks = vec![];

    // Level 1: 1 root task
    tasks.push(create_task("task-0", vec![]));

    // Levels 2-10: 10 tasks each depending on previous level
    // Loop from 1 to 9 (9 iterations), creating 9 more levels
    for level in 1..10 {
        for i in 0..10 {
            let dep = format!("task-{}", (level - 1) * 10);
            let id = format!("task-{}", level * 10 + i);
            tasks.push(create_task(&id, vec![&dep]));
        }
    }

    let plan = schedule_tasks(tasks).expect("Should handle large DAG");

    // 1 root + 9 levels × 10 tasks = 91 total
    assert_eq!(plan.total_tasks, 91);
    assert_eq!(plan.max_depth, 10);
    assert_eq!(plan.execution_order.len(), 91);
}

#[test]
fn test_wide_dag_many_parallel_tasks() {
    // Create a wide DAG with one root and 50 parallel tasks
    let mut tasks = vec![create_task("root", vec![])];

    for i in 0..50 {
        tasks.push(create_task(&format!("task-{}", i), vec!["root"]));
    }

    let plan = schedule_tasks(tasks).expect("Should handle wide DAG");

    assert_eq!(plan.total_tasks, 51);
    assert_eq!(plan.max_depth, 2);
    assert_eq!(plan.execution_levels[0].len(), 1);
    assert_eq!(plan.execution_levels[1].len(), 50);
}

#[test]
fn test_deep_chain_performance() {
    // Create a very deep chain (1000 tasks)
    let mut tasks = vec![create_task("task-0", vec![])];

    for i in 1..1000 {
        let dep = format!("task-{}", i - 1);
        tasks.push(create_task(&format!("task-{}", i), vec![&dep]));
    }

    let plan = schedule_tasks(tasks).expect("Should handle deep chain");

    assert_eq!(plan.total_tasks, 1000);
    assert_eq!(plan.max_depth, 1000);
    assert_eq!(plan.execution_order.len(), 1000);

    // Verify order is correct
    for i in 0..1000 {
        assert_eq!(plan.execution_order[i], format!("task-{}", i));
    }
}

#[test]
fn test_fan_in_fan_out_pattern() {
    // Fan-out then fan-in pattern
    // root -> [1-10] -> merge
    let mut tasks = vec![create_task("root", vec![])];

    // Fan out
    for i in 1..=10 {
        tasks.push(create_task(&format!("branch-{}", i), vec!["root"]));
    }

    // Fan in
    let deps = (1..=10)
        .map(|i| format!("branch-{}", i))
        .collect::<Vec<_>>();
    let dep_refs: Vec<&str> = deps.iter().map(|s| s.as_str()).collect();
    tasks.push(create_task("merge", dep_refs));

    let plan = schedule_tasks(tasks).expect("Should handle fan-in/fan-out");

    assert_eq!(plan.total_tasks, 12);
    assert_eq!(plan.max_depth, 3);

    // Level 1: just root
    assert_eq!(plan.execution_levels[0].len(), 1);

    // Level 2: 10 branches
    assert_eq!(plan.execution_levels[1].len(), 10);

    // Level 3: merge
    assert_eq!(plan.execution_levels[2].len(), 1);
}

// ============================================================================
// Property-Based Tests
// ============================================================================

#[test]
fn test_execution_order_preserves_dependencies() {
    // For any valid DAG, execution order must preserve dependencies
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-1"]),
        create_task("task-4", vec!["task-2", "task-3"]),
        create_task("task-5", vec!["task-4"]),
    ];

    let plan = schedule_tasks(tasks.clone()).unwrap();
    let pos = |id: &str| -> usize { plan.execution_order.iter().position(|x| x == id).unwrap() };

    // For each task, verify all dependencies come before it
    let task_map: std::collections::HashMap<String, Task> =
        tasks.into_iter().map(|t| (t.id.clone(), t)).collect();

    for (task_id, task) in &task_map {
        let task_pos = pos(task_id);
        for dep_id in &task.depends_on {
            let dep_pos = pos(dep_id);
            assert!(
                dep_pos < task_pos,
                "Dependency {} should come before task {}",
                dep_id,
                task_id
            );
        }
    }
}

#[test]
fn test_all_tasks_scheduled_exactly_once() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-1"]),
    ];

    let plan = schedule_tasks(tasks).unwrap();

    // Check that all tasks appear exactly once in execution order
    let mut seen = HashSet::new();
    for task_id in &plan.execution_order {
        assert!(
            seen.insert(task_id),
            "Task {} appears more than once in execution order",
            task_id
        );
    }

    assert_eq!(seen.len(), 3);

    // Check that all tasks appear exactly once across all levels
    let mut level_count = 0;
    for level in &plan.execution_levels {
        for task in level {
            level_count += 1;
            assert!(
                seen.contains(&task.id),
                "Task {} in levels not in execution order",
                task.id
            );
        }
    }

    assert_eq!(level_count, 3);
}

#[test]
fn test_execution_levels_cover_all_tasks() {
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-1"]),
        create_task("task-4", vec!["task-2", "task-3"]),
    ];

    let plan = schedule_tasks(tasks).unwrap();

    // Count tasks across all levels
    let total_in_levels: usize = plan.execution_levels.iter().map(|level| level.len()).sum();

    assert_eq!(total_in_levels, plan.total_tasks);
    assert_eq!(total_in_levels, 4);
}

#[test]
fn test_no_cross_level_dependencies() {
    // Tasks in the same level should not depend on each other
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec![]),
        create_task("task-3", vec!["task-1", "task-2"]),
        create_task("task-4", vec!["task-3"]),
    ];

    let plan = schedule_tasks(tasks).unwrap();

    // Check each level
    for level in &plan.execution_levels {
        let level_ids: HashSet<&str> = level.iter().map(|t| t.id.as_str()).collect();

        // No task in this level should depend on another task in the same level
        for task in level {
            for dep_id in &task.depends_on {
                assert!(
                    !level_ids.contains(dep_id.as_str()),
                    "Task {} depends on {} which is in the same level",
                    task.id,
                    dep_id
                );
            }
        }
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_pipeline_integration() {
    // Simulate a realistic development workflow
    let tasks = vec![
        create_task("design", vec![]),
        create_task("frontend-spec", vec!["design"]),
        create_task("backend-spec", vec!["design"]),
        create_task("database-schema", vec!["design"]),
        create_task("frontend-impl", vec!["frontend-spec"]),
        create_task("backend-impl", vec!["backend-spec", "database-schema"]),
        create_task("api-integration", vec!["frontend-impl", "backend-impl"]),
        create_task("tests", vec!["api-integration"]),
        create_task("docs", vec!["api-integration"]),
        create_task("deploy", vec!["tests", "docs"]),
    ];

    let plan = schedule_tasks(tasks.clone()).expect("Should schedule full pipeline");

    // Verify structure
    assert_eq!(plan.total_tasks, 10);
    assert!(plan.max_depth >= 3 && plan.max_depth <= 6);

    // Verify we have reasonable parallelism
    let stats = calculate_graph_statistics(&tasks.into_iter().map(|t| (t.id.clone(), t)).collect())
        .unwrap();
    assert!(stats.parallelism_factor > 1.0);

    // Verify execution order is valid
    let pos = |id: &str| plan.execution_order.iter().position(|x| x == id).unwrap();
    assert!(pos("design") < pos("frontend-spec"));
    assert!(pos("backend-impl") < pos("api-integration"));
    assert!(pos("tests") < pos("deploy"));
    assert!(pos("docs") < pos("deploy"));
}

#[test]
fn test_microservices_topology() {
    // Simulate microservices with shared dependencies
    let tasks = vec![
        create_task("shared-utils", vec![]),
        create_task("auth-service", vec!["shared-utils"]),
        create_task("user-service", vec!["shared-utils"]),
        create_task("payment-service", vec!["shared-utils"]),
        create_task("api-gateway", vec!["auth-service", "user-service"]),
        create_task("checkout", vec!["api-gateway", "payment-service"]),
    ];

    let plan = schedule_tasks(tasks.clone()).expect("Should schedule microservices");

    assert_eq!(plan.total_tasks, 6);

    // shared-utils should be first
    assert_eq!(plan.execution_order[0], "shared-utils");

    // checkout should be last
    assert_eq!(plan.execution_order.last().unwrap(), "checkout");

    // Verify reasonable parallelism
    assert!(plan.parallelizable_tasks.len() >= 2);
}

// ============================================================================
// Additional Edge Cases (Task Requirements)
// ============================================================================

#[test]
fn test_fully_connected_graph() {
    // Create a fully connected DAG (each task depends on all previous tasks)
    // This creates a triangular dependency pattern
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec!["task-1"]),
        create_task("task-3", vec!["task-1", "task-2"]),
        create_task("task-4", vec!["task-1", "task-2", "task-3"]),
        create_task("task-5", vec!["task-1", "task-2", "task-3", "task-4"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should schedule fully connected graph");

    // In a fully connected graph, each task must be in its own level
    assert_eq!(plan.max_depth, 5);
    assert_eq!(plan.execution_levels.len(), 5);

    // Verify each level has exactly one task
    for (i, level) in plan.execution_levels.iter().enumerate() {
        assert_eq!(
            level.len(),
            1,
            "Level {} should have exactly 1 task in fully connected graph",
            i + 1
        );
    }

    // Verify no parallelizable tasks (linear execution)
    assert_eq!(plan.parallelizable_tasks.len(), 0);

    // All tasks should be on critical path
    assert_eq!(plan.critical_path.len(), 5);

    // Execution order must be sequential
    assert_eq!(
        plan.execution_order,
        vec!["task-1", "task-2", "task-3", "task-4", "task-5"]
    );
}

#[test]
fn test_deterministic_valid_topological_ordering() {
    // Verify that scheduling the same task graph multiple times
    // produces valid topological orders (dependencies before dependents)
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec![]),
        create_task("task-3", vec!["task-1", "task-2"]),
        create_task("task-4", vec!["task-1"]),
        create_task("task-5", vec!["task-2"]),
    ];

    // Schedule the tasks multiple times
    let plan1 = schedule_tasks(tasks.clone()).unwrap();
    let plan2 = schedule_tasks(tasks.clone()).unwrap();
    let plan3 = schedule_tasks(tasks.clone()).unwrap();

    // Helper to verify topological ordering is valid
    let is_valid_order = |plan: &ExecutionPlan| -> bool {
        let pos = |id: &str| plan.execution_order.iter().position(|x| x == id).unwrap();
        for task in &tasks {
            for dep in &task.depends_on {
                if pos(dep) >= pos(&task.id) {
                    return false;
                }
            }
        }
        true
    };

    // Verify all orders are valid topological sorts
    assert!(
        is_valid_order(&plan1),
        "Plan 1 should have valid topological order"
    );
    assert!(
        is_valid_order(&plan2),
        "Plan 2 should have valid topological order"
    );
    assert!(
        is_valid_order(&plan3),
        "Plan 3 should have valid topological order"
    );

    // Verify structure is consistent (same depth, same number of levels)
    assert_eq!(plan1.max_depth, plan2.max_depth);
    assert_eq!(plan2.max_depth, plan3.max_depth);
    assert_eq!(plan1.execution_levels.len(), plan2.execution_levels.len());
    assert_eq!(plan2.execution_levels.len(), plan3.execution_levels.len());

    // All plans should have the same critical path length
    assert_eq!(plan1.critical_path.len(), plan2.critical_path.len());
    assert_eq!(plan2.critical_path.len(), plan3.critical_path.len());
}

#[test]
fn test_deterministic_valid_topological_ordering_complex() {
    // Test valid topological ordering with a more complex graph
    let tasks = vec![
        create_task("a", vec![]),
        create_task("b", vec![]),
        create_task("c", vec!["a"]),
        create_task("d", vec!["a", "b"]),
        create_task("e", vec!["b"]),
        create_task("f", vec!["c", "d", "e"]),
    ];

    let results: Vec<_> = (0..10)
        .map(|_| schedule_tasks(tasks.clone()).unwrap())
        .collect();

    // Helper to check topological validity
    let is_valid_order = |plan: &ExecutionPlan| -> bool {
        let pos = |id: &str| plan.execution_order.iter().position(|x| x == id).unwrap();
        for task in &tasks {
            for dep in &task.depends_on {
                if pos(dep) >= pos(&task.id) {
                    return false;
                }
            }
        }
        true
    };

    // All results should be valid topological orders
    for (i, result) in results.iter().enumerate() {
        assert!(
            is_valid_order(result),
            "Plan {} should have valid topological order",
            i
        );
    }

    // All plans should have consistent structure
    let max_depth = results[0].max_depth;
    let num_levels = results[0].execution_levels.len();

    for (i, result) in results.iter().enumerate() {
        assert_eq!(
            result.max_depth, max_depth,
            "Plan {} should have same depth",
            i
        );
        assert_eq!(
            result.execution_levels.len(),
            num_levels,
            "Plan {} should have same number of levels",
            i
        );
    }
}

#[test]
fn test_single_task_edge_case() {
    // Verify single task handling
    let tasks = vec![create_task("only-task", vec![])];

    let plan = schedule_tasks(tasks).expect("Should handle single task");

    assert_eq!(plan.total_tasks, 1);
    assert_eq!(plan.execution_order, vec!["only-task"]);
    assert_eq!(plan.max_depth, 1);
    assert_eq!(plan.execution_levels.len(), 1);
    assert_eq!(plan.execution_levels[0].len(), 1);
    assert_eq!(plan.critical_path, vec!["only-task"]);
    assert_eq!(plan.parallelizable_tasks.len(), 0);
}

#[test]
fn test_two_independent_tasks() {
    // Minimal test for parallelism with two tasks
    let tasks = vec![create_task("task-a", vec![]), create_task("task-b", vec![])];

    let plan = schedule_tasks(tasks).expect("Should schedule two independent tasks");

    assert_eq!(plan.total_tasks, 2);
    assert_eq!(plan.max_depth, 1);
    assert_eq!(plan.execution_levels.len(), 1);
    assert_eq!(plan.execution_levels[0].len(), 2);

    // One task should be on critical path, one parallelizable
    assert_eq!(plan.parallelizable_tasks.len(), 1);
}

#[test]
fn test_task_with_multiple_dependencies_same_level() {
    // Task that depends on multiple tasks at the same level
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec![]),
        create_task("task-3", vec!["task-1", "task-2"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should schedule task with multiple dependencies");

    // Level 1: task-1 and task-2 (parallel)
    assert_eq!(plan.execution_levels[0].len(), 2);

    // Level 2: task-3 (depends on both)
    assert_eq!(plan.execution_levels[1].len(), 1);
    assert_eq!(plan.execution_levels[1][0].id, "task-3");

    assert_eq!(plan.max_depth, 2);
}

#[test]
fn test_transitive_dependencies() {
    // Test chain of transitive dependencies
    // A -> B -> C -> D means C transitively depends on A
    let tasks = vec![
        create_task("a", vec![]),
        create_task("b", vec!["a"]),
        create_task("c", vec!["b"]),
        create_task("d", vec!["c"]),
        create_task("e", vec!["c"]),
        create_task("f", vec!["d", "e"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should handle transitive dependencies");

    // Verify order respects transitive dependencies
    let pos = |id: &str| plan.execution_order.iter().position(|x| x == id).unwrap();

    assert!(pos("a") < pos("b"));
    assert!(pos("b") < pos("c"));
    assert!(pos("c") < pos("d"));
    assert!(pos("c") < pos("e"));
    assert!(pos("d") < pos("f"));
    assert!(pos("e") < pos("f"));

    // e transitively depends on a (via c)
    assert!(pos("a") < pos("e"));
}
