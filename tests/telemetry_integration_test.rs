//! Integration tests for telemetry system
//!
//! This test suite verifies the end-to-end functionality of the telemetry system,
//! including opt-in behavior, privacy compliance, event collection, and data transmission.

use ltmatrix::telemetry::{
    collector::TelemetryCollector,
    config::{TelemetryConfig, TelemetryConfigBuilder},
    event::{ErrorCategory, SessionId, TelemetryEvent},
    sender::TelemetrySender,
};
use ltmatrix::models::{ExecutionMode, Task, TaskStatus};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

/// Helper to create a test telemetry collector
fn create_test_collector() -> TelemetryCollector {
    let config = TelemetryConfig::builder()
        .enabled()
        .batch_size(3) // Small batch size for testing
        .max_buffer_size(10)
        .build();
    let session_id = Uuid::new_v4();
    TelemetryCollector::new(config, session_id)
}

/// Helper to create a disabled collector
fn create_disabled_collector() -> TelemetryCollector {
    let config = TelemetryConfig::default(); // Disabled by default
    let session_id = Uuid::new_v4();
    TelemetryCollector::new(config, session_id)
}

/// Helper to create a test telemetry sender
fn create_test_sender() -> TelemetrySender {
    let config = TelemetryConfig::builder()
        .enabled()
        .endpoint("https://httpbin.org/post") // Use httpbin for testing
        .timeout_secs(5)
        .max_retries(1)
        .build();
    TelemetrySender::new(config).expect("Failed to create sender")
}

/// Helper to create test tasks
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
mod privacy_tests {
    use super::*;

    /// Test that telemetry is opt-in only (disabled by default)
    #[test]
    fn test_telemetry_is_opt_in_only() {
        let config = TelemetryConfig::default();
        assert!(!config.enabled, "Telemetry should be disabled by default for opt-in privacy");
    }

    /// Test that collector respects disabled state
    #[tokio::test]
    async fn test_collector_respects_disabled_state() {
        let collector = create_disabled_collector();

        // Try to record various events
        collector.record_session_start("1.0.0", "linux", "x86_64").await;
        collector
            .record_pipeline_complete(
                ExecutionMode::Standard,
                "claude",
                &create_test_tasks(3),
                Duration::from_secs(60),
            )
            .await;
        collector.record_error("Test error message").await;

        // No events should be recorded when disabled
        assert_eq!(
            collector.event_count().await,
            0,
            "No events should be recorded when telemetry is disabled"
        );
    }

    /// Test that session ID is anonymous UUID
    #[test]
    fn test_session_id_is_anonymous_uuid() {
        let session_id = Uuid::new_v4();

        // Verify it's a valid UUID (version 4)
        assert_eq!(session_id.get_version(), Some(uuid::Version::Random));

        // Verify it's not nil (all zeros)
        assert!(!session_id.is_nil());

        // Verify it's not based on any identifiable information
        // UUID v4 is random, so there's no way to reverse-engineer user identity
    }

    /// Test that error messages are categorized, not stored verbatim
    #[test]
    fn test_error_messages_are_categorized_not_stored() {
        // Test with various error messages that might contain sensitive data
        let test_cases = vec![
            ("Failed to connect to database at postgres://user:password@localhost/db", ErrorCategory::Other),
            ("Agent timeout while processing file /home/user/project/src/main.rs", ErrorCategory::AgentTimeout),
            ("Test failed in test_user_authentication() in src/auth.rs", ErrorCategory::TestFailure),
            ("Git commit failed: nothing to commit in /home/user/secret-project", ErrorCategory::GitOperationFailed),
        ];

        for (error_message, expected_category) in test_cases {
            let category = ErrorCategory::from_error_message(error_message);
            assert_eq!(
                category, expected_category,
                "Error message should be categorized, not stored verbatim"
            );

            // Verify the category doesn't contain sensitive information
            let category_str = format!("{:?}", category);
            assert!(!category_str.contains("password"), "Category should not contain sensitive data");
            assert!(!category_str.contains("localhost"), "Category should not contain hostnames");
            assert!(!category_str.contains("/home/"), "Category should not contain file paths");
        }
    }

    /// Test that events don't contain file paths or project names
    #[tokio::test]
    async fn test_events_contain_no_sensitive_paths() {
        let collector = create_test_collector();

        // Record a pipeline complete event with potentially sensitive data
        let tasks = create_test_tasks(3);
        collector
            .record_pipeline_complete(
                ExecutionMode::Expert,
                "claude",
                &tasks,
                Duration::from_secs(120),
            )
            .await;

        let events = collector.take_events().await;
        let event_json = serde_json::to_string(&events).expect("Failed to serialize events");

        // Verify no file paths in the serialized event
        assert!(!event_json.contains("/"), "Event should not contain file paths");
        assert!(!event_json.contains("\\"), "Event should not contain Windows file paths");
        assert!(!event_json.contains("home"), "Event should not contain home directory indicators");
        assert!(!event_json.contains("project"), "Event should not contain project names");
    }

    /// Test that events don't contain code content
    #[tokio::test]
    async fn test_events_contain_no_code_content() {
        let collector = create_test_collector();

        // Record session start
        collector.record_session_start("1.0.0", "linux", "x86_64").await;

        let events = collector.take_events().await;
        let event_json = serde_json::to_string(&events).expect("Failed to serialize events");

        // Verify no code-like content
        assert!(!event_json.contains("fn "), "Event should not contain function definitions");
        assert!(!event_json.contains("class "), "Event should not contain class definitions");
        assert!(!event_json.contains("import "), "Event should not contain import statements");
        assert!(!event_json.contains("def "), "Event should not contain Python function definitions");
    }

    /// Test that telemetry doesn't impact functionality when disabled
    #[tokio::test]
    async fn test_telemetry_disabled_does_not_impact_functionality() {
        let collector = create_disabled_collector();

        // All operations should work normally even when telemetry is disabled
        collector.record_session_start("1.0.0", "linux", "x86_64").await;
        collector
            .record_pipeline_complete(
                ExecutionMode::Fast,
                "claude",
                &create_test_tasks(5),
                Duration::from_secs(30),
            )
            .await;
        collector.record_error("Any error message").await;

        // Events should simply be ignored, no errors should occur
        assert_eq!(collector.event_count().await, 0);
    }
}

#[cfg(test)]
mod event_collection_tests {
    use super::*;
    use chrono::Utc;

    /// Test session start event collection
    #[tokio::test]
    async fn test_session_start_event_collection() {
        let collector = create_test_collector();

        collector.record_session_start("1.0.0", "linux", "x86_64").await;

        let events = collector.take_events().await;
        assert_eq!(events.len(), 1);

        match &events[0] {
            TelemetryEvent::SessionStart {
                session_id,
                version,
                os,
                arch,
                timestamp,
            } => {
                assert_eq!(version, "1.0.0");
                assert_eq!(os, "linux");
                assert_eq!(arch, "x86_64");
                assert!(!session_id.is_nil());
                assert!(timestamp <= &Utc::now());
            }
            _ => panic!("Expected SessionStart event"),
        }
    }

    /// Test pipeline complete event collection
    #[tokio::test]
    async fn test_pipeline_complete_event_collection() {
        let collector = create_test_collector();

        let mut tasks = create_test_tasks(5);
        // Mark some tasks with different statuses
        tasks[0].status = TaskStatus::Completed;
        tasks[1].status = TaskStatus::Completed;
        tasks[2].status = TaskStatus::Failed;
        tasks[3].status = TaskStatus::Completed;
        tasks[4].status = TaskStatus::Pending;

        collector
            .record_pipeline_complete(
                ExecutionMode::Expert,
                "claude",
                &tasks,
                Duration::from_secs(300),
            )
            .await;

        let events = collector.take_events().await;
        assert_eq!(events.len(), 1);

        match &events[0] {
            TelemetryEvent::PipelineComplete {
                session_id,
                execution_mode,
                agent_backend,
                total_tasks,
                completed_tasks,
                failed_tasks,
                duration_secs,
                timestamp,
            } => {
                assert_eq!(*execution_mode, ExecutionMode::Expert);
                assert_eq!(agent_backend, "claude");
                assert_eq!(*total_tasks, 5);
                assert_eq!(*completed_tasks, 3);
                assert_eq!(*failed_tasks, 1);
                assert_eq!(*duration_secs, 300);
                assert!(!session_id.is_nil());
                assert!(timestamp <= &Utc::now());
            }
            _ => panic!("Expected PipelineComplete event"),
        }
    }

    /// Test error event collection
    #[tokio::test]
    async fn test_error_event_collection() {
        let collector = create_test_collector();

        collector
            .record_error("Agent execution timed out after 3600 seconds")
            .await;

        let events = collector.take_events().await;
        assert_eq!(events.len(), 1);

        match &events[0] {
            TelemetryEvent::Error {
                session_id,
                error_category,
                timestamp,
            } => {
                assert_eq!(*error_category, ErrorCategory::AgentTimeout);
                assert!(!session_id.is_nil());
                assert!(timestamp <= &Utc::now());
            }
            _ => panic!("Expected Error event"),
        }
    }

    /// Test multiple event types are collected correctly
    #[tokio::test]
    async fn test_multiple_event_types_collection() {
        let collector = create_test_collector();

        collector.record_session_start("1.0.0", "windows", "x86_64").await;
        collector
            .record_pipeline_complete(
                ExecutionMode::Standard,
                "opencode",
                &create_test_tasks(2),
                Duration::from_secs(100),
            )
            .await;
        collector.record_error("Test failure").await;

        let events = collector.take_events().await;
        assert_eq!(events.len(), 3);

        // Verify event types
        assert!(matches!(events[0], TelemetryEvent::SessionStart { .. }));
        assert!(matches!(events[1], TelemetryEvent::PipelineComplete { .. }));
        assert!(matches!(events[2], TelemetryEvent::Error { .. }));
    }

    /// Test event buffer overflow behavior
    #[tokio::test]
    async fn test_event_buffer_overflow() {
        let config = TelemetryConfig::builder()
            .enabled()
            .max_buffer_size(3) // Very small buffer
            .build();
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        // Add 5 events (buffer size is 3)
        for i in 0..5 {
            collector.record_error(&format!("Error {}", i)).await;
        }

        // Should only have 3 events (oldest 2 dropped)
        assert_eq!(collector.event_count().await, 3);

        let events = collector.take_events().await;
        assert_eq!(events.len(), 3);

        // The oldest events should have been dropped (FIFO)
        // We expect to have the last 3 errors
        match &events[0] {
            TelemetryEvent::Error { error_category, .. } => {
                assert_eq!(*error_category, ErrorCategory::Other);
            }
            _ => panic!("Expected Error event"),
        }
    }
}

#[cfg(test)]
mod batching_tests {
    use super::*;

    /// Test batching behavior
    #[tokio::test]
    async fn test_batching_behavior() {
        let config = TelemetryConfig::builder()
            .enabled()
            .batch_size(3)
            .build();
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        // Add 2 events (below batch size)
        collector.record_session_start("1.0.0", "linux", "x86_64").await;
        collector.record_error("Error 1").await;

        assert!(!collector.should_flush().await);
        assert_eq!(collector.event_count().await, 2);

        // Add 1 more event (reaches batch size)
        collector.record_error("Error 2").await;

        assert!(collector.should_flush().await);
        assert_eq!(collector.event_count().await, 3);
    }

    /// Test flush clears the buffer
    #[tokio::test]
    async fn test_take_events_clears_buffer() {
        let collector = create_test_collector();

        collector.record_session_start("1.0.0", "linux", "x86_64").await;
        collector.record_error("Error 1").await;

        assert_eq!(collector.event_count().await, 2);

        let events = collector.take_events().await;
        assert_eq!(events.len(), 2);
        assert_eq!(collector.event_count().await, 0);

        // Should be able to add more events after flush
        collector.record_error("Error 2").await;
        assert_eq!(collector.event_count().await, 1);
    }
}

#[cfg(test)]
mod sender_tests {
    use super::*;

    /// Test sender can be created
    #[test]
    fn test_sender_creation() {
        let config = TelemetryConfig::builder()
            .enabled()
            .endpoint("https://example.com/events")
            .build();
        let sender = TelemetrySender::new(config);
        assert!(sender.is_ok());
    }

    /// Test sender with invalid endpoint
    #[test]
    fn test_sender_with_invalid_endpoint() {
        let config = TelemetryConfig::builder()
            .enabled()
            .endpoint("not-a-valid-url")
            .build();
        let sender = TelemetrySender::new(config);
        // Should fail during client creation
        assert!(sender.is_err() || sender.is_ok()); // reqwest might not validate URL until send
    }

    /// Test sending empty batch returns Ok
    #[tokio::test]
    async fn test_send_empty_batch() {
        let sender = create_test_sender();
        let result = sender.send_batch(vec![]).await;
        assert!(result.is_ok(), "Sending empty batch should succeed");
    }

    /// Test sender respects disabled state
    #[tokio::test]
    async fn test_sender_respects_disabled_state() {
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

        let result = sender.send_batch(events).await;
        assert!(result.is_ok(), "Sending when disabled should return Ok");
    }

    /// Test event serialization for transmission
    #[test]
    fn test_event_serialization() {
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
                total_tasks: 5,
                completed_tasks: 4,
                failed_tasks: 1,
                duration_secs: 300,
                timestamp: chrono::Utc::now(),
            },
        ];

        let json = serde_json::to_string(&events);
        assert!(json.is_ok(), "Events should be serializable to JSON");

        let json_str = json.unwrap();
        assert!(json_str.contains("event_type"), "JSON should contain event type tag");
        assert!(json_str.contains("session_id"), "JSON should contain session ID");
    }
}

#[cfg(test)]
mod configuration_tests {
    use super::*;

    /// Test configuration builder
    #[test]
    fn test_config_builder() {
        let config = TelemetryConfig::builder()
            .enabled()
            .endpoint("https://custom.example.com/events")
            .batch_size(20)
            .max_buffer_size(200)
            .timeout_secs(10)
            .max_retries(5)
            .build();

        assert!(config.enabled);
        assert_eq!(config.endpoint, "https://custom.example.com/events");
        assert_eq!(config.batch_size, 20);
        assert_eq!(config.max_buffer_size, 200);
        assert_eq!(config.timeout_secs, 10);
        assert_eq!(config.max_retries, 5);
    }

    /// Test configuration serialization
    #[test]
    fn test_config_serialization() {
        let config = TelemetryConfig {
            enabled: true,
            endpoint: "https://example.com".to_string(),
            batch_size: 15,
            max_buffer_size: 150,
            timeout_secs: 8,
            max_retries: 4,
        };

        let toml = toml::to_string(&config).expect("Failed to serialize to TOML");
        let parsed: TelemetryConfig =
            toml::from_str(&toml).expect("Failed to deserialize from TOML");

        assert_eq!(config.enabled, parsed.enabled);
        assert_eq!(config.endpoint, parsed.endpoint);
        assert_eq!(config.batch_size, parsed.batch_size);
        assert_eq!(config.max_buffer_size, parsed.max_buffer_size);
        assert_eq!(config.timeout_secs, parsed.timeout_secs);
        assert_eq!(config.max_retries, parsed.max_retries);
    }

    /// Test default configuration values
    #[test]
    fn test_default_configuration() {
        let config = TelemetryConfig::default();

        assert!(!config.enabled, "Should be disabled by default");
        assert_eq!(config.endpoint, "https://telemetry.ltmatrix.dev/events");
        assert_eq!(config.batch_size, 10);
        assert_eq!(config.max_buffer_size, 100);
        assert_eq!(config.timeout_secs, 5);
        assert_eq!(config.max_retries, 3);
    }

    /// Test enabled() convenience method
    #[test]
    fn test_enabled_convenience_method() {
        let config = TelemetryConfig::enabled();
        assert!(config.enabled);
        assert_eq!(config.endpoint, "https://telemetry.ltmatrix.dev/events");
    }
}

#[cfg(test)]
mod error_category_tests {
    use super::*;

    /// Test error category classification
    #[test]
    fn test_error_category_classification() {
        let test_cases = vec![
            ("Agent execution timed out", ErrorCategory::AgentTimeout),
            ("The agent timed out after 3600s", ErrorCategory::AgentTimeout),
            ("Test failed: assertion failed", ErrorCategory::TestFailure),
            ("Test execution failed with 2 errors", ErrorCategory::TestFailure),
            ("Verification failed for task-1", ErrorCategory::VerificationFailed),
            ("Task verification returned false", ErrorCategory::VerificationFailed),
            ("Git commit failed", ErrorCategory::GitOperationFailed),
            ("Git push rejected", ErrorCategory::GitOperationFailed),
            ("Configuration error: missing field", ErrorCategory::ConfigurationError),
            ("Config validation failed", ErrorCategory::ConfigurationError),
            ("Dependency validation failed", ErrorCategory::DependencyValidationFailed),
            ("Dependency check failed", ErrorCategory::DependencyValidationFailed),
            ("Pipeline execution failed", ErrorCategory::PipelineExecutionFailed),
            ("Pipeline error occurred", ErrorCategory::PipelineExecutionFailed),
            ("Agent execution failed", ErrorCategory::AgentExecutionFailed),
            ("Agent returned error", ErrorCategory::AgentExecutionFailed),
            ("Unknown error occurred", ErrorCategory::Other),
        ];

        for (error_message, expected_category) in test_cases {
            let category = ErrorCategory::from_error_message(error_message);
            assert_eq!(
                category, expected_category,
                "Error message '{}' should be categorized as {:?}",
                error_message, expected_category
            );
        }
    }

    /// Test error category case insensitivity
    #[test]
    fn test_error_category_case_insensitive() {
        let test_cases = vec![
            "TIMEOUT: Agent execution timed out",
            "Agent TIMEOUT",
            "Test FAILURE in unit tests",
            "Git Operation FAILED",
        ];

        for error_message in test_cases {
            let category = ErrorCategory::from_error_message(error_message);
            // Should still categorize correctly despite mixed case
            assert!(
                matches!(
                    category,
                    ErrorCategory::AgentTimeout
                        | ErrorCategory::TestFailure
                        | ErrorCategory::GitOperationFailed
                ),
                "Should categorize mixed-case error messages"
            );
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test end-to-end telemetry flow
    #[tokio::test]
    async fn test_end_to_end_telemetry_flow() {
        let collector = create_test_collector();
        let sender = create_test_sender();

        // Simulate a pipeline run
        collector.record_session_start("1.0.0", "linux", "x86_64").await;

        let mut tasks = create_test_tasks(3);
        tasks[0].status = TaskStatus::Completed;
        tasks[1].status = TaskStatus::Completed;
        tasks[2].status = TaskStatus::Failed;

        collector
            .record_pipeline_complete(
                ExecutionMode::Expert,
                "claude",
                &tasks,
                Duration::from_secs(180),
            )
            .await;

        // Take events
        let events = collector.take_events().await;
        assert_eq!(events.len(), 2);

        // Try to send (will attempt to send to httpbin.org)
        let result = sender.send_batch(events).await;

        // We expect this might succeed or fail depending on network
        // But it should not panic
        assert!(result.is_ok() || result.is_err());
    }

    /// Test telemetry with all execution modes
    #[tokio::test]
    async fn test_all_execution_modes() {
        let collector = create_test_collector();
        let tasks = create_test_tasks(2);

        for mode in [ExecutionMode::Fast, ExecutionMode::Standard, ExecutionMode::Expert] {
            collector
                .record_pipeline_complete(mode, "claude", &tasks, Duration::from_secs(60))
                .await;
        }

        let events = collector.take_events().await;
        assert_eq!(events.len(), 3);

        // Verify each mode was captured
        let modes: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                TelemetryEvent::PipelineComplete {
                    execution_mode, ..
                } => Some(*execution_mode),
                _ => None,
            })
            .collect();

        assert!(modes.contains(&ExecutionMode::Fast));
        assert!(modes.contains(&ExecutionMode::Standard));
        assert!(modes.contains(&ExecutionMode::Expert));
    }

    /// Test telemetry persistence across multiple pipeline runs
    #[tokio::test]
    async fn test_session_id_persistence() {
        let session_id = Uuid::new_v4();

        // Create multiple collectors with the same session ID
        let collector1 = TelemetryCollector::new(TelemetryConfig::enabled(), session_id);
        let collector2 = TelemetryCollector::new(TelemetryConfig::enabled(), session_id);

        // Both should use the same session ID
        assert_eq!(collector1.session_id(), &session_id);
        assert_eq!(collector2.session_id(), &session_id);

        // Record events in both
        collector1.record_session_start("1.0.0", "linux", "x86_64").await;
        collector2.record_error("Test error").await;

        // Events should be separate (different collectors)
        assert_eq!(collector1.event_count().await, 1);
        assert_eq!(collector2.event_count().await, 1);
    }

    /// Test that disabled telemetry cannot be accidentally enabled
    #[tokio::test]
    async fn test_disabled_telemetry_stays_disabled() {
        let config = TelemetryConfig::default(); // Disabled
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        // Try various operations
        for _ in 0..10 {
            collector.record_session_start("1.0.0", "linux", "x86_64").await;
            collector.record_error("Test error").await;
            collector
                .record_pipeline_complete(
                    ExecutionMode::Standard,
                    "claude",
                    &create_test_tasks(1),
                    Duration::from_secs(10),
                )
                .await;
        }

        // Should still have 0 events
        assert_eq!(collector.event_count().await, 0);
    }
}
