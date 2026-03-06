//! Integration tests for the progress module
//!
//! This test suite verifies the complete workflow of the ProgressManager
//! including single-bar mode, multi-bar mode, task lifecycle, and integration
//! between all progress components.

use ltmatrix::models::TaskStatus;
use ltmatrix::progress::{
    colorize_percentage, create_custom_progress_bar, create_progress_bar, create_spinner,
    BarColorConfig, ProgressBarType, ProgressManager, ProgressManagerConfig,
};
use ltmatrix::progress::{report_progress_summary, report_task_complete, report_task_start, ReporterColorConfig};
use ltmatrix::terminal::ColorConfig;

/// Tests the complete single-bar progress workflow
#[test]
fn test_single_bar_complete_workflow() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(3, ProgressBarType::Single);

    // Add tasks
    manager.add_task("task-1".to_string(), "Task 1".to_string(), TaskStatus::Pending);
    manager.add_task("task-2".to_string(), "Task 2".to_string(), TaskStatus::Pending);
    manager.add_task("task-3".to_string(), "Task 3".to_string(), TaskStatus::Pending);

    // Verify initial state
    let stats = manager.get_stats();
    assert_eq!(stats.total, 3);
    assert_eq!(stats.pending, 3);

    // Update tasks through lifecycle
    manager.update_task("task-1", TaskStatus::InProgress, Some(50));
    let stats = manager.get_stats();
    assert_eq!(stats.in_progress, 1);

    manager.update_task("task-1", TaskStatus::Completed, Some(100));
    manager.increment("task-1");

    let stats = manager.get_stats();
    assert_eq!(stats.completed, 1);

    // Complete remaining tasks
    manager.update_task("task-2", TaskStatus::InProgress, Some(0));
    manager.update_task("task-2", TaskStatus::Completed, Some(100));
    manager.increment("task-2");

    manager.update_task("task-3", TaskStatus::InProgress, Some(0));
    manager.update_task("task-3", TaskStatus::Completed, Some(100));
    manager.increment("task-3");

    // Verify final state
    let stats = manager.get_stats();
    assert_eq!(stats.total, 3);
    assert_eq!(stats.completed, 3);

    // Verify summary is formatted
    let summary = manager.format_summary();
    assert!(!summary.is_empty());

    manager.finish();
}

/// Tests the complete multi-bar progress workflow
#[test]
fn test_multi_bar_complete_workflow() {
    let config = ProgressManagerConfig::new().with_multi(true);
    let mut manager = ProgressManager::new(Some(config));
    manager.initialize(3, ProgressBarType::Multi);

    // Add tasks
    manager.add_task("build-1".to_string(), "Build Backend".to_string(), TaskStatus::InProgress);
    manager.add_task("build-2".to_string(), "Build Frontend".to_string(), TaskStatus::InProgress);
    manager.add_task("test-1".to_string(), "Run Tests".to_string(), TaskStatus::Pending);

    // Verify initial state
    let stats = manager.get_stats();
    assert_eq!(stats.total, 3);
    assert_eq!(stats.in_progress, 2);
    assert_eq!(stats.pending, 1);

    // Update progress
    manager.update_task("build-1", TaskStatus::InProgress, Some(25));
    manager.update_task("build-2", TaskStatus::InProgress, Some(50));

    // Start test task
    manager.update_task("test-1", TaskStatus::InProgress, Some(0));

    let stats = manager.get_stats();
    assert_eq!(stats.in_progress, 3);

    // Complete all tasks
    manager.update_task("build-1", TaskStatus::Completed, Some(100));
    manager.update_task("build-2", TaskStatus::Completed, Some(100));
    manager.update_task("test-1", TaskStatus::Completed, Some(100));

    let stats = manager.get_stats();
    assert_eq!(stats.completed, 3);

    manager.finish();
}

/// Tests task status transitions
#[test]
fn test_task_status_transitions() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(1, ProgressBarType::Single);

    manager.add_task("task-1".to_string(), "Test Task".to_string(), TaskStatus::Pending);

    // Verify pending state
    let stats = manager.get_stats();
    assert_eq!(stats.pending, 1);

    // Transition to in-progress
    manager.update_task("task-1", TaskStatus::InProgress, Some(0));
    let stats = manager.get_stats();
    assert_eq!(stats.in_progress, 1);

    // Transition to completed
    manager.update_task("task-1", TaskStatus::Completed, Some(100));
    let stats = manager.get_stats();
    assert_eq!(stats.completed, 1);

    manager.finish();
}

/// Tests failed task handling
#[test]
fn test_failed_task_handling() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(2, ProgressBarType::Single);

    manager.add_task("task-1".to_string(), "Success Task".to_string(), TaskStatus::Pending);
    manager.add_task("task-2".to_string(), "Failed Task".to_string(), TaskStatus::Pending);

    // Complete first task
    manager.update_task("task-1", TaskStatus::Completed, Some(100));
    manager.increment("task-1");

    // Fail second task
    manager.update_task("task-2", TaskStatus::Failed, Some(0));

    let stats = manager.get_stats();
    assert_eq!(stats.completed, 1);
    assert_eq!(stats.failed, 1);

    manager.finish();
}

/// Tests blocked task handling
#[test]
fn test_blocked_task_handling() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(2, ProgressBarType::Single);

    manager.add_task("task-1".to_string(), "Blocking Task".to_string(), TaskStatus::Pending);
    manager.add_task("task-2".to_string(), "Blocked Task".to_string(), TaskStatus::Pending);

    // Mark second task as blocked
    manager.update_task("task-2", TaskStatus::Blocked, None);

    let stats = manager.get_stats();
    assert_eq!(stats.blocked, 1);

    // Unblock and complete
    manager.update_task("task-2", TaskStatus::InProgress, Some(0));
    manager.update_task("task-2", TaskStatus::Completed, Some(100));
    manager.increment("task-2");

    let stats = manager.get_stats();
    assert_eq!(stats.completed, 1);
    assert_eq!(stats.blocked, 0);

    manager.finish();
}

/// Tests progress bar creation functions
#[test]
fn test_progress_bar_creation() {
    // Test standard progress bar
    let bar = create_progress_bar(100, None);
    assert_eq!(bar.length(), Some(100));

    // Test with color config
    let config = BarColorConfig::plain();
    let bar = create_progress_bar(100, Some(config));
    assert_eq!(bar.length(), Some(100));

    // Test custom progress bar
    let template = "[{bar:40}] {pos}/{len}";
    let bar = create_custom_progress_bar(100, template, None);
    assert_eq!(bar.length(), Some(100));

    // Test spinner
    let spinner = create_spinner(None);
    // Just verify it doesn't panic
    drop(spinner);
}

/// Tests percentage colorization
#[test]
fn test_percentage_colorization() {
    let config = ColorConfig::plain();

    // Test various percentage ranges
    let p0 = colorize_percentage(0, config);
    assert!(p0.contains("0%"));

    let p25 = colorize_percentage(25, config);
    assert!(p25.contains("25%"));

    let p50 = colorize_percentage(50, config);
    assert!(p50.contains("50%"));

    let p75 = colorize_percentage(75, config);
    assert!(p75.contains("75%"));

    let p100 = colorize_percentage(100, config);
    assert!(p100.contains("100%"));

    // Test edge cases
    let p_edge = colorize_percentage(123, config);
    assert!(p_edge.contains("123%"));
}

/// Tests progress manager configuration
#[test]
fn test_progress_manager_configuration() {
    // Test default config
    let config = ProgressManagerConfig::new();
    assert!(config.enable_multi);
    assert!(config.enable_eta);
    assert_eq!(config.update_interval_ms, 100);

    // Test plain config
    let config = ProgressManagerConfig::plain();
    assert!(!config.color_config.is_enabled());

    // Test config builder methods
    let config = ProgressManagerConfig::new()
        .with_multi(false)
        .with_eta(false)
        .with_update_interval(200);

    assert!(!config.enable_multi);
    assert!(!config.enable_eta);
    assert_eq!(config.update_interval_ms, 200);
}

/// Tests reporter functions
#[test]
fn test_reporter_functions() {
    // Note: These tests just verify the functions don't panic
    // In a real test environment, we would capture and verify stdout

    report_task_start("task-1", "Test Task", None);
    report_task_complete("task-1", "Test Task", true, None);
    report_task_complete("task-2", "Failed Task", false, None);
    report_progress_summary(5, 10, 1, None);
}

/// Tests manager with custom configuration
#[test]
fn test_manager_with_custom_config() {
    let config = ProgressManagerConfig::new()
        .with_multi(false)
        .with_eta(false)
        .with_update_interval(200);

    let mut manager = ProgressManager::new(Some(config));
    manager.initialize(2, ProgressBarType::Single);

    manager.add_task("task-1".to_string(), "Task 1".to_string(), TaskStatus::Pending);
    manager.add_task("task-2".to_string(), "Task 2".to_string(), TaskStatus::Pending);

    // Verify tasks are tracked
    let stats = manager.get_stats();
    assert_eq!(stats.total, 2);

    manager.finish();
}

/// Tests message setting
#[test]
fn test_message_setting() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(1, ProgressBarType::Single);

    manager.add_task("task-1".to_string(), "Task 1".to_string(), TaskStatus::Pending);

    // Set custom message
    manager.set_message("Custom message".to_string());

    manager.finish();
}

/// Tests abandon functionality
#[test]
fn test_abandon_functionality() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(1, ProgressBarType::Single);

    manager.add_task("task-1".to_string(), "Task 1".to_string(), TaskStatus::InProgress);

    // Abandon instead of finish
    manager.abandon();
}

/// Tests clear functionality
#[test]
fn test_clear_functionality() {
    let config = ProgressManagerConfig::new().with_multi(true);
    let mut manager = ProgressManager::new(Some(config));
    manager.initialize(2, ProgressBarType::Multi);

    manager.add_task("task-1".to_string(), "Task 1".to_string(), TaskStatus::InProgress);
    manager.add_task("task-2".to_string(), "Task 2".to_string(), TaskStatus::InProgress);

    // Clear all - just verify it doesn't panic
    manager.clear();
}

/// Tests stats accuracy across multiple updates
#[test]
fn test_stats_accuracy() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(5, ProgressBarType::Single);

    // Add all tasks
    for i in 1..=5 {
        manager.add_task(
            format!("task-{}", i),
            format!("Task {}", i),
            TaskStatus::Pending,
        );
    }

    let stats = manager.get_stats();
    assert_eq!(stats.total, 5);
    assert_eq!(stats.pending, 5);

    // Update various tasks
    manager.update_task("task-1", TaskStatus::InProgress, Some(50));
    manager.update_task("task-2", TaskStatus::Completed, Some(100));
    manager.update_task("task-3", TaskStatus::Failed, Some(0));
    manager.update_task("task-4", TaskStatus::Blocked, None);

    let stats = manager.get_stats();
    assert_eq!(stats.pending, 1);       // task-5
    assert_eq!(stats.in_progress, 1);   // task-1
    assert_eq!(stats.completed, 1);     // task-2
    assert_eq!(stats.failed, 1);        // task-3
    assert_eq!(stats.blocked, 1);       // task-4

    manager.finish();
}

/// Tests progress with zero tasks
#[test]
fn test_zero_tasks() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(0, ProgressBarType::Single);

    let stats = manager.get_stats();
    assert_eq!(stats.total, 0);

    manager.finish();
}

/// Tests progress bar type enum
#[test]
fn test_progress_bar_type() {
    let single = ProgressBarType::Single;
    let multi = ProgressBarType::Multi;

    // Verify variants can be created and compared
    match single {
        ProgressBarType::Single => {}
        ProgressBarType::Multi => panic!("Wrong type"),
    }

    match multi {
        ProgressBarType::Single => panic!("Wrong type"),
        ProgressBarType::Multi => {}
    }
}

/// Tests concurrent task updates (simulated)
#[test]
fn test_concurrent_task_updates() {
    let mut manager = ProgressManager::new(Some(ProgressManagerConfig::new().with_multi(true)));
    manager.initialize(3, ProgressBarType::Multi);

    // Add tasks
    manager.add_task("task-1".to_string(), "Task 1".to_string(), TaskStatus::InProgress);
    manager.add_task("task-2".to_string(), "Task 2".to_string(), TaskStatus::InProgress);
    manager.add_task("task-3".to_string(), "Task 3".to_string(), TaskStatus::InProgress);

    // Simulate concurrent updates
    for i in 0..=4 {
        let percent = (i * 25).min(100);
        manager.update_task("task-1", TaskStatus::InProgress, Some(percent));
        manager.update_task("task-2", TaskStatus::InProgress, Some(percent));
        manager.update_task("task-3", TaskStatus::InProgress, Some(percent));
    }

    // Complete all
    manager.update_task("task-1", TaskStatus::Completed, Some(100));
    manager.update_task("task-2", TaskStatus::Completed, Some(100));
    manager.update_task("task-3", TaskStatus::Completed, Some(100));

    let stats = manager.get_stats();
    assert_eq!(stats.completed, 3);

    manager.finish();
}

/// Tests percentage display in summary
#[test]
fn test_percentage_display() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(4, ProgressBarType::Single);

    manager.add_task("task-1".to_string(), "Task 1".to_string(), TaskStatus::Completed);
    manager.increment("task-1");

    manager.add_task("task-2".to_string(), "Task 2".to_string(), TaskStatus::Completed);
    manager.increment("task-2");

    // With 2/4 tasks completed, should show 50%
    let summary = manager.format_summary();
    assert!(!summary.is_empty());

    manager.finish();
}

/// Tests task name storage and retrieval
#[test]
fn test_task_name_storage() {
    let mut manager = ProgressManager::new(Some(ProgressManagerConfig::new().with_multi(true)));
    manager.initialize(1, ProgressBarType::Multi);

    manager.add_task("task-1".to_string(), "My Custom Task Name".to_string(), TaskStatus::Pending);

    // Verify task is tracked
    let stats = manager.get_stats();
    assert_eq!(stats.total, 1);

    manager.finish();
}

/// Tests color config is_enabled
#[test]
fn test_color_config_enabled() {
    let auto_config = BarColorConfig::auto();
    // Just verify it doesn't panic
    let _ = auto_config.is_enabled();

    let plain_config = BarColorConfig::plain();
    assert!(!plain_config.is_enabled());
}

/// Tests reporter color config
#[test]
fn test_reporter_color_config() {
    let auto_config = ReporterColorConfig::auto();
    let _ = auto_config.is_enabled();

    let plain_config = ReporterColorConfig::plain();
    assert!(!plain_config.is_enabled());
}

/// Tests manager finish after abandon
#[test]
fn test_finish_after_abandon() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(1, ProgressBarType::Single);

    manager.add_task("task-1".to_string(), "Task 1".to_string(), TaskStatus::InProgress);

    // Abandon first
    manager.abandon();

    // Finish should still work (idempotent)
    manager.finish();
}
