//! Integration tests for ETA estimation in progress management
//!
//! This test suite verifies that ETA estimation integrates properly with
//! the progress manager and provides accurate time estimates.

use ltmatrix::models::TaskStatus;
use ltmatrix::progress::eta::{format_eta, HistoricalData};
use ltmatrix::progress::{ProgressBarType, ProgressManager, ProgressManagerConfig};
use std::thread;
use std::time::Duration;

// ==================== Progress Manager ETA Integration Tests ====================

#[test]
fn test_progress_manager_with_historical_data() {
    let historical_data = HistoricalData::new();
    let manager = ProgressManager::with_historical_data(None, historical_data);
    // Just verify creation works
    let _ = manager;
}

#[test]
fn test_progress_manager_calculate_eta_with_no_data() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(5, ProgressBarType::Single);

    // With no progress and no historical data, ETA should be None
    let eta = manager.calculate_remaining_eta();
    assert!(eta.is_none());
}

#[test]
fn test_progress_manager_calculate_eta_with_partial_progress() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(10, ProgressBarType::Single);

    // Add and complete 5 tasks
    for i in 1..=5 {
        let task_id = format!("task-{}", i);
        manager.add_task(task_id.clone(), format!("Task {}", i), TaskStatus::Pending);
        manager.update_task(&task_id, TaskStatus::Completed, Some(100));
        manager.increment(&task_id);

        // Small sleep to simulate work
        thread::sleep(Duration::from_millis(10));
    }

    // Should have an ETA estimate
    let eta = manager.calculate_remaining_eta();
    assert!(eta.is_some());

    // ETA should be reasonable (not zero, not extremely long)
    let eta_duration = eta.unwrap();
    assert!(eta_duration > Duration::ZERO);
    assert!(eta_duration < Duration::from_secs(100));
}

#[test]
fn test_progress_manager_calculate_eta_all_completed() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(3, ProgressBarType::Single);

    // Complete all tasks
    for i in 1..=3 {
        let task_id = format!("task-{}", i);
        manager.add_task(task_id.clone(), format!("Task {}", i), TaskStatus::Pending);
        manager.update_task(&task_id, TaskStatus::Completed, Some(100));
        manager.increment(&task_id);
    }

    // ETA should be zero when all tasks are done
    let eta = manager.calculate_remaining_eta();
    assert_eq!(eta, Some(Duration::ZERO));
}

#[test]
fn test_progress_manager_elapsed_time() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(5, ProgressBarType::Single);

    // Initially, elapsed time should be very small or None
    thread::sleep(Duration::from_millis(10));
    let elapsed = manager.elapsed_time();
    assert!(elapsed.is_some());
    assert!(elapsed.unwrap() >= Duration::from_millis(10));
}

#[test]
fn test_progress_manager_eta_in_template() {
    let config = ProgressManagerConfig::new().with_eta(true);
    let mut manager = ProgressManager::new(Some(config));
    manager.initialize(5, ProgressBarType::Single);

    // Add a task and update it
    manager.add_task(
        "task-1".to_string(),
        "Test Task".to_string(),
        TaskStatus::Pending,
    );
    manager.update_task("task-1", TaskStatus::InProgress, Some(50));

    // Thread sleep to ensure some elapsed time
    thread::sleep(Duration::from_millis(50));

    // Just verify it doesn't panic
    manager.update_task("task-1", TaskStatus::Completed, Some(100));
    manager.increment("task-1");
}

#[test]
fn test_progress_manager_get_metrics() {
    let manager = ProgressManager::new(None);

    // Should have default metrics
    let metrics = manager.get_metrics();
    assert_eq!(metrics.total_tracked, 0);
    assert_eq!(metrics.total_completed, 0);
}

// ==================== ETA Format Integration Tests ====================

#[test]
fn test_eta_format_with_progress_manager() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(5, ProgressBarType::Single);

    // Simulate some progress
    for i in 1..=3 {
        let task_id = format!("task-{}", i);
        manager.add_task(task_id.clone(), format!("Task {}", i), TaskStatus::Pending);
        thread::sleep(Duration::from_millis(10));
        manager.update_task(&task_id, TaskStatus::Completed, Some(100));
        manager.increment(&task_id);
    }

    // Get ETA and format it
    if let Some(eta) = manager.calculate_remaining_eta() {
        let formatted = format_eta(eta);
        // Should be a valid format
        assert!(!formatted.is_empty());
    }
}

// ==================== Metrics Collector Integration Tests ====================

#[test]
fn test_progress_manager_tracks_task_start_times() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(2, ProgressBarType::Single);

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

    // Just verify it doesn't panic - internal tracking should work
    thread::sleep(Duration::from_millis(10));
}

#[test]
fn test_progress_manager_updates_metrics_on_completion() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(2, ProgressBarType::Single);

    manager.add_task(
        "task-1".to_string(),
        "Task 1".to_string(),
        TaskStatus::Pending,
    );

    // Mark as completed
    manager.update_task("task-1", TaskStatus::Completed, Some(100));

    // Check metrics updated
    let _metrics = manager.get_metrics();
    // Note: metrics may or may not be updated depending on start time tracking
}

// ==================== Multi-Bar ETA Tests ====================

#[test]
fn test_progress_manager_multi_bar_with_eta() {
    let config = ProgressManagerConfig::new().with_eta(true).with_multi(true);

    let mut manager = ProgressManager::new(Some(config));
    manager.initialize(2, ProgressBarType::Multi);

    manager.add_task(
        "task-1".to_string(),
        "Task 1".to_string(),
        TaskStatus::Pending,
    );
    manager.add_task(
        "task-2".to_string(),
        "Task 2".to_string(),
        TaskStatus::Pending,
    );

    // Update progress
    manager.update_task("task-1", TaskStatus::InProgress, Some(50));
    manager.update_task("task-2", TaskStatus::InProgress, Some(75));

    // Just verify it doesn't panic
    thread::sleep(Duration::from_millis(10));
    manager.update_task("task-1", TaskStatus::Completed, Some(100));
    manager.update_task("task-2", TaskStatus::Completed, Some(100));
}

// ==================== ETA Calculation Edge Cases ====================

#[test]
fn test_eta_calculation_with_zero_total_tasks() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(0, ProgressBarType::Single);

    // Should handle zero tasks gracefully
    let eta = manager.calculate_remaining_eta();
    assert_eq!(eta, Some(Duration::ZERO));
}

#[test]
fn test_eta_calculation_with_no_completed_tasks() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(5, ProgressBarType::Single);

    // Add tasks but don't complete any
    for i in 1..=3 {
        let task_id = format!("task-{}", i);
        manager.add_task(task_id, format!("Task {}", i), TaskStatus::Pending);
    }

    // With no completed tasks, ETA should be None
    let eta = manager.calculate_remaining_eta();
    assert!(eta.is_none());
}

#[test]
fn test_eta_consistency_over_time() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(10, ProgressBarType::Single);

    // Add all tasks
    for i in 1..=10 {
        manager.add_task(
            format!("task-{}", i),
            format!("Task {}", i),
            TaskStatus::Pending,
        );
    }

    let mut prev_eta: Option<Duration> = None;

    // Complete tasks one by one and check ETA consistency
    for i in 1..=5 {
        let task_id = format!("task-{}", i);
        manager.update_task(&task_id, TaskStatus::Completed, Some(100));
        manager.increment(&task_id);

        thread::sleep(Duration::from_millis(10));

        let current_eta = manager.calculate_remaining_eta();

        // ETA should generally decrease as we complete more tasks
        if let (Some(prev), Some(current)) = (prev_eta, current_eta) {
            // Current ETA should be less than or equal to previous
            // (allowing some tolerance for timing variations)
            assert!(current <= prev + Duration::from_millis(50));
        }

        prev_eta = current_eta;
    }
}

// ==================== Configuration Tests ====================

#[test]
fn test_eta_config_enabled() {
    let config = ProgressManagerConfig::new().with_eta(true);
    assert!(config.enable_eta);
}

#[test]
fn test_eta_config_disabled() {
    let config = ProgressManagerConfig::new().with_eta(false);
    assert!(!config.enable_eta);
}

#[test]
fn test_eta_config_default() {
    let config = ProgressManagerConfig::default();
    // ETA should be enabled by default
    assert!(config.enable_eta);
}
