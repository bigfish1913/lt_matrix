// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Telemetry event definitions
//!
//! This module defines the types of events that can be collected.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::ExecutionMode;

/// Anonymous session identifier
///
/// Generated once and persisted locally to track sessions across runs
/// without revealing user identity.
pub type SessionId = Uuid;

/// Telemetry events that can be collected
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum TelemetryEvent {
    /// Event sent when a telemetry session starts
    SessionStart {
        /// Anonymous session identifier
        session_id: SessionId,
        /// ltmatrix version
        version: String,
        /// Operating system
        os: String,
        /// System architecture
        arch: String,
        /// Timestamp when event occurred
        timestamp: DateTime<Utc>,
    },

    /// Event sent when a pipeline execution completes
    PipelineComplete {
        /// Anonymous session identifier
        session_id: SessionId,
        /// Execution mode used
        execution_mode: ExecutionMode,
        /// Agent backend used (e.g., "claude", "opencode")
        agent_backend: String,
        /// Total number of tasks
        total_tasks: usize,
        /// Number of tasks completed successfully
        completed_tasks: usize,
        /// Number of tasks that failed
        failed_tasks: usize,
        /// Pipeline duration in seconds
        duration_secs: u64,
        /// Timestamp when event occurred
        timestamp: DateTime<Utc>,
    },

    /// Event sent when an error occurs
    Error {
        /// Anonymous session identifier
        session_id: SessionId,
        /// Error category (not full error message)
        error_category: ErrorCategory,
        /// Timestamp when event occurred
        timestamp: DateTime<Utc>,
    },
}

impl TelemetryEvent {
    /// Get the session ID for this event
    pub fn session_id(&self) -> &SessionId {
        match self {
            TelemetryEvent::SessionStart { session_id, .. } => session_id,
            TelemetryEvent::PipelineComplete { session_id, .. } => session_id,
            TelemetryEvent::Error { session_id, .. } => session_id,
        }
    }
}

/// Categories of errors for telemetry
///
/// We only collect the category, not full error messages,
/// to protect user privacy and avoid leaking sensitive information.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Agent execution timed out
    AgentTimeout,

    /// Agent execution failed
    AgentExecutionFailed,

    /// Test execution failed
    TestFailure,

    /// Task verification failed
    VerificationFailed,

    /// Git operation failed
    GitOperationFailed,

    /// Configuration error
    ConfigurationError,

    /// Dependency validation failed
    DependencyValidationFailed,

    /// Pipeline execution failed
    PipelineExecutionFailed,

    /// Other uncategorized error
    Other,
}

impl ErrorCategory {
    /// Create an error category from an error message
    ///
    /// This analyzes error messages and categorizes them
    /// without including the full message.
    pub fn from_error_message(msg: &str) -> Self {
        let msg_lower = msg.to_lowercase();

        if msg_lower.contains("timeout") || msg_lower.contains("timed out") {
            return ErrorCategory::AgentTimeout;
        }

        if msg_lower.contains("test") {
            return ErrorCategory::TestFailure;
        }

        if msg_lower.contains("verif") {
            return ErrorCategory::VerificationFailed;
        }

        if msg_lower.contains("git") {
            return ErrorCategory::GitOperationFailed;
        }

        if msg_lower.contains("config") {
            return ErrorCategory::ConfigurationError;
        }

        if msg_lower.contains("depend") {
            return ErrorCategory::DependencyValidationFailed;
        }

        if msg_lower.contains("pipeline") {
            return ErrorCategory::PipelineExecutionFailed;
        }

        if msg_lower.contains("agent") {
            return ErrorCategory::AgentExecutionFailed;
        }

        ErrorCategory::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_extraction() {
        let session_id = Uuid::new_v4();

        let event = TelemetryEvent::SessionStart {
            session_id,
            version: "0.1.0".to_string(),
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            timestamp: Utc::now(),
        };

        assert_eq!(event.session_id(), &session_id);
    }

    #[test]
    fn test_error_category_timeout() {
        let category = ErrorCategory::from_error_message("Agent execution timed out after 3600s");
        assert_eq!(category, ErrorCategory::AgentTimeout);
    }

    #[test]
    fn test_error_category_test_failure() {
        let category = ErrorCategory::from_error_message("Test execution failed: 2 tests passed, 1 failed");
        assert_eq!(category, ErrorCategory::TestFailure);
    }

    #[test]
    fn test_error_category_git() {
        let category = ErrorCategory::from_error_message("Git commit failed: nothing to commit");
        assert_eq!(category, ErrorCategory::GitOperationFailed);
    }

    #[test]
    fn test_error_category_other() {
        let category = ErrorCategory::from_error_message("Some unknown error occurred");
        assert_eq!(category, ErrorCategory::Other);
    }

    #[test]
    fn test_telemetry_event_serialization() {
        let event = TelemetryEvent::SessionStart {
            session_id: Uuid::new_v4(),
            version: "0.1.0".to_string(),
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event);
        assert!(json.is_ok());
    }
}
