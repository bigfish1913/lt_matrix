//! Output formatter plugin trait
//!
//! This module defines the trait for custom output formatters.

use anyhow::Result;

use super::Plugin;
use crate::models::Task;

/// Trait for output formatter plugins
pub trait FormatterPlugin: Plugin {
    /// Format a single task
    fn format_task(&self, task: &Task) -> Result<String>;

    /// Format a list of tasks
    fn format_tasks(&self, tasks: &[Task]) -> Result<String>;

    /// Get the formatter name (e.g., "json", "text", "markdown")
    fn formatter_name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    // Tests would go here
}