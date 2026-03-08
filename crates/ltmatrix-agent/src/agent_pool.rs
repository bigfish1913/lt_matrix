// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Unified AgentPool integrating session management and warmup
//!
//! This module provides the main AgentPool type that combines:
//! - SessionPool for in-memory session management
//! - WarmupExecutor for pre-initializing sessions
//! - Configuration-driven behavior
//! - Thread-safe concurrent access

use std::sync::Arc;
use tokio::sync::Mutex;

use anyhow::Context;

use crate::backend::{AgentBackend, AgentSession, ExecutionConfig};
use crate::pool::SessionPool;
use crate::warmup::{WarmupExecutor, WarmupResult};
use ltmatrix_config::settings::{Config, PoolConfig, WarmupConfig};
use ltmatrix_core::{AgentType, Mode};

/// Unified agent pool that manages sessions and warmup
///
/// This is the main entry point for agent execution in the pipeline,
/// providing thread-safe access to sessions with optional warmup.
#[derive(Debug, Clone)]
pub struct AgentPool {
    /// Inner pool state protected by mutex for thread safety
    inner: Arc<Mutex<AgentPoolInner>>,
}

/// Inner state of the agent pool
#[derive(Debug)]
struct AgentPoolInner {
    /// Session pool for managing agent sessions
    sessions: SessionPool,

    /// Pool configuration
    config: PoolConfig,

    /// Warmup configuration
    warmup_config: WarmupConfig,

    /// Warmup executor (created when needed)
    warmup_executor: Option<WarmupExecutor>,

    /// Execution mode for mode-aware agent selection
    mode: Option<Mode>,

    /// Enabled agent types for this pool (based on mode)
    enabled_agent_types: Vec<AgentType>,
}

impl AgentPool {
    /// Create a new agent pool with the given configuration
    pub fn new(config: &Config) -> Self {
        let inner = AgentPoolInner {
            sessions: SessionPool::new(),
            config: config.pool.clone(),
            warmup_config: config.warmup.clone(),
            warmup_executor: None,
            mode: None,
            enabled_agent_types: AgentType::all().to_vec(),
        };

        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Create a new agent pool for a specific execution mode
    ///
    /// This creates a mode-aware pool that only initializes sessions
    /// for agent types enabled in the given mode.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the pool
    /// * `mode` - Execution mode (Fast, Standard, Expert)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix_agent::AgentPool;
    /// use ltmatrix_config::settings::Config;
    /// use ltmatrix_core::Mode;
    ///
    /// let config = Config::default();
    /// let pool = AgentPool::new_for_mode(&config, Mode::Fast);
    /// ```
    pub fn new_for_mode(config: &Config, mode: Mode) -> Self {
        let enabled_agent_types = mode.enabled_agents();
        let inner = AgentPoolInner {
            sessions: SessionPool::new(),
            config: config.pool.clone(),
            warmup_config: config.warmup.clone(),
            warmup_executor: None,
            mode: Some(mode),
            enabled_agent_types: enabled_agent_types.clone(),
        };

        tracing::info!(
            "Created agent pool for {:?} mode with enabled agent types: {:?}",
            mode,
            enabled_agent_types
        );

        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Create a new agent pool from default configuration
    pub fn from_default_config() -> Self {
        Self::new(&Config::default())
    }

    /// Get or create a session for a task
    ///
    /// This is the main method for task execution. It:
    /// 1. Checks if the task has a session_id for reuse (retry scenarios)
    /// 2. Checks if the task has a parent_session_id (dependency chains)
    /// 3. Otherwise creates a new session for the agent/model pair (reusing existing sessions)
    ///
    /// Note: Sessions are reused for the same (agent_name, model) pair across different tasks.
    /// This is intentional to reduce resource overhead and enable session sharing.
    ///
    /// # Arguments
    ///
    /// * `task` - Task to get session for (modified to store session_id)
    /// * `agent_name` - Name of the agent backend
    /// * `model` - Model identifier
    ///
    /// # Returns
    ///
    /// Returns the session ID as a string
    pub async fn get_or_create_session_for_task(
        &self,
        task: &mut ltmatrix_core::Task,
        agent_name: &str,
        model: &str,
    ) -> String {
        // Extract session_id to avoid borrow issues
        let existing_session_id = task.get_session_id().map(|s| s.to_string());

        let mut inner = self.inner.lock().await;

        // First, check if task already has a session (retry scenario)
        if let Some(session_id) = existing_session_id {
            if let Some(session) = inner.sessions.get(&session_id) {
                if !session.is_stale() {
                    // get_or_create will mark as accessed
                    tracing::debug!(
                        "Reusing session {} for task {} (retry)",
                        session_id,
                        task.id
                    );
                    return session_id;
                } else {
                    // Session is stale, clear it
                    tracing::debug!(
                        "Session {} for task {} is stale, creating new one",
                        session_id,
                        task.id
                    );
                    task.clear_session_id();
                }
            }
        }

        // Extract parent_session_id to avoid borrow issues
        let parent_session_id = task.get_parent_session_id().map(|s| s.to_string());

        // Next, check if task has a parent session (dependency chain)
        if let Some(parent_session_id) = parent_session_id {
            if let Some(session) = inner.sessions.get(&parent_session_id) {
                if !session.is_stale() {
                    // Use parent's session for dependency chain
                    task.set_session_id(&parent_session_id);
                    // Mark as accessed by calling get_or_create
                    inner.sessions.get_or_create(agent_name, model);
                    tracing::debug!(
                        "Task {} using parent session {} (dependency chain)",
                        task.id,
                        parent_session_id
                    );
                    return parent_session_id;
                }
            }
        }

        // Create new session (or reuse existing for same agent/model)
        let session = inner.sessions.get_or_create(agent_name, model);
        let session_id = session.session_id().to_string();
        task.set_session_id(&session_id);

        tracing::debug!(
            "Created new session {} for task {} (agent: {}, model: {})",
            session_id,
            task.id,
            agent_name,
            model
        );

        session_id
    }

    /// Warm up agent backends before task execution
    ///
    /// This method pre-initializes sessions for the given agents,
    /// improving first-response latency.
    ///
    /// # Arguments
    ///
    /// * `backends` - Slice of agent backends to warm up
    ///
    /// # Returns
    ///
    /// Vector of warmup results for each backend
    pub async fn warmup_agents<B>(&self, backends: &[&B]) -> Vec<WarmupResult>
    where
        B: AgentBackend + ?Sized,
    {
        // Get warmup config
        let warmup_config = {
            let inner = self.inner.lock().await;
            inner.warmup_config.clone()
        };

        // Create warmup executor
        let executor = WarmupExecutor::new(warmup_config);

        // Warm up all agents, getting the session pool for each
        let mut results = Vec::with_capacity(backends.len());
        for backend in backends {
            let mut inner = self.inner.lock().await;
            let result = executor.warmup_agent(*backend, &mut inner.sessions).await;
            match &result {
                Ok(result) => {
                    results.push(result.clone());
                }
                Err(e) => {
                    tracing::warn!("Error warming up agent: {}", e);
                    results.push(WarmupResult::Failed {
                        error: e.to_string(),
                        queries_executed: 0,
                    });
                }
            }
        }

        results
    }

    /// Execute a task with session management
    ///
    /// This is a convenience method that combines getting a session
    /// and executing the task with proper error handling.
    ///
    /// # Arguments
    ///
    /// * `task` - Task to execute (modified to store session_id)
    /// * `backend` - Agent backend to use
    /// * `prompt` - Prompt to execute
    /// * `config` - Execution configuration
    ///
    /// # Returns
    ///
    /// Result containing the agent response
    pub async fn execute_with_session<B>(
        &self,
        task: &mut ltmatrix_core::Task,
        backend: &B,
        prompt: &str,
        config: &ExecutionConfig,
    ) -> anyhow::Result<crate::backend::AgentResponse>
    where
        B: AgentBackend + ?Sized,
    {
        let agent = backend.agent();
        let agent_name = agent.name.clone();
        let agent_model = agent.model.clone();

        // Get or create session (will store session_id in task)
        let _session_id = self
            .get_or_create_session_for_task(task, &agent_name, &agent_model)
            .await;

        // Get the session from the pool
        let session_id = task.get_session_id().unwrap();
        let inner = self.inner.lock().await;
        let session = inner.sessions.get(session_id).ok_or_else(|| {
            anyhow::anyhow!("Session {} not found in pool", session_id)
        })?;

        // Execute with session
        let response = backend.execute_with_session(prompt, config, session).await?;

        Ok(response)
    }

    /// Clean up stale sessions
    ///
    /// Removes sessions that haven't been accessed within the
    /// configured stale threshold.
    ///
    /// # Returns
    ///
    /// Number of sessions removed
    pub async fn cleanup_stale_sessions(&self) -> usize {
        let mut inner = self.inner.lock().await;
        let removed = inner.sessions.cleanup_stale();

        if removed > 0 {
            tracing::info!("Cleaned up {} stale sessions", removed);
        }

        removed
    }

    /// Synchronous version of get_or_create_session_for_task for use in synchronous contexts.
    ///
    /// This uses try_lock() and returns a session ID if available.
    /// Returns an error if the lock is contended or session creation fails.
    pub fn get_session_for_task_sync(
        &self,
        task: &mut ltmatrix_core::Task,
        agent_name: &str,
        model: &str,
    ) -> anyhow::Result<String> {
        // Try to get the lock without blocking
        let mut inner = self.inner.try_lock()
            .context("AgentPool lock is contended, cannot get session synchronously")?;

        // Check for existing session (retry scenario)
        if let Some(session_id) = task.get_session_id() {
            if let Some(session) = inner.sessions.get(session_id) {
                if !session.is_stale() {
                    tracing::debug!("Reusing session {} for task {} (sync)", session_id, task.id);
                    return Ok(session_id.to_string());
                }
            }
            task.clear_session_id();
        }

        // Create new session (or reuse existing for same agent/model)
        let session = inner.sessions.get_or_create(agent_name, model);
        let session_id = session.session_id().to_string();
        task.set_session_id(&session_id);

        Ok(session_id)
    }

    /// Synchronous version of cleanup_stale_sessions for use in synchronous contexts.
    ///
    /// Returns the number of sessions that were removed.
    pub fn cleanup_stale_sessions_sync(&self) -> usize {
        if let Ok(mut inner) = self.inner.try_lock() {
            inner.sessions.cleanup_stale()
        } else {
            0 // Lock contended, skip cleanup
        }
    }

    /// Get pool statistics
    ///
    /// Returns information about the current state of the pool.
    pub async fn stats(&self) -> PoolStats {
        let inner = self.inner.lock().await;

        let total_sessions = inner.sessions.len();

        // Note: We can't count stale sessions without iterating over sessions,
        // which isn't exposed by the SessionPool API. We'll report 0 for now.
        PoolStats {
            total_sessions,
            active_sessions: total_sessions, // Assume all are active
            stale_sessions: 0, // Cannot determine without iteration
            max_sessions: inner.config.max_sessions,
            warmup_enabled: inner.warmup_config.enabled,
            total_created: total_sessions, // Simplified tracking
        }
    }

    /// Synchronous version of stats for use in synchronous contexts.
    ///
    /// This uses try_lock() and returns default stats if the lock is contended.
    pub fn stats_sync(&self) -> PoolStats {
        if let Ok(inner) = self.inner.try_lock() {
            let total_sessions = inner.sessions.len();
            PoolStats {
                total_sessions,
                active_sessions: total_sessions,
                stale_sessions: 0,
                max_sessions: inner.config.max_sessions,
                warmup_enabled: inner.warmup_config.enabled,
                total_created: total_sessions,
            }
        } else {
            // Lock is contended, return default stats
            PoolStats {
                total_sessions: 0,
                active_sessions: 0,
                stale_sessions: 0,
                max_sessions: 100,
                warmup_enabled: false,
                total_created: 0,
            }
        }
    }

    /// Get the underlying session pool (for advanced use cases)
    ///
    /// This provides direct access to the SessionPool for scenarios
    /// that need lower-level control.
    pub async fn with_session_pool<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&SessionPool) -> R,
    {
        let inner = self.inner.lock().await;
        f(&inner.sessions)
    }

    /// Start background cleanup task
    ///
    /// Spawns a tokio task that periodically cleans up stale sessions
    /// based on the configured cleanup interval.
    pub async fn spawn_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let pool = self.clone();
        let interval = {
            let inner = pool.inner.lock().await;
            inner.config.cleanup_interval_duration()
        };

        tokio::spawn(async move {
            let mut timer = tokio::time::interval(interval);
            loop {
                timer.tick().await;
                pool.cleanup_stale_sessions().await;
            }
        })
    }

    /// Check if an agent type is enabled in this pool
    ///
    /// Returns true if the agent type is enabled based on the mode
    /// configured for this pool.
    pub fn is_agent_type_enabled(&self, agent_type: AgentType) -> bool {
        // For synchronous access, we need to use try_lock
        if let Ok(inner) = self.inner.try_lock() {
            inner.enabled_agent_types.contains(&agent_type)
        } else {
            // If lock is contended, default to true (safe fallback)
            tracing::warn!("Could not check agent type enabled status, defaulting to true");
            true
        }
    }

    /// Check if an agent type is enabled in this pool (async version)
    pub async fn is_agent_type_enabled_async(&self, agent_type: AgentType) -> bool {
        let inner = self.inner.lock().await;
        inner.enabled_agent_types.contains(&agent_type)
    }

    /// Get the current execution mode
    pub async fn get_mode(&self) -> Option<Mode> {
        let inner = self.inner.lock().await;
        inner.mode
    }

    /// Get enabled agent types for this pool
    pub async fn get_enabled_agent_types(&self) -> Vec<AgentType> {
        let inner = self.inner.lock().await;
        inner.enabled_agent_types.clone()
    }

    /// Get the appropriate model for an agent type based on mode
    ///
    /// Returns the model name to use for the given agent type,
    /// considering the mode configured for this pool.
    pub fn get_model_for_agent_type(&self, agent_type: AgentType) -> &'static str {
        if let Ok(inner) = self.inner.try_lock() {
            if let Some(mode) = inner.mode {
                return match agent_type {
                    AgentType::Plan => mode.plan_model(),
                    AgentType::Dev => mode.exec_model(),
                    AgentType::Test => mode.exec_model(),
                    AgentType::Review => mode.review_model(),
                };
            }
        }
        // Default to exec_model if mode not set
        "claude-sonnet-4-6"
    }
}

/// Statistics about the agent pool
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolStats {
    /// Total number of sessions in the pool
    pub total_sessions: usize,

    /// Number of active (non-stale) sessions
    pub active_sessions: usize,

    /// Number of stale sessions
    pub stale_sessions: usize,

    /// Maximum sessions allowed in the pool
    pub max_sessions: usize,

    /// Whether warmup is enabled
    pub warmup_enabled: bool,

    /// Total number of sessions created (lifetime)
    pub total_created: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{AgentConfig, AgentResponse};
    use ltmatrix_core::Task;

    // Mock agent for testing
    struct MockAgent {
        agent: ltmatrix_core::Agent,
    }

    #[async_trait::async_trait]
    impl AgentBackend for MockAgent {
        async fn execute(
            &self,
            _prompt: &str,
            _config: &ExecutionConfig,
        ) -> anyhow::Result<AgentResponse> {
            Ok(AgentResponse {
                output: "Mock response".to_string(),
                ..Default::default()
            })
        }

        async fn execute_with_session(
            &self,
            prompt: &str,
            config: &ExecutionConfig,
            _session: &dyn crate::backend::AgentSession,
        ) -> anyhow::Result<AgentResponse> {
            self.execute(prompt, config).await
        }

        async fn execute_task(
            &self,
            _task: &Task,
            _context: &str,
            _config: &ExecutionConfig,
        ) -> anyhow::Result<AgentResponse> {
            Ok(AgentResponse::default())
        }

        async fn health_check(&self) -> anyhow::Result<bool> {
            Ok(true)
        }

        async fn validate_config(
            &self,
            _config: &AgentConfig,
        ) -> Result<(), crate::backend::AgentError> {
            Ok(())
        }

        fn agent(&self) -> &ltmatrix_core::Agent {
            &self.agent
        }
    }

    #[test]
    fn test_agent_pool_creation() {
        let pool = AgentPool::from_default_config();
        // Pool is created successfully
    }

    #[tokio::test]
    async fn test_pool_stats() {
        let pool = AgentPool::from_default_config();

        let stats = pool.stats().await;

        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.stale_sessions, 0);
    }

    #[tokio::test]
    async fn test_get_or_create_session_for_task() {
        let pool = AgentPool::from_default_config();
        let mut task = Task::new("task-1", "Test", "Description");

        let session_id = pool
            .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;

        assert!(!session_id.is_empty());
        assert_eq!(task.get_session_id(), Some(session_id.as_str()));
    }

    #[tokio::test]
    async fn test_session_reuse_on_retry() {
        let pool = AgentPool::from_default_config();
        let mut task = Task::new("task-1", "Test", "Description");

        // First execution
        let session_id1 = pool
            .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;

        // Simulate retry (task already has session_id)
        let session_id2 = pool
            .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;

        // Should reuse the same session
        assert_eq!(session_id1, session_id2);
    }

    #[tokio::test]
    async fn test_cleanup_stale_sessions() {
        let pool = AgentPool::from_default_config();
        let mut task = Task::new("task-1", "Test", "Description");

        // Create a session
        pool.get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;

        let stats_before = pool.stats().await;
        assert_eq!(stats_before.total_sessions, 1);

        // Cleanup won't remove fresh sessions
        let removed = pool.cleanup_stale_sessions().await;
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_new_for_mode_fast() {
        let config = Config::default();
        let pool = AgentPool::new_for_mode(&config, Mode::Fast);

        // Fast mode should only have Plan and Dev agents enabled
        assert!(pool.is_agent_type_enabled(AgentType::Plan));
        assert!(pool.is_agent_type_enabled(AgentType::Dev));
        assert!(!pool.is_agent_type_enabled(AgentType::Test));
        assert!(!pool.is_agent_type_enabled(AgentType::Review));
    }

    #[test]
    fn test_new_for_mode_standard() {
        let config = Config::default();
        let pool = AgentPool::new_for_mode(&config, Mode::Standard);

        // Standard mode should have Plan, Dev, and Test agents enabled
        assert!(pool.is_agent_type_enabled(AgentType::Plan));
        assert!(pool.is_agent_type_enabled(AgentType::Dev));
        assert!(pool.is_agent_type_enabled(AgentType::Test));
        assert!(!pool.is_agent_type_enabled(AgentType::Review));
    }

    #[test]
    fn test_new_for_mode_expert() {
        let config = Config::default();
        let pool = AgentPool::new_for_mode(&config, Mode::Expert);

        // Expert mode should have all agent types enabled
        assert!(pool.is_agent_type_enabled(AgentType::Plan));
        assert!(pool.is_agent_type_enabled(AgentType::Dev));
        assert!(pool.is_agent_type_enabled(AgentType::Test));
        assert!(pool.is_agent_type_enabled(AgentType::Review));
    }

    #[test]
    fn test_get_model_for_agent_type() {
        let config = Config::default();
        let pool = AgentPool::new_for_mode(&config, Mode::Expert);

        // Check models for each agent type
        assert_eq!(pool.get_model_for_agent_type(AgentType::Plan), "claude-opus-4-6");
        assert_eq!(pool.get_model_for_agent_type(AgentType::Dev), "claude-sonnet-4-6");
        assert_eq!(pool.get_model_for_agent_type(AgentType::Test), "claude-sonnet-4-6");
        assert_eq!(pool.get_model_for_agent_type(AgentType::Review), "claude-opus-4-6");
    }

    #[tokio::test]
    async fn test_get_mode() {
        let config = Config::default();
        let pool = AgentPool::new_for_mode(&config, Mode::Fast);

        let mode = pool.get_mode().await;
        assert_eq!(mode, Some(Mode::Fast));
    }

    #[tokio::test]
    async fn test_get_enabled_agent_types() {
        let config = Config::default();
        let pool = AgentPool::new_for_mode(&config, Mode::Fast);

        let enabled = pool.get_enabled_agent_types().await;
        assert_eq!(enabled.len(), 2);
        assert!(enabled.contains(&AgentType::Plan));
        assert!(enabled.contains(&AgentType::Dev));
    }
}
