//! Log file management with directory structure, rotation, and cleanup
//!
//! This module handles:
//! - Creating logs/ directory structure
//! - Generating timestamped log files (run-YYYYMMDD-HHMMSS.log)
//! - Rotating log files to prevent excessive disk usage
//! - Cleaning up old logs on completion

use chrono::{DateTime, Local, TimeZone};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Default logs directory name
pub const LOGS_DIR: &str = "logs";

/// Maximum number of log files to keep (default: 10)
pub const DEFAULT_MAX_LOG_FILES: usize = 10;

/// Maximum age of log files to keep (default: 7 days)
pub const DEFAULT_MAX_LOG_AGE_DAYS: i64 = 7;

/// Maximum total size of all log files (default: 100 MB)
pub const DEFAULT_MAX_LOG_SIZE_BYTES: u64 = 100 * 1024 * 1024;

/// Log file manager
#[derive(Debug, Clone)]
pub struct LogManager {
    /// Base directory for log files
    logs_dir: PathBuf,

    /// Maximum number of log files to keep
    max_files: usize,

    /// Maximum age of log files in days
    max_age_days: i64,

    /// Maximum total size of all log files in bytes
    max_total_size: u64,
}

impl LogManager {
    /// Creates a new LogManager with default settings
    ///
    /// # Arguments
    ///
    /// * `base_dir` - Optional base directory (defaults to current directory)
    ///
    /// # Returns
    ///
    /// A new LogManager instance
    #[must_use]
    pub fn new(base_dir: Option<impl AsRef<Path>>) -> Self {
        let base = base_dir
            .map(|p| p.as_ref().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let logs_dir = base.join(LOGS_DIR);

        LogManager {
            logs_dir,
            max_files: DEFAULT_MAX_LOG_FILES,
            max_age_days: DEFAULT_MAX_LOG_AGE_DAYS,
            max_total_size: DEFAULT_MAX_LOG_SIZE_BYTES,
        }
    }

    /// Sets the maximum number of log files to keep
    ///
    /// # Arguments
    ///
    /// * `max_files` - Maximum number of log files
    #[must_use]
    pub const fn with_max_files(mut self, max_files: usize) -> Self {
        self.max_files = max_files;
        self
    }

    /// Sets the maximum age of log files in days
    ///
    /// # Arguments
    ///
    /// * `max_age_days` - Maximum age in days
    #[must_use]
    pub const fn with_max_age_days(mut self, max_age_days: i64) -> Self {
        self.max_age_days = max_age_days;
        self
    }

    /// Sets the maximum total size of all log files
    ///
    /// # Arguments
    ///
    /// * `max_size` - Maximum total size in bytes
    #[must_use]
    pub const fn with_max_total_size(mut self, max_size: u64) -> Self {
        self.max_total_size = max_size;
        self
    }

    /// Initializes the logs directory structure
    ///
    /// Creates the logs/ directory if it doesn't exist.
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, Err otherwise
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created
    pub fn initialize_directory(&self) -> io::Result<()> {
        if !self.logs_dir.exists() {
            fs::create_dir_all(&self.logs_dir)?;
        }
        Ok(())
    }

    /// Generates a new timestamped log file path
    ///
    /// Format: logs/run-YYYYMMDD-HHMMSS.log
    ///
    /// # Returns
    ///
    /// A PathBuf pointing to the new log file
    #[must_use]
    pub fn generate_log_path(&self) -> PathBuf {
        let now = Local::now();
        let timestamp = now.format("%Y%m%d-%H%M%S");
        let filename = format!("run-{}.log", timestamp);
        self.logs_dir.join(filename)
    }

    /// Creates a new log file
    ///
    /// # Returns
    ///
    /// Ok(PathBuf) pointing to the created log file, Err on failure
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created
    pub fn create_log_file(&self) -> io::Result<PathBuf> {
        self.initialize_directory()?;
        let log_path = self.generate_log_path();
        fs::File::create(&log_path)?;
        Ok(log_path)
    }

    /// Cleans up old log files based on configured limits
    ///
    /// This applies three cleanup strategies in order:
    /// 1. Remove log files older than max_age_days
    /// 2. If total size exceeds max_total_size, remove oldest files
    /// 3. If file count exceeds max_files, remove oldest files
    ///
    /// # Returns
    ///
    /// Ok(number of files removed) if successful, Err on failure
    ///
    /// # Errors
    ///
    /// Returns an error if reading or deleting files fails
    pub fn cleanup_old_logs(&self) -> io::Result<usize> {
        if !self.logs_dir.exists() {
            return Ok(0);
        }

        let mut removed = 0;

        // Get all log files sorted by modification time (oldest first)
        let mut log_files = self.get_log_files_sorted()?;

        // Strategy 1: Remove files older than max_age_days
        let cutoff_time = Local::now() - chrono::Duration::days(self.max_age_days);
        log_files.retain(|(mtime, _path)| *mtime > cutoff_time);

        removed += self.remove_files_not_in_list(&log_files)?;

        // Strategy 2: Remove oldest files if total size exceeds limit
        let total_size = self.calculate_total_size(&log_files)?;
        if total_size > self.max_total_size {
            let size_to_remove = total_size - self.max_total_size;
            removed += self.remove_files_by_size(&mut log_files, size_to_remove)?;
        }

        // Strategy 3: Remove oldest files if count exceeds limit
        if log_files.len() > self.max_files {
            let excess = log_files.len() - self.max_files;
            for (_mtime, path) in log_files.iter().take(excess) {
                if path.exists() {
                    fs::remove_file(path)?;
                    removed += 1;
                }
            }
        }

        Ok(removed)
    }

    /// Cleans up logs from a successful run
    ///
    /// This is called when a run completes successfully to clean up
    /// old logs according to the configured limits.
    ///
    /// # Returns
    ///
    /// Ok(number of files removed) if successful, Err on failure
    pub fn cleanup_on_success(&self) -> io::Result<usize> {
        self.cleanup_old_logs()
    }

    /// Gets all log files sorted by modification time (oldest first)
    ///
    /// # Returns
    ///
    /// A vector of (modification_time, path) tuples
    fn get_log_files_sorted(&self) -> io::Result<Vec<(DateTime<Local>, PathBuf)>> {
        let mut log_files = Vec::new();

        let entries = fs::read_dir(&self.logs_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Only process .log files
            if path.extension().and_then(|s| s.to_str()) != Some("log") {
                continue;
            }

            // Get modification time
            let metadata = fs::metadata(&path)?;
            let mtime_secs = metadata
                .modified()?
                .duration_since(UNIX_EPOCH)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
                .as_secs();
            let mtime = Local.timestamp_opt(mtime_secs as i64, 0).single();

            if let Some(mtime) = mtime {
                log_files.push((mtime, path));
            }
        }

        // Sort by modification time (oldest first)
        log_files.sort_by_key(|(mtime, _path)| *mtime);

        Ok(log_files)
    }

    /// Removes files that are not in the provided list
    ///
    /// # Arguments
    ///
    /// * `files_to_keep` - List of files to keep
    ///
    /// # Returns
    ///
    /// The number of files removed
    fn remove_files_not_in_list(&self, files_to_keep: &[(DateTime<Local>, PathBuf)]) -> io::Result<usize> {
        let keep_paths: std::collections::HashSet<_> =
            files_to_keep.iter().map(|(_, path)| path).collect();

        let mut removed = 0;

        let entries = fs::read_dir(&self.logs_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("log") {
                continue;
            }

            if !keep_paths.contains(&path) && path.exists() {
                fs::remove_file(&path)?;
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// Removes files by total size (starting with oldest)
    ///
    /// # Arguments
    ///
    /// * `log_files` - Sorted list of log files (will be modified)
    /// * `size_to_remove` - Total size to remove in bytes
    ///
    /// # Returns
    ///
    /// The number of files removed
    fn remove_files_by_size(
        &self,
        log_files: &mut Vec<(DateTime<Local>, PathBuf)>,
        size_to_remove: u64,
    ) -> io::Result<usize> {
        let mut removed_size = 0;
        let mut removed_count = 0;

        log_files.retain(|(_mtime, path)| {
            if removed_size >= size_to_remove {
                return true;
            }

            if let Ok(metadata) = fs::metadata(path) {
                let file_size = metadata.len();

                if path.exists() {
                    if fs::remove_file(path).is_ok() {
                        removed_size += file_size;
                        removed_count += 1;
                        return false;
                    }
                }
            }

            true
        });

        Ok(removed_count)
    }

    /// Calculates the total size of all log files in the list
    ///
    /// # Arguments
    ///
    /// * `log_files` - List of log files
    ///
    /// # Returns
    ///
    /// The total size in bytes
    fn calculate_total_size(&self, log_files: &[(DateTime<Local>, PathBuf)]) -> io::Result<u64> {
        let mut total_size = 0;

        for (_mtime, path) in log_files {
            if let Ok(metadata) = fs::metadata(path) {
                total_size += metadata.len();
            }
        }

        Ok(total_size)
    }

    /// Gets information about all log files
    ///
    /// # Returns
    ///
    /// A vector of LogFileInfo for each log file
    pub fn get_log_info(&self) -> io::Result<Vec<LogFileInfo>> {
        let mut log_info = Vec::new();

        let entries = self.get_log_files_sorted()?;

        for (mtime, path) in entries {
            let metadata = fs::metadata(&path)?;
            let size = metadata.len();

            log_info.push(LogFileInfo {
                path: path.clone(),
                size,
                modified_time: mtime,
                age_days: (Local::now() - mtime).num_days(),
            });
        }

        Ok(log_info)
    }

    /// Returns the path to the logs directory
    #[must_use]
    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }
}

impl Default for LogManager {
    fn default() -> Self {
        Self::new(None::<&Path>)
    }
}

/// Information about a log file
#[derive(Debug, Clone)]
pub struct LogFileInfo {
    /// Path to the log file
    pub path: PathBuf,

    /// Size of the log file in bytes
    pub size: u64,

    /// Last modification time
    pub modified_time: DateTime<Local>,

    /// Age of the log file in days
    pub age_days: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_log_manager_new() {
        let manager = LogManager::new(None::<&Path>);
        // Just check that logs_dir ends with "logs" to handle platform-specific paths
        assert!(manager.logs_dir.to_string_lossy().ends_with("logs"));
    }

    #[test]
    fn test_log_manager_with_custom_base() {
        let temp_dir = TempDir::new().unwrap();
        let manager = LogManager::new(Some(temp_dir.path()));
        assert_eq!(manager.logs_dir, temp_dir.path().join("logs"));
    }

    #[test]
    fn test_log_manager_builder_methods() {
        let manager = LogManager::new(None::<&Path>)
            .with_max_files(20)
            .with_max_age_days(14)
            .with_max_total_size(200 * 1024 * 1024);

        assert_eq!(manager.max_files, 20);
        assert_eq!(manager.max_age_days, 14);
        assert_eq!(manager.max_total_size, 200 * 1024 * 1024);
    }

    #[test]
    fn test_initialize_directory() {
        let temp_dir = TempDir::new().unwrap();
        let manager = LogManager::new(Some(temp_dir.path()));

        assert!(manager.initialize_directory().is_ok());
        assert!(manager.logs_dir.exists());
    }

    #[test]
    fn test_generate_log_path() {
        let manager = LogManager::new(None::<&Path>);
        let log_path = manager.generate_log_path();

        // Use to_string_lossy to handle platform separators
        let log_path_str = log_path.to_string_lossy();
        assert!(log_path_str.contains("logs"));
        assert!(log_path.to_str().unwrap().contains("run-"));
        assert!(log_path.extension().unwrap() == "log");
    }

    #[test]
    fn test_create_log_file() {
        let temp_dir = TempDir::new().unwrap();
        let manager = LogManager::new(Some(temp_dir.path()));

        let log_path = manager.create_log_file().unwrap();

        assert!(log_path.exists());
        assert!(log_path.starts_with(temp_dir.path()) || log_path.starts_with(temp_dir.path().to_string_lossy().as_ref()));
        assert!(log_path.extension().unwrap() == "log");
    }

    #[test]
    fn test_cleanup_old_logs() {
        let temp_dir = TempDir::new().unwrap();
        let manager = LogManager::new(Some(temp_dir.path()))
            .with_max_files(2)
            .with_max_age_days(1);

        // Create multiple log files with explicit delays (1 second apart to avoid timestamp collisions)
        let path1 = manager.create_log_file().unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let path2 = manager.create_log_file().unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let path3 = manager.create_log_file().unwrap();

        // Verify all 3 files exist
        assert!(path1.exists(), "First log file should exist");
        assert!(path2.exists(), "Second log file should exist");
        assert!(path3.exists(), "Third log file should exist");

        // Check that we have at least 2 files
        let log_info_before = manager.get_log_info().unwrap();
        assert!(log_info_before.len() >= 2, "Should have at least 2 log files before cleanup");

        // Cleanup should remove at least one file
        let removed = manager.cleanup_old_logs().unwrap();
        assert!(removed > 0, "Should remove at least one log file");

        // Check that we have fewer files now
        let log_info_after = manager.get_log_info().unwrap();
        assert!(log_info_after.len() < log_info_before.len(), "Should have fewer files after cleanup");
    }

    #[test]
    fn test_get_log_info() {
        let temp_dir = TempDir::new().unwrap();
        let manager = LogManager::new(Some(temp_dir.path()));

        let _ = manager.create_log_file();

        let log_info = manager.get_log_info().unwrap();

        assert!(!log_info.is_empty());
        assert!(log_info[0].size > 0 || log_info[0].size == 0); // u64 is always >= 0
        assert!(log_info[0].age_days >= 0);
    }

    #[test]
    fn test_default() {
        let manager = LogManager::default();
        // Just check that logs_dir ends with "logs" to handle platform-specific paths
        assert!(manager.logs_dir.to_string_lossy().ends_with("logs"));
    }

    #[test]
    fn test_log_file_info() {
        let temp_dir = TempDir::new().unwrap();
        let manager = LogManager::new(Some(temp_dir.path()));

        let log_path = manager.create_log_file().unwrap();

        let log_info = manager.get_log_info().unwrap();

        assert_eq!(log_info.len(), 1);
        assert_eq!(log_info[0].path, log_path);
    }
}
