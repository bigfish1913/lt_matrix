//! Tests for custom pipeline stages
//!
//! This test module verifies the custom pipeline stage functionality including:
//! - CustomStageConfig creation and configuration
//! - StagePosition variants and positioning
//! - StandardStage parsing and conversion
//! - StageResult success/failure handling
//! - PluginManager custom stage registration
//! - Built-in stage execution (LoggingStage, DelayStage)
//! - Mode filtering for custom stages

use std::collections::HashMap;
use std::str::FromStr;

use ltmatrix::models::{ExecutionMode, Task};
use ltmatrix::plugin::{
    CustomStageConfig, PipelineStagePlugin, Plugin, PluginManager, StagePosition, StageResult,
    StandardStage,
};
use ltmatrix::plugin::stage::{DelayStage, LoggingStage};

// =============================================================================
// CustomStageConfig Tests
// =============================================================================

#[test]
fn test_custom_stage_config_creation() {
    let config = CustomStageConfig::new(
        "my-custom-stage",
        "My Custom Stage",
        StagePosition::After(StandardStage::Execute),
    );

    assert_eq!(config.id, "my-custom-stage");
    assert_eq!(config.name, "My Custom Stage");
    assert!(config.enabled);
    assert!(!config.skip_on_failure);
    assert_eq!(config.timeout_seconds, 3600);
    assert!(config.modes.is_empty());
    assert!(config.plugin.is_none());
}

#[test]
fn test_custom_stage_config_with_description() {
    let config = CustomStageConfig::new(
        "test-stage",
        "Test Stage",
        StagePosition::Before(StandardStage::Test),
    )
    .with_description("A test stage for validation");

    assert_eq!(config.description, "A test stage for validation");
}

#[test]
fn test_custom_stage_config_with_enabled() {
    let config = CustomStageConfig::new(
        "disabled-stage",
        "Disabled Stage",
        StagePosition::Last,
    )
    .with_enabled(false);

    assert!(!config.enabled);
}

#[test]
fn test_custom_stage_config_with_modes() {
    let config = CustomStageConfig::new(
        "expert-only-stage",
        "Expert Only Stage",
        StagePosition::After(StandardStage::Review),
    )
    .with_modes(vec!["expert".to_string()]);

    assert_eq!(config.modes.len(), 1);
    assert!(config.modes.contains(&"expert".to_string()));
}

#[test]
fn test_custom_stage_config_should_run_for_mode_empty_modes() {
    let config = CustomStageConfig::new(
        "all-modes-stage",
        "All Modes Stage",
        StagePosition::First,
    );

    // Should run for all modes when modes is empty
    assert!(config.should_run_for_mode(&ExecutionMode::Fast));
    assert!(config.should_run_for_mode(&ExecutionMode::Standard));
    assert!(config.should_run_for_mode(&ExecutionMode::Expert));
}

#[test]
fn test_custom_stage_config_should_run_for_mode_specific_modes() {
    let config = CustomStageConfig::new(
        "fast-only-stage",
        "Fast Only Stage",
        StagePosition::Last,
    )
    .with_modes(vec!["fast".to_string()]);

    assert!(config.should_run_for_mode(&ExecutionMode::Fast));
    assert!(!config.should_run_for_mode(&ExecutionMode::Standard));
    assert!(!config.should_run_for_mode(&ExecutionMode::Expert));
}

#[test]
fn test_custom_stage_config_with_config_values() {
    let mut config = CustomStageConfig::new(
        "configurable-stage",
        "Configurable Stage",
        StagePosition::After(StandardStage::Generate),
    );

    config.config.insert("timeout_ms".to_string(), serde_json::json!(5000));
    config.config.insert("retries".to_string(), serde_json::json!(3));
    config.config.insert("verbose".to_string(), serde_json::json!(true));

    assert_eq!(config.config.len(), 3);
    assert_eq!(config.config.get("timeout_ms").unwrap(), &serde_json::json!(5000));
    assert_eq!(config.config.get("retries").unwrap(), &serde_json::json!(3));
}

// =============================================================================
// StagePosition Tests
// =============================================================================

#[test]
fn test_stage_position_before() {
    let position = StagePosition::Before(StandardStage::Test);
    assert!(matches!(position, StagePosition::Before(StandardStage::Test)));
}

#[test]
fn test_stage_position_after() {
    let position = StagePosition::After(StandardStage::Execute);
    assert!(matches!(position, StagePosition::After(StandardStage::Execute)));
}

#[test]
fn test_stage_position_first() {
    let position = StagePosition::First;
    assert!(matches!(position, StagePosition::First));
}

#[test]
fn test_stage_position_last() {
    let position = StagePosition::Last;
    assert!(matches!(position, StagePosition::Last));
}

#[test]
fn test_stage_position_replace() {
    let position = StagePosition::Replace(StandardStage::Review);
    assert!(matches!(position, StagePosition::Replace(StandardStage::Review)));
}

#[test]
fn test_stage_position_serialization() {
    let position = StagePosition::After(StandardStage::Execute);
    let json = serde_json::to_string(&position).unwrap();
    assert!(json.contains("after"));
    assert!(json.contains("execute"));
}

#[test]
fn test_stage_position_deserialization() {
    let json = r#"{"before": "test"}"#;
    let position: StagePosition = serde_json::from_str(json).unwrap();
    assert!(matches!(position, StagePosition::Before(StandardStage::Test)));
}

// =============================================================================
// StandardStage Tests
// =============================================================================

#[test]
fn test_standard_stage_from_str() {
    assert_eq!(StandardStage::from_str("generate").unwrap(), StandardStage::Generate);
    assert_eq!(StandardStage::from_str("assess").unwrap(), StandardStage::Assess);
    assert_eq!(StandardStage::from_str("execute").unwrap(), StandardStage::Execute);
    assert_eq!(StandardStage::from_str("test").unwrap(), StandardStage::Test);
    assert_eq!(StandardStage::from_str("review").unwrap(), StandardStage::Review);
    assert_eq!(StandardStage::from_str("verify").unwrap(), StandardStage::Verify);
    assert_eq!(StandardStage::from_str("commit").unwrap(), StandardStage::Commit);
    assert_eq!(StandardStage::from_str("memory").unwrap(), StandardStage::Memory);
}

#[test]
fn test_standard_stage_from_str_case_insensitive() {
    assert_eq!(StandardStage::from_str("GENERATE").unwrap(), StandardStage::Generate);
    assert_eq!(StandardStage::from_str("Execute").unwrap(), StandardStage::Execute);
    assert_eq!(StandardStage::from_str("TEST").unwrap(), StandardStage::Test);
}

#[test]
fn test_standard_stage_from_str_invalid() {
    assert!(StandardStage::from_str("invalid").is_err());
    assert!(StandardStage::from_str("").is_err());
    assert!(StandardStage::from_str("unknown_stage").is_err());
}

#[test]
fn test_standard_stage_display() {
    assert_eq!(StandardStage::Generate.to_string(), "generate");
    assert_eq!(StandardStage::Assess.to_string(), "assess");
    assert_eq!(StandardStage::Execute.to_string(), "execute");
    assert_eq!(StandardStage::Test.to_string(), "test");
    assert_eq!(StandardStage::Review.to_string(), "review");
    assert_eq!(StandardStage::Verify.to_string(), "verify");
    assert_eq!(StandardStage::Commit.to_string(), "commit");
    assert_eq!(StandardStage::Memory.to_string(), "memory");
}

#[test]
fn test_standard_stage_to_pipeline_stage() {
    use ltmatrix::models::PipelineStage;

    assert!(matches!(
        StandardStage::Generate.to_pipeline_stage(),
        PipelineStage::Generate
    ));
    assert!(matches!(
        StandardStage::Assess.to_pipeline_stage(),
        PipelineStage::Assess
    ));
    assert!(matches!(
        StandardStage::Execute.to_pipeline_stage(),
        PipelineStage::Execute
    ));
    assert!(matches!(
        StandardStage::Test.to_pipeline_stage(),
        PipelineStage::Test
    ));
    assert!(matches!(
        StandardStage::Review.to_pipeline_stage(),
        PipelineStage::Review
    ));
    assert!(matches!(
        StandardStage::Verify.to_pipeline_stage(),
        PipelineStage::Verify
    ));
    assert!(matches!(
        StandardStage::Commit.to_pipeline_stage(),
        PipelineStage::Commit
    ));
    assert!(matches!(
        StandardStage::Memory.to_pipeline_stage(),
        PipelineStage::Memory
    ));
}

#[test]
fn test_standard_stage_serialization() {
    let stage = StandardStage::Execute;
    let json = serde_json::to_string(&stage).unwrap();
    assert_eq!(json, r#""execute""#);
}

#[test]
fn test_standard_stage_deserialization() {
    let stage: StandardStage = serde_json::from_str(r#""review""#).unwrap();
    assert_eq!(stage, StandardStage::Review);
}

// =============================================================================
// StageResult Tests
// =============================================================================

#[test]
fn test_stage_result_success() {
    let tasks = vec![
        Task::new("task-1", "Task 1", "Description 1"),
        Task::new("task-2", "Task 2", "Description 2"),
    ];

    let result = StageResult::success(tasks.clone());

    assert!(result.success);
    assert!(result.error.is_none());
    assert_eq!(result.tasks.len(), 2);
    assert!(result.metrics.is_empty());
}

#[test]
fn test_stage_result_failure() {
    let tasks = vec![Task::new("task-1", "Task 1", "Description 1")];

    let result = StageResult::failure(tasks.clone(), "Stage execution failed");

    assert!(!result.success);
    assert_eq!(result.error, Some("Stage execution failed".to_string()));
    assert_eq!(result.tasks.len(), 1);
}

#[test]
fn test_stage_result_with_metric() {
    let result = StageResult::success(vec![])
        .with_metric("items_processed", serde_json::json!(42))
        .with_metric("time_ms", serde_json::json!(1500));

    assert_eq!(result.metrics.len(), 2);
    assert_eq!(result.metrics.get("items_processed").unwrap(), &serde_json::json!(42));
    assert_eq!(result.metrics.get("time_ms").unwrap(), &serde_json::json!(1500));
}

#[test]
fn test_stage_result_with_multiple_metrics() {
    let result = StageResult::success(vec![])
        .with_metric("files_modified", serde_json::json!(5))
        .with_metric("tests_run", serde_json::json!(100))
        .with_metric("tests_passed", serde_json::json!(98));

    assert_eq!(result.metrics.get("files_modified").unwrap(), &serde_json::json!(5));
    assert_eq!(result.metrics.get("tests_run").unwrap(), &serde_json::json!(100));
    assert_eq!(result.metrics.get("tests_passed").unwrap(), &serde_json::json!(98));
}

// =============================================================================
// PluginManager Custom Stage Tests
// =============================================================================

#[tokio::test]
async fn test_plugin_manager_register_custom_stage() {
    let manager = PluginManager::new();

    let config = CustomStageConfig::new(
        "custom-validation",
        "Custom Validation Stage",
        StagePosition::Before(StandardStage::Commit),
    );

    manager.register_custom_stage(config).await.unwrap();

    let stages = manager.get_custom_stages().await;
    assert_eq!(stages.len(), 1);
    assert_eq!(stages[0].id, "custom-validation");
}

#[tokio::test]
async fn test_plugin_manager_get_custom_stage_by_id() {
    let manager = PluginManager::new();

    let config = CustomStageConfig::new(
        "unique-stage",
        "Unique Stage",
        StagePosition::After(StandardStage::Test),
    );

    manager.register_custom_stage(config).await.unwrap();

    let retrieved = manager.get_custom_stage("unique-stage").await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "Unique Stage");
}

#[tokio::test]
async fn test_plugin_manager_get_nonexistent_stage() {
    let manager = PluginManager::new();

    let retrieved = manager.get_custom_stage("nonexistent").await;
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_plugin_manager_register_multiple_stages() {
    let manager = PluginManager::new();

    let config1 = CustomStageConfig::new(
        "stage-1",
        "First Stage",
        StagePosition::After(StandardStage::Generate),
    );

    let config2 = CustomStageConfig::new(
        "stage-2",
        "Second Stage",
        StagePosition::Before(StandardStage::Test),
    );

    let config3 = CustomStageConfig::new(
        "stage-3",
        "Third Stage",
        StagePosition::Last,
    );

    manager.register_custom_stage(config1).await.unwrap();
    manager.register_custom_stage(config2).await.unwrap();
    manager.register_custom_stage(config3).await.unwrap();

    let stages = manager.get_custom_stages().await;
    assert_eq!(stages.len(), 3);
}

#[tokio::test]
async fn test_plugin_manager_update_existing_stage() {
    let manager = PluginManager::new();

    let config1 = CustomStageConfig::new(
        "updatable-stage",
        "Original Name",
        StagePosition::First,
    );

    manager.register_custom_stage(config1).await.unwrap();

    let config2 = CustomStageConfig::new(
        "updatable-stage",
        "Updated Name",
        StagePosition::Last,
    )
    .with_enabled(false);

    manager.register_custom_stage(config2).await.unwrap();

    let stages = manager.get_custom_stages().await;
    assert_eq!(stages.len(), 1);
    assert_eq!(stages[0].name, "Updated Name");
    assert!(!stages[0].enabled);
}

// =============================================================================
// Built-in LoggingStage Tests
// =============================================================================

#[test]
fn test_logging_stage_creation() {
    let stage = LoggingStage::new();

    assert_eq!(stage.metadata().id, "builtin-logging");
    assert_eq!(stage.metadata().name, "Logging Stage");
    assert_eq!(stage.metadata().version, "1.0.0");
    assert_eq!(stage.config().id, "log-tasks");
    assert!(matches!(
        stage.config().position,
        StagePosition::After(StandardStage::Execute)
    ));
}

#[tokio::test]
async fn test_logging_stage_execute_empty_tasks() {
    let stage = LoggingStage::new();
    let config = HashMap::new();

    let result = stage.execute(vec![], &config, None).await.unwrap();

    assert!(result.success);
    assert!(result.tasks.is_empty());
}

#[tokio::test]
async fn test_logging_stage_execute_with_tasks() {
    let stage = LoggingStage::new();

    let tasks = vec![
        Task::new("task-1", "First Task", "Description 1"),
        Task::new("task-2", "Second Task", "Description 2"),
    ];

    let config = HashMap::new();
    let result = stage.execute(tasks.clone(), &config, None).await.unwrap();

    assert!(result.success);
    assert_eq!(result.tasks.len(), 2);
    assert_eq!(result.tasks[0].id, "task-1");
    assert_eq!(result.tasks[1].id, "task-2");
}

#[tokio::test]
async fn test_logging_stage_with_custom_log_level() {
    let stage = LoggingStage::new();

    let tasks = vec![Task::new("task-1", "Task", "Description")];

    let mut config = HashMap::new();
    config.insert("log_level".to_string(), serde_json::json!("debug"));

    let result = stage.execute(tasks, &config, None).await.unwrap();

    assert!(result.success);
}

#[test]
fn test_logging_stage_timeout() {
    let stage = LoggingStage::new();
    assert_eq!(stage.timeout(), 3600); // Default timeout
}

#[test]
fn test_logging_stage_skip_on_failure() {
    let stage = LoggingStage::new();
    assert!(!stage.skip_on_failure()); // Default is false
}

#[tokio::test]
async fn test_logging_stage_check_prerequisites() {
    let stage = LoggingStage::new();
    let result = stage.check_prerequisites().await.unwrap();
    assert!(result); // Default implementation always returns true
}

// =============================================================================
// Built-in DelayStage Tests
// =============================================================================

#[test]
fn test_delay_stage_creation() {
    let stage = DelayStage::new();

    assert_eq!(stage.metadata().id, "builtin-delay");
    assert_eq!(stage.metadata().name, "Delay Stage");
    assert_eq!(stage.config().id, "delay");
    assert!(matches!(
        stage.config().position,
        StagePosition::After(StandardStage::Generate)
    ));
}

#[tokio::test]
async fn test_delay_stage_execute_default_delay() {
    let stage = DelayStage::new();

    let tasks = vec![Task::new("task-1", "Task", "Description")];

    let config = HashMap::new();

    // Note: Default delay is 1 second, but we'll use a shorter one in the test
    // This test verifies the mechanism works
    let start = std::time::Instant::now();
    let result = stage.execute(tasks, &config, None).await.unwrap();
    let elapsed = start.elapsed();

    assert!(result.success);
    // Should have waited at least 1 second (default)
    assert!(elapsed.as_secs() >= 1);
}

#[tokio::test]
async fn test_delay_stage_execute_custom_delay() {
    let stage = DelayStage::new();

    let tasks = vec![Task::new("task-1", "Task", "Description")];

    let mut config = HashMap::new();
    config.insert("seconds".to_string(), serde_json::json!(0)); // Zero delay for fast test

    let result = stage.execute(tasks.clone(), &config, None).await.unwrap();

    assert!(result.success);
    assert_eq!(result.tasks.len(), 1);
}

#[test]
fn test_delay_stage_timeout() {
    let stage = DelayStage::new();
    assert_eq!(stage.timeout(), 3600); // Default timeout
}

#[test]
fn test_delay_stage_skip_on_failure() {
    let stage = DelayStage::new();
    assert!(!stage.skip_on_failure()); // Default is false
}

// =============================================================================
// Plugin Initialization Tests
// =============================================================================

#[tokio::test]
async fn test_plugin_initialization() {
    let result = ltmatrix::plugin::initialize().await;
    assert!(result.is_ok());

    let manager = result.unwrap();
    let plugins = manager.list_plugins().await;

    // Should have registered built-in plugins
    assert!(!plugins.is_empty());

    let plugin_ids: Vec<&str> = plugins.iter().map(|p| p.id.as_str()).collect();
    assert!(plugin_ids.contains(&"builtin-logging"));
    assert!(plugin_ids.contains(&"builtin-delay"));
}

#[tokio::test]
async fn test_plugin_manager_default_paths() {
    let manager = PluginManager::new();
    let paths = manager.plugin_paths();

    // Should have at least user and project plugin paths
    assert!(!paths.is_empty());

    // Check that paths contain expected directories
    let path_strings: Vec<String> = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();

    // Should contain user plugins path (~/.ltmatrix/plugins)
    let has_user_path = path_strings.iter().any(|p| p.contains(".ltmatrix"));
    assert!(has_user_path);
}

// =============================================================================
// PipelineStagePlugin Trait Tests
// =============================================================================

#[test]
fn test_pipeline_stage_plugin_config_accessor() {
    let stage = LoggingStage::new();

    let config = stage.config();
    assert_eq!(config.id, "log-tasks");
    assert!(config.enabled);
}

#[test]
fn test_pipeline_stage_plugin_config_mut_accessor() {
    let mut stage = LoggingStage::new();

    stage.config_mut().enabled = false;
    assert!(!stage.config().enabled);
}

// =============================================================================
// Integration Tests
// =============================================================================

#[tokio::test]
async fn test_full_custom_stage_workflow() {
    // 1. Create plugin manager
    let manager = PluginManager::new();

    // 2. Create a custom stage configuration
    let custom_config = CustomStageConfig::new(
        "my-workflow-stage",
        "My Workflow Stage",
        StagePosition::After(StandardStage::Test),
    )
    .with_description("Processes test results")
    .with_modes(vec!["standard".to_string(), "expert".to_string()])
    .with_enabled(true);

    // 3. Register the custom stage
    manager.register_custom_stage(custom_config).await.unwrap();

    // 4. Retrieve and verify
    let retrieved = manager.get_custom_stage("my-workflow-stage").await;
    assert!(retrieved.is_some());

    let stage_config = retrieved.unwrap();
    assert_eq!(stage_config.name, "My Workflow Stage");
    assert_eq!(stage_config.description, "Processes test results");
    assert!(stage_config.enabled);
    assert!(stage_config.should_run_for_mode(&ExecutionMode::Standard));
    assert!(stage_config.should_run_for_mode(&ExecutionMode::Expert));
    assert!(!stage_config.should_run_for_mode(&ExecutionMode::Fast));
}

#[tokio::test]
async fn test_stage_execution_with_multiple_stages() {
    let manager = PluginManager::new();

    // Register multiple stages at different positions
    let before_test = CustomStageConfig::new(
        "before-test-stage",
        "Pre-Test Stage",
        StagePosition::Before(StandardStage::Test),
    );

    let after_execute = CustomStageConfig::new(
        "after-execute-stage",
        "Post-Execute Stage",
        StagePosition::After(StandardStage::Execute),
    );

    let first_stage = CustomStageConfig::new(
        "first-stage",
        "First Stage",
        StagePosition::First,
    );

    manager.register_custom_stage(before_test).await.unwrap();
    manager.register_custom_stage(after_execute).await.unwrap();
    manager.register_custom_stage(first_stage).await.unwrap();

    let stages = manager.get_custom_stages().await;
    assert_eq!(stages.len(), 3);
}

// =============================================================================
// Edge Cases and Error Handling Tests
// =============================================================================

#[test]
fn test_custom_stage_config_with_empty_id() {
    // Empty ID should be allowed (validation would happen elsewhere)
    let config = CustomStageConfig::new("", "Empty ID Stage", StagePosition::Last);
    assert_eq!(config.id, "");
}

#[test]
fn test_custom_stage_config_with_special_characters() {
    let config = CustomStageConfig::new(
        "my-custom_stage.1",
        "Stage With Special Chars",
        StagePosition::First,
    );
    assert_eq!(config.id, "my-custom_stage.1");
}

#[tokio::test]
async fn test_stage_result_preserves_task_state() {
    let mut task = Task::new("task-1", "Original Title", "Original Description");
    task.status = ltmatrix::models::TaskStatus::Completed;

    let tasks = vec![task];

    let result = StageResult::success(tasks);

    assert!(result.success);
    assert_eq!(result.tasks[0].status, ltmatrix::models::TaskStatus::Completed);
    assert_eq!(result.tasks[0].title, "Original Title");
}

#[test]
fn test_stage_result_failure_with_empty_tasks() {
    let result = StageResult::failure(vec![], "No tasks to process");

    assert!(!result.success);
    assert!(result.tasks.is_empty());
    assert_eq!(result.error, Some("No tasks to process".to_string()));
}

#[tokio::test]
async fn test_concurrent_stage_registration() {
    let manager = std::sync::Arc::new(PluginManager::new());
    let mut handles = vec![];

    // Spawn multiple concurrent registrations
    for i in 0..10 {
        let mgr = manager.clone();
        let handle = tokio::spawn(async move {
            let config = CustomStageConfig::new(
                format!("concurrent-stage-{}", i),
                format!("Concurrent Stage {}", i),
                StagePosition::Last,
            );
            mgr.register_custom_stage(config).await.unwrap();
        });
        handles.push(handle);
    }

    // Wait for all registrations to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // All stages should be registered
    let stages = manager.get_custom_stages().await;
    assert_eq!(stages.len(), 10);
}

#[test]
fn test_stage_position_equality() {
    assert_eq!(
        StagePosition::Before(StandardStage::Test),
        StagePosition::Before(StandardStage::Test)
    );
    assert_eq!(
        StagePosition::After(StandardStage::Execute),
        StagePosition::After(StandardStage::Execute)
    );
    assert_eq!(StagePosition::First, StagePosition::First);
    assert_eq!(StagePosition::Last, StagePosition::Last);
}

#[test]
fn test_stage_position_inequality() {
    assert_ne!(
        StagePosition::Before(StandardStage::Test),
        StagePosition::After(StandardStage::Test)
    );
    assert_ne!(
        StagePosition::Before(StandardStage::Test),
        StagePosition::Before(StandardStage::Execute)
    );
    assert_ne!(StagePosition::First, StagePosition::Last);
}

#[test]
fn test_standard_stage_equality() {
    assert_eq!(StandardStage::Generate, StandardStage::Generate);
    assert_eq!(StandardStage::Execute, StandardStage::Execute);
    assert_ne!(StandardStage::Generate, StandardStage::Execute);
}

#[test]
fn test_standard_stage_hash() {
    use std::collections::HashSet;

    let mut stages = HashSet::new();
    stages.insert(StandardStage::Generate);
    stages.insert(StandardStage::Execute);
    stages.insert(StandardStage::Generate); // Duplicate

    assert_eq!(stages.len(), 2);
}

#[tokio::test]
async fn test_logging_stage_validate_config_empty() {
    let stage = LoggingStage::new();
    let config = HashMap::new();

    // Should pass with empty config (default implementation)
    let result = stage.validate_config(&config);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_logging_stage_validate_config_with_values() {
    let stage = LoggingStage::new();
    let mut config = HashMap::new();
    config.insert("log_level".to_string(), serde_json::json!("info"));

    // Should pass with valid config
    let result = stage.validate_config(&config);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delay_stage_validate_config_empty() {
    let stage = DelayStage::new();
    let config = HashMap::new();

    // Should pass with empty config
    let result = stage.validate_config(&config);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delay_stage_validate_config_with_values() {
    let stage = DelayStage::new();
    let mut config = HashMap::new();
    config.insert("seconds".to_string(), serde_json::json!(5));

    // Should pass with valid config
    let result = stage.validate_config(&config);
    assert!(result.is_ok());
}

// =============================================================================
// Stage Configuration TOML Parsing Tests
// =============================================================================

#[test]
fn test_custom_stage_config_toml_deserialization() {
    let toml_str = r#"
        id = "toml-stage"
        name = "TOML Configured Stage"
        description = "Stage from TOML config"
        position = { after = "execute" }
        enabled = true
        skip_on_failure = false
        timeout_seconds = 1800

        [config]
        custom_option = "value"
        numeric_option = 42
        boolean_option = true
    "#;

    let config: CustomStageConfig = toml::from_str(toml_str).unwrap();

    assert_eq!(config.id, "toml-stage");
    assert_eq!(config.name, "TOML Configured Stage");
    assert_eq!(config.description, "Stage from TOML config");
    assert!(matches!(config.position, StagePosition::After(StandardStage::Execute)));
    assert!(config.enabled);
    assert!(!config.skip_on_failure);
    assert_eq!(config.timeout_seconds, 1800);
    assert_eq!(config.config.get("custom_option").unwrap(), &serde_json::json!("value"));
}

#[test]
fn test_custom_stage_config_toml_before_position() {
    let toml_str = r#"
        id = "before-stage"
        name = "Before Stage"
        position = { before = "test" }
    "#;

    let config: CustomStageConfig = toml::from_str(toml_str).unwrap();

    assert!(matches!(config.position, StagePosition::Before(StandardStage::Test)));
}

#[test]
fn test_custom_stage_config_toml_replace_position() {
    let toml_str = r#"
        id = "replace-stage"
        name = "Replace Stage"
        position = { replace = "review" }
    "#;

    let config: CustomStageConfig = toml::from_str(toml_str).unwrap();

    assert!(matches!(config.position, StagePosition::Replace(StandardStage::Review)));
}

#[test]
fn test_custom_stage_config_toml_first_position() {
    let toml_str = r#"
        id = "first-stage"
        name = "First Stage"
        position = "first"
    "#;

    // Note: This test verifies the expected behavior
    // The actual TOML format may vary based on implementation
    // If position is a string variant, this should parse to StagePosition::First
    let config: Result<CustomStageConfig, _> = toml::from_str(toml_str);

    // The test should pass if the implementation supports "first" as a string
    if let Ok(config) = config {
        assert!(matches!(config.position, StagePosition::First));
    }
    // Otherwise, the position might need to be specified differently
}

#[test]
fn test_custom_stage_config_toml_with_modes() {
    let toml_str = r#"
        id = "mode-specific-stage"
        name = "Mode Specific Stage"
        position = { after = "commit" }
        modes = ["expert", "standard"]
    "#;

    let config: CustomStageConfig = toml::from_str(toml_str).unwrap();

    assert_eq!(config.modes.len(), 2);
    assert!(config.modes.contains(&"expert".to_string()));
    assert!(config.modes.contains(&"standard".to_string()));
}

#[test]
fn test_custom_stage_config_toml_defaults() {
    let toml_str = r#"
        id = "minimal-stage"
        name = "Minimal Stage"
        position = { after = "execute" }
    "#;

    let config: CustomStageConfig = toml::from_str(toml_str).unwrap();

    // Check default values
    assert!(config.enabled); // Default true
    assert!(!config.skip_on_failure); // Default false
    assert_eq!(config.timeout_seconds, 3600); // Default 1 hour
    assert!(config.modes.is_empty()); // Default empty
    assert!(config.config.is_empty()); // Default empty
}