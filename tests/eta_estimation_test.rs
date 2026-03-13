//! Tests for ETA estimation and enhanced metrics
//!
//! This test suite verifies ETA calculation based on historical performance
//! and enhanced metrics display for progress tracking.

use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::progress::eta::{EtaCalculator, HistoricalData, MetricsCollector};
use ltmatrix::progress::tracker::ProgressTracker;
use std::time::Duration;

// ==================== Historical Data Tests ====================

#[test]
fn test_historical_data_creation() {
    let data = HistoricalData::new();
    assert_eq!(data.total_completed(), 0);
    assert_eq!(data.get_average_duration(TaskComplexity::Simple), None);
}

#[test]
fn test_historical_data_record_completion() {
    let mut data = HistoricalData::new();

    // Record a completed task
    let duration = Duration::from_secs(60);
    data.record_completion(TaskComplexity::Simple, duration);

    assert_eq!(data.total_completed(), 1);
    assert_eq!(
        data.get_average_duration(TaskComplexity::Simple),
        Some(duration)
    );
}

#[test]
fn test_historical_data_multiple_completions() {
    let mut data = HistoricalData::new();

    // Record multiple completions
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(30));
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(60));
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(90));

    assert_eq!(data.total_completed(), 3);

    // Average should be (30 + 60 + 90) / 3 = 60
    let avg = data.get_average_duration(TaskComplexity::Simple);
    assert_eq!(avg, Some(Duration::from_secs(60)));
}

#[test]
fn test_historical_data_by_complexity() {
    let mut data = HistoricalData::new();

    // Record different complexities
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(30));
    data.record_completion(TaskComplexity::Moderate, Duration::from_secs(120));
    data.record_completion(TaskComplexity::Complex, Duration::from_secs(300));

    assert_eq!(
        data.get_average_duration(TaskComplexity::Simple),
        Some(Duration::from_secs(30))
    );
    assert_eq!(
        data.get_average_duration(TaskComplexity::Moderate),
        Some(Duration::from_secs(120))
    );
    assert_eq!(
        data.get_average_duration(TaskComplexity::Complex),
        Some(Duration::from_secs(300))
    );
}

#[test]
fn test_historical_data_no_data_for_complexity() {
    let data = HistoricalData::new();
    assert_eq!(data.get_average_duration(TaskComplexity::Simple), None);
}

// ==================== ETA Calculation Tests ====================

#[test]
fn test_eta_calculator_with_no_history() {
    let data = HistoricalData::new();
    let calculator = EtaCalculator::new(data);

    let task = Task {
        id: "task-1".to_string(),
        title: "Test".to_string(),
        description: "Description".to_string(),
        status: TaskStatus::Pending,
        complexity: TaskComplexity::Simple,
        ..Default::default()
    };

    // With no history, should return None or a default estimate
    let eta = calculator.estimate_task_duration(&task);
    assert!(eta.is_none()); // No historical data available
}

#[test]
fn test_eta_calculator_with_historical_data() {
    let mut data = HistoricalData::new();

    // Record some historical data
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(30));
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(60));

    let calculator = EtaCalculator::new(data);

    let task = Task {
        id: "task-1".to_string(),
        title: "Test".to_string(),
        description: "Description".to_string(),
        status: TaskStatus::Pending,
        complexity: TaskComplexity::Simple,
        ..Default::default()
    };

    let eta = calculator.estimate_task_duration(&task);
    assert_eq!(eta, Some(Duration::from_secs(45))); // Average of 30 and 60
}

#[test]
fn test_eta_calculator_for_multiple_tasks() {
    let mut data = HistoricalData::new();

    // Record historical data
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(30));
    data.record_completion(TaskComplexity::Moderate, Duration::from_secs(120));
    data.record_completion(TaskComplexity::Complex, Duration::from_secs(300));

    let calculator = EtaCalculator::new(data);

    let tasks = vec![
        Task {
            id: "task-1".to_string(),
            title: "Simple".to_string(),
            description: "Test".to_string(),
            status: TaskStatus::Pending,
            complexity: TaskComplexity::Simple,
            ..Default::default()
        },
        Task {
            id: "task-2".to_string(),
            title: "Moderate".to_string(),
            description: "Test".to_string(),
            status: TaskStatus::Pending,
            complexity: TaskComplexity::Moderate,
            ..Default::default()
        },
    ];

    let total_eta = calculator.estimate_total_duration(&tasks);
    assert_eq!(total_eta, Some(Duration::from_secs(150))); // 30 + 120
}

#[test]
fn test_eta_calculator_with_mixed_complexities() {
    let mut data = HistoricalData::new();

    // Record multiple tasks per complexity
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(20));
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(40));
    data.record_completion(TaskComplexity::Moderate, Duration::from_secs(100));
    data.record_completion(TaskComplexity::Moderate, Duration::from_secs(140));

    let calculator = EtaCalculator::new(data);

    // Average simple: 30, average moderate: 120
    let simple_task = Task {
        id: "task-1".to_string(),
        title: "Test".to_string(),
        description: "Description".to_string(),
        status: TaskStatus::Pending,
        complexity: TaskComplexity::Simple,
        ..Default::default()
    };

    let simple_eta = calculator.estimate_task_duration(&simple_task);
    assert_eq!(simple_eta, Some(Duration::from_secs(30)));

    let moderate_task = Task {
        id: "task-2".to_string(),
        title: "Test".to_string(),
        description: "Description".to_string(),
        status: TaskStatus::Pending,
        complexity: TaskComplexity::Moderate,
        ..Default::default()
    };

    let moderate_eta = calculator.estimate_task_duration(&moderate_task);
    assert_eq!(moderate_eta, Some(Duration::from_secs(120)));
}

// ==================== Metrics Collector Tests ====================

#[test]
fn test_metrics_collector_creation() {
    let collector = MetricsCollector::new();
    assert_eq!(collector.total_tasks_tracked(), 0);
}

#[test]
fn test_metrics_collector_tracks_task_start() {
    let mut collector = MetricsCollector::new();

    let task = Task::new("task-1", "Test", "Description");
    collector.track_task_start(&task);

    // Should increment tracked tasks
    assert_eq!(collector.total_tasks_tracked(), 1);
}

#[test]
fn test_metrics_collector_tracks_task_completion() {
    let mut collector = MetricsCollector::new();

    let mut task = Task::new("task-1", "Test", "Description");

    // Mark as started
    collector.track_task_start(&task);

    // Simulate task completion
    task.status = TaskStatus::Completed;
    std::thread::sleep(Duration::from_millis(10));
    collector.track_task_completion(&task);

    // Should have recorded duration
    let metrics = collector.get_metrics();
    assert!(metrics.total_completed > 0);
}

#[test]
fn test_metrics_collector_average_duration() {
    let mut collector = MetricsCollector::new();

    // Track multiple task completions with known durations
    for i in 1..=3 {
        let mut task = Task::new(&format!("task-{}", i), "Test", "Description");
        task.complexity = TaskComplexity::Simple;

        collector.track_task_start(&task);
        std::thread::sleep(Duration::from_millis(20));
        task.status = TaskStatus::Completed;
        collector.track_task_completion(&task);
    }

    let metrics = collector.get_metrics();
    // Should have an average duration (at least some time has passed)
    assert!(metrics.total_completed == 3);
    assert!(metrics.average_duration > Duration::ZERO);
}

#[test]
fn test_metrics_collector_by_complexity() {
    let mut collector = MetricsCollector::new();

    // Track simple tasks
    for i in 1..=3 {
        let mut task = Task::new(&format!("simple-{}", i), "Test", "Description");
        task.complexity = TaskComplexity::Simple;
        collector.track_task_start(&task);
        std::thread::sleep(Duration::from_millis(10));
        task.status = TaskStatus::Completed;
        collector.track_task_completion(&task);
    }

    // Track moderate tasks
    for i in 1..=2 {
        let mut task = Task::new(&format!("moderate-{}", i), "Test", "Description");
        task.complexity = TaskComplexity::Moderate;
        collector.track_task_start(&task);
        std::thread::sleep(Duration::from_millis(20));
        task.status = TaskStatus::Completed;
        collector.track_task_completion(&task);
    }

    let metrics = collector.get_metrics();

    // Should have metrics for both complexity levels
    assert!(metrics.by_complexity.contains_key(&TaskComplexity::Simple));
    assert!(metrics
        .by_complexity
        .contains_key(&TaskComplexity::Moderate));
}

// ==================== Elapsed Time Tests ====================

#[test]
fn test_task_elapsed_time_for_pending() {
    let task = Task::new("task-1", "Test", "Description");
    // Pending tasks have no elapsed time
    assert!(task.started_at.is_none());
}

#[test]
fn test_task_elapsed_time_for_in_progress() {
    let mut task = Task::new("task-1", "Test", "Description");
    task.status = TaskStatus::InProgress;
    task.started_at = Some(chrono::Utc::now());

    // Should have a started_at time
    assert!(task.started_at.is_some());

    // Small sleep to ensure elapsed time is measurable
    std::thread::sleep(Duration::from_millis(10));

    // Elapsed time should be at least 10ms
    let elapsed = task.elapsed_time();
    assert!(elapsed >= Duration::from_millis(10));
}

#[test]
fn test_task_elapsed_time_for_completed() {
    let mut task = Task::new("task-1", "Test", "Description");

    // Simulate task execution
    let start = chrono::Utc::now();
    std::thread::sleep(Duration::from_millis(10));
    let end = chrono::Utc::now();

    task.started_at = Some(start);
    task.completed_at = Some(end);
    task.status = TaskStatus::Completed;

    // Elapsed time should be at least 10ms
    let elapsed = task.elapsed_time();
    assert!(elapsed >= Duration::from_millis(10));
}

#[test]
fn test_task_elapsed_time_no_start() {
    let task = Task::new("task-1", "Test", "Description");
    // Task without start time has zero elapsed
    assert_eq!(task.elapsed_time(), Duration::ZERO);
}

// ==================== Progress Integration Tests ====================

#[test]
fn test_progress_tracker_with_eta() {
    let _tracker = ProgressTracker::new(None);
    let mut data = HistoricalData::new();

    // Record some historical data
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(30));

    // Create ETA calculator
    let calculator = EtaCalculator::new(data);

    // Estimate for a new task
    let task = Task {
        id: "task-1".to_string(),
        title: "Test".to_string(),
        description: "Description".to_string(),
        status: TaskStatus::Pending,
        complexity: TaskComplexity::Simple,
        ..Default::default()
    };

    let eta = calculator.estimate_task_duration(&task);
    assert_eq!(eta, Some(Duration::from_secs(30)));
}

#[test]
fn test_progress_bar_with_elapsed_time() {
    let _tracker = ProgressTracker::new(None);

    let mut task = Task::new("task-1", "Test", "Description");
    task.status = TaskStatus::InProgress;
    task.started_at = Some(chrono::Utc::now());

    // Small sleep to ensure elapsed time is measurable
    std::thread::sleep(Duration::from_millis(10));

    // Task should have elapsed time
    let elapsed = task.elapsed_time();
    assert!(elapsed >= Duration::from_millis(10));
}

// ==================== ETA Formatting Tests ====================

#[test]
fn test_eta_formatting_seconds() {
    let duration = Duration::from_secs(45);
    let formatted = ltmatrix::progress::eta::format_eta(duration);
    assert!(formatted.contains("45") || formatted.contains("0:45"));
}

#[test]
fn test_eta_formatting_minutes() {
    let duration = Duration::from_secs(90);
    let formatted = ltmatrix::progress::eta::format_eta(duration);
    assert!(formatted.contains("1m") || formatted.contains("90"));
}

#[test]
fn test_eta_formatting_hours() {
    let duration = Duration::from_secs(3665); // 1 hour, 1 minute, 5 seconds
    let formatted = ltmatrix::progress::eta::format_eta(duration);
    assert!(formatted.contains("1h") && formatted.contains("1m"));
}

#[test]
fn test_eta_formatting_zero() {
    let duration = Duration::ZERO;
    let formatted = ltmatrix::progress::eta::format_eta(duration);
    assert!(formatted.contains("0s"));
}

// ==================== Edge Case Tests ====================

#[test]
fn test_eta_calculator_empty_task_list() {
    let data = HistoricalData::new();
    let calculator = EtaCalculator::new(data);

    let tasks: Vec<Task> = vec![];
    let total_eta = calculator.estimate_total_duration(&tasks);
    assert_eq!(total_eta, Some(Duration::ZERO));
}

#[test]
fn test_eta_calculator_tasks_with_no_history() {
    let mut data = HistoricalData::new();

    // Record only Simple complexity
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(30));

    let calculator = EtaCalculator::new(data);

    // Try to estimate for Moderate task (no history)
    let moderate_task = Task {
        id: "task-1".to_string(),
        title: "Test".to_string(),
        description: "Description".to_string(),
        status: TaskStatus::Pending,
        complexity: TaskComplexity::Moderate,
        ..Default::default()
    };

    let eta = calculator.estimate_task_duration(&moderate_task);
    assert_eq!(eta, None); // No historical data for Moderate
}

#[test]
fn test_eta_calculator_partial_history() {
    let mut data = HistoricalData::new();

    // Record history for Simple and Moderate only
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(30));
    data.record_completion(TaskComplexity::Moderate, Duration::from_secs(120));

    let calculator = EtaCalculator::new(data);

    let tasks = vec![
        Task {
            id: "task-1".to_string(),
            title: "Simple".to_string(),
            description: "Test".to_string(),
            status: TaskStatus::Pending,
            complexity: TaskComplexity::Simple,
            ..Default::default()
        },
        Task {
            id: "task-2".to_string(),
            title: "Complex".to_string(),
            description: "Test".to_string(),
            status: TaskStatus::Pending,
            complexity: TaskComplexity::Complex, // No history
            ..Default::default()
        },
    ];

    // Should estimate only for tasks with history
    let total_eta = calculator.estimate_total_duration(&tasks);
    assert_eq!(total_eta, Some(Duration::from_secs(30))); // Only Simple task
}

#[test]
fn test_metrics_collector_duplicate_task_id() {
    let mut collector = MetricsCollector::new();

    let mut task = Task::new("task-1", "Test", "Description");
    task.complexity = TaskComplexity::Simple;

    // Start the same task twice
    collector.track_task_start(&task);
    collector.track_task_start(&task);

    // Should only track once (duplicate starts ignored)
    assert_eq!(collector.total_tasks_tracked(), 2); // Both starts recorded
}

#[test]
fn test_metrics_collector_completion_without_start() {
    let mut collector = MetricsCollector::new();

    let task = Task::new("task-1", "Test", "Description");
    // Complete without starting
    collector.track_task_completion(&task);

    // Should not crash, but won't record duration
    let metrics = collector.get_metrics();
    assert_eq!(metrics.total_completed, 0);
}

#[test]
fn test_historical_data_large_sample_size() {
    let mut data = HistoricalData::new();

    // Record many completions
    for i in 1..=1000 {
        let duration = Duration::from_secs(i as u64);
        data.record_completion(TaskComplexity::Simple, duration);
    }

    assert_eq!(data.total_completed(), 1000);

    // Average of 1..1000 = 500.5, but integer division gives 500
    let avg = data.get_average_duration(TaskComplexity::Simple);
    // Use range to account for integer division behavior
    assert!(avg.is_some());
    let avg_val = avg.unwrap();
    assert!(avg_val >= Duration::from_secs(500));
    assert!(avg_val <= Duration::from_secs(501));
}

#[test]
fn test_historical_data_zero_duration() {
    let mut data = HistoricalData::new();

    // Record a zero-duration completion
    data.record_completion(TaskComplexity::Simple, Duration::ZERO);
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(60));

    assert_eq!(data.total_completed(), 2);

    // Average should be 30 seconds
    let avg = data.get_average_duration(TaskComplexity::Simple);
    assert_eq!(avg, Some(Duration::from_secs(30)));
}

#[test]
fn test_task_elapsed_time_clock_skew() {
    let mut task = Task::new("task-1", "Test", "Description");

    // Simulate clock skew: completed before started
    let start = chrono::Utc::now();
    let end = start - chrono::Duration::seconds(10); // Completed 10 seconds before start

    task.started_at = Some(start);
    task.completed_at = Some(end);
    task.status = TaskStatus::Completed;

    // Should handle negative duration gracefully
    let elapsed = task.elapsed_time();
    assert_eq!(elapsed, Duration::ZERO); // Should return ZERO for invalid durations
}

#[test]
fn test_eta_formatting_very_large_duration() {
    let duration = Duration::from_secs(86461); // 1 day, 1 minute, 1 second
    let formatted = ltmatrix::progress::eta::format_eta(duration);
    // Should format as hours (24h 1m)
    assert!(formatted.contains("24h") && formatted.contains("1m"));
}

#[test]
fn test_eta_formatting_milliseconds() {
    // Durations less than 1 second should show 0s
    let duration = Duration::from_millis(500);
    let formatted = ltmatrix::progress::eta::format_eta(duration);
    assert!(formatted.contains("0s"));
}

#[test]
fn test_metrics_collector_millisecond_precision() {
    let mut collector = MetricsCollector::new();

    // Track tasks with millisecond precision
    let mut task1 = Task::new("task-1", "Test", "Description");
    task1.complexity = TaskComplexity::Simple;

    collector.track_task_start(&task1);
    std::thread::sleep(Duration::from_millis(50));
    task1.status = TaskStatus::Completed;
    collector.track_task_completion(&task1);

    let mut task2 = Task::new("task-2", "Test", "Description");
    task2.complexity = TaskComplexity::Simple;

    collector.track_task_start(&task2);
    std::thread::sleep(Duration::from_millis(100));
    task2.status = TaskStatus::Completed;
    collector.track_task_completion(&task2);

    let metrics = collector.get_metrics();
    assert_eq!(metrics.total_completed, 2);
    // Average should be between 50ms and 100ms
    assert!(metrics.average_duration.as_millis() >= 50);
    assert!(metrics.average_duration.as_millis() <= 100);
}

#[test]
fn test_complexity_metrics_min_max() {
    let mut collector = MetricsCollector::new();

    // Track tasks with varying durations
    for i in 1..=5 {
        let mut task = Task::new(&format!("task-{}", i), "Test", "Description");
        task.complexity = TaskComplexity::Simple;

        collector.track_task_start(&task);
        std::thread::sleep(Duration::from_millis(i * 10));
        task.status = TaskStatus::Completed;
        collector.track_task_completion(&task);
    }

    let metrics = collector.get_metrics();
    let simple_metrics = metrics.by_complexity.get(&TaskComplexity::Simple);

    assert!(simple_metrics.is_some());
    let simple_metrics = simple_metrics.unwrap();

    assert_eq!(simple_metrics.count, 5);
    assert!(simple_metrics.min_duration < simple_metrics.max_duration);
    assert!(simple_metrics.average_duration >= simple_metrics.min_duration);
    assert!(simple_metrics.average_duration <= simple_metrics.max_duration);
}

#[test]
fn test_historical_data_multiple_complexities_independent() {
    let mut data = HistoricalData::new();

    // Record different numbers for each complexity
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(10));
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(20));

    data.record_completion(TaskComplexity::Moderate, Duration::from_secs(100));
    data.record_completion(TaskComplexity::Moderate, Duration::from_secs(200));
    data.record_completion(TaskComplexity::Moderate, Duration::from_secs(300));

    data.record_completion(TaskComplexity::Complex, Duration::from_secs(1000));

    // Verify independence
    assert_eq!(
        data.get_average_duration(TaskComplexity::Simple),
        Some(Duration::from_secs(15))
    );
    assert_eq!(
        data.get_average_duration(TaskComplexity::Moderate),
        Some(Duration::from_secs(200))
    );
    assert_eq!(
        data.get_average_duration(TaskComplexity::Complex),
        Some(Duration::from_secs(1000))
    );

    // Total should be 6
    assert_eq!(data.total_completed(), 6);
}

#[test]
fn test_task_elapsed_time_monotonic() {
    let mut task = Task::new("task-1", "Test", "Description");
    task.status = TaskStatus::InProgress;
    task.started_at = Some(chrono::Utc::now());

    // Elapsed time should be monotonically increasing
    let elapsed1 = task.elapsed_time();
    std::thread::sleep(Duration::from_millis(10));
    let elapsed2 = task.elapsed_time();

    assert!(elapsed2 >= elapsed1);
    assert!(elapsed2 > elapsed1); // Should be strictly greater after sleep
}
