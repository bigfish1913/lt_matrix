// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Run-level memory management
//!
//! This module provides run-scoped memory storage that tracks:
//! - Agent sessions and their states
//! - Context decisions made during execution
//! - Task execution history for the current run
//! - Session reuse patterns
//!
//! Memory is stored at `.ltmatrix/memory/run-{session-id}.json`

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Run-level memory structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMemory {
    /// Unique run identifier
    pub run_id: String,

    /// Schema version
    pub version: String,

    /// When this run started
    pub started_at: DateTime<Utc>,

    /// When this run ended (if completed)
    pub ended_at: Option<DateTime<Utc>>,

    /// Run status
    pub status: RunStatus,

    /// Execution mode for this run
    pub execution_mode: Option<String>,

    /// Agent sessions used in this run
    pub agent_sessions: HashMap<String, AgentSessionInfo>,

    /// Context decisions made
    pub context_decisions: Vec<ContextDecision>,

    /// Task execution history
    pub task_history: Vec<TaskExecutionRecord>,

    /// Session reuse statistics
    pub session_stats: SessionStats,

    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Run status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Completed,
    Failed,
    Aborted,
}

/// Information about an agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionInfo {
    /// Session ID
    pub session_id: String,

    /// Agent type (plan, dev, test, review)
    pub agent_type: String,

    /// When the session was created
    pub created_at: DateTime<Utc>,

    /// Number of times this session was used
    pub use_count: u32,

    /// Tasks executed with this session
    pub task_ids: Vec<String>,

    /// Whether this session is still active
    pub is_active: bool,
}

/// A context decision made during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextDecision {
    /// Decision ID
    pub id: String,

    /// When the decision was made
    pub timestamp: DateTime<Utc>,

    /// The context or question
    pub context: String,

    /// The decision made
    pub decision: String,

    /// Reasoning for the decision
    pub reasoning: Option<String>,

    /// Related task ID
    pub related_task: Option<String>,

    /// Whether this decision should be remembered for future runs
    pub persist_to_project: bool,
}

/// Record of a task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionRecord {
    /// Task ID
    pub task_id: String,

    /// Task title
    pub title: String,

    /// Agent type used
    pub agent_type: String,

    /// Session ID used (if any)
    pub session_id: Option<String>,

    /// When execution started
    pub started_at: DateTime<Utc>,

    /// When execution ended
    pub ended_at: Option<DateTime<Utc>>,

    /// Execution duration in seconds
    pub duration_secs: Option<u64>,

    /// Final status
    pub status: String,

    /// Number of retries
    pub retry_count: u32,

    /// Key outcomes
    pub outcomes: Vec<String>,

    /// Files modified
    pub files_modified: Vec<String>,
}

/// Session reuse statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionStats {
    /// Total sessions created
    pub total_sessions: u32,

    /// Sessions reused
    pub sessions_reused: u32,

    /// Average session use count
    pub avg_use_count: f64,

    /// Session reuse ratio
    pub reuse_ratio: f64,
}

impl Default for RunMemory {
    fn default() -> Self {
        RunMemory {
            run_id: Uuid::new_v4().to_string(),
            version: "1.0.0".to_string(),
            started_at: Utc::now(),
            ended_at: None,
            status: RunStatus::Running,
            execution_mode: None,
            agent_sessions: HashMap::new(),
            context_decisions: Vec::new(),
            task_history: Vec::new(),
            session_stats: SessionStats::default(),
            metadata: HashMap::new(),
        }
    }
}

impl RunMemory {
    /// Create a new run memory
    pub fn new() -> Self {
        info!("Created new run memory with ID");
        Self::default()
    }

    /// Create a new run memory with a specific execution mode
    pub fn with_mode(mode: impl Into<String>) -> Self {
        RunMemory {
            execution_mode: Some(mode.into()),
            ..Self::default()
        }
    }

    /// Load run memory from file
    pub async fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            debug!("Run memory file not found at {:?}, creating new", path);
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path)
            .await
            .context("Failed to read run memory file")?;

        let memory: RunMemory = serde_json::from_str(&content)
            .context("Failed to parse run memory JSON")?;

        info!("Loaded run memory from {:?}", path);
        Ok(memory)
    }

    /// Save run memory to file
    pub async fn save(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create memory directory")?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize run memory")?;

        fs::write(path, content)
            .await
            .context("Failed to write run memory file")?;

        debug!("Saved run memory to {:?}", path);
        Ok(())
    }

    /// Record an agent session
    pub fn record_session(
        &mut self,
        session_id: impl Into<String>,
        agent_type: impl Into<String>,
    ) {
        let session_id = session_id.into();
        let agent_type = agent_type.into();

        if let Some(session) = self.agent_sessions.get_mut(&session_id) {
            session.use_count += 1;
        } else {
            self.agent_sessions.insert(
                session_id.clone(),
                AgentSessionInfo {
                    session_id: session_id.clone(),
                    agent_type,
                    created_at: Utc::now(),
                    use_count: 1,
                    task_ids: Vec::new(),
                    is_active: true,
                },
            );
        }
        self.update_session_stats();
    }

    /// Associate a task with a session
    pub fn associate_task_with_session(&mut self, session_id: &str, task_id: &str) {
        if let Some(session) = self.agent_sessions.get_mut(session_id) {
            if !session.task_ids.contains(&task_id.to_string()) {
                session.task_ids.push(task_id.to_string());
            }
        }
    }

    /// Record a context decision
    pub fn record_decision(&mut self, decision: ContextDecision) {
        self.context_decisions.push(decision);
    }

    /// Record a task execution
    pub fn record_task_execution(&mut self, record: TaskExecutionRecord) {
        self.task_history.push(record);
    }

    /// Mark a session as inactive
    pub fn deactivate_session(&mut self, session_id: &str) {
        if let Some(session) = self.agent_sessions.get_mut(session_id) {
            session.is_active = false;
        }
    }

    /// Mark the run as completed
    pub fn complete(&mut self) {
        self.status = RunStatus::Completed;
        self.ended_at = Some(Utc::now());
        self.update_session_stats();
    }

    /// Mark the run as failed
    pub fn fail(&mut self) {
        self.status = RunStatus::Failed;
        self.ended_at = Some(Utc::now());
    }

    /// Update session statistics
    fn update_session_stats(&mut self) {
        let total = self.agent_sessions.len() as u32;
        if total == 0 {
            self.session_stats = SessionStats::default();
            return;
        }

        let total_uses: u32 = self.agent_sessions.values().map(|s| s.use_count).sum();
        let reused = self.agent_sessions.values().filter(|s| s.use_count > 1).count() as u32;

        self.session_stats = SessionStats {
            total_sessions: total,
            sessions_reused: reused,
            avg_use_count: total_uses as f64 / total as f64,
            reuse_ratio: reused as f64 / total as f64,
        };
    }

    /// Get decisions that should be persisted to project memory
    pub fn get_persistent_decisions(&self) -> Vec<&ContextDecision> {
        self.context_decisions
            .iter()
            .filter(|d| d.persist_to_project)
            .collect()
    }

    /// Generate a summary of this run
    pub fn generate_summary(&self) -> String {
        let mut summary = String::new();

        summary.push_str(&format!("# Run: {}\n\n", self.run_id));
        summary.push_str(&format!("**Status**: {:?}\n", self.status));
        summary.push_str(&format!("**Started**: {}\n", self.started_at.format("%Y-%m-%d %H:%M:%S")));

        if let Some(ref mode) = self.execution_mode {
            summary.push_str(&format!("**Mode**: {}\n", mode));
        }

        if let Some(ended) = self.ended_at {
            let duration = (ended - self.started_at).num_seconds();
            summary.push_str(&format!("**Duration**: {}s\n", duration));
        }

        // Session stats
        summary.push_str(&format!(
            "\n## Sessions\n- Total: {}\n- Reused: {}\n- Reuse ratio: {:.1}%\n",
            self.session_stats.total_sessions,
            self.session_stats.sessions_reused,
            self.session_stats.reuse_ratio * 100.0
        ));

        // Task summary
        if !self.task_history.is_empty() {
            let completed = self.task_history.iter().filter(|t| t.status == "completed").count();
            let failed = self.task_history.iter().filter(|t| t.status == "failed").count();
            summary.push_str(&format!(
                "\n## Tasks\n- Total: {}\n- Completed: {}\n- Failed: {}\n",
                self.task_history.len(),
                completed,
                failed
            ));
        }

        // Key decisions
        if !self.context_decisions.is_empty() {
            summary.push_str(&format!("\n## Decisions ({})\n", self.context_decisions.len()));
            for decision in self.context_decisions.iter().take(5) {
                summary.push_str(&format!("- {}\n", decision.decision));
            }
        }

        summary
    }
}

/// Get the run memory path for a specific run ID
pub fn get_run_memory_path(project_root: &Path, run_id: &str) -> PathBuf {
    project_root
        .join(".ltmatrix")
        .join("memory")
        .join(format!("run-{}.json", run_id))
}

/// Get the current run memory path (creates new run ID if needed)
pub fn get_current_run_memory_path(project_root: &Path) -> PathBuf {
    get_run_memory_path(project_root, &Uuid::new_v4().to_string())
}

/// Clean up old run memory files (keep last N)
pub async fn cleanup_old_run_memories(project_root: &Path, keep_count: usize) -> Result<Vec<PathBuf>> {
    let memory_dir = project_root.join(".ltmatrix").join("memory");

    if !memory_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(&memory_dir)
        .await
        .context("Failed to read memory directory")?;

    let mut run_files: Vec<(PathBuf, DateTime<Utc>)> = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("run-") && name.ends_with(".json") {
                // Try to get modification time
                let metadata = entry.metadata().await?;
                let modified: DateTime<Utc> = metadata
                    .modified()
                    .map(|t| t.into())
                    .unwrap_or_else(|_| Utc::now());
                run_files.push((path, modified));
            }
        }
    }

    // Sort by modification time (newest first)
    run_files.sort_by(|a, b| b.1.cmp(&a.1));

    // Remove old files
    let mut removed = Vec::new();
    for (path, _) in run_files.into_iter().skip(keep_count) {
        debug!("Removing old run memory: {:?}", path);
        fs::remove_file(&path).await?;
        removed.push(path);
    }

    if !removed.is_empty() {
        info!("Cleaned up {} old run memory files", removed.len());
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_run_memory_new() {
        let memory = RunMemory::new();
        assert_eq!(memory.status, RunStatus::Running);
        assert!(memory.agent_sessions.is_empty());
    }

    #[test]
    fn test_record_session() {
        let mut memory = RunMemory::new();
        memory.record_session("session-1", "dev");

        assert_eq!(memory.agent_sessions.len(), 1);
        assert_eq!(memory.session_stats.total_sessions, 1);
    }

    #[test]
    fn test_session_reuse() {
        let mut memory = RunMemory::new();
        memory.record_session("session-1", "dev");
        memory.record_session("session-1", "dev"); // Reuse

        assert_eq!(memory.agent_sessions.len(), 1);
        assert_eq!(memory.session_stats.sessions_reused, 1);
    }

    #[test]
    fn test_record_decision() {
        let mut memory = RunMemory::new();
        let decision = ContextDecision {
            id: "decision-1".to_string(),
            timestamp: Utc::now(),
            context: "Should we use async?".to_string(),
            decision: "Use Tokio for async runtime".to_string(),
            reasoning: Some("Better ecosystem support".to_string()),
            related_task: Some("task-1".to_string()),
            persist_to_project: true,
        };

        memory.record_decision(decision);
        assert_eq!(memory.context_decisions.len(), 1);
        assert_eq!(memory.get_persistent_decisions().len(), 1);
    }

    #[test]
    fn test_complete_run() {
        let mut memory = RunMemory::new();
        memory.complete();

        assert_eq!(memory.status, RunStatus::Completed);
        assert!(memory.ended_at.is_some());
    }

    #[test]
    fn test_generate_summary() {
        let mut memory = RunMemory::with_mode("standard");
        memory.record_session("session-1", "dev");
        memory.record_task_execution(TaskExecutionRecord {
            task_id: "task-1".to_string(),
            title: "Test task".to_string(),
            agent_type: "dev".to_string(),
            session_id: Some("session-1".to_string()),
            started_at: Utc::now(),
            ended_at: Some(Utc::now()),
            duration_secs: Some(60),
            status: "completed".to_string(),
            retry_count: 0,
            outcomes: vec!["Done".to_string()],
            files_modified: vec!["src/main.rs".to_string()],
        });

        let summary = memory.generate_summary();
        assert!(summary.contains("standard"));
        assert!(summary.contains("Sessions"));
        assert!(summary.contains("Tasks"));
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("run-test.json");

        let mut memory = RunMemory::new();
        memory.record_session("session-1", "dev");
        memory.save(&path).await.unwrap();

        let loaded = RunMemory::load(&path).await.unwrap();
        assert_eq!(loaded.agent_sessions.len(), 1);
    }

    #[tokio::test]
    async fn test_cleanup_old_memories() {
        let dir = tempdir().unwrap();
        let memory_dir = dir.path().join(".ltmatrix").join("memory");
        fs::create_dir_all(&memory_dir).await.unwrap();

        // Create multiple run files
        for i in 0..5 {
            let path = memory_dir.join(format!("run-{}.json", i));
            let memory = RunMemory::new();
            memory.save(&path).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Keep only 2
        let removed = cleanup_old_run_memories(dir.path(), 2).await.unwrap();
        assert_eq!(removed.len(), 3);
    }
}
