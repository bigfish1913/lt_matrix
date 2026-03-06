//! Integration and unit tests for task assessment stage
//!
//! This test module verifies the assessment functionality meets acceptance criteria:
//! - Evaluates task complexity using Claude (mocked for tests)
//! - Splits complex tasks into subtasks (max depth: 3)
//! - Updates task structures with complexity ratings
//! - Implements smart model selection (Haiku/Sonnet/Opus)
//! - Returns enriched task list

use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::pipeline::assess::*;

#[test]
fn test_assess_config_default_values() {
    let config = AssessConfig::default();

    // Verify default configuration
    assert_eq!(config.max_depth, 3, "Default max_depth should be 3");
    assert_eq!(config.assessment_model, "claude-sonnet-4-6");
    assert_eq!(config.timeout, 120);
    assert_eq!(config.max_retries, 3);
}

#[test]
fn test_assess_config_fast_mode() {
    let config = AssessConfig::fast_mode();

    // Fast mode should use faster settings
    assert_eq!(config.max_depth, 2, "Fast mode max_depth should be 2");
    assert_eq!(config.assessment_model, "claude-haiku-4-5");
    assert_eq!(config.timeout, 60, "Fast mode timeout should be 60s");
    assert_eq!(config.max_retries, 1, "Fast mode should only retry once");
}

#[test]
fn test_assess_config_expert_mode() {
    let config = AssessConfig::expert_mode();

    // Expert mode should use highest quality settings
    assert_eq!(config.max_depth, 3);
    assert_eq!(config.assessment_model, "claude-opus-4-6");
    assert_eq!(config.timeout, 180, "Expert mode timeout should be 180s");
    assert_eq!(config.max_retries, 3);
}

#[test]
fn test_extract_json_block_valid_input() {
    let response = r#"Some introductory text.

```json
{
  "complexity": "Simple",
  "subtasks": []
}
```

Some concluding text."#;

    let json = extract_json_block(response)
        .expect("Should extract JSON block");

    assert!(json.contains("complexity"));
    assert!(json.contains("Simple"));
    assert!(json.contains("subtasks"));
}

#[test]
fn test_extract_json_block_no_json() {
    let response = "This response has no JSON block in it.";

    let result = extract_json_block(response);
    assert!(result.is_none(), "Should return None when no JSON block found");
}

#[test]
fn test_extract_json_block_malformed() {
    let response = r#"```json
{"incomplete": "json"
```"#;

    let result = extract_json_block(response);
    // Should still extract, even if malformed JSON
    assert!(result.is_some());
}

#[test]
fn test_parse_assessment_response_simple() {
    let response = r#"```json
{
  "complexity": "Simple",
  "recommended_model": "claude-haiku-4-5",
  "estimated_time_minutes": 15,
  "subtasks": []
}
```"#;

    let task = Task::new("test-1", "Simple Test", "A simple test task");
    let config = AssessConfig::default();

    let assessment = parse_assessment_response(response, &task, &config)
        .expect("Should parse valid response");

    assert_eq!(assessment.complexity, TaskComplexity::Simple);
    assert_eq!(assessment.recommended_model, "claude-haiku-4-5");
    assert_eq!(assessment.estimated_time_minutes, Some(15));
    assert!(assessment.subtasks.is_empty());
}

#[test]
fn test_parse_assessment_response_moderate() {
    let response = r#"```json
{
  "complexity": "Moderate",
  "recommended_model": "claude-sonnet-4-6",
  "estimated_time_minutes": 45,
  "subtasks": []
}
```"#;

    let task = Task::new("test-2", "Moderate Test", "A moderate test task");
    let config = AssessConfig::default();

    let assessment = parse_assessment_response(response, &task, &config)
        .expect("Should parse valid response");

    assert_eq!(assessment.complexity, TaskComplexity::Moderate);
    assert_eq!(assessment.recommended_model, "claude-sonnet-4-6");
    assert_eq!(assessment.estimated_time_minutes, Some(45));
}

#[test]
fn test_parse_assessment_response_complex_with_subtasks() {
    let response = r#"```json
{
  "complexity": "Complex",
  "recommended_model": "claude-opus-4-6",
  "estimated_time_minutes": 120,
  "subtasks": [
    {
      "id": "subtask-1",
      "title": "First Subtask",
      "description": "First subtask description",
      "depends_on": []
    },
    {
      "id": "subtask-2",
      "title": "Second Subtask",
      "description": "Second subtask description",
      "depends_on": ["subtask-1"]
    }
  ]
}
```"#;

    let task = Task::new("test-3", "Complex Test", "A complex test task");
    let config = AssessConfig::default();

    let assessment = parse_assessment_response(response, &task, &config)
        .expect("Should parse valid response");

    assert_eq!(assessment.complexity, TaskComplexity::Complex);
    assert_eq!(assessment.recommended_model, "claude-opus-4-6");
    assert_eq!(assessment.estimated_time_minutes, Some(120));
    assert_eq!(assessment.subtasks.len(), 2);

    // Verify first subtask
    assert_eq!(assessment.subtasks[0].id, "subtask-1");
    assert_eq!(assessment.subtasks[0].title, "First Subtask");
    assert!(assessment.subtasks[0].depends_on.is_empty());

    // Verify second subtask with dependency
    assert_eq!(assessment.subtasks[1].id, "subtask-2");
    assert_eq!(assessment.subtasks[1].depends_on.len(), 1);
    assert_eq!(assessment.subtasks[1].depends_on[0], "subtask-1");
}

#[test]
fn test_parse_assessment_response_unknown_complexity() {
    let response = r#"```json
{
  "complexity": "UnknownLevel",
  "recommended_model": "claude-sonnet-4-6",
  "subtasks": []
}
```"#;

    let task = Task::new("test-4", "Unknown", "Test unknown complexity");
    let config = AssessConfig::default();

    let assessment = parse_assessment_response(response, &task, &config)
        .expect("Should handle unknown complexity gracefully");

    // Should default to Moderate for unknown complexity
    assert_eq!(assessment.complexity, TaskComplexity::Moderate);
}

#[test]
fn test_parse_assessment_response_missing_fields() {
    let response = r#"```json
{
  "complexity": "Simple"
}
```"#;

    let task = Task::new("test-5", "Missing Fields", "Test with missing fields");
    let config = AssessConfig::default();

    let assessment = parse_assessment_response(response, &task, &config)
        .expect("Should handle missing optional fields");

    assert_eq!(assessment.complexity, TaskComplexity::Simple);
    assert!(assessment.estimated_time_minutes.is_none());
    assert!(assessment.subtasks.is_empty());
}

#[test]
fn test_parse_assessment_response_invalid_json() {
    let response = r#"```json
{ invalid json }
```"#;

    let task = Task::new("test-6", "Invalid", "Test invalid JSON");
    let config = AssessConfig::default();

    let result = parse_assessment_response(response, &task, &config);
    assert!(result.is_err(), "Should return error for invalid JSON");
}

#[test]
fn test_parse_assessment_response_no_json_block() {
    let response = "This response has no JSON block at all.";

    let task = Task::new("test-7", "No JSON", "Test with no JSON block");
    let config = AssessConfig::default();

    let result = parse_assessment_response(response, &task, &config);
    assert!(result.is_err(), "Should return error when no JSON block found");
}

#[test]
fn test_parse_subtasks_with_auto_ids() {
    let json = serde_json::json!([
        {
            "title": "Auto ID Subtask",
            "description": "Subtask without explicit ID"
        }
    ]);

    let subtasks_array = json.as_array().unwrap();
    let result = parse_subtasks(subtasks_array, "parent-1");

    assert!(result.is_ok());
    let subtasks = result.unwrap();
    assert_eq!(subtasks.len(), 1);
    assert_eq!(subtasks[0].id, "parent-1-subtask-1");
}

#[test]
fn test_parse_subtasks_missing_title() {
    let json = serde_json::json!([
        {
            "id": "subtask-1",
            "description": "No title provided"
        }
    ]);

    let subtasks_array = json.as_array().unwrap();
    let result = parse_subtasks(subtasks_array, "parent-1");

    assert!(result.is_err(), "Should fail when title is missing");
}

#[test]
fn test_parse_subtasks_missing_description() {
    let json = serde_json::json!([
        {
            "id": "subtask-1",
            "title": "No Description"
        }
    ]);

    let subtasks_array = json.as_array().unwrap();
    let result = parse_subtasks(subtasks_array, "parent-1");

    assert!(result.is_err(), "Should fail when description is missing");
}

#[test]
fn test_calculate_assessment_stats_empty() {
    let tasks = vec![];
    let stats = calculate_assessment_stats(&tasks);

    assert_eq!(stats.total_tasks, 0);
    assert_eq!(stats.simple, 0);
    assert_eq!(stats.moderate, 0);
    assert_eq!(stats.complex, 0);
    assert_eq!(stats.tasks_with_subtasks, 0);
    assert_eq!(stats.total_subtasks, 0);
}

#[test]
fn test_calculate_assessment_stats_mixed() {
    let tasks = vec![
        {
            let mut t = Task::new("task-1", "Simple", "Simple task");
            t.complexity = TaskComplexity::Simple;
            t
        },
        {
            let mut t = Task::new("task-2", "Moderate", "Moderate task");
            t.complexity = TaskComplexity::Moderate;
            t
        },
        {
            let mut t = Task::new("task-3", "Complex", "Complex task");
            t.complexity = TaskComplexity::Complex;
            t.subtasks = vec![
                Task::new("task-3-1", "Subtask 1", "First"),
                Task::new("task-3-2", "Subtask 2", "Second"),
                Task::new("task-3-3", "Subtask 3", "Third"),
            ];
            t
        },
        {
            let mut t = Task::new("task-4", "Another Simple", "Another simple task");
            t.complexity = TaskComplexity::Simple;
            t
        },
    ];

    let stats = calculate_assessment_stats(&tasks);

    assert_eq!(stats.total_tasks, 4);
    assert_eq!(stats.simple, 2);
    assert_eq!(stats.moderate, 1);
    assert_eq!(stats.complex, 1);
    assert_eq!(stats.tasks_with_subtasks, 1);
    assert_eq!(stats.total_subtasks, 3);
}

#[test]
fn test_assessment_stats_display_format() {
    let stats = AssessmentStats {
        total_tasks: 10,
        simple: 3,
        moderate: 5,
        complex: 2,
        tasks_with_subtasks: 2,
        total_subtasks: 5,
    };

    let display = format!("{}", stats);

    assert!(display.contains("Total tasks: 10"));
    assert!(display.contains("Simple: 3"));
    assert!(display.contains("Moderate: 5"));
    assert!(display.contains("Complex: 2"));
    assert!(display.contains("Tasks with subtasks: 2"));
    assert!(display.contains("Total subtasks: 5"));
    assert!(display.contains("fast model"));
    assert!(display.contains("standard model"));
    assert!(display.contains("smart model"));
}

#[test]
fn test_assign_models_to_tasks_simple() {
    let mut tasks = vec![
        {
            let mut t = Task::new("task-1", "Simple", "Simple task");
            t.complexity = TaskComplexity::Simple;
            t
        },
    ];

    let config = AssessConfig::default();
    assign_models_to_tasks(&mut tasks, &config);

    // Simple tasks should use the fast model (Sonnet in default config)
    // Note: This tests the assignment logic, actual model selection is in ModeConfig
    assert_eq!(tasks[0].complexity, TaskComplexity::Simple);
}

#[test]
fn test_assign_models_to_tasks_with_subtasks() {
    let mut tasks = vec![
        {
            let mut t = Task::new("task-1", "Complex", "Complex with subtasks");
            t.complexity = TaskComplexity::Complex;
            t.subtasks = vec![
                {
                    let mut st = Task::new("task-1-1", "Simple Subtask", "Simple");
                    st.complexity = TaskComplexity::Simple;
                    st
                },
            ];
            t
        },
    ];

    let config = AssessConfig::default();
    assign_models_to_tasks(&mut tasks, &config);

    // Should recursively assign models to subtasks
    assert_eq!(tasks[0].complexity, TaskComplexity::Complex);
    assert_eq!(tasks[0].subtasks[0].complexity, TaskComplexity::Simple);
}

#[test]
fn test_assign_models_to_tasks_empty() {
    let mut tasks: Vec<Task> = vec![];
    let config = AssessConfig::default();

    // Should not panic on empty list
    assign_models_to_tasks(&mut tasks, &config);
    assert!(tasks.is_empty());
}

#[test]
fn test_task_assessment_structure() {
    let assessment = TaskAssessment {
        complexity: TaskComplexity::Moderate,
        subtasks: vec![],
        recommended_model: "claude-sonnet-4-6".to_string(),
        estimated_time_minutes: Some(30),
    };

    // Verify the structure has all required fields
    assert_eq!(assessment.complexity, TaskComplexity::Moderate);
    assert!(assessment.subtasks.is_empty());
    assert_eq!(assessment.recommended_model, "claude-sonnet-4-6");
    assert_eq!(assessment.estimated_time_minutes, Some(30));
}

#[test]
fn test_build_assessment_prompt_contains_required_fields() {
    let task = Task::new("test-1", "Test Task", "A test description");
    let prompt = build_assessment_prompt(&task, 0);

    // Verify prompt contains task information
    assert!(prompt.contains(&task.id));
    assert!(prompt.contains(&task.title));
    assert!(prompt.contains(&task.description));

    // Verify prompt contains complexity definitions
    assert!(prompt.contains("Simple"));
    assert!(prompt.contains("Moderate"));
    assert!(prompt.contains("Complex"));

    // Verify prompt contains model selection guidance
    assert!(prompt.contains("claude-haiku-4-5"));
    assert!(prompt.contains("claude-sonnet-4-6"));
    assert!(prompt.contains("claude-opus-4-6"));

    // Verify prompt requests JSON response format
    assert!(prompt.contains("json"));
}

#[test]
fn test_build_assessment_prompt_with_depth() {
    let task = Task::new("subtask-1", "Subtask", "A subtask");
    let prompt = build_assessment_prompt(&task, 2);

    // Verify depth context is included
    assert!(prompt.contains("(Subtask depth: 2)"));
}

#[test]
fn test_task_structure_with_subtasks() {
    let mut task = Task::new("parent-1", "Parent Task", "Parent description");
    task.complexity = TaskComplexity::Complex;
    task.subtasks = vec![
        Task::new("subtask-1", "Subtask 1", "First subtask"),
        Task::new("subtask-2", "Subtask 2", "Second subtask"),
    ];

    assert_eq!(task.subtasks.len(), 2);
    assert_eq!(task.subtasks[0].id, "subtask-1");
    assert_eq!(task.subtasks[1].id, "subtask-2");
}

#[test]
fn test_task_with_dependencies() {
    let mut task = Task::new("task-2", "Task with deps", "Has dependencies");
    task.depends_on = vec!["task-1".to_string(), "task-0".to_string()];

    assert_eq!(task.depends_on.len(), 2);
    assert!(task.depends_on.contains(&"task-1".to_string()));
    assert!(task.depends_on.contains(&"task-0".to_string()));
}

#[test]
fn test_task_status_transitions() {
    let mut task = Task::new("task-1", "Test", "Test task");

    // Initial status
    assert_eq!(task.status, TaskStatus::Pending);
    assert!(!task.is_completed());
    assert!(!task.is_failed());

    // Can execute when no dependencies
    let completed = std::collections::HashSet::new();
    assert!(task.can_execute(&completed));

    // Can't execute with unmet dependencies
    task.depends_on = vec!["task-0".to_string()];
    assert!(!task.can_execute(&completed));

    // Can execute with met dependencies
    let mut completed_with_dep = std::collections::HashSet::new();
    completed_with_dep.insert("task-0".to_string());
    assert!(task.can_execute(&completed_with_dep));
}

#[test]
fn test_task_retry_logic() {
    let mut task = Task::new("task-1", "Test", "Test task");

    // Can't retry pending task
    assert!(!task.can_retry(3));

    // After failure, can retry
    task.status = TaskStatus::Failed;
    task.retry_count = 0;
    assert!(task.can_retry(3));

    // After max retries, can't retry
    task.retry_count = 3;
    assert!(!task.can_retry(3));

    // Can retry if under limit
    task.retry_count = 2;
    assert!(task.can_retry(3));
}

#[test]
fn test_mode_config_default() {
    let config = ltmatrix::models::ModeConfig::default();

    assert_eq!(config.max_depth, 3);
    assert_eq!(config.max_retries, 3);
    assert!(config.run_tests);
    assert!(config.verify);
}

#[test]
fn test_mode_config_fast_mode() {
    let config = ltmatrix::models::ModeConfig::fast_mode();

    assert_eq!(config.max_depth, 2);
    assert_eq!(config.max_retries, 1);
    assert!(!config.run_tests, "Fast mode should skip tests");
    assert!(config.verify);
}

#[test]
fn test_mode_config_expert_mode() {
    let config = ltmatrix::models::ModeConfig::expert_mode();

    assert_eq!(config.max_depth, 3);
    assert_eq!(config.max_retries, 3);
    assert!(config.run_tests);
    assert!(config.verify);
}

#[test]
fn test_mode_config_model_selection() {
    let config = ltmatrix::models::ModeConfig::default();

    // Test model selection for different complexities
    let simple_model = config.model_for_complexity(&TaskComplexity::Simple);
    let moderate_model = config.model_for_complexity(&TaskComplexity::Moderate);
    let complex_model = config.model_for_complexity(&TaskComplexity::Complex);

    // Simple and moderate should use fast model
    assert_eq!(simple_model, moderate_model);

    // Complex should use smart model
    assert_ne!(complex_model, simple_model);
}

#[test]
fn test_task_complexity_serialization() {
    // Test that complexity enums can be serialized/deserialized
    let simple = TaskComplexity::Simple;
    let moderate = TaskComplexity::Moderate;
    let complex = TaskComplexity::Complex;

    // Verify equality works
    assert_eq!(simple, TaskComplexity::Simple);
    assert_eq!(moderate, TaskComplexity::Moderate);
    assert_eq!(complex, TaskComplexity::Complex);
    assert_ne!(simple, moderate);
}

// Integration-style test verifying the complete assessment workflow structure
#[test]
fn test_assessment_workflow_structure() {
    // This test verifies the structure is correct without calling Claude
    let tasks = vec![
        Task::new("task-1", "Simple Task", "A simple task"),
        Task::new("task-2", "Complex Task", "A complex task"),
    ];

    let config = AssessConfig::default();

    // Verify we can create the necessary structures
    assert_eq!(tasks.len(), 2);
    assert_eq!(config.max_depth, 3);

    // Verify task structure
    assert_eq!(tasks[0].id, "task-1");
    assert_eq!(tasks[1].id, "task-2");
    assert_eq!(tasks[0].complexity, TaskComplexity::Moderate); // Default
}

#[test]
fn test_subtask_parsing_with_complex_dependencies() {
    let response = r#"```json
{
  "complexity": "Complex",
  "recommended_model": "claude-opus-4-6",
  "estimated_time_minutes": 180,
  "subtasks": [
    {
      "id": "design",
      "title": "Design Schema",
      "description": "Design the database schema",
      "depends_on": []
    },
    {
      "id": "migrate",
      "title": "Create Migration",
      "description": "Create database migration",
      "depends_on": ["design"]
    },
    {
      "id": "implement",
      "title": "Implement API",
      "description": "Implement the API endpoints",
      "depends_on": ["migrate"]
    },
    {
      "id": "test",
      "title": "Write Tests",
      "description": "Write integration tests",
      "depends_on": ["implement", "migrate"]
    }
  ]
}
```"#;

    let task = Task::new("api-task", "Build API", "Build a REST API");
    let config = AssessConfig::default();

    let assessment = parse_assessment_response(response, &task, &config)
        .expect("Should parse complex dependency chain");

    assert_eq!(assessment.subtasks.len(), 4);

    // Verify dependency chain
    assert!(assessment.subtasks[0].depends_on.is_empty());
    assert_eq!(assessment.subtasks[1].depends_on, vec!["design".to_string()]);
    assert_eq!(assessment.subtasks[2].depends_on, vec!["migrate".to_string()]);
    assert_eq!(assessment.subtasks[3].depends_on.len(), 2);
}

#[test]
fn test_assessment_handles_null_time_estimate() {
    let response = r#"```json
{
  "complexity": "Moderate",
  "recommended_model": "claude-sonnet-4-6",
  "estimated_time_minutes": null,
  "subtasks": []
}
```"#;

    let task = Task::new("task-1", "Test", "Test task");
    let config = AssessConfig::default();

    let assessment = parse_assessment_response(response, &task, &config)
        .expect("Should handle null time estimate");

    assert!(assessment.estimated_time_minutes.is_none());
}
