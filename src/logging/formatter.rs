//! Custom formatters for structured logging
//!
//! This module provides formatters for console and file output with
//! support for colors, timestamps, and structured formatting.

use tracing_subscriber::fmt::format::Writer;
use std::fmt::Write;
use chrono::{DateTime, Local};
use tracing::Event;
use tracing::field::{Field, Visit};

/// Timestamp format used in logs
pub const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

/// Formats a log level with ANSI color codes
pub fn format_level(level: &tracing::Level) -> String {
    let (level_str, color) = match level {
        &tracing::Level::TRACE => ("TRACE", console::Color::White),
        &tracing::Level::DEBUG => ("DEBUG", console::Color::Blue),
        &tracing::Level::INFO => ("INFO", console::Color::Green),
        &tracing::Level::WARN => ("WARN", console::Color::Yellow),
        &tracing::Level::ERROR => ("ERROR", console::Color::Red),
    };

    console_style(level_str.to_string(), color, true)
}

/// Applies ANSI color styling to text
fn console_style(text: String, color: console::Color, bright: bool) -> String {
    let style = if bright {
        console::Style::new().fg(color).bright()
    } else {
        console::Style::new().fg(color).dim()
    };

    style.apply_to(text).to_string()
}

/// Extracts and formats the message from an event
pub fn format_message(event: &Event<'_>) -> String {
    let mut message = String::new();
    let mut visitor = MessageVisitor(String::new());
    event.record(&mut visitor);
    message.push_str(&visitor.0);
    message
}

/// Formats a timestamp in the standard log format
#[must_use]
pub fn format_timestamp(dt: DateTime<Local>) -> String {
    dt.format(TIMESTAMP_FORMAT).to_string()
}

/// Returns the current timestamp as a formatted string
#[must_use]
pub fn current_timestamp() -> String {
    format_timestamp(Local::now())
}

/// Formats a complete log line for console output
pub fn format_console_line(event: &Event<'_>) -> String {
    let timestamp = current_timestamp();
    let metadata = event.metadata();
    let level = format_level(metadata.level());
    let module = metadata
        .module_path()
        .unwrap_or("ltmatrix")
        .replace("ltmatrix::", "");
    let message = format_message(event);

    format!(
        "{} {} [{}] {}",
        console_style(timestamp, console::Color::Black, true),
        level,
        console_style(module, console::Color::Cyan, false),
        message
    )
}

/// Formats a complete log line for file output (no colors)
pub fn format_file_line(event: &Event<'_>) -> String {
    let timestamp = current_timestamp();
    let metadata = event.metadata();
    let level = metadata.level().as_str();
    let module = metadata
        .module_path()
        .unwrap_or("ltmatrix")
        .replace("ltmatrix::", "");
    let message = format_message(event);

    format!("{} {:>5} [{}] {}", timestamp, level, module, message)
}

/// Visitor for extracting field values from events
struct MessageVisitor(String);

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{:?}", value);
        } else if !self.0.is_empty() {
            // Append additional fields
            self.0.push_str(&format!(" {}={:?}", field.name(), value));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        } else if !self.0.is_empty() {
            self.0.push_str(&format!(" {}={}", field.name(), value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        let dt = Local::now();
        let formatted = format_timestamp(dt);
        assert!(formatted.len() > 0);
        // Should match pattern: YYYY-MM-DD HH:MM:SS.mmm
        assert!(formatted.chars().all(|c| c.is_ascii_digit() || c == ' ' || c == '-' || c == ':' || c == '.'));
    }

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts.len() > 0);
        assert!(ts.contains(' ')); // Date and time separator
    }

    #[test]
    fn test_console_style() {
        let styled = console_style("test".to_string(), console::Color::Green, true);
        assert!(styled.len() > 4); // Should have ANSI codes
    }

    #[test]
    fn test_format_level() {
        let info = format_level(&tracing::Level::INFO);
        assert!(info.contains("INFO"));
        assert!(info.len() > 4); // Should have ANSI codes
    }
}
