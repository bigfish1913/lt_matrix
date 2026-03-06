//! Progress bar implementation with color support
//!
//! This module provides progress bars with colorized output for terminals.

use crate::terminal::{self, ColorConfig};
use indicatif::{ProgressBar, ProgressStyle};

/// Color configuration for progress bars
#[derive(Debug, Clone, Copy)]
pub struct BarColorConfig {
    pub inner: ColorConfig,
}

impl BarColorConfig {
    /// Creates a new BarColorConfig that auto-detects terminal capabilities
    #[must_use]
    pub fn auto() -> Self {
        BarColorConfig {
            inner: ColorConfig::auto(),
        }
    }

    /// Creates a BarColorConfig with colors disabled
    #[must_use]
    pub fn plain() -> Self {
        BarColorConfig {
            inner: ColorConfig::plain(),
        }
    }

    /// Returns true if colors are enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }
}

impl Default for BarColorConfig {
    fn default() -> Self {
        Self::auto()
    }
}

/// Creates a new progress bar with the given length and color configuration
///
/// # Arguments
///
/// * `len` - The total length/number of items for the progress bar
/// * `config` - Optional color configuration
///
/// # Returns
///
/// A configured ProgressBar
pub fn create_progress_bar(len: u64, config: Option<BarColorConfig>) -> ProgressBar {
    let color_config = config.unwrap_or_default();

    let bar = ProgressBar::new(len);

    if color_config.is_enabled() {
        // Colored progress bar
        let style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .expect("Invalid progress bar template")
            .progress_chars("=> ");

        bar.set_style(style);
    } else {
        // Plain progress bar
        let style = ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40}] {pos}/{len} {msg}")
            .expect("Invalid progress bar template")
            .progress_chars("=> ");

        bar.set_style(style);
    }

    bar
}

/// Creates a new progress bar with a custom template
///
/// # Arguments
///
/// * `len` - The total length/number of items for the progress bar
/// * `template` - The progress bar template string
/// * `config` - Optional color configuration
///
/// # Returns
///
/// A configured ProgressBar
pub fn create_custom_progress_bar(
    len: u64,
    template: &str,
    config: Option<BarColorConfig>,
) -> ProgressBar {
    let bar = ProgressBar::new(len);

    let color_config = config.unwrap_or_default();

    if color_config.is_enabled() {
        let style = ProgressStyle::default_bar()
            .template(template)
            .unwrap_or_else(|_| {
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] [{bar:40}] {pos}/{len} {msg}")
                    .unwrap()
            })
            .progress_chars("=> ");

        bar.set_style(style);
    } else {
        let style = ProgressStyle::default_bar()
            .template(template)
            .unwrap_or_else(|_| {
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] [{bar:40}] {pos}/{len} {msg}")
                    .unwrap()
            })
            .progress_chars("=> ");

        bar.set_style(style);
    }

    bar
}

/// Creates a spinner-style progress bar (for indeterminate progress)
///
/// # Arguments
///
/// * `config` - Optional color configuration
///
/// # Returns
///
/// A configured spinner ProgressBar
pub fn create_spinner(config: Option<BarColorConfig>) -> ProgressBar {
    let color_config = config.unwrap_or_default();
    let bar = ProgressBar::new_spinner();

    if color_config.is_enabled() {
        let style = ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("Invalid spinner template");

        bar.set_style(style);
    } else {
        let style = ProgressStyle::default_spinner()
            .template("{msg}")
            .expect("Invalid spinner template");

        bar.set_style(style);
    }

    bar
}

/// Colorizes a progress percentage for display
///
/// # Arguments
///
/// * `percent` - The percentage value (0-100)
/// * `config` - The color configuration
///
/// # Returns
///
/// A colorized percentage string
#[must_use]
pub fn colorize_percentage(percent: u64, config: ColorConfig) -> String {
    let text = format!("{}%", percent);

    if !config.is_enabled() {
        return text;
    }

    let color = match percent {
        0..=25 => terminal::Color::Red,
        26..=50 => terminal::Color::Yellow,
        51..=75 => terminal::Color::Blue,
        76..=100 => terminal::Color::BrightGreen,
        _ => terminal::Color::White,
    };

    terminal::style_text(&text, color, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_progress_bar() {
        let bar = create_progress_bar(100, None);
        assert_eq!(bar.length(), Some(100));
    }

    #[test]
    fn test_create_progress_bar_plain() {
        let config = BarColorConfig::plain();
        let bar = create_progress_bar(100, Some(config));
        assert_eq!(bar.length(), Some(100));
    }

    #[test]
    fn test_create_spinner() {
        let spinner = create_spinner(None);
        // Just verify it doesn't panic
        drop(spinner);
    }

    #[test]
    fn test_create_spinner_plain() {
        let config = BarColorConfig::plain();
        let spinner = create_spinner(Some(config));
        // Just verify it doesn't panic
        drop(spinner);
    }

    #[test]
    fn test_colorize_percentage() {
        let config = ColorConfig::plain();
        assert_eq!(colorize_percentage(0, config), "0%");
        assert_eq!(colorize_percentage(50, config), "50%");
        assert_eq!(colorize_percentage(100, config), "100%");
    }

    #[test]
    fn test_bar_color_config_auto() {
        let config = BarColorConfig::auto();
        // Just verify it doesn't panic
        let _ = config.is_enabled();
    }

    #[test]
    fn test_bar_color_config_plain() {
        let config = BarColorConfig::plain();
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_bar_color_config_default() {
        let config = BarColorConfig::default();
        // Just verify it doesn't panic
        let _ = config.is_enabled();
    }
}
