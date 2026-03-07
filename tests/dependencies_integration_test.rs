//! Integration tests for task dependency resolution and topological sorting
//!
//! These tests verify:
//! - Dependency graph construction from task.depends_on arrays
//! - Topological sorting for correct execution order
//! - Circular dependency detection
//! - Execution level calculation for parallel execution
//! - Critical path identification
//! - Blocked task handling strategies

use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::tasks::scheduler::schedule_tasks;
use ltmatrix::tasks::topology::{
    visualize_dependency_graph, visualize_execution_plan, TopologyConfig,
};
use std::collections::HashSet;

// =============================================================================
// Test Helper Functions
// =============================================================================

/// Create a sample task with the given ID, title, and dependencies
fn create_task(id: &str, title: &str, depends_on: Vec<&str>) -> Task {
    let mut task = Task::new(id, title, format!("Description for {}", title));
    task.depends_on = depends_on.iter().map(|s| s.to_string()).collect();
    task.complexity = TaskComplexity::Moderate;
    task
}

/// Create a task with a specific status
fn create_task_with_status(id: &str, title: &str, depends_on: Vec<&str>, status: TaskStatus) -> Task {
    let mut task = create_task(id, title, depends_on);
    task.status = status;
    task
}

// =============================================================================
// Basic Dependency Resolution Tests
// =============================================================================

/// Test simple linear dependency chain: A -> B -> C
#[test]
fn test_linear_dependency_chain() {
    let tasks = vec![
        create_task("task-c", "Task C", vec!["task-b"]),
        create_task("task-b", "Task B", vec!["task-a"]),
        create_task("task-a", "Task A", vec![]),
    ];

    let plan = schedule_tasks(tasks).expect("Should create execution plan");

    // Verify execution order respects dependencies
    assert_eq!(plan.total_tasks, 3, "Should have 3 tasks");

    // Task A should come before Task B
    let a_pos = plan.execution_order.iter().position(|id| id == "task-a").unwrap();
    let b_pos = plan.execution_order.iter().position(|id| id == "task-b").unwrap();
    let c_pos = plan.execution_order.iter().position(|id| id == "task-c").unwrap();

    assert!(a_pos < b_pos, "Task A should execute before Task B");
    assert!(b_pos < c_pos, "Task B should execute before Task C");

    // Verify critical path is the entire chain
    assert_eq!(plan.critical_path.len(), 3, "All tasks should be on critical path");
}

/// Test diamond dependency pattern: A -> B, A -> C, B -> D, C -> D
#[test]
fn test_diamond_dependency_pattern() {
    let tasks = vec![
        create_task("task-d", "Task D", vec!["task-b", "task-c"]),
        create_task("task-c", "Task C", vec!["task-a"]),
        create_task("task-b", "Task B", vec!["task-a"]),
        create_task("task-a", "Task A", vec![]),
    ];

    let plan = schedule_tasks(tasks).expect("Should create execution plan");

    // Verify execution order respects dependencies
    let a_pos = plan.execution_order.iter().position(|id| id == "task-a").unwrap();
    let b_pos = plan.execution_order.iter().position(|id| id == "task-b").unwrap();
    let c_pos = plan.execution_order.iter().position(|id| id == "task-c").unwrap();
    let d_pos = plan.execution_order.iter().position(|id| id == "task-d").unwrap();

    // A must come before B, C, and D
    assert!(a_pos < b_pos, "A should come before B");
    assert!(a_pos < c_pos, "A should come before C");
    assert!(a_pos < d_pos, "A should come before D");

    // B and C must come before D
    assert!(b_pos < d_pos, "B should come before D");
    assert!(c_pos < d_pos, "C should come before D");

    // Verify parallelizable tasks
    // Either B or C can be parallelized (not both on critical path)
    assert!(!plan.parallelizable_tasks.is_empty() || plan.critical_path.len() == 4,
            "Should identify parallelizable tasks or all on critical path");
}

/// Test multiple independent task chains
#[test]
fn test_multiple_independent_chains() {
    let tasks = vec![
        // Chain 1: A -> B -> C
        create_task("chain1-a", "Chain 1 A", vec![]),
        create_task("chain1-b", "Chain 1 B", vec!["chain1-a"]),
        create_task("chain1-c", "Chain 1 C", vec!["chain1-b"]),
        // Chain 2: D -> E -> F
        create_task("chain2-d", "Chain 2 D", vec![]),
        create_task("chain2-e", "Chain 2 E", vec!["chain2-d"]),
        create_task("chain2-f", "Chain 2 F", vec!["chain2-e"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should create execution plan");

    // Verify all 6 tasks are accounted for
    assert_eq!(plan.total_tasks, 6, "Should have 6 tasks");

    // Verify execution levels - should have 3 levels
    assert_eq!(plan.execution_levels.len(), 3, "Should have 3 execution levels");

    // Level 0 should have chain1-a and chain2-d (no dependencies)
    assert_eq!(plan.execution_levels[0].len(), 2, "Level 0 should have 2 tasks");

    // Verify chain1 dependencies
    let chain1_a_pos = plan.execution_order.iter().position(|id| id == "chain1-a").unwrap();
    let chain1_b_pos = plan.execution_order.iter().position(|id| id == "chain1-b").unwrap();
    let chain1_c_pos = plan.execution_order.iter().position(|id| id == "chain1-c").unwrap();
    assert!(chain1_a_pos < chain1_b_pos, "Chain1-A before Chain1-B");
    assert!(chain1_b_pos < chain1_c_pos, "Chain1-B before Chain1-C");

    // Verify chain2 dependencies
    let chain2_d_pos = plan.execution_order.iter().position(|id| id == "chain2-d").unwrap();
    let chain2_e_pos = plan.execution_order.iter().position(|id| id == "chain2-e").unwrap();
    let chain2_f_pos = plan.execution_order.iter().position(|id| id == "chain2-f").unwrap();
    assert!(chain2_d_pos < chain2_e_pos, "Chain2-D before Chain2-E");
    assert!(chain2_e_pos < chain2_f_pos, "Chain2-E before Chain2-F");
}

/// Test single task with no dependencies
#[test]
fn test_single_task_no_dependencies() {
    let tasks = vec![
        create_task("solo", "Solo Task", vec![]),
    ];

    let plan = schedule_tasks(tasks).expect("Should create execution plan");

    assert_eq!(plan.total_tasks, 1, "Should have 1 task");
    assert_eq!(plan.execution_order.len(), 1, "Execution order should have 1 task");
    assert_eq!(plan.execution_order[0], "solo", "Execution order should be [solo]");
    assert_eq!(plan.execution_levels.len(), 1, "Should have 1 execution level");
    assert_eq!(plan.critical_path.len(), 1, "Critical path should have 1 task");
    assert_eq!(plan.max_depth, 1, "Max depth should be 1");
}

/// Test empty task list
#[test]
fn test_empty_task_list() {
    let tasks: Vec<Task> = vec![];
    let plan = schedule_tasks(tasks).expect("Should create execution plan");

    assert_eq!(plan.total_tasks, 0, "Should have 0 tasks");
    assert!(plan.execution_order.is_empty(), "Execution order should be empty");
    assert!(plan.execution_levels.is_empty(), "Execution levels should be empty");
}

// =============================================================================
// Circular Dependency Detection Tests
// =============================================================================

/// Test simple cycle detection: A -> B -> A
#[test]
fn test_simple_circular_dependency() {
    let tasks = vec![
        create_task("task-a", "Task A", vec!["task-b"]),
        create_task("task-b", "Task B", vec!["task-a"]),
    ];

    let result = schedule_tasks(tasks);
    assert!(result.is_err(), "Should detect circular dependency");

    let error = result.unwrap_err();
    let error_msg = error.to_string();
    assert!(
        error_msg.contains("Circular dependency") || error_msg.contains("cycle"),
        "Error message should mention circular dependency"
    );
}

/// Test longer cycle detection: A -> B -> C -> A
#[test]
fn test_longer_circular_dependency() {
    let tasks = vec![
        create_task("task-a", "Task A", vec!["task-c"]),
        create_task("task-b", "Task B", vec!["task-a"]),
        create_task("task-c", "Task C", vec!["task-b"]),
    ];

    let result = schedule_tasks(tasks);
    assert!(result.is_err(), "Should detect circular dependency");
}

/// Test self-referential dependency: A -> A
#[test]
fn test_self_referential_dependency() {
    let tasks = vec![
        create_task("task-a", "Task A", vec!["task-a"]),
    ];

    let result = schedule_tasks(tasks);
    assert!(result.is_err(), "Should detect self-referential dependency");
}

/// Test diamond with cycle in one branch
#[test]
fn test_diamond_with_embedded_cycle() {
    let tasks = vec![
        create_task("task-a", "Task A", vec![]),
        create_task("task-b", "Task B", vec!["task-a", "task-c"]), // Creates cycle with C
        create_task("task-c", "Task C", vec!["task-b"]),
        create_task("task-d", "Task D", vec!["task-a"]),
        create_task("task-e", "Task E", vec!["task-b", "task-d"]),
    ];

    let result = schedule_tasks(tasks);
    assert!(result.is_err(), "Should detect embedded cycle");
}

// =============================================================================
// Missing Dependency Tests
// =============================================================================

/// Test dependency on non-existent task
#[test]
fn test_missing_dependency() {
    let tasks = vec![
        create_task("task-a", "Task A", vec!["non-existent"]),
    ];

    let result = schedule_tasks(tasks);
    assert!(result.is_err(), "Should detect missing dependency");

    let error = result.unwrap_err();
    let error_msg = error.to_string();
    assert!(
        error_msg.contains("Missing") || error_msg.contains("non-existent"),
        "Error message should mention missing dependency"
    );
}

/// Test multiple missing dependencies
#[test]
fn test_multiple_missing_dependencies() {
    let tasks = vec![
        create_task("task-a", "Task A", vec!["missing-1", "missing-2"]),
        create_task("task-b", "Task B", vec!["task-a", "missing-3"]),
    ];

    let result = schedule_tasks(tasks);
    assert!(result.is_err(), "Should detect missing dependencies");
}

// =============================================================================
// Execution Level Tests
// =============================================================================

/// Test execution level calculation for parallel tasks
#[test]
fn test_execution_levels_parallel_tasks() {
    let tasks = vec![
        // Root level - 3 independent tasks
        create_task("root-1", "Root 1", vec![]),
        create_task("root-2", "Root 2", vec![]),
        create_task("root-3", "Root 3", vec![]),
        // Level 1 - depends on roots
        create_task("mid-1", "Mid 1", vec!["root-1", "root-2"]),
        create_task("mid-2", "Mid 2", vec!["root-2", "root-3"]),
        // Level 2 - depends on mids
        create_task("final", "Final", vec!["mid-1", "mid-2"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should create execution plan");

    // Verify 3 execution levels
    assert_eq!(plan.execution_levels.len(), 3, "Should have 3 execution levels");

    // Level 0: root tasks (3)
    assert_eq!(plan.execution_levels[0].len(), 3, "Level 0 should have 3 root tasks");

    // Level 1: mid tasks (2)
    assert_eq!(plan.execution_levels[1].len(), 2, "Level 1 should have 2 mid tasks");

    // Level 2: final task (1)
    assert_eq!(plan.execution_levels[2].len(), 1, "Level 2 should have 1 final task");

    // Verify tasks in each level
    let level_0_ids: HashSet<&str> = plan.execution_levels[0]
        .iter()
        .map(|t| t.id.as_str())
        .collect();
    assert!(level_0_ids.contains("root-1"), "Level 0 should contain root-1");
    assert!(level_0_ids.contains("root-2"), "Level 0 should contain root-2");
    assert!(level_0_ids.contains("root-3"), "Level 0 should contain root-3");
}

/// Test that tasks in same level have no dependencies on each other
#[test]
fn test_same_level_tasks_are_independent() {
    let tasks = vec![
        create_task("a", "Task A", vec![]),
        create_task("b", "Task B", vec![]),
        create_task("c", "Task C", vec![]),
        create_task("d", "Task D", vec!["a", "b", "c"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should create execution plan");

    // All tasks in level 0 should have no dependencies on each other
    let level_0 = &plan.execution_levels[0];
    for task in level_0 {
        assert!(task.depends_on.is_empty(),
                "Level 0 task {} should have no dependencies", task.id);
    }
}

// =============================================================================
// Critical Path Tests
// =============================================================================

/// Test critical path in linear chain
#[test]
fn test_critical_path_linear_chain() {
    let tasks = vec![
        create_task("a", "Task A", vec![]),
        create_task("b", "Task B", vec!["a"]),
        create_task("c", "Task C", vec!["b"]),
        create_task("d", "Task D", vec!["c"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should create execution plan");

    // In a linear chain, all tasks are on the critical path
    assert_eq!(plan.critical_path.len(), 4, "All 4 tasks should be on critical path");

    // Critical path should be in order
    assert_eq!(plan.critical_path[0], "a", "Critical path should start with a");
    assert_eq!(plan.critical_path[1], "b", "Critical path should continue with b");
    assert_eq!(plan.critical_path[2], "c", "Critical path should continue with c");
    assert_eq!(plan.critical_path[3], "d", "Critical path should end with d");
}

/// Test critical path with parallel branches
#[test]
fn test_critical_path_parallel_branches() {
    let tasks = vec![
        create_task("root", "Root", vec![]),
        // Branch 1: Shorter
        create_task("short-1", "Short 1", vec!["root"]),
        // Branch 2: Longer
        create_task("long-1", "Long 1", vec!["root"]),
        create_task("long-2", "Long 2", vec!["long-1"]),
        create_task("long-3", "Long 3", vec!["long-2"]),
        // Final task depends on both branches
        create_task("final", "Final", vec!["short-1", "long-3"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should create execution plan");

    // Critical path should follow the longer branch
    assert!(plan.critical_path.len() >= 4, "Critical path should include the longer branch");

    // Root and final should always be on critical path
    assert!(plan.critical_path.contains(&"root".to_string()),
            "Root should be on critical path");
    assert!(plan.critical_path.contains(&"final".to_string()),
            "Final should be on critical path");
}

// =============================================================================
// Parallelizable Tasks Tests
// =============================================================================

/// Test identification of parallelizable tasks
#[test]
fn test_parallelizable_tasks_identification() {
    let tasks = vec![
        create_task("a", "Task A", vec![]),
        create_task("b", "Task B", vec![]),
        create_task("c", "Task C", vec!["a"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should create execution plan");

    // Task B should be parallelizable (not on critical path if A->C is critical)
    // or at least we should have some identification
    // Note: parallelizable_tasks is a HashSet, so length is always >= 0
    assert!(!plan.parallelizable_tasks.is_empty() || plan.critical_path.len() == 3,
            "Should identify parallelizable tasks or all tasks on critical path");
}

/// Test that all tasks are either parallelizable or on critical path
#[test]
fn test_tasks_are_parallelizable_or_critical() {
    let tasks = vec![
        create_task("a", "Task A", vec![]),
        create_task("b", "Task B", vec![]),
        create_task("c", "Task C", vec!["a"]),
        create_task("d", "Task D", vec!["b"]),
        create_task("e", "Task E", vec!["c", "d"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should create execution plan");

    // Every task should either be on critical path or be parallelizable
    let critical_set: HashSet<String> = plan.critical_path.iter().cloned().collect();

    for task_id in &plan.execution_order {
        let is_critical = critical_set.contains(task_id);
        let is_parallelizable = plan.parallelizable_tasks.contains(task_id);

        assert!(is_critical || is_parallelizable,
                "Task {} should be either critical or parallelizable", task_id);
    }
}

// =============================================================================
// Topology Visualization Tests
// =============================================================================

/// Test dependency graph visualization
#[test]
fn test_dependency_graph_visualization() {
    let tasks = vec![
        create_task("a", "Task A", vec![]),
        create_task("b", "Task B", vec!["a"]),
        create_task("c", "Task C", vec!["a"]),
    ];

    let visualization = visualize_dependency_graph(&tasks, None);

    // Visualization shows task IDs and DAG structure
    assert!(visualization.contains("a"), "Should contain task a");
    assert!(visualization.contains("b"), "Should contain task b");
    assert!(visualization.contains("c"), "Should contain task c");
    assert!(visualization.contains("DAG"), "Should contain DAG header");
}

/// Test execution plan visualization
#[test]
fn test_execution_plan_visualization() {
    let tasks = vec![
        create_task("a", "Task A", vec![]),
        create_task("b", "Task B", vec!["a"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should create plan");
    let visualization = visualize_execution_plan(&plan, None);

    assert!(visualization.contains("Execution Plan"), "Should contain header");
    assert!(visualization.contains("Total Tasks: 2"), "Should show total tasks");
    assert!(visualization.contains("Level"), "Should show execution levels");
}

/// Test visualization with empty tasks
#[test]
fn test_visualization_empty_tasks() {
    let tasks: Vec<Task> = vec![];
    let visualization = visualize_dependency_graph(&tasks, None);

    assert!(visualization.contains("No tasks"), "Should indicate no tasks");
}

/// Test visualization with custom config
#[test]
fn test_visualization_with_config() {
    let tasks = vec![
        create_task_with_status("a", "Task A", vec![], TaskStatus::Completed),
        create_task_with_status("b", "Task B", vec!["a"], TaskStatus::InProgress),
    ];

    let config = TopologyConfig {
        show_status: true,
        highlight_critical: true,
        show_levels: true,
        compact: false,
    };

    let visualization = visualize_dependency_graph(&tasks, Some(config));

    // Visualization shows task IDs with the configured options
    assert!(visualization.contains("a"), "Should contain task a");
    assert!(visualization.contains("b"), "Should contain task b");
}

// =============================================================================
// Complex Graph Tests
// =============================================================================

/// Test complex multi-level dependency graph
#[test]
fn test_complex_dependency_graph() {
    let tasks = vec![
        // Level 0: Roots
        create_task("setup", "Setup Project", vec![]),
        create_task("docs-plan", "Plan Documentation", vec![]),

        // Level 1: Core implementation
        create_task("core", "Core Module", vec!["setup"]),
        create_task("config", "Configuration", vec!["setup"]),
        create_task("docs-setup", "Docs Setup", vec!["docs-plan"]),

        // Level 2: Features
        create_task("feature-a", "Feature A", vec!["core", "config"]),
        create_task("feature-b", "Feature B", vec!["core"]),
        create_task("tests-core", "Core Tests", vec!["core"]),

        // Level 3: Integration
        create_task("integration", "Integration", vec!["feature-a", "feature-b"]),
        create_task("docs-api", "API Docs", vec!["feature-a", "docs-setup"]),

        // Level 4: Final
        create_task("e2e-tests", "E2E Tests", vec!["integration"]),
        create_task("release", "Release", vec!["integration", "tests-core", "docs-api"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should handle complex graph");

    // Verify all tasks are scheduled
    assert_eq!(plan.total_tasks, 12, "Should have 12 tasks");

    // Verify max depth
    assert!(plan.max_depth >= 4, "Should have at least 4 levels");

    // Verify critical path exists
    assert!(!plan.critical_path.is_empty(), "Should have critical path");

    // Verify setup comes before everything that depends on it
    let setup_pos = plan.execution_order.iter().position(|id| id == "setup").unwrap();
    let core_pos = plan.execution_order.iter().position(|id| id == "core").unwrap();
    let feature_a_pos = plan.execution_order.iter().position(|id| id == "feature-a").unwrap();

    assert!(setup_pos < core_pos, "Setup should come before core");
    assert!(core_pos < feature_a_pos, "Core should come before feature-a");

    // Verify integration comes after its dependencies
    let integration_pos = plan.execution_order.iter().position(|id| id == "integration").unwrap();
    let feature_b_pos = plan.execution_order.iter().position(|id| id == "feature-b").unwrap();

    assert!(feature_a_pos < integration_pos, "Feature A should come before integration");
    assert!(feature_b_pos < integration_pos, "Feature B should come before integration");
}

/// Test graph with many parallel branches
#[test]
fn test_many_parallel_branches() {
    let mut tasks = Vec::new();

    // Create root
    tasks.push(create_task("root", "Root", vec![]));

    // Create 10 parallel branches from root
    for i in 0..10 {
        let branch_id = format!("branch-{}", i);
        tasks.push(create_task(&branch_id, &format!("Branch {}", i), vec!["root"]));
    }

    // Create final task that depends on all branches
    let all_branches: Vec<&str> = (0..10).map(|i| {
        Box::leak(format!("branch-{}", i).into_boxed_str()) as &str
    }).collect();
    tasks.push(create_task("final", "Final", all_branches));

    let plan = schedule_tasks(tasks).expect("Should handle many branches");

    assert_eq!(plan.total_tasks, 12, "Should have 12 tasks");
    assert_eq!(plan.execution_levels.len(), 3, "Should have 3 levels");

    // Level 1 should have 10 parallel tasks
    assert_eq!(plan.execution_levels[1].len(), 10, "Level 1 should have 10 parallel branches");
}

// =============================================================================
// Edge Cases
// =============================================================================

/// Test graph where all tasks have same dependency
#[test]
fn test_all_depend_on_same_task() {
    let tasks = vec![
        create_task("common", "Common Dependency", vec![]),
        create_task("a", "Task A", vec!["common"]),
        create_task("b", "Task B", vec!["common"]),
        create_task("c", "Task C", vec!["common"]),
    ];

    let plan = schedule_tasks(tasks).expect("Should handle common dependency");

    assert_eq!(plan.total_tasks, 4, "Should have 4 tasks");

    // Common should come first
    let common_pos = plan.execution_order.iter().position(|id| id == "common").unwrap();
    for id in &["a", "b", "c"] {
        let pos = plan.execution_order.iter().position(|t| t == *id).unwrap();
        assert!(common_pos < pos, "Common should come before {}", id);
    }
}

/// Test deeply nested dependency chain
#[test]
fn test_deeply_nested_chain() {
    let mut tasks = Vec::new();

    // Create a chain of 50 tasks
    tasks.push(create_task("task-0", "Task 0", vec![]));
    for i in 1..50 {
        let prev_id = format!("task-{}", i - 1);
        let curr_id = format!("task-{}", i);
        tasks.push(create_task(&curr_id, &format!("Task {}", i), vec![&prev_id]));
    }

    let plan = schedule_tasks(tasks).expect("Should handle deep chain");

    assert_eq!(plan.total_tasks, 50, "Should have 50 tasks");
    assert_eq!(plan.max_depth, 50, "Max depth should be 50");
    assert_eq!(plan.critical_path.len(), 50, "All tasks should be on critical path");
}

/// Test task with many dependencies
#[test]
fn test_task_with_many_dependencies() {
    let mut tasks = Vec::new();

    // Create 20 root tasks
    for i in 0..20 {
        tasks.push(create_task(&format!("dep-{}", i), &format!("Dep {}", i), vec![]));
    }

    // Create one task that depends on all of them
    let all_deps: Vec<&str> = (0..20).map(|i| {
        Box::leak(format!("dep-{}", i).into_boxed_str()) as &str
    }).collect();
    tasks.push(create_task("final", "Final Task", all_deps));

    let plan = schedule_tasks(tasks).expect("Should handle many dependencies");

    assert_eq!(plan.total_tasks, 21, "Should have 21 tasks");

    // Final should be in last level
    let final_level = plan.execution_levels.iter()
        .position(|level| level.iter().any(|t| t.id == "final"))
        .unwrap();
    assert_eq!(final_level, 1, "Final should be in level 1 (after roots)");
}

// =============================================================================
// Task Status Interaction Tests
// =============================================================================

/// Test that completed tasks are still included in dependency resolution
#[test]
fn test_completed_tasks_in_graph() {
    let tasks = vec![
        create_task_with_status("a", "Task A", vec![], TaskStatus::Completed),
        create_task_with_status("b", "Task B", vec!["a"], TaskStatus::InProgress),
        create_task_with_status("c", "Task C", vec!["b"], TaskStatus::Pending),
    ];

    let plan = schedule_tasks(tasks).expect("Should handle tasks with status");

    assert_eq!(plan.total_tasks, 3, "Should have 3 tasks");

    // Order should still respect dependencies regardless of status
    let a_pos = plan.execution_order.iter().position(|id| id == "a").unwrap();
    let b_pos = plan.execution_order.iter().position(|id| id == "b").unwrap();
    let c_pos = plan.execution_order.iter().position(|id| id == "c").unwrap();

    assert!(a_pos < b_pos, "A should come before B");
    assert!(b_pos < c_pos, "B should come before C");
}

// =============================================================================
// Performance Tests
// =============================================================================

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    /// Benchmark scheduling for large task graphs
    #[test]
    fn bench_large_task_graph() {
        let mut tasks = Vec::new();

        // Create a graph with 1000 tasks
        // 100 root tasks, each with 10 dependent tasks
        for i in 0..100 {
            tasks.push(create_task(&format!("root-{}", i), &format!("Root {}", i), vec![]));
        }

        for i in 0..100 {
            for j in 0..10 {
                let root_id = format!("root-{}", i);
                let dep_id = format!("dep-{}-{}", i, j);
                tasks.push(create_task(&dep_id, &format!("Dep {}-{}", i, j), vec![&root_id]));
            }
        }

        let start = Instant::now();
        let result = schedule_tasks(tasks);
        let duration = start.elapsed();

        assert!(result.is_ok(), "Should handle 1000 tasks");
        let plan = result.unwrap();
        assert_eq!(plan.total_tasks, 1100, "Should have 1100 tasks");

        // Should complete in reasonable time (< 1 second)
        assert!(duration < std::time::Duration::from_secs(1),
                "Scheduling 1000 tasks should take < 1s, took {:?}", duration);
    }

    /// Benchmark cycle detection in large graphs
    #[test]
    fn bench_cycle_detection_large_graph() {
        let mut tasks = Vec::new();

        // Create a large graph with a cycle at the end
        for i in 0..500 {
            if i == 0 {
                tasks.push(create_task("task-0", "Task 0", vec![]));
            } else {
                let prev = format!("task-{}", i - 1);
                tasks.push(create_task(&format!("task-{}", i), &format!("Task {}", i), vec![&prev]));
            }
        }

        // Add cycle: last task depends on middle task
        tasks.push(create_task("cyclic", "Cyclic", vec!["task-250"]));
        tasks.push(create_task("task-250", "Task 250", vec!["cyclic"]));

        let start = Instant::now();
        let result = schedule_tasks(tasks);
        let duration = start.elapsed();

        assert!(result.is_err(), "Should detect cycle");

        // Should detect cycle in reasonable time
        assert!(duration < std::time::Duration::from_secs(2),
                "Cycle detection should take < 2s, took {:?}", duration);
    }
}

// =============================================================================
// Integration with Blocked Strategy Tests
// =============================================================================

/// Test identifying blocked tasks (dependencies on failed tasks)
#[test]
fn test_identify_blocked_tasks() {
    let tasks = vec![
        create_task_with_status("a", "Task A", vec![], TaskStatus::Failed),
        create_task_with_status("b", "Task B", vec!["a"], TaskStatus::Pending),
        create_task_with_status("c", "Task C", vec!["b"], TaskStatus::Pending),
    ];

    let plan = schedule_tasks(tasks).expect("Should create plan");

    // B depends on failed task A, so B is blocked
    // The scheduler should still create the plan
    assert_eq!(plan.total_tasks, 3, "Should have 3 tasks");

    // Order should still be valid
    let a_pos = plan.execution_order.iter().position(|id| id == "a").unwrap();
    let b_pos = plan.execution_order.iter().position(|id| id == "b").unwrap();
    assert!(a_pos < b_pos, "A should come before B");
}

/// Test task can_execute with completed dependencies
#[test]
fn test_task_can_execute_method() {
    let task = create_task("dependent", "Dependent Task", vec!["prereq-1", "prereq-2"]);

    let mut completed = HashSet::new();

    // No dependencies completed
    assert!(!task.can_execute(&completed), "Should not execute without completed deps");

    // One dependency completed
    completed.insert("prereq-1".to_string());
    assert!(!task.can_execute(&completed), "Should not execute with partial deps");

    // All dependencies completed
    completed.insert("prereq-2".to_string());
    assert!(task.can_execute(&completed), "Should execute with all deps completed");
}