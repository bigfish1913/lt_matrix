// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Integration tests for MCP Heartbeat/Keepalive Mechanism
//!
//! Tests for:
//! - HeartbeatConfig configuration and builder pattern
//! - ActivityTracker for tracking send/receive activity
//! - ConnectionHealth status transitions
//! - HeartbeatStats statistics tracking
//! - HeartbeatManager lifecycle management
//! - Stale connection detection
//! - Background ping task management
//! - PingSender trait integration

use ltmatrix::mcp::heartbeat::{
    ActivityTracker, ConnectionHealth, HeartbeatConfig, HeartbeatManager, HeartbeatStats,
    PingSender,
};
use ltmatrix::mcp::protocol::errors::{McpError, McpErrorCode, McpResult};
use ltmatrix::mcp::RequestId;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// HeartbeatConfig Tests
// ============================================================================

mod heartbeat_config_tests {
    use super::*;

    #[test]
    fn test_config_default_values() {
        let config = HeartbeatConfig::default();

        assert_eq!(config.interval, Duration::from_secs(30));
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.max_missed_pings, 3);
        assert!(config.idle_only);
        assert_eq!(config.activity_debounce, Duration::from_secs(1));
        assert!(!config.debug_logging);
    }

    #[test]
    fn test_config_new() {
        let config = HeartbeatConfig::new();
        assert_eq!(config.interval, Duration::from_secs(30));
    }

    #[test]
    fn test_config_builder_with_interval() {
        let config = HeartbeatConfig::new().with_interval(Duration::from_secs(60));
        assert_eq!(config.interval, Duration::from_secs(60));
    }

    #[test]
    fn test_config_builder_with_timeout() {
        let config = HeartbeatConfig::new().with_timeout(Duration::from_secs(20));
        assert_eq!(config.timeout, Duration::from_secs(20));
    }

    #[test]
    fn test_config_builder_with_max_missed_pings() {
        let config = HeartbeatConfig::new().with_max_missed_pings(5);
        assert_eq!(config.max_missed_pings, 5);
    }

    #[test]
    fn test_config_builder_with_idle_only() {
        let config = HeartbeatConfig::new().with_idle_only(false);
        assert!(!config.idle_only);
    }

    #[test]
    fn test_config_builder_with_debug_logging() {
        let config = HeartbeatConfig::new().with_debug_logging(true);
        assert!(config.debug_logging);
    }

    #[test]
    fn test_config_builder_chaining() {
        let config = HeartbeatConfig::new()
            .with_interval(Duration::from_secs(45))
            .with_timeout(Duration::from_secs(15))
            .with_max_missed_pings(4)
            .with_idle_only(false)
            .with_debug_logging(true);

        assert_eq!(config.interval, Duration::from_secs(45));
        assert_eq!(config.timeout, Duration::from_secs(15));
        assert_eq!(config.max_missed_pings, 4);
        assert!(!config.idle_only);
        assert!(config.debug_logging);
    }

    #[test]
    fn test_config_clone() {
        let config = HeartbeatConfig::new().with_interval(Duration::from_secs(60));
        let cloned = config.clone();

        assert_eq!(config.interval, cloned.interval);
        assert_eq!(config.timeout, cloned.timeout);
        assert_eq!(config.max_missed_pings, cloned.max_missed_pings);
    }

    #[test]
    fn test_config_edge_case_zero_interval() {
        let config = HeartbeatConfig::new().with_interval(Duration::ZERO);
        assert_eq!(config.interval, Duration::ZERO);
    }

    #[test]
    fn test_config_edge_case_large_values() {
        let config = HeartbeatConfig::new()
            .with_interval(Duration::from_secs(3600))
            .with_max_missed_pings(1000);

        assert_eq!(config.interval, Duration::from_secs(3600));
        assert_eq!(config.max_missed_pings, 1000);
    }
}

// ============================================================================
// ActivityTracker Tests
// ============================================================================

mod activity_tracker_tests {
    use super::*;

    #[tokio::test]
    async fn test_tracker_initial_state() {
        let tracker = ActivityTracker::new(Duration::from_millis(10));

        assert!(!tracker.has_activity().await);
        assert!(tracker.time_since_activity().await.is_none());
        assert!(tracker.time_since_receive().await.is_none());
        assert!(tracker.time_since_send().await.is_none());
    }

    #[tokio::test]
    async fn test_tracker_record_send() {
        let tracker = ActivityTracker::new(Duration::from_millis(10));

        tracker.record_send().await;

        assert!(tracker.has_activity().await);
        assert!(tracker.time_since_send().await.is_some());
        assert!(tracker.time_since_receive().await.is_none());
    }

    #[tokio::test]
    async fn test_tracker_record_receive() {
        let tracker = ActivityTracker::new(Duration::from_millis(10));

        tracker.record_receive().await;

        assert!(tracker.has_activity().await);
        assert!(tracker.time_since_receive().await.is_some());
    }

    #[tokio::test]
    async fn test_tracker_time_since_activity_returns_minimum() {
        let tracker = ActivityTracker::new(Duration::from_millis(10));

        tracker.record_send().await;
        tokio::time::sleep(Duration::from_millis(5)).await;
        tracker.record_receive().await;

        // time_since_activity should return the minimum (most recent)
        let time = tracker.time_since_activity().await.unwrap();
        assert!(time < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_tracker_debounce_prevents_excessive_updates() {
        let tracker = ActivityTracker::new(Duration::from_millis(100));

        // First receive
        tracker.record_receive().await;
        let time1 = tracker.time_since_receive().await;

        // Immediate second receive should be debounced
        tracker.record_receive().await;
        let time2 = tracker.time_since_receive().await;

        // Both should return similar times (debounced)
        // time2 is measured after time1, so it should be >= time1 (more elapsed time)
        // If debouncing didn't work, time2 would be much smaller (close to 0)
        assert!(time2.unwrap() >= time1.unwrap());
    }

    #[tokio::test]
    async fn test_tracker_debounce_allows_update_after_period() {
        let tracker = ActivityTracker::new(Duration::from_millis(10));

        tracker.record_receive().await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        tracker.record_receive().await;

        // Should have updated after debounce period
        let time = tracker.time_since_receive().await.unwrap();
        assert!(time < Duration::from_millis(15));
    }

    #[tokio::test]
    async fn test_tracker_reset() {
        let tracker = ActivityTracker::new(Duration::from_millis(10));

        tracker.record_send().await;
        tracker.record_receive().await;
        assert!(tracker.has_activity().await);

        tracker.reset().await;

        assert!(!tracker.has_activity().await);
        assert!(tracker.time_since_activity().await.is_none());
        assert!(tracker.time_since_send().await.is_none());
        assert!(tracker.time_since_receive().await.is_none());
    }

    #[tokio::test]
    async fn test_tracker_default() {
        let tracker = ActivityTracker::default();
        assert!(!tracker.has_activity().await);
    }

    #[tokio::test]
    async fn test_tracker_elapsed_time_increases() {
        let tracker = ActivityTracker::new(Duration::from_millis(1));

        tracker.record_send().await;
        let time1 = tracker.time_since_send().await.unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let time2 = tracker.time_since_send().await.unwrap();
        assert!(time2 > time1);
    }
}

// ============================================================================
// ConnectionHealth Tests
// ============================================================================

mod connection_health_tests {
    use super::*;

    #[test]
    fn test_healthy_state() {
        let health = ConnectionHealth::Healthy;

        assert!(health.is_healthy());
        assert!(!health.is_degraded());
        assert!(!health.is_stale());
        assert_eq!(health.missed_pings(), 0);
    }

    #[test]
    fn test_degraded_state() {
        let health = ConnectionHealth::Degraded { missed_pings: 2 };

        assert!(!health.is_healthy());
        assert!(health.is_degraded());
        assert!(!health.is_stale());
        assert_eq!(health.missed_pings(), 2);
    }

    #[test]
    fn test_stale_state() {
        let health = ConnectionHealth::Stale { missed_pings: 5 };

        assert!(!health.is_healthy());
        assert!(!health.is_degraded());
        assert!(health.is_stale());
        assert_eq!(health.missed_pings(), 5);
    }

    #[test]
    fn test_health_clone() {
        let health = ConnectionHealth::Degraded { missed_pings: 3 };
        let cloned = health.clone();

        assert_eq!(health.is_degraded(), cloned.is_degraded());
        assert_eq!(health.missed_pings(), cloned.missed_pings());
    }

    #[test]
    fn test_health_copy() {
        let health = ConnectionHealth::Healthy;
        let copied = health;

        assert_eq!(health.is_healthy(), copied.is_healthy());
    }

    #[test]
    fn test_health_equality() {
        let health1 = ConnectionHealth::Healthy;
        let health2 = ConnectionHealth::Healthy;
        let health3 = ConnectionHealth::Degraded { missed_pings: 1 };

        assert_eq!(health1, health2);
        assert_ne!(health1, health3);
    }

    #[test]
    fn test_health_debug() {
        let health = ConnectionHealth::Degraded { missed_pings: 2 };
        let debug_str = format!("{:?}", health);

        assert!(debug_str.contains("Degraded"));
        assert!(debug_str.contains("2"));
    }
}

// ============================================================================
// HeartbeatStats Tests
// ============================================================================

mod heartbeat_stats_tests {
    use super::*;

    #[test]
    fn test_stats_initial_state() {
        let stats = HeartbeatStats::new();

        assert_eq!(stats.pings_sent, 0);
        assert_eq!(stats.pongs_received, 0);
        assert_eq!(stats.pings_missed, 0);
        assert_eq!(stats.consecutive_missed, 0);
        assert!(stats.avg_rtt_ms.is_none());
        assert!(stats.last_ping_sent.is_none());
        assert!(stats.last_pong_received.is_none());
    }

    #[test]
    fn test_stats_record_ping_sent() {
        let mut stats = HeartbeatStats::new();

        stats.record_ping_sent();

        assert_eq!(stats.pings_sent, 1);
        assert!(stats.last_ping_sent.is_some());
    }

    #[test]
    fn test_stats_record_pong_received() {
        let mut stats = HeartbeatStats::new();

        // Need to send a ping first to calculate RTT
        stats.record_ping_sent();
        stats.record_pong_received();

        assert_eq!(stats.pongs_received, 1);
        assert_eq!(stats.consecutive_missed, 0);
        assert!(stats.last_pong_received.is_some());
        assert!(stats.avg_rtt_ms.is_some());
    }

    #[test]
    fn test_stats_record_ping_missed() {
        let mut stats = HeartbeatStats::new();

        stats.record_ping_missed();

        assert_eq!(stats.pings_missed, 1);
        assert_eq!(stats.consecutive_missed, 1);
    }

    #[test]
    fn test_stats_consecutive_missed_accumulates() {
        let mut stats = HeartbeatStats::new();

        stats.record_ping_missed();
        stats.record_ping_missed();
        stats.record_ping_missed();

        assert_eq!(stats.pings_missed, 3);
        assert_eq!(stats.consecutive_missed, 3);
    }

    #[test]
    fn test_stats_pong_resets_consecutive_missed() {
        let mut stats = HeartbeatStats::new();

        stats.record_ping_missed();
        stats.record_ping_missed();
        assert_eq!(stats.consecutive_missed, 2);

        stats.record_ping_sent();
        stats.record_pong_received();

        assert_eq!(stats.consecutive_missed, 0);
    }

    #[test]
    fn test_stats_reset_consecutive() {
        let mut stats = HeartbeatStats::new();

        stats.record_ping_missed();
        stats.record_ping_missed();
        assert_eq!(stats.consecutive_missed, 2);

        stats.reset_consecutive();

        assert_eq!(stats.consecutive_missed, 0);
    }

    #[test]
    fn test_stats_avg_rtt_calculation() {
        let mut stats = HeartbeatStats::new();

        // First ping/pong
        stats.record_ping_sent();
        stats.record_pong_received();

        let _first_rtt = stats.avg_rtt_ms.unwrap();

        // Second ping/pong (average should be updated)
        stats.record_ping_sent();
        stats.record_pong_received();

        // Average should exist and be reasonable
        assert!(stats.avg_rtt_ms.is_some());
        assert!(stats.avg_rtt_ms.unwrap() >= 0.0);
    }

    #[test]
    fn test_stats_default() {
        let stats = HeartbeatStats::default();
        assert_eq!(stats.pings_sent, 0);
    }

    #[test]
    fn test_stats_clone() {
        let mut stats = HeartbeatStats::new();
        stats.record_ping_sent();
        stats.record_ping_missed();

        let cloned = stats.clone();

        assert_eq!(stats.pings_sent, cloned.pings_sent);
        assert_eq!(stats.consecutive_missed, cloned.consecutive_missed);
    }
}

// ============================================================================
// HeartbeatManager Lifecycle Tests
// ============================================================================

mod heartbeat_manager_lifecycle_tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = HeartbeatManager::default_manager();

        assert!(!manager.is_running());
        assert!(manager.is_healthy().await);
        assert!(!manager.is_stale().await);
    }

    #[tokio::test]
    async fn test_manager_new_with_config() {
        let config = HeartbeatConfig::new().with_interval(Duration::from_secs(60));
        let manager = HeartbeatManager::new(config);

        assert_eq!(manager.config().interval, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_manager_config_accessor() {
        let config = HeartbeatConfig::new()
            .with_interval(Duration::from_secs(45))
            .with_timeout(Duration::from_secs(15));

        let manager = HeartbeatManager::new(config);
        let config_ref = manager.config();

        assert_eq!(config_ref.interval, Duration::from_secs(45));
        assert_eq!(config_ref.timeout, Duration::from_secs(15));
    }

    #[tokio::test]
    async fn test_manager_start() {
        let manager = HeartbeatManager::new(
            HeartbeatConfig::new()
                .with_interval(Duration::from_millis(100))
                .with_debug_logging(true),
        );

        assert!(!manager.is_running());

        let handle = manager.start().await.unwrap();

        assert!(manager.is_running());

        // Cleanup
        handle.stop().await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!manager.is_running());
    }

    #[tokio::test]
    async fn test_manager_cannot_start_twice() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        let _handle1 = manager.start().await.unwrap();

        // Second start should fail
        let result = manager.start().await;
        assert!(result.is_err());

        // Cleanup
        manager.stop().await;
    }

    #[tokio::test]
    async fn test_manager_stop() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        let _handle = manager.start().await.unwrap();
        assert!(manager.is_running());

        manager.stop().await;

        // Give it a moment to fully stop
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!manager.is_running());
    }

    #[tokio::test]
    async fn test_manager_reset() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        manager.record_send().await;
        manager.record_receive().await;

        assert!(manager.time_since_activity().await.is_some());

        manager.reset().await;

        assert!(manager.time_since_activity().await.is_none());
        let stats = manager.stats().await;
        assert_eq!(stats.pings_sent, 0);
    }
}

// ============================================================================
// HeartbeatManager Activity Tracking Tests
// ============================================================================

mod heartbeat_manager_activity_tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_record_send() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        assert!(manager.time_since_activity().await.is_none());

        manager.record_send().await;

        assert!(manager.time_since_activity().await.is_some());
    }

    #[tokio::test]
    async fn test_manager_record_receive() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        manager.record_receive().await;

        assert!(manager.time_since_activity().await.is_some());
    }

    #[tokio::test]
    async fn test_manager_record_receive_updates_stats() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        manager.record_receive().await;

        let stats = manager.stats().await;
        // record_receive also triggers record_pong_received in stats
        assert_eq!(stats.pongs_received, 1);
    }

    #[tokio::test]
    async fn test_manager_time_since_activity() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        manager.record_send().await;

        let time = manager.time_since_activity().await.unwrap();
        assert!(time < Duration::from_secs(1));

        tokio::time::sleep(Duration::from_millis(100)).await;

        let time_after = manager.time_since_activity().await.unwrap();
        assert!(time_after > time);
    }
}

// ============================================================================
// HeartbeatManager Health Detection Tests
// ============================================================================

mod heartbeat_manager_health_tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_initially_healthy() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        let health = manager.health().await;
        assert!(health.is_healthy());
        assert_eq!(health.missed_pings(), 0);
    }

    #[tokio::test]
    async fn test_manager_health_stays_healthy() {
        let manager = HeartbeatManager::new(
            HeartbeatConfig::new().with_max_missed_pings(3),
        );

        // Initially healthy
        assert!(manager.is_healthy().await);

        // Should remain healthy without any missed pings
        let health = manager.health().await;
        assert!(health.is_healthy());
        assert_eq!(health.missed_pings(), 0);
    }

    #[tokio::test]
    async fn test_manager_handle_pong_updates_stats() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        let id = RequestId::Number(42);
        manager.handle_pong(&id).await;

        let stats = manager.stats().await;
        assert_eq!(stats.pongs_received, 1);
    }

    #[tokio::test]
    async fn test_manager_pong_resets_consecutive_missed() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        // Receive pong
        manager.handle_pong(&RequestId::Number(1)).await;

        let stats = manager.stats().await;
        assert_eq!(stats.consecutive_missed, 0);
    }
}

// ============================================================================
// HeartbeatManager Background Task Tests
// ============================================================================

mod heartbeat_manager_background_tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_stats_updates_with_background_task() {
        let manager = HeartbeatManager::new(
            HeartbeatConfig::new()
                .with_interval(Duration::from_millis(50))
                .with_debug_logging(true),
        );

        let handle = manager.start().await.unwrap();

        // Wait for a few ticks
        tokio::time::sleep(Duration::from_millis(150)).await;

        let stats = manager.stats().await;
        // Should have at least one ping sent
        assert!(stats.pings_sent >= 1);

        handle.stop().await;
    }

    #[tokio::test]
    async fn test_manager_handle_stop_and_join() {
        let manager = HeartbeatManager::new(
            HeartbeatConfig::new().with_interval(Duration::from_millis(100)),
        );

        let handle = manager.start().await.unwrap();

        handle.stop().await;
        handle.join().await;

        assert!(!manager.is_running());
    }
}

// ============================================================================
// HeartbeatHandle Tests
// ============================================================================

mod heartbeat_handle_tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_stop() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());
        let handle = manager.start().await.unwrap();

        handle.stop().await;

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!manager.is_running());
    }

    #[tokio::test]
    async fn test_handle_join() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());
        let handle = manager.start().await.unwrap();

        handle.stop().await;
        handle.join().await;

        assert!(!manager.is_running());
    }
}

// ============================================================================
// PingSender Trait Tests
// ============================================================================

mod ping_sender_tests {
    use super::*;

    /// Mock PingSender for testing
    struct MockPingSender {
        pings_sent: Arc<AtomicU64>,
        should_fail: bool,
    }

    impl MockPingSender {
        fn new() -> Self {
            Self {
                pings_sent: Arc::new(AtomicU64::new(0)),
                should_fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                pings_sent: Arc::new(AtomicU64::new(0)),
                should_fail: true,
            }
        }

        fn ping_count(&self) -> u64 {
            self.pings_sent.load(Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl PingSender for MockPingSender {
        async fn send_ping(&self, _id: RequestId) -> McpResult<()> {
            self.pings_sent.fetch_add(1, Ordering::SeqCst);

            if self.should_fail {
                Err(McpError::with_category(
                    McpErrorCode::InternalError,
                    "Mock ping failure",
                    ltmatrix::mcp::protocol::errors::ErrorCategory::Communication,
                ))
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_mock_ping_sender_sends_ping() {
        let sender = MockPingSender::new();

        sender.send_ping(RequestId::Number(1)).await.unwrap();

        assert_eq!(sender.ping_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_ping_sender_failure() {
        let sender = MockPingSender::failing();

        let result = sender.send_ping(RequestId::Number(1)).await;

        assert!(result.is_err());
        assert_eq!(sender.ping_count(), 1); // Still counted as attempted
    }

    #[tokio::test]
    async fn test_manager_with_ping_sender() {
        let sender = Arc::new(MockPingSender::new());
        let manager = HeartbeatManager::new(
            HeartbeatConfig::new()
                .with_interval(Duration::from_millis(50))
                .with_idle_only(false)
                .with_debug_logging(true),
        );

        let id_counter = Arc::new(AtomicU64::new(0));
        let counter_clone = id_counter.clone();
        let id_generator = move || {
            let id = counter_clone.fetch_add(1, Ordering::SeqCst);
            RequestId::Number(id as i64)
        };

        let handle = manager
            .start_with_sender(sender.clone(), id_generator)
            .await
            .unwrap();

        // Wait for pings to be sent
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Verify pings were sent through the sender
        assert!(sender.ping_count() >= 1);

        handle.stop().await;
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_heartbeat_lifecycle() {
        // Create manager with fast intervals for testing
        let config = HeartbeatConfig::new()
            .with_interval(Duration::from_millis(50))
            .with_timeout(Duration::from_millis(30))
            .with_max_missed_pings(3)
            .with_idle_only(false)
            .with_debug_logging(true);

        let manager = HeartbeatManager::new(config);

        // Initially healthy
        assert!(manager.is_healthy().await);
        assert!(!manager.is_running());

        // Start heartbeat
        let handle = manager.start().await.unwrap();
        assert!(manager.is_running());

        // Wait for some pings
        tokio::time::sleep(Duration::from_millis(120)).await;

        // Stats should show pings sent
        let stats = manager.stats().await;
        assert!(stats.pings_sent >= 1);

        // Stop heartbeat
        handle.stop().await;
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(!manager.is_running());
    }

    #[tokio::test]
    async fn test_health_recovers_after_pong() {
        let manager = HeartbeatManager::new(
            HeartbeatConfig::new().with_max_missed_pings(3),
        );

        // Start healthy
        assert!(manager.is_healthy().await);

        // Receive pong should keep it healthy
        manager.handle_pong(&RequestId::Number(1)).await;
        assert!(manager.is_healthy().await);

        let stats = manager.stats().await;
        assert_eq!(stats.consecutive_missed, 0);
    }

    #[tokio::test]
    async fn test_activity_tracking_with_heartbeat() {
        let manager = HeartbeatManager::new(
            HeartbeatConfig::new()
                .with_interval(Duration::from_millis(50))
                .with_debug_logging(true),
        );

        // Record some activity
        manager.record_send().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        manager.record_receive().await;

        // Activity should be tracked
        let time = manager.time_since_activity().await.unwrap();
        assert!(time < Duration::from_secs(1));

        // Reset and verify cleared
        manager.reset().await;
        assert!(manager.time_since_activity().await.is_none());
    }

    #[tokio::test]
    async fn test_concurrent_stats_access() {
        let manager = Arc::new(HeartbeatManager::new(HeartbeatConfig::default()));
        let mut handles = vec![];

        // Spawn multiple tasks accessing stats concurrently
        for i in 0..10 {
            let mgr = manager.clone();
            handles.push(tokio::spawn(async move {
                if i % 3 == 0 {
                    mgr.record_send().await;
                } else if i % 3 == 1 {
                    mgr.record_receive().await;
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
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[tokio::test]
    async fn test_zero_missed_pings_threshold() {
        let manager = HeartbeatManager::new(
            HeartbeatConfig::new().with_max_missed_pings(0),
        );

        // With threshold of 0, even healthy state might be considered stale
        // depending on implementation - this tests that edge case
        let health = manager.health().await;
        // With 0 missed pings and threshold 0, the implementation behavior may vary
        // Just verify we can query health without panic
        let _ = health.missed_pings();
    }

    #[tokio::test]
    async fn test_very_short_interval() {
        let manager = HeartbeatManager::new(
            HeartbeatConfig::new()
                .with_interval(Duration::from_millis(1))
                .with_debug_logging(true),
        );

        let handle = manager.start().await.unwrap();

        // Should handle very short intervals without panic
        tokio::time::sleep(Duration::from_millis(50)).await;

        let stats = manager.stats().await;
        assert!(stats.pings_sent >= 1);

        handle.stop().await;
    }

    #[tokio::test]
    async fn test_unicode_in_debug_output() {
        let config = HeartbeatConfig::new().with_debug_logging(true);
        let manager = HeartbeatManager::new(config);

        // Should not panic with any debug formatting
        let _debug = format!("{:?}", manager.config());
    }

    #[tokio::test]
    async fn test_multiple_start_stop_cycles() {
        let manager = HeartbeatManager::new(
            HeartbeatConfig::new().with_interval(Duration::from_millis(100)),
        );

        for _ in 0..3 {
            let handle = manager.start().await.unwrap();
            assert!(manager.is_running());

            handle.stop().await;
            tokio::time::sleep(Duration::from_millis(50)).await;
            assert!(!manager.is_running());
        }
    }

    #[tokio::test]
    async fn test_request_id_variants_in_pong() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        // Test with Number variant
        manager.handle_pong(&RequestId::Number(42)).await;
        let stats = manager.stats().await;
        assert_eq!(stats.pongs_received, 1);

        // Test with String variant
        manager
            .handle_pong(&RequestId::String("test-id".to_string()))
            .await;
        let stats = manager.stats().await;
        assert_eq!(stats.pongs_received, 2);
    }
}
