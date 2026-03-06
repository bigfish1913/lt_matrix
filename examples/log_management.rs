//! Example demonstrating log file management with rotation and cleanup
//!
//! Run with: cargo run --example log_management

use ltmatrix::logging::{init_logging_with_management, LogLevel, LogManager};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Log File Management Demo ===\n");

    // Demo 1: Create a temporary directory for logs
    println!("1. Creating logs directory:");
    let temp_dir = TempDir::new()?;
    println!("   Created temporary directory: {:?}", temp_dir.path());

    // Demo 2: Create log manager with custom settings
    println!("\n2. Creating log manager:");
    let _log_manager = LogManager::new(Some(temp_dir.path()))
        .with_max_files(5)
        .with_max_age_days(7)
        .with_max_total_size(50 * 1024 * 1024); // 50 MB
    println!("   Max files: 5");
    println!("   Max age: 7 days");
    println!("   Max total size: 50 MB");

    // Demo 3: Initialize logging with automatic log file creation
    println!("\n3. Initializing logging with automatic log file creation:");
    let (_guard, manager) = init_logging_with_management(LogLevel::Info, Some(temp_dir.path()))?;
    println!("   Logging initialized successfully!");

    // Demo 4: Generate some log entries
    println!("\n4. Writing log entries:");
    for i in 1..=5 {
        tracing::info!("Test log entry {}", i);
        tracing::warn!("This is a warning message {}", i);
        thread::sleep(Duration::from_millis(100));
    }
    println!("   Wrote 5 log entries");

    // Demo 5: Get log file information
    println!("\n5. Checking log files:");
    let log_info = manager.get_log_info()?;
    println!("   Found {} log file(s):", log_info.len());
    for info in &log_info {
        let filename = info.path.file_name().unwrap_or_else(|| "unknown".as_ref());
        let size_kb = info.size / 1024;
        println!(
            "   - {} ({} KB, {} days old)",
            filename.to_string_lossy(),
            size_kb,
            info.age_days
        );
    }

    // Demo 6: Create multiple log files to demonstrate rotation
    println!("\n6. Creating multiple log files to demonstrate rotation:");
    for i in 1..=3 {
        let log_path = manager.create_log_file()?;
        let filename = log_path.file_name().unwrap().to_string_lossy();
        println!("   Created: {}", filename);

        // Write some content
        use std::fs::File;
        use std::io::Write;
        let mut file = File::create(&log_path)?;
        writeln!(
            file,
            "Log file {} - Created at {:?}",
            i,
            chrono::Local::now()
        )?;
        writeln!(file, "This is test content for rotation demo")?;

        thread::sleep(Duration::from_millis(50));
    }

    // Demo 7: Check log files again
    println!("\n7. Checking log files after creating multiple:");
    let log_info = manager.get_log_info()?;
    println!("   Found {} log file(s):", log_info.len());

    // Demo 8: Run cleanup
    println!("\n8. Running cleanup (removes old/extra logs):");
    let removed = manager.cleanup_old_logs()?;
    println!("   Removed {} log file(s)", removed);

    // Demo 9: Check final state
    println!("\n9. Final log file count:");
    let final_count = manager.get_log_info()?.len();
    println!("   {} log file(s) remaining", final_count);

    println!("\n=== Demo Complete ===");
    println!("\nKey Features Demonstrated:");
    println!("✓ Automatic logs/ directory creation");
    println!("✓ Timestamped log file naming (run-YYYYMMDD-HHMMSS.log)");
    println!("✓ Multiple log file creation");
    println!("✓ Automatic cleanup and rotation");
    println!("✓ Configurable limits (files, age, size)");

    Ok(())
}
