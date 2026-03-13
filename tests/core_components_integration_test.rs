// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Integration tests for core component interactions
//!
//! These tests verify that the main components work together correctly:
//! - Agent backend and session management
//! - Telemetry collection and sending
//! - Plugin system integration
//! - Configuration propagation

use chrono::Utc;
use ltmatrix::agent::backend::{
    AgentConfig, AgentConfigBuilder, AgentError, AgentResponse, AgentSession, ExecutionConfig,
    MemorySession,
};
use ltmatrix::agent::session::{SessionData, SessionManager};
use ltmatrix::models::{ExecutionMode, Task, TaskStatus};
use ltmatrix::telemetry::collector::TelemetryCollector;
use ltmatrix::telemetry::config::TelemetryConfig;
use ltmatrix::telemetry::event::{ErrorCategory, TelemetryEvent};
use std::time::Duration;
use uuid::Uuid;

// ============================================================================
// Agent Backend Integration Tests
// ============================================================================

mod agent_backend_integration {
    use super::*;
    use ltmatrix::agent::AgentSession; // Import the trait

    #[test]
    fn test_agent_config_to_execution_config() {
        // Create agent config
        let agent_config = AgentConfig::builder()
            .name("claude")
            .model("claude-sonnet-4-6")
            .command("claude")
            .timeout_secs(7200)
            .max_retries(5)
            .enable_session(true)
            .build();

        // Create matching execution config
        let exec_config = ExecutionConfig {
            model: agent_config.model.clone(),
            max_retries: agent_config.max_retries,
            timeout: agent_config.timeout_secs,
            enable_session: agent_config.enable_session,
            env_vars: vec![],
        };

        assert_eq!(exec_config.model, "claude-sonnet-4-6");
        assert_eq!(exec_config.max_retries, 5);
        assert_eq!(exec_config.timeout, 7200);
        assert!(exec_config.enable_session);
    }

    #[test]
    fn test_memory_session_lifecycle() {
        let mut session = MemorySession::default();

        // Initial state
        assert_eq!(session.reuse_count(), 0);
        assert!(!session.is_stale());

        // Simulate multiple accesses
        for i in 1..=5 {
            session.mark_accessed();
            assert_eq!(session.reuse_count(), i);
        }

        // Still not stale after recent access
        assert!(!session.is_stale());
    }

    #[test]
    fn test_agent_response_from_execution() {
        // Simulate successful execution response
        let response = AgentResponse {
            output: "Task completed successfully".to_string(),
            structured_data: Some(serde_json::json!({
                "files_modified": 3,
                "tests_passed": 5
            })),
            is_complete: true,
            error: None,
        };

        assert!(response.is_complete);
        assert!(response.error.is_none());
        assert!(response.structured_data.is_some());

        let data = response.structured_data.unwrap();
        assert_eq!(data["files_modified"], 3);
    }

    #[test]
    fn test_agent_error_categories() {
        // Test that different errors map to correct categories
        let timeout_error = AgentError::Timeout {
            command: "claude".to_string(),
            timeout_secs: 3600,
        };

        let exec_error = AgentError::ExecutionFailed {
            command: "claude".to_string(),
            message: "Process exited with code 1".to_string(),
        };

        // Verify error messages are descriptive
        assert!(timeout_error.to_string().contains("timed out"));
        assert!(exec_error.to_string().contains("failed"));
    }

    #[test]
    fn test_config_validation_prevents_invalid_configs() {
        // Empty name should fail
        let invalid_config = AgentConfig::builder()
            .name("")
            .model("claude-sonnet-4-6")
            .command("claude")
            .build();

        assert!(invalid_config.validate().is_err());

        // Zero timeout should fail
        let invalid_config = AgentConfig::builder()
            .name("claude")
            .model("claude-sonnet-4-6")
            .command("claude")
            .timeout_secs(0)
            .build();

        assert!(invalid_config.validate().is_err());
    }
}

// ============================================================================
// Session Management Integration Tests
// ============================================================================

mod session_management_integration {
    use super::*;

    #[tokio::test]
    async fn test_session_reuse_across_executions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(temp_dir.path()).unwrap();

        // Create initial session
        let session = manager
            .create_session("claude", "claude-sonnet-4-6")
            .await
            .unwrap();

        let session_id = session.session_id.clone();

        // Simulate multiple loads (reuse scenarios)
        for expected_count in 1..=3 {
            let loaded = manager.load_session(&session_id).await.unwrap().unwrap();
            assert_eq!(loaded.reuse_count, expected_count);
        }
    }

    #[tokio::test]
    async fn test_session_persistence_across_manager_instances() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create session with first manager
        let manager1 = SessionManager::new(temp_dir.path()).unwrap();
        let session = manager1
            .create_session("claude", "claude-sonnet-4-6")
            .await
            .unwrap();

        let session_id = session.session_id.clone();

        // Create new manager and verify session persists
        let manager2 = SessionManager::new(temp_dir.path()).unwrap();
        let loaded = manager2.load_session(&session_id).await.unwrap().unwrap();

        assert_eq!(loaded.session_id, session_id);
        assert_eq!(loaded.agent_name, "claude");
    }

    #[tokio::test]
    async fn test_session_cleanup_removes_only_stale() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(temp_dir.path()).unwrap();

        // Create fresh session
        let fresh = manager
            .create_session("claude", "claude-sonnet-4-6")
            .await
            .unwrap();

        // Create stale session (by modifying file directly)
        let stale = manager
            .create_session("claude", "claude-sonnet-4-6")
            .await
            .unwrap();

        // Manually set stale session's last_accessed time
        let mut stale_data = stale.clone();
        stale_data.last_accessed = Utc::now() - chrono::Duration::hours(2);
        manager.save_session(&stale_data).await.unwrap();

        // Cleanup should remove only the stale session
        let cleaned = manager.cleanup_stale_sessions().await.unwrap();
        assert_eq!(cleaned, 1);

        // Fresh session should still exist
        assert!(manager
            .load_session(&fresh.session_id)
            .await
            .unwrap()
            .is_some());

        // Stale session should be gone
        assert!(manager
            .load_session(&stale.session_id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_multiple_agents_separate_sessions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(temp_dir.path()).unwrap();

        // Create sessions for different agents
        let claude = manager
            .create_session("claude", "claude-sonnet-4-6")
            .await
            .unwrap();
        let opencode = manager.create_session("opencode", "gpt-4").await.unwrap();

        // Verify they're separate
        assert_ne!(claude.session_id, opencode.session_id);
        assert_ne!(claude.agent_name, opencode.agent_name);

        // List should contain both
        let sessions = manager.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
    }
}

// ============================================================================
// Telemetry Integration Tests
// ============================================================================

mod telemetry_integration {
    use super::*;

    #[tokio::test]
    async fn test_telemetry_event_flow() {
        let config = TelemetryConfig::enabled();
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        // Record session start
        collector
            .record_session_start("0.1.0", "linux", "x86_64")
            .await;

        // Record pipeline completion
        let tasks = vec![
            Task::new("task-1", "Task 1", "Description"),
            Task::new("task-2", "Task 2", "Description"),
        ];

        collector
            .record_pipeline_complete(
                ExecutionMode::Standard,
                "claude",
                &tasks,
                Duration::from_secs(120),
            )
            .await;

        // Record an error
        collector.record_error("Agent timeout occurred").await;

        // Verify events collected
        assert_eq!(collector.event_count().await, 3);

        // Take events and verify
        let events = collector.take_events().await;
        assert_eq!(events.len(), 3);
        assert_eq!(collector.event_count().await, 0);
    }

    #[tokio::test]
    async fn test_telemetry_disabled_no_events() {
        let config = TelemetryConfig::default(); // disabled
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        // Try to record events
        collector
            .record_session_start("0.1.0", "linux", "x86_64")
            .await;
        collector.record_error("Test error").await;

        // Should have no events
        assert_eq!(collector.event_count().await, 0);
    }

    #[tokio::test]
    async fn test_telemetry_buffer_management() {
        let config = TelemetryConfig::builder()
            .enabled()
            .max_buffer_size(3)
            .batch_size(2)
            .build();
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        // Add events up to and beyond buffer limit
        collector
            .record_session_start("0.1.0", "linux", "x86_64")
            .await;
        collector.record_error("Error 1").await;
        collector.record_error("Error 2").await;
        collector.record_error("Error 3").await; // Should trigger buffer limit

        // Buffer should be capped at max_buffer_size
        assert_eq!(collector.event_count().await, 3);

        // Should be ready to flush
        assert!(collector.should_flush().await);
    }

    #[test]
    fn test_error_category_from_agent_errors() {
        // Test that agent errors map to correct telemetry categories
        let timeout_cat = ErrorCategory::from_error_message("Agent execution timed out");
        assert_eq!(timeout_cat, ErrorCategory::AgentTimeout);

        let agent_cat = ErrorCategory::from_error_message("Agent failed to execute");
        assert_eq!(agent_cat, ErrorCategory::AgentExecutionFailed);

        let config_cat = ErrorCategory::from_error_message("Invalid configuration");
        assert_eq!(config_cat, ErrorCategory::ConfigurationError);
    }
}

// ============================================================================
// Configuration Integration Tests
// ============================================================================

mod configuration_integration {
    use super::*;

    #[test]
    fn test_agent_config_telemetry_config_alignment() {
        // Verify agent config timeout aligns with telemetry timeout
        let agent_config = AgentConfig::default();
        let telemetry_config = TelemetryConfig::default();

        // Telemetry timeout should be less than agent timeout
        // (so telemetry doesn't block agent operations)
        assert!(telemetry_config.timeout_secs < agent_config.timeout_secs);
    }

    #[test]
    fn test_execution_mode_propagation() {
        // Test that execution mode affects various configurations
        let modes = [
            ExecutionMode::Fast,
            ExecutionMode::Standard,
            ExecutionMode::Expert,
        ];

        for mode in modes {
            // Verify mode can be used in telemetry
            let event = TelemetryEvent::PipelineComplete {
                session_id: Uuid::new_v4(),
                execution_mode: mode,
                agent_backend: "claude".to_string(),
                total_tasks: 1,
                completed_tasks: 1,
                failed_tasks: 0,
                duration_secs: 60,
                timestamp: Utc::now(),
            };

            // Verify serialization works
            let json = serde_json::to_string(&event).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_config_builder_patterns() {
        // Test that builder patterns work consistently across config types
        let agent_config = AgentConfig::builder()
            .name("test")
            .model("test-model")
            .command("test-cmd")
            .timeout_secs(1800)
            .max_retries(2)
            .enable_session(false)
            .build();

        let telemetry_config = TelemetryConfig::builder()
            .enabled()
            .endpoint("https://example.com/telemetry")
            .batch_size(20)
            .timeout_secs(10)
            .build();

        // Verify all settings applied correctly
        assert_eq!(agent_config.name, "test");
        assert_eq!(agent_config.timeout_secs, 1800);
        assert!(!agent_config.enable_session);

        assert!(telemetry_config.enabled);
        assert_eq!(telemetry_config.batch_size, 20);
        assert_eq!(telemetry_config.timeout_secs, 10);
    }
}

// ============================================================================
// Cross-Component Integration Tests
// ============================================================================

mod cross_component_integration {
    use super::*;

    #[tokio::test]
    async fn test_full_session_telemetry_flow() {
        // Simulate a full flow: create session -> execute -> record telemetry
        let temp_dir = tempfile::tempdir().unwrap();
        let session_manager = SessionManager::new(temp_dir.path()).unwrap();

        let telemetry_config = TelemetryConfig::enabled();
        let session_id = Uuid::new_v4();
        let telemetry = TelemetryCollector::new(telemetry_config, session_id);

        // 1. Create session
        let session = session_manager
            .create_session("claude", "claude-sonnet-4-6")
            .await
            .unwrap();

        // 2. Record session start
        telemetry
            .record_session_start("0.1.0", "linux", "x86_64")
            .await;

        // 3. Simulate task execution
        let tasks = vec![Task::new("task-1", "Implement feature", "Description")];

        // 4. Record completion
        telemetry
            .record_pipeline_complete(
                ExecutionMode::Standard,
                "claude",
                &tasks,
                Duration::from_secs(300),
            )
            .await;

        // 5. Verify session was reused (loaded and incremented count)
        let loaded_session = session_manager
            .load_session(&session.session_id)
            .await
            .unwrap()
            .unwrap();
        assert!(loaded_session.reuse_count >= 1);

        // 6. Verify telemetry events
        let events = telemetry.take_events().await;
        assert_eq!(events.len(), 2);

        // Verify event types
        let has_session_start = events
            .iter()
            .any(|e| matches!(e, TelemetryEvent::SessionStart { .. }));
        let has_pipeline_complete = events
            .iter()
            .any(|e| matches!(e, TelemetryEvent::PipelineComplete { .. }));
        assert!(has_session_start);
        assert!(has_pipeline_complete);
    }

    #[test]
    fn test_error_handling_consistency() {
        // Verify error handling is consistent across components

        // Agent error
        let agent_error = AgentError::Timeout {
            command: "claude".to_string(),
            timeout_secs: 3600,
        };

        // Telemetry error category
        let category = ErrorCategory::from_error_message(&agent_error.to_string());

        // Should categorize as timeout
        assert_eq!(category, ErrorCategory::AgentTimeout);
    }

    #[test]
    fn test_session_data_memory_session_alignment() {
        // Verify SessionData and MemorySession have compatible fields

        // Create SessionData
        let session_data = SessionData::new("claude", "claude-sonnet-4-6");

        // Create equivalent MemorySession
        let memory_session = MemorySession {
            session_id: session_data.session_id.clone(),
            agent_name: session_data.agent_name.clone(),
            model: session_data.model.clone(),
            created_at: session_data.created_at,
            last_accessed: session_data.last_accessed,
            reuse_count: session_data.reuse_count,
        };

        // Verify alignment
        assert_eq!(session_data.session_id, memory_session.session_id);
        assert_eq!(session_data.agent_name, memory_session.agent_name);
        assert_eq!(session_data.model, memory_session.model);
    }

    #[tokio::test]
    async fn test_concurrent_session_operations() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = std::sync::Arc::new(SessionManager::new(temp_dir.path()).unwrap());

        // Create initial sessions
        let session1 = manager
            .create_session("claude", "claude-sonnet-4-6")
            .await
            .unwrap();
        let session2 = manager.create_session("opencode", "gpt-4").await.unwrap();

        let id1 = session1.session_id.clone();
        let id2 = session2.session_id.clone();

        // Spawn concurrent operations
        let m1 = manager.clone();
        let m2 = manager.clone();

        let handle1 = tokio::spawn(async move {
            for _ in 0..5 {
                let _ = m1.load_session(&id1).await;
            }
        });

        let handle2 = tokio::spawn(async move {
            for _ in 0..5 {
                let _ = m2.load_session(&id2).await;
            }
        });

        // Wait for completion
        handle1.await.unwrap();
        handle2.await.unwrap();

        // Verify sessions still exist
        let sessions = manager.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
    }
}

// ============================================================================
// Edge Cases and Stress Tests
// ============================================================================

mod edge_cases {
    use super::*;

    #[tokio::test]
    async fn test_empty_session_list() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(temp_dir.path()).unwrap();

        let sessions = manager.list_sessions().await.unwrap();
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_load_nonexistent_session() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(temp_dir.path()).unwrap();

        let result = manager.load_session("nonexistent-id").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_session() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(temp_dir.path()).unwrap();

        let deleted = manager.delete_session("nonexistent-id").await.unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_empty_agent_config_validation() {
        let config = AgentConfig {
            name: "".to_string(),
            model: "".to_string(),
            command: "".to_string(),
            timeout_secs: 0,
            max_retries: 0,
            enable_session: false,
        };

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_large_agent_response() {
        // Test handling of large responses
        let large_output = "x".repeat(1_000_000); // 1MB of data

        let response = AgentResponse {
            output: large_output.clone(),
            structured_data: None,
            is_complete: true,
            error: None,
        };

        assert_eq!(response.output.len(), 1_000_000);
    }

    #[tokio::test]
    async fn test_rapid_session_creation_and_deletion() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(temp_dir.path()).unwrap();

        // Create and immediately delete many sessions
        let mut session_ids = Vec::new();

        for _ in 0..50 {
            let session = manager
                .create_session("claude", "claude-sonnet-4-6")
                .await
                .unwrap();
            session_ids.push(session.session_id);
        }

        assert_eq!(manager.list_sessions().await.unwrap().len(), 50);

        // Delete all
        for id in session_ids {
            manager.delete_session(&id).await.unwrap();
        }

        assert!(manager.list_sessions().await.unwrap().is_empty());
    }

    #[test]
    fn test_unicode_in_agent_names() {
        let config = AgentConfig::builder()
            .name("claude-中文-日本語")
            .model("claude-sonnet-4-6")
            .command("claude")
            .build();

        assert!(config.validate().is_ok());
    }
}
