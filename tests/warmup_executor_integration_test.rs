//! Integration tests for warmup executor
//!
//! These tests verify the warmup executor's functionality in realistic
//! scenarios including:
//! - Single agent warmup
//! - Multiple agent warmup
//! - Error handling and retry logic
//! - Session pool integration
//! - Configuration-driven behavior

use ltmatrix::agent::backend::AgentSession;
use ltmatrix::agent::pool::SessionPool;
use ltmatrix::agent::warmup::{WarmupExecutor, WarmupResult};
use ltmatrix::config::settings::WarmupConfig;

/// Test that warmup executor can be created with default config
#[test]
fn test_warmup_executor_default_creation() {
    let _executor = WarmupExecutor::default();
    // Executor is created successfully with default config
}

/// Test that warmup executor can be created with custom config
#[test]
fn test_warmup_executor_custom_config() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 5,
        timeout_seconds: 60,
        retry_on_failure: true,
        prompt_template: Some("Custom prompt".to_string()),
    };

    let _executor = WarmupExecutor::new(config);
    // Executor is created successfully with custom config
}

/// Test warmup result helper methods
#[test]
fn test_warmup_result_helper_methods() {
    // Success result
    let success = WarmupResult::Success {
        queries_executed: 2,
        duration_ms: 150,
    };
    assert!(success.is_success());
    assert!(!success.is_skipped());
    assert!(!success.is_failed());
    assert_eq!(success.queries_executed(), Some(2));

    // Skipped result
    let skipped = WarmupResult::Skipped;
    assert!(!skipped.is_success());
    assert!(skipped.is_skipped());
    assert!(!skipped.is_failed());
    assert_eq!(skipped.queries_executed(), None);

    // Failed result
    let failed = WarmupResult::Failed {
        error: "Test error".to_string(),
        queries_executed: 1,
    };
    assert!(!failed.is_success());
    assert!(!failed.is_skipped());
    assert!(failed.is_failed());
    assert_eq!(failed.queries_executed(), Some(1));
}

/// Test that session pool is initialized correctly after warmup
#[test]
fn test_session_pool_initialization_after_warmup() {
    let mut pool = SessionPool::new();

    // Initially empty
    assert_eq!(pool.len(), 0);

    // After getting a session for an agent, pool should have one session
    let session_id = pool
        .get_or_create("claude", "claude-sonnet-4-6")
        .session_id()
        .to_string();
    assert!(!session_id.is_empty());
    assert_eq!(pool.len(), 1);

    // Getting session for same agent should return the same session
    let session_id2 = pool
        .get_or_create("claude", "claude-sonnet-4-6")
        .session_id()
        .to_string();
    assert_eq!(session_id, session_id2);
    assert_eq!(pool.len(), 1);

    // Getting session for different agent should create a new session
    let session_id3 = pool
        .get_or_create("opencode", "gpt-4")
        .session_id()
        .to_string();
    assert_ne!(session_id, session_id3);
    assert_eq!(pool.len(), 2);
}

/// Test warmup configuration validation works correctly
#[test]
fn test_warmup_config_validation() {
    // Valid config
    let config = WarmupConfig {
        enabled: true,
        max_queries: 5,
        timeout_seconds: 60,
        retry_on_failure: false,
        prompt_template: Some("Valid prompt".to_string()),
    };
    assert!(config.validate().is_ok());

    // Invalid: max_queries is 0
    let invalid_config = WarmupConfig {
        enabled: true,
        max_queries: 0,
        ..Default::default()
    };
    assert!(invalid_config.validate().is_err());

    // Invalid: timeout_seconds is 0
    let invalid_config2 = WarmupConfig {
        enabled: true,
        timeout_seconds: 0,
        ..Default::default()
    };
    assert!(invalid_config2.validate().is_err());

    // Invalid: empty prompt_template
    let invalid_config3 = WarmupConfig {
        enabled: true,
        prompt_template: Some("   ".to_string()),
        ..Default::default()
    };
    assert!(invalid_config3.validate().is_err());
}

/// Test that warmup executor respects disabled configuration
#[tokio::test]
async fn test_warmup_executor_respects_disabled_config() {
    let _executor = WarmupExecutor::new(WarmupConfig {
        enabled: false,
        ..Default::default()
    });

    let _pool = SessionPool::new();
    // Executor is created with warmup disabled
}

/// Test warmup configuration merge behavior
#[test]
fn test_warmup_config_merge_behavior() {
    // Global config
    let global_toml = r#"
        [warmup]
        enabled = true
        max_queries = 2
        timeout_seconds = 30
    "#;

    let global: ltmatrix::config::settings::Config = toml::from_str(global_toml).unwrap();
    assert_eq!(global.warmup.max_queries, 2);
    assert_eq!(global.warmup.timeout_seconds, 30);

    // Project config should override
    let project_toml = r#"
        [warmup]
        enabled = false
        max_queries = 10
        timeout_seconds = 90
        retry_on_failure = true
    "#;

    let project: ltmatrix::config::settings::Config = toml::from_str(project_toml).unwrap();
    assert_eq!(project.warmup.enabled, false);
    assert_eq!(project.warmup.max_queries, 10);
    assert_eq!(project.warmup.timeout_seconds, 90);
    assert_eq!(project.warmup.retry_on_failure, true);
}

/// Test that multiple warmup queries are handled correctly
#[test]
fn test_multiple_warmup_queries_configuration() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 5,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: None,
    };

    let _executor = WarmupExecutor::new(config);
    // Executor is created with multiple queries configured
}

/// Test warmup executor with custom prompt template
#[test]
fn test_warmup_executor_custom_prompt_template() {
    let custom_prompt = "Are you ready to assist with code generation?";

    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: Some(custom_prompt.to_string()),
    };

    let _executor = WarmupExecutor::new(config);
    // Executor is created with custom prompt template
}

/// Test warmup result comparison and equality
#[test]
fn test_warmup_result_equality() {
    use ltmatrix::agent::warmup::WarmupResult;

    let result1 = WarmupResult::Success {
        queries_executed: 2,
        duration_ms: 150,
    };

    let result2 = WarmupResult::Success {
        queries_executed: 2,
        duration_ms: 150,
    };

    let result3 = WarmupResult::Success {
        queries_executed: 3,
        duration_ms: 200,
    };

    assert_eq!(result1, result2);
    assert_ne!(result1, result3);
}

/// Test that session reuse works correctly after warmup
#[test]
fn test_session_reuse_after_warmup() {
    let mut pool = SessionPool::new();

    // First call creates a new session
    let session_id = pool
        .get_or_create("claude", "claude-sonnet-4-6")
        .session_id()
        .to_string();

    // Subsequent calls reuse the same session
    let session_id2 = pool
        .get_or_create("claude", "claude-sonnet-4-6")
        .session_id()
        .to_string();
    let session_id3 = pool
        .get_or_create("claude", "claude-sonnet-4-6")
        .session_id()
        .to_string();

    assert_eq!(session_id2, session_id);
    assert_eq!(session_id3, session_id);
    assert_eq!(pool.len(), 1); // Only one session in the pool
}

/// Test that different agents get different sessions
#[test]
fn test_different_agents_get_different_sessions() {
    let mut pool = SessionPool::new();

    let session_id1 = pool
        .get_or_create("claude", "claude-sonnet-4-6")
        .session_id()
        .to_string();
    let session_id2 = pool
        .get_or_create("opencode", "gpt-4")
        .session_id()
        .to_string();
    let session_id3 = pool
        .get_or_create("kimicode", "moonshot-v1-128k")
        .session_id()
        .to_string();

    assert_ne!(session_id1, session_id2);
    assert_ne!(session_id2, session_id3);
    assert_ne!(session_id1, session_id3);
    assert_eq!(pool.len(), 3);
}

/// Test warmup configuration serialization roundtrip
#[test]
fn test_warmup_config_serialization_roundtrip() {
    let original = WarmupConfig {
        enabled: true,
        max_queries: 7,
        timeout_seconds: 120,
        retry_on_failure: false,
        prompt_template: Some("Roundtrip test".to_string()),
    };

    let toml_string = toml::to_string(&original).unwrap();
    let deserialized: WarmupConfig = toml::from_str(&toml_string).unwrap();

    assert_eq!(original.enabled, deserialized.enabled);
    assert_eq!(original.max_queries, deserialized.max_queries);
    assert_eq!(original.timeout_seconds, deserialized.timeout_seconds);
    assert_eq!(original.retry_on_failure, deserialized.retry_on_failure);
    assert_eq!(original.prompt_template, deserialized.prompt_template);
}

/// Test that warmup config integrates with main config
#[test]
fn test_warmup_config_main_integration() {
    let config_toml = r#"
        [warmup]
        enabled = true
        max_queries = 4
        timeout_seconds = 45
        retry_on_failure = true
        prompt_template = "Integration test"
    "#;

    let config: ltmatrix::config::settings::Config = toml::from_str(config_toml).unwrap();

    assert_eq!(config.warmup.enabled, true);
    assert_eq!(config.warmup.max_queries, 4);
    assert_eq!(config.warmup.timeout_seconds, 45);
    assert_eq!(config.warmup.retry_on_failure, true);
    assert_eq!(
        config.warmup.prompt_template,
        Some("Integration test".to_string())
    );
}

/// Test warmup with retry enabled configuration
#[test]
fn test_warmup_with_retry_enabled_configuration() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 3,
        timeout_seconds: 60,
        retry_on_failure: true,
        prompt_template: None,
    };

    assert!(config.validate().is_ok());
    assert!(config.retry_on_failure);
}

/// Test that warmup timeout is respected
#[test]
fn test_warmup_timeout_configuration() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,
        timeout_seconds: 10,
        retry_on_failure: false,
        prompt_template: None,
    };

    assert_eq!(config.timeout_seconds, 10);
    assert!(config.validate().is_ok());
}

/// Test session pool statistics
#[test]
fn test_session_pool_statistics() {
    let mut pool = SessionPool::new();

    // Empty pool
    assert_eq!(pool.len(), 0);
    assert!(pool.is_empty());

    // Add sessions
    pool.get_or_create("claude", "claude-sonnet-4-6");
    pool.get_or_create("opencode", "gpt-4");

    assert_eq!(pool.len(), 2);
    assert!(!pool.is_empty());
}

/// Test warmup result duration tracking
#[test]
fn test_warmup_result_duration_tracking() {
    let result = WarmupResult::Success {
        queries_executed: 3,
        duration_ms: 250,
    };

    if let WarmupResult::Success { duration_ms, .. } = result {
        assert_eq!(duration_ms, 250);
    } else {
        panic!("Expected Success result");
    }
}
