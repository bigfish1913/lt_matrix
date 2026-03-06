//! Terminal styling and color management
//!
//! This module provides terminal-aware color and formatting support with:
//! - Automatic terminal capability detection
//! - NO_COLOR environment variable support
//! - Per-color styling for task statuses, log levels, and progress indicators
//! - Cross-platform color support via the `console` crate

use std::env;
use std::io::IsTerminal;

/// Global color configuration
///
/// This struct manages whether colors should be output based on:
/// - Terminal capability detection
/// - NO_COLOR environment variable
/// - Explicit --no-color flag
#[derive(Debug, Clone, Copy)]
pub struct ColorConfig {
    /// Whether colors are enabled
    pub enabled: bool,
}

impl ColorConfig {
    /// Creates a new ColorConfig by auto-detecting terminal capabilities
    ///
    /// This checks:
    /// 1. NO_COLOR environment variable (https://no-color.org/)
    /// 2. Whether stdout is a terminal
    /// 3. Defaults to enabled if both checks pass
    #[must_use]
    pub fn auto() -> Self {
        let enabled = Self::check_no_color().is_none() && std::io::stdout().is_terminal();
        ColorConfig { enabled }
    }

    /// Creates a ColorConfig with explicit color control
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to force colors on/off
    /// * `check_no_color` - If true, still checks NO_COLOR env var
    #[must_use]
    pub fn with_config(enabled: bool, check_no_color: bool) -> Self {
        if check_no_color && Self::check_no_color().is_some() {
            ColorConfig { enabled: false }
        } else {
            ColorConfig { enabled }
        }
    }

    /// Creates a ColorConfig with colors disabled
    #[must_use]
    pub fn plain() -> Self {
        ColorConfig { enabled: false }
    }

    /// Checks the NO_COLOR environment variable
    ///
    /// Returns Some(value) if NO_COLOR is set (any value), None otherwise
    /// See: https://no-color.org/
    fn check_no_color() -> Option<String> {
        env::var("NO_COLOR").ok()
    }

    /// Returns true if colors are enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self::auto()
    }
}

/// Styles a text string with the given color if colors are enabled
///
/// # Arguments
///
/// * `text` - The text to style
/// * `color` - The color to apply
/// * `config` - The color configuration
///
/// # Returns
///
/// The styled text if colors are enabled, otherwise the original text
#[must_use]
pub fn style_text(text: &str, color: Color, config: ColorConfig) -> String {
    if !config.is_enabled() {
        return text.to_string();
    }

    let style = match color {
        Color::Red => console::Style::new().fg(console::Color::Red),
        Color::Green => console::Style::new().fg(console::Color::Green),
        Color::Yellow => console::Style::new().fg(console::Color::Yellow),
        Color::Blue => console::Style::new().fg(console::Color::Blue),
        Color::Magenta => console::Style::new().fg(console::Color::Magenta),
        Color::Cyan => console::Style::new().fg(console::Color::Cyan),
        Color::White => console::Style::new().fg(console::Color::White),
        Color::Black => console::Style::new().fg(console::Color::Black),
        Color::BrightRed => console::Style::new().fg(console::Color::Red).bright(),
        Color::BrightGreen => console::Style::new().fg(console::Color::Green).bright(),
        Color::BrightYellow => console::Style::new().fg(console::Color::Yellow).bright(),
        Color::BrightBlue => console::Style::new().fg(console::Color::Blue).bright(),
        Color::BrightMagenta => console::Style::new().fg(console::Color::Magenta).bright(),
        Color::BrightCyan => console::Style::new().fg(console::Color::Cyan).bright(),
        Color::BrightWhite => console::Style::new().fg(console::Color::White).bright(),
        Color::Dim => console::Style::new().dim(),
        Color::Bold => console::Style::new().bold(),
    };

    style.apply_to(text).to_string()
}

/// Available colors for text styling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Black,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Dim,
    Bold,
}

/// Colorizes a task status for terminal display
///
/// # Arguments
///
/// * `status` - The task status to colorize
/// * `config` - The color configuration
///
/// # Returns
///
/// The colorized status string
#[must_use]
pub fn colorize_status(status: &str, config: ColorConfig) -> String {
    if !config.is_enabled() {
        return status.to_string();
    }

    let (status_text, color) = match status.to_lowercase().as_str() {
        "pending" => ("PENDING", Color::Yellow),
        "in_progress" | "in-progress" | "inprogress" => ("IN_PROGRESS", Color::Blue),
        "completed" => ("COMPLETED", Color::BrightGreen),
        "failed" => ("FAILED", Color::BrightRed),
        "blocked" => ("BLOCKED", Color::BrightMagenta),
        _ => (status, Color::White),
    };

    style_text(status_text, color, config)
}

/// Colorizes a log level for terminal display
///
/// # Arguments
///
/// * `level` - The log level to colorize
/// * `config` - The color configuration
///
/// # Returns
///
/// The colorized log level string
#[must_use]
pub fn colorize_log_level(level: &str, config: ColorConfig) -> String {
    if !config.is_enabled() {
        return level.to_string();
    }

    let (level_text, color) = match level.to_lowercase().as_str() {
        "trace" => ("TRACE", Color::BrightWhite),
        "debug" => ("DEBUG", Color::BrightBlue),
        "info" => ("INFO", Color::BrightGreen),
        "warn" | "warning" => ("WARN", Color::BrightYellow),
        "error" => ("ERROR", Color::BrightRed),
        _ => (level, Color::White),
    };

    style_text(level_text, color, config)
}

/// Colorizes a progress indicator for terminal display
///
/// # Arguments
///
/// * `progress` - The progress indicator text
/// * `config` - The color configuration
///
/// # Returns
///
/// The colorized progress indicator string
#[must_use]
pub fn colorize_progress(progress: &str, config: ColorConfig) -> String {
    if !config.is_enabled() {
        return progress.to_string();
    }

    style_text(progress, Color::BrightCyan, config)
}

/// Creates a success message (green)
///
/// # Arguments
///
/// * `text` - The message text
/// * `config` - The color configuration
///
/// # Returns
///
/// The colorized success message
#[must_use]
pub fn success(text: &str, config: ColorConfig) -> String {
    style_text(text, Color::BrightGreen, config)
}

/// Creates an error message (red)
///
/// # Arguments
///
/// * `text` - The message text
/// * `config` - The color configuration
///
/// # Returns
///
/// The colorized error message
#[must_use]
pub fn error(text: &str, config: ColorConfig) -> String {
    style_text(text, Color::BrightRed, config)
}

/// Creates a warning message (yellow)
///
/// # Arguments
///
/// * `text` - The message text
/// * `config` - The color configuration
///
/// # Returns
///
/// The colorized warning message
#[must_use]
pub fn warning(text: &str, config: ColorConfig) -> String {
    style_text(text, Color::BrightYellow, config)
}

/// Creates an info message (blue)
///
/// # Arguments
///
/// * `text` - The message text
/// * `config` - The color configuration
///
/// # Returns
///
/// The colorized info message
#[must_use]
pub fn info(text: &str, config: ColorConfig) -> String {
    style_text(text, Color::BrightBlue, config)
}

/// Creates a dim/faint text (for secondary information)
///
/// # Arguments
///
/// * `text` - The message text
/// * `config` - The color configuration
///
/// # Returns
///
/// The colorized dim text
#[must_use]
pub fn dim(text: &str, config: ColorConfig) -> String {
    style_text(text, Color::Dim, config)
}

/// Creates a bold text (for emphasis)
///
/// # Arguments
///
/// * `text` - The message text
/// * `config` - The color configuration
///
/// # Returns
///
/// The colorized bold text
#[must_use]
pub fn bold(text: &str, config: ColorConfig) -> String {
    style_text(text, Color::Bold, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_config_auto() {
        let config = ColorConfig::auto();
        // Just ensure it doesn't panic
        let _ = config.is_enabled();
    }

    #[test]
    fn test_color_config_plain() {
        let config = ColorConfig::plain();
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_color_config_with_config() {
        let config = ColorConfig::with_config(true, false);
        assert!(config.is_enabled());

        let config = ColorConfig::with_config(false, false);
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_style_text_with_colors_disabled() {
        let config = ColorConfig::plain();
        let styled = style_text("test", Color::Red, config);
        assert_eq!(styled, "test");
    }

    #[test]
    fn test_colorize_status() {
        let config = ColorConfig::plain();
        assert_eq!(colorize_status("pending", config), "pending");
        assert_eq!(colorize_status("completed", config), "completed");
        assert_eq!(colorize_status("failed", config), "failed");
    }

    #[test]
    fn test_colorize_log_level() {
        let config = ColorConfig::plain();
        assert_eq!(colorize_log_level("info", config), "info");
        assert_eq!(colorize_log_level("error", config), "error");
        assert_eq!(colorize_log_level("warn", config), "warn");
    }

    #[test]
    fn test_colorize_progress() {
        let config = ColorConfig::plain();
        assert_eq!(colorize_progress("50%", config), "50%");
    }

    #[test]
    fn test_message_helpers() {
        let config = ColorConfig::plain();
        assert_eq!(success("Success!", config), "Success!");
        assert_eq!(error("Error!", config), "Error!");
        assert_eq!(warning("Warning!", config), "Warning!");
        assert_eq!(info("Info!", config), "Info!");
        assert_eq!(dim("Dim", config), "Dim");
        assert_eq!(bold("Bold", config), "Bold");
    }
}
