// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Telemetry collector
//!
//! This module handles collecting telemetry events during pipeline execution.

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::telemetry::config::TelemetryConfig;
use crate::telemetry::event::{ErrorCategory, SessionId, TelemetryEvent};
use chrono::Utc;
use ltmatrix_core::{ExecutionMode, Task};
use std::time::Duration;

/// Telemetry collector that gathers events during pipeline execution
#[derive(Debug, Clone)]
pub struct TelemetryCollector {
    /// Configuration for telemetry
    config: TelemetryConfig,

    /// Anonymous session identifier
    session_id: SessionId,

    /// Buffer of collected events
    events: Arc<Mutex<Vec<TelemetryEvent>>>,
}

impl TelemetryCollector {
    /// Create a new telemetry collector
    pub fn new(config: TelemetryConfig, session_id: SessionId) -> Self {
        TelemetryCollector {
            config,
            session_id,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Check if telemetry is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the session ID
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// Record a session start event
    pub async fn record_session_start(&self, version: &str, os: &str, arch: &str) {
        if !self.is_enabled() {
            return;
        }

        let event = TelemetryEvent::SessionStart {
            session_id: self.session_id,
            version: version.to_string(),
            os: os.to_string(),
            arch: arch.to_string(),
            timestamp: Utc::now(),
        };

        self.add_event(event).await;
        info!("Telemetry: Recorded session start");
    }

    /// Record a pipeline completion event
    pub async fn record_pipeline_complete(
        &self,
        execution_mode: ExecutionMode,
        agent_backend: &str,
        tasks: &[Task],
        duration: Duration,
    ) {
        if !self.is_enabled() {
            return;
        }

        let total_tasks = tasks.len();
        let completed_tasks = tasks.iter().filter(|t| t.is_completed()).count();
        let failed_tasks = tasks.iter().filter(|t| t.is_failed()).count();

        let event = TelemetryEvent::PipelineComplete {
            session_id: self.session_id,
            execution_mode,
            agent_backend: agent_backend.to_string(),
            total_tasks,
            completed_tasks,
            failed_tasks,
            duration_secs: duration.as_secs(),
            timestamp: Utc::now(),
        };

        self.add_event(event).await;
        info!(
            "Telemetry: Recorded pipeline completion: {} tasks, {} completed, {} failed",
            total_tasks, completed_tasks, failed_tasks
        );
    }

    /// Record an error event
    pub async fn record_error(&self, error_message: &str) {
        if !self.is_enabled() {
            return;
        }

        let error_category = ErrorCategory::from_error_message(error_message);

        let event = TelemetryEvent::Error {
            session_id: self.session_id,
            error_category,
            timestamp: Utc::now(),
        };

        self.add_event(event).await;
        debug!("Telemetry: Recorded error: {:?}", error_category);
    }

    /// Add an event to the buffer
    async fn add_event(&self, event: TelemetryEvent) {
        let mut events = self.events.lock().await;

        // Check buffer size limit
        if events.len() >= self.config.max_buffer_size {
            debug!("Telemetry buffer full, dropping oldest event");
            events.remove(0);
        }

        events.push(event);
    }

    /// Get all buffered events and clear the buffer
    pub async fn take_events(&self) -> Vec<TelemetryEvent> {
        let mut events = self.events.lock().await;
        std::mem::take(&mut *events)
    }

    /// Get the number of buffered events
    pub async fn event_count(&self) -> usize {
        let events = self.events.lock().await;
        events.len()
    }

    /// Check if we should flush events (batch size reached)
    pub async fn should_flush(&self) -> bool {
        self.event_count().await >= self.config.batch_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ltmatrix_core::TaskStatus;
    use uuid::Uuid;

    fn create_test_collector() -> TelemetryCollector {
        let config = TelemetryConfig::enabled();
        let session_id = Uuid::new_v4();
        TelemetryCollector::new(config, session_id)
    }

    #[tokio::test]
    async fn test_collector_enabled() {
        let collector = create_test_collector();
        assert!(collector.is_enabled());
    }

    #[tokio::test]
    async fn test_collector_disabled() {
        let config = TelemetryConfig::default(); // disabled by default
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);
        assert!(!collector.is_enabled());
    }

    #[tokio::test]
    async fn test_record_session_start() {
        let collector = create_test_collector();

        collector
            .record_session_start("0.1.0", "linux", "x86_64")
            .await;

        assert_eq!(collector.event_count().await, 1);
    }

    #[tokio::test]
    async fn test_record_pipeline_complete() {
        let collector = create_test_collector();

        let tasks = vec![
            Task::new("task-1", "Test task 1", "Description 1"),
            Task::new("task-2", "Test task 2", "Description 2"),
        ];

        // Mark one as completed
        let mut completed_task = tasks[0].clone();
        completed_task.status = TaskStatus::Completed;

        collector
            .record_pipeline_complete(
                ExecutionMode::Standard,
                "claude",
                &[tasks[0].clone(), completed_task],
                Duration::from_secs(60),
            )
            .await;

        assert_eq!(collector.event_count().await, 1);
    }

    #[tokio::test]
    async fn test_record_error() {
        let collector = create_test_collector();

        collector.record_error("Agent execution timed out").await;

        assert_eq!(collector.event_count().await, 1);
    }

    #[tokio::test]
    async fn test_take_events() {
        let collector = create_test_collector();

        collector
            .record_session_start("0.1.0", "linux", "x86_64")
            .await;
        collector.record_error("Test error").await;

        assert_eq!(collector.event_count().await, 2);

        let events = collector.take_events().await;
        assert_eq!(events.len(), 2);
        assert_eq!(collector.event_count().await, 0);
    }

    #[tokio::test]
    async fn test_buffer_limit() {
        let config = TelemetryConfig::builder()
            .enabled()
            .max_buffer_size(2)
            .build();
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        // Add 3 events (buffer size is 2)
        collector
            .record_session_start("0.1.0", "linux", "x86_64")
            .await;
        collector.record_error("Error 1").await;
        collector.record_error("Error 2").await;

        // Should only have 2 events (oldest dropped)
        assert_eq!(collector.event_count().await, 2);
    }

    #[tokio::test]
    async fn test_should_flush() {
        let config = TelemetryConfig::builder().enabled().batch_size(3).build();
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        collector
            .record_session_start("0.1.0", "linux", "x86_64")
            .await;
        assert!(!collector.should_flush().await);

        collector.record_error("Error 1").await;
        assert!(!collector.should_flush().await);

        collector.record_error("Error 2").await;
        assert!(collector.should_flush().await);
    }

    #[tokio::test]
    async fn test_no_events_when_disabled() {
        let config = TelemetryConfig::default(); // disabled
        let session_id = Uuid::new_v4();
        let collector = TelemetryCollector::new(config, session_id);

        collector
            .record_session_start("0.1.0", "linux", "x86_64")
            .await;
        collector.record_error("Test error").await;

        assert_eq!(collector.event_count().await, 0);
    }
}
