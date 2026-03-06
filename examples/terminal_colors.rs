//! Example demonstrating terminal color and formatting capabilities
//!
//! Run with: cargo run --example terminal_colors

use ltmatrix::terminal::{self, ColorConfig};
use ltmatrix::progress::{
    create_progress_bar, create_spinner, report_task_start, report_task_complete,
    report_progress_summary, TrackerColorConfig,
};
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== Terminal Color & Formatting Demo ===\n");

    // Demo 1: Color configuration
    println!("1. Color Configuration:");
    let config = ColorConfig::auto();
    println!("   Auto-detected colors enabled: {}", config.is_enabled());

    let plain_config = ColorConfig::plain();
    println!("   Plain colors enabled: {}", plain_config.is_enabled());
    println!();

    // Demo 2: Colorized text
    println!("2. Colorized Text:");
    println!("   {}", terminal::success("Success message", config));
    println!("   {}", terminal::error("Error message", config));
    println!("   {}", terminal::warning("Warning message", config));
    println!("   {}", terminal::info("Info message", config));
    println!("   {} {}", terminal::bold("Bold text", config), terminal::dim("Dim text", config));
    println!();

    // Demo 3: Task status colors
    println!("3. Task Status Colors:");
    for status in ["pending", "in_progress", "completed", "failed", "blocked"] {
        println!("   {}", terminal::colorize_status(status, config));
    }
    println!();

    // Demo 4: Log level colors
    println!("4. Log Level Colors:");
    for level in ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"] {
        println!("   {}", terminal::colorize_log_level(level, config));
    }
    println!();

    // Demo 5: Progress bar
    println!("5. Progress Bar:");
    let bar = create_progress_bar(10, None);
    for i in 0..=10 {
        bar.set_message(format!("Processing item {}", i));
        bar.inc(1);
        thread::sleep(Duration::from_millis(100));
    }
    bar.finish_with_message("Done!");
    println!();

    // Demo 6: Task reporting
    println!("6. Task Reporting:");
    report_task_start("task-1", "Build database schema", None);
    thread::sleep(Duration::from_millis(500));
    report_task_complete("task-1", "Build database schema", true, None);
    println!();

    // Demo 7: Progress summary
    println!("7. Progress Summary:");
    report_progress_summary(5, 10, 1, None);
    println!();

    // Demo 8: Spinner
    println!("8. Spinner (will run for 2 seconds):");
    let spinner = create_spinner(None);
    spinner.set_message("Processing...");
    spinner.enable_steady_tick(Duration::from_millis(100));
    thread::sleep(Duration::from_secs(2));
    spinner.finish_with_message("Processing complete!");
    println!();

    // Demo 9: Plain text mode (respects NO_COLOR)
    println!("9. Plain Text Mode (same output with colors disabled):");
    println!("   {}", terminal::success("Success", plain_config));
    println!("   {}", terminal::error("Error", plain_config));
    println!("   {}", terminal::warning("Warning", plain_config));
    println!("   {}", terminal::colorize_status("completed", plain_config));
    println!();

    println!("=== Demo Complete ===");
    println!();
    println!("Try setting NO_COLOR=1 or using --no-color flag to disable colors!");
}
