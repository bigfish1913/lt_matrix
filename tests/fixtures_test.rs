//! Tests for fixture loading and mock agent functionality
//!
//! This test file verifies that all fixtures can be loaded correctly
//! and that mock agents work as expected.

mod fixtures;

use fixtures::*;
use std::path::PathBuf;

// =============================================================================
// Task Fixture Tests
// =============================================================================

#[test]
fn test_load_simple_task_list() {
    let tasks = load_tasks_fixture("simple_task").expect("Failed to load simple_task fixture");

    assert_eq!(tasks.len(), 3, "Should have 3 tasks");

    // Verify linear dependency chain
    assert!(tasks[0].depends_on.is_empty(), "First task has no dependencies");
    assert_eq!(tasks[1].depends_on, vec!["task-001"], "Second task depends on first");
    assert_eq!(tasks[2].depends_on, vec!["task-002"], "Third task depends on second");
}

#[test]
fn test_load_complex_task_list() {
    let tasks = load_tasks_fixture("complex_task").expect("Failed to load complex_task fixture");

    assert!(!tasks.is_empty(), "Should have tasks");

    // Find task with subtasks
    let task_with_subtasks = tasks
        .iter()
        .find(|t| !t.subtasks.is_empty())
        .expect("Should have at least one task with subtasks");

    assert!(!task_with_subtasks.subtasks.is_empty(), "Task should have subtasks");

    // Verify session IDs are preserved
    let task_with_session = tasks
        .iter()
        .find(|t| t.session_id.is_some())
        .expect("Should have task with session ID");

    assert!(task_with_session.session_id.is_some());
}

#[test]
fn test_load_failed_tasks_list() {
    let tasks = load_tasks_fixture("failed_tasks").expect("Failed to load failed_tasks fixture");

    // Check for failed tasks
    let failed_tasks: Vec<_> = tasks.iter().filter(|t| t.is_failed()).collect();
    assert!(!failed_tasks.is_empty(), "Should have failed tasks");

    // Check for tasks with errors
    let tasks_with_errors: Vec<_> = tasks.iter().filter(|t| t.error.is_some()).collect();
    assert!(!tasks_with_errors.is_empty(), "Should have tasks with errors");
}

#[test]
fn test_load_circular_dependency() {
    let tasks = load_tasks_fixture("circular_dependency").expect("Failed to load circular_dependency fixture");

    assert_eq!(tasks.len(), 3, "Should have 3 tasks in circular chain");

    // Verify circular dependency exists
    let task_a = tasks.iter().find(|t| t.id == "task-a").expect("task-a should exist");
    let task_b = tasks.iter().find(|t| t.id == "task-b").expect("task-b should exist");
    let task_c = tasks.iter().find(|t| t.id == "task-c").expect("task-c should exist");

    // A depends on C, B depends on A, C depends on B
    assert!(task_a.depends_on.contains(&"task-c".to_string()));
    assert!(task_b.depends_on.contains(&"task-a".to_string()));
    assert!(task_c.depends_on.contains(&"task-b".to_string()));
}

#[test]
fn test_load_diamond_dependency() {
    let tasks = load_tasks_fixture("diamond_dependency").expect("Failed to load diamond_dependency fixture");

    // Find root, left, right, and merge tasks
    let root = tasks.iter().find(|t| t.id == "task-root").expect("Root task should exist");
    let left = tasks.iter().find(|t| t.id == "task-left").expect("Left task should exist");
    let right = tasks.iter().find(|t| t.id == "task-right").expect("Right task should exist");
    let merge = tasks.iter().find(|t| t.id == "task-merge").expect("Merge task should exist");

    // Verify diamond structure
    assert!(root.depends_on.is_empty(), "Root has no dependencies");
    assert!(left.depends_on.contains(&"task-root".to_string()), "Left depends on root");
    assert!(right.depends_on.contains(&"task-root".to_string()), "Right depends on root");
    assert!(merge.depends_on.contains(&"task-left".to_string()), "Merge depends on left");
    assert!(merge.depends_on.contains(&"task-right".to_string()), "Merge depends on right");
}

// =============================================================================
// Response Fixture Tests
// =============================================================================

#[test]
fn test_load_generate_success_response() {
    let response = load_response_fixture("generate_success").expect("Failed to load generate_success");

    assert!(response.get("output").is_some(), "Should have output field");
    assert!(response.get("is_complete").is_some(), "Should have is_complete field");

    let is_complete = response.get("is_complete").and_then(|v| v.as_bool()).unwrap_or(false);
    assert!(is_complete, "Response should be complete");

    let structured_data = response.get("structured_data").expect("Should have structured_data");
    let tasks = structured_data.get("tasks").and_then(|t| t.as_array()).expect("Should have tasks array");
    assert!(!tasks.is_empty(), "Should have tasks in response");
}

#[test]
fn test_load_execute_success_response() {
    let response = load_response_fixture("execute_success").expect("Failed to load execute_success");

    assert!(response.get("output").is_some(), "Should have output field");

    let structured_data = response.get("structured_data").expect("Should have structured_data");
    let files_modified = structured_data.get("files_modified").and_then(|f| f.as_array());
    assert!(files_modified.is_some(), "Should have files_modified");
}

#[test]
fn test_load_error_timeout_response() {
    let response = load_response_fixture("error_timeout").expect("Failed to load error_timeout");

    let is_complete = response.get("is_complete").and_then(|v| v.as_bool()).unwrap_or(true);
    assert!(!is_complete, "Error response should not be complete");

    let error = response.get("error").and_then(|e| e.as_str());
    assert!(error.is_some(), "Should have error message");
}

#[test]
fn test_load_verify_success_response() {
    let response = load_response_fixture("verify_success").expect("Failed to load verify_success");

    let structured_data = response.get("structured_data").expect("Should have structured_data");
    let passed = structured_data.get("passed").and_then(|p| p.as_bool()).unwrap_or(false);
    assert!(passed, "Verification should pass");
}

#[test]
fn test_load_verify_failure_response() {
    let response = load_response_fixture("verify_failure").expect("Failed to load verify_failure");

    let structured_data = response.get("structured_data").expect("Should have structured_data");
    let passed = structured_data.get("passed").and_then(|p| p.as_bool()).unwrap_or(true);
    assert!(!passed, "Verification should fail");

    let unmet_criteria = structured_data.get("unmet_criteria").and_then(|u| u.as_array());
    assert!(unmet_criteria.is_some(), "Should have unmet criteria");
    assert!(!unmet_criteria.unwrap().is_empty(), "Should have at least one unmet criterion");
}

// =============================================================================
// Project Fixture Tests
// =============================================================================

#[test]
fn test_rust_project_fixture_exists() {
    let path = get_project_path("rust_basic");
    assert!(path.exists(), "Rust project fixture should exist");

    let cargo_toml = path.join("Cargo.toml");
    assert!(cargo_toml.exists(), "Cargo.toml should exist");

    let src_main = path.join("src/main.rs");
    assert!(src_main.exists(), "src/main.rs should exist");
}

#[test]
fn test_node_project_fixture_exists() {
    let path = get_project_path("node_basic");
    assert!(path.exists(), "Node project fixture should exist");

    let package_json = path.join("package.json");
    assert!(package_json.exists(), "package.json should exist");

    let index_js = path.join("index.js");
    assert!(index_js.exists(), "index.js should exist");
}

#[test]
fn test_python_project_fixture_exists() {
    let path = get_project_path("python_basic");
    assert!(path.exists(), "Python project fixture should exist");

    let pyproject_toml = path.join("pyproject.toml");
    assert!(pyproject_toml.exists(), "pyproject.toml should exist");
}

// =============================================================================
// Utility Function Tests
// =============================================================================

#[test]
fn test_fixture_path_generation() {
    let path = fixture_path("tasks", "test.json");
    assert!(path.to_str().unwrap().contains("tests/fixtures/tasks/test.json"));
}

#[test]
fn test_fixture_exists_check() {
    assert!(fixture_exists("tasks", "simple_task_list.json"));
    assert!(fixture_exists("tasks", "complex_task_list.json"));
    assert!(!fixture_exists("tasks", "nonexistent.json"));
}

#[test]
fn test_list_fixtures() {
    let task_fixtures = list_fixtures("tasks");
    assert!(!task_fixtures.is_empty(), "Should have task fixtures");
    assert!(task_fixtures.contains(&"simple_task_list.json".to_string()));

    let response_fixtures = list_fixtures("responses");
    assert!(!response_fixtures.is_empty(), "Should have response fixtures");
}

// =============================================================================
// Task Status Tests
// =============================================================================

#[test]
fn test_task_status_from_fixture() {
    let tasks = load_tasks_fixture("complex_task").expect("Failed to load fixture");

    // Check various task statuses
    let completed: Vec<_> = tasks.iter().filter(|t| t.is_completed()).collect();
    let failed: Vec<_> = tasks.iter().filter(|t| t.is_failed()).collect();

    // At least some tasks should be completed based on the fixture
    assert!(!completed.is_empty() || !failed.is_empty(), "Should have tasks in terminal states");
}

// =============================================================================
// Dependency Resolution Tests
// =============================================================================

#[test]
fn test_linear_dependency_chain() {
    let tasks = load_tasks_fixture("simple_task").expect("Failed to load fixture");

    // Create a completed set
    let mut completed = std::collections::HashSet::new();

    // First task should be executable
    assert!(tasks[0].can_execute(&completed), "First task should be executable with no completed");

    // Complete first task
    completed.insert(tasks[0].id.clone());

    // Second task should now be executable
    assert!(tasks[1].can_execute(&completed), "Second task should be executable after first completes");

    // Complete second task
    completed.insert(tasks[1].id.clone());

    // Third task should now be executable
    assert!(tasks[2].can_execute(&completed), "Third task should be executable after first two complete");
}

#[test]
fn test_parallel_execution_diamond() {
    let tasks = load_tasks_fixture("diamond_dependency").expect("Failed to load fixture");

    let mut completed = std::collections::HashSet::new();

    // Root can execute
    let root = tasks.iter().find(|t| t.id == "task-root").unwrap();
    assert!(root.can_execute(&completed));

    // Complete root
    completed.insert("task-root".to_string());

    // Both left and right can execute in parallel
    let left = tasks.iter().find(|t| t.id == "task-left").unwrap();
    let right = tasks.iter().find(|t| t.id == "task-right").unwrap();

    assert!(left.can_execute(&completed), "Left should be executable after root");
    assert!(right.can_execute(&completed), "Right should be executable after root");

    // But merge cannot execute yet
    let merge = tasks.iter().find(|t| t.id == "task-merge").unwrap();
    assert!(!merge.can_execute(&completed), "Merge should wait for both branches");

    // Complete left
    completed.insert("task-left".to_string());
    assert!(!merge.can_execute(&completed), "Merge should still wait for right");

    // Complete right
    completed.insert("task-right".to_string());
    assert!(merge.can_execute(&completed), "Merge can execute after both branches");
}
