//! Integration tests for dry-run mode

use ltmatrix::dryrun::{run_dry_run, DryRunConfig};
use ltmatrix::models::ExecutionMode;

#[tokio::test]
async fn test_dry_run_basic_functionality() {
    let goal = "build a simple web server";
    let config = DryRunConfig::default();

    let result = run_dry_run(goal, &config).await.unwrap();

    // Verify that tasks were generated
    assert!(!result.tasks.is_empty(), "Should generate tasks");

    // Verify that execution plan was created
    assert!(!result.execution_plan.execution_order.is_empty(), "Should create execution plan");

    // Verify statistics
    assert_eq!(result.statistics.total_tasks, result.tasks.len());
    assert!(result.statistics.execution_depth > 0, "Should have execution depth");
}

#[tokio::test]
async fn test_dry_run_with_complex_goal() {
    let goal = "implement a complete authentication system with OAuth, JWT, and session management";
    let config = DryRunConfig {
        execution_mode: ExecutionMode::Expert,
        ..Default::default()
    };

    let result = run_dry_run(goal, &config).await.unwrap();

    // Verify we got a reasonable number of tasks
    assert!(result.tasks.len() >= 3, "Should generate at least 3 tasks for complex goal");

    // Verify execution plan structure
    assert!(result.execution_plan.max_depth > 1, "Complex tasks should have multiple levels");
    assert!(!result.execution_plan.critical_path.is_empty(), "Should have critical path");
}

#[tokio::test]
async fn test_dry_run_statistics() {
    let goal = "write unit tests for existing code";
    let config = DryRunConfig::default();

    let result = run_dry_run(goal, &config).await.unwrap();

    // Verify statistics are calculated correctly
    assert_eq!(
        result.statistics.total_tasks,
        result.statistics.simple_tasks + result.statistics.moderate_tasks + result.statistics.complex_tasks,
        "Total tasks should equal sum of complexity categories"
    );

    // Verify execution depth matches plan
    assert_eq!(
        result.statistics.execution_depth,
        result.execution_plan.max_depth,
        "Execution depth should match plan"
    );

    // Verify critical path length
    assert_eq!(
        result.statistics.critical_path_length,
        result.execution_plan.critical_path.len(),
        "Critical path length should match plan"
    );
}

#[tokio::test]
async fn test_dry_run_mode_json_output() {
    let goal = "create a REST API endpoint";
    let config = DryRunConfig {
        json_output: true,
        ..Default::default()
    };

    let result = run_dry_run(goal, &config).await.unwrap();

    // The function should still work, just output JSON instead
    assert!(!result.tasks.is_empty());
    assert!(!result.execution_plan.execution_order.is_empty());
}

#[tokio::test]
async fn test_dry_run_parallel_execution_levels() {
    let goal = "build a full-stack application with frontend and backend";
    let config = DryRunConfig::default();

    let result = run_dry_run(goal, &config).await.unwrap();

    // Verify execution levels are present
    assert!(!result.execution_plan.execution_levels.is_empty());

    // Verify that tasks within each level can theoretically run in parallel
    for (level_idx, level) in result.execution_plan.execution_levels.iter().enumerate() {
        println!("Level {} has {} tasks", level_idx + 1, level.len());

        // Check that tasks in the same level don't depend on each other
        for (i, task_i) in level.iter().enumerate() {
            for (j, task_j) in level.iter().enumerate() {
                if i != j {
                    assert!(
                        !task_i.depends_on.contains(&task_j.id),
                        "Tasks in same execution level should not depend on each other"
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn test_dry_run_critical_path_identification() {
    let goal = "implement a database migration system";
    let config = DryRunConfig::default();

    let result = run_dry_run(goal, &config).await.unwrap();

    // Verify critical path is identified
    assert!(!result.execution_plan.critical_path.is_empty());

    // Verify critical path is a subset of execution order
    for task_id in &result.execution_plan.critical_path {
        assert!(
            result.execution_plan.execution_order.contains(task_id),
            "Critical path tasks should be in execution order"
        );
    }

    // Verify critical path is connected (consecutive tasks should have dependencies)
    for window in result.execution_plan.critical_path.windows(2) {
        let first_task = result.tasks.iter().find(|t| &t.id == &window[0]);
        let second_task = result.tasks.iter().find(|t| &t.id == &window[1]);

        if let (Some(first), Some(second)) = (first_task, second_task) {
            // Either the second depends on the first, or they share a common dependency
            let has_direct_dependency = second.depends_on.contains(&first.id);
            let has_common_dependency = !first.depends_on.is_empty()
                && !second.depends_on.is_empty()
                && first.depends_on.iter().any(|dep| second.depends_on.contains(dep));

            assert!(
                has_direct_dependency || has_common_dependency,
                "Critical path tasks should have dependency relationships"
            );
        }
    }
}

#[tokio::test]
async fn test_dry_run_task_complexity_distribution() {
    let goal = "implement a complete feature from design to deployment";
    let config = DryRunConfig {
        execution_mode: ExecutionMode::Standard,
        ..Default::default()
    };

    let result = run_dry_run(goal, &config).await.unwrap();

    // Verify complexity distribution makes sense for a real project
    // Most projects should have a mix of complexity levels
    let has_simple = result.statistics.simple_tasks > 0;
    let has_moderate = result.statistics.moderate_tasks > 0;
    let has_complex = result.statistics.complex_tasks > 0;

    // At least one complexity level should be present
    assert!(
        has_simple || has_moderate || has_complex,
        "Should have at least one complexity level"
    );
}

#[tokio::test]
async fn test_dry_run_execution_order_preserves_dependencies() {
    let goal = "build a multi-module system with dependencies";
    let config = DryRunConfig::default();

    let result = run_dry_run(goal, &config).await.unwrap();

    // Verify that execution order preserves dependencies
    // For each task, all its dependencies should appear before it in the execution order
    for task in &result.tasks {
        for dep_id in &task.depends_on {
            let dep_position = result
                .execution_plan
                .execution_order
                .iter()
                .position(|id| id == dep_id);
            let task_position = result
                .execution_plan
                .execution_order
                .iter()
                .position(|id| id == &task.id);

            if let (Some(dep_pos), Some(task_pos)) = (dep_position, task_position) {
                assert!(
                    dep_pos < task_pos,
                    "Task {} should come after its dependency {}",
                    task.id,
                    dep_id
                );
            }
        }
    }
}
