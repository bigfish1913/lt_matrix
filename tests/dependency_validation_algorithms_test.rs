//! Algorithm tests for dependency validation logic
//!
//! These tests specifically verify the graph traversal algorithms used in
//! validate_dependencies() and validate_dependencies_with_stats():
//!
//! - Missing dependency detection using HashSet lookup
//! - Circular dependency detection using DFS
//! - Cycle detection accuracy and chain extraction
//! - Graph statistics calculation
//! - Depth calculation algorithm
//!
//! This file tests the algorithms themselves, ensuring correct implementation
//! of graph theory concepts for task dependency validation.

use ltmatrix::models::Task;
use ltmatrix::pipeline::generate::{validate_dependencies, validate_dependencies_with_stats};

// ============================================================================
// Test Helpers
// ============================================================================

/// Creates a task with dependencies
fn create_task(id: &str, deps: Vec<&str>) -> Task {
    let mut task = Task::new(id, id, format!("Task {}", id));
    task.depends_on = deps.into_iter().map(|s| s.to_string()).collect();
    task
}

// ============================================================================
// Missing Dependency Detection Algorithm Tests
// ============================================================================

#[test]
fn test_algorithm_missing_detection_hashset_lookup() {
    // Test that the algorithm correctly uses HashSet for O(1) lookup
    // by creating a task with many dependencies where only one is missing

    let mut deps = vec![];
    for i in 1..=50 {
        deps.push(format!("existing-{}", i));
    }
    deps.push("non-existent".to_string());

    let mut tasks = vec![];
    for i in 1..=50 {
        tasks.push(create_task(&format!("existing-{}", i), vec![]));
    }

    let mut task_with_missing = Task::new("test", "Test", "Test task");
    task_with_missing.depends_on = deps;
    tasks.push(task_with_missing);

    let errors = validate_dependencies(&tasks);

    // Should detect exactly one missing dependency
    assert_eq!(errors.len(), 1);
    match &errors[0] {
        ltmatrix::pipeline::generate::ValidationError::MissingDependency { task, dependency } => {
            assert_eq!(task, "test");
            assert_eq!(dependency, "non-existent");
        }
        _ => panic!("Expected MissingDependency error"),
    }
}

#[test]
fn test_algorithm_missing_detection_multiple_tasks_same_missing() {
    // Test when multiple tasks reference the same missing dependency

    let tasks = vec![
        create_task("task-1", vec!["missing"]),
        create_task("task-2", vec!["missing"]),
        create_task("task-3", vec!["missing"]),
    ];

    let errors = validate_dependencies(&tasks);

    // Should detect missing dependency for each task
    assert_eq!(errors.len(), 3);
    assert!(errors.iter().all(|e| matches!(
        e,
        ltmatrix::pipeline::generate::ValidationError::MissingDependency { .. }
    )));
}

#[test]
fn test_algorithm_missing_detection_empty_task_id_set() {
    // Test with empty task list
    let tasks: Vec<Task> = vec![];
    let errors = validate_dependencies(&tasks);

    assert!(errors.is_empty());
}

// ============================================================================
// Circular Dependency Detection (DFS Algorithm) Tests
// ============================================================================

#[test]
fn test_algorithm_dfs_cycle_detection_two_nodes() {
    // Simple 2-node cycle: A -> B -> A
    let tasks = vec![
        create_task("a", vec!["b"]),
        create_task("b", vec!["a"]),
    ];

    let errors = validate_dependencies(&tasks);

    assert_eq!(errors.len(), 1);
    match &errors[0] {
        ltmatrix::pipeline::generate::ValidationError::CircularDependency { chain } => {
            assert_eq!(chain.len(), 2);
            assert!(chain.contains(&"a".to_string()));
            assert!(chain.contains(&"b".to_string()));
        }
        _ => panic!("Expected CircularDependency error"),
    }
}

#[test]
fn test_algorithm_dfs_cycle_detection_three_nodes() {
    // 3-node cycle: A -> B -> C -> A
    let tasks = vec![
        create_task("a", vec!["c"]),
        create_task("b", vec!["a"]),
        create_task("c", vec!["b"]),
    ];

    let errors = validate_dependencies(&tasks);

    assert_eq!(errors.len(), 1);
    match &errors[0] {
        ltmatrix::pipeline::generate::ValidationError::CircularDependency { chain } => {
            assert_eq!(chain.len(), 3);
            assert!(chain.contains(&"a".to_string()));
            assert!(chain.contains(&"b".to_string()));
            assert!(chain.contains(&"c".to_string()));
        }
        _ => panic!("Expected CircularDependency error"),
    }
}

#[test]
fn test_algorithm_dfs_cycle_extraction_from_path() {
    // Test that DFS correctly extracts the cycle from the path
    // When cycle is detected, it should extract from cycle start position

    let tasks = vec![
        create_task("entry", vec!["middle"]),
        create_task("middle", vec!["cycle-start"]),
        create_task("cycle-start", vec!["cycle-mid"]),
        create_task("cycle-mid", vec!["cycle-end"]),
        create_task("cycle-end", vec!["cycle-start"]),
    ];

    let errors = validate_dependencies(&tasks);

    // Should detect at least one circular dependency
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| matches!(
        e,
        ltmatrix::pipeline::generate::ValidationError::CircularDependency { .. }
    )));

    // Check that at least one circular dependency contains cycle nodes
    let has_cycle = errors.iter().any(|e| {
        if let ltmatrix::pipeline::generate::ValidationError::CircularDependency { chain } = e {
            chain.len() >= 2 && (chain.contains(&"cycle-start".to_string()) || chain.contains(&"cycle-end".to_string()))
        } else {
            false
        }
    });
    assert!(has_cycle, "Should detect cycle involving cycle-start/cycle-end");
}

#[test]
fn test_algorithm_dfs_multiple_disjoint_cycles() {
    // Test detection of multiple independent cycles
    let tasks = vec![
        create_task("a1", vec!["a2"]),
        create_task("a2", vec!["a1"]),
        create_task("b1", vec!["b2"]),
        create_task("b2", vec!["b1"]),
    ];

    let errors = validate_dependencies(&tasks);

    // Should detect both cycles
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|e| matches!(
        e,
        ltmatrix::pipeline::generate::ValidationError::CircularDependency { .. }
    )));
}

#[test]
fn test_algorithm_dfs_self_loop() {
    // Test self-loop: A -> A
    let tasks = vec![create_task("a", vec!["a"])];

    let errors = validate_dependencies(&tasks);

    assert_eq!(errors.len(), 1);
    match &errors[0] {
        ltmatrix::pipeline::generate::ValidationError::CircularDependency { chain } => {
            assert_eq!(chain.len(), 1);
            assert_eq!(chain[0], "a");
        }
        _ => panic!("Expected CircularDependency error"),
    }
}

#[test]
fn test_algorithm_dfs_no_false_positives_diamond() {
    // Diamond structure should NOT be detected as a cycle
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D
    let tasks = vec![
        create_task("a", vec![]),
        create_task("b", vec!["a"]),
        create_task("c", vec!["a"]),
        create_task("d", vec!["b", "c"]),
    ];

    let errors = validate_dependencies(&tasks);

    assert!(errors.is_empty(), "Diamond structure should not be detected as a cycle");
}

#[test]
fn test_algorithm_dfs_complex_acyclic_graph() {
    // Complex but valid DAG
    let tasks = vec![
        create_task("root", vec![]),
        create_task("a", vec!["root"]),
        create_task("b", vec!["root"]),
        create_task("c", vec!["a"]),
        create_task("d", vec!["a", "b"]),
        create_task("e", vec!["c", "d"]),
    ];

    let errors = validate_dependencies(&tasks);

    assert!(errors.is_empty(), "Complex DAG should be valid");
}

// ============================================================================
// Combined Error Detection Tests
// ============================================================================

#[test]
fn test_algorithm_combined_missing_and_circular() {
    // Test detection of both missing and circular dependencies

    let tasks = vec![
        create_task("task-1", vec!["missing"]), // Missing dependency
        create_task("a", vec!["b"]), // Part of cycle
        create_task("b", vec!["a"]), // Part of cycle
    ];

    let errors = validate_dependencies(&tasks);

    assert_eq!(errors.len(), 2);
    assert!(errors.iter().any(|e| matches!(
        e,
        ltmatrix::pipeline::generate::ValidationError::MissingDependency { .. }
    )));
    assert!(errors.iter().any(|e| matches!(
        e,
        ltmatrix::pipeline::generate::ValidationError::CircularDependency { .. }
    )));
}

#[test]
fn test_algorithm_cycle_with_missing_references() {
    // Cycle that also references non-existent tasks
    let tasks = vec![
        create_task("a", vec!["b", "missing-1"]),
        create_task("b", vec!["a", "missing-2"]),
    ];

    let errors = validate_dependencies(&tasks);

    // Should detect 2 missing + 1 circular
    assert_eq!(errors.len(), 3);
}

// ============================================================================
// Graph Statistics Calculation Tests
// ============================================================================

#[test]
fn test_algorithm_stats_calculation_linear_chain() {
    // Test statistics for linear chain: A -> B -> C -> D
    let tasks = vec![
        create_task("a", vec![]),
        create_task("b", vec!["a"]),
        create_task("c", vec!["b"]),
        create_task("d", vec!["c"]),
    ];

    let result = validate_dependencies_with_stats(&tasks);

    assert!(result.is_valid);
    assert_eq!(result.stats.total_tasks, 4);
    assert_eq!(result.stats.tasks_with_dependencies, 3);
    assert_eq!(result.stats.total_dependencies, 3);
    assert_eq!(result.stats.max_depth, 3);
    assert_eq!(result.stats.root_tasks, 1);
    assert_eq!(result.stats.leaf_tasks, 1);
    assert!(result.stats.is_dag);
}

#[test]
fn test_algorithm_stats_calculation_diamond() {
    // Test statistics for diamond structure
    let tasks = vec![
        create_task("a", vec![]),
        create_task("b", vec!["a"]),
        create_task("c", vec!["a"]),
        create_task("d", vec!["b", "c"]),
    ];

    let result = validate_dependencies_with_stats(&tasks);

    assert!(result.is_valid);
    assert_eq!(result.stats.total_tasks, 4);
    assert_eq!(result.stats.tasks_with_dependencies, 3);
    assert_eq!(result.stats.total_dependencies, 4);
    assert_eq!(result.stats.max_depth, 2);
    assert_eq!(result.stats.root_tasks, 1);
    assert_eq!(result.stats.leaf_tasks, 1);
}

#[test]
fn test_algorithm_stats_calculation_independent_tasks() {
    // Test statistics for tasks with no dependencies
    let tasks = vec![
        create_task("task-1", vec![]),
        create_task("task-2", vec![]),
        create_task("task-3", vec![]),
    ];

    let result = validate_dependencies_with_stats(&tasks);

    assert!(result.is_valid);
    assert_eq!(result.stats.total_tasks, 3);
    assert_eq!(result.stats.tasks_with_dependencies, 0);
    assert_eq!(result.stats.total_dependencies, 0);
    assert_eq!(result.stats.max_depth, 0);
    assert_eq!(result.stats.root_tasks, 3); // All are roots
    assert_eq!(result.stats.leaf_tasks, 3); // All are leaves
}

#[test]
fn test_algorithm_stats_with_missing_dependencies() {
    // Test statistics include error counts
    let tasks = vec![
        create_task("task-1", vec!["missing-1"]),
        create_task("task-2", vec!["missing-2", "missing-3"]),
    ];

    let result = validate_dependencies_with_stats(&tasks);

    assert!(!result.is_valid);
    assert_eq!(result.stats.missing_dependencies, 3);
    assert!(!result.stats.is_dag);
}

#[test]
fn test_algorithm_stats_with_circular_dependencies() {
    // Test statistics include circular dependency count
    let tasks = vec![
        create_task("a", vec!["b"]),
        create_task("b", vec!["a"]),
        create_task("c", vec!["d"]),
        create_task("d", vec!["c"]),
    ];

    let result = validate_dependencies_with_stats(&tasks);

    assert!(!result.is_valid);
    assert_eq!(result.stats.circular_dependencies, 2);
    assert!(!result.stats.is_dag);
}

// ============================================================================
// Depth Calculation Algorithm Tests
// ============================================================================

#[test]
fn test_algorithm_depth_calculation_single_node() {
    let tasks = vec![create_task("a", vec![])];

    let result = validate_dependencies_with_stats(&tasks);

    assert_eq!(result.stats.max_depth, 0);
}

#[test]
fn test_algorithm_depth_calculation_linear() {
    let tasks = vec![
        create_task("a", vec![]),
        create_task("b", vec!["a"]),
        create_task("c", vec!["b"]),
        create_task("d", vec!["c"]),
    ];

    let result = validate_dependencies_with_stats(&tasks);

    assert_eq!(result.stats.max_depth, 3);
}

#[test]
fn test_algorithm_depth_calculation_branching() {
    let tasks = vec![
        create_task("a", vec![]),
        create_task("b", vec!["a"]),
        create_task("c", vec!["a"]),
        create_task("d", vec!["b"]),
        create_task("e", vec!["c"]),
    ];

    let result = validate_dependencies_with_stats(&tasks);

    assert_eq!(result.stats.max_depth, 2);
}

#[test]
fn test_algorithm_depth_calculation_complex() {
    // Complex graph with varying depths
    let tasks = vec![
        create_task("root", vec![]),
        create_task("branch1", vec!["root"]),
        create_task("branch2", vec!["root"]),
        create_task("leaf1", vec!["branch1"]),
        create_task("leaf2", vec!["branch2"]),
        create_task("deep1", vec!["leaf1"]),
        create_task("deep2", vec!["leaf1", "leaf2"]),
    ];

    let result = validate_dependencies_with_stats(&tasks);

    // Depth should be 3 (root -> branch1 -> leaf1 -> deep1/deep2)
    assert_eq!(result.stats.max_depth, 3);
}

// ============================================================================
// Performance and Scalability Tests
// ============================================================================

#[test]
fn test_algorithm_performance_large_linear_chain() {
    // Test algorithm handles large linear chains efficiently
    let mut tasks = vec![];
    for i in 1..=100 {
        if i == 1 {
            tasks.push(create_task("task-1", vec![]));
        } else {
            tasks.push(create_task(&format!("task-{}", i), vec![&format!("task-{}", i - 1)]));
        }
    }

    let result = validate_dependencies_with_stats(&tasks);

    assert!(result.is_valid);
    assert_eq!(result.stats.total_tasks, 100);
    assert_eq!(result.stats.max_depth, 99);
}

#[test]
fn test_algorithm_performance_many_independent_tasks() {
    // Test with many independent tasks (no dependencies)
    let tasks: Vec<Task> = (1..=500)
        .map(|i| create_task(&format!("task-{}", i), vec![]))
        .collect();

    let result = validate_dependencies_with_stats(&tasks);

    assert!(result.is_valid);
    assert_eq!(result.stats.total_tasks, 500);
    assert_eq!(result.stats.root_tasks, 500);
    assert_eq!(result.stats.leaf_tasks, 500);
}

#[test]
fn test_algorithm_performance_wide_dependency_graph() {
    // Test wide graph: one root with many children
    let mut tasks = vec![create_task("root", vec![])];

    for i in 1..=100 {
        tasks.push(create_task(&format!("child-{}", i), vec!["root"]));
    }

    let result = validate_dependencies_with_stats(&tasks);

    assert!(result.is_valid);
    assert_eq!(result.stats.total_tasks, 101);
    assert_eq!(result.stats.max_depth, 1);
    assert_eq!(result.stats.tasks_with_dependencies, 100);
}

// ============================================================================
// Edge Case Algorithm Tests
// ============================================================================

#[test]
fn test_algorithm_empty_task_list() {
    let tasks: Vec<Task> = vec![];
    let result = validate_dependencies_with_stats(&tasks);

    assert!(result.is_valid);
    assert!(result.errors.is_empty());
    assert_eq!(result.stats.total_tasks, 0);
}

#[test]
fn test_algorithm_single_task_no_dependencies() {
    let tasks = vec![create_task("solo", vec![])];
    let result = validate_dependencies_with_stats(&tasks);

    assert!(result.is_valid);
    assert_eq!(result.stats.total_tasks, 1);
    assert_eq!(result.stats.max_depth, 0);
}

#[test]
fn test_algorithm_single_task_with_missing_dependency() {
    let tasks = vec![create_task("solo", vec!["missing"])];
    let result = validate_dependencies_with_stats(&tasks);

    assert!(!result.is_valid);
    assert_eq!(result.stats.missing_dependencies, 1);
}

#[test]
fn test_algorithm_duplicate_dependencies() {
    // Task with duplicate dependency references
    let mut task = Task::new("test", "Test", "Test task");
    task.depends_on = vec!["a".to_string(), "a".to_string(), "a".to_string()];

    let tasks = vec![create_task("a", vec![]), task];

    let result = validate_dependencies_with_stats(&tasks);

    // Should still be valid (just has duplicate references)
    assert!(result.is_valid);
    assert_eq!(result.stats.total_dependencies, 3); // Counts duplicates
}

// ============================================================================
// Validation Result Structure Tests
// ============================================================================

#[test]
fn test_algorithm_result_structure_valid() {
    let tasks = vec![
        create_task("a", vec![]),
        create_task("b", vec!["a"]),
    ];

    let result = validate_dependencies_with_stats(&tasks);

    // Verify result structure
    assert!(result.is_valid);
    assert!(result.errors.is_empty());
    assert_eq!(result.stats.total_tasks, 2);
    assert!(result.stats.is_dag);
}

#[test]
fn test_algorithm_result_structure_invalid() {
    let tasks = vec![
        create_task("a", vec!["b"]),
        create_task("b", vec!["a"]),
    ];

    let result = validate_dependencies_with_stats(&tasks);

    // Verify result structure
    assert!(!result.is_valid);
    assert!(!result.errors.is_empty());
    assert!(!result.stats.is_dag);
}
