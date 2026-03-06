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

use std::collections::HashMap;

use crate::agent::backend::{AgentSession, MemorySession};

/// In-memory pool of agent sessions indexed by session ID.
///
/// The pool is **not** thread-safe by itself; callers that share it across
/// async tasks must wrap it in an `Arc<Mutex<SessionPool>>`.
#[derive(Debug, Default)]
pub struct SessionPool {
    /// All registered sessions, keyed by session_id.
    sessions: HashMap<String, MemorySession>,
}

impl SessionPool {
    /// Create an empty pool.
    pub fn new() -> Self {
        SessionPool {
            sessions: HashMap::new(),
        }
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
}
