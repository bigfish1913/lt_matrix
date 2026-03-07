//! Custom formatters for structured logging
//!
//! This module provides formatters for console and file output with
//! support for colors, timestamps, and structured formatting.

use crate::terminal::{self, ColorConfig};
use chrono::{DateTime, Local};
use tracing::field::{Field, Visit};
use tracing::Event;

/// Timestamp format used in logs
pub const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

/// Global color configuration for logging
///
/// # Safety
///
/// This static is only written to once during initialization via `init_color_config`
/// and only read afterwards via `get_color_config`. The write-once, read-many pattern
/// ensures no data races occur.
static mut COLOR_CONFIG: Option<ColorConfig> = None;

/// Initialize the color configuration for logging formatters
///
/// # Safety
///
/// This function should only be called once during application initialization.
/// It uses unsafe mutable static to share the color config with formatters.
pub fn init_color_config(config: ColorConfig) {
    unsafe {
        COLOR_CONFIG = Some(config);
    }
}

/// Get the current color configuration
///
/// # Safety
///
/// Returns ColorConfig::auto() if not initialized
fn get_color_config() -> ColorConfig {
    unsafe { COLOR_CONFIG.unwrap_or_else(|| ColorConfig::auto()) }
}

/// Formats a log level with ANSI color codes
pub fn format_level(level: &tracing::Level) -> String {
    let config = get_color_config();
    terminal::colorize_log_level(level.as_str(), config)
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
    let config = get_color_config();

    let module = metadata
        .module_path()
        .unwrap_or("ltmatrix")
        .replace("ltmatrix::", "");
    let module_colored = terminal::style_text(&module, terminal::Color::Cyan, config);

    let timestamp_colored = terminal::dim(&timestamp, config);
    let message = format_message(event);

    format!(
        "{} {} [{}] {}",
        timestamp_colored, level, module_colored, message
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
        assert!(formatted.chars().all(|c| c.is_ascii_digit()
            || c == ' '
            || c == '-'
            || c == ':'
            || c == '.'));
    }

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts.len() > 0);
        assert!(ts.contains(' ')); // Date and time separator
    }

    #[test]
    fn test_terminal_style_text() {
        let config = ColorConfig::plain();
        let styled = terminal::style_text("test", terminal::Color::Green, config);
        // With plain config, should just return the text
        assert_eq!(styled, "test");
    }

    #[test]
    fn test_format_level() {
        let info = format_level(&tracing::Level::INFO);
        assert!(info.contains("INFO"));
        // ANSI codes may not be added on Windows or without a terminal
        // Just verify it contains the level text
    }
}
