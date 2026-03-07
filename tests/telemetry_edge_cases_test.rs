//! Edge case and stress tests for telemetry system
//!
//! This test suite covers edge cases, boundary conditions, and stress scenarios
//! for the telemetry implementation.

use ltmatrix::telemetry::{
    collector::TelemetryCollector,
    config::TelemetryConfig,
    event::{ErrorCategory, SessionId, TelemetryEvent},
    sender::TelemetrySender,
};
use ltmatrix::models::{ExecutionMode, Task, TaskStatus};
use std::time::Duration;
use uuid::Uuid;

#[cfg(test)]
mod buffer_management_tests {
    use super::*;

    /// Test buffer overflow with many events
    #[tokio::test]
    async fn test_buffer_overflow_with_many_events() {
        let config = TelemetryConfig::builder()
            .enabled()
            .max_buffer_size(5) // Very small buffer
            .batch_size(10) // Larger than buffer
            .build();
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        // Add 20 events (buffer size is 5)
        for i in 0..20 {
            collector
                .record_error(&format!("Error number {}", i))
                .await;
        }

        // Should only have 5 events
        assert_eq!(collector.event_count().await, 5);

        // Take events and verify buffer is cleared
        let events = collector.take_events().await;
        assert_eq!(events.len(), 5);
        assert_eq!(collector.event_count().await, 0);
    }

    /// Test buffer overflow with different event types
    #[tokio::test]
    async fn test_buffer_overflow_mixed_event_types() {
        let config = TelemetryConfig::builder()
            .enabled()
            .max_buffer_size(3)
            .build();
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        // Add mixed event types
        collector.record_session_start("1.0.0", "linux", "x86_64").await;
        collector.record_error("Error 1").await;
        collector
            .record_pipeline_complete(
                ExecutionMode::Standard,
                "claude",
                &vec![Task::new("task-1", "Test", "Desc")],
                Duration::from_secs(60),
            )
            .await;
        collector.record_error("Error 2").await;

        // Should have 3 events (oldest dropped)
        assert_eq!(collector.event_count().await, 3);

        let events = collector.take_events().await;
        assert_eq!(events.len(), 3);

        // Verify we have the most recent events
        // SessionStart should be dropped (oldest)
        let event_types: Vec<_> = events
            .iter()
            .map(|e| match e {
                TelemetryEvent::SessionStart { .. } => "session_start",
                TelemetryEvent::PipelineComplete { .. } => "pipeline_complete",
                TelemetryEvent::Error { .. } => "error",
            })
            .collect();

        assert_eq!(
            event_types,
            vec!["error", "pipeline_complete", "error"]
        );
    }

    /// Test rapid event collection
    #[tokio::test]
    async fn test_rapid_event_collection() {
        let collector = TelemetryCollector::new(
            TelemetryConfig::builder()
                .enabled()
                .max_buffer_size(1000)
                .build(),
            Uuid::new_v4(),
        );

        // Add many events rapidly
        for i in 0..500 {
            collector.record_error(&format!("Error {}", i)).await;
        }

        assert_eq!(collector.event_count().await, 500);
    }

    /// Test concurrent event collection
    #[tokio::test]
    async fn test_concurrent_event_collection() {
        let collector = TelemetryCollector::new(
            TelemetryConfig::builder()
                .enabled()
                .max_buffer_size(1000)
                .build(),
            Uuid::new_v4(),
        );

        // Spawn multiple tasks to add events concurrently
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let collector = collector.clone();
                tokio::spawn(async move {
                    for j in 0..100 {
                        collector
                            .record_error(&format!("Error {}-{}", i, j))
                            .await;
                    }
                })
            })
            .collect();

        // Wait for all tasks
        for handle in handles {
            handle.await.expect("Task failed");
        }

        // Should have 1000 events
        assert_eq!(collector.event_count().await, 1000);
    }
}

#[cfg(test)]
mod batching_edge_cases_tests {
    use super::*;

    /// Test batch size equals buffer size
    #[tokio::test]
    async fn test_batch_size_equals_buffer_size() {
        let config = TelemetryConfig::builder()
            .enabled()
            .batch_size(5)
            .max_buffer_size(5)
            .build();
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        // Add 5 events (exactly at batch size)
        for _ in 0..5 {
            collector.record_error("Test error").await;
        }

        assert!(collector.should_flush().await);
        assert_eq!(collector.event_count().await, 5);
    }

    /// Test batch size larger than buffer size
    #[tokio::test]
    async fn test_batch_size_larger_than_buffer_size() {
        let config = TelemetryConfig::builder()
            .enabled()
            .batch_size(100)
            .max_buffer_size(10)
            .build();
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        // Fill buffer
        for _ in 0..10 {
            collector.record_error("Test error").await;
        }

        // Should not flush yet (buffer size < batch size)
        assert!(!collector.should_flush().await);
        assert_eq!(collector.event_count().await, 10);
    }

    /// Test batch size of 1
    #[tokio::test]
    async fn test_batch_size_of_one() {
        let config = TelemetryConfig::builder()
            .enabled()
            .batch_size(1)
            .max_buffer_size(10)
            .build();
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        collector.record_error("Error 1").await;
        assert!(collector.should_flush().await);

        collector.record_error("Error 2").await;
        assert!(collector.should_flush().await);
    }

    /// Test zero batch size (flushes immediately when events are present)
    #[tokio::test]
    async fn test_zero_batch_size() {
        let config = TelemetryConfig::builder()
            .enabled()
            .batch_size(0)
            .max_buffer_size(10)
            .build();
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        // Add events
        for _ in 0..5 {
            collector.record_error("Test error").await;
        }

        // Should flush immediately (any event count >= 0 triggers flush)
        assert!(collector.should_flush().await);
        assert_eq!(collector.event_count().await, 5);
    }
}

#[cfg(test)]
mod error_categorization_edge_cases {
    use super::*;

    /// Test empty error message
    #[test]
    fn test_empty_error_message() {
        let category = ErrorCategory::from_error_message("");
        assert_eq!(category, ErrorCategory::Other);
    }

    /// Test error message with only keywords
    #[test]
    fn test_error_message_with_only_keywords() {
        let test_cases = vec![
            ("timeout", ErrorCategory::AgentTimeout),
            ("test", ErrorCategory::TestFailure),
            ("git", ErrorCategory::GitOperationFailed),
            ("config", ErrorCategory::ConfigurationError),
        ];

        for (msg, expected) in test_cases {
            let category = ErrorCategory::from_error_message(msg);
            assert_eq!(category, expected, "Failed for message: {}", msg);
        }
    }

    /// Test error message with multiple keywords
    #[test]
    fn test_error_message_with_multiple_keywords() {
        // Should match the first pattern found
        let msg = "Test execution timeout during git operation";
        let category = ErrorCategory::from_error_message(msg);

        // Should match "timeout" first (implementation checks timeout before test)
        assert_eq!(category, ErrorCategory::AgentTimeout);
    }

    /// Test error message with special characters
    #[test]
    fn test_error_message_with_special_characters() {
        let msg = "Error: test failed! @#$%^&*()";
        let category = ErrorCategory::from_error_message(msg);
        assert_eq!(category, ErrorCategory::TestFailure);
    }

    /// Test error message with unicode characters
    #[test]
    fn test_error_message_with_unicode() {
        let msg = "Test failed: 测试失败 🚫";
        let category = ErrorCategory::from_error_message(msg);
        assert_eq!(category, ErrorCategory::TestFailure);
    }

    /// Test error message with very long content
    #[test]
    fn test_very_long_error_message() {
        let long_msg = "Test failure: ".to_string() + &"x".repeat(10000);
        let category = ErrorCategory::from_error_message(&long_msg);
        assert_eq!(category, ErrorCategory::TestFailure);
    }

    /// Test error message with newlines
    #[test]
    fn test_error_message_with_newlines() {
        let msg = "Error: test failure\n\
                   at line 42\n\
                   in function test";

        let category = ErrorCategory::from_error_message(msg);
        assert_eq!(category, ErrorCategory::TestFailure);
    }
}

#[cfg(test)]
mod session_id_tests {
    use super::*;

    /// Test different session IDs are unique
    #[test]
    fn test_session_ids_are_unique() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        assert_ne!(id1, id2, "Session IDs should be unique");
    }

    /// Test session ID consistency across events
    #[tokio::test]
    async fn test_session_id_consistency_across_events() {
        let config = TelemetryConfig::enabled();
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        // Record multiple events
        collector.record_session_start("1.0.0", "linux", "x86_64").await;
        collector.record_error("Error 1").await;
        collector
            .record_pipeline_complete(
                ExecutionMode::Standard,
                "claude",
                &vec![Task::new("task-1", "Test", "Desc")],
                Duration::from_secs(60),
            )
            .await;

        let events = collector.take_events().await;

        // All events should have the same session ID
        for event in &events {
            assert_eq!(event.session_id(), &session_id);
        }
    }

    /// Test nil UUID is not used
    #[test]
    fn test_nil_uuid_is_not_generated() {
        let id = Uuid::new_v4();
        assert!(!id.is_nil(), "UUID v4 should not be nil");
    }

    /// Test session ID format
    #[test]
    fn test_session_id_format() {
        let id = Uuid::new_v4();
        let id_string = id.to_string();

        // UUID format: 8-4-4-4-12 hex digits
        assert_eq!(id_string.len(), 36);
        assert_eq!(id_string.chars().filter(|&c| c == '-').count(), 4);
    }
}

#[cfg(test)]
mod configuration_edge_cases {
    use super::*;

    /// Test configuration with extreme values
    #[test]
    fn test_extreme_configuration_values() {
        let config = TelemetryConfig::builder()
            .enabled()
            .batch_size(1)
            .max_buffer_size(1)
            .timeout_secs(1)
            .max_retries(0)
            .build();

        assert_eq!(config.batch_size, 1);
        assert_eq!(config.max_buffer_size, 1);
        assert_eq!(config.timeout_secs, 1);
        assert_eq!(config.max_retries, 0);
    }

    /// Test configuration with very large values
    #[test]
    fn test_very_large_configuration_values() {
        let config = TelemetryConfig::builder()
            .enabled()
            .batch_size(1000000)
            .max_buffer_size(10000000)
            .timeout_secs(3600)
            .max_retries(1000)
            .build();

        assert_eq!(config.batch_size, 1000000);
        assert_eq!(config.max_buffer_size, 10000000);
        assert_eq!(config.timeout_secs, 3600);
        assert_eq!(config.max_retries, 1000);
    }

    /// Test empty endpoint string
    #[test]
    fn test_empty_endpoint_string() {
        let config = TelemetryConfig::builder().enabled().endpoint("").build();

        // Should accept empty string (though it won't work)
        assert_eq!(config.endpoint, "");
    }

    /// Test endpoint with special characters
    #[test]
    fn test_endpoint_with_special_characters() {
        let endpoint = "https://example.com:8080/path/to/endPoint?query=value&other=123#fragment";
        let config = TelemetryConfig::builder().enabled().endpoint(endpoint).build();

        assert_eq!(config.endpoint, endpoint);
    }
}

#[cfg(test)]
mod serialization_edge_cases {
    use super::*;

    /// Test serialization of all event types
    #[test]
    fn test_serialization_of_all_event_types() {
        let session_id = Uuid::new_v4();

        let events = vec![
            TelemetryEvent::SessionStart {
                session_id,
                version: "1.0.0".to_string(),
                os: "linux".to_string(),
                arch: "x86_64".to_string(),
                timestamp: chrono::Utc::now(),
            },
            TelemetryEvent::PipelineComplete {
                session_id,
                execution_mode: ExecutionMode::Standard,
                agent_backend: "claude".to_string(),
                total_tasks: 10,
                completed_tasks: 8,
                failed_tasks: 2,
                duration_secs: 300,
                timestamp: chrono::Utc::now(),
            },
            TelemetryEvent::Error {
                session_id,
                error_category: ErrorCategory::TestFailure,
                timestamp: chrono::Utc::now(),
            },
        ];

        let json = serde_json::to_string(&events);
        assert!(json.is_ok());

        // Test deserialization
        let parsed: Result<Vec<TelemetryEvent>, _> = serde_json::from_str(&json.unwrap());
        assert!(parsed.is_ok());
    }

    /// Test serialization with special characters in strings
    #[test]
    fn test_serialization_with_special_characters() {
        let session_id = Uuid::new_v4();

        let event = TelemetryEvent::SessionStart {
            session_id,
            version: "1.0.0-beta+build.123".to_string(),
            os: "Linux (Ubuntu 20.04 LTS)".to_string(),
            arch: "x86_64 (64-bit)".to_string(),
            timestamp: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&event);
        assert!(json.is_ok());

        let parsed: Result<TelemetryEvent, _> = serde_json::from_str(&json.unwrap());
        assert!(parsed.is_ok());
    }

    /// Test TOML serialization of config
    #[test]
    fn test_toml_config_serialization() {
        let config = TelemetryConfig {
            enabled: true,
            endpoint: "https://example.com/events".to_string(),
            batch_size: 42,
            max_buffer_size: 420,
            timeout_secs: 7,
            max_retries: 13,
        };

        let toml = toml::to_string(&config).expect("Failed to serialize");
        let parsed: TelemetryConfig =
            toml::from_str(&toml).expect("Failed to deserialize");

        assert_eq!(config.enabled, parsed.enabled);
        assert_eq!(config.endpoint, parsed.endpoint);
        assert_eq!(config.batch_size, parsed.batch_size);
        assert_eq!(config.max_buffer_size, parsed.max_buffer_size);
        assert_eq!(config.timeout_secs, parsed.timeout_secs);
        assert_eq!(config.max_retries, parsed.max_retries);
    }
}

#[cfg(test)]
mod disabled_state_tests {
    use super::*;

    /// Test that all operations are no-ops when disabled
    #[tokio::test]
    async fn test_all_operations_are_no_ops_when_disabled() {
        let config = TelemetryConfig::default(); // Disabled
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        // All these should do nothing
        collector.record_session_start("1.0.0", "linux", "x86_64").await;
        collector
            .record_pipeline_complete(
                ExecutionMode::Expert,
                "claude",
                &create_test_tasks(10),
                Duration::from_secs(600),
            )
            .await;
        collector
            .record_error("This is a very long error message with lots of details")
            .await;

        // Should have no events
        assert_eq!(collector.event_count().await, 0);

        // take_events should return empty vec
        let events = collector.take_events().await;
        assert!(events.is_empty());

        // should_flush should always return false
        assert!(!collector.should_flush().await);
    }

    /// Test that disabled sender doesn't send
    #[tokio::test]
    async fn test_disabled_sender_doesnt_send() {
        let config = TelemetryConfig::default(); // Disabled
        let sender = TelemetrySender::new(config).expect("Failed to create sender");

        let session_id = Uuid::new_v4();
        let events = vec![TelemetryEvent::SessionStart {
            session_id,
            version: "1.0.0".to_string(),
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            timestamp: chrono::Utc::now(),
        }];

        // Should return Ok without sending
        let result = sender.send_batch(events).await;
        assert!(result.is_ok());
    }
}

/// Helper function to create test tasks
fn create_test_tasks(count: usize) -> Vec<Task> {
    (0..count)
        .map(|i| Task::new(
            format!("task-{}", i),
            format!("Test task {}", i),
            format!("Description for task {}", i),
        ))
        .collect()
}

#[cfg(test)]
mod stress_tests {
    use super::*;

    /// Test many sequential events
    #[tokio::test]
    async fn test_many_sequential_events() {
        let collector = TelemetryCollector::new(
            TelemetryConfig::builder()
                .enabled()
                .max_buffer_size(10000)
                .batch_size(100)
                .build(),
            Uuid::new_v4(),
        );

        // Add 5000 events
        for i in 0..5000 {
            collector.record_error(&format!("Error {}", i)).await;
        }

        assert_eq!(collector.event_count().await, 5000);
    }

    /// Test event collection with large task counts
    #[tokio::test]
    async fn test_large_task_counts() {
        let collector = TelemetryCollector::new(
            TelemetryConfig::builder().enabled().build(),
            Uuid::new_v4(),
        );

        // Create a pipeline with 1000 tasks
        let tasks: Vec<Task> = (0..1000)
            .map(|i| Task::new(
                format!("task-{:04}", i),
                format!("Task {}", i),
                format!("Description {}", i),
            ))
            .collect();

        collector
            .record_pipeline_complete(
                ExecutionMode::Standard,
                "claude",
                &tasks,
                Duration::from_secs(3600),
            )
            .await;

        let events = collector.take_events().await;
        assert_eq!(events.len(), 1);

        match &events[0] {
            TelemetryEvent::PipelineComplete {
                total_tasks,
                completed_tasks,
                failed_tasks,
                ..
            } => {
                assert_eq!(*total_tasks, 1000);
                // All tasks are pending by default
                assert_eq!(*completed_tasks, 0);
                assert_eq!(*failed_tasks, 0);
            }
            _ => panic!("Expected PipelineComplete event"),
        }
    }

    /// Test rapid flush cycles
    #[tokio::test]
    async fn test_rapid_flush_cycles() {
        let collector = TelemetryCollector::new(
            TelemetryConfig::builder()
                .enabled()
                .batch_size(2)
                .max_buffer_size(100)
                .build(),
            Uuid::new_v4(),
        );

        // Add events and flush multiple times
        for cycle in 0..10 {
            collector.record_session_start("1.0.0", "linux", "x86_64").await;
            collector.record_error(&format!("Cycle {} error", cycle)).await;

            assert!(collector.should_flush().await);

            let events = collector.take_events().await;
            assert_eq!(events.len(), 2);
            assert_eq!(collector.event_count().await, 0);
        }
    }
}
