// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Tests for failure report surfacing functionality
//!
//! This test module verifies the `classify_error`, `generate_failure_report`,
//! and related functionality work correctly.

use ltmatrix::models::{Task, TaskStatus};
use ltmatrix::pipeline::execute::{
    classify_error, generate_failure_report, ErrorClass, FailureAction,
};
use std::collections::HashMap;

fn create_failed_task(id: &str, error: &str) -> Task {
    let mut task = Task::new(id, "Failed task", "This task will fail");
    task.status = TaskStatus::Failed;
    task.error = Some(error.to_string());
    task.retry_count = 1;
    task
}

#[test]
fn test_classify_error_retryable_timeout() {
    let error = "Operation timeout after 30 seconds";
    assert_eq!(classify_error(error), ErrorClass::Retryable);
}

#[test]
fn test_classify_error_retryable_rate_limit() {
    let error = "Rate limit exceeded: too many requests";
    assert_eq!(classify_error(error), ErrorClass::Retryable);
}

#[test]
fn test_classify_error_non_retryable_syntax() {
    let error = "Syntax error: unexpected token at line 42";
    assert_eq!(classify_error(error), ErrorClass::NonRetryable);
}

#[test]
fn test_classify_error_non_retryable_permission() {
    let error = "Permission denied: cannot access file";
    assert_eq!(classify_error(error), ErrorClass::NonRetryable);
}

#[test]
fn test_generate_failure_report_suggests_retry_for_retryable() {
    let task = create_failed_task("task-1", "timeout exceeded");
    let task_map: HashMap<String, Task> = [(task.id.clone(), task.clone())].into_iter().collect();
    let report = generate_failure_report(&task, &task_map);
    assert_eq!(report.suggested_action, FailureAction::Retry);
}

#[test]
fn test_generate_failure_report_suggests_skip_for_non_retryable() {
    let task = create_failed_task("task-1", "syntax error in code");
    let task_map: HashMap<String, Task> = [(task.id.clone(), task.clone())].into_iter().collect();
    let report = generate_failure_report(&task, &task_map);
    assert_eq!(report.suggested_action, FailureAction::Skip);
}

#[test]
fn test_generate_failure_report_identifies_blocked_downstream() {
    let task1 = create_failed_task("task-1", "timeout exceeded");
    let mut task2 = Task::new("task-2", "Dependent task", "Depends on task-1");
    task2.depends_on = vec!["task-1".to_string()];

    let task_map: HashMap<String, Task> =
        [(task1.id.clone(), task1.clone()), (task2.id.clone(), task2)]
            .into_iter()
            .collect();

    let report = generate_failure_report(&task1, &task_map);
    assert!(report.blocked_downstream.contains(&"task-2".to_string()));
}
