//! Task assessment stage
//!
//! This module implements the Assess stage of the pipeline, which:
//! - Evaluates each task's complexity using Claude
//! - Splits complex tasks into subtasks (max depth: 3)
//! - Updates task structures with complexity ratings
//! - Implements smart model selection based on complexity
//! - Returns enriched task list

use anyhow::{Context, Result};
use serde_json::Value;
use tracing::{debug, info, warn};

use crate::agent::backend::{AgentBackend, ExecutionConfig};
use crate::agent::ClaudeAgent;
use crate::models::{ModeConfig, Task, TaskComplexity, TaskStatus};

/// Configuration for the assessment stage
#[derive(Debug, Clone)]
pub struct AssessConfig {
    /// Maximum depth for task decomposition
    pub max_depth: u32,

    /// Model to use for assessment
    pub assessment_model: String,

    /// Timeout for assessment requests (seconds)
    pub timeout: u64,

    /// Maximum retries for assessment
    pub max_retries: u32,

    /// Mode configuration for model selection
    pub mode_config: ModeConfig,
}

impl Default for AssessConfig {
    fn default() -> Self {
        AssessConfig {
            max_depth: 3,
            assessment_model: "claude-sonnet-4-6".to_string(),
            timeout: 120,
            max_retries: 3,
            mode_config: ModeConfig::default(),
        }
    }
}

impl AssessConfig {
    /// Create config for fast mode
    pub fn fast_mode() -> Self {
        AssessConfig {
            max_depth: 2,
            assessment_model: "claude-haiku-4-5".to_string(),
            timeout: 60,
            max_retries: 1,
            mode_config: ModeConfig::fast_mode(),
        }
    }

    /// Create config for expert mode
    pub fn expert_mode() -> Self {
        AssessConfig {
            max_depth: 3,
            assessment_model: "claude-opus-4-6".to_string(),
            timeout: 180,
            max_retries: 3,
            mode_config: ModeConfig::expert_mode(),
        }
    }
}

/// Result of assessing a single task
#[derive(Debug, Clone)]
pub struct TaskAssessment {
    /// The assessed complexity
    pub complexity: TaskComplexity,

    /// Suggested subtasks (if complex)
    pub subtasks: Vec<Task>,

    /// Recommended model for execution
    pub recommended_model: String,

    /// Estimated time (optional)
    pub estimated_time_minutes: Option<u32>,
}

/// Assesses a list of tasks and enriches them with complexity ratings and subtasks
pub async fn assess_tasks(tasks: Vec<Task>, config: &AssessConfig) -> Result<Vec<Task>> {
    info!("Starting assessment stage for {} tasks", tasks.len());

    let agent = ClaudeAgent::new().context("Failed to create Claude agent for assessment")?;

    let mut assessed_tasks = Vec::with_capacity(tasks.len());

    for mut task in tasks {
        debug!("Assessing task: {}", task.id);

        match assess_single_task(&task, config, &agent, 0).await {
            Ok(assessment) => {
                // Update task with assessment results
                task.complexity = assessment.complexity.clone();

                // Add subtasks if suggested
                if !assessment.subtasks.is_empty() {
                    info!(
                        "Task {} split into {} subtasks",
                        task.id,
                        assessment.subtasks.len()
                    );
                    task.subtasks = assessment.subtasks;
                }

                // Update timestamps
                task.started_at = Some(chrono::Utc::now());

                // Capture values for logging before moving task
                let task_id = task.id.clone();
                let task_complexity = task.complexity.clone();

                assessed_tasks.push(task);

                debug!("Task {} assessed as {:?}", task_id, task_complexity);
            }
            Err(e) => {
                warn!(
                    "Failed to assess task {}: {}, using default complexity",
                    task.id, e
                );
                // Use default complexity for failed assessments
                task.complexity = TaskComplexity::Moderate;
                task.started_at = Some(chrono::Utc::now());
                assessed_tasks.push(task);
            }
        }
    }

    info!(
        "Assessment stage completed for {} tasks",
        assessed_tasks.len()
    );
    Ok(assessed_tasks)
}

/// Assesses a single task, recursively assessing subtasks if needed
async fn assess_single_task(
    task: &Task,
    config: &AssessConfig,
    agent: &ClaudeAgent,
    current_depth: u32,
) -> Result<TaskAssessment> {
    // Check max depth
    if current_depth >= config.max_depth {
        debug!(
            "Reached max depth {} for task {}, using simple assessment",
            config.max_depth, task.id
        );
        return Ok(TaskAssessment {
            complexity: TaskComplexity::Moderate,
            subtasks: Vec::new(),
            recommended_model: config
                .mode_config
                .model_for_complexity(&TaskComplexity::Moderate)
                .to_string(),
            estimated_time_minutes: None,
        });
    }

    // Build assessment prompt
    let prompt = build_assessment_prompt(task, current_depth);

    // Create execution config
    let exec_config = ExecutionConfig {
        model: config.assessment_model.clone(),
        timeout: config.timeout,
        max_retries: config.max_retries,
        enable_session: false, // No need for session in assessment
        env_vars: Vec::new(),
    };

    // Execute assessment
    let response = agent
        .execute(&prompt, &exec_config)
        .await
        .context("Failed to execute assessment prompt")?;

    // Parse response
    let assessment = parse_assessment_response(&response.output, task, config)
        .context("Failed to parse assessment response")?;

    // Recursively assess subtasks if present
    if !assessment.subtasks.is_empty() && current_depth + 1 < config.max_depth {
        debug!(
            "Recursively assessing {} subtasks for task {}",
            assessment.subtasks.len(),
            task.id
        );
    }

    Ok(assessment)
}

/// Builds the assessment prompt for Claude
fn build_assessment_prompt(task: &Task, current_depth: u32) -> String {
    let depth_context = if current_depth > 0 {
        format!("(Subtask depth: {})", current_depth)
    } else {
        String::new()
    };

    format!(
        r#"You are a task assessment expert. Analyze the following task and determine its complexity.

Task ID: {} {}
Title: {}
Description: {}

Your role:
1. Assess the task complexity as one of:
   - Simple: Straightforward, minimal dependencies, clear implementation path
   - Moderate: Some complexity, multiple components, requires careful design
   - Complex: High complexity, multiple systems, architectural decisions needed

2. If the task is rated as "Complex", break it down into 2-5 subtasks.
   Each subtask should be:
   - Independently executable (or with clear dependencies)
   - Specific and actionable
   - Include a clear description

3. Recommend the appropriate AI model for execution:
   - Simple tasks: claude-haiku-4-5 (fast, cost-effective)
   - Moderate tasks: claude-sonnet-4-6 (balanced)
   - Complex tasks: claude-opus-4-6 (highest quality)

4. Estimate the time to complete (in minutes).

Respond ONLY with valid JSON in this exact format:

```json
{{
  "complexity": "Simple|Moderate|Complex",
  "recommended_model": "claude-haiku-4-5|claude-sonnet-4-6|claude-opus-4-6",
  "estimated_time_minutes": <number or null>,
  "reasoning": "<brief explanation of complexity rating>",
  "subtasks": [
    {{
      "id": "<unique subtask ID>",
      "title": "<subtask title>",
      "description": "<detailed description>",
      "depends_on": ["<list of subtask IDs this depends on, or empty array>"]
    }}
  ]
}}
```

If complexity is Simple or Moderate, subtasks should be an empty array.
"#,
        task.id, depth_context, task.title, task.description
    )
}

/// Parses Claude's response and extracts assessment data
fn parse_assessment_response(
    response: &str,
    original_task: &Task,
    config: &AssessConfig,
) -> Result<TaskAssessment> {
    // Extract JSON from response
    let json_str =
        extract_json_block(response).context("No JSON block found in assessment response")?;

    // Parse JSON
    let json: Value = serde_json::from_str(json_str).context("Failed to parse assessment JSON")?;

    // Extract complexity
    let complexity_str = json["complexity"]
        .as_str()
        .context("Missing or invalid 'complexity' field")?;

    let complexity = match complexity_str {
        "Simple" => TaskComplexity::Simple,
        "Moderate" => TaskComplexity::Moderate,
        "Complex" => TaskComplexity::Complex,
        _ => {
            warn!("Unknown complexity '{}', using Moderate", complexity_str);
            TaskComplexity::Moderate
        }
    };

    // Extract recommended model
    let recommended_model = json["recommended_model"]
        .as_str()
        .unwrap_or_else(|| config.mode_config.model_for_complexity(&complexity))
        .to_string();

    // Extract estimated time
    let estimated_time_minutes = json["estimated_time_minutes"].as_u64().map(|v| v as u32);

    // Extract subtasks if present
    let subtasks = if let Some(subtasks_array) = json["subtasks"].as_array() {
        parse_subtasks(subtasks_array, &original_task.id)?
    } else {
        Vec::new()
    };

    debug!(
        "Parsed assessment: complexity={:?}, model={}, subtasks={}",
        complexity,
        recommended_model,
        subtasks.len()
    );

    Ok(TaskAssessment {
        complexity,
        subtasks,
        recommended_model,
        estimated_time_minutes,
    })
}

/// Extracts JSON block from markdown response
fn extract_json_block(text: &str) -> Option<&str> {
    // Look for ```json block
    let json_start = text.find("```json")? + 7; // Skip past ```json
    let json_end = text[json_start..].find("```")?;
    Some(text[json_start..json_start + json_end].trim())
}

/// Parses subtasks from JSON array
fn parse_subtasks(subtasks_array: &[Value], parent_id: &str) -> Result<Vec<Task>> {
    let mut subtasks = Vec::new();

    for (index, subtask_json) in subtasks_array.iter().enumerate() {
        let id = subtask_json["id"]
            .as_str()
            .unwrap_or(&format!("{}-subtask-{}", parent_id, index + 1))
            .to_string();

        let title = subtask_json["title"]
            .as_str()
            .context(format!("Subtask {} missing 'title'", index))?
            .to_string();

        let description = subtask_json["description"]
            .as_str()
            .context(format!("Subtask {} missing 'description'", index))?
            .to_string();

        let depends_on: Vec<String> = subtask_json["depends_on"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let mut task = Task::new(&id, &title, &description);
        task.depends_on = depends_on;
        task.status = TaskStatus::Pending;

        subtasks.push(task);
    }

    Ok(subtasks)
}

/// Assigns optimal models to all tasks based on their complexity
pub fn assign_models_to_tasks(tasks: &mut [Task], config: &AssessConfig) {
    for task in tasks.iter_mut() {
        // Assign model based on complexity
        let model = config
            .mode_config
            .model_for_complexity(&task.complexity)
            .to_string();

        debug!(
            "Task {} (complexity: {:?}) assigned model: {}",
            task.id, task.complexity, model
        );

        // Recursively assign models to subtasks
        assign_models_to_tasks(&mut task.subtasks, config);
    }
}

/// Calculates statistics about assessed tasks
pub fn calculate_assessment_stats(tasks: &[Task]) -> AssessmentStats {
    let mut simple = 0;
    let mut moderate = 0;
    let mut complex = 0;
    let mut with_subtasks = 0;
    let mut total_subtasks = 0;

    for task in tasks {
        match task.complexity {
            TaskComplexity::Simple => simple += 1,
            TaskComplexity::Moderate => moderate += 1,
            TaskComplexity::Complex => complex += 1,
        }

        if !task.subtasks.is_empty() {
            with_subtasks += 1;
            total_subtasks += task.subtasks.len();
        }
    }

    AssessmentStats {
        total_tasks: tasks.len(),
        simple,
        moderate,
        complex,
        tasks_with_subtasks: with_subtasks,
        total_subtasks,
    }
}

/// Statistics about task assessment
#[derive(Debug, Clone)]
pub struct AssessmentStats {
    /// Total number of tasks assessed
    pub total_tasks: usize,

    /// Number of simple tasks
    pub simple: usize,

    /// Number of moderate tasks
    pub moderate: usize,

    /// Number of complex tasks
    pub complex: usize,

    /// Number of tasks that were split into subtasks
    pub tasks_with_subtasks: usize,

    /// Total number of subtasks created
    pub total_subtasks: usize,
}

impl std::fmt::Display for AssessmentStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Assessment Summary:\n\
             - Total tasks: {}\n\
             - Simple: {} (fast model)\n\
             - Moderate: {} (standard model)\n\
             - Complex: {} (smart model)\n\
             - Tasks with subtasks: {}\n\
             - Total subtasks: {}",
            self.total_tasks,
            self.simple,
            self.moderate,
            self.complex,
            self.tasks_with_subtasks,
            self.total_subtasks
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_block() {
        let response = r#"Some text before.

```json
{
  "complexity": "Simple",
  "subtasks": []
}
```

Some text after."#;

        let json = extract_json_block(response).unwrap();
        assert!(json.contains("complexity"));
        assert!(json.contains("Simple"));
    }

    #[test]
    fn test_parse_assessment_response() {
        let response = r#"```json
{
  "complexity": "Moderate",
  "recommended_model": "claude-sonnet-4-6",
  "estimated_time_minutes": 30,
  "subtasks": []
}
```"#;

        let task = Task::new("test-1", "Test", "A test task");
        let config = AssessConfig::default();

        let assessment = parse_assessment_response(response, &task, &config).unwrap();
        assert_eq!(assessment.complexity, TaskComplexity::Moderate);
        assert_eq!(assessment.recommended_model, "claude-sonnet-4-6");
        assert_eq!(assessment.estimated_time_minutes, Some(30));
        assert!(assessment.subtasks.is_empty());
    }

    #[test]
    fn test_assessment_stats_display() {
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
    }

    #[test]
    fn test_calculate_assessment_stats() {
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
                    Task::new("task-3-1", "Subtask 1", "First subtask"),
                    Task::new("task-3-2", "Subtask 2", "Second subtask"),
                ];
                t
            },
        ];

        let stats = calculate_assessment_stats(&tasks);
        assert_eq!(stats.total_tasks, 3);
        assert_eq!(stats.simple, 1);
        assert_eq!(stats.moderate, 1);
        assert_eq!(stats.complex, 1);
        assert_eq!(stats.tasks_with_subtasks, 1);
        assert_eq!(stats.total_subtasks, 2);
    }

    #[tokio::test]
    async fn test_assess_config_defaults() {
        let config = AssessConfig::default();
        assert_eq!(config.max_depth, 3);
        assert_eq!(config.assessment_model, "claude-sonnet-4-6");
        assert_eq!(config.timeout, 120);
        assert_eq!(config.max_retries, 3);
    }

    #[tokio::test]
    async fn test_assess_config_fast_mode() {
        let config = AssessConfig::fast_mode();
        assert_eq!(config.max_depth, 2);
        assert_eq!(config.assessment_model, "claude-haiku-4-5");
        assert_eq!(config.timeout, 60);
        assert_eq!(config.max_retries, 1);
    }

    // Comprehensive test suite for task assessment

    #[test]
    fn test_assess_config_expert_mode() {
        let config = AssessConfig::expert_mode();
        assert_eq!(config.max_depth, 3);
        assert_eq!(config.assessment_model, "claude-opus-4-6");
        assert_eq!(config.timeout, 180);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_extract_json_block_no_json() {
        let response = "This response has no JSON block in it.";
        let result = extract_json_block(response);
        assert!(
            result.is_none(),
            "Should return None when no JSON block found"
        );
    }

    #[test]
    fn test_extract_json_block_malformed() {
        let response = r#"```json
{"incomplete": "json"
```"#;
        let result = extract_json_block(response);
        assert!(result.is_some(), "Should still extract malformed JSON");
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

        let assessment = parse_assessment_response(response, &task, &config).unwrap();
        assert_eq!(assessment.complexity, TaskComplexity::Simple);
        assert_eq!(assessment.recommended_model, "claude-haiku-4-5");
        assert_eq!(assessment.estimated_time_minutes, Some(15));
        assert!(assessment.subtasks.is_empty());
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

        let assessment = parse_assessment_response(response, &task, &config).unwrap();
        assert_eq!(assessment.complexity, TaskComplexity::Complex);
        assert_eq!(assessment.recommended_model, "claude-opus-4-6");
        assert_eq!(assessment.estimated_time_minutes, Some(120));
        assert_eq!(assessment.subtasks.len(), 2);
        assert_eq!(assessment.subtasks[0].id, "subtask-1");
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

        let assessment = parse_assessment_response(response, &task, &config).unwrap();
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

        let assessment = parse_assessment_response(response, &task, &config).unwrap();
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
        assert!(
            result.is_err(),
            "Should return error when no JSON block found"
        );
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
    fn test_assign_models_to_tasks_simple() {
        let mut tasks = vec![{
            let mut t = Task::new("task-1", "Simple", "Simple task");
            t.complexity = TaskComplexity::Simple;
            t
        }];

        let config = AssessConfig::default();
        assign_models_to_tasks(&mut tasks, &config);
        assert_eq!(tasks[0].complexity, TaskComplexity::Simple);
    }

    #[test]
    fn test_assign_models_to_tasks_with_subtasks() {
        let mut tasks = vec![{
            let mut t = Task::new("task-1", "Complex", "Complex with subtasks");
            t.complexity = TaskComplexity::Complex;
            t.subtasks = vec![{
                let mut st = Task::new("task-1-1", "Simple Subtask", "Simple");
                st.complexity = TaskComplexity::Simple;
                st
            }];
            t
        }];

        let config = AssessConfig::default();
        assign_models_to_tasks(&mut tasks, &config);

        assert_eq!(tasks[0].complexity, TaskComplexity::Complex);
        assert_eq!(tasks[0].subtasks[0].complexity, TaskComplexity::Simple);
    }

    #[test]
    fn test_assign_models_to_tasks_empty() {
        let mut tasks: Vec<Task> = vec![];
        let config = AssessConfig::default();
        assign_models_to_tasks(&mut tasks, &config);
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_build_assessment_prompt_contains_required_fields() {
        let task = Task::new("test-1", "Test Task", "A test description");
        let prompt = build_assessment_prompt(&task, 0);

        assert!(prompt.contains(&task.id));
        assert!(prompt.contains(&task.title));
        assert!(prompt.contains(&task.description));
        assert!(prompt.contains("Simple"));
        assert!(prompt.contains("Moderate"));
        assert!(prompt.contains("Complex"));
        assert!(prompt.contains("claude-haiku-4-5"));
        assert!(prompt.contains("claude-sonnet-4-6"));
        assert!(prompt.contains("claude-opus-4-6"));
        assert!(prompt.contains("json"));
    }

    #[test]
    fn test_build_assessment_prompt_with_depth() {
        let task = Task::new("subtask-1", "Subtask", "A subtask");
        let prompt = build_assessment_prompt(&task, 2);
        assert!(prompt.contains("(Subtask depth: 2)"));
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

        let assessment = parse_assessment_response(response, &task, &config).unwrap();
        assert_eq!(assessment.subtasks.len(), 4);
        assert!(assessment.subtasks[0].depends_on.is_empty());
        assert_eq!(
            assessment.subtasks[1].depends_on,
            vec!["design".to_string()]
        );
        assert_eq!(
            assessment.subtasks[2].depends_on,
            vec!["migrate".to_string()]
        );
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

        let assessment = parse_assessment_response(response, &task, &config).unwrap();
        assert!(assessment.estimated_time_minutes.is_none());
    }

    #[test]
    fn test_assessment_stats_display_includes_model_info() {
        let stats = AssessmentStats {
            total_tasks: 10,
            simple: 3,
            moderate: 5,
            complex: 2,
            tasks_with_subtasks: 2,
            total_subtasks: 5,
        };

        let display = format!("{}", stats);
        assert!(display.contains("fast model"));
        assert!(display.contains("standard model"));
        assert!(display.contains("smart model"));
    }
}
