//! Fixture Loading Utilities for Tests
//!
//! This module provides utilities for loading test fixtures from the
//! tests/fixtures directory. It simplifies accessing task lists, agent
//! responses, and project structures in test code.
//!
//! # Usage
//!
//! ```rust,ignore
//! use ltmatrix::testing::fixtures::{load_tasks, load_response, get_project_path};
//!
//! // Load a task list fixture
//! let tasks = load_tasks("simple_task_list").unwrap();
//!
//! // Load an agent response fixture
//! let response = load_response("generate_success").unwrap();
//!
//! // Get path to a project fixture
//! let rust_project = get_project_path("rust_basic");
//! ```

use std::path::{Path, PathBuf};
use std::fs;

use ltmatrix::models::{Task, TaskStatus, TaskComplexity};

/// Base directory for fixtures
pub const FIXTURES_DIR: &str = "tests/fixtures";

/// Get the path to the fixtures directory
pub fn fixtures_path() -> PathBuf {
    PathBuf::from(FIXTURES_DIR)
}

/// Get the path to a specific fixture file
pub fn fixture_path(category: &str, name: &str) -> PathBuf {
    fixtures_path().join(category).join(name)
}

/// Get the path to a project fixture
pub fn get_project_path(project_name: &str) -> PathBuf {
    fixtures_path().join("projects").join(project_name)
}

/// Load a JSON fixture file
pub fn load_json_fixture(category: &str, name: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let path = fixture_path(category, &format!("{}.json", name));
    let content = fs::read_to_string(&path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    Ok(json)
}

/// Load a task list fixture
pub fn load_tasks_fixture(name: &str) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
    let json = load_json_fixture("tasks", &format!("{}_list", name))?;

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

/// Parse a task from JSON value
fn parse_task_from_json(json: &serde_json::Value) -> Result<Task, Box<dyn std::error::Error>> {
    let id = json.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?.to_string();
    let title = json.get("title").and_then(|v| v.as_str()).ok_or("Missing title")?.to_string();
    let description = json.get("description").and_then(|v| v.as_str()).ok_or("Missing description")?.to_string();

    let mut task = Task::new(id, title, description);

    // Parse status
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

    // Parse complexity
    if let Some(complexity_str) = json.get("complexity").and_then(|v| v.as_str()) {
        task.complexity = match complexity_str {
            "simple" => TaskComplexity::Simple,
            "moderate" => TaskComplexity::Moderate,
            "complex" => TaskComplexity::Complex,
            _ => TaskComplexity::Moderate,
        };
    }

    // Parse dependencies
    if let Some(deps) = json.get("depends_on").and_then(|v| v.as_array()) {
        task.depends_on = deps
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
    }

    // Parse subtasks recursively
    if let Some(subtasks) = json.get("subtasks").and_then(|v| v.as_array()) {
        for subtask_json in subtasks {
            let subtask = parse_task_from_json(subtask_json)?;
            task.subtasks.push(subtask);
        }
    }

    // Parse retry count
    if let Some(count) = json.get("retry_count").and_then(|v| v.as_u64()) {
        task.retry_count = count as u32;
    }

    // Parse session ID
    if let Some(session_id) = json.get("session_id").and_then(|v| v.as_str()) {
        task.session_id = Some(session_id.to_string());
    }

    // Parse parent session ID
    if let Some(parent_session_id) = json.get("parent_session_id").and_then(|v| v.as_str()) {
        task.parent_session_id = Some(parent_session_id.to_string());
    }

    // Parse error
    if let Some(error) = json.get("error").and_then(|v| v.as_str()) {
        task.error = Some(error.to_string());
    }

    Ok(task)
}

/// Load an agent response fixture
pub fn load_response_fixture(name: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    load_json_fixture("responses", name)
}

/// Load raw fixture content as string
pub fn load_fixture_content(category: &str, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = fixture_path(category, name);
    let content = fs::read_to_string(&path)?;
    Ok(content)
}

/// Check if a fixture exists
pub fn fixture_exists(category: &str, name: &str) -> bool {
    fixture_path(category, name).exists()
}

/// List all fixtures in a category
pub fn list_fixtures(category: &str) -> Vec<String> {
    let dir = fixtures_path().join(category);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixtures_path() {
        let path = fixtures_path();
        assert!(path.ends_with("tests/fixtures"));
    }

    #[test]
    fn test_fixture_path() {
        let path = fixture_path("tasks", "test.json");
        assert!(path.ends_with("tests/fixtures/tasks/test.json"));
    }

    #[test]
    fn test_get_project_path() {
        let path = get_project_path("rust_basic");
        assert!(path.ends_with("tests/fixtures/projects/rust_basic"));
    }

    #[test]
    fn test_load_tasks_fixture() {
        let result = load_tasks_fixture("simple_task");
        assert!(result.is_ok());

        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 3);

        assert_eq!(tasks[0].id, "task-001");
        assert_eq!(tasks[1].depends_on, vec!["task-001"]);
        assert_eq!(tasks[2].depends_on, vec!["task-002"]);
    }

    #[test]
    fn test_load_response_fixture() {
        let result = load_response_fixture("generate_success");
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.get("output").is_some());
        assert!(response.get("is_complete").is_some());
    }

    #[test]
    fn test_fixture_exists() {
        assert!(fixture_exists("tasks", "simple_task_list.json"));
        assert!(!fixture_exists("tasks", "nonexistent.json"));
    }

    #[test]
    fn test_list_fixtures() {
        let fixtures = list_fixtures("tasks");
        assert!(!fixtures.is_empty());
        assert!(fixtures.contains(&"simple_task_list.json".to_string()));
    }

    #[test]
    fn test_load_complex_tasks() {
        let result = load_tasks_fixture("complex_task");
        assert!(result.is_ok());

        let tasks = result.unwrap();
        assert!(!tasks.is_empty());

        // Check for subtasks
        let task_002 = tasks.iter().find(|t| t.id == "task-002").expect("task-002 should exist");
        assert!(!task_002.subtasks.is_empty());
    }

    #[test]
    fn test_load_circular_dependency() {
        let result = load_tasks_fixture("circular_dependency");
        assert!(result.is_ok());

        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 3);

        // Verify circular dependency: A -> C -> B -> A
        let task_a = tasks.iter().find(|t| t.id == "task-a").expect("task-a should exist");
        assert!(task_a.depends_on.contains(&"task-c".to_string()));
    }
}
