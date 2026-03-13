//! Logging Directory Structure Demo
//!
//! This example demonstrates the logging directory structure and management features:
//! - Automatic logs/ directory creation
//! - Timestamped log files (run-YYYYMMDD-HHMMSS.log)
//! - Log rotation based on age, size, and count
//! - Cleanup on successful completion
//!
//! # Usage
//!
//! Run this example to see the logging directory structure:
//! ```bash
//! cargo run --example log_directory_demo
//! ```

use ltmatrix::logging::file_manager::LogManager;
use ltmatrix::logging::level::LogLevel;
use ltmatrix::logging::logger;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("ltmatrix Logging Directory Structure Demo");
    println!("=========================================\n");

    // Demonstrate LogManager features
    println!("1. Creating LogManager with default settings:");
    println!("-------------------------------------------");
    let log_manager = LogManager::new(None::<&Path>);
    println!("✓ Logs directory: {}", log_manager.logs_dir().display());
    println!(
        "✓ Max files: {}",
        log_manager.get_log_info().unwrap_or_default().len()
    );
    println!("✓ Max age: 7 days");
    println!("✓ Max total size: 100 MB");
    println!();

    // Initialize directory
    println!("2. Initializing logs directory:");
    println!("--------------------------------");
    log_manager.initialize_directory()?;
    println!("✓ Directory created/verified");
    println!();

    // Generate timestamped log file paths
    println!("3. Generating timestamped log file paths:");
    println!("------------------------------------------");
    for i in 0..3 {
        let log_path = log_manager.generate_log_path();
        let filename = log_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<invalid>");
        println!("  {}. {}", i + 1, filename);

        // Add small delay to ensure different timestamps
        thread::sleep(Duration::from_millis(100));
    }
    println!();

    // Create actual log files
    println!("4. Creating log files:");
    println!("---------------------");
    let mut created_files = Vec::new();
    for i in 0..3 {
        let log_path = log_manager.create_log_file()?;
        let filename = log_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<invalid>");
        println!("  ✓ Created: {}", filename);

        // Write some sample content
        let content = format!(
            "# Log file created at: {}\n\
             # Run {}\n\
             INFO: ltmatrix logging system initialized\n\
             INFO: This is a sample log entry\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            i + 1
        );
        fs::write(&log_path, content)?;

        created_files.push(log_path);

        // Add delay to ensure different timestamps
        thread::sleep(Duration::from_millis(200));
    }
    println!();

    // Show log file information
    println!("5. Log file information:");
    println!("------------------------");
    let log_info = log_manager.get_log_info()?;
    for (i, info) in log_info.iter().enumerate() {
        let filename = info
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<invalid>");
        println!("  File {}:", i + 1);
        println!("    Name: {}", filename);
        println!("    Size: {} bytes", info.size);
        println!("    Age: {} days", info.age_days);
        println!(
            "    Modified: {}",
            info.modified_time.format("%Y-%m-%d %H:%M:%S")
        );
    }
    println!();

    // Demonstrate cleanup
    println!("6. Demonstrating log cleanup:");
    println!("----------------------------");

    // Set strict limits for demonstration
    let strict_manager = LogManager::new(None::<&Path>)
        .with_max_files(2)
        .with_max_age_days(0); // Remove files older than now

    println!("  Cleanup policy:");
    println!("    - Keep max 2 files");
    println!("    - Remove files older than 0 days");
    println!("    - Total files before cleanup: {}", log_info.len());

    let removed = strict_manager.cleanup_old_logs()?;
    println!("  ✓ Removed {} file(s)", removed);

    let remaining = strict_manager.get_log_info()?;
    println!("    - Total files after cleanup: {}", remaining.len());
    println!();

    // Demonstrate initialization with logging
    println!("7. Initializing logging with automatic management:");
    println!("----------------------------------------------------");
    let (_guard, _managed_log) =
        logger::init_logging_with_management(LogLevel::Info, None::<&Path>)?;
    println!("  ✓ Logging initialized with automatic file management");
    println!("  ✓ Log file created in logs/ directory");
    println!("  ✓ Old logs will be cleaned up on completion");

    // Log some test messages
    tracing::info!("This is an info message");
    tracing::warn!("This is a warning message");
    tracing::error!("This is an error message");
    println!();

    // Demonstrate cleanup on success
    println!("8. Cleanup on successful completion:");
    println!("-------------------------------------");
    println!("  When a run completes successfully, old logs are automatically");
    println!("  cleaned up based on the configured limits:");
    println!("    - Max files: Keep only the N most recent log files");
    println!("    - Max age: Remove logs older than N days");
    println!("    - Max size: Remove oldest logs if total size exceeds limit");
    println!();
    println!("  To cleanup manually:");
    println!("    let removed = log_manager.cleanup_on_success()?;");
    println!();

    // Show what was created
    println!("9. Summary of created files:");
    println!("---------------------------");
    if log_manager.logs_dir().exists() {
        let entries = fs::read_dir(log_manager.logs_dir())?;
        let mut log_files = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("log") {
                log_files.push(path);
            }
        }

        if log_files.is_empty() {
            println!("  No log files in logs/ directory");
        } else {
            println!("  Log files in logs/ directory:");
            for (i, path) in log_files.iter().enumerate() {
                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("<invalid>");
                println!("    {}. {}", i + 1, filename);
            }
        }
    }
    println!();

    println!("✓ Demo completed successfully!");
    println!();
    println!("Key Features:");
    println!("  - Automatic logs/ directory creation");
    println!("  - Timestamped log files (run-YYYYMMDD-HHMMSS.log)");
    println!("  - Automatic log rotation (age, size, count)");
    println!("  - Cleanup on successful completion");
    println!("  - Integration with tracing subsystem");
    println!("  - Configurable via LogManager builder");

    Ok(())
}
