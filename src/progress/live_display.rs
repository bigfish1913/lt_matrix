// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Live progress display with fixed header and scrolling messages
//!
//! This module provides a terminal UI that shows:
//! - Fixed header with overall progress, elapsed time, and last activity
//! - Scrolling message area for Claude output and status updates
//! - Animated spinner and progress indicators with background refresh

use std::io::{self, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::terminal::ColorConfig;

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
    /// Start time
    start_time: Instant,
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
}

/// Spinner frames for animation
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Progress animation frames
const ANIMATION: &[&str] = &["█", "▓", "▒", "░", "▓", "▒"];

/// Refresh interval in milliseconds
const REFRESH_INTERVAL_MS: u64 = 150;

impl LiveDisplay {
    /// Create a new live display
    pub fn new(enabled: bool) -> Self {
        LiveDisplay {
            color_config: ColorConfig::auto(),
            start_time: Instant::now(),
            last_activity: Arc::new(Mutex::new(Instant::now())),
            stats: Arc::new(Mutex::new(ProgressStats::default())),
            messages: Arc::new(Mutex::new(Vec::new())),
            max_messages: 100,
            enabled,
            header_lines: 6,
            frame: Arc::new(Mutex::new(0)),
            refresh_handle: Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start the background refresh thread
    pub fn start(&self) {
        if !self.enabled {
            return;
        }

        // Already running
        if self.running.load(Ordering::SeqCst) {
            return;
        }

        self.running.store(true, Ordering::SeqCst);

        let stats = Arc::clone(&self.stats);
        let messages = Arc::clone(&self.messages);
        let last_activity = Arc::clone(&self.last_activity);
        let frame = Arc::clone(&self.frame);
        let running = Arc::clone(&self.running);
        let start_time = self.start_time;
        let header_lines = self.header_lines;

        let handle = thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                // Render display
                Self::render_internal(
                    &stats,
                    &messages,
                    &last_activity,
                    &frame,
                    start_time,
                    header_lines,
                );

                thread::sleep(Duration::from_millis(REFRESH_INTERVAL_MS));
            }
        });

        if let Ok(mut h) = self.refresh_handle.lock() {
            *h = Some(handle);
        }
    }

    /// Stop the background refresh thread
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);

        if let Ok(mut handle) = self.refresh_handle.lock() {
            if let Some(h) = handle.take() {
                let _ = h.join();
            }
        }
    }

    /// Get spinner frame
    fn get_spinner(frame: usize) -> &'static str {
        SPINNER_FRAMES[frame % SPINNER_FRAMES.len()]
    }

    /// Get progress animation characters
    fn get_progress_animation(frame: usize) -> &'static str {
        ANIMATION[frame % ANIMATION.len()]
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
                MessageLevel::Info => "ℹ",
                MessageLevel::Success => "✓",
                MessageLevel::Warning => "⚠",
                MessageLevel::Error => "✗",
                MessageLevel::Debug => "◇",
            };
            println!("[{}] {} {}", prefix, source, message);
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

    /// Create a progress bar string
    fn create_progress_bar(current: usize, total: usize, width: usize) -> String {
        if total == 0 {
            return "░".repeat(width);
        }

        let percentage = current as f64 / total as f64;
        let filled = (percentage * width as f64).round() as usize;
        let filled = filled.min(width);

        let bar: String = "█".repeat(filled);
        let empty: String = "░".repeat(width.saturating_sub(filled));

        format!("{}{} {:.0}%", bar, empty, percentage * 100.0)
    }

    /// Internal render function used by background thread
    fn render_internal(
        stats: &Arc<Mutex<ProgressStats>>,
        messages: &Arc<Mutex<Vec<LogMessage>>>,
        last_activity: &Arc<Mutex<Instant>>,
        frame: &Arc<Mutex<usize>>,
        start_time: Instant,
        header_lines: usize,
    ) {
        let stats = match stats.lock() {
            Ok(s) => s.clone(),
            Err(_) => return,
        };

        let messages = match messages.lock() {
            Ok(m) => m.clone(),
            Err(_) => return,
        };

        let elapsed = start_time.elapsed();
        let idle = Self::format_idle_time(last_activity);

        // Get and increment animation frame
        let frame_num = match frame.lock() {
            Ok(mut f) => {
                let current = *f;
                *f = (*f + 1) % SPINNER_FRAMES.len();
                current
            }
            Err(_) => 0,
        };

        let (width, height) = Self::get_terminal_size();
        let width = width.max(40);
        let height = height.max(10);

        let mut output = String::new();
        output.push_str("\x1B[H");
        output.push_str("\x1B[?25l");

        // ===== HEADER =====
        let border = "═".repeat(width.saturating_sub(22));
        let spinner = Self::get_spinner(frame_num);

        output.push_str(&format!("\x1B[2K\r\x1B[1;36m╔{}╗\x1B[0m\n", border));
        output.push_str(&format!(
            "\x1B[2K\r\x1B[1;36m║\x1B[1;37m{:.^width$}\x1B[0m \x1B[1;33m{}\x1B[0m \x1B[36m║\x1B[0m\n",
            format!(" ltmatrix - {} {} ", stats.stage, spinner),
            width = width.saturating_sub(4)
        ));
        output.push_str(&format!("\x1B[2K\r\x1B[1;36m╠{}╣\x1B[0m\n", border));

        // Progress bar
        let progress_bar = Self::create_progress_bar(
            stats.completed_tasks,
            stats.total_tasks.max(stats.completed_tasks),
            width.saturating_sub(22),
        );
        let anim = Self::get_progress_animation(frame_num);
        output.push_str(&format!(
            "\x1B[2K\r\x1B[1;36m║\x1B[32m{} \x1B[1;33m{}\x1B[0m \x1B[1;36m║\x1B[0m\n",
            progress_bar, anim
        ));

        // Stats line
        let stats_line = format!(
            "阶段: {}/{}  │  任务: {}/{}  │  失败: {}  │  耗时: {}  │  {}",
            stats.stage_index + 1,
            stats.total_stages,
            stats.completed_tasks,
            stats.total_tasks,
            stats.failed_tasks,
            Self::format_duration(elapsed),
            idle
        );
        output.push_str(&format!(
            "\x1B[2K\r\x1B[1;36m║\x1B[90m{:<width$}\x1B[0m \x1B[1;36m║\x1B[0m\n",
            stats_line,
            width = width.saturating_sub(4)
        ));

        // Current task
        let current_line = if let Some(ref task) = stats.current_task {
            let truncated = if task.len() > width.saturating_sub(15) {
                format!("{}...", &task[..width.saturating_sub(18)])
            } else {
                task.clone()
            };
            format!("当前: {}", truncated)
        } else {
            format!("{} 等待中...", Self::get_spinner(frame_num))
        };
        output.push_str(&format!(
            "\x1B[2K\r\x1B[1;36m║\x1B[33m{:<width$}\x1B[0m \x1B[1;36m║\x1B[0m\n",
            current_line,
            width = width.saturating_sub(4)
        ));
        output.push_str(&format!("\x1B[2K\r\x1B[1;36m╚{}╝\x1B[0m\n", border));

        // ===== MESSAGES =====
        let available_lines = height.saturating_sub(header_lines + 2);
        let msg_start = messages.len().saturating_sub(available_lines);
        let mut lines_written = 0;

        for msg in messages.iter().skip(msg_start) {
            let prefix = match msg.level {
                MessageLevel::Info => "\x1B[34mℹ\x1B[0m",
                MessageLevel::Success => "\x1B[32m✓\x1B[0m",
                MessageLevel::Warning => "\x1B[33m⚠\x1B[0m",
                MessageLevel::Error => "\x1B[31m✗\x1B[0m",
                MessageLevel::Debug => "\x1B[90m◇\x1B[0m",
            };

            let time_ago = msg.timestamp.elapsed();
            let time_str = if time_ago < Duration::from_secs(60) {
                format!("{}s", time_ago.as_secs())
            } else {
                format!("{}m", time_ago.as_secs() / 60)
            };

            let source = if msg.source.is_empty() {
                String::new()
            } else {
                format!("[{}] ", msg.source)
            };

            let max_msg_len = width.saturating_sub(time_str.len() + source.len() + 5);
            let truncated_msg = if msg.message.len() > max_msg_len {
                format!("{}...", &msg.message[..max_msg_len.saturating_sub(3)])
            } else {
                msg.message.clone()
            };

            output.push_str(&format!(
                "\x1B[2K\r{} \x1B[90m{:>3}\x1B[0m \x1B[36m{:<10}\x1B[0m {}\n",
                prefix, time_str, source, truncated_msg
            ));
            lines_written += 1;
        }

        let max_clear = 3;
        let extra_lines_to_clear = available_lines.saturating_sub(lines_written).min(max_clear);
        for _ in 0..extra_lines_to_clear {
            output.push_str("\x1B[2K\r\n");
        }

        output.push_str("\x1B[?25h");

        let _ = io::stdout().write_all(output.as_bytes());
        let _ = io::stdout().flush();
    }

    /// Render the display (manual trigger)
    pub fn render(&self) {
        if !self.enabled {
            return;
        }
        Self::render_internal(
            &self.stats,
            &self.messages,
            &self.last_activity,
            &self.frame,
            self.start_time,
            self.header_lines,
        );
    }

    /// Clear the display and show final summary
    pub fn finish(&self, success: bool, summary: &str) {
        // Stop background thread first
        self.stop();

        if !self.enabled {
            println!("{}", summary);
            return;
        }

        let mut output = String::new();
        output.push_str("\x1B[2J\x1B[H");

        let elapsed = self.start_time.elapsed();
        let status = if success {
            "\x1B[32m✓ 完成\x1B[0m"
        } else {
            "\x1B[31m✗ 失败\x1B[0m"
        };

        output.push_str(&format!(
            "\n{} \x1B[90m总耗时: {}\x1B[0m\n\n",
            status,
            Self::format_duration(elapsed)
        ));
        output.push_str(summary);
        output.push_str("\n");

        let _ = io::stdout().write_all(output.as_bytes());
        let _ = io::stdout().flush();
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
