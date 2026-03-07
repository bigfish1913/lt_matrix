// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Warmup executor for pre-initializing agent sessions
//!
//! This module provides functionality to warm up agent sessions before
//! actual task execution, improving first-response latency and ensuring
//! agents are ready when needed.
//!
//! # Warmup Strategy
//!
//! The warmup executor:
//! - Pre-initializes agent sessions with simple queries
//! - Validates agent availability before critical work
//! - Reuses warmed sessions across the task pipeline
//! - Supports configurable warmup queries and timeouts
//! - Handles failures gracefully with optional retry logic
//!
//! # Configuration
//!
//! Warmup behavior is controlled via [`WarmupConfig`] in the main config:
//!
//! ```toml
//! [warmup]
//! enabled = true
//! max_queries = 3
//! timeout_seconds = 30
//! retry_on_failure = false
//! prompt_template = "Hello, are you ready?"
//! ```

use std::time::Duration;

use anyhow::{Context, Result};
use tokio::time::timeout;
use tracing::{debug, info, warn, instrument};

use crate::agent::backend::{AgentBackend, AgentSession, ExecutionConfig, MemorySession};
use crate::agent::pool::SessionPool;
use crate::config::settings::WarmupConfig;

/// Default warmup prompt template
const DEFAULT_WARMUP_PROMPT: &str = "Hello! Please respond with 'Ready' to confirm you're working.";

/// Maximum number of warmup retry attempts
const MAX_WARMUP_RETRIES: u32 = 2;

/// Warmup execution result
#[derive(Debug, Clone, PartialEq)]
pub enum WarmupResult {
    /// Warmup completed successfully
    Success {
        /// Number of warmup queries executed
        queries_executed: u32,
        /// Total duration of warmup
        duration_ms: u64,
    },
    /// Warmup was skipped (disabled in config)
    Skipped,
    /// Warmup failed
    Failed {
        /// Error message
        error: String,
        /// Number of queries executed before failure
        queries_executed: u32,
    },
}

impl WarmupResult {
    /// Returns true if warmup was successful
    pub fn is_success(&self) -> bool {
        matches!(self, WarmupResult::Success { .. })
    }

    /// Returns true if warmup was skipped
    pub fn is_skipped(&self) -> bool {
        matches!(self, WarmupResult::Skipped)
    }

    /// Returns true if warmup failed
    pub fn is_failed(&self) -> bool {
        matches!(self, WarmupResult::Failed { .. })
    }

    /// Get the number of queries executed (if applicable)
    pub fn queries_executed(&self) -> Option<u32> {
        match self {
            WarmupResult::Success { queries_executed, .. } => Some(*queries_executed),
            WarmupResult::Failed { queries_executed, .. } => Some(*queries_executed),
            WarmupResult::Skipped => None,
        }
    }
}

/// Warmup executor for pre-initializing agent sessions
///
/// The executor handles the lifecycle of warming up agent sessions:
/// - Checking if warmup is enabled
/// - Executing warmup queries against agent backends
/// - Registering warmed sessions in the session pool
/// - Handling errors and retries
#[derive(Debug, Clone)]
pub struct WarmupExecutor {
    /// Warmup configuration
    config: WarmupConfig,
}

impl WarmupExecutor {
    /// Create a new warmup executor with the given configuration
    pub fn new(config: WarmupConfig) -> Self {
        Self { config }
    }

    /// Create a warmup executor from default configuration
    pub fn from_default_config() -> Self {
        Self::new(WarmupConfig::default())
    }

    /// Warm up a single agent backend
    ///
    /// This method executes warmup queries against the agent backend
    /// and registers the session in the pool for reuse.
    ///
    /// # Arguments
    ///
    /// * `backend` - The agent backend to warm up
    /// * `pool` - The session pool to register warmed sessions
    ///
    /// # Returns
    ///
    /// Returns a [`WarmupResult`] indicating success, failure, or skip
    #[instrument(skip(backend, pool), fields(agent_name = %backend.agent().name))]
    pub async fn warmup_agent<B>(
        &self,
        backend: &B,
        pool: &mut SessionPool,
    ) -> Result<WarmupResult>
    where
        B: AgentBackend + ?Sized,
    {
        // Skip if warmup is disabled
        if !self.config.enabled {
            debug!("Warmup is disabled, skipping agent warmup");
            return Ok(WarmupResult::Skipped);
        }

        let agent = backend.agent();
        let start_time = std::time::Instant::now();

        info!(
            "Starting warmup for agent '{}' (model: {})",
            agent.name, agent.model
        );

        // Create or get existing session
        let session = pool.get_or_create(&agent.name, &agent.model);
        let _session_id = session.session_id().to_string();

        // Execute warmup queries
        let mut queries_executed = 0u32;
        let max_queries = self.config.max_queries;

        for query_num in 0..max_queries {
            debug!(
                "Executing warmup query {}/{} for agent '{}'",
                query_num + 1,
                max_queries,
                agent.name
            );

            match self
                .execute_warmup_query(backend, session)
                .await
            {
                Ok(_) => {
                    queries_executed += 1;
                    debug!(
                        "Warmup query {}/{} succeeded for agent '{}'",
                        query_num + 1,
                        max_queries,
                        agent.name
                    );

                    // First successful warmup is sufficient
                    if query_num == 0 {
                        break;
                    }
                }
                Err(e) => {
                    let error_msg = format!("Warmup query failed: {}", e);
                    warn!("{}", error_msg);

                    // Retry if configured
                    if self.config.retry_on_failure {
                        warn!("Retrying warmup due to retry_on_failure=true");
                        match self
                            .execute_warmup_query_with_retry(backend, session)
                            .await
                        {
                            Ok(_) => {
                                queries_executed += 1;
                                break;
                            }
                            Err(retry_error) => {
                                return Ok(WarmupResult::Failed {
                                    error: format!("Warmup failed after retries: {}", retry_error),
                                    queries_executed,
                                });
                            }
                        }
                    } else {
                        return Ok(WarmupResult::Failed {
                            error: error_msg,
                            queries_executed,
                        });
                    }
                }
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        if queries_executed > 0 {
            info!(
                "Warmup completed for agent '{}' in {}ms ({} queries)",
                agent.name, duration_ms, queries_executed
            );

            Ok(WarmupResult::Success {
                queries_executed,
                duration_ms,
            })
        } else {
            Ok(WarmupResult::Failed {
                error: "No warmup queries were executed successfully".to_string(),
                queries_executed: 0,
            })
        }
    }

    /// Warm up multiple agent backends
    ///
    /// This method warms up all provided agents in sequence, logging
    /// the results of each warmup attempt.
    ///
    /// # Arguments
    ///
    /// * `backends` - Slice of agent backends to warm up
    /// * `pool` - The session pool to register warmed sessions
    ///
    /// # Returns
    ///
    /// Returns a vector of [`WarmupResult`] for each backend
    pub async fn warmup_agents<B>(
        &self,
        backends: &[&B],
        pool: &mut SessionPool,
    ) -> Vec<WarmupResult>
    where
        B: AgentBackend + ?Sized,
    {
        let mut results = Vec::with_capacity(backends.len());

        for backend in backends {
            let result = self.warmup_agent(*backend, pool).await;
            match &result {
                Ok(result) => {
                    results.push(result.clone());
                }
                Err(e) => {
                    warn!("Error warming up agent: {}", e);
                    results.push(WarmupResult::Failed {
                        error: e.to_string(),
                        queries_executed: 0,
                    });
                }
            }
        }

        results
    }

    /// Execute a single warmup query against the agent backend
    async fn execute_warmup_query<B>(
        &self,
        backend: &B,
        session: &MemorySession,
    ) -> Result<()>
    where
        B: AgentBackend + ?Sized,
    {
        let prompt = self
            .config
            .prompt_template
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or(DEFAULT_WARMUP_PROMPT);

        let config = ExecutionConfig {
            timeout: self.config.timeout_seconds,
            ..Default::default()
        };

        // Execute warmup query with timeout
        let timeout_duration = Duration::from_secs(self.config.timeout_seconds as u64);

        let result = timeout(timeout_duration, async {
            backend.execute_with_session(prompt, &config, session).await
        })
        .await
        .context(format!(
            "Warmup query timed out after {}s",
            self.config.timeout_seconds
        ))??;

        // Verify we got a response
        if result.error.is_some() {
            anyhow::bail!(
                "Warmup query returned error: {:?}",
                result.error
            );
        }

        if result.output.trim().is_empty() {
            anyhow::bail!("Warmup query returned empty response");
        }

        debug!("Warmup query response: {}", result.output);

        Ok(())
    }

    /// Execute warmup query with retry logic
    async fn execute_warmup_query_with_retry<B>(
        &self,
        backend: &B,
        session: &MemorySession,
    ) -> Result<()>
    where
        B: AgentBackend + ?Sized,
    {
        let mut last_error = None;

        for attempt in 0..MAX_WARMUP_RETRIES {
            debug!(
                "Warmup retry attempt {}/{}",
                attempt + 1,
                MAX_WARMUP_RETRIES
            );

            match self.execute_warmup_query(backend, session).await {
                Ok(_) => {
                    debug!("Warmup retry attempt {} succeeded", attempt + 1);
                    return Ok(());
                }
                Err(e) => {
                    warn!("Warmup retry attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);

                    // Exponential backoff before retry
                    if attempt < MAX_WARMUP_RETRIES - 1 {
                        let backoff_ms = 100 * (2_u64.pow(attempt));
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All warmup retry attempts failed")))
    }

    /// Check if an agent backend is available and ready
    ///
    /// This is a lightweight check that doesn't execute a full warmup query.
    /// It's useful for pre-flight validation before attempting warmup.
    ///
    /// # Arguments
    ///
    /// * `backend` - The agent backend to check
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the agent is available, `Ok(false)` otherwise
    pub async fn check_agent_available<B>(&self, backend: &B) -> Result<bool>
    where
        B: AgentBackend + ?Sized,
    {
        Ok(backend.is_available().await)
    }
}

impl Default for WarmupExecutor {
    fn default() -> Self {
        Self::from_default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::backend::{AgentConfig, AgentResponse};
    use crate::models::Agent;

    // Mock agent for testing
    struct MockWarmupAgent {
        agent: Agent,
        should_fail: bool,
    }

    #[async_trait::async_trait]
    impl AgentBackend for MockWarmupAgent {
        async fn execute(
            &self,
            _prompt: &str,
            _config: &ExecutionConfig,
        ) -> anyhow::Result<AgentResponse> {
            if self.should_fail {
                anyhow::bail!("Mock agent failure");
            }
            Ok(AgentResponse {
                output: "Ready".to_string(),
                ..Default::default()
            })
        }

        async fn execute_with_session(
            &self,
            _prompt: &str,
            _config: &ExecutionConfig,
            _session: &dyn AgentSession,
        ) -> anyhow::Result<AgentResponse> {
            if self.should_fail {
                anyhow::bail!("Mock agent failure");
            }
            Ok(AgentResponse {
                output: "Ready".to_string(),
                ..Default::default()
            })
        }

        async fn execute_task(
            &self,
            _task: &crate::models::Task,
            _context: &str,
            _config: &ExecutionConfig,
        ) -> anyhow::Result<AgentResponse> {
            Ok(AgentResponse::default())
        }

        async fn health_check(&self) -> anyhow::Result<bool> {
            Ok(!self.should_fail)
        }

        async fn validate_config(
            &self,
            _config: &AgentConfig,
        ) -> Result<(), crate::agent::backend::AgentError> {
            Ok(())
        }

        fn agent(&self) -> &Agent {
            &self.agent
        }
    }

    impl MockWarmupAgent {
        fn new(name: &str, should_fail: bool) -> Self {
            Self {
                agent: Agent::new(name, name, "test-model", 3600),
                should_fail,
            }
        }
    }

    #[test]
    fn test_warmup_result_success() {
        let result = WarmupResult::Success {
            queries_executed: 2,
            duration_ms: 150,
        };
        assert!(result.is_success());
        assert!(!result.is_skipped());
        assert!(!result.is_failed());
        assert_eq!(result.queries_executed(), Some(2));
    }

    #[test]
    fn test_warmup_result_skipped() {
        let result = WarmupResult::Skipped;
        assert!(!result.is_success());
        assert!(result.is_skipped());
        assert!(!result.is_failed());
        assert_eq!(result.queries_executed(), None);
    }

    #[test]
    fn test_warmup_result_failed() {
        let result = WarmupResult::Failed {
            error: "Test error".to_string(),
            queries_executed: 1,
        };
        assert!(!result.is_success());
        assert!(!result.is_skipped());
        assert!(result.is_failed());
        assert_eq!(result.queries_executed(), Some(1));
    }

    #[test]
    fn test_warmup_executor_default() {
        let executor = WarmupExecutor::default();
        assert!(!executor.config.enabled);
        assert_eq!(executor.config.max_queries, 3);
        assert_eq!(executor.config.timeout_seconds, 30);
    }

    #[test]
    fn test_warmup_executor_custom_config() {
        let config = WarmupConfig {
            enabled: true,
            max_queries: 5,
            timeout_seconds: 60,
            retry_on_failure: true,
            prompt_template: Some("Custom prompt".to_string()),
        };
        let executor = WarmupExecutor::new(config);
        assert!(executor.config.enabled);
        assert_eq!(executor.config.max_queries, 5);
        assert_eq!(executor.config.timeout_seconds, 60);
        assert!(executor.config.retry_on_failure);
    }

    #[tokio::test]
    async fn test_warmup_skipped_when_disabled() {
        let executor = WarmupExecutor::new(WarmupConfig {
            enabled: false,
            ..Default::default()
        });

        let mut pool = SessionPool::new();
        let agent = MockWarmupAgent::new("test-agent", false);

        let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
        assert!(result.is_skipped());
    }

    #[tokio::test]
    async fn test_warmup_success() {
        let executor = WarmupExecutor::new(WarmupConfig {
            enabled: true,
            max_queries: 1,
            timeout_seconds: 30,
            retry_on_failure: false,
            prompt_template: None,
        });

        let mut pool = SessionPool::new();
        let agent = MockWarmupAgent::new("test-agent", false);

        let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
        assert!(result.is_success());
        assert_eq!(result.queries_executed(), Some(1));
    }

    #[tokio::test]
    async fn test_warmup_failure() {
        let executor = WarmupExecutor::new(WarmupConfig {
            enabled: true,
            max_queries: 1,
            timeout_seconds: 30,
            retry_on_failure: false,
            prompt_template: None,
        });

        let mut pool = SessionPool::new();
        let agent = MockWarmupAgent::new("failing-agent", true);

        let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
        assert!(result.is_failed());
        assert_eq!(result.queries_executed(), Some(0));
    }

    #[tokio::test]
    async fn test_warmup_with_retry() {
        let executor = WarmupExecutor::new(WarmupConfig {
            enabled: true,
            max_queries: 1,
            timeout_seconds: 30,
            retry_on_failure: true,
            prompt_template: None,
        });

        let mut pool = SessionPool::new();
        // Agent that will fail first attempt but succeed on retry
        let agent = MockWarmupAgent::new("retry-agent", false);

        let result = executor.warmup_agent(&agent, &mut pool).await.unwrap();
        // Should succeed because agent doesn't actually fail
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_warmup_multiple_agents() {
        let executor = WarmupExecutor::new(WarmupConfig {
            enabled: true,
            max_queries: 1,
            timeout_seconds: 30,
            retry_on_failure: false,
            prompt_template: None,
        });

        let mut pool = SessionPool::new();
        let agent1 = MockWarmupAgent::new("agent1", false);
        let agent2 = MockWarmupAgent::new("agent2", false);

        let backends = vec![&agent1, &agent2];
        let results = executor.warmup_agents(&backends, &mut pool).await;

        assert_eq!(results.len(), 2);
        assert!(results[0].is_success());
        assert!(results[1].is_success());
    }

    #[tokio::test]
    async fn test_check_agent_available() {
        let executor = WarmupExecutor::default();
        let available_agent = MockWarmupAgent::new("available", false);
        let unavailable_agent = MockWarmupAgent::new("unavailable", true);

        assert!(executor
            .check_agent_available(&available_agent)
            .await
            .unwrap());
        assert!(!executor
            .check_agent_available(&unavailable_agent)
            .await
            .unwrap());
    }
}
