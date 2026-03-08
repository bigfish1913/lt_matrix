// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Output formatting and reporting
//!
//! This module provides multiple output formats for ltmatrix results:
//! - TerminalFormatter: Beautiful colored terminal output with progress bars
//! - JsonFormatter: Structured JSON for programmatic parsing
//! - MarkdownFormatter: Human-readable markdown reports

use anyhow::Result;
use console;
use serde_json::json;
use std::io::Write;

use crate::cli::args::OutputFormat;
use ltmatrix_core::{Task, TaskComplexity, TaskStatus};

/// Result data that can be formatted
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// The goal that was executed
    pub goal: String,

    /// Tasks that were executed
    pub tasks: Vec<Task>,

    /// Total execution time in seconds
    pub total_time: u64,

    /// Number of tasks that completed successfully
    pub completed_count: usize,

    /// Number of tasks that failed
    pub failed_count: usize,

    /// Number of retries attempted
    pub total_retries: u32,

    /// Whether this was a dry run
    pub dry_run: bool,
}

/// Trait for output formatters
pub trait Formatter {
    /// Format the execution result and output it
    fn format_result(&self, result: &ExecutionResult) -> Result<String>;

    /// Format a single task status update
    fn format_task_update(&self, task: &Task, update_type: TaskUpdateType) -> Result<String>;

    /// Format progress information
    fn format_progress(&self, current: usize, total: usize, message: &str) -> Result<String>;
}

/// Types of task updates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskUpdateType {
    Started,
    InProgress,
    Completed,
    Failed,
    Retrying,
}

/// Terminal formatter with colors and progress bars
pub struct TerminalFormatter {
    /// Whether to use colors
    use_colors: bool,

    /// Whether to show progress bars
    show_progress: bool,
}

impl TerminalFormatter {
    /// Create a new terminal formatter
    pub fn new() -> Self {
        TerminalFormatter {
            use_colors: true,
            show_progress: true,
        }
    }

    /// Create a formatter without colors
    pub fn no_colors() -> Self {
        TerminalFormatter {
            use_colors: true,
            show_progress: true,
        }
    }

    /// Create a formatter without progress bars
    pub fn no_progress() -> Self {
        TerminalFormatter {
            use_colors: true,
            show_progress: false,
        }
    }

    /// Format task status with color
    fn format_status(&self, status: &TaskStatus) -> String {
        if !self.use_colors {
            return format!("{:?}", status);
        }

        match status {
            TaskStatus::Pending => console::style("Pending").dim().to_string(),
            TaskStatus::InProgress => console::style("In Progress").yellow().to_string(),
            TaskStatus::Completed => console::style("Completed").green().to_string(),
            TaskStatus::Failed => console::style("Failed").red().to_string(),
            TaskStatus::Blocked => console::style("Blocked").bright().red().to_string(),
        }
    }

    /// Format complexity with color
    fn format_complexity(&self, complexity: &TaskComplexity) -> String {
        if !self.use_colors {
            return format!("{:?}", complexity);
        }

        match complexity {
            TaskComplexity::Simple => console::style("Simple").cyan().to_string(),
            TaskComplexity::Moderate => console::style("Moderate").yellow().to_string(),
            TaskComplexity::Complex => console::style("Complex").red().to_string(),
        }
    }
}

impl Default for TerminalFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for TerminalFormatter {
    fn format_result(&self, result: &ExecutionResult) -> Result<String> {
        let mut output = String::new();

        // Header
        if self.use_colors {
            output.push_str(&format!(
                "{}\n",
                console::style("╔═══════════════════════════════════════════════════════════════╗")
                    .bold()
            ));
            output.push_str(&format!(
                "{}\n",
                console::style("║              LTMATRIX EXECUTION REPORT                        ║")
                    .bold()
            ));
            output.push_str(&format!(
                "{}\n",
                console::style("╚═══════════════════════════════════════════════════════════════╝")
                    .bold()
            ));
        } else {
            output.push_str("LTMATRIX EXECUTION REPORT\n");
            output.push_str(&str::repeat("=", 70));
            output.push('\n');
        }
        output.push('\n');

        // Goal
        output.push_str(&format!("Goal: {}\n", result.goal));
        if result.dry_run {
            output.push_str(
                &console::style("Mode: DRY RUN (no changes made)")
                    .yellow()
                    .to_string(),
            );
        } else {
            output.push_str(&console::style("Mode: EXECUTION").green().to_string());
        }
        output.push('\n');
        output.push('\n');

        // Summary statistics
        if self.use_colors {
            output.push_str(&console::style("SUMMARY\n").bold().to_string());
        } else {
            output.push_str("SUMMARY\n");
        }
        output.push_str(&format!("  Total Tasks: {}\n", result.tasks.len()));
        output.push_str(&format!("  Completed: {}\n", result.completed_count));
        output.push_str(&format!("  Failed: {}\n", result.failed_count));
        output.push_str(&format!("  Total Retries: {}\n", result.total_retries));
        output.push_str(&format!("  Total Time: {}s\n", result.total_time));

        let success_rate = if result.tasks.is_empty() {
            0.0
        } else {
            (result.completed_count as f64 / result.tasks.len() as f64) * 100.0
        };
        output.push_str(&format!("  Success Rate: {:.1}%\n", success_rate));
        output.push('\n');

        // Task breakdown by complexity
        let (simple, moderate, complex) =
            result
                .tasks
                .iter()
                .fold((0, 0, 0), |(s, m, c), task| match task.complexity {
                    TaskComplexity::Simple => (s + 1, m, c),
                    TaskComplexity::Moderate => (s, m + 1, c),
                    TaskComplexity::Complex => (s, m, c + 1),
                });

        if self.use_colors {
            output.push_str(&console::style("COMPLEXITY BREAKDOWN\n").bold().to_string());
        } else {
            output.push_str("COMPLEXITY BREAKDOWN\n");
        }
        output.push_str(&format!(
            "  Simple: {} {}\n",
            simple,
            self.format_complexity(&TaskComplexity::Simple)
        ));
        output.push_str(&format!(
            "  Moderate: {} {}\n",
            moderate,
            self.format_complexity(&TaskComplexity::Moderate)
        ));
        output.push_str(&format!(
            "  Complex: {} {}\n",
            complex,
            self.format_complexity(&TaskComplexity::Complex)
        ));
        output.push('\n');

        // Task details
        if self.use_colors {
            output.push_str(&console::style("TASK DETAILS\n").bold().to_string());
        } else {
            output.push_str("TASK DETAILS\n");
        }

        for (i, task) in result.tasks.iter().enumerate() {
            output.push_str(&format!("  {}. {} ({})\n", i + 1, task.id, task.title));
            output.push_str(&format!(
                "     Status: {}\n",
                self.format_status(&task.status)
            ));
            output.push_str(&format!(
                "     Complexity: {}\n",
                self.format_complexity(&task.complexity)
            ));

            if !task.depends_on.is_empty() {
                output.push_str(&format!(
                    "     Dependencies: {}\n",
                    task.depends_on.join(", ")
                ));
            }

            if !task.subtasks.is_empty() {
                output.push_str(&format!("     Subtasks: {}\n", task.subtasks.len()));
            }

            if task.retry_count > 0 {
                output.push_str(&format!("     Retries: {}\n", task.retry_count));
            }

            if let Some(error) = &task.error {
                output.push_str(&format!("     Error: {}\n", console::style(error).red()));
            }

            // Timing information
            if let Some(started) = task.started_at {
                if let Some(completed) = task.completed_at {
                    let duration = completed.signed_duration_since(started);
                    output.push_str(&format!("     Duration: {}s\n", duration.num_seconds()));
                }
            }

            output.push('\n');
        }

        Ok(output)
    }

    fn format_task_update(&self, task: &Task, update_type: TaskUpdateType) -> Result<String> {
        let status_str = match update_type {
            TaskUpdateType::Started => format!("▶ {} started", console::style(&task.id).cyan()),
            TaskUpdateType::InProgress => {
                format!("↻ {} in progress", console::style(&task.id).yellow())
            }
            TaskUpdateType::Completed => {
                format!("✓ {} completed", console::style(&task.id).green())
            }
            TaskUpdateType::Failed => format!("✗ {} failed", console::style(&task.id).red()),
            TaskUpdateType::Retrying => format!(
                "⟳ {} retrying (attempt {})",
                console::style(&task.id).yellow(),
                task.retry_count + 1
            ),
        };

        Ok(format!(
            "{} {}",
            status_str,
            console::style(format!("- {}", task.title)).dim()
        ))
    }

    fn format_progress(&self, current: usize, total: usize, message: &str) -> Result<String> {
        if !self.show_progress {
            return Ok(format!("[{}/{}] {}", current, total, message));
        }

        let percentage = if total > 0 {
            (current as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let bar_width = 40;
        let filled = (percentage / 100.0 * bar_width as f64) as usize;
        let filled_str = "█".repeat(filled);
        let empty_str = "░".repeat(bar_width - filled);

        Ok(format!(
            "[{}] {}% [{}/{}] {}",
            console::style(format!("{}{}", filled_str, empty_str)).green(),
            console::style(percentage as usize).bold(),
            console::style(current).cyan(),
            console::style(total).dim(),
            message
        ))
    }
}

/// JSON formatter for structured output
pub struct JsonFormatter {
    /// Whether to use compact formatting
    compact: bool,
}

impl JsonFormatter {
    /// Create a new JSON formatter with pretty printing
    pub fn new() -> Self {
        JsonFormatter { compact: false }
    }

    /// Create a compact JSON formatter
    pub fn compact() -> Self {
        JsonFormatter { compact: true }
    }
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for JsonFormatter {
    fn format_result(&self, result: &ExecutionResult) -> Result<String> {
        let output = json!({
            "goal": result.goal,
            "mode": if result.dry_run { "dry_run" } else { "execution" },
            "summary": {
                "total_tasks": result.tasks.len(),
                "completed": result.completed_count,
                "failed": result.failed_count,
                "total_retries": result.total_retries,
                "total_time_seconds": result.total_time,
                "success_rate": if result.tasks.is_empty() {
                    0.0
                } else {
                    (result.completed_count as f64 / result.tasks.len() as f64) * 100.0
                }
            },
            "complexity_breakdown": {
                "simple": result.tasks.iter().filter(|t| t.complexity == TaskComplexity::Simple).count(),
                "moderate": result.tasks.iter().filter(|t| t.complexity == TaskComplexity::Moderate).count(),
                "complex": result.tasks.iter().filter(|t| t.complexity == TaskComplexity::Complex).count(),
            },
            "tasks": result.tasks.iter().map(|task| {
                let mut task_obj = json!({
                    "id": task.id,
                    "title": task.title,
                    "description": task.description,
                    "status": format!("{:?}", task.status),
                    "complexity": format!("{:?}", task.complexity),
                    "depends_on": task.depends_on,
                    "subtasks_count": task.subtasks.len(),
                    "retry_count": task.retry_count,
                });

                if let Some(error) = &task.error {
                    task_obj["error"] = json!(error);
                }

                if let Some(started) = task.started_at {
                    task_obj["started_at"] = json!(started.to_rfc3339());
                }

                if let Some(completed) = task.completed_at {
                    task_obj["completed_at"] = json!(completed.to_rfc3339());
                    if let Some(started) = task.started_at {
                        let duration = completed.signed_duration_since(started);
                        task_obj["duration_seconds"] = json!(duration.num_seconds());
                    }
                }

                task_obj
            }).collect::<Vec<_>>()
        });

        if self.compact {
            Ok(serde_json::to_string(&output)?)
        } else {
            Ok(serde_json::to_string_pretty(&output)?)
        }
    }

    fn format_task_update(&self, task: &Task, update_type: TaskUpdateType) -> Result<String> {
        let update = json!({
            "type": format!("{:?}", update_type),
            "task": {
                "id": task.id,
                "title": task.title,
                "status": format!("{:?}", task.status),
                "retry_count": task.retry_count,
            }
        });

        if self.compact {
            Ok(serde_json::to_string(&update)?)
        } else {
            Ok(serde_json::to_string_pretty(&update)?)
        }
    }

    fn format_progress(&self, current: usize, total: usize, message: &str) -> Result<String> {
        let progress = json!({
            "current": current,
            "total": total,
            "percentage": if total > 0 {
                (current as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            "message": message
        });

        if self.compact {
            Ok(serde_json::to_string(&progress)?)
        } else {
            Ok(serde_json::to_string_pretty(&progress)?)
        }
    }
}

/// Markdown formatter for human-readable reports
pub struct MarkdownFormatter {
    /// Whether to include detailed task information
    detailed: bool,
}

impl MarkdownFormatter {
    /// Create a new markdown formatter
    pub fn new() -> Self {
        MarkdownFormatter { detailed: true }
    }

    /// Create a summary-only formatter
    pub fn summary_only() -> Self {
        MarkdownFormatter { detailed: false }
    }
}

impl Default for MarkdownFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for MarkdownFormatter {
    fn format_result(&self, result: &ExecutionResult) -> Result<String> {
        let mut output = String::new();

        // Title and metadata
        output.push_str("# LTMATRIX Execution Report\n\n");
        output.push_str(&format!("**Goal:** {}\n\n", result.goal));
        output.push_str(&format!(
            "**Mode:** {}\n\n",
            if result.dry_run {
                "Dry Run"
            } else {
                "Execution"
            }
        ));

        // Summary section
        output.push_str("## Summary\n\n");
        output.push_str(&format!("- **Total Tasks:** {}\n", result.tasks.len()));
        output.push_str(&format!("- **Completed:** {}\n", result.completed_count));
        output.push_str(&format!("- **Failed:** {}\n", result.failed_count));
        output.push_str(&format!("- **Total Retries:** {}\n", result.total_retries));
        output.push_str(&format!("- **Total Time:** {}s\n", result.total_time));

        let success_rate = if result.tasks.is_empty() {
            0.0
        } else {
            (result.completed_count as f64 / result.tasks.len() as f64) * 100.0
        };
        output.push_str(&format!("- **Success Rate:** {:.1}%\n\n", success_rate));

        // Complexity breakdown
        let (simple, moderate, complex) =
            result
                .tasks
                .iter()
                .fold((0, 0, 0), |(s, m, c), task| match task.complexity {
                    TaskComplexity::Simple => (s + 1, m, c),
                    TaskComplexity::Moderate => (s, m + 1, c),
                    TaskComplexity::Complex => (s, m, c + 1),
                });

        output.push_str("## Complexity Breakdown\n\n");
        output.push_str(&format!("- **Simple:** {} tasks\n", simple));
        output.push_str(&format!("- **Moderate:** {} tasks\n", moderate));
        output.push_str(&format!("- **Complex:** {} tasks\n\n", complex));

        // Task details
        if self.detailed {
            output.push_str("## Task Details\n\n");

            for (i, task) in result.tasks.iter().enumerate() {
                output.push_str(&format!("### {}. {}\n\n", i + 1, task.id));
                output.push_str(&format!("**Title:** {}\n\n", task.title));
                output.push_str(&format!("**Description:** {}\n\n", task.description));
                output.push_str(&format!("**Status:** {:?}\n\n", task.status));
                output.push_str(&format!("**Complexity:** {:?}\n\n", task.complexity));

                if !task.depends_on.is_empty() {
                    output.push_str(&format!(
                        "**Dependencies:** {}\n\n",
                        task.depends_on.join(", ")
                    ));
                }

                if !task.subtasks.is_empty() {
                    output.push_str(&format!("**Subtasks:** {} items\n\n", task.subtasks.len()));
                    for (j, subtask) in task.subtasks.iter().enumerate() {
                        output.push_str(&format!(
                            "  {}. {} - {:?}\n",
                            j + 1,
                            subtask.id,
                            subtask.status
                        ));
                    }
                    output.push('\n');
                }

                if task.retry_count > 0 {
                    output.push_str(&format!("**Retries:** {}\n\n", task.retry_count));
                }

                if let Some(error) = &task.error {
                    output.push_str(&format!("**Error:** {}\n\n", error));
                }

                // Timing
                if let Some(started) = task.started_at {
                    if let Some(completed) = task.completed_at {
                        let duration = completed.signed_duration_since(started);
                        output.push_str(&format!("**Duration:** {}s\n\n", duration.num_seconds()));
                    }
                }

                output.push_str("---\n\n");
            }
        }

        // Footer
        output.push_str(&format!(
            "*Generated by ltmatrix on {}*\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        Ok(output)
    }

    fn format_task_update(&self, task: &Task, update_type: TaskUpdateType) -> Result<String> {
        let icon = match update_type {
            TaskUpdateType::Started => "▶",
            TaskUpdateType::InProgress => "↻",
            TaskUpdateType::Completed => "✓",
            TaskUpdateType::Failed => "✗",
            TaskUpdateType::Retrying => "⟳",
        };

        Ok(format!(
            "{} **{}** - {} ({:?})",
            icon, task.id, task.title, task.status
        ))
    }

    fn format_progress(&self, current: usize, total: usize, message: &str) -> Result<String> {
        let percentage = if total > 0 {
            (current as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Ok(format!(
            "**Progress:** [{}/{}] {:.0}% - {}",
            current, total, percentage, message
        ))
    }
}

/// Create a formatter based on output format
pub fn create_formatter(format: OutputFormat) -> Box<dyn Formatter> {
    match format {
        OutputFormat::Text => Box::new(TerminalFormatter::new()),
        OutputFormat::Json => Box::new(JsonFormatter::new()),
        OutputFormat::JsonCompact => Box::new(JsonFormatter::compact()),
    }
}

/// Report generator for final reports
pub struct ReportGenerator {
    /// Formatter to use for output
    formatter: Box<dyn Formatter>,
}

impl ReportGenerator {
    /// Create a new report generator with the specified format
    pub fn new(format: OutputFormat) -> Self {
        ReportGenerator {
            formatter: create_formatter(format),
        }
    }

    /// Generate and write a report to a file
    pub async fn generate_report_to_file(
        &self,
        result: &ExecutionResult,
        path: &std::path::Path,
    ) -> Result<()> {
        let content = self.formatter.format_result(result)?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Generate a report to stdout
    pub fn generate_report_to_stdout(&self, result: &ExecutionResult) -> Result<()> {
        let content = self.formatter.format_result(result)?;
        print!("{}", content);
        Ok(())
    }

    /// Generate and print a task status update
    pub fn print_task_update(&self, task: &Task, update_type: TaskUpdateType) -> Result<()> {
        let content = self.formatter.format_task_update(task, update_type)?;
        println!("{}", content);
        Ok(())
    }

    /// Generate and print progress information
    pub fn print_progress(&self, current: usize, total: usize, message: &str) -> Result<()> {
        let content = self.formatter.format_progress(current, total, message)?;

        // Print with carriage return to update in-place
        print!("\r{}", content);
        std::io::stdout().flush()?;

        Ok(())
    }

    /// Print a newline after progress updates
    pub fn finish_progress(&self) {
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ltmatrix_core::TaskComplexity;

    #[test]
    fn test_terminal_formatter_basic() {
        let formatter = TerminalFormatter::new();
        let result = create_test_result();

        let output = formatter.format_result(&result).unwrap();
        assert!(output.contains("LTMATRIX EXECUTION REPORT"));
        assert!(output.contains("SUMMARY"));
        assert!(output.contains("Test Goal"));
    }

    #[test]
    fn test_json_formatter() {
        let formatter = JsonFormatter::new();
        let result = create_test_result();

        let output = formatter.format_result(&result).unwrap();
        assert!(output.contains("\"goal\""));
        assert!(output.contains("\"summary\""));
        assert!(output.contains("\"tasks\""));

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["goal"], "Test Goal");
    }

    #[test]
    fn test_json_compact_formatter() {
        let formatter = JsonFormatter::compact();
        let result = create_test_result();

        let output = formatter.format_result(&result).unwrap();

        // Compact should not have newlines or extra spaces
        assert!(!output.contains("\n"));

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["goal"], "Test Goal");
    }

    #[test]
    fn test_markdown_formatter() {
        let formatter = MarkdownFormatter::new();
        let result = create_test_result();

        let output = formatter.format_result(&result).unwrap();
        assert!(output.contains("# LTMATRIX Execution Report"));
        assert!(output.contains("## Summary"));
        assert!(output.contains("## Complexity Breakdown"));
        assert!(output.contains("## Task Details"));
    }

    #[test]
    fn test_markdown_summary_only() {
        let formatter = MarkdownFormatter::summary_only();
        let result = create_test_result();

        let output = formatter.format_result(&result).unwrap();
        assert!(output.contains("## Summary"));
        assert!(!output.contains("## Task Details"));
    }

    #[test]
    fn test_task_update_formatting() {
        let formatter = TerminalFormatter::new();
        let task = Task::new("task-1", "Test Task", "A test task");

        let started = formatter
            .format_task_update(&task, TaskUpdateType::Started)
            .unwrap();
        assert!(started.contains("started"));

        let completed = formatter
            .format_task_update(&task, TaskUpdateType::Completed)
            .unwrap();
        assert!(completed.contains("completed"));
    }

    #[test]
    fn test_progress_formatting() {
        let formatter = TerminalFormatter::new();
        let progress = formatter.format_progress(5, 10, "Halfway there").unwrap();

        assert!(progress.contains("5/10"));
        assert!(progress.contains("50%"));
        assert!(progress.contains("Halfway there"));
    }

    #[test]
    fn test_report_generator_stdout() {
        let generator = ReportGenerator::new(OutputFormat::Text);
        let result = create_test_result();

        // Should not panic
        generator.generate_report_to_stdout(&result).unwrap();
    }

    fn create_test_result() -> ExecutionResult {
        let mut task = Task::new("task-1", "Test Task", "A test task");
        task.complexity = TaskComplexity::Simple;
        task.status = TaskStatus::Completed;

        ExecutionResult {
            goal: "Test Goal".to_string(),
            tasks: vec![task],
            total_time: 60,
            completed_count: 1,
            failed_count: 0,
            total_retries: 0,
            dry_run: false,
        }
    }
}
