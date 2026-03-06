//! Comprehensive tests for output formatters
//!
//! Tests cover:
//! - TerminalFormatter with colors and progress bars
//! - JsonFormatter (pretty and compact modes)
//! - MarkdownFormatter (detailed and summary-only)
//! - ReportGenerator (stdout and file output)
//! - CLI --output flag integration
//! - Edge cases and error handling

use ltmatrix::cli::args::OutputFormat;
use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::output::{
    create_formatter, ExecutionResult, Formatter, JsonFormatter, MarkdownFormatter,
    ReportGenerator, TaskUpdateType, TerminalFormatter,
};
use chrono::{Duration, Utc};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test task with various states
fn create_test_task(
    id: &str,
    title: &str,
    status: TaskStatus,
    complexity: TaskComplexity,
) -> Task {
    let mut task = Task::new(id, title, "Test description");
    task.status = status;
    task.complexity = complexity;
    task
}

/// Helper to create a task with timing information
fn create_task_with_timing(
    id: &str,
    title: &str,
    status: TaskStatus,
    duration_secs: i64,
) -> Task {
    let mut task = Task::new(id, title, "Test description");
    task.status = status;

    if duration_secs > 0 {
        let now = Utc::now();
        task.started_at = Some(now - Duration::seconds(duration_secs));
        task.completed_at = Some(now);
    }

    task
}

/// Helper to create a task with dependencies
fn create_task_with_deps(id: &str, deps: Vec<&str>) -> Task {
    let mut task = Task::new(id, "Test Task", "Test description");
    task.depends_on = deps.into_iter().map(String::from).collect();
    task
}

/// Helper to create a task with subtasks
fn create_task_with_subtasks(id: &str, subtask_count: usize) -> Task {
    let mut task = Task::new(id, "Parent Task", "Parent task description");
    task.complexity = TaskComplexity::Complex;

    for i in 0..subtask_count {
        let mut subtask = Task::new(&format!("{}-sub{}", id, i + 1), "Subtask", "Subtask description");
        subtask.status = TaskStatus::Completed;
        task.subtasks.push(subtask);
    }

    task
}

/// Helper to create a failed task with error
fn create_failed_task(id: &str, error: &str) -> Task {
    let mut task = Task::new(id, "Failed Task", "This task failed");
    task.status = TaskStatus::Failed;
    task.error = Some(error.to_string());
    task.retry_count = 2;
    task
}

/// Helper to create a comprehensive execution result
fn create_execution_result(
    goal: &str,
    completed_count: usize,
    failed_count: usize,
    total_time: u64,
    dry_run: bool,
) -> ExecutionResult {
    let mut tasks = Vec::new();

    // Add completed tasks
    for i in 0..completed_count {
        let complexity = match i % 3 {
            0 => TaskComplexity::Simple,
            1 => TaskComplexity::Moderate,
            _ => TaskComplexity::Complex,
        };
        tasks.push(create_test_task(
            &format!("task-{}", i + 1),
            &format!("Completed Task {}", i + 1),
            TaskStatus::Completed,
            complexity,
        ));
    }

    // Add failed tasks
    for i in 0..failed_count {
        tasks.push(create_failed_task(
            &format!("failed-{}", i + 1),
            &format!("Error message {}", i + 1),
        ));
    }

    let total_retries = tasks
        .iter()
        .map(|t| t.retry_count)
        .sum::<u32>();

    ExecutionResult {
        goal: goal.to_string(),
        tasks,
        total_time,
        completed_count,
        failed_count,
        total_retries,
        dry_run,
    }
}

// ============================================================================
// TerminalFormatter Tests
// ============================================================================

#[test]
fn test_terminal_formatter_default_creation() {
    let formatter = TerminalFormatter::new();
    let result = create_execution_result("Test goal", 3, 1, 120, false);

    let output = formatter.format_result(&result).unwrap();

    // Verify header
    assert!(output.contains("LTMATRIX EXECUTION REPORT"));
    assert!(output.contains("Test goal"));

    // Verify summary section
    assert!(output.contains("SUMMARY"));
    assert!(output.contains("Total Tasks: 4"));
    assert!(output.contains("Completed: 3"));
    assert!(output.contains("Failed: 1"));

    // Verify complexity breakdown
    assert!(output.contains("COMPLEXITY BREAKDOWN"));
}

#[test]
fn test_terminal_formatter_dry_run_mode() {
    let formatter = TerminalFormatter::new();
    let result = create_execution_result("Dry run test", 2, 0, 0, true);

    let output = formatter.format_result(&result).unwrap();

    assert!(output.contains("DRY RUN"));
}

#[test]
fn test_terminal_formatter_task_update_types() {
    let formatter = TerminalFormatter::new();
    let task = create_test_task("task-1", "Test Task", TaskStatus::InProgress, TaskComplexity::Moderate);

    // Test all update types
    let started = formatter.format_task_update(&task, TaskUpdateType::Started).unwrap();
    assert!(started.contains("started"));
    assert!(started.contains("task-1"));

    let in_progress = formatter.format_task_update(&task, TaskUpdateType::InProgress).unwrap();
    assert!(in_progress.contains("in progress"));

    let completed = formatter.format_task_update(&task, TaskUpdateType::Completed).unwrap();
    assert!(completed.contains("completed"));

    let failed = formatter.format_task_update(&task, TaskUpdateType::Failed).unwrap();
    assert!(failed.contains("failed"));

    let retrying = formatter.format_task_update(&task, TaskUpdateType::Retrying).unwrap();
    assert!(retrying.contains("retrying"));
    assert!(retrying.contains("attempt")); // Should show attempt number
}

#[test]
fn test_terminal_formatter_progress_bar() {
    let formatter = TerminalFormatter::new();

    // Test various progress states
    let progress_0 = formatter.format_progress(0, 10, "Starting").unwrap();
    assert!(progress_0.contains("0/10"));
    assert!(progress_0.contains("0%"));

    let progress_50 = formatter.format_progress(5, 10, "Halfway").unwrap();
    assert!(progress_50.contains("5/10"));
    assert!(progress_50.contains("50%"));

    let progress_100 = formatter.format_progress(10, 10, "Complete").unwrap();
    assert!(progress_100.contains("10/10"));
    assert!(progress_100.contains("100%"));
}

#[test]
fn test_terminal_formatter_no_progress_mode() {
    let formatter = TerminalFormatter::no_progress();
    let progress = formatter.format_progress(5, 10, "Test").unwrap();

    // Should not have progress bar characters
    assert!(!progress.contains("█"));
    assert!(!progress.contains("░"));

    // Should still have basic info
    assert!(progress.contains("5/10"));
}

#[test]
fn test_terminal_formatter_with_dependencies() {
    let formatter = TerminalFormatter::new();
    let mut result = create_execution_result("Test", 1, 0, 60, false);
    result.tasks[0] = create_task_with_deps("task-1", vec!["task-0", "task-dep"]);

    let output = formatter.format_result(&result).unwrap();
    assert!(output.contains("Dependencies"));
    assert!(output.contains("task-0"));
    assert!(output.contains("task-dep"));
}

#[test]
fn test_terminal_formatter_with_subtasks() {
    let formatter = TerminalFormatter::new();
    let mut result = create_execution_result("Test", 1, 0, 60, false);
    result.tasks[0] = create_task_with_subtasks("task-1", 3);

    let output = formatter.format_result(&result).unwrap();
    assert!(output.contains("Subtasks"));
    assert!(output.contains("3"));
}

#[test]
fn test_terminal_formatter_with_error() {
    let formatter = TerminalFormatter::new();
    let mut result = create_execution_result("Test", 0, 1, 30, false);
    result.tasks[0] = create_failed_task("task-1", "Critical error occurred");

    let output = formatter.format_result(&result).unwrap();
    assert!(output.contains("Error"));
    assert!(output.contains("Critical error occurred"));
}

#[test]
fn test_terminal_formatter_timing_information() {
    let formatter = TerminalFormatter::new();
    let mut result = create_execution_result("Test", 1, 0, 120, false);
    result.tasks[0] = create_task_with_timing("task-1", "Timed Task", TaskStatus::Completed, 45);

    let output = formatter.format_result(&result).unwrap();
    assert!(output.contains("Duration"));
    assert!(output.contains("45s"));
}

#[test]
fn test_terminal_formatter_empty_tasks() {
    let formatter = TerminalFormatter::new();
    let result = ExecutionResult {
        goal: "Empty test".to_string(),
        tasks: vec![],
        total_time: 0,
        completed_count: 0,
        failed_count: 0,
        total_retries: 0,
        dry_run: false,
    };

    let output = formatter.format_result(&result).unwrap();
    assert!(output.contains("Total Tasks: 0"));
    assert!(output.contains("Success Rate: 0.0%"));
}

// ============================================================================
// JsonFormatter Tests
// ============================================================================

#[test]
fn test_json_formatter_valid_output() {
    let formatter = JsonFormatter::new();
    let result = create_execution_result("JSON test", 2, 1, 90, false);

    let output = formatter.format_result(&result).unwrap();

    // Verify valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    // Check top-level structure
    assert_eq!(parsed["goal"], "JSON test");
    assert_eq!(parsed["mode"], "execution");

    // Check summary
    assert_eq!(parsed["summary"]["total_tasks"], 3);
    assert_eq!(parsed["summary"]["completed"], 2);
    assert_eq!(parsed["summary"]["failed"], 1);
    assert_eq!(parsed["summary"]["total_time_seconds"], 90);

    // Check complexity breakdown
    assert!(parsed["complexity_breakdown"].is_object());

    // Check tasks array
    assert!(parsed["tasks"].is_array());
    assert_eq!(parsed["tasks"].as_array().unwrap().len(), 3);
}

#[test]
fn test_json_formatter_dry_run() {
    let formatter = JsonFormatter::new();
    let result = create_execution_result("Dry run", 0, 0, 0, true);

    let output = formatter.format_result(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(parsed["mode"], "dry_run");
}

#[test]
fn test_json_formatter_compact_mode() {
    let formatter = JsonFormatter::compact();
    let result = create_execution_result("Compact test", 1, 0, 30, false);

    let output = formatter.format_result(&result).unwrap();

    // Compact mode should not have newlines
    assert!(!output.contains("\n"));

    // Should still be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["goal"], "Compact test");
}

#[test]
fn test_json_formatter_task_details() {
    let formatter = JsonFormatter::new();
    let mut result = create_execution_result("Details test", 1, 0, 60, false);
    result.tasks[0] = create_task_with_timing("task-1", "Timed", TaskStatus::Completed, 45);

    let output = formatter.format_result(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let task = &parsed["tasks"][0];
    assert_eq!(task["id"], "task-1");
    assert_eq!(task["title"], "Timed");
    assert_eq!(task["status"], "Completed");
    assert!(task["started_at"].is_string());
    assert!(task["completed_at"].is_string());
    assert_eq!(task["duration_seconds"], 45);
}

#[test]
fn test_json_formatter_error_handling() {
    let formatter = JsonFormatter::new();
    let mut result = create_execution_result("Error test", 0, 1, 15, false);
    result.tasks[0] = create_failed_task("task-1", "Test error");

    let output = formatter.format_result(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let task = &parsed["tasks"][0];
    assert_eq!(task["status"], "Failed");
    assert_eq!(task["error"], "Test error");
    assert_eq!(task["retry_count"], 2);
}

#[test]
fn test_json_formatter_task_update() {
    let formatter = JsonFormatter::new();
    let task = create_test_task("task-1", "Test", TaskStatus::InProgress, TaskComplexity::Moderate);

    let update = formatter.format_task_update(&task, TaskUpdateType::Started).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&update).unwrap();

    assert_eq!(parsed["type"], "Started");
    assert_eq!(parsed["task"]["id"], "task-1");
    assert_eq!(parsed["task"]["title"], "Test");
}

#[test]
fn test_json_formatter_progress() {
    let formatter = JsonFormatter::new();
    let progress = formatter.format_progress(7, 10, "Almost done").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&progress).unwrap();

    assert_eq!(parsed["current"], 7);
    assert_eq!(parsed["total"], 10);
    assert_eq!(parsed["percentage"], 70.0);
    assert_eq!(parsed["message"], "Almost done");
}

#[test]
fn test_json_formatter_zero_division() {
    let formatter = JsonFormatter::new();
    let result = ExecutionResult {
        goal: "Zero test".to_string(),
        tasks: vec![],
        total_time: 0,
        completed_count: 0,
        failed_count: 0,
        total_retries: 0,
        dry_run: false,
    };

    let output = formatter.format_result(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    // Should handle division by zero gracefully
    assert_eq!(parsed["summary"]["success_rate"], 0.0);
}

// ============================================================================
// MarkdownFormatter Tests
// ============================================================================

#[test]
fn test_markdown_formatter_structure() {
    let formatter = MarkdownFormatter::new();
    let result = create_execution_result("MD test", 2, 0, 85, false);

    let output = formatter.format_result(&result).unwrap();

    // Check main sections
    assert!(output.contains("# LTMATRIX Execution Report"));
    assert!(output.contains("## Summary"));
    assert!(output.contains("## Complexity Breakdown"));
    assert!(output.contains("## Task Details"));

    // Check footer
    assert!(output.contains("*Generated by ltmatrix"));
}

#[test]
fn test_markdown_formatter_summary_only() {
    let formatter = MarkdownFormatter::summary_only();
    let result = create_execution_result("Summary test", 1, 0, 30, false);

    let output = formatter.format_result(&result).unwrap();

    // Should have summary
    assert!(output.contains("## Summary"));

    // Should NOT have task details
    assert!(!output.contains("## Task Details"));
}

#[test]
fn test_markdown_formatter_goal_and_mode() {
    let formatter = MarkdownFormatter::new();

    // Test execution mode
    let exec_result = create_execution_result("Exec goal", 1, 0, 30, false);
    let exec_output = formatter.format_result(&exec_result).unwrap();
    assert!(exec_output.contains("**Goal:** Exec goal"));
    assert!(exec_output.contains("**Mode:** Execution"));

    // Test dry run mode
    let dry_result = create_execution_result("Dry goal", 1, 0, 0, true);
    let dry_output = formatter.format_result(&dry_result).unwrap();
    assert!(dry_output.contains("**Mode:** Dry Run"));
}

#[test]
fn test_markdown_formatter_task_details() {
    let formatter = MarkdownFormatter::new();
    let mut result = create_execution_result("Details", 1, 0, 60, false);
    result.tasks[0] = create_task_with_deps("task-1", vec!["dep-1", "dep-2"]);

    let output = formatter.format_result(&result).unwrap();

    // Check task section
    assert!(output.contains("### 1. task-1"));
    assert!(output.contains("**Title:**"));
    assert!(output.contains("**Description:**"));
    assert!(output.contains("**Status:**"));
    assert!(output.contains("**Complexity:**"));
    assert!(output.contains("**Dependencies:** dep-1, dep-2"));
}

#[test]
fn test_markdown_formatter_with_subtasks() {
    let formatter = MarkdownFormatter::new();
    let mut result = create_execution_result("Subtasks", 1, 0, 90, false);
    result.tasks[0] = create_task_with_subtasks("parent-1", 3);

    let output = formatter.format_result(&result).unwrap();

    assert!(output.contains("**Subtasks:** 3 items"));
    assert!(output.contains("1. parent-1-sub1"));
    assert!(output.contains("2. parent-1-sub2"));
    assert!(output.contains("3. parent-1-sub3"));
}

#[test]
fn test_markdown_formatter_with_error() {
    let formatter = MarkdownFormatter::new();
    let mut result = create_execution_result("Error", 0, 1, 20, false);
    result.tasks[0] = create_failed_task("task-1", "Error message");

    let output = formatter.format_result(&result).unwrap();

    assert!(output.contains("**Error:** Error message"));
    assert!(output.contains("**Retries:** 2"));
}

#[test]
fn test_markdown_formatter_task_update() {
    let formatter = MarkdownFormatter::new();
    let task = create_test_task("task-1", "Test Task", TaskStatus::Completed, TaskComplexity::Simple);

    let update = formatter.format_task_update(&task, TaskUpdateType::Completed).unwrap();
    assert!(update.contains("✓"));
    assert!(update.contains("**task-1**"));
    assert!(update.contains("Test Task"));
}

#[test]
fn test_markdown_formatter_progress() {
    let formatter = MarkdownFormatter::new();
    let progress = formatter.format_progress(3, 4, "Almost").unwrap();

    assert!(progress.contains("**Progress:**"));
    assert!(progress.contains("[3/4]"));
    assert!(progress.contains("75%"));
    assert!(progress.contains("Almost"));
}

// ============================================================================
// create_formatter Tests
// ============================================================================

#[test]
fn test_create_formatter_text() {
    let formatter = create_formatter(OutputFormat::Text);
    let result = create_execution_result("Test", 1, 0, 30, false);

    let output = formatter.format_result(&result).unwrap();
    // TerminalFormatter produces text output
    assert!(!output.is_empty());
}

#[test]
fn test_create_formatter_json() {
    let formatter = create_formatter(OutputFormat::Json);
    let result = create_execution_result("Test", 1, 0, 30, false);

    let output = formatter.format_result(&result).unwrap();

    // Should be valid JSON
    let _: serde_json::Value = serde_json::from_str(&output).unwrap();

    // JsonFormatter (non-compact) should have newlines
    assert!(output.contains("\n"));
}

#[test]
fn test_create_formatter_json_compact() {
    let formatter = create_formatter(OutputFormat::JsonCompact);
    let result = create_execution_result("Test", 1, 0, 30, false);

    let output = formatter.format_result(&result).unwrap();

    // Should be valid JSON
    let _: serde_json::Value = serde_json::from_str(&output).unwrap();

    // Compact should not have newlines
    assert!(!output.contains("\n"));
}

// ============================================================================
// ReportGenerator Tests
// ============================================================================

#[test]
fn test_report_generator_stdout() {
    let generator = ReportGenerator::new(OutputFormat::Text);
    let result = create_execution_result("Stdout test", 2, 0, 60, false);

    // Should not panic
    generator.generate_report_to_stdout(&result).unwrap();
}

#[test]
fn test_report_generator_stdout_json() {
    let generator = ReportGenerator::new(OutputFormat::Json);
    let result = create_execution_result("JSON stdout", 1, 0, 30, false);

    // Capture stdout by redirecting would be complex, just ensure no panic
    generator.generate_report_to_stdout(&result).unwrap();
}

#[tokio::test]
async fn test_report_generator_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("report.txt");

    let generator = ReportGenerator::new(OutputFormat::Text);
    let result = create_execution_result("File test", 1, 0, 45, false);

    generator
        .generate_report_to_file(&result, &file_path)
        .await
        .unwrap();

    // Verify file was created and contains content
    assert!(file_path.exists());
    let content = tokio::fs::read_to_string(&file_path).await.unwrap();
    assert!(content.contains("LTMATRIX EXECUTION REPORT"));
    assert!(content.contains("File test"));
}

#[tokio::test]
async fn test_report_generator_to_file_json() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("report.json");

    let generator = ReportGenerator::new(OutputFormat::Json);
    let result = create_execution_result("JSON file", 2, 1, 90, false);

    generator
        .generate_report_to_file(&result, &file_path)
        .await
        .unwrap();

    // Verify file was created with valid JSON
    assert!(file_path.exists());
    let content = tokio::fs::read_to_string(&file_path).await.unwrap();
    let _: serde_json::Value = serde_json::from_str(&content).unwrap();
}

#[tokio::test]
async fn test_report_generator_creates_directories() {
    let temp_dir = TempDir::new().unwrap();
    let nested_path = temp_dir.path().join("nested").join("dir").join("report.txt");

    let generator = ReportGenerator::new(OutputFormat::Text);
    let result = create_execution_result("Nested test", 1, 0, 30, false);

    generator
        .generate_report_to_file(&result, &nested_path)
        .await
        .unwrap();

    // Verify nested directories were created
    assert!(nested_path.exists());
    assert!(nested_path.parent().unwrap().exists());
}

#[test]
fn test_report_generator_print_task_update() {
    let generator = ReportGenerator::new(OutputFormat::Text);
    let task = create_test_task("task-1", "Update test", TaskStatus::InProgress, TaskComplexity::Moderate);

    // Should not panic
    generator
        .print_task_update(&task, TaskUpdateType::Started)
        .unwrap();
}

#[test]
fn test_report_generator_print_progress() {
    let generator = ReportGenerator::new(OutputFormat::Text);

    // Should not panic
    generator.print_progress(5, 10, "Halfway").unwrap();
    generator.finish_progress();
}

#[test]
fn test_report_generator_all_formats() {
    for format in [OutputFormat::Text, OutputFormat::Json, OutputFormat::JsonCompact] {
        let generator = ReportGenerator::new(format);
        let result = create_execution_result("Format test", 1, 0, 30, false);

        // All formats should work
        generator.generate_report_to_stdout(&result).unwrap();
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_workflow_terminal() {
    let result = create_execution_result("Full workflow", 3, 1, 150, false);

    let generator = ReportGenerator::new(OutputFormat::Text);

    // Print updates
    for (i, task) in result.tasks.iter().enumerate() {
        let update_type = if task.is_completed() {
            TaskUpdateType::Completed
        } else if task.is_failed() {
            TaskUpdateType::Failed
        } else {
            TaskUpdateType::InProgress
        };

        generator.print_task_update(task, update_type).unwrap();
        generator.print_progress(i + 1, result.tasks.len(), "Processing").unwrap();
    }

    generator.finish_progress();

    // Print final report
    generator.generate_report_to_stdout(&result).unwrap();
}

#[test]
fn test_full_workflow_json() {
    let result = create_execution_result("JSON workflow", 2, 0, 75, false);

    let generator = ReportGenerator::new(OutputFormat::Json);

    // Test all methods
    for task in &result.tasks {
        generator
            .print_task_update(task, TaskUpdateType::Completed)
            .unwrap();
    }

    generator.generate_report_to_stdout(&result).unwrap();
}

#[tokio::test]
async fn test_full_workflow_file_output() {
    let temp_dir = TempDir::new().unwrap();

    let result = create_execution_result("File workflow", 2, 1, 120, true);

    // Generate reports in different formats
    let formats = [
        (OutputFormat::Text, PathBuf::from("report.txt")),
        (OutputFormat::Json, PathBuf::from("report.json")),
        (OutputFormat::JsonCompact, PathBuf::from("report-compact.json")),
    ];

    for (format, filename) in formats {
        let generator = ReportGenerator::new(format);
        let filepath = temp_dir.path().join(filename);

        generator
            .generate_report_to_file(&result, &filepath)
            .await
            .unwrap();

        assert!(filepath.exists());

        // Verify content
        let content = tokio::fs::read_to_string(&filepath).await.unwrap();
        assert!(!content.is_empty());

        // JSON formats should be valid
        if matches!(format, OutputFormat::Json | OutputFormat::JsonCompact) {
            let _: serde_json::Value = serde_json::from_str(&content).unwrap();
        }
    }
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_empty_task_list() {
    let formatter = TerminalFormatter::new();
    let result = ExecutionResult {
        goal: "Empty".to_string(),
        tasks: vec![],
        total_time: 0,
        completed_count: 0,
        failed_count: 0,
        total_retries: 0,
        dry_run: false,
    };

    // Should handle gracefully
    let output = formatter.format_result(&result).unwrap();
    assert!(output.contains("Total Tasks: 0"));
}

#[test]
fn test_all_tasks_failed() {
    let formatter = JsonFormatter::new();
    let mut result = create_execution_result("All failed", 0, 3, 45, false);

    // Create 3 failed tasks
    for i in 0..3 {
        result.tasks.push(create_failed_task(&format!("fail-{}", i), "Task failed"));
    }
    result.failed_count = 3;

    let output = formatter.format_result(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(parsed["summary"]["completed"], 0);
    assert_eq!(parsed["summary"]["failed"], 3);
    assert_eq!(parsed["summary"]["success_rate"], 0.0);
}

#[test]
fn test_mixed_complexity_tasks() {
    let formatter = MarkdownFormatter::new();
    let mut result = create_execution_result("Mixed complexity", 3, 0, 90, false);

    result.tasks[0].complexity = TaskComplexity::Simple;
    result.tasks[1].complexity = TaskComplexity::Moderate;
    result.tasks[2].complexity = TaskComplexity::Complex;

    let output = formatter.format_result(&result).unwrap();

    // Should mention all complexity levels
    assert!(output.contains("**Simple:** 1 tasks"));
    assert!(output.contains("**Moderate:** 1 tasks"));
    assert!(output.contains("**Complex:** 1 tasks"));
}

#[test]
fn test_task_with_many_retries() {
    let formatter = TerminalFormatter::new();
    let mut task = create_failed_task("retry-task", "Persistent error");
    task.retry_count = 10;

    let mut result = create_execution_result("Retry test", 0, 1, 60, false);
    result.tasks[0] = task;
    result.total_retries = 10;

    let output = formatter.format_result(&result).unwrap();

    assert!(output.contains("Total Retries: 10"));
    assert!(output.contains("Retries: 10"));
}

#[test]
fn test_long_task_titles() {
    let formatter = TerminalFormatter::new();
    let long_title = "This is a very long task title that might cause formatting issues \
                      but should still be handled gracefully by the formatter";

    let task = create_test_task("long-1", long_title, TaskStatus::Pending, TaskComplexity::Moderate);
    let mut result = create_execution_result("Long title", 1, 0, 30, false);
    result.tasks[0] = task;

    // Should handle without panicking
    let output = formatter.format_result(&result).unwrap();
    assert!(output.contains(long_title));
}

#[test]
fn test_special_characters_in_output() {
    let formatter = JsonFormatter::new();
    let mut task = create_test_task("special-1", "Task with quotes", TaskStatus::Completed, TaskComplexity::Simple);
    task.description = "Description with \"quotes\"".to_string();
    task.error = Some("Error: 'test' with \"mixed\" quotes".to_string());

    let mut result = create_execution_result("Special chars", 1, 0, 30, false);
    result.tasks[0] = task;

    // JSON should properly escape special characters
    let output = formatter.format_result(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(parsed["tasks"][0]["title"], "Task with quotes");
    assert!(parsed["tasks"][0]["error"].is_string());
}

#[test]
fn test_unicode_characters() {
    let formatter = TerminalFormatter::new();
    let result = create_execution_result("Unicode test: 你好 🎉", 1, 0, 30, false);

    // Should handle Unicode gracefully
    let output = formatter.format_result(&result).unwrap();
    assert!(output.contains("你好"));
    assert!(output.contains("🎉"));
}

#[test]
fn test_zero_total_time() {
    let formatter = JsonFormatter::new();
    let result = create_execution_result("Zero time", 1, 0, 0, false);

    let output = formatter.format_result(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(parsed["summary"]["total_time_seconds"], 0);
}

#[test]
fn test_large_number_of_tasks() {
    let formatter = TerminalFormatter::new();
    let mut result = ExecutionResult {
        goal: "Many tasks".to_string(),
        tasks: vec![],
        total_time: 600,
        completed_count: 100,
        failed_count: 0,
        total_retries: 0,
        dry_run: false,
    };

    // Create 100 tasks
    for i in 0..100 {
        result.tasks.push(create_test_task(
            &format!("task-{}", i),
            &format!("Task {}", i),
            TaskStatus::Completed,
            TaskComplexity::Simple,
        ));
    }

    // Should handle large task lists
    let output = formatter.format_result(&result).unwrap();
    assert!(output.contains("Total Tasks: 100"));
    assert!(output.contains("Success Rate: 100.0%"));
}

#[test]
fn test_deep_subtask_hierarchy() {
    let formatter = JsonFormatter::new();

    // Create a task with deeply nested subtasks
    let mut root_task = Task::new("root", "Root Task", "Root description");
    root_task.complexity = TaskComplexity::Complex;

    let mut level1 = Task::new("level1", "Level 1", "L1 desc");
    let mut level2 = Task::new("level2", "Level 2", "L2 desc");
    let mut level3 = Task::new("level3", "Level 3", "L3 desc");

    level3.status = TaskStatus::Completed;
    level2.subtasks.push(level3);
    level2.status = TaskStatus::Completed;
    level1.subtasks.push(level2);
    level1.status = TaskStatus::Completed;
    root_task.subtasks.push(level1);
    root_task.status = TaskStatus::Completed;

    let mut result = create_execution_result("Deep hierarchy", 1, 0, 120, false);
    result.tasks[0] = root_task;

    let output = formatter.format_result(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    // Should handle nested structure
    assert!(parsed["tasks"][0]["subtasks_count"].is_number());
}
