//! Session management for agent backends
//!
//! This module provides session file management for agent subprocess communication,
//! allowing session reuse across retry attempts and dependent task chains.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

/// Session information stored on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// Unique session identifier
    pub session_id: String,

    /// Agent backend name
    pub agent_name: String,

    /// Model being used
    pub model: String,

    /// Session creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last access timestamp
    pub last_accessed: chrono::DateTime<chrono::Utc>,

    /// Number of times this session has been reused
    pub reuse_count: u32,

    /// Session file path
    #[serde(skip)]
    pub file_path: PathBuf,
}

impl SessionData {
    /// Create a new session data structure
    pub fn new(agent_name: impl Into<String>, model: impl Into<String>) -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        SessionData {
            session_id,
            agent_name: agent_name.into(),
            model: model.into(),
            created_at: now,
            last_accessed: now,
            reuse_count: 0,
            file_path: PathBuf::new(),
        }
    }

    /// Update the last accessed time and increment reuse count
    pub fn mark_accessed(&mut self) {
        self.last_accessed = chrono::Utc::now();
        self.reuse_count += 1;
    }

    /// Check if this session is stale (older than 1 hour)
    pub fn is_stale(&self) -> bool {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(self.last_accessed);
        duration.num_seconds() > 3600 // 1 hour
    }
}

/// Manages session files for agent subprocess communication
#[derive(Debug, Clone)]
pub struct SessionManager {
    /// Base directory for session files
    pub sessions_dir: PathBuf,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        let sessions_dir = base_dir.as_ref().join(".ltmatrix").join("sessions");

        // Ensure sessions directory exists
        std::fs::create_dir_all(&sessions_dir).context("Failed to create sessions directory")?;

        info!(
            "Session manager initialized with directory: {:?}",
            sessions_dir
        );

        Ok(SessionManager { sessions_dir })
    }

    /// Create a default session manager in the current directory
    pub fn default_manager() -> Result<Self> {
        Self::new(std::env::current_dir()?)
    }

    /// Create a new session file
    pub async fn create_session(
        &self,
        agent_name: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<SessionData> {
        let mut session = SessionData::new(agent_name, model);

        // Generate unique filename
        let filename = format!("{}-{}.json", session.agent_name, session.session_id);
        let file_path = self.sessions_dir.join(&filename);
        session.file_path = file_path.clone();

        // Write session data to file
        self.save_session(&session).await?;

        debug!(
            "Created new session: {} ({:?})",
            session.session_id, file_path
        );

        Ok(session)
    }

    /// Load an existing session by ID
    pub async fn load_session(&self, session_id: &str) -> Result<Option<SessionData>> {
        // Find session file by ID
        let entry = match self.find_session_file(session_id).await? {
            Some(entry) => entry,
            None => return Ok(None),
        };

        // Read and parse session file
        let content = fs::read_to_string(&entry.path())
            .await
            .context("Failed to read session file")?;

        let mut session: SessionData =
            serde_json::from_str(&content).context("Failed to parse session file")?;

        session.file_path = entry.path().to_path_buf();
        session.mark_accessed();

        debug!(
            "Loaded session: {} (reused {} times)",
            session.session_id, session.reuse_count
        );

        // Save updated access time
        self.save_session(&session).await?;

        Ok(Some(session))
    }

    /// Save session data to disk
    pub async fn save_session(&self, session: &SessionData) -> Result<()> {
        let content =
            serde_json::to_string_pretty(session).context("Failed to serialize session data")?;

        fs::write(&session.file_path, content)
            .await
            .context("Failed to write session file")?;

        Ok(())
    }

    /// Delete a session file
    pub async fn delete_session(&self, session_id: &str) -> Result<bool> {
        let entry = match self.find_session_file(session_id).await? {
            Some(entry) => entry,
            None => return Ok(false),
        };

        fs::remove_file(entry.path())
            .await
            .context("Failed to delete session file")?;

        debug!("Deleted session: {}", session_id);

        Ok(true)
    }

    /// Clean up stale session files
    pub async fn cleanup_stale_sessions(&self) -> Result<usize> {
        let mut entries = fs::read_dir(&self.sessions_dir)
            .await
            .context("Failed to read sessions directory")?;

        let mut cleaned = 0;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only process JSON files
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            // Try to load and check if stale
            match fs::read_to_string(&path).await {
                Ok(content) => {
                    match serde_json::from_str::<SessionData>(&content) {
                        Ok(session) if session.is_stale() => {
                            fs::remove_file(&path)
                                .await
                                .context("Failed to remove stale session")?;
                            cleaned += 1;
                            debug!("Removed stale session: {}", session.session_id);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            warn!("Failed to parse session file {:?}: {}", path, e);
                            // Remove malformed session files
                            fs::remove_file(&path).await.ok();
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to read session file {:?}: {}", path, e);
                }
            }
        }

        if cleaned > 0 {
            info!("Cleaned up {} stale session(s)", cleaned);
        }

        Ok(cleaned)
    }

    /// Find a session file by session ID
    async fn find_session_file(&self, session_id: &str) -> Result<Option<fs::DirEntry>> {
        let mut entries = fs::read_dir(&self.sessions_dir)
            .await
            .context("Failed to read sessions directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Check if filename contains session ID
            if path.to_string_lossy().contains(session_id) {
                return Ok(Some(entry));
            }
        }

        Ok(None)
    }

    /// Get all session files
    pub async fn list_sessions(&self) -> Result<Vec<SessionData>> {
        let mut entries = fs::read_dir(&self.sessions_dir)
            .await
            .context("Failed to read sessions directory")?;

        let mut sessions = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let content = match fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut session: SessionData = match serde_json::from_str(&content) {
                Ok(s) => s,
                Err(_) => continue,
            };

            session.file_path = path;
            sessions.push(session);
        }

        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(temp_dir.path()).unwrap();

        assert!(manager.sessions_dir.exists());
    }

    #[tokio::test]
    async fn test_session_creation_and_loading() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(temp_dir.path()).unwrap();

        // Create session
        let session = manager
            .create_session("claude", "claude-sonnet-4-6")
            .await
            .unwrap();

        // Load session
        let loaded = manager
            .load_session(&session.session_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(loaded.session_id, session.session_id);
        assert_eq!(loaded.agent_name, "claude");
        assert_eq!(loaded.model, "claude-sonnet-4-6");
    }

    #[tokio::test]
    async fn test_session_reuse_count() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(temp_dir.path()).unwrap();

        let mut session = manager
            .create_session("claude", "claude-sonnet-4-6")
            .await
            .unwrap();
        assert_eq!(session.reuse_count, 0);

        session.mark_accessed();
        assert_eq!(session.reuse_count, 1);
    }
}
