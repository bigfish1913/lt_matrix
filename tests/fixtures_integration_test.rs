//! Integration tests for fixtures and mocks
//!
//! This test file verifies the complete fixtures system works correctly
//! and tests the mock agent implementations in integration scenarios.

use std::path::PathBuf;
use std::fs;
use ltmatrix::models::{Task, TaskStatus, TaskComplexity};
use ltmatrix::agent::backend::{
    AgentBackend, AgentConfig, AgentError, ExecutionConfig,
};

// Include the local mocks module
#[path = "fixtures/mocks/mod.rs"]
mod mocks;

use mocks::{MockAgent, MockResponse, MockAgentBuilder, FailingMockAgent, DelayedMockAgent};

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
fn load_json_fixture(category: &str, name: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
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
    let id = json.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?.to_string();
    let title = json.get("title").and_then(|v| v.as_str()).ok_or("Missing title")?.to_string();
    let description = json.get("description").and_then(|v| v.as_str()).ok_or("Missing description")?.to_string();

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
        task.depends_on = deps.iter().filter_map(|v| v.as_str().map(String::from)).collect();
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
                .filter_map(|e| e.path().file_name().and_then(|n| n.to_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Get the path to a project fixture
fn get_project_path(name: &str) -> PathBuf {
    PathBuf::from(FIXTURES_DIR).join("projects").join(name)
}

// =============================================================================
// Error Response Fixture Tests
// =============================================================================

#[test]
fn test_load_error_timeout_response() {
    let response = load_response_fixture("error_timeout").expect("Failed to load response fixture");

    assert!(response.get("output").is_some(), "Should have output");
    assert!(response.get("is_complete").is_some(), "Should have is_complete");

    let is_complete = response.get("is_complete").and_then(|v| v.as_bool()).unwrap();
    assert!(!is_complete, "Timeout response should not be complete");

    let error = response.get("error").and_then(|v| v.as_str());
    assert!(error.is_some(), "Should have error message");
}

// =============================================================================
// Project Fixture Content Validation Tests
// =============================================================================

#[test]
fn test_rust_project_cargo_toml_valid() {
    let path = get_project_path("rust_basic");
    let cargo_toml_path = path.join("Cargo.toml");

    let content = fs::read_to_string(&cargo_toml_path).expect("Failed to read Cargo.toml");

    // Verify it's valid TOML
    let result: Result<toml::Value, _> = toml::from_str(&content);
    assert!(result.is_ok(), "Cargo.toml should be valid TOML");

    let parsed = result.unwrap();
    assert_eq!(parsed["package"]["name"].as_str().unwrap(), "test-project");
}

#[test]
fn test_node_project_package_json_valid() {
    let path = get_project_path("node_basic");
    let package_json_path = path.join("package.json");

    let content = fs::read_to_string(&package_json_path).expect("Failed to read package.json");

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["name"].as_str().unwrap(), "test-project");
    assert_eq!(parsed["main"].as_str().unwrap(), "index.js");
}

#[test]
fn test_python_project_pyproject_valid() {
    let path = get_project_path("python_basic");
    let pyproject_path = path.join("pyproject.toml");

    let content = fs::read_to_string(&pyproject_path).expect("Failed to read pyproject.toml");

    // Verify it's valid TOML
    let result: Result<toml::Value, _> = toml::from_str(&content);
    assert!(result.is_ok(), "pyproject.toml should be valid TOML");

    let parsed = result.unwrap();
    assert_eq!(parsed["project"]["name"].as_str().unwrap(), "test-project");
}

// =============================================================================
// Mock Agent Integration Tests
// =============================================================================

#[tokio::test]
async fn test_mock_agent_in_pipeline_scenario() {
    let mut mock = MockAgent::new();

    // Simulate a typical pipeline flow
    mock.set_response("generate", MockResponse::success("Tasks generated successfully"));
    mock.set_response("execute", MockResponse::success("Task executed"));
    mock.set_response("verify", MockResponse::success_with_data(
        "Verification passed",
        serde_json::json!({ "passed": true }),
    ));

    let config = ExecutionConfig::default();

    // Test generate
    let gen_response = mock.execute("Generate tasks for feature X", &config).await;
    assert!(gen_response.is_ok());
    assert!(gen_response.unwrap().is_complete);

    // Test execute
    let exec_response = mock.execute("Execute task-001", &config).await;
    assert!(exec_response.is_ok());

    // Test verify
    let verify_response = mock.execute("Verify task-001 completion", &config).await;
    assert!(verify_response.is_ok());

    // Verify call recording
    assert_eq!(mock.call_count(), 3);
    let calls = mock.get_calls();
    assert_eq!(calls[0].prompt, "Generate tasks for feature X");
}

#[tokio::test]
async fn test_failing_mock_agent_in_error_scenario() {
    let mock = FailingMockAgent::execution_failed("Simulated failure");

    let config = ExecutionConfig::default();
    let result = mock.execute("any prompt", &config).await;

    assert!(result.is_err());
    assert!(mock.health_check().await.is_ok());
    assert!(!mock.health_check().await.unwrap());
}

#[tokio::test]
async fn test_delayed_mock_agent_timing() {
    let mut delayed_mock = DelayedMockAgent::from_millis(100);
    delayed_mock.set_response("execute", MockResponse::success("Delayed response"));

    let config = ExecutionConfig::default();

    let start = std::time::Instant::now();
    let result = delayed_mock.execute("test", &config).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert!(elapsed >= std::time::Duration::from_millis(100));
}

#[tokio::test]
async fn test_mock_agent_builder_fluent_api() {
    let mock = MockAgentBuilder::new()
        .name("custom-agent")
        .model("custom-model")
        .generate_response(MockResponse::success("Generated"))
        .execute_response(MockResponse::success("Executed"))
        .verify_response(MockResponse::success("Verified"))
        .healthy(true)
        .build();

    assert_eq!(mock.agent().name, "custom-agent");
    assert_eq!(mock.agent().model, "custom-model");

    let config = ExecutionConfig::default();

    let gen = mock.execute("generate", &config).await.unwrap();
    assert!(gen.is_complete);

    mock.clear_calls();
    assert_eq!(mock.call_count(), 0);
}

// =============================================================================
// Mock Response Edge Cases
// =============================================================================

#[tokio::test]
async fn test_mock_response_sequence_progression() {
    let responses = vec![
        MockResponse::success("First response"),
        MockResponse::success("Second response"),
        MockResponse::failure("Final failure"),
    ];

    let mut mock = MockAgent::new();
    mock.set_response("execute", MockResponse::Sequence { responses });

    let config = ExecutionConfig::default();

    // First call
    let r1 = mock.execute("test", &config).await.unwrap();
    assert_eq!(r1.output, "First response");

    // Second call
    let r2 = mock.execute("test", &config).await.unwrap();
    assert_eq!(r2.output, "Second response");

    // Third call should fail
    let r3 = mock.execute("test", &config).await;
    assert!(r3.is_err());
}

#[tokio::test]
async fn test_mock_response_delayed_wrapper() {
    let inner_response = MockResponse::success("Inner response");
    let delayed = MockResponse::delayed(inner_response, 50);

    let mut mock = MockAgent::new();
    mock.set_response("execute", delayed);

    let config = ExecutionConfig::default();

    let start = std::time::Instant::now();
    let result = mock.execute("test", &config).await.unwrap();
    let elapsed = start.elapsed();

    assert!(elapsed >= std::time::Duration::from_millis(50));
    assert_eq!(result.output, "Inner response");
}

// =============================================================================
// Task Fixture Edge Cases
// =============================================================================

#[test]
fn test_load_empty_tasks_fixture() {
    // Create a temporary empty fixture
    let temp_dir = PathBuf::from(FIXTURES_DIR).join("tasks");
    let temp_file = temp_dir.join("empty_list.json");

    let original_content = if temp_file.exists() {
        Some(fs::read_to_string(&temp_file).unwrap())
    } else {
        None
    };

    // Write empty tasks array
    fs::write(&temp_file, br#"{"tasks": []}"#).unwrap();

    let result = load_tasks_fixture("empty");

    // Clean up
    if let Some(content) = original_content {
        fs::write(&temp_file, content).unwrap();
    } else {
        fs::remove_file(&temp_file).unwrap();
    }

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_load_malformed_task_fixture() {
    // Create a temporary malformed fixture
    let temp_dir = PathBuf::from(FIXTURES_DIR).join("tasks");
    let temp_file = temp_dir.join("malformed_list.json");

    let original_content = if temp_file.exists() {
        Some(fs::read_to_string(&temp_file).unwrap())
    } else {
        None
    };

    // Write malformed JSON
    fs::write(&temp_file, "{ invalid json }").unwrap();

    let result = load_tasks_fixture("malformed");

    // Clean up
    if let Some(content) = original_content {
        fs::write(&temp_file, content).unwrap();
    } else {
        fs::remove_file(&temp_file).unwrap();
    }

    assert!(result.is_err());
}

// =============================================================================
// Project Fixture Structure Validation
// =============================================================================

#[test]
fn test_rust_project_has_source_directory() {
    let path = get_project_path("rust_basic");
    let src_path = path.join("src");

    assert!(src_path.exists(), "Rust project should have src directory");
    assert!(src_path.join("main.rs").exists(), "Should have main.rs");
}

#[test]
fn test_node_project_has_entry_point() {
    let path = get_project_path("node_basic");

    assert!(path.join("index.js").exists(), "Node project should have index.js");
}

#[test]
fn test_python_project_has_pyproject() {
    let path = get_project_path("python_basic");

    assert!(path.join("pyproject.toml").exists(), "Python project should have pyproject.toml");
}

// =============================================================================
// Mock Agent Call Recording Tests
// =============================================================================

#[tokio::test]
async fn test_mock_agent_records_session_id() {
    use ltmatrix::agent::backend::AgentSession;
    use chrono::{Utc, Duration};

    struct TestSession {
        id: String,
        created_at: chrono::DateTime<Utc>,
        last_accessed: chrono::DateTime<Utc>,
        reuse_count: u32,
    }

    impl AgentSession for TestSession {
        fn session_id(&self) -> &str {
            &self.id
        }

        fn agent_name(&self) -> &str {
            "test-agent"
        }

        fn model(&self) -> &str {
            "test-model"
        }

        fn created_at(&self) -> chrono::DateTime<Utc> {
            self.created_at
        }

        fn last_accessed(&self) -> chrono::DateTime<Utc> {
            self.last_accessed
        }

        fn reuse_count(&self) -> u32 {
            self.reuse_count
        }

        fn mark_accessed(&mut self) {
            self.last_accessed = Utc::now();
            self.reuse_count += 1;
        }

        fn is_stale(&self) -> bool {
            self.last_accessed < Utc::now() - Duration::hours(1)
        }
    }

    let mut mock = MockAgent::new();
    mock.set_response("execute_with_session", MockResponse::success("Session response"));

    let config = ExecutionConfig::default();
    let now = Utc::now();
    let session = TestSession {
        id: "test-session-123".to_string(),
        created_at: now,
        last_accessed: now,
        reuse_count: 0,
    };

    let _ = mock.execute_with_session("test prompt", &config, &session).await;

    let calls = mock.get_calls();
    let session_call = calls.iter().find(|c| c.session_id.is_some()).expect("Should have call with session");

    assert_eq!(session_call.session_id, Some("test-session-123".to_string()));
}

#[tokio::test]
async fn test_mock_agent_clear_calls() {
    let mock = MockAgent::new();

    let config = ExecutionConfig::default();
    let _ = mock.execute("first", &config).await;
    let _ = mock.execute("second", &config).await;

    assert_eq!(mock.call_count(), 2);

    mock.clear_calls();
    assert_eq!(mock.call_count(), 0);

    let _ = mock.execute("third", &config).await;
    assert_eq!(mock.call_count(), 1);
}
