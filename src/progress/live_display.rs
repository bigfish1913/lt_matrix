// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Live progress display with clean, minimal output
//!
//! This module provides a terminal UI that shows:
//! - Single-line status with spinner, progress bar, and current task
//! - Clean message output without screen flickering
//! - No complex borders or animations that cause display issues

use std::io::{self, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::terminal::ColorConfig;

/// Enable ANSI escape code support on Windows
#[cfg(windows)]
fn enable_ansi_support() -> io::Result<()> {
    use std::os::windows::raw::HANDLE;

    const STD_OUTPUT_HANDLE: u32 = 0xFFFFFFF5;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    extern "system" {
        fn GetStdHandle(nStdHandle: u32) -> HANDLE;
        fn GetConsoleMode(hConsoleHandle: HANDLE, lpMode: *mut u32) -> i32;
        fn SetConsoleMode(hConsoleHandle: HANDLE, dwMode: u32) -> i32;
    }

    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle.is_null() {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to get stdout handle"));
        }

        let mut mode: u32 = 0;
        if GetConsoleMode(handle, &mut mode) == 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to get console mode"));
        }

        if SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) == 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to set console mode"));
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn enable_ansi_support() -> io::Result<()> {
    Ok(())
}

/// Message severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLevel {
    Info,
    Success,
    Warning,
    Error,
    Debug,
}

/// A log message with timestamp
#[derive(Debug, Clone)]
pub struct LogMessage {
    pub timestamp: Instant,
    pub level: MessageLevel,
    pub message: String,
    pub source: String,
}

/// Statistics for the progress display
#[derive(Debug, Clone, Default)]
pub struct ProgressStats {
    pub stage: String,
    pub stage_index: usize,
    pub total_stages: usize,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub current_task: Option<String>,
}

/// Live progress display manager with background animation
pub struct LiveDisplay {
    /// Color configuration
    color_config: ColorConfig,
    /// Start time (mutable for reset)
    start_time: Arc<Mutex<Option<Instant>>>,
    /// Last activity time
    last_activity: Arc<Mutex<Instant>>,
    /// Progress statistics
    stats: Arc<Mutex<ProgressStats>>,
    /// Recent log messages (limited to prevent memory growth)
    messages: Arc<Mutex<Vec<LogMessage>>>,
    /// Maximum messages to keep
    max_messages: usize,
    /// Whether display is enabled
    enabled: bool,
    /// Number of lines used for header
    header_lines: usize,
    /// Animation frame counter
    frame: Arc<Mutex<usize>>,
    /// Background refresh thread handle
    refresh_handle: Mutex<Option<JoinHandle<()>>>,
    /// Flag to stop background thread
    running: Arc<AtomicBool>,
    /// Track if we've taken over the terminal
    terminal_captured: Arc<Mutex<bool>>,
}

/// Simple spinner frames (ASCII-safe)
const SPINNER_FRAMES: &[&str] = &["-", "\\", "|", "/"];

/// Refresh interval in milliseconds
const REFRESH_INTERVAL_MS: u64 = 200;

impl LiveDisplay {
    /// Create a new live display
    pub fn new(enabled: bool) -> Self {
        LiveDisplay {
            color_config: ColorConfig::auto(),
            start_time: Arc::new(Mutex::new(None)),
            last_activity: Arc::new(Mutex::new(Instant::now())),
            stats: Arc::new(Mutex::new(ProgressStats::default())),
            messages: Arc::new(Mutex::new(Vec::new())),
            max_messages: 50,
            enabled,
            header_lines: 3, // Number of status lines we use
            frame: Arc::new(Mutex::new(0)),
            refresh_handle: Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
            terminal_captured: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the background refresh thread
    pub fn start(&self) {
        if !self.enabled {
            return;
        }

        // Enable ANSI support on Windows - this is critical for Windows terminals
        #[cfg(windows)]
        {
            let _ = enable_ansi_support();
        }

        // Always reset start time for a fresh start - this ensures time is accurate
        if let Ok(mut start) = self.start_time.lock() {
            *start = Some(Instant::now());
        }

        // Mark terminal as captured
        if let Ok(mut captured) = self.terminal_captured.lock() {
            *captured = true;
        }

        // Already running - just update the start time above and return
        if self.running.load(Ordering::SeqCst) {
            return;
        }

        // Print initial empty lines for cursor movement
        let header_lines = self.header_lines;
        for _ in 0..(header_lines + 1) {
            println!();
        }

        self.running.store(true, Ordering::SeqCst);

        let stats = Arc::clone(&self.stats);
        let messages = Arc::clone(&self.messages);
        let last_activity = Arc::clone(&self.last_activity);
        let start_time = Arc::clone(&self.start_time);
        let frame = Arc::clone(&self.frame);
        let running = Arc::clone(&self.running);
        let terminal_captured = Arc::clone(&self.terminal_captured);

        let handle = thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                let current_start_time = if let Ok(st) = start_time.lock() {
                    *st
                } else {
                    None
                };

                Self::render_frame(
                    &stats,
                    &messages,
                    &last_activity,
                    &frame,
                    current_start_time,
                    header_lines,
                    &terminal_captured,
                );

                thread::sleep(Duration::from_millis(REFRESH_INTERVAL_MS));
            }
        });

        if let Ok(mut h) = self.refresh_handle.lock() {
            *h = Some(handle);
        }
    }

    /// Stop the background refresh thread and restore terminal
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);

        if let Ok(mut handle) = self.refresh_handle.lock() {
            if let Some(h) = handle.take() {
                let _ = h.join();
            }
        }

        self.restore_terminal();
    }

    /// Restore terminal to normal state
    fn restore_terminal(&self) {
        let is_captured = if let Ok(c) = self.terminal_captured.lock() {
            *c
        } else {
            false
        };

        if !is_captured {
            return;
        }

        if let Ok(mut captured) = self.terminal_captured.lock() {
            *captured = false;
        }

        // Restore terminal - just show cursor and add newline
        let term = console::Term::stdout();
        let _ = term.show_cursor();
        println!();
    }

    /// Get spinner frame
    fn get_spinner(frame: usize) -> &'static str {
        SPINNER_FRAMES[frame % SPINNER_FRAMES.len()]
    }

    /// Get progress animation characters (no longer used, kept for compatibility)
    fn get_progress_animation(_frame: usize) -> &'static str {
        ""
    }

    /// Get terminal size using console crate
    fn get_terminal_size() -> (usize, usize) {
        let term = console::Term::stdout();
        let (width, height) = term.size();
        (width as usize, height as usize)
    }

    /// Update the progress statistics
    pub fn update_stats(&self, new_stats: ProgressStats) {
        if !self.enabled {
            return;
        }
        if let Ok(mut stats) = self.stats.lock() {
            *stats = new_stats;
        }
        self.touch_activity();
    }

    /// Update just the current task
    pub fn set_current_task(&self, task: Option<String>) {
        if !self.enabled {
            return;
        }
        if let Ok(mut stats) = self.stats.lock() {
            stats.current_task = task;
        }
        self.touch_activity();
    }

    /// Add a log message
    pub fn log(&self, level: MessageLevel, source: &str, message: &str) {
        if !self.enabled {
            let prefix = match level {
                MessageLevel::Info => "i",
                MessageLevel::Success => "+",
                MessageLevel::Warning => "!",
                MessageLevel::Error => "x",
                MessageLevel::Debug => "-",
            };
            let colored_prefix = match level {
                MessageLevel::Info => console::style(prefix).blue(),
                MessageLevel::Success => console::style(prefix).green(),
                MessageLevel::Warning => console::style(prefix).yellow(),
                MessageLevel::Error => console::style(prefix).red(),
                MessageLevel::Debug => console::style(prefix).dim(),
            };
            let source_part = if source.is_empty() {
                String::new()
            } else {
                format!("[{}] ", console::style(source).cyan())
            };
            println!("{} {}{}", colored_prefix, source_part, message);
            return;
        }

        let msg = LogMessage {
            timestamp: Instant::now(),
            level,
            message: message.to_string(),
            source: source.to_string(),
        };

        if let Ok(mut messages) = self.messages.lock() {
            messages.push(msg);
            if messages.len() > self.max_messages {
                messages.remove(0);
            }
        }

        self.touch_activity();
    }

    /// Log info message
    pub fn info(&self, source: &str, message: &str) {
        self.log(MessageLevel::Info, source, message);
    }

    /// Log success message
    pub fn success(&self, source: &str, message: &str) {
        self.log(MessageLevel::Success, source, message);
    }

    /// Log warning message
    pub fn warning(&self, source: &str, message: &str) {
        self.log(MessageLevel::Warning, source, message);
    }

    /// Log error message
    pub fn error(&self, source: &str, message: &str) {
        self.log(MessageLevel::Error, source, message);
    }

    /// Log debug message
    pub fn debug(&self, source: &str, message: &str) {
        self.log(MessageLevel::Debug, source, message);
    }

    /// Update last activity timestamp
    fn touch_activity(&self) {
        if let Ok(mut last) = self.last_activity.lock() {
            *last = Instant::now();
        }
    }

    /// Format duration as human readable string
    fn format_duration(duration: Duration) -> String {
        let total_secs = duration.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }

    /// Format time since last activity
    fn format_idle_time(last_activity: &Arc<Mutex<Instant>>) -> String {
        if let Ok(last) = last_activity.lock() {
            let idle = last.elapsed();
            if idle < Duration::from_secs(5) {
                return "活跃".to_string();
            } else if idle < Duration::from_secs(60) {
                return format!("{}秒前", idle.as_secs());
            } else {
                return format!("{}分钟前", idle.as_secs() / 60);
            }
        }
        "未知".to_string()
    }

    /// Create a compact progress bar string
    fn create_progress_bar(current: usize, total: usize, width: usize) -> String {
        if total == 0 {
            return format!("[{}] 0%", " ".repeat(width));
        }

        let percentage = current as f64 / total as f64;
        let filled = (percentage * width as f64).round() as usize;
        let filled = filled.min(width);

        let bar: String = "=".repeat(filled);
        let empty: String = " ".repeat(width.saturating_sub(filled));

        format!("[{}{}] {:3.0}%", bar, empty, percentage * 100.0)
    }

    /// Render a single frame (used by background thread)
    fn render_frame(
        stats: &Arc<Mutex<ProgressStats>>,
        messages: &Arc<Mutex<Vec<LogMessage>>>,
        last_activity: &Arc<Mutex<Instant>>,
        frame: &Arc<Mutex<usize>>,
        start_time: Option<Instant>,
        header_lines: usize,
        terminal_captured: &Arc<Mutex<bool>>,
    ) {
        let is_captured = if let Ok(c) = terminal_captured.lock() {
            *c
        } else {
            false
        };

        if !is_captured {
            return;
        }

        let stats = match stats.lock() {
            Ok(s) => s.clone(),
            Err(_) => return,
        };

        let messages = match messages.lock() {
            Ok(m) => m.clone(),
            Err(_) => return,
        };

        let elapsed = match start_time {
            Some(st) => st.elapsed(),
            None => Duration::from_secs(0),
        };

        let frame_num = match frame.lock() {
            Ok(mut f) => {
                let current = *f;
                *f = (*f + 1) % SPINNER_FRAMES.len();
                current
            }
            Err(_) => 0,
        };

        let term = console::Term::stdout();
        let (width, _) = term.size();
        let width = (width as usize).max(40);

        // Move cursor up to rewrite status lines (no flickering clear)
        for _ in 0..header_lines {
            let _ = term.move_cursor_up(1);
        }

        let spinner = Self::get_spinner(frame_num);

        // Line 1: Single line with all info
        let progress_pct = if stats.total_tasks > 0 {
            (stats.completed_tasks * 100) / stats.total_tasks
        } else {
            0
        };

        let current_task = if let Some(ref task) = stats.current_task {
            let max_len = width.saturating_sub(35);
            if task.len() > max_len {
                format!("{}...", &task[..max_len.saturating_sub(3)])
            } else {
                task.clone()
            }
        } else {
            "Waiting...".to_string()
        };

        let status_line = format!(
            "{} [{}] {} ({}/{}) {}",
            spinner,
            Self::format_duration(elapsed),
            stats.stage,
            stats.completed_tasks,
            stats.total_tasks,
            current_task
        );

        let _ = term.clear_line();
        let _ = term.write_line(&format!("{}", console::style(status_line).cyan().bold()));

        // Line 2: Progress bar
        let progress_bar = Self::create_progress_bar(
            stats.completed_tasks,
            stats.total_tasks.max(stats.completed_tasks),
            width.saturating_sub(15),
        );
        let _ = term.clear_line();
        let _ = term.write_line(&format!(
            "  {}",
            console::style(progress_bar).green()
        ));

        // Line 3: Current task (target) - highlighted
        let _ = term.clear_line();
        let target = if let Some(ref task) = stats.current_task {
            let max_len = width.saturating_sub(12);
            if task.len() > max_len {
                format!("> {}", &task[..max_len.saturating_sub(3)])
            } else {
                format!("> {}", task)
            }
        } else {
            format!("{} Waiting...", spinner)
        };
        let _ = term.write_line(&format!("{}", console::style(target).yellow().bold()));

        // Flush output
        let _ = term.flush();
    }

    /// Render the display (manual trigger)
    pub fn render(&self) {
        if !self.enabled {
            return;
        }
        let current_start_time = if let Ok(st) = self.start_time.lock() {
            *st
        } else {
            None
        };
        Self::render_frame(
            &self.stats,
            &self.messages,
            &self.last_activity,
            &self.frame,
            current_start_time,
            self.header_lines,
            &self.terminal_captured,
        );
    }

    /// Clear the display and show final summary
    pub fn finish(&self, success: bool, summary: &str) {
        self.stop();

        if !self.enabled {
            println!("{}", summary);
            return;
        }

        let current_start_time = if let Ok(st) = self.start_time.lock() {
            *st
        } else {
            None
        };

        let elapsed = match current_start_time {
            Some(st) => st.elapsed(),
            None => Duration::from_secs(0),
        };

        let status = if success {
            console::style("[OK]").green().bold()
        } else {
            console::style("[FAIL]").red().bold()
        };

        println!();
        println!(
            "{} Completed in {}",
            status,
            console::style(Self::format_duration(elapsed)).cyan()
        );
        println!();
        println!("{}", summary);
        println!();
    }

    /// Reset for a new execution
    pub fn reset(&self) {
        self.stop();

        if let Ok(mut start) = self.start_time.lock() {
            *start = None;
        }
        if let Ok(mut msgs) = self.messages.lock() {
            msgs.clear();
        }
        if let Ok(mut st) = self.stats.lock() {
            *st = ProgressStats::default();
        }
    }
}

impl Drop for LiveDisplay {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Default for LiveDisplay {
    fn default() -> Self {
        Self::new(true)
    }
}

/// Global display instance
static LIVE_DISPLAY: std::sync::OnceLock<LiveDisplay> = std::sync::OnceLock::new();

/// Get or create the global display
pub fn get_display() -> &'static LiveDisplay {
    LIVE_DISPLAY.get_or_init(|| LiveDisplay::new(true))
}

/// Initialize the global display with enabled/disabled state
pub fn init_display(enabled: bool) -> &'static LiveDisplay {
    LIVE_DISPLAY.get_or_init(|| LiveDisplay::new(enabled))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_display_new() {
        let display = LiveDisplay::new(false);
        assert!(!display.enabled);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(LiveDisplay::format_duration(Duration::from_secs(5)), "5s");
        assert_eq!(
            LiveDisplay::format_duration(Duration::from_secs(65)),
            "1m 5s"
        );
        assert_eq!(
            LiveDisplay::format_duration(Duration::from_secs(3665)),
            "1h 1m 5s"
        );
    }

    #[test]
    fn test_progress_bar() {
        let bar = LiveDisplay::create_progress_bar(5, 10, 20);
        assert!(bar.contains("50%"));
    }

    #[test]
    fn test_log_messages() {
        let display = LiveDisplay::new(false);
        display.info("test", "Hello world");

        let messages = display.messages.lock().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].level, MessageLevel::Info);
        assert_eq!(messages[0].message, "Hello world");
    }
}
