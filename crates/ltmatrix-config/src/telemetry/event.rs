// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Telemetry event definitions
//!
//! This module defines the types of events that can be collected.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use ltmatrix_core::ExecutionMode;

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

    // =========================================================================
    // TelemetryEvent Tests
    // =========================================================================

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
    fn test_session_id_extraction_pipeline_complete() {
        let session_id = Uuid::new_v4();

        let event = TelemetryEvent::PipelineComplete {
            session_id,
            execution_mode: ExecutionMode::Standard,
            agent_backend: "claude".to_string(),
            total_tasks: 10,
            completed_tasks: 8,
            failed_tasks: 2,
            duration_secs: 3600,
            timestamp: Utc::now(),
        };

        assert_eq!(event.session_id(), &session_id);
    }

    #[test]
    fn test_session_id_extraction_error() {
        let session_id = Uuid::new_v4();

        let event = TelemetryEvent::Error {
            session_id,
            error_category: ErrorCategory::AgentTimeout,
            timestamp: Utc::now(),
        };

        assert_eq!(event.session_id(), &session_id);
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

    #[test]
    fn test_telemetry_event_deserialization() {
        let session_id = Uuid::new_v4();
        let event = TelemetryEvent::SessionStart {
            session_id,
            version: "0.1.0".to_string(),
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: TelemetryEvent = serde_json::from_str(&json).unwrap();

        if let TelemetryEvent::SessionStart { session_id: parsed_id, .. } = parsed {
            assert_eq!(parsed_id, session_id);
        } else {
            panic!("Expected SessionStart event");
        }
    }

    #[test]
    fn test_pipeline_complete_serialization() {
        let event = TelemetryEvent::PipelineComplete {
            session_id: Uuid::new_v4(),
            execution_mode: ExecutionMode::Fast,
            agent_backend: "claude".to_string(),
            total_tasks: 5,
            completed_tasks: 5,
            failed_tasks: 0,
            duration_secs: 1800,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("PipelineComplete") || json.contains("pipeline_complete"));
        assert!(json.contains("Fast") || json.contains("fast"));
    }

    #[test]
    fn test_error_event_serialization() {
        let event = TelemetryEvent::Error {
            session_id: Uuid::new_v4(),
            error_category: ErrorCategory::TestFailure,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Error") || json.contains("error"));
        assert!(json.contains("TestFailure") || json.contains("test_failure"));
    }

    #[test]
    fn test_event_clone() {
        let event = TelemetryEvent::SessionStart {
            session_id: Uuid::new_v4(),
            version: "0.1.0".to_string(),
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            timestamp: Utc::now(),
        };

        let cloned = event.clone();
        if let (TelemetryEvent::SessionStart { os: os1, .. }, TelemetryEvent::SessionStart { os: os2, .. }) = (&event, &cloned) {
            assert_eq!(os1, os2);
        } else {
            panic!("Expected SessionStart events");
        }
    }

    // =========================================================================
    // ErrorCategory Tests
    // =========================================================================

    #[test]
    fn test_error_category_timeout() {
        let category = ErrorCategory::from_error_message("Agent execution timed out after 3600s");
        assert_eq!(category, ErrorCategory::AgentTimeout);
    }

    #[test]
    fn test_error_category_timeout_uppercase() {
        let category = ErrorCategory::from_error_message("TIMEOUT ERROR");
        assert_eq!(category, ErrorCategory::AgentTimeout);
    }

    #[test]
    fn test_error_category_test_failure() {
        let category = ErrorCategory::from_error_message("Test execution failed: 2 tests passed, 1 failed");
        assert_eq!(category, ErrorCategory::TestFailure);
    }

    #[test]
    fn test_error_category_test_case_insensitive() {
        let category = ErrorCategory::from_error_message("TEST case failed");
        assert_eq!(category, ErrorCategory::TestFailure);
    }

    #[test]
    fn test_error_category_verification_failed() {
        let category = ErrorCategory::from_error_message("Verification of task failed");
        assert_eq!(category, ErrorCategory::VerificationFailed);
    }

    #[test]
    fn test_error_category_verify_spelling() {
        let category = ErrorCategory::from_error_message("Could not verify the result");
        assert_eq!(category, ErrorCategory::VerificationFailed);
    }

    #[test]
    fn test_error_category_git() {
        let category = ErrorCategory::from_error_message("Git commit failed: nothing to commit");
        assert_eq!(category, ErrorCategory::GitOperationFailed);
    }

    #[test]
    fn test_error_category_git_push() {
        let category = ErrorCategory::from_error_message("git push rejected");
        assert_eq!(category, ErrorCategory::GitOperationFailed);
    }

    #[test]
    fn test_error_category_config() {
        let category = ErrorCategory::from_error_message("Configuration file not found");
        assert_eq!(category, ErrorCategory::ConfigurationError);
    }

    #[test]
    fn test_error_category_dependency() {
        let category = ErrorCategory::from_error_message("Dependency resolution failed");
        assert_eq!(category, ErrorCategory::DependencyValidationFailed);
    }

    #[test]
    fn test_error_category_dependencies() {
        let category = ErrorCategory::from_error_message("Missing dependencies");
        assert_eq!(category, ErrorCategory::DependencyValidationFailed);
    }

    #[test]
    fn test_error_category_pipeline() {
        let category = ErrorCategory::from_error_message("Pipeline stage failed");
        assert_eq!(category, ErrorCategory::PipelineExecutionFailed);
    }

    #[test]
    fn test_error_category_agent_execution() {
        let category = ErrorCategory::from_error_message("Agent returned non-zero exit code");
        assert_eq!(category, ErrorCategory::AgentExecutionFailed);
    }

    #[test]
    fn test_error_category_other() {
        let category = ErrorCategory::from_error_message("Some unknown error occurred");
        assert_eq!(category, ErrorCategory::Other);
    }

    #[test]
    fn test_error_category_empty_message() {
        let category = ErrorCategory::from_error_message("");
        assert_eq!(category, ErrorCategory::Other);
    }

    #[test]
    fn test_error_category_clone() {
        let category = ErrorCategory::AgentTimeout;
        let cloned = category.clone();
        assert_eq!(category, cloned);
    }

    #[test]
    fn test_error_category_partial_eq() {
        assert_eq!(ErrorCategory::AgentTimeout, ErrorCategory::AgentTimeout);
        assert_ne!(ErrorCategory::AgentTimeout, ErrorCategory::TestFailure);
    }

    #[test]
    fn test_error_category_serialization() {
        let category = ErrorCategory::AgentTimeout;
        let json = serde_json::to_string(&category).unwrap();
        assert!(json.contains("AgentTimeout"));

        let parsed: ErrorCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ErrorCategory::AgentTimeout);
    }

    #[test]
    fn test_all_error_categories_serializable() {
        let categories = [
            ErrorCategory::AgentTimeout,
            ErrorCategory::AgentExecutionFailed,
            ErrorCategory::TestFailure,
            ErrorCategory::VerificationFailed,
            ErrorCategory::GitOperationFailed,
            ErrorCategory::ConfigurationError,
            ErrorCategory::DependencyValidationFailed,
            ErrorCategory::PipelineExecutionFailed,
            ErrorCategory::Other,
        ];

        for category in categories {
            let json = serde_json::to_string(&category);
            assert!(json.is_ok(), "Failed to serialize {:?}", category);

            let parsed: Result<ErrorCategory, _> = serde_json::from_str(&json.unwrap());
            assert!(parsed.is_ok(), "Failed to deserialize {:?}", category);
        }
    }
}
