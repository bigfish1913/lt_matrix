//! Workspace state persistence module
//!
//! Provides functionality for saving and loading workspace state,
//! including task manifests and metadata.

use crate::models::{Task, TaskStatus};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Metadata about the workspace state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateMetadata {
    /// Version of the state format
    pub version: String,

    /// Timestamp when the state was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Timestamp when the state was last modified
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

impl Default for StateMetadata {
    fn default() -> Self {
        let now = chrono::Utc::now();
        StateMetadata {
            version: "1.0".to_string(),
            created_at: now,
            modified_at: now,
        }
    }
}

/// Represents the persistent state of a workspace
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceState {
    /// Path to the project root directory
    pub project_root: PathBuf,

    /// List of tasks in the workspace
    pub tasks: Vec<Task>,

    /// Metadata about the state
    pub metadata: StateMetadata,
}

impl WorkspaceState {
    /// Creates a new workspace state with the given project root and tasks
    pub fn new(project_root: PathBuf, tasks: Vec<Task>) -> Self {
        WorkspaceState {
            project_root,
            tasks,
            metadata: StateMetadata::default(),
        }
    }

    /// Returns the path to the tasks-manifest.json file
    pub fn manifest_path(&self) -> PathBuf {
        self.project_root
            .join(".ltmatrix")
            .join("tasks-manifest.json")
    }

    /// Saves the workspace state to disk
    ///
    /// Creates the .ltmatrix directory if it doesn't exist,
    /// serializes the state to JSON, and writes it to tasks-manifest.json.
    ///
    /// # Returns
    ///
    /// Returns `Ok(WorkspaceState)` with updated metadata on success,
    /// or an error if serialization or file operations fail.
    pub fn save(&self) -> Result<WorkspaceState, anyhow::Error> {
        // Create .ltmatrix directory if it doesn't exist
        let ltmatrix_dir = self.project_root.join(".ltmatrix");
        fs::create_dir_all(&ltmatrix_dir)?;

        // Update metadata
        let mut updated_state = self.clone();
        updated_state.metadata.modified_at = chrono::Utc::now();

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&updated_state)?;

        // Write to file
        let manifest_path = self.manifest_path();
        fs::write(&manifest_path, json)?;

        Ok(updated_state)
    }

    /// Loads workspace state from the tasks-manifest.json file
    ///
    /// # Arguments
    ///
    /// * `project_root` - Path to the project root directory
    ///
    /// # Returns
    ///
    /// Returns `Ok(WorkspaceState)` on success, or an error if:
    /// - The .ltmatrix directory doesn't exist
    /// - The tasks-manifest.json file doesn't exist
    /// - The file contains invalid JSON
    pub fn load(project_root: PathBuf) -> Result<WorkspaceState, anyhow::Error> {
        let manifest_path = project_root
            .join(".ltmatrix")
            .join("tasks-manifest.json");

        // Read the file with enhanced error context
        let json = fs::read_to_string(&manifest_path)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to read workspace manifest file at {:?}: {}. \
                    The file may not exist or may be corrupted. \
                    Use load_or_create() to automatically create a new state.",
                    manifest_path, e
                )
            })?;

        // Deserialize with enhanced error context
        let state: WorkspaceState = serde_json::from_str(&json)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse workspace manifest file at {:?}: {}. \
                    The file may be corrupted. Consider using load_or_create() to recover.",
                    manifest_path, e
                )
            })?;

        Ok(state)
    }

    /// Loads workspace state and transforms task statuses
    ///
    /// This method loads the workspace state and automatically resets
    /// InProgress and Blocked tasks to Pending. This is useful for recovery
    /// scenarios where tasks may have been left in an inconsistent state.
    ///
    /// # Arguments
    ///
    /// * `project_root` - Path to the project root directory
    ///
    /// # Returns
    ///
    /// Returns `Ok(WorkspaceState)` with transformed statuses on success,
    /// or an error if loading or transformation fails.
    pub fn load_with_transform(project_root: PathBuf) -> Result<WorkspaceState, anyhow::Error> {
        let manifest_path = project_root
            .join(".ltmatrix")
            .join("tasks-manifest.json");

        // Read the file with enhanced error context
        let json = fs::read_to_string(&manifest_path)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to read workspace manifest file at {:?}: {}. \
                    The file may not exist or may be corrupted. \
                    Use load_or_create() to automatically create a new state.",
                    manifest_path, e
                )
            })?;

        // Deserialize with enhanced error context
        let state: WorkspaceState = serde_json::from_str(&json)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse workspace manifest file at {:?}: {}. \
                    The file may be corrupted. Consider using load_or_create() to recover.",
                    manifest_path, e
                )
            })?;

        // Transform task statuses
        let transformed_state = Self::transform_task_states(state);

        Ok(transformed_state)
    }

    /// Transforms task statuses to reset inconsistent states
    ///
    /// Resets InProgress and Blocked tasks to Pending, preserving
    /// Completed, Failed, and Pending statuses.
    fn transform_task_states(mut state: WorkspaceState) -> WorkspaceState {
        for task in &mut state.tasks {
            Self::transform_task_status_recursive(task);
        }
        state
    }

    /// Loads workspace state, creating a new empty state if the file doesn't exist or is corrupted
    ///
    /// This method provides graceful fallback for corrupted or missing state files.
    /// It will attempt to load the existing state, but if that fails, it will
    /// create a new empty state instead.
    ///
    /// # Arguments
    ///
    /// * `project_root` - Path to the project root directory
    ///
    /// # Returns
    ///
    /// Returns `Ok(WorkspaceState)` - either loaded or newly created.
    /// Never returns an error - always provides a valid state.
    ///
    /// # Behavior
    ///
    /// - If the file exists and is valid: loads and returns the state
    /// - If the file doesn't exist: creates new empty state
    /// - If the file is corrupted: creates new empty state (data loss warning logged)
    pub fn load_or_create(project_root: PathBuf) -> Result<WorkspaceState, anyhow::Error> {
        // Try to load existing state
        let load_result = Self::load(project_root.clone());

        match load_result {
            Ok(state) => Ok(state),
            Err(e) => {
                // Log the corruption/error details
                tracing::warn!(
                    "Failed to load workspace state from {:?}: {}. Creating new empty state.",
                    project_root, e
                );

                // Create new empty state as fallback
                let new_state = WorkspaceState::new(project_root, vec![]);

                // Save the new empty state so it exists for next time
                if let Err(save_err) = new_state.save() {
                    tracing::warn!(
                        "Failed to save new empty workspace state: {}",
                        save_err
                    );
                }

                Ok(new_state)
            }
        }
    }

    /// Recursively transforms task status for a task and its subtasks
    fn transform_task_status_recursive(task: &mut Task) {
        // Transform current task status
        match task.status {
            TaskStatus::InProgress | TaskStatus::Blocked => {
                task.status = TaskStatus::Pending;
                // Clear timestamps when resetting
                task.started_at = None;
            }
            TaskStatus::Pending | TaskStatus::Completed | TaskStatus::Failed => {
                // Preserve these states
            }
        }

        // Recursively transform subtasks
        for subtask in &mut task.subtasks {
            Self::transform_task_status_recursive(subtask);
        }
    }

    /// Detects orphaned tasks in the workspace
    ///
    /// An orphaned task is one that depends on a non-existent task ID.
    /// This can happen when tasks are deleted but their dependencies are not cleaned up.
    ///
    /// # Returns
    ///
    /// Returns a vector of task IDs that have broken dependencies, along with
    /// the missing dependency IDs.
    pub fn detect_orphaned_tasks(&self) -> Vec<(String, Vec<String>)> {
        let mut orphaned = Vec::new();

        // Build a set of all valid task IDs (including subtasks)
        let mut valid_ids = std::collections::HashSet::new();
        for task in &self.tasks {
            Self::collect_task_ids_recursive(task, &mut valid_ids);
        }

        // Check each task for broken dependencies
        for task in &self.tasks {
            let missing_deps: Vec<String> = task
                .depends_on
                .iter()
                .filter(|dep_id| !valid_ids.contains(*dep_id))
                .cloned()
                .collect();

            if !missing_deps.is_empty() {
                orphaned.push((task.id.clone(), missing_deps));
            }

            // Check subtasks recursively
            Self::detect_orphaned_recursive(task, &valid_ids, &mut orphaned);
        }

        orphaned
    }

    /// Collects all task IDs (including subtasks) into a set
    fn collect_task_ids_recursive(task: &Task, ids: &mut std::collections::HashSet<String>) {
        ids.insert(task.id.clone());
        for subtask in &task.subtasks {
            Self::collect_task_ids_recursive(subtask, ids);
        }
    }

    /// Recursively detects orphaned tasks in subtasks
    fn detect_orphaned_recursive(
        task: &Task,
        valid_ids: &std::collections::HashSet<String>,
        orphaned: &mut Vec<(String, Vec<String>)>,
    ) {
        for subtask in &task.subtasks {
            let missing_deps: Vec<String> = subtask
                .depends_on
                .iter()
                .filter(|dep_id| !valid_ids.contains(*dep_id))
                .cloned()
                .collect();

            if !missing_deps.is_empty() {
                orphaned.push((subtask.id.clone(), missing_deps));
            }

            // Recurse into nested subtasks
            Self::detect_orphaned_recursive(subtask, valid_ids, orphaned);
        }
    }

    /// Cleans up orphaned tasks by removing invalid dependencies
    ///
    /// This method removes any dependencies that reference non-existent tasks.
    /// It returns the number of dependencies that were removed.
    ///
    /// # Returns
    ///
    /// Returns the total count of dependency references that were cleaned up.
    pub fn cleanup_orphaned_dependencies(&mut self) -> usize {
        let mut cleaned_count = 0;

        // Build a set of all valid task IDs
        let mut valid_ids = std::collections::HashSet::new();
        for task in &self.tasks {
            Self::collect_task_ids_recursive(task, &mut valid_ids);
        }

        // Clean up dependencies for each task
        for task in &mut self.tasks {
            cleaned_count += Self::cleanup_dependencies_recursive(task, &valid_ids);
        }

        cleaned_count
    }

    /// Recursively cleans up dependencies for a task and its subtasks
    fn cleanup_dependencies_recursive(
        task: &mut Task,
        valid_ids: &std::collections::HashSet<String>,
    ) -> usize {
        let original_len = task.depends_on.len();

        // Keep only valid dependencies
        task.depends_on.retain(|dep_id| valid_ids.contains(dep_id));

        let cleaned = original_len - task.depends_on.len();

        // Recurse into subtasks
        let subtask_cleaned: usize = task
            .subtasks
            .iter_mut()
            .map(|subtask| Self::cleanup_dependencies_recursive(subtask, valid_ids))
            .sum();

        cleaned + subtask_cleaned
    }

    /// Validates the task dependency graph
    ///
    /// Checks for:
    /// - Circular dependencies
    /// - Self-dependencies
    /// - Orphaned dependencies
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the graph is valid, or an error describing the issues found.
    pub fn validate_dependency_graph(&self) -> Result<(), anyhow::Error> {
        // Check for orphaned tasks
        let orphaned = self.detect_orphaned_tasks();
        if !orphaned.is_empty() {
            let mut error_msg = "Found tasks with orphaned dependencies:\n".to_string();
            for (task_id, missing_deps) in &orphaned {
                error_msg.push_str(&format!(
                    "  - Task '{}' depends on: {:?}\n",
                    task_id, missing_deps
                ));
            }
            return Err(anyhow::anyhow!(error_msg));
        }

        // Check for circular dependencies using DFS
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for task in &self.tasks {
            if let Err(cycle) = self.check_cycles_recursive(task, &mut visited, &mut rec_stack) {
                return Err(anyhow::anyhow!("Circular dependency detected: {}", cycle));
            }
        }

        Ok(())
    }

    /// Recursively checks for circular dependencies using DFS
    fn check_cycles_recursive(
        &self,
        task: &Task,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
    ) -> Result<(), String> {
        // Check if this task is already in the recursion stack (cycle detected)
        if rec_stack.contains(&task.id) {
            return Err(format!("cycle involving task '{}'", task.id));
        }

        // Skip if already visited
        if visited.contains(&task.id) {
            return Ok(());
        }

        // Mark as visited and add to recursion stack
        visited.insert(task.id.clone());
        rec_stack.insert(task.id.clone());

        // Check all dependencies
        for dep_id in &task.depends_on {
            // Find the dependency task
            let dep_task = Self::find_task_by_id_recursive(dep_id, &self.tasks)
                .ok_or_else(|| format!("dependency '{}' not found", dep_id))?;

            // Recursively check the dependency
            self.check_cycles_recursive_helper(dep_task, visited, rec_stack)?;
        }

        // Remove from recursion stack
        rec_stack.remove(&task.id);

        Ok(())
    }

    /// Helper for cycle checking that searches the entire task tree
    fn check_cycles_recursive_helper(
        &self,
        task: &Task,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
    ) -> Result<(), String> {
        if rec_stack.contains(&task.id) {
            return Err(format!("cycle involving task '{}'", task.id));
        }

        if visited.contains(&task.id) {
            return Ok(());
        }

        visited.insert(task.id.clone());
        rec_stack.insert(task.id.clone());

        for dep_id in &task.depends_on {
            let dep_task = Self::find_task_by_id_recursive(dep_id, &self.tasks)
                .ok_or_else(|| format!("dependency '{}' not found", dep_id))?;

            self.check_cycles_recursive_helper(dep_task, visited, rec_stack)?;
        }

        rec_stack.remove(&task.id);
        Ok(())
    }

    /// Finds a task by ID (including searching in subtasks)
    fn find_task_by_id_recursive<'a>(
        id: &str,
        tasks: &'a [Task],
    ) -> Option<&'a Task> {
        for task in tasks {
            if task.id == id {
                return Some(task);
            }
            if let Some(found) = Self::find_task_by_id_recursive(id, &task.subtasks) {
                return Some(found);
            }
        }
        None
    }

    /// Removes all workspace state files and directories
    ///
    /// This will delete the entire .ltmatrix directory including:
    /// - tasks-manifest.json
    /// - Any other state files
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if cleanup succeeded, or an error if deletion failed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::workspace::WorkspaceState;
    /// use std::path::PathBuf;
    ///
    /// let project_root = PathBuf::from("/path/to/project");
    /// WorkspaceState::cleanup(&project_root).unwrap();
    /// ```
    pub fn cleanup(project_root: &PathBuf) -> Result<(), anyhow::Error> {
        let ltmatrix_dir = project_root.join(".ltmatrix");

        // Check if directory exists
        if !ltmatrix_dir.exists() {
            return Ok(()); // Nothing to clean up
        }

        // Remove the directory and all its contents
        fs::remove_dir_all(&ltmatrix_dir)
            .map_err(|e| anyhow::anyhow!("Failed to remove .ltmatrix directory: {}", e))?;

        Ok(())
    }

    /// Checks if workspace state exists
    ///
    /// # Returns
    ///
    /// Returns `true` if the tasks-manifest.json file exists, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::workspace::WorkspaceState;
    /// use std::path::PathBuf;
    ///
    /// let project_root = PathBuf::from("/path/to/project");
    /// if WorkspaceState::exists(&project_root) {
    ///     println!("Workspace state exists");
    /// }
    /// ```
    pub fn exists(project_root: &PathBuf) -> bool {
        let manifest_path = project_root
            .join(".ltmatrix")
            .join("tasks-manifest.json");
        manifest_path.exists()
    }

    /// Resets all tasks to Pending status
    ///
    /// This method resets all tasks to Pending status, clearing any in-progress
    /// or blocked states. This is useful for manual intervention when you want
    /// to restart all tasks from scratch.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if reset succeeded, or an error if save failed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::workspace::WorkspaceState;
    /// use std::path::PathBuf;
    ///
    /// let project_root = PathBuf::from("/path/to/project");
    /// let mut state = WorkspaceState::load(project_root).unwrap();
    /// state.reset_all().unwrap();
    /// ```
    pub fn reset_all(&mut self) -> Result<(), anyhow::Error> {
        for task in &mut self.tasks {
            Self::reset_task_recursive(task);
        }
        Ok(())
    }

    /// Recursively resets a task and its subtasks to Pending
    fn reset_task_recursive(task: &mut Task) {
        // Reset current task
        task.status = TaskStatus::Pending;
        task.started_at = None;
        task.completed_at = None;
        task.error = None;
        // Note: We preserve retry_count and session_id for potential reuse

        // Recursively reset subtasks
        for subtask in &mut task.subtasks {
            Self::reset_task_recursive(subtask);
        }
    }

    /// Resets only failed tasks to Pending status
    ///
    /// This method resets only failed tasks, allowing you to retry failed tasks
    /// while preserving completed and pending tasks.
    ///
    /// # Returns
    ///
    /// Returns the number of tasks that were reset.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::workspace::WorkspaceState;
    /// use std::path::PathBuf;
    ///
    /// let project_root = PathBuf::from("/path/to/project");
    /// let mut state = WorkspaceState::load(project_root).unwrap();
    /// let reset_count = state.reset_failed().unwrap();
    /// println!("Reset {} failed tasks", reset_count);
    /// ```
    pub fn reset_failed(&mut self) -> Result<usize, anyhow::Error> {
        let mut count = 0;
        for task in &mut self.tasks {
            count += Self::reset_failed_recursive(task);
        }
        Ok(count)
    }

    /// Recursively resets failed tasks and returns count
    fn reset_failed_recursive(task: &mut Task) -> usize {
        let mut count = 0;

        // Reset failed tasks
        if task.status == TaskStatus::Failed {
            task.status = TaskStatus::Pending;
            task.started_at = None;
            task.completed_at = None;
            task.error = None;
            count += 1;
        }

        // Recursively process subtasks
        for subtask in &mut task.subtasks {
            count += Self::reset_failed_recursive(subtask);
        }

        count
    }

    /// Gets a summary of task statuses
    ///
    /// # Returns
    ///
    /// Returns a summary with counts of each task status.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::workspace::WorkspaceState;
    /// use std::path::PathBuf;
    ///
    /// let project_root = PathBuf::from("/path/to/project");
    /// let state = WorkspaceState::load(project_root).unwrap();
    /// let summary = state.status_summary();
    /// println!("Completed: {}, Pending: {}, Failed: {}",
    ///          summary.completed, summary.pending, summary.failed);
    /// ```
    pub fn status_summary(&self) -> TaskStatusSummary {
        let mut summary = TaskStatusSummary::default();

        for task in &self.tasks {
            Self::summarize_task_recursive(task, &mut summary);
        }

        summary
    }

    /// Recursively summarizes task statuses
    fn summarize_task_recursive(task: &Task, summary: &mut TaskStatusSummary) {
        match task.status {
            TaskStatus::Pending => summary.pending += 1,
            TaskStatus::InProgress => summary.in_progress += 1,
            TaskStatus::Completed => summary.completed += 1,
            TaskStatus::Failed => summary.failed += 1,
            TaskStatus::Blocked => summary.blocked += 1,
        }

        // Recursively count subtasks
        for subtask in &task.subtasks {
            Self::summarize_task_recursive(subtask, summary);
        }
    }
}

/// Summary of task statuses in the workspace
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskStatusSummary {
    /// Number of pending tasks
    pub pending: usize,

    /// Number of in-progress tasks
    pub in_progress: usize,

    /// Number of completed tasks
    pub completed: usize,

    /// Number of failed tasks
    pub failed: usize,

    /// Number of blocked tasks
    pub blocked: usize,

    /// Total number of tasks
    pub total: usize,
}

impl TaskStatusSummary {
    /// Returns the total count of all tasks
    pub fn total(&self) -> usize {
        self.pending + self.in_progress + self.completed + self.failed + self.blocked
    }

    /// Returns the percentage of completed tasks (0.0 to 100.0)
    pub fn completion_percentage(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            (self.completed as f64 / total as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_metadata_default() {
        let metadata = StateMetadata::default();
        assert_eq!(metadata.version, "1.0");
        assert_eq!(metadata.created_at, metadata.modified_at);
    }
}
