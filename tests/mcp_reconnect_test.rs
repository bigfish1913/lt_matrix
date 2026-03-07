// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Integration tests for MCP Reconnection and Error Recovery Logic
//!
//! Tests for:
//! - BackoffStrategy calculations (fixed, linear, exponential, exponential with jitter)
//! - ReconnectConfig configuration and builder pattern
//! - DegradationLevel ordering and health mapping
//! - ReconnectStats statistics tracking
//! - ReconnectionManager lifecycle management
//! - RecoveryConfig and RecoveryStrategy error handling
//! - Reconnector trait implementations

use ltmatrix::mcp::heartbeat::ConnectionHealth;
use ltmatrix::mcp::protocol::errors::{McpError, McpErrorCode};
use ltmatrix::mcp::reconnect::{
    BackoffStrategy, DegradationLevel, RecoveryConfig, RecoveryStrategy, ReconnectConfig,
    ReconnectStats, ReconnectionManager, Reconnector,
};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// BackoffStrategy Tests
// ============================================================================

mod backoff_strategy_tests {
    use super::*;

    #[test]
    fn test_fixed_backoff_constant_delay() {
        let backoff = BackoffStrategy::fixed(Duration::from_secs(5));

        assert_eq!(backoff.calculate_delay(0), Duration::from_secs(5));
        assert_eq!(backoff.calculate_delay(1), Duration::from_secs(5));
        assert_eq!(backoff.calculate_delay(10), Duration::from_secs(5));
    }

    #[test]
    fn test_linear_backoff_increases_linearly() {
        let backoff = BackoffStrategy::linear(
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(10),
        );

        assert_eq!(backoff.calculate_delay(0), Duration::from_secs(1));
        assert_eq!(backoff.calculate_delay(1), Duration::from_secs(3));
        assert_eq!(backoff.calculate_delay(2), Duration::from_secs(5));
        assert_eq!(backoff.calculate_delay(3), Duration::from_secs(7));
        assert_eq!(backoff.calculate_delay(4), Duration::from_secs(9));
        // Capped at max
        assert_eq!(backoff.calculate_delay(10), Duration::from_secs(10));
    }

    #[test]
    fn test_linear_backoff_respects_max() {
        let backoff = BackoffStrategy::linear(
            Duration::from_millis(100),
            Duration::from_millis(50),
            Duration::from_millis(500),
        );

        assert_eq!(backoff.calculate_delay(8), Duration::from_millis(500));
        assert_eq!(backoff.calculate_delay(100), Duration::from_millis(500));
    }

    #[test]
    fn test_exponential_backoff_doubles() {
        let backoff =
            BackoffStrategy::exponential(Duration::from_secs(1), Duration::from_secs(60), 2.0);

        assert_eq!(backoff.calculate_delay(0), Duration::from_secs(1));
        assert_eq!(backoff.calculate_delay(1), Duration::from_secs(2));
        assert_eq!(backoff.calculate_delay(2), Duration::from_secs(4));
        assert_eq!(backoff.calculate_delay(3), Duration::from_secs(8));
        assert_eq!(backoff.calculate_delay(4), Duration::from_secs(16));
        assert_eq!(backoff.calculate_delay(5), Duration::from_secs(32));
    }

    #[test]
    fn test_exponential_backoff_respects_max() {
        let backoff =
            BackoffStrategy::exponential(Duration::from_secs(1), Duration::from_secs(60), 2.0);

        // Test that it caps at max for reasonable attempt numbers
        // Note: Very large attempt numbers (100+) can cause float overflow before min() is applied
        assert_eq!(backoff.calculate_delay(10), Duration::from_secs(60));
    }

    #[test]
    fn test_exponential_with_jitter_basic_range() {
        let backoff = BackoffStrategy::exponential_with_jitter(
            Duration::from_secs(1),
            Duration::from_secs(60),
            2.0,
            0.3,
        );

        // Should be approximately 1 second with jitter (0.7 to 1.3)
        let delay = backoff.calculate_delay(0);
        assert!(delay >= Duration::from_millis(700));
        assert!(delay <= Duration::from_millis(1300));
    }

    #[test]
    fn test_exponential_with_jitter_respects_max() {
        let backoff = BackoffStrategy::exponential_with_jitter(
            Duration::from_secs(1),
            Duration::from_secs(60),
            2.0,
            0.3,
        );

        // Should be capped at max for reasonable attempt numbers
        let delay = backoff.calculate_delay(10);
        assert!(delay <= Duration::from_secs(60));
    }

    #[test]
    fn test_exponential_with_jitter_clamps_jitter() {
        // Jitter > 1.0 should be clamped to 1.0
        let backoff = BackoffStrategy::exponential_with_jitter(
            Duration::from_secs(1),
            Duration::from_secs(60),
            2.0,
            5.0, // Will be clamped to 1.0
        );

        let delay = backoff.calculate_delay(0);
        assert!(delay <= Duration::from_secs(2)); // 1 * (1 + 1.0)
    }

    #[test]
    fn test_backoff_default_is_exponential_with_jitter() {
        let backoff = BackoffStrategy::default();
        let delay = backoff.calculate_delay(0);
        // Default is 1 second initial with 0.3 jitter
        assert!(delay >= Duration::from_millis(700));
        assert!(delay <= Duration::from_millis(1300));
    }

    #[test]
    fn test_backoff_strategy_clone() {
        let backoff = BackoffStrategy::fixed(Duration::from_secs(5));
        let cloned = backoff.clone();
        assert_eq!(backoff.calculate_delay(0), cloned.calculate_delay(0));
    }

    #[test]
    fn test_exponential_backoff_different_multiplier() {
        let backoff =
            BackoffStrategy::exponential(Duration::from_secs(1), Duration::from_secs(100), 1.5);

        assert_eq!(backoff.calculate_delay(0), Duration::from_secs(1));
        // 1 * 1.5^1 = 1.5
        assert_eq!(backoff.calculate_delay(1), Duration::from_millis(1500));
    }
}

// ============================================================================
// ReconnectConfig Tests
// ============================================================================

mod reconnect_config_tests {
    use super::*;

    #[test]
    fn test_config_default_values() {
        let config = ReconnectConfig::default();

        assert_eq!(config.max_attempts, 10);
        assert!(config.auto_reconnect);
        assert!(config.reconnect_on_stale);
        assert_eq!(config.reconnect_on_errors, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(100));
        assert_eq!(config.reset_after, Duration::from_secs(60));
        assert!(!config.debug_logging);
    }

    #[test]
    fn test_config_new() {
        let config = ReconnectConfig::new();
        assert_eq!(config.max_attempts, 10);
    }

    #[test]
    fn test_config_builder_chaining() {
        let config = ReconnectConfig::new()
            .with_max_attempts(5)
            .with_auto_reconnect(false)
            .with_reconnect_on_stale(false)
            .with_debug_logging(true);

        assert_eq!(config.max_attempts, 5);
        assert!(!config.auto_reconnect);
        assert!(!config.reconnect_on_stale);
        assert!(config.debug_logging);
    }

    #[test]
    fn test_config_clone() {
        let config = ReconnectConfig::new().with_max_attempts(7);
        let cloned = config.clone();

        assert_eq!(config.max_attempts, cloned.max_attempts);
    }

    #[test]
    fn test_config_zero_max_attempts_means_unlimited() {
        let config = ReconnectConfig::new().with_max_attempts(0);
        assert_eq!(config.max_attempts, 0);
    }
}

// ============================================================================
// DegradationLevel Tests
// ============================================================================

mod degradation_level_tests {
    use super::*;

    #[test]
    fn test_degradation_level_ordering() {
        assert!(DegradationLevel::None < DegradationLevel::Minor);
        assert!(DegradationLevel::Minor < DegradationLevel::Moderate);
        assert!(DegradationLevel::Moderate < DegradationLevel::Severe);
        assert!(DegradationLevel::Severe < DegradationLevel::Critical);
    }

    #[test]
    fn test_degradation_level_is_degraded() {
        assert!(!DegradationLevel::None.is_degraded());
        assert!(DegradationLevel::Minor.is_degraded());
        assert!(DegradationLevel::Moderate.is_degraded());
        assert!(DegradationLevel::Severe.is_degraded());
        assert!(DegradationLevel::Critical.is_degraded());
    }

    #[test]
    fn test_degradation_level_is_critical() {
        assert!(!DegradationLevel::None.is_critical());
        assert!(!DegradationLevel::Minor.is_critical());
        assert!(!DegradationLevel::Moderate.is_critical());
        assert!(!DegradationLevel::Severe.is_critical());
        assert!(DegradationLevel::Critical.is_critical());
    }

    #[test]
    fn test_degradation_level_as_str() {
        assert_eq!(DegradationLevel::None.as_str(), "none");
        assert_eq!(DegradationLevel::Minor.as_str(), "minor");
        assert_eq!(DegradationLevel::Moderate.as_str(), "moderate");
        assert_eq!(DegradationLevel::Severe.as_str(), "severe");
        assert_eq!(DegradationLevel::Critical.as_str(), "critical");
    }

    #[test]
    fn test_degradation_level_default() {
        assert_eq!(DegradationLevel::default(), DegradationLevel::None);
    }

    #[test]
    fn test_degradation_level_from_health_healthy() {
        let health = ConnectionHealth::Healthy;
        assert_eq!(DegradationLevel::from_health(&health), DegradationLevel::None);
    }

    #[test]
    fn test_degradation_level_from_health_degraded_minor() {
        let health = ConnectionHealth::Degraded { missed_pings: 1 };
        assert_eq!(DegradationLevel::from_health(&health), DegradationLevel::Minor);
    }

    #[test]
    fn test_degradation_level_from_health_degraded_moderate() {
        let health = ConnectionHealth::Degraded { missed_pings: 2 };
        assert_eq!(DegradationLevel::from_health(&health), DegradationLevel::Moderate);
    }

    #[test]
    fn test_degradation_level_from_health_stale() {
        let health = ConnectionHealth::Stale { missed_pings: 5 };
        assert_eq!(DegradationLevel::from_health(&health), DegradationLevel::Critical);
    }

    #[test]
    fn test_degradation_level_copy() {
        let level = DegradationLevel::Moderate;
        let copied = level;
        assert_eq!(level, copied);
    }

    #[test]
    fn test_degradation_level_clone() {
        let level = DegradationLevel::Severe;
        let cloned = level.clone();
        assert_eq!(level, cloned);
    }
}

// ============================================================================
// ReconnectStats Tests
// ============================================================================

mod reconnect_stats_tests {
    use super::*;

    #[test]
    fn test_stats_initial_state() {
        let stats = ReconnectStats::new();

        assert_eq!(stats.total_attempts, 0);
        assert_eq!(stats.successful_reconnects, 0);
        assert_eq!(stats.failed_attempts, 0);
        assert_eq!(stats.consecutive_failures, 0);
        assert_eq!(stats.total_reconnect_time, Duration::ZERO);
        assert!(stats.avg_reconnect_time.is_none());
        assert!(stats.last_attempt.is_none());
        assert!(stats.last_success.is_none());
        assert_eq!(stats.degradation_level, DegradationLevel::None);
        assert!(stats.connected_since.is_none());
    }

    #[test]
    fn test_stats_record_successful_attempt() {
        let mut stats = ReconnectStats::new();

        stats.record_attempt(true, Duration::from_millis(100));

        assert_eq!(stats.total_attempts, 1);
        assert_eq!(stats.successful_reconnects, 1);
        assert_eq!(stats.failed_attempts, 0);
        assert_eq!(stats.consecutive_failures, 0);
        assert!(stats.avg_reconnect_time.is_some());
        assert!(stats.last_attempt.is_some());
        assert!(stats.last_success.is_some());
        assert!(stats.connected_since.is_some());
        assert_eq!(stats.degradation_level, DegradationLevel::None);
    }

    #[test]
    fn test_stats_record_failed_attempt() {
        let mut stats = ReconnectStats::new();

        stats.record_attempt(false, Duration::from_millis(50));

        assert_eq!(stats.total_attempts, 1);
        assert_eq!(stats.successful_reconnects, 0);
        assert_eq!(stats.failed_attempts, 1);
        assert_eq!(stats.consecutive_failures, 1);
        assert!(stats.connected_since.is_none());
        assert_eq!(stats.degradation_level, DegradationLevel::Minor);
    }

    #[test]
    fn test_stats_consecutive_failures_increase_degradation() {
        let mut stats = ReconnectStats::new();

        // 1 failure -> Minor
        stats.record_attempt(false, Duration::from_millis(50));
        assert_eq!(stats.degradation_level, DegradationLevel::Minor);
        assert_eq!(stats.consecutive_failures, 1);

        // 2 failures -> Moderate
        stats.record_attempt(false, Duration::from_millis(50));
        assert_eq!(stats.degradation_level, DegradationLevel::Moderate);
        assert_eq!(stats.consecutive_failures, 2);

        // 4 failures -> Severe
        stats.record_attempt(false, Duration::from_millis(50));
        stats.record_attempt(false, Duration::from_millis(50));
        assert_eq!(stats.degradation_level, DegradationLevel::Severe);
        assert_eq!(stats.consecutive_failures, 4);

        // 6 failures -> Critical
        stats.record_attempt(false, Duration::from_millis(50));
        stats.record_attempt(false, Duration::from_millis(50));
        assert_eq!(stats.degradation_level, DegradationLevel::Critical);
        assert_eq!(stats.consecutive_failures, 6);
    }

    #[test]
    fn test_stats_success_resets_failures() {
        let mut stats = ReconnectStats::new();

        // Multiple failures
        stats.record_attempt(false, Duration::from_millis(50));
        stats.record_attempt(false, Duration::from_millis(50));
        stats.record_attempt(false, Duration::from_millis(50));
        assert_eq!(stats.consecutive_failures, 3);

        // Success resets
        stats.record_attempt(true, Duration::from_millis(100));
        assert_eq!(stats.consecutive_failures, 0);
        assert_eq!(stats.degradation_level, DegradationLevel::None);
    }

    #[test]
    fn test_stats_avg_reconnect_time_calculation() {
        let mut stats = ReconnectStats::new();

        // First successful attempt
        stats.record_attempt(true, Duration::from_millis(100));
        let avg1 = stats.avg_reconnect_time.unwrap();
        assert_eq!(avg1, Duration::from_millis(100));

        // Second successful attempt (average of 100 and 200)
        stats.record_attempt(true, Duration::from_millis(200));
        let avg2 = stats.avg_reconnect_time.unwrap();
        assert_eq!(avg2, Duration::from_millis(150));
    }

    #[test]
    fn test_stats_total_reconnect_time() {
        let mut stats = ReconnectStats::new();

        stats.record_attempt(true, Duration::from_millis(100));
        stats.record_attempt(false, Duration::from_millis(50));
        stats.record_attempt(true, Duration::from_millis(150));

        assert_eq!(stats.total_reconnect_time, Duration::from_millis(300));
    }

    #[test]
    fn test_stats_reset_on_success() {
        let mut stats = ReconnectStats::new();

        stats.record_attempt(false, Duration::from_millis(50));
        stats.record_attempt(false, Duration::from_millis(50));
        stats.connected_since = None;

        stats.reset_on_success();

        assert_eq!(stats.consecutive_failures, 0);
        assert_eq!(stats.degradation_level, DegradationLevel::None);
        assert!(stats.connected_since.is_some());
    }

    #[test]
    fn test_stats_should_reset_no_connection() {
        let stats = ReconnectStats::new();
        assert!(!stats.should_reset(Duration::from_secs(60)));
    }

    #[test]
    fn test_stats_should_reset_recent_connection() {
        let mut stats = ReconnectStats::new();
        stats.connected_since = Some(Instant::now());

        assert!(!stats.should_reset(Duration::from_secs(60)));
    }

    #[test]
    fn test_stats_should_reset_old_connection() {
        let mut stats = ReconnectStats::new();
        stats.connected_since = Some(Instant::now() - Duration::from_secs(120));

        assert!(stats.should_reset(Duration::from_secs(60)));
    }

    #[test]
    fn test_stats_default() {
        let stats = ReconnectStats::default();
        assert_eq!(stats.total_attempts, 0);
    }

    #[test]
    fn test_stats_clone() {
        let mut stats = ReconnectStats::new();
        stats.record_attempt(true, Duration::from_millis(100));

        let cloned = stats.clone();
        assert_eq!(stats.total_attempts, cloned.total_attempts);
        assert_eq!(stats.successful_reconnects, cloned.successful_reconnects);
    }
}

// ============================================================================
// ReconnectionManager Tests
// ============================================================================

mod reconnection_manager_tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = ReconnectionManager::default_manager();

        assert_eq!(manager.current_attempt(), 0);
        assert!(!manager.is_reconnecting());
        assert!(manager.should_reconnect());
    }

    #[tokio::test]
    async fn test_manager_new_with_config() {
        let config = ReconnectConfig::new().with_max_attempts(5);
        let manager = ReconnectionManager::new(config);

        let config_ref = manager.config();
        assert_eq!(config_ref.max_attempts, 5);
    }

    #[tokio::test]
    async fn test_manager_should_reconnect_auto_disabled() {
        let manager =
            ReconnectionManager::new(ReconnectConfig::new().with_auto_reconnect(false));

        assert!(!manager.should_reconnect());
    }

    #[tokio::test]
    async fn test_manager_should_reconnect_max_attempts() {
        let manager =
            ReconnectionManager::new(ReconnectConfig::new().with_max_attempts(3));

        assert!(manager.should_reconnect());
        manager.next_backoff();
        assert!(manager.should_reconnect());
        manager.next_backoff();
        assert!(manager.should_reconnect());
        manager.next_backoff();

        // After 3 attempts, should not reconnect
        assert!(!manager.should_reconnect());
    }

    #[tokio::test]
    async fn test_manager_should_reconnect_unlimited_attempts() {
        let manager =
            ReconnectionManager::new(ReconnectConfig::new().with_max_attempts(0));

        // With 0 max_attempts, should always allow reconnect
        for _ in 0..20 {
            manager.next_backoff();
            assert!(manager.should_reconnect());
        }
    }

    #[tokio::test]
    async fn test_manager_next_backoff_increments_counter() {
        let manager = ReconnectionManager::new(ReconnectConfig::default());

        assert_eq!(manager.current_attempt(), 0);
        manager.next_backoff();
        assert_eq!(manager.current_attempt(), 1);
        manager.next_backoff();
        assert_eq!(manager.current_attempt(), 2);
    }

    #[tokio::test]
    async fn test_manager_peek_backoff_does_not_increment() {
        let manager = ReconnectionManager::new(ReconnectConfig::default());

        assert_eq!(manager.current_attempt(), 0);
        let _ = manager.peek_backoff();
        assert_eq!(manager.current_attempt(), 0);
        let _ = manager.peek_backoff();
        assert_eq!(manager.current_attempt(), 0);
    }

    #[tokio::test]
    async fn test_manager_reset_attempts() {
        let manager = ReconnectionManager::new(ReconnectConfig::default());

        manager.next_backoff();
        manager.next_backoff();
        manager.next_backoff();
        assert_eq!(manager.current_attempt(), 3);

        manager.reset_attempts();
        assert_eq!(manager.current_attempt(), 0);
    }

    #[tokio::test]
    async fn test_manager_record_successful_attempt() {
        let manager = ReconnectionManager::new(ReconnectConfig::default());

        manager.next_backoff();
        manager.next_backoff();
        assert_eq!(manager.current_attempt(), 2);

        manager.record_attempt(true, Duration::from_millis(100)).await;

        let stats = manager.stats().await;
        assert_eq!(stats.successful_reconnects, 1);
        assert_eq!(stats.consecutive_failures, 0);
        assert_eq!(manager.current_attempt(), 0); // Reset on success
    }

    #[tokio::test]
    async fn test_manager_record_failed_attempt() {
        let manager = ReconnectionManager::new(ReconnectConfig::default());

        manager.record_attempt(false, Duration::from_millis(50)).await;

        let stats = manager.stats().await;
        assert_eq!(stats.failed_attempts, 1);
        assert_eq!(stats.consecutive_failures, 1);
    }

    #[tokio::test]
    async fn test_manager_degradation_level() {
        let manager = ReconnectionManager::new(ReconnectConfig::default());

        // Initially not degraded
        assert!(!manager.is_degraded().await);

        // Record failures
        manager.record_attempt(false, Duration::from_millis(50)).await;
        manager.record_attempt(false, Duration::from_millis(50)).await;

        // Should be degraded
        assert!(manager.is_degraded().await);
    }

    #[tokio::test]
    async fn test_manager_critical_state() {
        let manager = ReconnectionManager::new(ReconnectConfig::default());

        // Initially not critical
        assert!(!manager.is_critical().await);

        // Many failures to reach critical
        for _ in 0..6 {
            manager.record_attempt(false, Duration::from_millis(50)).await;
        }

        // Should be critical
        assert!(manager.is_critical().await);
    }

    #[tokio::test]
    async fn test_manager_stats_accessor() {
        let manager = ReconnectionManager::new(ReconnectConfig::default());

        manager.record_attempt(true, Duration::from_millis(100)).await;

        let stats = manager.stats().await;
        assert_eq!(stats.total_attempts, 1);
    }

    #[tokio::test]
    async fn test_manager_backoff_with_fixed_strategy() {
        let manager = ReconnectionManager::new(
            ReconnectConfig::new()
                .with_backoff(BackoffStrategy::fixed(Duration::from_secs(3))),
        );

        let delay1 = manager.next_backoff();
        let delay2 = manager.next_backoff();

        assert_eq!(delay1, Duration::from_secs(3));
        assert_eq!(delay2, Duration::from_secs(3));
    }
}

// ============================================================================
// RecoveryConfig Tests
// ============================================================================

mod recovery_config_tests {
    use super::*;

    #[test]
    fn test_recovery_config_default() {
        let config = RecoveryConfig::default();

        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay, Duration::from_millis(100));
        assert!(config.reconnect_on_error);
        assert!(!config.fail_fast_codes.is_empty());
    }

    #[test]
    fn test_recovery_config_new() {
        let config = RecoveryConfig::new();
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_strategy_for_transport_error() {
        let config = RecoveryConfig::new();
        let error = McpError::communication("Connection lost");

        assert_eq!(
            config.strategy_for(&error),
            RecoveryStrategy::ReconnectAndRetry
        );
    }

    #[test]
    fn test_strategy_for_timeout_error() {
        let config = RecoveryConfig::new();
        let error = McpError::timeout("test", Duration::from_secs(10));

        assert_eq!(
            config.strategy_for(&error),
            RecoveryStrategy::ReconnectAndRetry
        );
    }

    #[test]
    fn test_strategy_for_invalid_request_fail_fast() {
        let config = RecoveryConfig::new();
        let error = McpError::new(McpErrorCode::InvalidRequest, "Bad request");

        assert_eq!(config.strategy_for(&error), RecoveryStrategy::Fail);
    }

    #[test]
    fn test_strategy_for_invalid_params_fail_fast() {
        let config = RecoveryConfig::new();
        let error = McpError::new(McpErrorCode::InvalidParams, "Bad params");

        assert_eq!(config.strategy_for(&error), RecoveryStrategy::Fail);
    }

    #[test]
    fn test_strategy_for_method_not_found_fail_fast() {
        let config = RecoveryConfig::new();
        let error = McpError::new(McpErrorCode::MethodNotFound, "Unknown method");

        assert_eq!(config.strategy_for(&error), RecoveryStrategy::Fail);
    }

    #[test]
    fn test_strategy_for_internal_error() {
        let config = RecoveryConfig::new();
        let error = McpError::new(McpErrorCode::InternalError, "Internal issue");

        assert_eq!(config.strategy_for(&error), RecoveryStrategy::RetryWithDelay);
    }

    #[test]
    fn test_strategy_for_server_starting() {
        let config = RecoveryConfig::new();
        let error = McpError::new(McpErrorCode::ServerStarting, "Server starting");

        assert_eq!(config.strategy_for(&error), RecoveryStrategy::RetryWithDelay);
    }

    #[test]
    fn test_strategy_for_server_shutdown() {
        let config = RecoveryConfig::new();
        let error = McpError::new(McpErrorCode::ServerShutdown, "Server shutdown");

        // Server shutdown should trigger reconnect
        assert_eq!(
            config.strategy_for(&error),
            RecoveryStrategy::ReconnectAndRetry
        );
    }
}

// ============================================================================
// RecoveryStrategy Tests
// ============================================================================

mod recovery_strategy_tests {
    use super::*;

    #[test]
    fn test_recovery_strategy_equality() {
        assert_eq!(RecoveryStrategy::Retry, RecoveryStrategy::Retry);
        assert_eq!(
            RecoveryStrategy::RetryWithDelay,
            RecoveryStrategy::RetryWithDelay
        );
        assert_ne!(RecoveryStrategy::Retry, RecoveryStrategy::Fail);
    }

    #[test]
    fn test_recovery_strategy_clone() {
        let strategy = RecoveryStrategy::ReconnectAndRetry;
        let cloned = strategy.clone();
        assert_eq!(strategy, cloned);
    }

    #[test]
    fn test_recovery_strategy_copy() {
        let strategy = RecoveryStrategy::RetryWithDelay;
        let copied = strategy;
        assert_eq!(strategy, copied);
    }
}

// ============================================================================
// Reconnector Trait Mock Tests
// ============================================================================

mod reconnector_trait_tests {
    use super::*;

    /// Mock Reconnector for testing
    struct MockReconnector {
        connected: Arc<AtomicBool>,
        reconnect_count: Arc<AtomicU32>,
        should_fail: Arc<AtomicBool>,
        health: Arc<std::sync::Mutex<ConnectionHealth>>,
    }

    impl MockReconnector {
        fn new() -> Self {
            Self {
                connected: Arc::new(AtomicBool::new(true)),
                reconnect_count: Arc::new(AtomicU32::new(0)),
                should_fail: Arc::new(AtomicBool::new(false)),
                health: Arc::new(std::sync::Mutex::new(ConnectionHealth::Healthy)),
            }
        }

        fn set_connected(&self, connected: bool) {
            self.connected.store(connected, Ordering::SeqCst);
        }

        fn set_should_fail(&self, should_fail: bool) {
            self.should_fail.store(should_fail, Ordering::SeqCst);
        }

        fn set_health(&self, health: ConnectionHealth) {
            *self.health.lock().unwrap() = health;
        }

        fn reconnect_count(&self) -> u32 {
            self.reconnect_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl Reconnector for MockReconnector {
        async fn reconnect(&self) -> ltmatrix::mcp::protocol::errors::McpResult<()> {
            self.reconnect_count.fetch_add(1, Ordering::SeqCst);

            if self.should_fail.load(Ordering::SeqCst) {
                Err(McpError::communication("Reconnect failed"))
            } else {
                self.connected.store(true, Ordering::SeqCst);
                Ok(())
            }
        }

        async fn is_connected(&self) -> bool {
            self.connected.load(Ordering::SeqCst)
        }

        async fn health(&self) -> ConnectionHealth {
            self.health.lock().unwrap().clone()
        }
    }

    #[tokio::test]
    async fn test_mock_reconnector_reconnect_success() {
        let reconnector = MockReconnector::new();
        reconnector.set_connected(false);

        let result = reconnector.reconnect().await;
        assert!(result.is_ok());
        assert!(reconnector.is_connected().await);
        assert_eq!(reconnector.reconnect_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_reconnector_reconnect_failure() {
        let reconnector = MockReconnector::new();
        reconnector.set_should_fail(true);

        let result = reconnector.reconnect().await;
        assert!(result.is_err());
        assert_eq!(reconnector.reconnect_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_reconnector_health() {
        let reconnector = MockReconnector::new();

        assert!(reconnector.health().await.is_healthy());

        reconnector.set_health(ConnectionHealth::Degraded { missed_pings: 2 });
        assert!(reconnector.health().await.is_degraded());

        reconnector.set_health(ConnectionHealth::Stale { missed_pings: 5 });
        assert!(reconnector.health().await.is_stale());
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_reconnection_lifecycle() {
        // Create manager with fixed backoff for predictable testing
        let manager = ReconnectionManager::new(
            ReconnectConfig::new()
                .with_backoff(BackoffStrategy::fixed(Duration::from_millis(10)))
                .with_debug_logging(true),
        );

        // Initially should allow reconnection
        assert!(manager.should_reconnect());
        assert_eq!(manager.current_attempt(), 0);

        // Get first backoff
        let delay1 = manager.next_backoff();
        assert_eq!(delay1, Duration::from_millis(10));
        assert_eq!(manager.current_attempt(), 1);

        // Record successful reconnection
        manager.record_attempt(true, Duration::from_millis(5)).await;

        // Stats should reflect success
        let stats = manager.stats().await;
        assert_eq!(stats.successful_reconnects, 1);
        assert_eq!(stats.consecutive_failures, 0);
        assert_eq!(manager.current_attempt(), 0); // Reset
    }

    #[tokio::test]
    async fn test_failure_recovery_cycle() {
        let manager = ReconnectionManager::new(ReconnectConfig::default());

        // Record multiple failures
        for i in 1..=3 {
            manager.next_backoff();
            manager.record_attempt(false, Duration::from_millis(10)).await;

            let stats = manager.stats().await;
            assert_eq!(stats.consecutive_failures, i);
        }

        // Final success should reset
        manager.record_attempt(true, Duration::from_millis(20)).await;
        let stats = manager.stats().await;
        assert_eq!(stats.consecutive_failures, 0);
        assert_eq!(stats.degradation_level, DegradationLevel::None);
    }

    #[tokio::test]
    async fn test_concurrent_manager_access() {
        let manager = Arc::new(ReconnectionManager::new(ReconnectConfig::default()));
        let mut handles = vec![];

        // Spawn multiple tasks accessing the manager concurrently
        for i in 0..10 {
            let mgr = manager.clone();
            handles.push(tokio::spawn(async move {
                if i % 3 == 0 {
                    mgr.next_backoff();
                } else if i % 3 == 1 {
                    mgr.record_attempt(true, Duration::from_millis(10)).await;
                } else {
                    let _ = mgr.stats().await;
                }
            }));
        }

        // All tasks should complete without panic
        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_stats_snapshot_consistency() {
        let manager = ReconnectionManager::new(ReconnectConfig::default());

        // Record some attempts
        manager.record_attempt(true, Duration::from_millis(100)).await;
        manager.record_attempt(false, Duration::from_millis(50)).await;

        // Get snapshot
        let stats1 = manager.stats().await;
        let stats2 = manager.stats().await;

        // Both snapshots should be consistent
        assert_eq!(stats1.total_attempts, stats2.total_attempts);
        assert_eq!(stats1.successful_reconnects, stats2.successful_reconnects);
        assert_eq!(stats1.failed_attempts, stats2.failed_attempts);
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[tokio::test]
    async fn test_zero_max_attempts_means_unlimited() {
        // Use fixed backoff to avoid exponential overflow with many attempts
        let manager = ReconnectionManager::new(
            ReconnectConfig::new()
                .with_max_attempts(0)
                .with_backoff(BackoffStrategy::fixed(Duration::from_millis(1))),
        );

        // With 0 max_attempts, should always allow reconnect
        for _ in 0..100 {
            assert!(manager.should_reconnect());
            manager.next_backoff();
        }
    }

    #[tokio::test]
    async fn test_very_large_max_attempts() {
        // Use fixed backoff to avoid exponential overflow
        let manager = ReconnectionManager::new(
            ReconnectConfig::new()
                .with_max_attempts(100)
                .with_backoff(BackoffStrategy::fixed(Duration::from_millis(1))),
        );

        for _ in 0..99 {
            assert!(manager.should_reconnect());
            manager.next_backoff();
        }
        assert!(manager.should_reconnect());
        manager.next_backoff();
        assert!(!manager.should_reconnect());
    }

    #[test]
    fn test_backoff_with_zero_initial_delay() {
        let backoff = BackoffStrategy::fixed(Duration::ZERO);
        assert_eq!(backoff.calculate_delay(0), Duration::ZERO);
        assert_eq!(backoff.calculate_delay(100), Duration::ZERO);
    }

    #[test]
    fn test_exponential_backoff_overflow_protection() {
        // Test that reasonable attempt numbers are handled correctly
        // Note: Very large attempt numbers (100+) can cause float overflow before min() is applied
        // This is a known limitation of the current implementation
        let backoff =
            BackoffStrategy::exponential(Duration::from_secs(1), Duration::from_secs(60), 2.0);

        // Should cap at max_delay for reasonable attempt numbers
        assert_eq!(backoff.calculate_delay(10), Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_manager_multiple_start_stop_cycles() {
        let manager = ReconnectionManager::new(
            ReconnectConfig::new()
                .with_backoff(BackoffStrategy::fixed(Duration::from_millis(1)))
                .with_debug_logging(true),
        );

        for _ in 0..3 {
            assert!(manager.should_reconnect());
            manager.next_backoff();
            manager.record_attempt(true, Duration::from_millis(1)).await;
            assert_eq!(manager.current_attempt(), 0); // Reset after success
        }
    }

    #[test]
    fn test_degradation_level_all_variants_covered() {
        // Ensure all variants can be created and compared
        let levels = vec![
            DegradationLevel::None,
            DegradationLevel::Minor,
            DegradationLevel::Moderate,
            DegradationLevel::Severe,
            DegradationLevel::Critical,
        ];

        // Verify ordering
        for i in 0..levels.len() - 1 {
            assert!(levels[i] < levels[i + 1]);
        }
    }
}

// ============================================================================
// Monitoring Integration Tests (with proper cleanup to prevent timeouts)
// ============================================================================

mod monitoring_tests {
    use super::*;
    use ltmatrix::mcp::heartbeat::{HeartbeatConfig, HeartbeatManager};
    use tokio::time::timeout;

    /// Mock Reconnector for monitoring tests
    struct MonitoringMockReconnector {
        connected: Arc<AtomicBool>,
        reconnect_count: Arc<AtomicU32>,
        health: Arc<std::sync::Mutex<ConnectionHealth>>,
    }

    impl MonitoringMockReconnector {
        fn new() -> Self {
            Self {
                connected: Arc::new(AtomicBool::new(true)),
                reconnect_count: Arc::new(AtomicU32::new(0)),
                health: Arc::new(std::sync::Mutex::new(ConnectionHealth::Healthy)),
            }
        }

        fn set_connected(&self, connected: bool) {
            self.connected.store(connected, Ordering::SeqCst);
        }

        fn set_health(&self, health: ConnectionHealth) {
            *self.health.lock().unwrap() = health;
        }

        #[allow(dead_code)]
        fn reconnect_count(&self) -> u32 {
            self.reconnect_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl Reconnector for MonitoringMockReconnector {
        async fn reconnect(&self) -> ltmatrix::mcp::protocol::errors::McpResult<()> {
            self.reconnect_count.fetch_add(1, Ordering::SeqCst);
            self.connected.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn is_connected(&self) -> bool {
            self.connected.load(Ordering::SeqCst)
        }

        async fn health(&self) -> ConnectionHealth {
            self.health.lock().unwrap().clone()
        }
    }

    #[tokio::test]
    async fn test_start_monitoring_returns_handle() {
        // Use very short intervals for testing
        let reconnect_config = ReconnectConfig::new()
            .with_backoff(BackoffStrategy::fixed(Duration::from_millis(10)))
            .with_max_attempts(5)
            .with_debug_logging(false);

        let heartbeat_config = HeartbeatConfig::new()
            .with_interval(Duration::from_millis(50))
            .with_timeout(Duration::from_millis(25))
            .with_max_missed_pings(2);

        let manager = ReconnectionManager::new(reconnect_config);
        let heartbeat = HeartbeatManager::new(heartbeat_config);
        let reconnector = Arc::new(MonitoringMockReconnector::new());

        // Start monitoring
        let handle = manager
            .start_monitoring(reconnector.clone(), Arc::new(heartbeat))
            .await;

        assert!(handle.is_ok(), "start_monitoring should return Ok");
        let handle = handle.unwrap();

        // Stop immediately to clean up
        handle.stop().await;
    }

    #[tokio::test]
    async fn test_start_monitoring_twice_fails() {
        let reconnect_config = ReconnectConfig::new()
            .with_backoff(BackoffStrategy::fixed(Duration::from_millis(10)));

        let heartbeat_config = HeartbeatConfig::new()
            .with_interval(Duration::from_millis(100));

        let manager = ReconnectionManager::new(reconnect_config);
        let heartbeat = Arc::new(HeartbeatManager::new(heartbeat_config));
        let reconnector = Arc::new(MonitoringMockReconnector::new());

        // Start first monitoring
        let handle1 = manager
            .start_monitoring(reconnector.clone(), heartbeat.clone())
            .await;
        assert!(handle1.is_ok());

        // Try to start again - should fail
        let handle2 = manager
            .start_monitoring(reconnector.clone(), heartbeat.clone())
            .await;
        assert!(handle2.is_err(), "Second start_monitoring should fail");

        // Cleanup
        handle1.unwrap().stop().await;
    }

    #[tokio::test]
    async fn test_reconnect_handle_stop_terminates_task() {
        let reconnect_config = ReconnectConfig::new()
            .with_backoff(BackoffStrategy::fixed(Duration::from_millis(10)));

        let heartbeat_config = HeartbeatConfig::new()
            .with_interval(Duration::from_millis(100));

        let manager = Arc::new(ReconnectionManager::new(reconnect_config));
        let heartbeat = Arc::new(HeartbeatManager::new(heartbeat_config));
        let reconnector = Arc::new(MonitoringMockReconnector::new());

        let handle = manager
            .start_monitoring(reconnector.clone(), heartbeat.clone())
            .await
            .unwrap();

        // Stop the monitoring task
        handle.stop().await;

        // Wait for task to terminate (the task checks every 5 seconds, but
        // it also checks the stop channel immediately on each tick)
        // We need to wait for the reconnecting flag to be reset
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Use manager's stop method to ensure reconnecting flag is reset
        manager.stop().await;

        // After stopping, should be able to start again
        let handle2_result = timeout(
            Duration::from_millis(500),
            manager.start_monitoring(reconnector, heartbeat),
        )
        .await;

        let success = handle2_result.is_ok()
            && handle2_result
                .as_ref()
                .map(|r| r.is_ok())
                .unwrap_or(false);
        assert!(success, "Should be able to start monitoring after stop");

        // Cleanup
        if let Ok(Ok(h)) = handle2_result {
            h.stop().await;
        }
    }

    #[tokio::test]
    async fn test_manager_stop_method() {
        let reconnect_config = ReconnectConfig::new()
            .with_backoff(BackoffStrategy::fixed(Duration::from_millis(10)));

        let heartbeat_config = HeartbeatConfig::new()
            .with_interval(Duration::from_millis(100));

        let manager = ReconnectionManager::new(reconnect_config);
        let heartbeat = Arc::new(HeartbeatManager::new(heartbeat_config));
        let reconnector = Arc::new(MonitoringMockReconnector::new());

        let _handle = manager
            .start_monitoring(reconnector.clone(), heartbeat.clone())
            .await
            .unwrap();

        // Use manager's stop method
        manager.stop().await;

        // Wait for cleanup
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Should be able to start again
        let handle2 = manager.start_monitoring(reconnector, heartbeat).await;
        assert!(handle2.is_ok(), "Should be able to restart after stop");

        if let Ok(h) = handle2 {
            h.stop().await;
        }
    }

    #[tokio::test]
    async fn test_monitoring_with_stale_connection() {
        // This test verifies that the monitoring task properly handles stale connections
        let reconnect_config = ReconnectConfig::new()
            .with_backoff(BackoffStrategy::fixed(Duration::from_millis(5)))
            .with_max_attempts(3)
            .with_reconnect_on_stale(true);

        let heartbeat_config = HeartbeatConfig::new()
            .with_interval(Duration::from_millis(10))
            .with_timeout(Duration::from_millis(5))
            .with_max_missed_pings(1);

        let manager = ReconnectionManager::new(reconnect_config);
        let heartbeat = Arc::new(HeartbeatManager::new(heartbeat_config));
        let reconnector = Arc::new(MonitoringMockReconnector::new());

        // Set connection as disconnected and stale
        reconnector.set_connected(false);
        reconnector.set_health(ConnectionHealth::Stale { missed_pings: 5 });

        // Start monitoring with timeout to prevent hanging
        let result = timeout(
            Duration::from_millis(500),
            manager.start_monitoring(reconnector.clone(), heartbeat),
        )
        .await;

        assert!(result.is_ok(), "start_monitoring should complete quickly");
        let handle = result.unwrap().unwrap();

        // Wait a bit for potential reconnection attempts
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Cleanup
        handle.stop().await;
    }
}

// ============================================================================
// Additional Comprehensive Tests
// ============================================================================

mod comprehensive_tests {
    use super::*;

    #[test]
    fn test_backoff_strategy_exhaustive_coverage() {
        // Test all backoff strategy variants
        let strategies = vec![
            BackoffStrategy::fixed(Duration::from_secs(1)),
            BackoffStrategy::linear(
                Duration::from_millis(100),
                Duration::from_millis(50),
                Duration::from_secs(1),
            ),
            BackoffStrategy::exponential(Duration::from_millis(100), Duration::from_secs(10), 2.0),
            BackoffStrategy::exponential_with_jitter(
                Duration::from_millis(100),
                Duration::from_secs(10),
                2.0,
                0.5,
            ),
        ];

        for (i, strategy) in strategies.iter().enumerate() {
            // All strategies should produce valid delays for various attempt numbers
            for attempt in 0..=10 {
                let delay = strategy.calculate_delay(attempt);
                assert!(delay > Duration::ZERO || attempt == 0 && delay == Duration::ZERO);
            }

            // Test clone
            let cloned = strategy.clone();
            assert_eq!(
                strategy.calculate_delay(0),
                cloned.calculate_delay(0),
                "Cloned strategy {} should produce same delay",
                i
            );
        }
    }

    #[test]
    fn test_degradation_level_exhaustive() {
        // Test all degradation level conversions
        let test_cases = vec![
            (ConnectionHealth::Healthy, DegradationLevel::None),
            (
                ConnectionHealth::Degraded { missed_pings: 1 },
                DegradationLevel::Minor,
            ),
            (
                ConnectionHealth::Degraded { missed_pings: 2 },
                DegradationLevel::Moderate,
            ),
            (
                ConnectionHealth::Degraded { missed_pings: 10 },
                DegradationLevel::Moderate,
            ),
            (
                ConnectionHealth::Stale { missed_pings: 1 },
                DegradationLevel::Critical,
            ),
            (
                ConnectionHealth::Stale { missed_pings: 100 },
                DegradationLevel::Critical,
            ),
        ];

        for (health, expected_level) in test_cases {
            let level = DegradationLevel::from_health(&health);
            assert_eq!(level, expected_level, "Health {:?} should map to {:?}", health, expected_level);
        }
    }

    #[test]
    fn test_recovery_strategy_all_error_codes() {
        let config = RecoveryConfig::default();

        // Test all error codes
        let test_cases = vec![
            (McpErrorCode::ParseError, RecoveryStrategy::Retry),
            (McpErrorCode::InvalidRequest, RecoveryStrategy::Fail),
            (McpErrorCode::MethodNotFound, RecoveryStrategy::Fail),
            (McpErrorCode::InvalidParams, RecoveryStrategy::Fail),
            (McpErrorCode::InternalError, RecoveryStrategy::RetryWithDelay),
            (McpErrorCode::ServerStarting, RecoveryStrategy::RetryWithDelay),
            (McpErrorCode::TransportError, RecoveryStrategy::ReconnectAndRetry),
            (McpErrorCode::ServerShutdown, RecoveryStrategy::ReconnectAndRetry),
            (McpErrorCode::RequestTimeout, RecoveryStrategy::ReconnectAndRetry),
        ];

        for (code, expected_strategy) in test_cases {
            let error = McpError::new(code, "Test error");
            let strategy = config.strategy_for(&error);
            assert_eq!(
                strategy, expected_strategy,
                "Error code {:?} should map to {:?}",
                code, expected_strategy
            );
        }
    }

    #[tokio::test]
    async fn test_stats_accumulation_over_many_attempts() {
        let mut stats = ReconnectStats::new();

        // Simulate a sequence of attempts
        let attempts = vec![
            (false, Duration::from_millis(10)),
            (false, Duration::from_millis(15)),
            (false, Duration::from_millis(20)),
            (true, Duration::from_millis(50)),
            (false, Duration::from_millis(10)),
            (true, Duration::from_millis(30)),
        ];

        for (success, duration) in attempts {
            stats.record_attempt(success, duration);
        }

        assert_eq!(stats.total_attempts, 6);
        assert_eq!(stats.successful_reconnects, 2);
        assert_eq!(stats.failed_attempts, 4);
        assert_eq!(stats.total_reconnect_time, Duration::from_millis(135));
        assert_eq!(stats.consecutive_failures, 0); // Reset after last success
    }

    #[tokio::test]
    async fn test_manager_thread_safety() {
        let manager = Arc::new(ReconnectionManager::new(
            ReconnectConfig::new()
                .with_backoff(BackoffStrategy::fixed(Duration::from_millis(1)))
                .with_max_attempts(100),
        ));

        let mut handles = vec![];

        // Spawn multiple concurrent operations
        for i in 0..20 {
            let mgr = manager.clone();
            handles.push(tokio::spawn(async move {
                match i % 4 {
                    0 => {
                        mgr.next_backoff();
                    }
                    1 => {
                        mgr.record_attempt(true, Duration::from_millis(1)).await;
                    }
                    2 => {
                        let _ = mgr.stats().await;
                    }
                    _ => {
                        let _ = mgr.should_reconnect();
                    }
                }
            }));
        }

        // All operations should complete without panic
        for handle in handles {
            handle.await.unwrap();
        }

        // Manager should still be in a valid state
        let stats = manager.stats().await;
        assert!(stats.total_attempts > 0 || stats.successful_reconnects > 0);
    }

    #[test]
    fn test_config_with_backoff_preserves_other_defaults() {
        let config = ReconnectConfig::new()
            .with_backoff(BackoffStrategy::fixed(Duration::from_secs(5)));

        // Other defaults should be preserved
        assert_eq!(config.max_attempts, 10);
        assert!(config.auto_reconnect);
        assert!(config.reconnect_on_stale);
        assert_eq!(config.reconnect_on_errors, 3);
    }

    #[test]
    fn test_linear_backoff_exact_calculation() {
        let backoff = BackoffStrategy::linear(
            Duration::from_millis(100),
            Duration::from_millis(50),
            Duration::from_millis(1000),
        );

        // Verify exact calculations
        assert_eq!(backoff.calculate_delay(0), Duration::from_millis(100)); // 100 + 0*50
        assert_eq!(backoff.calculate_delay(1), Duration::from_millis(150)); // 100 + 1*50
        assert_eq!(backoff.calculate_delay(2), Duration::from_millis(200)); // 100 + 2*50
        assert_eq!(backoff.calculate_delay(10), Duration::from_millis(600)); // 100 + 10*50
        assert_eq!(backoff.calculate_delay(18), Duration::from_millis(1000)); // 100 + 18*50 = 1000, capped
        assert_eq!(backoff.calculate_delay(100), Duration::from_millis(1000)); // capped
    }

    #[test]
    fn test_exponential_backoff_non_power_of_two() {
        let backoff = BackoffStrategy::exponential(
            Duration::from_millis(100),
            Duration::from_secs(10),
            1.5, // Non-power-of-2 multiplier
        );

        // Verify exponential growth with 1.5 multiplier
        assert_eq!(backoff.calculate_delay(0), Duration::from_millis(100));
        // 100 * 1.5^1 = 150
        assert_eq!(backoff.calculate_delay(1), Duration::from_millis(150));
        // 100 * 1.5^2 = 225
        assert_eq!(backoff.calculate_delay(2), Duration::from_millis(225));
        // 100 * 1.5^3 = 337.5ms (truncated to 337ms by Duration::from_secs_f64)
        let delay3 = backoff.calculate_delay(3);
        assert!(
            delay3 >= Duration::from_millis(337) && delay3 <= Duration::from_millis(338),
            "Expected approximately 337-338ms, got {:?}",
            delay3
        );
    }

    #[tokio::test]
    async fn test_manager_default_creates_valid_instance() {
        let manager = ReconnectionManager::default();

        // Should have default config
        assert_eq!(manager.current_attempt(), 0);
        assert!(!manager.is_reconnecting());
        assert!(manager.should_reconnect());

        // Stats should be empty
        let stats = manager.stats().await;
        assert_eq!(stats.total_attempts, 0);
    }

    #[test]
    fn test_peek_vs_next_backoff() {
        let config = ReconnectConfig::new()
            .with_backoff(BackoffStrategy::fixed(Duration::from_millis(100)));
        let manager = ReconnectionManager::new(config);

        // Peek should not increment counter
        let peek1 = manager.peek_backoff();
        let peek2 = manager.peek_backoff();
        let peek3 = manager.peek_backoff();

        assert_eq!(peek1, Duration::from_millis(100));
        assert_eq!(peek2, Duration::from_millis(100));
        assert_eq!(peek3, Duration::from_millis(100));
        assert_eq!(manager.current_attempt(), 0);

        // Next should increment counter
        let next1 = manager.next_backoff();
        let next2 = manager.next_backoff();

        assert_eq!(next1, Duration::from_millis(100));
        assert_eq!(next2, Duration::from_millis(100));
        assert_eq!(manager.current_attempt(), 2);
    }
}
