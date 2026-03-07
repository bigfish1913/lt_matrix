// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Validation plugin trait
//!
//! This module defines the trait for custom validation rules.

use anyhow::Result;
use async_trait::async_trait;

use super::Plugin;
use crate::models::Task;

/// Result of a validation check
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub passed: bool,

    /// Validation messages
    pub messages: Vec<ValidationMessage>,
}

impl ValidationResult {
    /// Create a passing result
    pub fn pass() -> Self {
        Self {
            passed: true,
            messages: Vec::new(),
        }
    }

    /// Create a failing result with a message
    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            passed: false,
            messages: vec![ValidationMessage::error(message)],
        }
    }
}

/// A single validation message
#[derive(Debug, Clone)]
pub struct ValidationMessage {
    /// Message severity
    pub severity: Severity,

    /// Message text
    pub message: String,

    /// Optional location (file:line)
    pub location: Option<String>,
}

impl ValidationMessage {
    /// Create an error message
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            location: None,
        }
    }

    /// Create a warning message
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            location: None,
        }
    }
}

/// Message severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Trait for validator plugins
#[async_trait]
pub trait ValidatorPlugin: Plugin {
    /// Validate a task
    async fn validate_task(&self, task: &Task) -> Result<ValidationResult>;

    /// Validate project structure
    async fn validate_project(&self, path: &std::path::Path) -> Result<ValidationResult>;

    /// Get the validation category (e.g., "security", "style", "performance")
    fn category(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result() {
        let result = ValidationResult::pass();
        assert!(result.passed);

        let result = ValidationResult::fail("Test error");
        assert!(!result.passed);
        assert_eq!(result.messages.len(), 1);
    }

    #[test]
    fn test_validation_message() {
        let msg = ValidationMessage::error("Test error");
        assert_eq!(msg.severity, Severity::Error);

        let msg = ValidationMessage::warning("Test warning");
        assert_eq!(msg.severity, Severity::Warning);
    }
}