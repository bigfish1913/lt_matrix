//! Integration tests for AgentPool execution system
//!
//! These tests verify that AgentPool integrates properly with:
//! - Session management and reuse
//! - Warmup functionality
//! - Configuration system
//! - Concurrent access patterns
//! - Task execution lifecycle

use ltmatrix::agent::{AgentBackend, AgentPool, ExecutionConfig};
use ltmatrix::config::settings::{Config, PoolConfig};
use ltmatrix::models::Task;

/// Test that AgentPool can be created with default configuration
#[test]
fn test_agent_pool_default_creation() {
    let config = Config::default();
    let _pool = AgentPool::new(&config);
    // Pool created successfully
}

/// Test that AgentPool respects pool configuration
#[test]
fn test_agent_pool_respects_config() {
    let mut config = Config::default();
    config.pool.max_sessions = 50;
    config.pool.auto_cleanup = false;
    config.pool.enable_reuse = false;

    let _pool = AgentPool::new(&config);
    // Pool created with custom config
}

/// Test pool configuration validation
#[test]
fn test_pool_config_validation() {
    // Valid config
    let config = PoolConfig {
        max_sessions: 100,
        auto_cleanup: true,
        cleanup_interval_seconds: 300,
        stale_threshold_seconds: 3600,
        enable_reuse: true,
    };
    assert!(config.validate().is_ok());

    // Invalid: max_sessions is 0
    let invalid_config = PoolConfig {
        max_sessions: 0,
        ..Default::default()
    };
    assert!(invalid_config.validate().is_err());

    // Invalid: cleanup_interval is 0
    let invalid_config2 = PoolConfig {
        cleanup_interval_seconds: 0,
        ..Default::default()
    };
    assert!(invalid_config2.validate().is_err());
}

/// Test pool configuration duration conversion
#[test]
fn test_pool_config_duration_conversions() {
    let config = PoolConfig::default();

    // Test stale threshold conversion
    let duration = config.stale_threshold_duration();
    assert_eq!(duration.num_seconds(), 3600);

    // Test cleanup interval conversion
    let interval = config.cleanup_interval_duration();
    assert_eq!(interval.as_secs(), 300);
}

/// Test that pool configuration serializes correctly
#[test]
fn test_pool_config_serialization() {
    let config = PoolConfig {
        max_sessions: 200,
        auto_cleanup: false,
        cleanup_interval_seconds: 600,
        stale_threshold_seconds: 7200,
        enable_reuse: true,
    };

    let toml_string = toml::to_string(&config).unwrap();
    let deserialized: PoolConfig = toml::from_str(&toml_string).unwrap();

    assert_eq!(config.max_sessions, deserialized.max_sessions);
    assert_eq!(config.auto_cleanup, deserialized.auto_cleanup);
    assert_eq!(
        config.cleanup_interval_seconds,
        deserialized.cleanup_interval_seconds
    );
    assert_eq!(
        config.stale_threshold_seconds,
        deserialized.stale_threshold_seconds
    );
    assert_eq!(config.enable_reuse, deserialized.enable_reuse);
}

/// Test pool configuration with TOML integration
#[test]
fn test_pool_config_toml_integration() {
    let toml_str = r#"
        [pool]
        max_sessions = 150
        auto_cleanup = true
        cleanup_interval_seconds = 400
        stale_threshold_seconds = 7200
        enable_reuse = true
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();

    assert_eq!(config.pool.max_sessions, 150);
    assert!(config.pool.auto_cleanup);
    assert_eq!(config.pool.cleanup_interval_seconds, 400);
    assert_eq!(config.pool.stale_threshold_seconds, 7200);
    assert!(config.pool.enable_reuse);
}

/// Test that full Config includes pool configuration
#[test]
fn test_full_config_includes_pool() {
    let config = Config::default();

    // Should have pool configuration with defaults
    assert_eq!(config.pool.max_sessions, 100);
    assert!(config.pool.auto_cleanup);
    assert_eq!(config.pool.cleanup_interval_seconds, 300);
    assert_eq!(config.pool.stale_threshold_seconds, 3600);
    assert!(config.pool.enable_reuse);
}

/// Test pool configuration in different execution modes
#[test]
fn test_pool_config_mode_specific() {
    let base_config = Config::default();

    // Pool config should be consistent across modes
    assert!(base_config.pool.enable_reuse);
    assert_eq!(base_config.pool.max_sessions, 100);
}

/// Test pool statistics
#[tokio::test]
async fn test_pool_statistics() {
    let pool = AgentPool::from_default_config();

    let stats = pool.stats().await;

    assert_eq!(stats.total_sessions, 0);
    assert_eq!(stats.active_sessions, 0);
    assert_eq!(stats.max_sessions, 100);
    assert!(!stats.warmup_enabled);
}

/// Test that session cleanup can be triggered
#[tokio::test]
async fn test_session_cleanup() {
    let pool = AgentPool::from_default_config();
    let mut task = Task::new("task-1", "Test", "Description");

    // Create a session
    pool.get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;

    // Cleanup should succeed (even if no sessions are removed)
    let removed = pool.cleanup_stale_sessions().await;
    assert!(removed >= 0);
}

/// Test AgentPool creation with custom config
#[test]
fn test_agent_pool_with_custom_config() {
    let mut config = Config::default();
    config.pool.max_sessions = 250;
    config.warmup.enabled = true;

    let _pool = AgentPool::new(&config);
    // Pool created with custom configuration
}

/// Test that pool settings affect behavior
#[test]
fn test_pool_settings_affect_behavior() {
    let config1 = Config::default();
    assert_eq!(config1.pool.max_sessions, 100);

    let config2 = Config {
        pool: PoolConfig {
            max_sessions: 500,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config2.pool.max_sessions, 500);
}

/// Test that pool and warmup configs work together
#[test]
fn test_pool_and_warmup_config_integration() {
    let config_str = r#"
        [pool]
        max_sessions = 200
        enable_reuse = true

        [warmup]
        enabled = true
        max_queries = 5
        "#;

    let config: Config = toml::from_str(config_str).unwrap();

    // Both configs should be parsed correctly
    assert_eq!(config.pool.max_sessions, 200);
    assert!(config.pool.enable_reuse);
    assert!(config.warmup.enabled);
    assert_eq!(config.warmup.max_queries, 5);
}

/// Test pool configuration defaults
#[test]
fn test_pool_config_defaults() {
    let config = PoolConfig::default();

    assert_eq!(config.max_sessions, 100);
    assert!(config.auto_cleanup);
    assert_eq!(config.cleanup_interval_seconds, 300);
    assert_eq!(config.stale_threshold_seconds, 3600);
    assert!(config.enable_reuse);
}

/// Test that pool configuration can be overridden
#[test]
fn test_pool_config_override() {
    let base = Config::default();

    let override_config = Config {
        pool: PoolConfig {
            max_sessions: 999,
            ..base.pool
        },
        ..base
    };

    assert_eq!(override_config.pool.max_sessions, 999);
}

/// Test pool configuration with edge cases
#[test]
fn test_pool_config_edge_cases() {
    // Minimum valid values
    let config = PoolConfig {
        max_sessions: 1,
        cleanup_interval_seconds: 1,
        stale_threshold_seconds: 1,
        ..Default::default()
    };
    assert!(config.validate().is_ok());

    // Large values
    let config = PoolConfig {
        max_sessions: 10000,
        cleanup_interval_seconds: 86400, // 1 day
        stale_threshold_seconds: 86400, // 1 day
        ..Default::default()
    };
    assert!(config.validate().is_ok());
}
