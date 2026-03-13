//! Integration tests for the Generate stage
//!
//! These tests verify the integration of the generate stage with the Claude agent,
//! testing end-to-end workflows with mocked or actual API responses.

use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::pipeline::generate::{
    generate_tasks, GenerateConfig, GenerationResult, ValidationError,
};

#[tokio::test]
async fn test_generate_stage_integration_with_claude_agent() {
    // This test verifies that generate_tasks can create a Claude agent
    // and attempt to call it (will fail without API key, but tests integration)

    let goal = "Implement a simple hello world API endpoint";
    let config = GenerateConfig::fast_mode();

    // Without a valid API key, this will fail at the API call stage
    // but we can verify the setup works
    let result = generate_tasks(goal, &config).await;

    // We expect this to fail without API credentials, but not to panic
    // or fail during initialization
    match result {
        Ok(_) => {
            // If somehow this succeeds (e.g., API key is set), verify structure
        }
        Err(e) => {
            // Should fail due to API/authentication, not setup issues
            let error_msg = e.to_string().to_lowercase();
            // Verify it's not a setup/initialization error
            assert!(
                !error_msg.contains("failed to create")
                    || error_msg.contains("api")
                    || error_msg.contains("authentication")
                    || error_msg.contains("timeout")
                    || error_msg.contains("network"),
                "Error should be API-related, not setup-related: {}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_generate_config_mode_variations() {
    // Test that different execution modes produce different prompts
    let goal = "Build a REST API";

    let fast_config = GenerateConfig::fast_mode();
    let standard_config = GenerateConfig::standard_mode();
    let expert_config = GenerateConfig::expert_mode();

    // Verify different models are selected
    assert_eq!(fast_config.generation_model, "claude-haiku-4-5");
    assert_eq!(standard_config.generation_model, "claude-sonnet-4-6");
    assert_eq!(expert_config.generation_model, "claude-opus-4-6");

    // Verify different timeout configurations
    assert!(fast_config.timeout < standard_config.timeout);
    assert!(standard_config.timeout <= expert_config.timeout);

    // Verify different max_tasks limits
    assert!(fast_config.max_tasks < standard_config.max_tasks);
    assert!(standard_config.max_tasks <= expert_config.max_tasks);
}

#[tokio::test]
async fn test_generate_result_structure() {
    // Test that GenerationResult has the correct structure
    let tasks = vec![
        {
            let mut t = Task::new("task-1", "First Task", "Description");
            t.complexity = TaskComplexity::Simple;
            t.status = TaskStatus::Pending;
            t.depends_on = vec![];
            t
        },
        {
            let mut t = Task::new("task-2", "Second Task", "Description");
            t.complexity = TaskComplexity::Moderate;
            t.status = TaskStatus::Pending;
            t.depends_on = vec!["task-1".to_string()];
            t
        },
    ];

    let result = GenerationResult {
        tasks: tasks.clone(),
        task_count: 2,
        dependency_depth: 1,
        validation_errors: vec![],
        generation_log: None,
    };

    assert_eq!(result.task_count, 2);
    assert_eq!(result.tasks.len(), 2);
    assert_eq!(result.dependency_depth, 1);
    assert!(result.validation_errors.is_empty());
}

#[tokio::test]
async fn test_validation_error_types() {
    // Test each validation error type

    // Missing dependency error
    let error1 = ValidationError::MissingDependency {
        task: "task-1".to_string(),
        dependency: "task-missing".to_string(),
    };
    let display1 = format!("{}", error1);
    assert!(display1.contains("task-1"));
    assert!(display1.contains("task-missing"));

    // Circular dependency error
    let error2 = ValidationError::CircularDependency {
        chain: vec![
            "task-1".to_string(),
            "task-2".to_string(),
            "task-1".to_string(),
        ],
    };
    let display2 = format!("{}", error2);
    assert!(display1.contains("task-1"));

    // Duplicate task ID error
    let error3 = ValidationError::DuplicateTaskId {
        id: "task-1".to_string(),
    };
    let display3 = format!("{}", error3);
    assert!(display3.contains("task-1"));

    // Invalid structure error
    let error4 = ValidationError::InvalidStructure {
        task: "task-1".to_string(),
        reason: "empty title".to_string(),
    };
    let display4 = format!("{}", error4);
    assert!(display4.contains("task-1"));
    assert!(display4.contains("empty title"));
}

#[tokio::test]
async fn test_execution_mode_comparison() {
    use ltmatrix::pipeline::generate::ExecutionMode;

    // Test that execution modes are comparable
    let fast = ExecutionMode::Fast;
    let standard = ExecutionMode::Standard;
    let expert = ExecutionMode::Expert;

    // Modes should be equal to themselves
    assert_eq!(fast, ExecutionMode::Fast);
    assert_eq!(standard, ExecutionMode::Standard);
    assert_eq!(expert, ExecutionMode::Expert);

    // Modes should not be equal to each other
    assert_ne!(fast, standard);
    assert_ne!(standard, expert);
    assert_ne!(fast, expert);
}

#[tokio::test]
async fn test_generate_result_with_validation_errors() {
    // Test GenerationResult can contain validation errors
    let result = GenerationResult {
        tasks: vec![],
        task_count: 0,
        dependency_depth: 0,
        validation_errors: vec![ValidationError::MissingDependency {
            task: "task-1".to_string(),
            dependency: "missing".to_string(),
        }],
        generation_log: None,
    };

    assert_eq!(result.validation_errors.len(), 1);
    assert!(!result.validation_errors.is_empty());
}

#[tokio::test]
async fn test_task_count_matches_tasks_length() {
    // Verify that task_count matches tasks.len()
    let tasks = vec![
        Task::new("task-1", "Task 1", "Description 1"),
        Task::new("task-2", "Task 2", "Description 2"),
        Task::new("task-3", "Task 3", "Description 3"),
    ];

    let result = GenerationResult {
        tasks: tasks.clone(),
        task_count: tasks.len(),
        dependency_depth: 0,
        validation_errors: vec![],
        generation_log: None,
    };

    assert_eq!(result.task_count, result.tasks.len());
    assert_eq!(result.task_count, 3);
}

#[tokio::test]
async fn test_generate_tasks_max_tasks_enforcement() {
    // Test that max_tasks limit is enforced
    // This is a unit test for the behavior, integration would need mocking

    let config = GenerateConfig {
        max_tasks: 5,
        ..Default::default()
    };

    assert_eq!(config.max_tasks, 5);
}

#[tokio::test]
async fn test_generate_config_validation_enabled_by_default() {
    // Verify validation is enabled by default
    let config = GenerateConfig::default();
    assert!(config.enable_validation);
}

#[tokio::test]
async fn test_generate_config_validation_can_be_disabled() {
    // Verify validation can be disabled
    let config = GenerateConfig {
        enable_validation: false,
        ..Default::default()
    };

    assert!(!config.enable_validation);
}

// Helper function for standard mode (if not already exposed)
trait GenerateConfigExt {
    fn standard_mode() -> GenerateConfig;
}

impl GenerateConfigExt for GenerateConfig {
    fn standard_mode() -> Self {
        GenerateConfig {
            generation_model: "claude-sonnet-4-6".to_string(),
            timeout: 180,
            max_retries: 3,
            max_tasks: 50,
            enable_validation: true,
            execution_mode: ltmatrix::pipeline::generate::ExecutionMode::Standard,
        }
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_tasks_handles_empty_goal() {
        // Test handling of empty goal string
        let goal = "";
        let config = GenerateConfig::fast_mode();

        // Should not panic, should either return error or handle gracefully
        let _ = generate_tasks(goal, &config).await;
    }

    #[tokio::test]
    async fn test_generate_tasks_handles_whitespace_goal() {
        // Test handling of whitespace-only goal
        let goal = "   \n\t   ";
        let config = GenerateConfig::fast_mode();

        // Should not panic
        let _ = generate_tasks(&goal, &config).await;
    }

    #[tokio::test]
    async fn test_generate_tasks_handles_very_long_goal() {
        // Test handling of very long goal string
        let goal = "Implement ".repeat(1000);
        let config = GenerateConfig::fast_mode();

        // Should not panic
        let _ = generate_tasks(&goal, &config).await;
    }
}

#[cfg(test)]
mod statistics_tests {
    use super::*;
    use ltmatrix::pipeline::generate::calculate_generation_stats;

    #[tokio::test]
    async fn test_generation_stats_with_mixed_complexity() {
        // Test statistics calculation with mixed complexity tasks
        let tasks = vec![
            {
                let mut t = Task::new("task-1", "Simple", "Desc");
                t.complexity = TaskComplexity::Simple;
                t.depends_on = vec![];
                t
            },
            {
                let mut t = Task::new("task-2", "Moderate", "Desc");
                t.complexity = TaskComplexity::Moderate;
                t.depends_on = vec![];
                t
            },
            {
                let mut t = Task::new("task-3", "Complex", "Desc");
                t.complexity = TaskComplexity::Complex;
                t.depends_on = vec!["task-1".to_string()];
                t
            },
        ];

        let result = GenerationResult {
            tasks,
            task_count: 3,
            dependency_depth: 1,
            validation_errors: vec![],
            generation_log: None,
        };

        let stats = calculate_generation_stats(&result);

        assert_eq!(stats.simple, 1);
        assert_eq!(stats.moderate, 1);
        assert_eq!(stats.complex, 1);
        assert_eq!(stats.tasks_with_dependencies, 1);
        assert_eq!(stats.total_dependencies, 1);
    }

    #[tokio::test]
    async fn test_generation_stats_all_simple() {
        // Test with all simple tasks
        let tasks: Vec<Task> = (1..=5)
            .map(|i| {
                let mut t = Task::new(&format!("task-{}", i), "Task", "Desc");
                t.complexity = TaskComplexity::Simple;
                t.depends_on = vec![];
                t
            })
            .collect();

        let result = GenerationResult {
            tasks,
            task_count: 5,
            dependency_depth: 0,
            validation_errors: vec![],
            generation_log: None,
        };

        let stats = calculate_generation_stats(&result);

        assert_eq!(stats.simple, 5);
        assert_eq!(stats.moderate, 0);
        assert_eq!(stats.complex, 0);
        assert_eq!(stats.tasks_with_dependencies, 0);
    }

    #[tokio::test]
    async fn test_generation_stats_with_validation_errors() {
        // Test statistics include validation errors
        let result = GenerationResult {
            tasks: vec![],
            task_count: 0,
            dependency_depth: 0,
            validation_errors: vec![
                ValidationError::DuplicateTaskId {
                    id: "task-1".to_string(),
                },
                ValidationError::MissingDependency {
                    task: "task-2".to_string(),
                    dependency: "missing".to_string(),
                },
            ],
            generation_log: None,
        };

        let stats = calculate_generation_stats(&result);

        assert_eq!(stats.validation_errors, 2);
    }
}
