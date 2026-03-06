//! Progress Manager Demo
//!
//! This example demonstrates the usage of the ProgressManager for tracking
//! and displaying task progress with both single and multi-line progress bars.

use ltmatrix::models::TaskStatus;
use ltmatrix::progress::{ProgressBarType, ProgressManager, ProgressManagerConfig};
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== Single Progress Bar Demo ===\n");
    demo_single_progress_bar();

    println!("\n=== Multi Progress Bar Demo ===\n");
    demo_multi_progress_bar();
}

/// Demonstrates single progress bar for overall progress
fn demo_single_progress_bar() {
    let mut manager = ProgressManager::new(None);
    manager.initialize(5, ProgressBarType::Single);

    // Add some tasks
    manager.add_task(
        "task-1".to_string(),
        "Initialize Project".to_string(),
        TaskStatus::Pending,
    );
    manager.add_task(
        "task-2".to_string(),
        "Setup Dependencies".to_string(),
        TaskStatus::Pending,
    );
    manager.add_task(
        "task-3".to_string(),
        "Write Tests".to_string(),
        TaskStatus::Pending,
    );
    manager.add_task(
        "task-4".to_string(),
        "Run Tests".to_string(),
        TaskStatus::Pending,
    );
    manager.add_task(
        "task-5".to_string(),
        "Deploy".to_string(),
        TaskStatus::Pending,
    );

    // Simulate task execution
    let tasks = vec!["task-1", "task-2", "task-3", "task-4", "task-5"];

    for task_id in tasks {
        // Mark task as in progress
        manager.update_task(task_id, TaskStatus::InProgress, Some(0));
        thread::sleep(Duration::from_millis(500));

        // Simulate progress
        for percent in (0..=100).step_by(25) {
            thread::sleep(Duration::from_millis(100));
            if percent > 0 {
                manager.set_message(format!("Working on {} - {}%", task_id, percent));
            }
        }

        // Mark task as completed
        manager.update_task(task_id, TaskStatus::Completed, Some(100));
        manager.increment(task_id);
        thread::sleep(Duration::from_millis(200));
    }

    manager.finish();
    println!("\nFinal Stats: {}", manager.format_summary());
}

/// Demonstrates multi-line progress bars for individual tasks
fn demo_multi_progress_bar() {
    let config = ProgressManagerConfig::new().with_multi(true);
    let mut manager = ProgressManager::new(Some(config));
    manager.initialize(3, ProgressBarType::Multi);

    // Add tasks
    manager.add_task(
        "build-1".to_string(),
        "Building Backend".to_string(),
        TaskStatus::InProgress,
    );
    manager.add_task(
        "build-2".to_string(),
        "Building Frontend".to_string(),
        TaskStatus::InProgress,
    );
    manager.add_task(
        "build-3".to_string(),
        "Running Tests".to_string(),
        TaskStatus::Pending,
    );

    // Simulate concurrent task execution
    for i in 0..=4 {
        thread::sleep(Duration::from_millis(300));

        // Update build-1
        let percent1 = (i * 25).min(100);
        manager.update_task("build-1", TaskStatus::InProgress, Some(percent1));

        // Update build-2
        let percent2 = (i * 20).min(100);
        manager.update_task("build-2", TaskStatus::InProgress, Some(percent2));

        // Start build-3 after others progress
        if i >= 2 {
            let percent3 = ((i - 2) * 30).min(100);
            manager.update_task("build-3", TaskStatus::InProgress, Some(percent3));
        }
    }

    // Mark all as completed
    manager.update_task("build-1", TaskStatus::Completed, Some(100));
    manager.update_task("build-2", TaskStatus::Completed, Some(100));
    manager.update_task("build-3", TaskStatus::Completed, Some(100));

    thread::sleep(Duration::from_millis(500));
    manager.finish();

    println!("\nFinal Stats: {}", manager.format_summary());
}
