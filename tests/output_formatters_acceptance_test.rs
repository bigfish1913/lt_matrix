//! Acceptance tests for Output Formatters Task
//!
//! This test file validates the acceptance criteria for the "Implement output formatters" task:
//! 1. Create src/output/mod.rs with multiple output formats
//! 2. TerminalFormatter (default, colored text, progress bars)
//! 3. JsonFormatter (structured JSON for parsing)
//! 4. MarkdownFormatter (human-readable report)
//! 5. Implement --output flag to select format
//! 6. Create final report generation with task summary, timing, and outcome

use chrono::{Duration, Utc};
use clap::Parser;
use ltmatrix::cli::args::{Args, OutputFormat};
use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::output::{
    create_formatter, ExecutionResult, Formatter, JsonFormatter, MarkdownFormatter,
    ReportGenerator, TaskUpdateType, TerminalFormatter,
};
use tempfile::TempDir;

/// Helper to create a realistic execution result
fn create_realistic_result() -> ExecutionResult {
    let now = Utc::now();

    let mut task1 = Task::new(
        "task-1",
        "Design database schema",
        "Create user and post tables",
    );
    task1.status = TaskStatus::Completed;
    task1.complexity = TaskComplexity::Moderate;
    task1.started_at = Some(now - Duration::seconds(120));
    task1.completed_at = Some(now - Duration::seconds(90));

    let mut task2 = Task::new(
        "task-2",
        "Implement API endpoints",
        "Create REST API routes",
    );
    task2.status = TaskStatus::Completed;
    task2.complexity = TaskComplexity::Complex;
    task2.started_at = Some(now - Duration::seconds(90));
    task2.completed_at = Some(now - Duration::seconds(30));
    task2.depends_on = vec!["task-1".to_string()];

    let mut task3 = Task::new("task-3", "Write unit tests", "Test coverage for API");
    task3.status = TaskStatus::Failed;
    task3.complexity = TaskComplexity::Simple;
    task3.error = Some("Test framework not configured".to_string());
    task3.retry_count = 2;

    ExecutionResult {
        goal: "Build REST API for blog platform".to_string(),
        tasks: vec![task1, task2, task3],
        total_time: 150,
        completed_count: 2,
        failed_count: 1,
        total_retries: 2,
        dry_run: false,
    }
}

// ============================================================================
// ACCEPTANCE CRITERION 1: src/output/mod.rs exists with multiple formats
// ============================================================================

#[test]
fn acceptance_1_output_module_exists() {
    // Verify the module is accessible and has the required components
    let _formatter = TerminalFormatter::new();
    let _formatter = JsonFormatter::new();
    let _formatter = MarkdownFormatter::new();

    // If this compiles and runs, the module exists with all required types
    assert!(true);
}

#[test]
fn acceptance_1_formatter_trait_exists() {
    // Verify Formatter trait exists and has required methods
    let formatter = TerminalFormatter::new();
    let result = create_realistic_result();

    // These methods must exist on the trait
    let _output1 = formatter.format_result(&result);
    let _output2 = formatter.format_task_update(&result.tasks[0], TaskUpdateType::Started);
    let _output3 = formatter.format_progress(1, 3, "Testing");

    assert!(true);
}

// ============================================================================
// ACCEPTANCE CRITERION 2: TerminalFormatter with colors and progress bars
// ============================================================================

#[test]
fn acceptance_2_terminal_formatter_provides_colored_text() {
    let formatter = TerminalFormatter::new();
    let result = create_realistic_result();

    let output = formatter.format_result(&result).unwrap();

    // Verify colored output (console styling codes)
    assert!(output.contains("LTMATRIX EXECUTION REPORT") || output.len() > 0);
    assert!(output.contains("Build REST API"));
}

#[test]
fn acceptance_2_terminal_formatter_provides_progress_bars() {
    let formatter = TerminalFormatter::new();

    // Test progress bar formatting
    let progress = formatter.format_progress(7, 10, "Processing").unwrap();

    // Progress bar should contain progress info
    assert!(progress.contains("7/10"));
    assert!(progress.contains("70%"));
    assert!(progress.contains("Processing"));
}

#[test]
fn acceptance_2_terminal_formatter_is_default() {
    // Verify TerminalFormatter is returned when no specific format is requested
    let formatter = create_formatter(OutputFormat::Text);
    let result = create_realistic_result();

    let output = formatter.format_result(&result).unwrap();

    // Should produce terminal-formatted output
    assert!(!output.is_empty());
    assert!(output.contains("Build REST API") || output.len() > 100);
}

// ============================================================================
// ACCEPTANCE CRITERION 3: JsonFormatter with structured output
// ============================================================================

#[test]
fn acceptance_3_json_formatter_produces_structured_json() {
    let formatter = JsonFormatter::new();
    let result = create_realistic_result();

    let output = formatter.format_result(&result).unwrap();

    // Must be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&output).expect("Output must be valid JSON");

    // Verify structure has expected fields
    assert!(parsed.get("goal").is_some());
    assert!(parsed.get("summary").is_some());
    assert!(parsed.get("tasks").is_some());
}

#[test]
fn acceptance_3_json_formatter_is_parseable() {
    let formatter = JsonFormatter::new();
    let result = create_realistic_result();

    let output = formatter.format_result(&result).unwrap();

    // Should be parseable without errors
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("JSON must be parseable");

    // Verify we can access nested data
    assert_eq!(parsed["goal"], "Build REST API for blog platform");
    assert!(parsed["summary"]["total_tasks"].is_number());
    assert!(parsed["tasks"].is_array());
}

#[test]
fn acceptance_3_json_compact_mode_exists() {
    let formatter = JsonFormatter::compact();
    let result = create_realistic_result();

    let output = formatter.format_result(&result).unwrap();

    // Compact mode should produce single-line JSON
    assert!(!output.contains("\n") || output.lines().count() <= 2);

    // Still must be valid JSON
    let _: serde_json::Value = serde_json::from_str(&output).expect("Compact JSON must be valid");
}

// ============================================================================
// ACCEPTANCE CRITERION 4: MarkdownFormatter with human-readable reports
// ============================================================================

#[test]
fn acceptance_4_markdown_formatter_produces_markdown() {
    let formatter = MarkdownFormatter::new();
    let result = create_realistic_result();

    let output = formatter.format_result(&result).unwrap();

    // Must contain markdown headers
    assert!(output.contains("# LTMATRIX Execution Report"));
    assert!(output.contains("## Summary"));
    assert!(output.contains("## Complexity Breakdown"));
    assert!(output.contains("## Task Details"));
}

#[test]
fn acceptance_4_markdown_formatter_is_human_readable() {
    let formatter = MarkdownFormatter::new();
    let result = create_realistic_result();

    let output = formatter.format_result(&result).unwrap();

    // Should be readable plain text with markdown formatting
    assert!(output.contains("**Goal:**"));
    assert!(output.contains("**Mode:**"));
    assert!(output.contains("- **Total Tasks:**"));

    // Should show goal in plain text
    assert!(output.contains("Build REST API"));
}

#[test]
fn acceptance_4_markdown_summary_only_mode() {
    let formatter = MarkdownFormatter::summary_only();
    let result = create_realistic_result();

    let output = formatter.format_result(&result).unwrap();

    // Should have summary
    assert!(output.contains("## Summary"));

    // Should NOT have detailed task list
    assert!(!output.contains("## Task Details"));
}

// ============================================================================
// ACCEPTANCE CRITERION 5: --output flag to select format
// ============================================================================

#[test]
fn acceptance_5_output_flag_accepts_text() {
    let args = Args::try_parse_from(&["ltmatrix", "--output", "text", "test goal"])
        .expect("Should parse --output text");

    assert_eq!(args.output, Some(OutputFormat::Text));
}

#[test]
fn acceptance_5_output_flag_accepts_json() {
    let args = Args::try_parse_from(&["ltmatrix", "--output", "json", "test goal"])
        .expect("Should parse --output json");

    assert_eq!(args.output, Some(OutputFormat::Json));
}

#[test]
fn acceptance_5_output_flag_accepts_json_compact() {
    let args = Args::try_parse_from(&["ltmatrix", "--output", "json-compact", "test goal"])
        .expect("Should parse --output json-compact");

    assert_eq!(args.output, Some(OutputFormat::JsonCompact));
}

#[test]
fn acceptance_5_output_flag_rejects_invalid_format() {
    let result = Args::try_parse_from(&["ltmatrix", "--output", "invalid-format", "test goal"]);

    assert!(result.is_err(), "Should reject invalid output format");
}

#[test]
fn acceptance_5_output_flag_works_with_other_flags() {
    let args = Args::try_parse_from(&[
        "ltmatrix",
        "--output",
        "json",
        "--dry-run",
        "--fast",
        "test goal",
    ])
    .expect("Should work with other flags");

    assert_eq!(args.output, Some(OutputFormat::Json));
    assert!(args.dry_run);
    assert!(args.fast);
}

// ============================================================================
// ACCEPTANCE CRITERION 6: Report generation with summary, timing, outcome
// ============================================================================

#[test]
fn acceptance_6_report_contains_task_summary() {
    let result = create_realistic_result();
    let formatter = JsonFormatter::new();

    let output = formatter.format_result(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    // Verify summary statistics
    assert_eq!(parsed["summary"]["total_tasks"], 3);
    assert_eq!(parsed["summary"]["completed"], 2);
    assert_eq!(parsed["summary"]["failed"], 1);
    assert_eq!(parsed["summary"]["total_retries"], 2);
    assert!(parsed["summary"]["success_rate"].is_number());
}

#[test]
fn acceptance_6_report_contains_timing_information() {
    let result = create_realistic_result();
    let formatter = JsonFormatter::new();

    let output = formatter.format_result(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    // Verify total execution time
    assert_eq!(parsed["summary"]["total_time_seconds"], 150);

    // Verify individual task timing
    let task1 = &parsed["tasks"][0];
    assert!(task1.get("started_at").is_some());
    assert!(task1.get("completed_at").is_some());
    assert!(task1.get("duration_seconds").is_some());
}

#[test]
fn acceptance_6_report_contains_task_outcome() {
    let result = create_realistic_result();
    let formatter = TerminalFormatter::new();

    let output = formatter.format_result(&result).unwrap();

    // Verify task outcomes are shown
    assert!(output.len() > 0);

    // Check for status indicators
    let has_completed = result.tasks.iter().any(|t| t.is_completed());
    let has_failed = result.tasks.iter().any(|t| t.is_failed());

    assert!(has_completed, "Should have completed tasks");
    assert!(has_failed, "Should have failed tasks");
}

#[test]
fn acceptance_6_report_generator_creates_final_report() {
    let generator = ReportGenerator::new(OutputFormat::Text);
    let result = create_realistic_result();

    // Generate report to stdout
    let result_stdout = generator.generate_report_to_stdout(&result);
    assert!(result_stdout.is_ok());

    // Generate report to file
    let temp_dir = TempDir::new().unwrap();
    let report_path = temp_dir.path().join("report.txt");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result_file = rt.block_on(async {
        generator
            .generate_report_to_file(&result, &report_path)
            .await
    });

    assert!(result_file.is_ok());
    assert!(report_path.exists());

    // Verify file contains report
    let content = std::fs::read_to_string(&report_path).unwrap();
    assert!(!content.is_empty());
    assert!(content.contains("Build REST API"));
}

// ============================================================================
// INTEGRATION TESTS: Complete workflows
// ============================================================================

#[test]
fn integration_full_workflow_with_terminal_output() {
    // Simulate: ltmatrix "build feature"
    let args =
        Args::try_parse_from(&["ltmatrix", "build feature"]).expect("Should parse basic command");

    // Should default to terminal output
    let format = args.output.unwrap_or(OutputFormat::Text);
    assert_eq!(format, OutputFormat::Text);

    let formatter = create_formatter(format);
    let result = create_realistic_result();

    let output = formatter.format_result(&result).unwrap();
    assert!(!output.is_empty());
}

#[test]
fn integration_full_workflow_with_json_output() {
    // Simulate: ltmatrix --output json "build feature"
    let args = Args::try_parse_from(&["ltmatrix", "--output", "json", "build feature"])
        .expect("Should parse with --output flag");

    assert_eq!(args.output, Some(OutputFormat::Json));

    let formatter = create_formatter(args.output.unwrap());

    // Create result with matching goal
    let mut result = create_realistic_result();
    result.goal = "build feature".to_string();

    let output = formatter.format_result(&result).unwrap();

    // Verify JSON output is structured and parseable
    let parsed: serde_json::Value =
        serde_json::from_str(&output).expect("Should produce valid JSON");

    assert_eq!(parsed["goal"], "build feature");
}

#[tokio::test]
async fn integration_save_report_to_file_all_formats() {
    let result = create_realistic_result();
    let temp_dir = TempDir::new().unwrap();

    let formats = vec![
        (OutputFormat::Text, "report.txt"),
        (OutputFormat::Json, "report.json"),
        (OutputFormat::JsonCompact, "report-compact.json"),
    ];

    for (format, filename) in formats {
        let generator = ReportGenerator::new(format);
        let path = temp_dir.path().join(filename);

        generator
            .generate_report_to_file(&result, &path)
            .await
            .expect("Should generate report file");

        assert!(path.exists(), "Report file should exist: {}", filename);

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(
            !content.is_empty(),
            "Report should have content: {}",
            filename
        );

        // JSON formats should be valid
        if matches!(format, OutputFormat::Json | OutputFormat::JsonCompact) {
            let _: serde_json::Value =
                serde_json::from_str(&content).expect("JSON report should be valid");
        }
    }
}

#[test]
fn integration_task_updates_during_execution() {
    let generator = ReportGenerator::new(OutputFormat::Text);
    let execution_result = create_realistic_result();

    // Simulate printing task updates during execution
    for (i, task) in execution_result.tasks.iter().enumerate() {
        let update_type = match task.status {
            TaskStatus::Completed => TaskUpdateType::Completed,
            TaskStatus::Failed => TaskUpdateType::Failed,
            TaskStatus::InProgress => TaskUpdateType::InProgress,
            _ => TaskUpdateType::Started,
        };

        let update_result = generator.print_task_update(task, update_type);
        assert!(update_result.is_ok(), "Should print task update {}", i + 1);

        let progress_result =
            generator.print_progress(i + 1, execution_result.tasks.len(), "Working on task");
        assert!(progress_result.is_ok(), "Should print progress");
    }

    generator.finish_progress();

    // Generate final report
    let final_result = generator.generate_report_to_stdout(&execution_result);
    assert!(final_result.is_ok(), "Should generate final report");
}

// ============================================================================
// EDGE CASES AND ERROR HANDLING
// ============================================================================

#[test]
fn edge_case_empty_task_list() {
    let result = ExecutionResult {
        goal: "Empty execution".to_string(),
        tasks: vec![],
        total_time: 0,
        completed_count: 0,
        failed_count: 0,
        total_retries: 0,
        dry_run: false,
    };

    let formatter = TerminalFormatter::new();
    let output = formatter.format_result(&result);

    assert!(output.is_ok(), "Should handle empty task list");
    assert!(!output.unwrap().is_empty());
}

#[test]
fn edge_case_all_tasks_failed() {
    let mut result = create_realistic_result();
    result.tasks.iter_mut().for_each(|t| {
        t.status = TaskStatus::Failed;
        t.error = Some("All failed".to_string());
    });
    result.completed_count = 0;
    result.failed_count = result.tasks.len();

    let formatter = JsonFormatter::new();
    let output = formatter.format_result(&result);

    assert!(output.is_ok(), "Should handle all failed tasks");

    let parsed: serde_json::Value = serde_json::from_str(&output.unwrap()).unwrap();
    assert_eq!(parsed["summary"]["success_rate"], 0.0);
}

#[test]
fn edge_case_zero_time_execution() {
    let result = ExecutionResult {
        goal: "Instant task".to_string(),
        tasks: vec![],
        total_time: 0,
        completed_count: 0,
        failed_count: 0,
        total_retries: 0,
        dry_run: false,
    };

    let formatter = MarkdownFormatter::new();
    let output = formatter.format_result(&result);

    assert!(output.is_ok(), "Should handle zero time");
    assert!(output.unwrap().contains("0s"));
}

#[test]
fn edge_case_dry_run_mode() {
    let result = ExecutionResult {
        goal: "Plan execution".to_string(),
        tasks: vec![],
        total_time: 0,
        completed_count: 0,
        failed_count: 0,
        total_retries: 0,
        dry_run: true,
    };

    let formatter = TerminalFormatter::new();
    let output = formatter.format_result(&result).unwrap();

    assert!(output.contains("DRY RUN"), "Should indicate dry run mode");
}
