//! ETA Estimation Demo
//!
//! This example demonstrates the enhanced progress manager with ETA estimation
//! and metrics collection features.

use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix::progress::eta::{HistoricalData, MetricsCollector};
use ltmatrix::progress::{ProgressBarType, ProgressManager, ProgressManagerConfig};
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== ETA Estimation Demo ===\n");

    // First, run a session to build up historical data
    println!("Building historical data...");
    let mut historical_data = build_historical_data();
    println!("Historical data: {} tasks completed\n", historical_data.total_completed());

    // Demo 1: Progress Manager with ETA
    demo_eta_enabled_manager(&historical_data);

    // Demo 2: Progress Manager without ETA
    demo_eta_disabled_manager();

    // Demo 3: Metrics collection
    demo_metrics_collection();
}

/// Builds some historical data for ETA estimation
fn build_historical_data() -> HistoricalData {
    let mut data = HistoricalData::new();

    // Simulate some past task completions
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(10));
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(15));
    data.record_completion(TaskComplexity::Simple, Duration::from_secs(20));

    data.record_completion(TaskComplexity::Moderate, Duration::from_secs(30));
    data.record_completion(TaskComplexity::Moderate, Duration::from_secs(40));

    data.record_completion(TaskComplexity::Complex, Duration::from_secs(90));

    data
}

/// Demonstrates progress manager with ETA enabled
fn demo_eta_enabled_manager(historical_data: &HistoricalData) {
    println!("--- Demo 1: ETA Enabled ---");

    let config = ProgressManagerConfig::new()
        .with_eta(true)
        .with_multi(false);

    let historical_clone = HistoricalData::new(); // Start fresh for demo
    let mut manager = ProgressManager::with_historical_data(Some(config), historical_clone);

    manager.initialize(3, ProgressBarType::Single);

    println!("Starting 3 tasks with ETA estimation...\n");

    // Add tasks
    manager.add_task(
        "task-1".to_string(),
        "Simple Task".to_string(),
        TaskStatus::Pending,
    );
    manager.add_task(
        "task-2".to_string(),
        "Moderate Task".to_string(),
        TaskStatus::Pending,
    );
    manager.add_task(
        "task-3".to_string(),
        "Complex Task".to_string(),
        TaskStatus::Pending,
    );

    // Simulate task execution
    for i in 1..=3 {
        let task_id = format!("task-{}", i);
        manager.update_task(&task_id, TaskStatus::InProgress, Some(0));

        println!("Working on {}...", task_id);
        thread::sleep(Duration::from_millis(500));

        // Update progress
        for percent in (0..=100).step_by(25) {
            thread::sleep(Duration::from_millis(100));
            if percent > 0 {
                let elapsed = manager.elapsed_time().unwrap_or_default();
                let eta_str = if let Some(eta) = manager.calculate_remaining_eta() {
                    format!(" (ETA: {}s)", eta.as_secs())
                } else {
                    String::new()
                };
                println!("  Progress: {}% | Elapsed: {}s{}", percent, elapsed.as_secs(), eta_str);
            }
        }

        manager.update_task(&task_id, TaskStatus::Completed, Some(100));
        manager.increment(&task_id);
        println!("{} completed!\n", task_id);
    }

    manager.finish();
    println!("Final Stats: {}\n", manager.format_summary());
}

/// Demonstrates progress manager without ETA
fn demo_eta_disabled_manager() {
    println!("--- Demo 2: ETA Disabled ---");

    let config = ProgressManagerConfig::new()
        .with_eta(false)
        .with_multi(false);

    let mut manager = ProgressManager::new(Some(config));
    manager.initialize(2, ProgressBarType::Single);

    println!("Starting 2 tasks without ETA...\n");

    manager.add_task(
        "task-a".to_string(),
        "Task A".to_string(),
        TaskStatus::Pending,
    );
    manager.add_task(
        "task-b".to_string(),
        "Task B".to_string(),
        TaskStatus::Pending,
    );

    // Quick execution
    for task_id in ["task-a", "task-b"] {
        manager.update_task(task_id, TaskStatus::InProgress, Some(50));
        thread::sleep(Duration::from_millis(200));
        manager.update_task(task_id, TaskStatus::Completed, Some(100));
        manager.increment(task_id);
    }

    manager.finish();
    println!("Completed: {}\n", manager.format_summary());
}

/// Demonstrates metrics collection
fn demo_metrics_collection() {
    println!("--- Demo 3: Metrics Collection ---");

    let mut collector = MetricsCollector::new();

    // Create and track some tasks
    let mut task1 = Task::new("metrics-1", "Metrics Task 1", "Test");
    task1.complexity = TaskComplexity::Simple;

    let mut task2 = Task::new("metrics-2", "Metrics Task 2", "Test");
    task2.complexity = TaskComplexity::Moderate;

    let mut task3 = Task::new("metrics-3", "Metrics Task 3", "Test");
    task3.complexity = TaskComplexity::Simple;

    // Track execution
    println!("Tracking task execution times...");

    collector.track_task_start(&task1);
    thread::sleep(Duration::from_millis(50));
    task1.status = TaskStatus::Completed;
    collector.track_task_completion(&task1);

    collector.track_task_start(&task2);
    thread::sleep(Duration::from_millis(100));
    task2.status = TaskStatus::Completed;
    collector.track_task_completion(&task2);

    collector.track_task_start(&task3);
    thread::sleep(Duration::from_millis(75));
    task3.status = TaskStatus::Completed;
    collector.track_task_completion(&task3);

    // Get and display metrics
    let metrics = collector.get_metrics();

    println!("\nMetrics Summary:");
    println!("  Total tracked: {}", metrics.total_tracked);
    println!("  Total completed: {}", metrics.total_completed);
    println!("  Average duration: {}ms", metrics.average_duration.as_millis());

    println!("\nBy Complexity:");
    for (complexity, complexity_metrics) in &metrics.by_complexity {
        println!(
            "  {:?}: {} tasks, avg: {}ms, min: {}ms, max: {}ms",
            complexity,
            complexity_metrics.count,
            complexity_metrics.average_duration.as_millis(),
            complexity_metrics.min_duration.as_millis(),
            complexity_metrics.max_duration.as_millis()
        );
    }
}
