//! Tests for fixture loading and mock agent functionality
//!
//! This test file verifies that all fixtures can be loaded correctly
//! and that mock agents work as expected.

use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use std::fs;
use std::path::PathBuf;

// =============================================================================
// Fixture Loading Utilities
// =============================================================================

/// Base directory for fixtures
const FIXTURES_DIR: &str = "tests/fixtures";

/// Get the path to the specific fixture file
fn fixture_path(category: &str, name: &str) -> PathBuf {
    PathBuf::from(FIXTURES_DIR).join(category).join(name)
}

/// Load a JSON fixture file
fn load_json_fixture(
    category: &str,
    name: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let path = fixture_path(category, name);
    let content = fs::read_to_string(&path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    Ok(json)
}

/// Load a task list fixture
fn load_tasks_fixture(name: &str) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
    let json = load_json_fixture("tasks", &format!("{}_list.json", name))?;

    let tasks = json
        .get("tasks")
        .and_then(|t| t.as_array())
        .ok_or("No tasks array in fixture")?;

    let mut result = Vec::new();
    for task_json in tasks {
        let task = parse_task_from_json(task_json)?;
        result.push(task);
    }

    Ok(result)
}

/// Load an agent response fixture
fn load_response_fixture(name: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    load_json_fixture("responses", &format!("{}.json", name))
}

/// Parse a task from JSON value
fn parse_task_from_json(json: &serde_json::Value) -> Result<Task, Box<dyn std::error::Error>> {
    let id = json
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing id")?
        .to_string();
    let title = json
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or("Missing title")?
        .to_string();
    let description = json
        .get("description")
        .and_then(|v| v.as_str())
        .ok_or("Missing description")?
        .to_string();

    let mut task = Task::new(id, title, description);

    if let Some(status_str) = json.get("status").and_then(|v| v.as_str()) {
        task.status = match status_str {
            "pending" => TaskStatus::Pending,
            "in_progress" => TaskStatus::InProgress,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "blocked" => TaskStatus::Blocked,
            _ => TaskStatus::Pending,
        };
    }

    if let Some(complexity_str) = json.get("complexity").and_then(|v| v.as_str()) {
        task.complexity = match complexity_str {
            "simple" => TaskComplexity::Simple,
            "moderate" => TaskComplexity::Moderate,
            "complex" => TaskComplexity::Complex,
            _ => TaskComplexity::Moderate,
        };
    }

    if let Some(deps) = json.get("depends_on").and_then(|v| v.as_array()) {
        task.depends_on = deps
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
    }

    if let Some(subtasks) = json.get("subtasks").and_then(|v| v.as_array()) {
        for subtask_json in subtasks {
            let subtask = parse_task_from_json(subtask_json)?;
            task.subtasks.push(subtask);
        }
    }

    if let Some(count) = json.get("retry_count").and_then(|v| v.as_u64()) {
        task.retry_count = count as u32;
    }

    if let Some(session_id) = json.get("session_id").and_then(|v| v.as_str()) {
        task.session_id = Some(session_id.to_string());
    }

    if let Some(error) = json.get("error").and_then(|v| v.as_str()) {
        task.error = Some(error.to_string());
    }

    Ok(task)
}

/// Check if a fixture exists
fn fixture_exists(category: &str, name: &str) -> bool {
    fixture_path(category, name).exists()
}

/// List all fixtures in a category
fn list_fixtures(category: &str) -> Vec<String> {
    let dir = PathBuf::from(FIXTURES_DIR).join(category);
    if !dir.exists() {
        return Vec::new();
    }

    fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    e.path()
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(String::from)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Get the path to a project fixture
fn get_project_path(name: &str) -> PathBuf {
    PathBuf::from(FIXTURES_DIR).join("projects").join(name)
}

// =============================================================================
// Task Fixture Tests
// =============================================================================

#[test]
fn test_load_simple_task_list() {
    let tasks = load_tasks_fixture("simple_task").expect("Failed to load simple_task fixture");

    assert_eq!(tasks.len(), 3, "Should have 3 tasks");

    // Verify linear dependency chain
    assert!(
        tasks[0].depends_on.is_empty(),
        "First task has no dependencies"
    );
    assert_eq!(
        tasks[1].depends_on,
        vec!["task-001"],
        "Second task depends on first"
    );
    assert_eq!(
        tasks[2].depends_on,
        vec!["task-002"],
        "Third task depends on second"
    );
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

    assert!(
        !task_with_subtasks.subtasks.is_empty(),
        "Task should have subtasks"
    );

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
    assert!(
        !tasks_with_errors.is_empty(),
        "Should have tasks with errors"
    );
}

#[test]
fn test_load_circular_dependency() {
    let tasks = load_tasks_fixture("circular_dependency")
        .expect("Failed to load circular_dependency fixture");

    assert_eq!(tasks.len(), 3, "Should have 3 tasks in circular chain");

    // Verify circular dependency exists
    let task_a = tasks
        .iter()
        .find(|t| t.id == "task-a")
        .expect("task-a should exist");
    let task_b = tasks
        .iter()
        .find(|t| t.id == "task-b")
        .expect("task-b should exist");
    let task_c = tasks
        .iter()
        .find(|t| t.id == "task-c")
        .expect("task-c should exist");

    // A depends on C, B depends on A, C depends on B
    assert!(task_a.depends_on.contains(&"task-c".to_string()));
    assert!(task_b.depends_on.contains(&"task-a".to_string()));
    assert!(task_c.depends_on.contains(&"task-b".to_string()));
}

#[test]
fn test_load_diamond_dependency() {
    let tasks = load_tasks_fixture("diamond_dependency")
        .expect("Failed to load diamond_dependency fixture");

    // Find root, left, right, and merge tasks
    let root = tasks
        .iter()
        .find(|t| t.id == "task-root")
        .expect("Root task should exist");
    let left = tasks
        .iter()
        .find(|t| t.id == "task-left")
        .expect("Left task should exist");
    let right = tasks
        .iter()
        .find(|t| t.id == "task-right")
        .expect("Right task should exist");
    let merge = tasks
        .iter()
        .find(|t| t.id == "task-merge")
        .expect("Merge task should exist");

    // Verify diamond structure
    assert!(root.depends_on.is_empty(), "Root has no dependencies");
    assert!(
        left.depends_on.contains(&"task-root".to_string()),
        "Left depends on root"
    );
    assert!(
        right.depends_on.contains(&"task-root".to_string()),
        "Right depends on root"
    );
    assert!(
        merge.depends_on.contains(&"task-left".to_string()),
        "Merge depends on left"
    );
    assert!(
        merge.depends_on.contains(&"task-right".to_string()),
        "Merge depends on right"
    );
}

// =============================================================================
// Response Fixture Tests
// =============================================================================

#[test]
fn test_load_generate_response() {
    let response =
        load_response_fixture("generate_success").expect("Failed to load response fixture");

    assert!(response.get("output").is_some(), "Should have output");
    assert!(
        response.get("is_complete").is_some(),
        "Should have is_complete"
    );
}

#[test]
fn test_load_execute_response() {
    let response =
        load_response_fixture("execute_success").expect("Failed to load response fixture");

    assert!(response.get("output").is_some(), "Should have output");
    assert!(
        response.get("structured_data").is_some(),
        "Should have structured_data"
    );
}

#[test]
fn test_load_verify_success_response() {
    let response =
        load_response_fixture("verify_success").expect("Failed to load response fixture");

    let structured_data = response
        .get("structured_data")
        .expect("Should have structured_data");
    let passed = structured_data
        .get("passed")
        .and_then(|p| p.as_bool())
        .unwrap_or(false);
    assert!(passed, "Verification should pass");
}

#[test]
fn test_load_verify_failure_response() {
    let response =
        load_response_fixture("verify_failure").expect("Failed to load response fixture");

    let structured_data = response
        .get("structured_data")
        .expect("Should have structured_data");
    let passed = structured_data
        .get("passed")
        .and_then(|p: &serde_json::Value| p.as_bool())
        .unwrap_or(true);
    assert!(!passed, "Verification should fail");

    let unmet = structured_data
        .get("unmet_criteria")
        .and_then(|u: &serde_json::Value| u.as_array())
        .expect("Should have unmet_criteria");
    assert!(!unmet.is_empty(), "Should have unmet criteria");
}

// =============================================================================
// Project Fixture Tests
// =============================================================================

#[test]
fn test_rust_project_fixture_exists() {
    let path = get_project_path("rust_basic");
    assert!(path.exists(), "rust_basic project should exist");
    assert!(path.join("Cargo.toml").exists(), "Cargo.toml should exist");
    assert!(path.join("src").exists(), "src directory should exist");
}

#[test]
fn test_node_project_fixture_exists() {
    let path = get_project_path("node_basic");
    assert!(path.exists(), "node_basic project should exist");
    assert!(
        path.join("package.json").exists(),
        "package.json should exist"
    );
}

#[test]
fn test_python_project_fixture_exists() {
    let path = get_project_path("python_basic");
    assert!(path.exists(), "python_basic project should exist");
    assert!(
        path.join("pyproject.toml").exists(),
        "pyproject.toml should exist"
    );
}

// =============================================================================
// Utility Function Tests
// =============================================================================

#[test]
fn test_fixture_path_function() {
    let path = fixture_path("tasks", "test.json");
    assert!(path.ends_with("tests/fixtures/tasks/test.json"));
}

#[test]
fn test_fixture_exists_function() {
    assert!(fixture_exists("tasks", "simple_task_list.json"));
    assert!(!fixture_exists("tasks", "nonexistent.json"));
}

#[test]
fn test_list_fixtures_function() {
    let task_fixtures = list_fixtures("tasks");
    assert!(!task_fixtures.is_empty());
    assert!(task_fixtures.contains(&"simple_task_list.json".to_string()));

    let response_fixtures = list_fixtures("responses");
    assert!(!response_fixtures.is_empty());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_load_nonexistent_fixture() {
    let result = load_tasks_fixture("nonexistent_fixture");
    assert!(result.is_err(), "Should fail to load nonexistent fixture");
}

#[test]
fn test_list_fixtures_empty_category() {
    let fixtures = list_fixtures("nonexistent_category");
    assert!(
        fixtures.is_empty(),
        "Nonexistent category should return empty list"
    );
}
