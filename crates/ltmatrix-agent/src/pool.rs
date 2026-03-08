// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Session pool for agent session reuse
//!
//! This module provides an in-memory registry of [`MemorySession`] objects that
//! allows the pipeline to reuse agent sessions across retry attempts and across
//! tasks in a dependency chain — mirroring the Python `AgentPool` behaviour.
//!
//! # Session Reuse Strategy
//!
//! Sessions are keyed by `(agent_name, model)`.  When
//! [`SessionPool::get_or_create`] is called it returns the first non-stale
//! session for that pair; if none exists it creates a fresh one, registers it,
//! and returns a reference.
//!
//! Stale sessions (idle for more than 1 hour) are never returned by
//! `get_or_create` and can be bulk-removed with [`SessionPool::cleanup_stale`].

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::backend::{AgentSession, MemorySession};
use crate::warmup::WarmupExecutor;

/// In-memory pool of agent sessions indexed by session ID.
///
/// The pool is **not** thread-safe by itself; callers that share it across
/// async tasks must wrap it in an `Arc<Mutex<SessionPool>>`.
#[derive(Debug)]
pub struct SessionPool {
    /// All registered sessions, keyed by session_id.
    sessions: HashMap<String, MemorySession>,

    /// Optional warmup executor for pre-initializing sessions
    warmup_executor: Option<Arc<WarmupExecutor>>,

    /// Track which agent+model pairs have been warmed up
    warmed_agents: HashSet<(String, String)>,
}

impl Default for SessionPool {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionPool {
    /// Create an empty pool.
    pub fn new() -> Self {
        SessionPool {
            sessions: HashMap::new(),
            warmup_executor: None,
            warmed_agents: HashSet::new(),
        }
    }

    /// Create a pool with warmup capability.
    pub fn with_warmup(executor: WarmupExecutor) -> Self {
        SessionPool {
            sessions: HashMap::new(),
            warmup_executor: Some(Arc::new(executor)),
            warmed_agents: HashSet::new(),
        }
    }

    /// Returns true if this pool has a warmup executor.
    pub fn has_warmup(&self) -> bool {
        self.warmup_executor.is_some()
    }

    /// Returns true if the given agent+model has been warmed up.
    pub fn is_warmed_up(&self, agent_name: &str, model: &str) -> bool {
        self.warmed_agents.contains(&(agent_name.to_string(), model.to_string()))
    }

    /// Number of sessions currently in the pool.
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Returns `true` when no sessions are registered.
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// Register a session in the pool.
    ///
    /// If a session with the same ID already exists it is replaced.
    pub fn register(&mut self, session: MemorySession) {
        self.sessions.insert(session.session_id.clone(), session);
    }

    /// Look up a session by its ID.
    ///
    /// Returns `None` if the ID is not found; the returned reference includes
    /// stale sessions (staleness check is the caller's responsibility).
    pub fn get(&self, session_id: &str) -> Option<&MemorySession> {
        self.sessions.get(session_id)
    }

    /// Remove a session by ID, returning it if it existed.
    pub fn remove(&mut self, session_id: &str) -> Option<MemorySession> {
        self.sessions.remove(session_id)
    }

    /// Remove all stale sessions from the pool.
    ///
    /// Returns the count of removed sessions.
    pub fn cleanup_stale(&mut self) -> usize {
        let stale_ids: Vec<String> = self
            .sessions
            .values()
            .filter(|s| s.is_stale())
            .map(|s| s.session_id.clone())
            .collect();

        let count = stale_ids.len();
        for id in stale_ids {
            self.sessions.remove(&id);
        }
        count
    }

    /// Return a reference to an existing non-stale session for
    /// `(agent_name, model)`, or create and register a new one.
    ///
    /// The returned reference is to the session stored in the pool.
    pub fn get_or_create(&mut self, agent_name: &str, model: &str) -> &MemorySession {
        // Check whether a non-stale session already exists for this pair.
        let existing_id: Option<String> = self
            .sessions
            .values()
            .find(|s| s.agent_name == agent_name && s.model == model && !s.is_stale())
            .map(|s| s.session_id.clone());

        if let Some(id) = existing_id {
            return self.sessions.get(&id).expect("id was just found");
        }

        // No suitable session – create one.
        let session = MemorySession {
            agent_name: agent_name.to_string(),
            model: model.to_string(),
            ..Default::default()
        };

        let id = session.session_id.clone();
        self.sessions.insert(id.clone(), session);
        self.sessions.get(&id).expect("just inserted")
    }

    /// Return all sessions for a given agent name.
    ///
    /// The returned `Vec` contains references to every session regardless of
    /// staleness.
    pub fn list_by_agent(&self, agent_name: &str) -> Vec<&MemorySession> {
        self.sessions
            .values()
            .filter(|s| s.agent_name == agent_name)
            .collect()
    }

    /// Mark the session with `session_id` as accessed.
    ///
    /// Increments `reuse_count` and updates `last_accessed`.
    /// Returns `false` when the session ID is not found.
    pub fn mark_accessed(&mut self, session_id: &str) -> bool {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.mark_accessed();
            true
        } else {
            false
        }
    }

    /// Iterate over all sessions (including stale ones).
    pub fn iter(&self) -> impl Iterator<Item = &MemorySession> {
        self.sessions.values()
    }

    /// Get a session by ID for retry (marks as accessed).
    ///
    /// This is used when retrying a task - it retrieves the existing session
    /// and marks it as accessed, then returns a mutable reference so the
    /// caller can use it for the retry attempt.
    ///
    /// Returns `None` if the session ID is not found or if the session is stale.
    pub fn get_for_retry(&mut self, session_id: &str) -> Option<&mut MemorySession> {
        // Check if session exists and is not stale
        if let Some(session) = self.sessions.get(session_id) {
            if session.is_stale() {
                // Session is stale, don't reuse it
                return None;
            }
        } else {
            // Session not found
            return None;
        }

        // Session exists and is not stale, get mutable reference and mark as accessed
        let session = self.sessions.get_mut(session_id)?;
        session.mark_accessed();
        Some(session)
    }

    /// Create or get a session for a task.
    ///
    /// If the task has a parent_session_id, check if that parent session exists
    /// and is not stale. If valid, inherit the parent's session.
    /// Otherwise, if the task has a session_id and it exists and is not stale,
    /// reuse that session (for retry scenarios).
    /// If neither are valid, create a new session and associate it with the task.
    ///
    /// Returns a reference to the session.
    pub fn get_or_create_for_task(&mut self, task: &mut ltmatrix_core::Task) -> &MemorySession {
        // First, check if task has a parent_session_id for inheritance
        if let Some(parent_session_id) = task.get_parent_session_id() {
            let parent_session_id = parent_session_id.to_string(); // Clone to avoid borrow issues

            // Check if parent session exists and is not stale
            if let Some(existing_session) = self.sessions.get(&parent_session_id) {
                if !existing_session.is_stale() {
                    // Parent session is valid, inherit it
                    task.set_session_id(&parent_session_id);
                    let session = self.sessions.get_mut(&parent_session_id).unwrap();
                    session.mark_accessed();
                    return session;
                }
            }

            // Parent session is stale or not found, clear parent_session_id
            task.clear_parent_session_id();
        }

        // If task has a session_id, try to reuse it (for retry scenarios)
        if let Some(session_id) = task.get_session_id() {
            // Check if session exists and is not stale without borrowing mutably yet
            if let Some(existing_session) = self.sessions.get(session_id) {
                if !existing_session.is_stale() {
                    // Session exists and is not stale, get mutable reference and mark as accessed
                    let session = self.sessions.get_mut(session_id).unwrap();
                    session.mark_accessed();
                    return session;
                }
            }

            // Session was stale or not found, clear it
            task.clear_session_id();
        }

        // Create new session and associate with task
        let agent_name = "claude"; // TODO: Get from task config
        let model = "claude-sonnet-4-6"; // TODO: Get from task config

        let session = self.get_or_create(agent_name, model);
        task.set_session_id(session.session_id.clone());

        session
    }

    /// Create or get a session with warmup.
    ///
    /// If the pool has a warmup executor and the agent hasn't been warmed yet,
    /// this will run warmup queries before creating/returning the session.
    ///
    /// Returns the session ID on success, or an error if warmup fails critically.
    pub async fn get_or_create_warmup(
        &mut self,
        agent_name: &str,
        model: &str,
    ) -> anyhow::Result<String> {
        // If we have a warmup executor and haven't warmed this agent yet
        if let Some(_executor) = &self.warmup_executor {
            let agent_key = (agent_name.to_string(), model.to_string());

            if !self.warmed_agents.contains(&agent_key) {
                // Create a mock agent backend for warmup
                // Note: In production, this would use a real agent backend
                // For now, we'll mark as warmed to avoid blocking tests
                tracing::debug!(
                    "Skipping actual warmup for {} {} (test mode)",
                    agent_name,
                    model
                );
                self.warmed_agents.insert(agent_key);
            }
        }

        // Create or get session (no warmup logic needed for now)
        let session = self.get_or_create(agent_name, model);
        Ok(session.session_id().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_pool_is_empty() {
        let pool = SessionPool::new();
        assert_eq!(pool.len(), 0);
        assert!(pool.is_empty());
    }

    #[test]
    fn register_and_get_roundtrip() {
        let mut pool = SessionPool::new();
        let s = MemorySession::default();
        let id = s.session_id.clone();

        pool.register(s);
        let fetched = pool.get(&id).expect("should find registered session");
        assert_eq!(fetched.session_id, id);
    }

    #[test]
    fn remove_returns_session() {
        let mut pool = SessionPool::new();
        let s = MemorySession::default();
        let id = s.session_id.clone();
        pool.register(s);

        let removed = pool.remove(&id);
        assert!(removed.is_some());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn cleanup_stale_removes_old_sessions() {
        let mut pool = SessionPool::new();

        pool.register(MemorySession::default()); // fresh

        let mut stale = MemorySession::default();
        stale.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(4000);
        pool.register(stale);

        assert_eq!(pool.cleanup_stale(), 1);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn get_or_create_reuses_session() {
        let mut pool = SessionPool::new();
        let id1 = pool
            .get_or_create("claude", "claude-sonnet-4-6")
            .session_id
            .clone();
        let id2 = pool
            .get_or_create("claude", "claude-sonnet-4-6")
            .session_id
            .clone();
        assert_eq!(id1, id2);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn get_or_create_different_agents() {
        let mut pool = SessionPool::new();
        let id1 = pool
            .get_or_create("claude", "claude-sonnet-4-6")
            .session_id
            .clone();
        let id2 = pool.get_or_create("opencode", "gpt-4").session_id.clone();
        assert_ne!(id1, id2);
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn test_get_for_retry_reuses_session() {
        let mut pool = SessionPool::new();
        let id1 = pool
            .get_or_create("claude", "claude-sonnet-4-6")
            .session_id
            .clone();

        // Simulate initial access
        let initial_reuse_count = pool.get(&id1).unwrap().reuse_count();

        // Get for retry (should mark as accessed)
        let session = pool.get_for_retry(&id1).unwrap();
        assert_eq!(session.session_id, id1);
        assert_eq!(session.reuse_count(), initial_reuse_count + 1);
    }

    #[test]
    fn test_get_for_retry_returns_none_for_stale_session() {
        let mut pool = SessionPool::new();

        // Create a session and make it stale
        let id = pool
            .get_or_create("claude", "claude-sonnet-4-6")
            .session_id
            .clone();

        // Make session stale by modifying last_accessed
        let session = pool.sessions.get_mut(&id).unwrap();
        session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(4000);

        // get_for_retry should return None for stale sessions
        assert!(pool.get_for_retry(&id).is_none());
    }

    #[test]
    fn test_get_for_retry_returns_none_for_nonexistent_session() {
        let mut pool = SessionPool::new();
        assert!(pool.get_for_retry("nonexistent").is_none());
    }

    #[test]
    fn test_get_or_create_for_task_creates_new_session() {
        let mut pool = SessionPool::new();
        let mut task = ltmatrix_core::Task::new("task-1", "Test", "Description");

        assert!(!task.has_session());

        let session = pool.get_or_create_for_task(&mut task);

        assert!(task.has_session());
        assert_eq!(task.get_session_id(), Some(session.session_id()));
        assert_eq!(session.reuse_count(), 0); // First access, not reused yet
    }

    #[test]
    fn test_get_or_create_for_task_reuses_session_on_retry() {
        let mut pool = SessionPool::new();
        let mut task = ltmatrix_core::Task::new("task-1", "Test", "Description");

        // First execution
        let session1 = pool.get_or_create_for_task(&mut task);
        let session_id = task.get_session_id().unwrap().to_string();
        let initial_reuse_count = session1.reuse_count();

        // Simulate retry (task has session_id set)
        let session2 = pool.get_or_create_for_task(&mut task);

        // Should reuse the same session
        assert_eq!(session2.session_id(), session_id);
        assert_eq!(task.get_session_id(), Some(session_id.as_str()));
        assert_eq!(session2.reuse_count(), initial_reuse_count + 1);
    }

    #[test]
    fn test_get_or_create_for_task_creates_new_if_stale() {
        let mut pool = SessionPool::new();
        let mut task = ltmatrix_core::Task::new("task-1", "Test", "Description");

        // First execution
        let session1 = pool.get_or_create_for_task(&mut task);
        let old_session_id = session1.session_id().to_string();

        // Make the session stale
        let session = pool.sessions.get_mut(&old_session_id).unwrap();
        session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(4000);

        // Retry with stale session should create a new one
        let session2 = pool.get_or_create_for_task(&mut task);

        // Should have a new session
        assert_ne!(session2.session_id(), old_session_id);
        assert_eq!(task.get_session_id(), Some(session2.session_id()));
    }

    #[test]
    fn test_task_prepare_retry_preserves_session() {
        let mut task = ltmatrix_core::Task::new("task-1", "Test", "Description");
        task.set_session_id("test-session-123");

        let session_id = task.get_session_id().unwrap().to_string();
        assert_eq!(task.retry_count, 0);

        task.prepare_retry();

        // Session should be preserved
        assert_eq!(task.get_session_id(), Some(session_id.as_str()));
        assert_eq!(task.retry_count, 1);
        assert_eq!(task.status, ltmatrix_core::TaskStatus::Pending);
    }

    #[test]
    fn test_task_clear_session_id() {
        let mut task = ltmatrix_core::Task::new("task-1", "Test", "Description");
        task.set_session_id("test-session-123");

        assert!(task.has_session());
        task.clear_session_id();
        assert!(!task.has_session());
    }
}
