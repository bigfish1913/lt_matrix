//! CLI Integration tests for output formatters
//!
//! Tests the integration between CLI arguments and output formatters,
//! including the --output flag and various command combinations.

use clap::Parser;
use ltmatrix::cli::args::{Args, OutputFormat};
use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::output::{create_formatter, ExecutionResult, ReportGenerator};
use tempfile::TempDir;

/// Helper to parse CLI args
fn parse_args(args: &[&str]) -> Args {
    Args::try_parse_from(std::iter::once("ltmatrix").chain(args.iter().cloned())).unwrap()
}

/// Helper to create a minimal execution result for testing
fn create_minimal_result() -> ExecutionResult {
    let mut task = Task::new("test-1", "Test Task", "Test description");
    task.status = TaskStatus::Completed;
    task.complexity = TaskComplexity::Simple;

    ExecutionResult {
        goal: "Test goal".to_string(),
        tasks: vec![task],
        total_time: 30,
        completed_count: 1,
        failed_count: 0,
        total_retries: 0,
        dry_run: false,
    }
}

// ============================================================================
// CLI Argument Parsing Tests
// ============================================================================

#[test]
fn test_cli_output_format_default() {
    let args = parse_args(&["test goal"]);
    assert_eq!(args.output, None);
    assert_eq!(args.get_execution_mode().to_string(), "standard");
}

#[test]
fn test_cli_output_format_text() {
    let args = parse_args(&["--output", "text", "test goal"]);
    assert_eq!(args.output, Some(OutputFormat::Text));
}

#[test]
fn test_cli_output_format_json() {
    let args = parse_args(&["--output", "json", "test goal"]);
    assert_eq!(args.output, Some(OutputFormat::Json));
}

#[test]
fn test_cli_output_format_json_compact() {
    let args = parse_args(&["--output", "json-compact", "test goal"]);
    assert_eq!(args.output, Some(OutputFormat::JsonCompact));
}

#[test]
fn test_cli_output_with_dry_run() {
    let args = parse_args(&["--output", "json", "--dry-run", "test goal"]);
    assert_eq!(args.output, Some(OutputFormat::Json));
    assert!(args.dry_run);
}

#[test]
fn test_cli_output_with_fast_mode() {
    let args = parse_args(&["--output", "text", "--fast", "test goal"]);
    assert_eq!(args.output, Some(OutputFormat::Text));
    assert!(args.fast);
    assert_eq!(args.get_execution_mode().to_string(), "fast");
}

#[test]
fn test_cli_output_with_expert_mode() {
    let args = parse_args(&["--output", "json", "--expert", "test goal"]);
    assert_eq!(args.output, Some(OutputFormat::Json));
    assert!(args.expert);
    assert_eq!(args.get_execution_mode().to_string(), "expert");
}

#[test]
fn test_cli_output_with_mode_override() {
    let args = parse_args(&["--output", "json-compact", "--mode", "fast", "test goal"]);
    assert_eq!(args.output, Some(OutputFormat::JsonCompact));
    assert_eq!(args.get_execution_mode().to_string(), "fast");
}

#[test]
fn test_cli_output_with_all_options() {
    let args = parse_args(&[
        "--output",
        "json",
        "--dry-run",
        "--log-level",
        "debug",
        "--max-retries",
        "5",
        "test goal",
    ]);

    assert_eq!(args.output, Some(OutputFormat::Json));
    assert!(args.dry_run);
    assert_eq!(args.log_level.unwrap().to_string(), "debug");
    assert_eq!(args.max_retries, Some(5));
}

#[test]
fn test_cli_invalid_output_format() {
    // This should fail to parse
    let result = Args::try_parse_from(&["ltmatrix", "--output", "invalid", "test"]);
    assert!(result.is_err());
}

// ============================================================================
// Formatter Creation from CLI Args
// ============================================================================

#[test]
fn test_create_formatter_from_cli_none() {
    let args = parse_args(&["test goal"]);
    let format = args.output.unwrap_or(OutputFormat::Text);

    let formatter = create_formatter(format);
    let result = create_minimal_result();

    let output = formatter.format_result(&result).unwrap();
    assert!(!output.is_empty());
}

#[test]
fn test_create_formatter_from_cli_text() {
    let args = parse_args(&["--output", "text", "test"]);
    let format = args.output.unwrap();

    let formatter = create_formatter(format);
    let result = create_minimal_result();

    let output = formatter.format_result(&result).unwrap();
    assert!(output.contains("LTMATRIX"));
}

#[test]
fn test_create_formatter_from_cli_json() {
    let args = parse_args(&["--output", "json", "test"]);
    let format = args.output.unwrap();

    let formatter = create_formatter(format);
    let result = create_minimal_result();

    let output = formatter.format_result(&result).unwrap();

    // Verify it's valid JSON
    let _: serde_json::Value = serde_json::from_str(&output).unwrap();
}

#[test]
fn test_create_formatter_from_cli_json_compact() {
    let args = parse_args(&["--output", "json-compact", "test"]);
    let format = args.output.unwrap();

    let formatter = create_formatter(format);
    let result = create_minimal_result();

    let output = formatter.format_result(&result).unwrap();

    // Compact JSON should not have newlines
    assert!(!output.contains("\n"));

    // But should still be valid
    let _: serde_json::Value = serde_json::from_str(&output).unwrap();
}

// ============================================================================
// Report Generator Integration Tests
// ============================================================================

#[test]
fn test_report_generator_from_cli_args() {
    let args = parse_args(&["--output", "json", "test"]);
    let format = args.output.unwrap();

    let generator = ReportGenerator::new(format);
    let result = create_minimal_result();

    // Should not panic
    generator.generate_report_to_stdout(&result).unwrap();
}

#[tokio::test]
async fn test_report_file_from_cli_output_path() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.json");

    let args = parse_args(&["--output", "json", "test"]);
    let format = args.output.unwrap();

    let generator = ReportGenerator::new(format);
    let result = create_minimal_result();

    generator
        .generate_report_to_file(&result, &output_path)
        .await
        .unwrap();

    assert!(output_path.exists());

    // Verify JSON format
    let content = tokio::fs::read_to_string(&output_path).await.unwrap();
    let _: serde_json::Value = serde_json::from_str(&content).unwrap();
}

#[tokio::test]
async fn test_report_file_different_formats() {
    let temp_dir = TempDir::new().unwrap();

    let test_cases = vec![
        (OutputFormat::Text, "report.txt"),
        (OutputFormat::Json, "report.json"),
        (OutputFormat::JsonCompact, "report-compact.json"),
    ];

    for (format, filename) in test_cases {
        let output_path = temp_dir.path().join(filename);

        let generator = ReportGenerator::new(format);
        let result = create_minimal_result();

        generator
            .generate_report_to_file(&result, &output_path)
            .await
            .unwrap();

        assert!(
            output_path.exists(),
            "Failed for format: {:?} with file: {}",
            format,
            filename
        );

        let content = tokio::fs::read_to_string(&output_path).await.unwrap();
        assert!(!content.is_empty());
    }
}

// ============================================================================
// End-to-End CLI Workflow Tests
// ============================================================================

#[test]
fn test_full_cli_workflow_text_output() {
    // Simulate: ltmatrix --output text "implement feature"
    let args = parse_args(&["--output", "text", "implement feature"]);

    assert_eq!(args.output, Some(OutputFormat::Text));
    assert_eq!(args.goal.as_ref().unwrap(), "implement feature");

    // Create formatter
    let format = args.output.unwrap();
    let formatter = create_formatter(format);

    // Simulate execution result
    let mut result = create_minimal_result();
    result.goal = "implement feature".to_string();

    // Generate output
    let output = formatter.format_result(&result).unwrap();
    assert!(output.contains("implement feature"));
    assert!(output.contains("LTMATRIX"));
}

#[test]
fn test_full_cli_workflow_json_output() {
    // Simulate: ltmatrix --output json "fix bug"
    let args = parse_args(&["--output", "json", "fix bug"]);

    assert_eq!(args.output, Some(OutputFormat::Json));
    assert_eq!(args.goal.as_ref().unwrap(), "fix bug");

    let format = args.output.unwrap();
    let formatter = create_formatter(format);

    let mut result = create_minimal_result();
    result.goal = "fix bug".to_string();

    let output = formatter.format_result(&result).unwrap();

    // Verify JSON structure
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["goal"], "fix bug");
    assert!(parsed["summary"].is_object());
    assert!(parsed["tasks"].is_array());
}

#[test]
fn test_full_cli_workflow_dry_run_json() {
    // Simulate: ltmatrix --output json --dry-run "plan refactoring"
    let args = parse_args(&["--output", "json", "--dry-run", "plan refactoring"]);

    assert_eq!(args.output, Some(OutputFormat::Json));
    assert!(args.dry_run);

    let format = args.output.unwrap();
    let formatter = create_formatter(format);

    let mut result = create_minimal_result();
    result.goal = "plan refactoring".to_string();
    result.dry_run = true;

    let output = formatter.format_result(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(parsed["goal"], "plan refactoring");
    assert_eq!(parsed["mode"], "dry_run");
}

#[test]
fn test_full_cli_workflow_fast_mode() {
    // Simulate: ltmatrix --output text --fast "quick fix"
    let args = parse_args(&["--output", "text", "--fast", "quick fix"]);

    assert_eq!(args.output, Some(OutputFormat::Text));
    assert!(args.fast);

    let format = args.output.unwrap();
    let formatter = create_formatter(format);

    let mut result = create_minimal_result();
    result.goal = "quick fix".to_string();

    let output = formatter.format_result(&result).unwrap();
    assert!(output.contains("quick fix"));
}

#[test]
fn test_full_cli_workflow_expert_mode_compact_json() {
    // Simulate: ltmatrix --output json-compact --expert "critical system"
    let args = parse_args(&["--output", "json-compact", "--expert", "critical system"]);

    assert_eq!(args.output, Some(OutputFormat::JsonCompact));
    assert!(args.expert);

    let format = args.output.unwrap();
    let formatter = create_formatter(format);

    let mut result = create_minimal_result();
    result.goal = "critical system".to_string();

    let output = formatter.format_result(&result).unwrap();

    // Compact JSON should be single line
    assert!(!output.contains("\n"));

    // But still valid
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["goal"], "critical system");
}

// ============================================================================
// Output Format Selection Logic
// ============================================================================

#[test]
fn test_output_format_display() {
    assert_eq!(OutputFormat::Text.to_string(), "text");
    assert_eq!(OutputFormat::Json.to_string(), "json");
    assert_eq!(OutputFormat::JsonCompact.to_string(), "json-compact");
}

#[test]
fn test_all_output_formats_produce_output() {
    let result = create_minimal_result();

    for format in [
        OutputFormat::Text,
        OutputFormat::Json,
        OutputFormat::JsonCompact,
    ] {
        let formatter = create_formatter(format);
        let output = formatter.format_result(&result).unwrap();

        assert!(
            !output.is_empty(),
            "Format {:?} produced empty output",
            format
        );
    }
}

#[test]
fn test_output_format_consistency() {
    // Same result should produce consistent data across formats
    let result = create_minimal_result();

    // Terminal output
    let text_formatter = create_formatter(OutputFormat::Text);
    let text_output = text_formatter.format_result(&result).unwrap();
    assert!(text_output.contains("Test goal"));

    // JSON output
    let json_formatter = create_formatter(OutputFormat::Json);
    let json_output = json_formatter.format_result(&result).unwrap();
    let json_parsed: serde_json::Value = serde_json::from_str(&json_output).unwrap();
    assert_eq!(json_parsed["goal"], "Test goal");

    // Compact JSON output
    let compact_formatter = create_formatter(OutputFormat::JsonCompact);
    let compact_output = compact_formatter.format_result(&result).unwrap();
    let compact_parsed: serde_json::Value = serde_json::from_str(&compact_output).unwrap();
    assert_eq!(compact_parsed["goal"], "Test goal");
}

// ============================================================================
// Real-World Scenario Tests
// ============================================================================

#[test]
fn test_scenario_successful_task_execution() {
    // Scenario: User runs ltmatrix to implement a feature, all tasks succeed
    let args = parse_args(&["--output", "json", "implement user authentication"]);

    let mut task1 = Task::new("task-1", "Design database schema", "Create user table");
    task1.status = TaskStatus::Completed;
    task1.complexity = TaskComplexity::Moderate;

    let mut task2 = Task::new(
        "task-2",
        "Implement API endpoints",
        "Create login/signup routes",
    );
    task2.status = TaskStatus::Completed;
    task2.complexity = TaskComplexity::Complex;
    task2.depends_on = vec!["task-1".to_string()];

    let mut result = create_minimal_result();
    result.goal = "implement user authentication".to_string();
    result.tasks = vec![task1, task2];
    result.completed_count = 2;
    result.failed_count = 0;
    result.total_time = 300;

    let format = args.output.unwrap();
    let formatter = create_formatter(format);
    let output = formatter.format_result(&result).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["goal"], "implement user authentication");
    assert_eq!(parsed["summary"]["completed"], 2);
    assert_eq!(parsed["summary"]["failed"], 0);
    assert_eq!(parsed["summary"]["success_rate"], 100.0);
}

#[test]
fn test_scenario_partial_failure_with_retries() {
    // Scenario: Some tasks fail but succeed after retries
    let args = parse_args(&["--output", "json", "add error handling"]);

    let mut task1 = Task::new("task-1", "Add validation", "Validate inputs");
    task1.status = TaskStatus::Completed;
    task1.complexity = TaskComplexity::Simple;

    let mut task2 = Task::new(
        "task-2",
        "Add error middleware",
        "Error handling middleware",
    );
    task2.status = TaskStatus::Completed;
    task2.complexity = TaskComplexity::Moderate;
    task2.retry_count = 2; // Failed twice before succeeding
    task2.error = Some("Initial attempt failed, corrected on retry".to_string());

    let mut result = create_minimal_result();
    result.goal = "add error handling".to_string();
    result.tasks = vec![task1, task2];
    result.completed_count = 2;
    result.failed_count = 0;
    result.total_retries = 2;
    result.total_time = 180;

    let format = args.output.unwrap();
    let formatter = create_formatter(format);
    let output = formatter.format_result(&result).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["summary"]["total_retries"], 2);
    assert_eq!(parsed["tasks"][1]["retry_count"], 2);
}

#[test]
fn test_scenario_dry_run_planning() {
    // Scenario: User runs in dry-run mode to see what would be done
    let args = parse_args(&["--output", "text", "--dry-run", "refactor database layer"]);

    let mut task1 = Task::new("task-1", "Analyze current schema", "Review existing tables");
    task1.status = TaskStatus::Pending;
    task1.complexity = TaskComplexity::Simple;

    let mut task2 = Task::new("task-2", "Design new schema", "Create optimized structure");
    task2.status = TaskStatus::Pending;
    task2.complexity = TaskComplexity::Complex;

    let mut result = create_minimal_result();
    result.goal = "refactor database layer".to_string();
    result.tasks = vec![task1, task2];
    result.completed_count = 0;
    result.failed_count = 0;
    result.total_time = 0;
    result.dry_run = true;

    let format = args.output.unwrap();
    let formatter = create_formatter(format);
    let output = formatter.format_result(&result).unwrap();

    assert!(output.contains("DRY RUN"));
    assert!(output.contains("refactor database layer"));
}

#[tokio::test]
async fn test_scenario_save_report_to_file() {
    // Scenario: User wants to save execution report to a file
    let temp_dir = TempDir::new().unwrap();
    let report_path = temp_dir.path().join("execution_report.json");

    let args = parse_args(&["--output", "json", "build microservice"]);

    let mut result = create_minimal_result();
    result.goal = "build microservice".to_string();
    result.total_time = 600;

    let format = args.output.unwrap();
    let generator = ReportGenerator::new(format);

    generator
        .generate_report_to_file(&result, &report_path)
        .await
        .unwrap();

    assert!(report_path.exists());

    let content = tokio::fs::read_to_string(&report_path).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["goal"], "build microservice");
}

#[test]
fn test_scenario_mixed_complexity_breakdown() {
    // Scenario: Real project with various task complexities
    let args = parse_args(&["--output", "json", "implement e-commerce platform"]);

    let mut simple_task = Task::new("task-1", "Add logo", "Simple UI addition");
    simple_task.status = TaskStatus::Completed;
    simple_task.complexity = TaskComplexity::Simple;

    let mut moderate_task = Task::new("task-2", "Create product model", "Database schema");
    moderate_task.status = TaskStatus::Completed;
    moderate_task.complexity = TaskComplexity::Moderate;

    let mut complex_task = Task::new(
        "task-3",
        "Implement payment processing",
        "Stripe integration",
    );
    complex_task.status = TaskStatus::Completed;
    complex_task.complexity = TaskComplexity::Complex;

    let mut result = create_minimal_result();
    result.goal = "implement e-commerce platform".to_string();
    result.tasks = vec![simple_task, moderate_task, complex_task];
    result.completed_count = 3;
    result.total_time = 900;

    let format = args.output.unwrap();
    let formatter = create_formatter(format);
    let output = formatter.format_result(&result).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["complexity_breakdown"]["simple"], 1);
    assert_eq!(parsed["complexity_breakdown"]["moderate"], 1);
    assert_eq!(parsed["complexity_breakdown"]["complex"], 1);
}
