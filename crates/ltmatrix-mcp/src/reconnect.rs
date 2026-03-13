// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Reconnection and Error Recovery Logic for MCP Connections
//!
//! This module provides automatic reconnection capabilities with exponential backoff,
//! graceful degradation strategies, and integration with connection health monitoring.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    ReconnectionManager                           │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐       │
//! │  │  Backoff      │  │  Reconnect    │  │  Degradation  │       │
//! │  │  Strategy     │  │  Policy       │  │  Handler      │       │
//! │  │               │  │               │  │               │       │
//! │  │  exponential  │  │  max attempts │  │  fallbacks    │       │
//! │  │  + jitter     │  │  conditions   │  │  retries      │       │
//! │  └───────────────┘  └───────────────┘  └───────────────┘       │
//! │         │                   │                   │               │
//! │         └───────────────────┼───────────────────┘               │
//! │                             ▼                                   │
//! │              ┌──────────────────────┐                           │
//! │              │   Health Monitor     │                           │
//! │              │   Integration        │                           │
//! │              └──────────────────────┘                           │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::mcp::reconnect::{ReconnectionManager, ReconnectConfig, BackoffStrategy};
//! use std::time::Duration;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ReconnectConfig {
//!         max_attempts: 5,
//!         backoff: BackoffStrategy::exponential(
//!             Duration::from_secs(1),
//!             Duration::from_secs(60),
//!             2.0,
//!         ),
//!         ..Default::default()
//!     };
//!
//!     let manager = ReconnectionManager::new(config);
//!
//!     // Check if should attempt reconnection
//!     if manager.should_reconnect() {
//!         let delay = manager.next_backoff();
//!         tokio::time::sleep(delay).await;
//!         // Attempt reconnection...
//!         manager.record_attempt(true, std::time::Duration::from_millis(100)).await; // success
//!     }
//!
//!     Ok(())
//! }
//! ```

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

use crate::heartbeat::{ConnectionHealth, HeartbeatManager};
use crate::protocol::errors::{McpError, McpErrorCode, McpResult};

// ============================================================================
// Backoff Strategy
// ============================================================================

/// Backoff strategy for reconnection attempts
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// Fixed delay between attempts
    Fixed {
        /// Delay duration
        delay: Duration,
    },

    /// Linear backoff (delay increases linearly)
    Linear {
        /// Initial delay
        initial_delay: Duration,
        /// Increment per attempt
        increment: Duration,
        /// Maximum delay
        max_delay: Duration,
    },

    /// Exponential backoff (delay doubles each attempt)
    Exponential {
        /// Initial delay
        initial_delay: Duration,
        /// Maximum delay cap
        max_delay: Duration,
        /// Multiplier (typically 2.0)
        multiplier: f64,
    },

    /// Exponential backoff with jitter
    ExponentialWithJitter {
        /// Initial delay
        initial_delay: Duration,
        /// Maximum delay cap
        max_delay: Duration,
        /// Multiplier
        multiplier: f64,
        /// Jitter factor (0.0 to 1.0)
        jitter: f64,
    },
}

impl BackoffStrategy {
    /// Create a fixed backoff strategy
    pub fn fixed(delay: Duration) -> Self {
        Self::Fixed { delay }
    }

    /// Create a linear backoff strategy
    pub fn linear(initial_delay: Duration, increment: Duration, max_delay: Duration) -> Self {
        Self::Linear {
            initial_delay,
            increment,
            max_delay,
        }
    }

    /// Create an exponential backoff strategy
    pub fn exponential(initial_delay: Duration, max_delay: Duration, multiplier: f64) -> Self {
        Self::Exponential {
            initial_delay,
            max_delay,
            multiplier,
        }
    }

    /// Create an exponential backoff strategy with jitter
    pub fn exponential_with_jitter(
        initial_delay: Duration,
        max_delay: Duration,
        multiplier: f64,
        jitter: f64,
    ) -> Self {
        Self::ExponentialWithJitter {
            initial_delay,
            max_delay,
            multiplier,
            jitter: jitter.clamp(0.0, 1.0),
        }
    }

    /// Calculate the delay for a given attempt number (0-indexed)
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        match self {
            Self::Fixed { delay } => *delay,

            Self::Linear {
                initial_delay,
                increment,
                max_delay,
            } => {
                let delay = *initial_delay + *increment * attempt;
                delay.min(*max_delay)
            }

            Self::Exponential {
                initial_delay,
                max_delay,
                multiplier,
            } => {
                let delay_secs = initial_delay.as_secs_f64() * multiplier.powi(attempt as i32);
                let delay = Duration::from_secs_f64(delay_secs);
                delay.min(*max_delay)
            }

            Self::ExponentialWithJitter {
                initial_delay,
                max_delay,
                multiplier,
                jitter,
            } => {
                let base_delay_secs = initial_delay.as_secs_f64() * multiplier.powi(attempt as i32);

                // Apply jitter: random value in [1-jitter, 1+jitter]
                // For simplicity, we use a deterministic jitter based on attempt
                let jitter_factor = 1.0 + (*jitter * ((attempt as f64 % 10.0) / 5.0 - 1.0));
                let delay_secs = base_delay_secs * jitter_factor;

                let delay = Duration::from_secs_f64(delay_secs.max(0.0));
                delay.min(*max_delay)
            }
        }
    }
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        Self::exponential_with_jitter(Duration::from_secs(1), Duration::from_secs(60), 2.0, 0.3)
    }
}

// ============================================================================
// Reconnection Configuration
// ============================================================================

/// Configuration for reconnection behavior
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Maximum number of reconnection attempts (0 = unlimited)
    pub max_attempts: u32,

    /// Backoff strategy for delays between attempts
    pub backoff: BackoffStrategy,

    /// Enable automatic reconnection on connection loss
    pub auto_reconnect: bool,

    /// Delay before starting reconnection after disconnect
    pub initial_delay: Duration,

    /// Reset attempt counter after successful connection for this duration
    pub reset_after: Duration,

    /// Trigger reconnection when connection health is stale
    pub reconnect_on_stale: bool,

    /// Trigger reconnection after this many consecutive errors
    pub reconnect_on_errors: u32,

    /// Enable debug logging
    pub debug_logging: bool,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_attempts: 10,
            backoff: BackoffStrategy::default(),
            auto_reconnect: true,
            initial_delay: Duration::from_millis(100),
            reset_after: Duration::from_secs(60),
            reconnect_on_stale: true,
            reconnect_on_errors: 3,
            debug_logging: false,
        }
    }
}

impl ReconnectConfig {
    /// Create a new reconnect config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum attempts
    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }

    /// Set backoff strategy
    pub fn with_backoff(mut self, backoff: BackoffStrategy) -> Self {
        self.backoff = backoff;
        self
    }

    /// Enable/disable auto reconnect
    pub fn with_auto_reconnect(mut self, enabled: bool) -> Self {
        self.auto_reconnect = enabled;
        self
    }

    /// Enable debug logging
    pub fn with_debug_logging(mut self, enabled: bool) -> Self {
        self.debug_logging = enabled;
        self
    }

    /// Set reconnect on stale
    pub fn with_reconnect_on_stale(mut self, enabled: bool) -> Self {
        self.reconnect_on_stale = enabled;
        self
    }
}

// ============================================================================
// Degradation Level
// ============================================================================

/// Degradation level for graceful degradation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DegradationLevel {
    /// Full functionality available
    None,

    /// Minor degradation - some features may be slower
    Minor,

    /// Moderate degradation - non-essential features disabled
    Moderate,

    /// Severe degradation - only essential features available
    Severe,

    /// Critical - connection unusable, reconnection required
    Critical,
}

impl Default for DegradationLevel {
    fn default() -> Self {
        Self::None
    }
}

impl DegradationLevel {
    /// Check if degraded
    pub fn is_degraded(&self) -> bool {
        *self != DegradationLevel::None
    }

    /// Check if critical
    pub fn is_critical(&self) -> bool {
        *self == DegradationLevel::Critical
    }

    /// Get human-readable name
    pub fn as_str(&self) -> &'static str {
        match self {
            DegradationLevel::None => "none",
            DegradationLevel::Minor => "minor",
            DegradationLevel::Moderate => "moderate",
            DegradationLevel::Severe => "severe",
            DegradationLevel::Critical => "critical",
        }
    }

    /// Determine degradation level from connection health
    pub fn from_health(health: &ConnectionHealth) -> Self {
        match health {
            ConnectionHealth::Healthy => DegradationLevel::None,
            ConnectionHealth::Degraded { missed_pings } => {
                if *missed_pings <= 1 {
                    DegradationLevel::Minor
                } else {
                    DegradationLevel::Moderate
                }
            }
            ConnectionHealth::Stale { .. } => DegradationLevel::Critical,
        }
    }
}

// ============================================================================
// Reconnection Statistics
// ============================================================================

/// Statistics for reconnection attempts
#[derive(Debug, Clone, Default)]
pub struct ReconnectStats {
    /// Total reconnection attempts
    pub total_attempts: u64,

    /// Successful reconnections
    pub successful_reconnects: u64,

    /// Failed reconnection attempts
    pub failed_attempts: u64,

    /// Current consecutive failures
    pub consecutive_failures: u32,

    /// Total time spent in reconnection
    pub total_reconnect_time: Duration,

    /// Average reconnection time
    pub avg_reconnect_time: Option<Duration>,

    /// Last reconnection attempt time
    pub last_attempt: Option<Instant>,

    /// Last successful reconnection time
    pub last_success: Option<Instant>,

    /// Current degradation level
    pub degradation_level: DegradationLevel,

    /// Time of last successful connection (for reset_after calculation)
    pub connected_since: Option<Instant>,
}

impl ReconnectStats {
    /// Create new stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a reconnection attempt
    pub fn record_attempt(&mut self, success: bool, duration: Duration) {
        self.total_attempts += 1;
        self.last_attempt = Some(Instant::now());

        if success {
            self.successful_reconnects += 1;
            self.consecutive_failures = 0;
            self.last_success = Some(Instant::now());
            self.connected_since = Some(Instant::now());
            self.degradation_level = DegradationLevel::None;

            // Update average
            self.avg_reconnect_time = Some(match self.avg_reconnect_time {
                Some(avg) => (avg + duration) / 2,
                None => duration,
            });
        } else {
            self.failed_attempts += 1;
            self.consecutive_failures += 1;
            self.connected_since = None;

            // Increase degradation level
            self.degradation_level = match self.consecutive_failures {
                0..=1 => DegradationLevel::Minor,
                2..=3 => DegradationLevel::Moderate,
                4..=5 => DegradationLevel::Severe,
                _ => DegradationLevel::Critical,
            };
        }

        self.total_reconnect_time += duration;
    }

    /// Reset stats after successful connection
    pub fn reset_on_success(&mut self) {
        self.consecutive_failures = 0;
        self.degradation_level = DegradationLevel::None;
        self.connected_since = Some(Instant::now());
    }

    /// Check if attempt counter should be reset
    pub fn should_reset(&self, reset_after: Duration) -> bool {
        if let Some(connected) = self.connected_since {
            connected.elapsed() >= reset_after
        } else {
            false
        }
    }
}

// ============================================================================
// Reconnection Manager
// ============================================================================

/// Manager for reconnection logic
///
/// This struct coordinates:
/// - Backoff timing for reconnection attempts
/// - Attempt counting and limits
/// - Degradation level tracking
/// - Integration with heartbeat health monitoring
pub struct ReconnectionManager {
    /// Configuration
    config: ReconnectConfig,

    /// Statistics
    stats: Arc<Mutex<ReconnectStats>>,

    /// Current attempt counter
    current_attempt: AtomicU32,

    /// Reconnection in progress flag
    reconnecting: AtomicBool,

    /// Background task handle
    task_handle: Mutex<Option<JoinHandle<()>>>,

    /// Stop signal sender
    stop_tx: Mutex<Option<mpsc::Sender<()>>>,
}

/// Handle to control reconnection behavior
#[derive(Debug)]
pub struct ReconnectHandle {
    /// Stop signal sender
    stop_tx: mpsc::Sender<()>,

    /// Task handle
    task_handle: Mutex<Option<JoinHandle<()>>>,
}

impl ReconnectHandle {
    /// Stop the reconnection task
    pub async fn stop(&self) {
        let _ = self.stop_tx.send(()).await;
    }

    /// Wait for the reconnection task to complete
    pub async fn join(&self) {
        if let Some(handle) = self.task_handle.lock().await.take() {
            let _ = handle.await;
        }
    }
}

/// Trait for performing reconnection
///
/// This must be implemented by the client layer to enable
/// the reconnection manager to trigger reconnections.
#[async_trait::async_trait]
pub trait Reconnector: Send + Sync {
    /// Attempt to reconnect
    ///
    /// Returns Ok(()) on successful reconnection, Err on failure.
    async fn reconnect(&self) -> McpResult<()>;

    /// Check if currently connected
    async fn is_connected(&self) -> bool;

    /// Get current connection health
    async fn health(&self) -> ConnectionHealth;
}

impl ReconnectionManager {
    /// Create a new reconnection manager
    pub fn new(config: ReconnectConfig) -> Self {
        Self {
            config,
            stats: Arc::new(Mutex::new(ReconnectStats::new())),
            current_attempt: AtomicU32::new(0),
            reconnecting: AtomicBool::new(false),
            task_handle: Mutex::new(None),
            stop_tx: Mutex::new(None),
        }
    }

    /// Create with default configuration
    pub fn default_manager() -> Self {
        Self::new(ReconnectConfig::default())
    }

    /// Get the configuration
    pub fn config(&self) -> &ReconnectConfig {
        &self.config
    }

    /// Get current statistics
    pub async fn stats(&self) -> ReconnectStats {
        self.stats.lock().await.clone()
    }

    /// Get current attempt number
    pub fn current_attempt(&self) -> u32 {
        self.current_attempt.load(Ordering::SeqCst)
    }

    /// Check if reconnection is in progress
    pub fn is_reconnecting(&self) -> bool {
        self.reconnecting.load(Ordering::SeqCst)
    }

    /// Check if should attempt reconnection
    ///
    /// Returns true if:
    /// - Auto reconnect is enabled
    /// - Not already reconnecting
    /// - Haven't exceeded max attempts (if limited)
    pub fn should_reconnect(&self) -> bool {
        if !self.config.auto_reconnect {
            return false;
        }

        if self.reconnecting.load(Ordering::SeqCst) {
            return false;
        }

        if self.config.max_attempts > 0 {
            let attempt = self.current_attempt.load(Ordering::SeqCst);
            if attempt >= self.config.max_attempts {
                return false;
            }
        }

        true
    }

    /// Get the next backoff delay
    ///
    /// This increments the attempt counter.
    pub fn next_backoff(&self) -> Duration {
        let attempt = self.current_attempt.fetch_add(1, Ordering::SeqCst);
        self.config.backoff.calculate_delay(attempt)
    }

    /// Calculate backoff without incrementing counter
    pub fn peek_backoff(&self) -> Duration {
        let attempt = self.current_attempt.load(Ordering::SeqCst);
        self.config.backoff.calculate_delay(attempt)
    }

    /// Record a reconnection attempt result
    ///
    /// This should be called after each reconnection attempt.
    pub async fn record_attempt(&self, success: bool, duration: Duration) {
        let mut stats = self.stats.lock().await;
        stats.record_attempt(success, duration);

        if success {
            self.current_attempt.store(0, Ordering::SeqCst);
            self.reconnecting.store(false, Ordering::SeqCst);

            tracing::info!(
                attempts = stats.total_attempts,
                duration_ms = duration.as_millis(),
                "Reconnection successful"
            );
        } else {
            self.reconnecting.store(false, Ordering::SeqCst);

            tracing::warn!(
                attempt = self.current_attempt.load(Ordering::SeqCst),
                consecutive_failures = stats.consecutive_failures,
                degradation = stats.degradation_level.as_str(),
                "Reconnection failed"
            );
        }
    }

    /// Reset attempt counter (e.g., after successful connection)
    pub fn reset_attempts(&self) {
        self.current_attempt.store(0, Ordering::SeqCst);
    }

    /// Start automatic reconnection monitoring
    ///
    /// This starts a background task that monitors connection health
    /// and triggers reconnection when needed.
    pub async fn start_monitoring<R: Reconnector + 'static>(
        &self,
        reconnector: Arc<R>,
        heartbeat: Arc<HeartbeatManager>,
    ) -> McpResult<ReconnectHandle> {
        if self.reconnecting.swap(true, Ordering::SeqCst) {
            return Err(McpError::with_category(
                McpErrorCode::SessionError,
                "Reconnection monitoring already running",
                crate::protocol::errors::ErrorCategory::Protocol,
            ));
        }

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        *self.stop_tx.lock().await = Some(stop_tx.clone());

        let config = self.config.clone();
        let stats = self.stats.clone();
        let current_attempt = Arc::new(AtomicU32::new(0));
        let reconnecting = Arc::new(AtomicBool::new(true));

        // Copy atomic references
        let self_attempt = Arc::new(AtomicU32::new(0));
        let self_reconnecting = Arc::new(AtomicBool::new(true));

        let handle = tokio::spawn(async move {
            let check_interval = Duration::from_secs(5);
            let mut interval = tokio::time::interval(check_interval);

            loop {
                tokio::select! {
                    _ = stop_rx.recv() => {
                        tracing::debug!("Reconnection monitoring stopped");
                        break;
                    }

                    _ = interval.tick() => {
                        // Check connection health
                        let health = heartbeat.health().await;

                        // Determine degradation level
                        let degradation = DegradationLevel::from_health(&health);

                        // Check if should reset attempt counter and update stats
                        {
                            let mut s = stats.lock().await;
                            if s.should_reset(config.reset_after) {
                                s.reset_on_success();
                                current_attempt.store(0, Ordering::SeqCst);
                            }
                            s.degradation_level = degradation;
                        }

                        // Check if reconnection needed
                        let needs_reconnect = (config.reconnect_on_stale && health.is_stale())
                            || (stats.lock().await.consecutive_failures >= config.reconnect_on_errors);

                        if needs_reconnect && !reconnector.is_connected().await {
                            if config.debug_logging {
                                tracing::debug!(
                                    health = ?health,
                                    degradation = degradation.as_str(),
                                    "Triggering reconnection"
                                );
                            }

                            // Trigger reconnection
                            let attempt = current_attempt.fetch_add(1, Ordering::SeqCst);

                            // Check max attempts
                            if config.max_attempts > 0 && attempt >= config.max_attempts {
                                tracing::error!(
                                    attempts = attempt,
                                    max = config.max_attempts,
                                    "Max reconnection attempts reached"
                                );
                                continue;
                            }

                            let delay = config.backoff.calculate_delay(attempt);

                            if config.debug_logging {
                                tracing::debug!(
                                    attempt = attempt,
                                    delay_ms = delay.as_millis(),
                                    "Waiting before reconnection attempt"
                                );
                            }

                            tokio::time::sleep(delay).await;

                            let start = Instant::now();
                            let result = reconnector.reconnect().await;
                            let duration = start.elapsed();

                            let mut s = stats.lock().await;
                            match result {
                                Ok(()) => {
                                    s.record_attempt(true, duration);
                                    current_attempt.store(0, Ordering::SeqCst);

                                    tracing::info!(
                                        attempt = attempt,
                                        duration_ms = duration.as_millis(),
                                        "Automatic reconnection successful"
                                    );
                                }
                                Err(e) => {
                                    s.record_attempt(false, duration);

                                    tracing::warn!(
                                        attempt = attempt,
                                        error = %e,
                                        duration_ms = duration.as_millis(),
                                        "Automatic reconnection failed"
                                    );
                                }
                            }
                        }
                    }
                }
            }

            reconnecting.store(false, Ordering::SeqCst);
            self_reconnecting.store(false, Ordering::SeqCst);
        });

        // Note: handle is moved into ReconnectHandle, we don't store a separate copy
        // The manager's task_handle remains None since we can't clone JoinHandle

        tracing::info!(
            max_attempts = config.max_attempts,
            auto_reconnect = config.auto_reconnect,
            "Reconnection monitoring started"
        );

        Ok(ReconnectHandle {
            stop_tx,
            task_handle: Mutex::new(Some(handle)),
        })
    }

    /// Stop reconnection monitoring
    pub async fn stop(&self) {
        if let Some(tx) = self.stop_tx.lock().await.take() {
            let _ = tx.send(()).await;
        }

        self.reconnecting.store(false, Ordering::SeqCst);
    }

    /// Get current degradation level
    pub async fn degradation_level(&self) -> DegradationLevel {
        self.stats.lock().await.degradation_level
    }

    /// Check if connection is in a degraded state
    pub async fn is_degraded(&self) -> bool {
        self.degradation_level().await.is_degraded()
    }

    /// Check if connection is in critical state
    pub async fn is_critical(&self) -> bool {
        self.degradation_level().await.is_critical()
    }
}

impl Default for ReconnectionManager {
    fn default() -> Self {
        Self::default_manager()
    }
}

// ============================================================================
// Error Recovery
// ============================================================================

/// Error recovery strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Retry the operation immediately
    Retry,

    /// Retry after a delay
    RetryWithDelay,

    /// Reconnect and retry
    ReconnectAndRetry,

    /// Fail immediately without retry
    Fail,

    /// Use fallback/default value
    Fallback,
}

/// Error recovery configuration
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    /// Maximum retry attempts
    pub max_retries: u32,

    /// Delay between retries
    pub retry_delay: Duration,

    /// Whether to attempt reconnection on connection errors
    pub reconnect_on_error: bool,

    /// Errors that should trigger immediate failure
    pub fail_fast_codes: Vec<McpErrorCode>,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
            reconnect_on_error: true,
            fail_fast_codes: vec![
                McpErrorCode::InvalidRequest,
                McpErrorCode::MethodNotFound,
                McpErrorCode::InvalidParams,
            ],
        }
    }
}

impl RecoveryConfig {
    /// Create new recovery config
    pub fn new() -> Self {
        Self::default()
    }

    /// Determine recovery strategy for an error
    pub fn strategy_for(&self, error: &McpError) -> RecoveryStrategy {
        // Check for fail-fast codes
        if self.fail_fast_codes.contains(&error.code) {
            return RecoveryStrategy::Fail;
        }

        // Check error category
        match error.code {
            McpErrorCode::TransportError
            | McpErrorCode::ServerShutdown
            | McpErrorCode::RequestTimeout => {
                if self.reconnect_on_error {
                    RecoveryStrategy::ReconnectAndRetry
                } else {
                    RecoveryStrategy::RetryWithDelay
                }
            }
            McpErrorCode::InternalError | McpErrorCode::ServerStarting => {
                RecoveryStrategy::RetryWithDelay
            }
            _ => RecoveryStrategy::Retry,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_fixed() {
        let backoff = BackoffStrategy::fixed(Duration::from_secs(5));

        assert_eq!(backoff.calculate_delay(0), Duration::from_secs(5));
        assert_eq!(backoff.calculate_delay(1), Duration::from_secs(5));
        assert_eq!(backoff.calculate_delay(10), Duration::from_secs(5));
    }

    #[test]
    fn test_backoff_linear() {
        let backoff = BackoffStrategy::linear(
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(10),
        );

        assert_eq!(backoff.calculate_delay(0), Duration::from_secs(1));
        assert_eq!(backoff.calculate_delay(1), Duration::from_secs(3));
        assert_eq!(backoff.calculate_delay(2), Duration::from_secs(5));
        // Capped at max
        assert_eq!(backoff.calculate_delay(10), Duration::from_secs(10));
    }

    #[test]
    fn test_backoff_exponential() {
        let backoff =
            BackoffStrategy::exponential(Duration::from_secs(1), Duration::from_secs(60), 2.0);

        assert_eq!(backoff.calculate_delay(0), Duration::from_secs(1));
        assert_eq!(backoff.calculate_delay(1), Duration::from_secs(2));
        assert_eq!(backoff.calculate_delay(2), Duration::from_secs(4));
        assert_eq!(backoff.calculate_delay(3), Duration::from_secs(8));
        // Capped at max
        assert_eq!(backoff.calculate_delay(10), Duration::from_secs(60));
    }

    #[test]
    fn test_backoff_exponential_with_jitter() {
        let backoff = BackoffStrategy::exponential_with_jitter(
            Duration::from_secs(1),
            Duration::from_secs(60),
            2.0,
            0.3,
        );

        // Should be approximately 1 second with jitter
        let delay = backoff.calculate_delay(0);
        assert!(delay >= Duration::from_millis(700));
        assert!(delay <= Duration::from_millis(1300));

        // Should be approximately 2 seconds with jitter
        let delay = backoff.calculate_delay(1);
        assert!(delay >= Duration::from_millis(1400));
        assert!(delay <= Duration::from_millis(2600));
    }

    #[test]
    fn test_reconnect_config_default() {
        let config = ReconnectConfig::default();

        assert_eq!(config.max_attempts, 10);
        assert!(config.auto_reconnect);
        assert!(config.reconnect_on_stale);
        assert_eq!(config.reconnect_on_errors, 3);
    }

    #[test]
    fn test_reconnect_config_builder() {
        let config = ReconnectConfig::new()
            .with_max_attempts(5)
            .with_auto_reconnect(false)
            .with_reconnect_on_stale(false)
            .with_debug_logging(true);

        assert_eq!(config.max_attempts, 5);
        assert!(!config.auto_reconnect);
        assert!(!config.reconnect_on_stale);
        assert!(config.debug_logging);
    }

    #[test]
    fn test_degradation_level() {
        let healthy = ConnectionHealth::Healthy;
        assert_eq!(
            DegradationLevel::from_health(&healthy),
            DegradationLevel::None
        );

        let degraded1 = ConnectionHealth::Degraded { missed_pings: 1 };
        assert_eq!(
            DegradationLevel::from_health(&degraded1),
            DegradationLevel::Minor
        );

        let degraded2 = ConnectionHealth::Degraded { missed_pings: 2 };
        assert_eq!(
            DegradationLevel::from_health(&degraded2),
            DegradationLevel::Moderate
        );

        let stale = ConnectionHealth::Stale { missed_pings: 5 };
        assert_eq!(
            DegradationLevel::from_health(&stale),
            DegradationLevel::Critical
        );
    }

    #[test]
    fn test_degradation_level_methods() {
        assert!(!DegradationLevel::None.is_degraded());
        assert!(DegradationLevel::Minor.is_degraded());
        assert!(DegradationLevel::Critical.is_critical());
        assert!(!DegradationLevel::Moderate.is_critical());
    }

    #[test]
    fn test_reconnect_stats() {
        let mut stats = ReconnectStats::new();

        // Record successful attempt
        stats.record_attempt(true, Duration::from_millis(100));
        assert_eq!(stats.total_attempts, 1);
        assert_eq!(stats.successful_reconnects, 1);
        assert_eq!(stats.consecutive_failures, 0);
        assert!(stats.avg_reconnect_time.is_some());

        // Record failed attempt
        stats.record_attempt(false, Duration::from_millis(50));
        assert_eq!(stats.total_attempts, 2);
        assert_eq!(stats.failed_attempts, 1);
        assert_eq!(stats.consecutive_failures, 1);
        assert_eq!(stats.degradation_level, DegradationLevel::Minor);

        // More failures
        stats.record_attempt(false, Duration::from_millis(50));
        stats.record_attempt(false, Duration::from_millis(50));
        assert_eq!(stats.consecutive_failures, 3);
        assert_eq!(stats.degradation_level, DegradationLevel::Moderate);
    }

    #[test]
    fn test_reconnect_stats_reset() {
        let mut stats = ReconnectStats::new();

        stats.record_attempt(false, Duration::from_millis(50));
        stats.record_attempt(false, Duration::from_millis(50));
        assert_eq!(stats.consecutive_failures, 2);

        stats.reset_on_success();
        assert_eq!(stats.consecutive_failures, 0);
        assert_eq!(stats.degradation_level, DegradationLevel::None);
    }

    #[tokio::test]
    async fn test_reconnection_manager_creation() {
        let manager = ReconnectionManager::default_manager();

        assert_eq!(manager.current_attempt(), 0);
        assert!(!manager.is_reconnecting());
        assert!(manager.should_reconnect());
    }

    #[tokio::test]
    async fn test_reconnection_manager_backoff() {
        let manager = ReconnectionManager::new(
            ReconnectConfig::new().with_backoff(BackoffStrategy::fixed(Duration::from_secs(2))),
        );

        let delay1 = manager.next_backoff();
        let delay2 = manager.next_backoff();

        assert_eq!(delay1, Duration::from_secs(2));
        assert_eq!(delay2, Duration::from_secs(2));
        assert_eq!(manager.current_attempt(), 2);
    }

    #[tokio::test]
    async fn test_reconnection_manager_max_attempts() {
        let manager = ReconnectionManager::new(ReconnectConfig::new().with_max_attempts(3));

        assert!(manager.should_reconnect());
        manager.next_backoff();
        assert!(manager.should_reconnect());
        manager.next_backoff();
        assert!(manager.should_reconnect());
        manager.next_backoff();

        // After 3 attempts, should not reconnect
        assert!(!manager.should_reconnect());
    }

    #[tokio::test]
    async fn test_reconnection_manager_record_attempt() {
        let manager = ReconnectionManager::new(ReconnectConfig::default());

        // Record failed attempt
        manager
            .record_attempt(false, Duration::from_millis(100))
            .await;
        let stats = manager.stats().await;
        assert_eq!(stats.failed_attempts, 1);
        assert_eq!(stats.consecutive_failures, 1);

        // Record successful attempt
        manager
            .record_attempt(true, Duration::from_millis(150))
            .await;
        let stats = manager.stats().await;
        assert_eq!(stats.successful_reconnects, 1);
        assert_eq!(stats.consecutive_failures, 0);
        assert_eq!(manager.current_attempt(), 0);
    }

    #[tokio::test]
    async fn test_reconnection_manager_reset() {
        let manager = ReconnectionManager::new(ReconnectConfig::default());

        manager.next_backoff();
        manager.next_backoff();
        manager.next_backoff();
        assert_eq!(manager.current_attempt(), 3);

        manager.reset_attempts();
        assert_eq!(manager.current_attempt(), 0);
    }

    #[tokio::test]
    async fn test_reconnection_manager_degradation() {
        let manager = ReconnectionManager::new(ReconnectConfig::default());

        // Initially not degraded
        assert!(!manager.is_degraded().await);

        // Record failures
        manager
            .record_attempt(false, Duration::from_millis(50))
            .await;
        manager
            .record_attempt(false, Duration::from_millis(50))
            .await;

        // Should be degraded
        assert!(manager.is_degraded().await);
    }

    #[test]
    fn test_recovery_strategy() {
        let config = RecoveryConfig::new();

        // Transport errors should trigger reconnect
        let error = McpError::communication("Connection lost");
        assert_eq!(
            config.strategy_for(&error),
            RecoveryStrategy::ReconnectAndRetry
        );

        // Invalid request should fail fast
        let error = McpError::new(McpErrorCode::InvalidRequest, "Bad request");
        assert_eq!(config.strategy_for(&error), RecoveryStrategy::Fail);

        // Timeout should trigger reconnect
        let error = McpError::timeout("test", Duration::from_secs(10));
        assert_eq!(
            config.strategy_for(&error),
            RecoveryStrategy::ReconnectAndRetry
        );
    }

    #[tokio::test]
    async fn test_reconnection_manager_auto_disabled() {
        let manager = ReconnectionManager::new(ReconnectConfig::new().with_auto_reconnect(false));

        assert!(!manager.should_reconnect());
    }

    #[tokio::test]
    async fn test_reconnection_manager_stats_should_reset() {
        let mut stats = ReconnectStats::new();

        // No connection time, should not reset
        assert!(!stats.should_reset(Duration::from_secs(60)));

        // Set connected_since to now
        stats.connected_since = Some(Instant::now());

        // Should not reset yet
        assert!(!stats.should_reset(Duration::from_secs(60)));

        // Set connected_since to past
        stats.connected_since = Some(Instant::now() - Duration::from_secs(120));

        // Should reset now
        assert!(stats.should_reset(Duration::from_secs(60)));
    }
}
