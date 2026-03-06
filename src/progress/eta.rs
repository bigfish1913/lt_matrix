//! ETA estimation and metrics collection
//!
//! This module provides ETA calculation based on historical task performance
//! and enhanced metrics collection for progress tracking.

use crate::models::{Task, TaskComplexity};
use std::collections::HashMap;
use std::time::Duration;

/// Historical data about completed tasks
#[derive(Debug, Clone, Default)]
pub struct HistoricalData {
    /// Completed task durations by complexity
    completed_by_complexity: HashMap<TaskComplexity, Vec<Duration>>,
}

impl HistoricalData {
    /// Creates a new empty historical data tracker
    #[must_use]
    pub fn new() -> Self {
        HistoricalData {
            completed_by_complexity: HashMap::new(),
        }
    }

    /// Records a task completion with its duration
    pub fn record_completion(&mut self, complexity: TaskComplexity, duration: Duration) {
        self.completed_by_complexity
            .entry(complexity)
            .or_insert_with(Vec::new)
            .push(duration);
    }

    /// Returns the total number of completed tasks tracked
    #[must_use]
    pub fn total_completed(&self) -> usize {
        self.completed_by_complexity
            .values()
            .map(|v| v.len())
            .sum()
    }

    /// Returns the average duration for a given complexity level
    #[must_use]
    pub fn get_average_duration(&self, complexity: TaskComplexity) -> Option<Duration> {
        let durations = self.completed_by_complexity.get(&complexity)?;
        if durations.is_empty() {
            return None;
        }

        let total: Duration = durations.iter().sum();
        Some(total / durations.len() as u32)
    }
}

/// ETA calculator based on historical performance
#[derive(Debug, Clone)]
pub struct EtaCalculator {
    /// Historical data for estimation
    historical: HistoricalData,
}

impl EtaCalculator {
    /// Creates a new ETA calculator with historical data
    #[must_use]
    pub fn new(historical: HistoricalData) -> Self {
        EtaCalculator { historical }
    }

    /// Estimates the duration for a single task
    #[must_use]
    pub fn estimate_task_duration(&self, task: &Task) -> Option<Duration> {
        self.historical.get_average_duration(task.complexity)
    }

    /// Estimates the total duration for multiple tasks
    #[must_use]
    pub fn estimate_total_duration(&self, tasks: &[Task]) -> Option<Duration> {
        if tasks.is_empty() {
            return Some(Duration::ZERO);
        }

        let mut total = Duration::ZERO;
        let mut has_data = false;

        for task in tasks {
            if let Some(duration) = self.estimate_task_duration(task) {
                total += duration;
                has_data = true;
            }
        }

        if has_data {
            Some(total)
        } else {
            None
        }
    }
}

/// Metrics collected during task execution
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    /// Total number of tasks tracked
    pub total_tracked: usize,

    /// Number of completed tasks
    pub total_completed: usize,

    /// Average duration across all completed tasks
    pub average_duration: Duration,

    /// Metrics broken down by complexity
    pub by_complexity: HashMap<TaskComplexity, ComplexityMetrics>,
}

/// Metrics for a specific complexity level
#[derive(Debug, Clone, Default)]
pub struct ComplexityMetrics {
    /// Number of tasks completed
    pub count: usize,

    /// Average duration for this complexity
    pub average_duration: Duration,

    /// Minimum duration
    pub min_duration: Duration,

    /// Maximum duration
    pub max_duration: Duration,
}

/// Collects and aggregates task execution metrics
#[derive(Debug, Clone, Default)]
pub struct MetricsCollector {
    /// Task start times
    start_times: HashMap<String, std::time::Instant>,

    /// Collected metrics
    metrics: Metrics,
}

impl MetricsCollector {
    /// Creates a new metrics collector
    #[must_use]
    pub fn new() -> Self {
        MetricsCollector {
            start_times: HashMap::new(),
            metrics: Metrics::default(),
        }
    }

    /// Tracks the start of a task
    pub fn track_task_start(&mut self, task: &Task) {
        self.start_times.insert(task.id.clone(), std::time::Instant::now());
        self.metrics.total_tracked += 1;
    }

    /// Tracks the completion of a task
    pub fn track_task_completion(&mut self, task: &Task) {
        if let Some(start_time) = self.start_times.get(&task.id) {
            let duration = start_time.elapsed();
            self.update_metrics(task.complexity, duration);
            self.metrics.total_completed += 1;
        }
    }

    /// Updates metrics with a new completion
    fn update_metrics(&mut self, complexity: TaskComplexity, duration: Duration) {
        // Update overall average using milliseconds for precision
        let total_completed = self.metrics.total_completed;
        let new_average = if total_completed > 0 {
            let current_ms = self.metrics.average_duration.as_millis();
            let new_ms = duration.as_millis();
            let total_ms = current_ms * total_completed as u128 + new_ms;
            let avg_ms = total_ms / (total_completed as u128 + 1);
            Duration::from_millis(avg_ms as u64)
        } else {
            duration
        };
        self.metrics.average_duration = new_average;

        // Update complexity-specific metrics
        let metrics = self
            .metrics
            .by_complexity
            .entry(complexity)
            .or_insert_with(ComplexityMetrics::default);

        metrics.count += 1;
        let count = metrics.count;
        let current_ms = metrics.average_duration.as_millis();
        let new_ms = duration.as_millis();
        let total_ms = current_ms * (count - 1) as u128 + new_ms;
        let avg_ms = total_ms / count as u128;
        metrics.average_duration = Duration::from_millis(avg_ms as u64);
        metrics.min_duration = metrics.min_duration.min(duration);
        metrics.max_duration = metrics.max_duration.max(duration);
    }

    /// Returns the collected metrics
    #[must_use]
    pub fn get_metrics(&self) -> Metrics {
        self.metrics.clone()
    }

    /// Returns the total number of tasks tracked
    #[must_use]
    pub fn total_tasks_tracked(&self) -> usize {
        self.metrics.total_tracked
    }
}

/// Formats a duration as a human-readable ETA string
#[must_use]
pub fn format_eta(duration: Duration) -> String {
    let secs = duration.as_secs();

    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        let mins = secs / 60;
        let remaining_secs = secs % 60;
        if remaining_secs > 0 {
            format!("{}m {}s", mins, remaining_secs)
        } else {
            format!("{}m", mins)
        }
    } else {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        if mins > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}h", hours)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_historical_data_new() {
        let data = HistoricalData::new();
        assert_eq!(data.total_completed(), 0);
    }

    #[test]
    fn test_historical_data_record_completion() {
        let mut data = HistoricalData::new();
        data.record_completion(TaskComplexity::Simple, Duration::from_secs(60));
        assert_eq!(data.total_completed(), 1);
    }

    #[test]
    fn test_eta_calculator_new() {
        let data = HistoricalData::new();
        let calculator = EtaCalculator::new(data);
        // Just verify creation works
        let _ = calculator;
    }

    #[test]
    fn test_eta_calculator_estimate_task_duration() {
        let mut data = HistoricalData::new();
        data.record_completion(TaskComplexity::Simple, Duration::from_secs(30));
        data.record_completion(TaskComplexity::Simple, Duration::from_secs(60));

        let calculator = EtaCalculator::new(data);

        let mut task = Task::new("task-1", "Test", "Description");
        task.complexity = TaskComplexity::Simple;

        let eta = calculator.estimate_task_duration(&task);
        assert_eq!(eta, Some(Duration::from_secs(45)));
    }

    #[test]
    fn test_metrics_collector_new() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.total_tasks_tracked(), 0);
    }

    #[test]
    fn test_format_eta_seconds() {
        let formatted = format_eta(Duration::from_secs(45));
        assert!(formatted.contains("45s"));
    }

    #[test]
    fn test_format_eta_minutes() {
        let formatted = format_eta(Duration::from_secs(90));
        assert!(formatted.contains("1m") && formatted.contains("30s"));
    }

    #[test]
    fn test_format_eta_hours() {
        let formatted = format_eta(Duration::from_secs(3665));
        assert!(formatted.contains("1h") && formatted.contains("1m"));
    }
}
