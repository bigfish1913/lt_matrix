//! Example usage of output formatters
//!
//! This example demonstrates how to use the different output formatters
//! provided by ltmatrix.

use ltmatrix::cli::args::OutputFormat;
use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::output::{ExecutionResult, Formatter, JsonFormatter, MarkdownFormatter, ReportGenerator, TerminalFormatter, TaskUpdateType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Output Formatter Examples ===\n");

    // Create some sample tasks
    let mut task1 = Task::new("task-1", "Implement user model", "Create user data model");
    task1.complexity = TaskComplexity::Simple;
    task1.status = TaskStatus::Completed;

    let mut task2 = Task::new("task-2", "Create API endpoints", "Build REST API");
    task2.complexity = TaskComplexity::Moderate;
    task2.status = TaskStatus::Completed;

    let mut task3 = Task::new("task-3", "Write tests", "Add comprehensive tests");
    task3.complexity = TaskComplexity::Simple;
    task3.status = TaskStatus::Failed;
    task3.error = Some("Test framework not found".to_string());

    // Create execution result
    let result = ExecutionResult {
        goal: "Build user authentication system".to_string(),
        tasks: vec![task1, task2, task3],
        total_time: 180,
        completed_count: 2,
        failed_count: 1,
        total_retries: 1,
        dry_run: false,
    };

    // Terminal formatter
    println!("--- Terminal Formatter ---");
    let terminal_formatter = TerminalFormatter::new();
    let terminal_output = terminal_formatter.format_result(&result)?;
    println!("{}", terminal_output);

    // JSON formatter
    println!("\n--- JSON Formatter ---");
    let json_formatter = JsonFormatter::new();
    let json_output = json_formatter.format_result(&result)?;
    println!("{}", json_output);

    // Markdown formatter
    println!("\n--- Markdown Formatter ---");
    let markdown_formatter = MarkdownFormatter::new();
    let markdown_output = markdown_formatter.format_result(&result)?;
    println!("{}", markdown_output);

    // Task updates
    println!("\n--- Task Updates ---");
    let formatter = TerminalFormatter::new();
    let sample_task = &result.tasks[0];
    println!("{}", formatter.format_task_update(sample_task, TaskUpdateType::Started)?);
    println!("{}", formatter.format_task_update(sample_task, TaskUpdateType::Completed)?);

    // Progress updates
    println!("\n--- Progress Updates ---");
    println!("{}", formatter.format_progress(1, 5, "Initializing...")?);
    println!();
    println!("{}", formatter.format_progress(2, 5, "Processing...")?);
    println!();
    println!("{}", formatter.format_progress(3, 5, "Almost there...")?);
    println!();

    // Report generator
    println!("\n--- Report Generator ---");
    let generator = ReportGenerator::new(OutputFormat::Text);

    // Print a task update
    generator.print_task_update(&result.tasks[1], TaskUpdateType::Started)?;

    // Print some progress
    generator.print_progress(4, 5, "Finalizing...")?;
    generator.finish_progress();

    Ok(())
}
