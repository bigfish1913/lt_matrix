//! Progress management with multi-bar support
//!
//! This module provides the ProgressManager for coordinating multiple progress bars,
//! tracking task progress, and displaying real-time updates.

use crate::models::{Task, TaskStatus};
use crate::terminal::ColorConfig;
use crate::progress::tracker::{ProgressTracker, TrackerColorConfig};
use crate::progress::eta::{EtaCalculator, HistoricalData, MetricsCollector, format_eta};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// Internal color config wrapper for progress bars
#[derive(Debug, Clone, Copy)]
struct BarColorConfig {
    pub inner: ColorConfig,
}

impl BarColorConfig {
    fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }
}

/// Configuration for ProgressManager
#[derive(Debug, Clone, Copy)]
pub struct ProgressManagerConfig {
    /// Color configuration
    pub color_config: ColorConfig,

    /// Enable multi-line progress bars
    pub enable_multi: bool,

    /// Enable ETA estimation
    pub enable_eta: bool,

    /// Update interval for progress bars (in milliseconds)
    pub update_interval_ms: u64,
}

impl ProgressManagerConfig {
    /// Creates a new ProgressManagerConfig with defaults
    #[must_use]
    pub fn new() -> Self {
        ProgressManagerConfig {
            color_config: ColorConfig::auto(),
            enable_multi: true,
            enable_eta: true,
            update_interval_ms: 100,
        }
    }

    /// Creates a plain config without colors
    #[must_use]
    pub fn plain() -> Self {
        ProgressManagerConfig {
            color_config: ColorConfig::plain(),
            enable_multi: true,
            enable_eta: true,
            update_interval_ms: 100,
        }
    }

    /// Sets whether multi-line progress is enabled
    #[must_use]
    pub fn with_multi(mut self, enable: bool) -> Self {
        self.enable_multi = enable;
        self
    }

    /// Sets whether ETA estimation is enabled
    #[must_use]
    pub fn with_eta(mut self, enable: bool) -> Self {
        self.enable_eta = enable;
        self
    }

    /// Sets the update interval
    #[must_use]
    pub fn with_update_interval(mut self, millis: u64) -> Self {
        self.update_interval_ms = millis;
        self
    }
}

impl Default for ProgressManagerConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of progress bar
#[derive(Debug, Clone, Copy)]
pub enum ProgressBarType {
    /// Single progress bar for overall progress
    Single,
    /// Multi-line progress bars for individual tasks
    Multi,
}

/// Manages progress display with support for single and multi-line progress bars
pub struct ProgressManager {
    /// Multi-progress instance for managing multiple bars
    multi: Option<MultiProgress>,

    /// Main progress bar (for single mode)
    main_bar: Option<ProgressBar>,

    /// Individual task progress bars (for multi mode)
    task_bars: HashMap<String, ProgressBar>,

    /// Task tracker
    tracker: ProgressTracker,

    /// Configuration
    config: ProgressManagerConfig,

    /// Total number of tasks
    total_tasks: usize,

    /// Current task names (for multi mode)
    task_names: Arc<Mutex<HashMap<String, String>>>,

    /// ETA calculator based on historical data
    eta_calculator: Option<EtaCalculator>,

    /// Metrics collector for tracking task performance
    metrics_collector: MetricsCollector,

    /// Task start times for elapsed time calculation
    task_start_times: Arc<Mutex<HashMap<String, Instant>>>,

    /// Overall start time for the entire task set
    overall_start_time: Option<Instant>,
}

impl ProgressManager {
    /// Creates a new ProgressManager with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Optional configuration
    ///
    /// # Returns
    ///
    /// A new ProgressManager instance
    #[must_use]
    pub fn new(config: Option<ProgressManagerConfig>) -> Self {
        let config = config.unwrap_or_default();
        let multi = if config.enable_multi {
            Some(MultiProgress::new())
        } else {
            None
        };

        ProgressManager {
            multi,
            main_bar: None,
            task_bars: HashMap::new(),
            tracker: ProgressTracker::new(Some(TrackerColorConfig {
                inner: config.color_config,
            })),
            config,
            total_tasks: 0,
            task_names: Arc::new(Mutex::new(HashMap::new())),
            eta_calculator: None,
            metrics_collector: MetricsCollector::new(),
            task_start_times: Arc::new(Mutex::new(HashMap::new())),
            overall_start_time: None,
        }
    }

    /// Creates a new ProgressManager with historical data for ETA estimation
    ///
    /// # Arguments
    ///
    /// * `config` - Optional configuration
    /// * `historical_data` - Historical task completion data
    ///
    /// # Returns
    ///
    /// A new ProgressManager instance with ETA calculation
    #[must_use]
    pub fn with_historical_data(
        config: Option<ProgressManagerConfig>,
        historical_data: HistoricalData,
    ) -> Self {
        let config = config.unwrap_or_default();
        let multi = if config.enable_multi {
            Some(MultiProgress::new())
        } else {
            None
        };

        let eta_calculator = if config.enable_eta {
            Some(EtaCalculator::new(historical_data))
        } else {
            None
        };

        ProgressManager {
            multi,
            main_bar: None,
            task_bars: HashMap::new(),
            tracker: ProgressTracker::new(Some(TrackerColorConfig {
                inner: config.color_config,
            })),
            config,
            total_tasks: 0,
            task_names: Arc::new(Mutex::new(HashMap::new())),
            eta_calculator,
            metrics_collector: MetricsCollector::new(),
            task_start_times: Arc::new(Mutex::new(HashMap::new())),
            overall_start_time: None,
        }
    }

    /// Initializes the progress manager with a total number of tasks
    ///
    /// # Arguments
    ///
    /// * `total_tasks` - Total number of tasks to track
    /// * `bar_type` - Type of progress bar to use
    pub fn initialize(&mut self, total_tasks: usize, bar_type: ProgressBarType) {
        self.total_tasks = total_tasks;
        self.overall_start_time = Some(Instant::now());

        match bar_type {
            ProgressBarType::Single => {
                self.initialize_single_bar();
            }
            ProgressBarType::Multi => {
                self.initialize_multi_bars();
            }
        }
    }

    /// Initializes a single progress bar for overall progress
    fn initialize_single_bar(&mut self) {
        let bar = if let Some(multi) = &self.multi {
            multi.add(ProgressBar::new(self.total_tasks as u64))
        } else {
            ProgressBar::new(self.total_tasks as u64)
        };

        let color_config = BarColorConfig {
            inner: self.config.color_config,
        };

        let template = if self.config.enable_eta {
            if color_config.is_enabled() {
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) ETA: {eta} {msg}"
            } else {
                "[{elapsed_precise}] [{bar:40}] {pos}/{len} ({percent}%) ETA: {eta} {msg}"
            }
        } else {
            if color_config.is_enabled() {
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}"
            } else {
                "[{elapsed_precise}] [{bar:40}] {pos}/{len} ({percent}%) {msg}"
            }
        };

        let style = ProgressStyle::default_bar()
            .template(template)
            .expect("Invalid progress bar template")
            .progress_chars("=> ");

        bar.set_style(style);
        bar.enable_steady_tick(Duration::from_millis(self.config.update_interval_ms));

        self.main_bar = Some(bar);
    }

    /// Initializes multi-line progress bars for individual tasks
    fn initialize_multi_bars(&mut self) {
        // Multi-mode doesn't need initialization here
        // Bars are created on-demand as tasks are added
    }

    /// Adds a task to track
    ///
    /// # Arguments
    ///
    /// * `task_id` - Unique task identifier
    /// * `task_name` - Human-readable task name
    /// * `status` - Initial task status
    pub fn add_task(&mut self, task_id: String, task_name: String, status: TaskStatus) {
        // Store task start time if the task is in progress or about to start
        if status == TaskStatus::InProgress || status == TaskStatus::Pending {
            let mut start_times = self.task_start_times.lock().unwrap();
            start_times.entry(task_id.clone()).or_insert_with(Instant::now);
        }

        // Add to tracker
        self.tracker.add_task(task_id.clone(), status.clone());

        // Store task name
        let mut names = self.task_names.lock().unwrap();
        names.insert(task_id.clone(), task_name.clone());
        drop(names);

        // Create progress bar for multi mode
        if self.config.enable_multi {
            let bar = if let Some(multi) = &self.multi {
                multi.add(ProgressBar::new(100))
            } else {
                ProgressBar::new(100)
            };

            let color_config = BarColorConfig {
                inner: self.config.color_config,
            };

            let template = if self.config.enable_eta {
                if color_config.is_enabled() {
                    "{spinner:.cyan} {msg}: [{bar:20.blue/yellow}] {percent}% (ETA: {eta})"
                } else {
                    "{msg}: [{bar:20}] {percent}% (ETA: {eta})"
                }
            } else {
                if color_config.is_enabled() {
                    "{spinner:.cyan} {msg}: [{bar:20.blue/yellow}] {percent}%"
                } else {
                    "{msg}: [{bar:20}] {percent}%"
                }
            };

            let style = ProgressStyle::default_bar()
                .template(template)
                .expect("Invalid progress bar template")
                .progress_chars("=> ");

            bar.set_style(style);
            bar.set_message(task_name.clone());
            bar.set_position(0);

            self.task_bars.insert(task_id, bar);
        }

        // Update main bar if in single mode
        if let Some(main_bar) = &self.main_bar {
            main_bar.set_length(self.total_tasks as u64);
        }
    }

    /// Updates the progress of a task
    ///
    /// # Arguments
    ///
    /// * `task_id` - Task identifier
    /// * `status` - New task status
    /// * `percent` - Optional percentage complete (0-100)
    pub fn update_task(&mut self, task_id: &str, status: TaskStatus, percent: Option<u64>) {
        // Update tracker
        self.tracker.update_task(task_id, status.clone());

        // Track completion in metrics collector
        if status == TaskStatus::Completed {
            if let Some(start_time) = self.task_start_times.lock().unwrap().get(task_id) {
                let _elapsed = start_time.elapsed();
                // Create a temporary Task for metrics tracking
                let task = Task {
                    id: task_id.to_string(),
                    title: self.task_names.lock().unwrap().get(task_id).cloned().unwrap_or_default(),
                    description: String::new(),
                    status: status.clone(),
                    ..Default::default()
                };
                self.metrics_collector.track_task_completion(&task);
            }
        }

        // Update individual task bar if in multi mode
        if let Some(bar) = self.task_bars.get(task_id) {
            if let Some(p) = percent {
                bar.set_position(p);
            }

            // Update bar message based on status
            let names = self.task_names.lock().unwrap();
            if let Some(name) = names.get(task_id) {
                let status_text = match status {
                    TaskStatus::Pending => "Pending",
                    TaskStatus::InProgress => "In Progress",
                    TaskStatus::Completed => "Completed",
                    TaskStatus::Failed => "Failed",
                    TaskStatus::Blocked => "Blocked",
                };

                bar.set_message(format!("{} - {}", name, status_text));

                // Mark completed tasks as done
                if status == TaskStatus::Completed {
                    bar.finish_with_message(format!("{} - Completed", name));
                } else if status == TaskStatus::Failed {
                    bar.abandon_with_message(format!("{} - Failed", name));
                }
            }
        }

        // Update main bar if in single mode
        if let Some(main_bar) = &self.main_bar {
            let stats = self.tracker.get_stats();
            main_bar.set_position(stats.completed as u64);

            // Calculate ETA if enabled
            let eta_str = if self.config.enable_eta {
                self.calculate_remaining_eta().map(|eta| format_eta(eta))
            } else {
                None
            };

            // Update main bar message with current tasks
            let names = self.task_names.lock().unwrap();
            let current_tasks: String = names
                .iter()
                .filter(|(id, _)| {
                    self.tracker
                        .get_status(id)
                        .map_or(false, |s| s == TaskStatus::InProgress)
                })
                .map(|(_, name)| name.clone())
                .take(3)
                .collect::<Vec<String>>()
                .join(", ");

            if !current_tasks.is_empty() {
                let msg = if stats.in_progress > 3 {
                    format!("Running: {} (+ {} more)", current_tasks, stats.in_progress.saturating_sub(3))
                } else {
                    format!("Running: {}", current_tasks)
                };
                main_bar.set_message(msg);
            } else {
                let completion_msg = format!(
                    "Completed: {}/{} ({}%)",
                    stats.completed,
                    self.total_tasks,
                    if self.total_tasks > 0 {
                        (stats.completed * 100) / self.total_tasks
                    } else {
                        0
                    }
                );

                let msg = if let Some(eta) = eta_str {
                    format!("{} ETA: {}", completion_msg, eta)
                } else {
                    completion_msg
                };

                main_bar.set_message(msg);
            }
        }
    }

    /// Increments the main progress bar (single mode only)
    ///
    /// # Arguments
    ///
    /// * `task_id` - Task identifier that completed
    pub fn increment(&mut self, task_id: &str) {
        if let Some(main_bar) = &self.main_bar {
            main_bar.inc(1);
            self.tracker.update_task(task_id, TaskStatus::Completed);

            // Update completion percentage
            let stats = self.tracker.get_stats();
            if self.total_tasks > 0 {
                let percent = (stats.completed * 100) / self.total_tasks;
                main_bar.set_message(format!(
                    "Completed: {}/{} ({}%)",
                    stats.completed, self.total_tasks, percent
                ));
            }
        }
    }

    /// Returns the current progress statistics
    ///
    /// # Returns
    ///
    /// Task statistics including completed, in-progress, pending, etc.
    #[must_use]
    pub fn get_stats(&self) -> crate::progress::TaskStats {
        self.tracker.get_stats()
    }

    /// Returns the formatted progress summary
    ///
    /// # Returns
    ///
    /// A colorized string summarizing current progress
    #[must_use]
    pub fn format_summary(&self) -> String {
        self.tracker.format_summary()
    }

    /// Prints the current progress summary
    pub fn print_summary(&self) {
        self.tracker.print_summary();
    }

    /// Calculates the remaining ETA based on progress and historical data
    ///
    /// # Returns
    ///
    /// Optional duration estimate for remaining work
    #[must_use]
    pub fn calculate_remaining_eta(&self) -> Option<Duration> {
        let stats = self.tracker.get_stats();
        let remaining_tasks = self.total_tasks.saturating_sub(stats.completed);

        if remaining_tasks == 0 {
            return Some(Duration::ZERO);
        }

        // Calculate based on elapsed time if available
        if let Some(start_time) = self.overall_start_time {
            let elapsed = start_time.elapsed();
            if stats.completed > 0 {
                // Calculate average time per completed task
                let avg_per_task = elapsed / stats.completed as u32;
                let estimated_remaining = avg_per_task * remaining_tasks as u32;
                return Some(estimated_remaining);
            }
        }

        // Fall back to historical data if available
        if self.eta_calculator.is_some() {
            // This would require task complexity information
            // For now, return None if we can't estimate
            None
        } else {
            None
        }
    }

    /// Returns the elapsed time since the start of the task set
    ///
    /// # Returns
    ///
    /// Optional duration since start
    #[must_use]
    pub fn elapsed_time(&self) -> Option<Duration> {
        self.overall_start_time.map(|start| start.elapsed())
    }

    /// Returns the collected metrics
    ///
    /// # Returns
    ///
    /// Metrics collected during task execution
    #[must_use]
    pub fn get_metrics(&self) -> crate::progress::eta::Metrics {
        self.metrics_collector.get_metrics()
    }

    /// Finishes all progress bars
    pub fn finish(&self) {
        if let Some(main_bar) = &self.main_bar {
            main_bar.finish();
        }

        for bar in self.task_bars.values() {
            bar.finish();
        }
    }

    /// Abandons all progress bars (for error conditions)
    pub fn abandon(&self) {
        if let Some(main_bar) = &self.main_bar {
            main_bar.abandon();
        }

        for bar in self.task_bars.values() {
            bar.abandon();
        }
    }

    /// Clears all progress bars
    pub fn clear(&mut self) {
        if let Some(multi) = &self.multi {
            let _ = multi.clear();
        }

        self.task_bars.clear();
    }

    /// Sets a message for the main progress bar
    ///
    /// # Arguments
    ///
    /// * `msg` - Message to display
    pub fn set_message(&self, msg: String) {
        if let Some(main_bar) = &self.main_bar {
            main_bar.set_message(msg);
        }
    }
}

impl Drop for ProgressManager {
    fn drop(&mut self) {
        // Ensure all bars are properly finished
        self.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_manager_config_default() {
        let config = ProgressManagerConfig::default();
        assert!(config.enable_multi);
        assert!(config.enable_eta);
        assert_eq!(config.update_interval_ms, 100);
    }

    #[test]
    fn test_progress_manager_config_plain() {
        let config = ProgressManagerConfig::plain();
        assert!(!config.color_config.is_enabled());
        assert!(config.enable_multi);
        assert!(config.enable_eta);
    }

    #[test]
    fn test_progress_manager_config_with_multi() {
        let config = ProgressManagerConfig::new().with_multi(false);
        assert!(!config.enable_multi);
    }

    #[test]
    fn test_progress_manager_config_with_eta() {
        let config = ProgressManagerConfig::new().with_eta(false);
        assert!(!config.enable_eta);
    }

    #[test]
    fn test_progress_manager_config_with_update_interval() {
        let config = ProgressManagerConfig::new().with_update_interval(200);
        assert_eq!(config.update_interval_ms, 200);
    }

    #[test]
    fn test_progress_manager_new() {
        let manager = ProgressManager::new(None);
        let stats = manager.get_stats();
        assert_eq!(stats.total, 0);
    }

    #[test]
    fn test_progress_manager_new_with_config() {
        let config = ProgressManagerConfig::plain();
        let manager = ProgressManager::new(Some(config));
        // Just verify it doesn't panic
        let _ = manager.get_stats();
    }

    #[test]
    fn test_progress_manager_initialize_single() {
        let mut manager = ProgressManager::new(None);
        manager.initialize(10, ProgressBarType::Single);
        assert_eq!(manager.total_tasks, 10);
        assert!(manager.main_bar.is_some());
    }

    #[test]
    fn test_progress_manager_initialize_multi() {
        let mut manager = ProgressManager::new(Some(ProgressManagerConfig::new().with_multi(true)));
        manager.initialize(10, ProgressBarType::Multi);
        assert_eq!(manager.total_tasks, 10);
        assert!(manager.main_bar.is_none());
    }

    #[test]
    fn test_progress_manager_add_task() {
        let mut manager = ProgressManager::new(None);
        manager.initialize(2, ProgressBarType::Single);
        manager.add_task(
            "task-1".to_string(),
            "Test Task".to_string(),
            TaskStatus::Pending,
        );

        let stats = manager.get_stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.pending, 1);
    }

    #[test]
    fn test_progress_manager_update_task() {
        let mut manager = ProgressManager::new(None);
        manager.initialize(2, ProgressBarType::Single);
        manager.add_task(
            "task-1".to_string(),
            "Test Task".to_string(),
            TaskStatus::Pending,
        );
        manager.update_task("task-1", TaskStatus::Completed, Some(100));

        let status = manager.tracker.get_status("task-1");
        assert_eq!(status, Some(TaskStatus::Completed));
    }

    #[test]
    fn test_progress_manager_format_summary() {
        let mut manager = ProgressManager::new(None);
        manager.initialize(2, ProgressBarType::Single);
        manager.add_task(
            "task-1".to_string(),
            "Test Task".to_string(),
            TaskStatus::Completed,
        );

        let summary = manager.format_summary();
        assert!(!summary.is_empty());
    }

    #[test]
    fn test_progress_manager_get_stats() {
        let mut manager = ProgressManager::new(None);
        manager.initialize(3, ProgressBarType::Single);
        manager.add_task(
            "task-1".to_string(),
            "Task 1".to_string(),
            TaskStatus::Pending,
        );
        manager.add_task(
            "task-2".to_string(),
            "Task 2".to_string(),
            TaskStatus::InProgress,
        );
        manager.add_task(
            "task-3".to_string(),
            "Task 3".to_string(),
            TaskStatus::Completed,
        );

        let stats = manager.get_stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.in_progress, 1);
        assert_eq!(stats.completed, 1);
    }

    #[test]
    fn test_progress_bar_type_variants() {
        // Just verify the enum variants exist
        let _single = ProgressBarType::Single;
        let _multi = ProgressBarType::Multi;
    }
}
