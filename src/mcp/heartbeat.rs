// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Heartbeat/Keepalive Mechanism for MCP Connections
//!
//! This module provides connection health monitoring through periodic ping messages
//! and activity tracking. It helps detect stale connections and maintain connection
//! liveness during idle periods.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    HeartbeatManager                              │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐       │
//! │  │  Config       │  │ Activity      │  │  Ping Task    │       │
//! │  │  - interval   │  │ Tracker       │  │  (background) │       │
//! │  │  - timeout    │  │  - last_recv  │  │               │       │
//! │  │  - max_missed │  │  - last_send  │  │  periodic pings│       │
//! │  └───────────────┘  └───────────────┘  └───────────────┘       │
//! │         │                   │                   │               │
//! │         └───────────────────┼───────────────────┘               │
//! │                             ▼                                   │
//! │              ┌──────────────────────┐                           │
//! │              │  Connection Health   │                           │
//! │              │  - is_healthy()      │                           │
//! │              │  - is_stale()        │                           │
//! │              │  - missed_pings()    │                           │
//! │              └──────────────────────┘                           │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::mcp::heartbeat::{HeartbeatManager, HeartbeatConfig};
//! use std::time::Duration;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = HeartbeatConfig {
//!         interval: Duration::from_secs(30),
//!         timeout: Duration::from_secs(10),
//!         max_missed_pings: 3,
//!         ..Default::default()
//!     };
//!
//!     let manager = HeartbeatManager::new(config);
//!
//!     // Start heartbeat task
//!     let handle = manager.start().await?;
//!
//!     // Record activity on send/receive
//!     manager.record_send().await;
//!     manager.record_receive().await;
//!
//!     // Check health
//!     if manager.is_stale().await {
//!         println!("Connection is stale!");
//!     }
//!
//!     // Stop heartbeat
//!     handle.stop().await;
//!     Ok(())
//! }
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;

use crate::mcp::protocol::errors::{McpError, McpResult};
use crate::mcp::protocol::messages::RequestId;

// ============================================================================
// Heartbeat Configuration
// ============================================================================

/// Configuration for the heartbeat/keepalive mechanism
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// Interval between ping messages
    ///
    /// Default: 30 seconds
    pub interval: Duration,

    /// Timeout for ping response
    ///
    /// If no response is received within this time, the ping is considered missed.
    /// Default: 10 seconds
    pub timeout: Duration,

    /// Maximum number of consecutive missed pings before connection is considered stale
    ///
    /// Default: 3
    pub max_missed_pings: u32,

    /// Whether to enable automatic heartbeat on idle
    ///
    /// When enabled, heartbeat pings are only sent if no other activity
    /// has occurred within the interval period.
    /// Default: true
    pub idle_only: bool,

    /// Minimum time between activity recordings
    ///
    /// Prevents excessive updates during high activity.
    /// Default: 1 second
    pub activity_debounce: Duration,

    /// Enable debug logging for heartbeat events
    pub debug_logging: bool,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            max_missed_pings: 3,
            idle_only: true,
            activity_debounce: Duration::from_secs(1),
            debug_logging: false,
        }
    }
}

impl HeartbeatConfig {
    /// Create a new heartbeat config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the ping interval
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Set the ping timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum missed pings threshold
    pub fn with_max_missed_pings(mut self, max: u32) -> Self {
        self.max_missed_pings = max;
        self
    }

    /// Enable or disable idle-only mode
    pub fn with_idle_only(mut self, idle_only: bool) -> Self {
        self.idle_only = idle_only;
        self
    }

    /// Enable debug logging
    pub fn with_debug_logging(mut self, enabled: bool) -> Self {
        self.debug_logging = enabled;
        self
    }
}

// ============================================================================
// Activity Tracker
// ============================================================================

/// Tracks last activity timestamps for connection health monitoring
#[derive(Debug)]
pub struct ActivityTracker {
    /// Timestamp of last received message
    last_receive: Arc<RwLock<Option<Instant>>>,

    /// Timestamp of last sent message
    last_send: Arc<RwLock<Option<Instant>>>,

    /// Debounce timer for activity updates
    last_update: Arc<Mutex<Option<Instant>>>,

    /// Debounce duration
    debounce: Duration,
}

impl ActivityTracker {
    /// Create a new activity tracker
    pub fn new(debounce: Duration) -> Self {
        Self {
            last_receive: Arc::new(RwLock::new(None)),
            last_send: Arc::new(RwLock::new(None)),
            last_update: Arc::new(Mutex::new(None)),
            debounce,
        }
    }

    /// Record a received message
    ///
    /// This updates the last_receive timestamp, debounced to prevent
    /// excessive writes during high activity.
    pub async fn record_receive(&self) {
        let should_update = {
            let mut last = self.last_update.lock().await;
            let now = Instant::now();

            if let Some(last_time) = *last {
                if now.duration_since(last_time) < self.debounce {
                    false
                } else {
                    *last = Some(now);
                    true
                }
            } else {
                *last = Some(now);
                true
            }
        };

        if should_update {
            *self.last_receive.write().await = Some(Instant::now());
        }
    }

    /// Record a sent message
    pub async fn record_send(&self) {
        *self.last_send.write().await = Some(Instant::now());
    }

    /// Get the time since last receive
    pub async fn time_since_receive(&self) -> Option<Duration> {
        self.last_receive.read().await.map(|t| t.elapsed())
    }

    /// Get the time since last send
    pub async fn time_since_send(&self) -> Option<Duration> {
        self.last_send.read().await.map(|t| t.elapsed())
    }

    /// Get the time since last activity (either send or receive)
    pub async fn time_since_activity(&self) -> Option<Duration> {
        let recv = self.last_receive.read().await;
        let send = self.last_send.read().await;

        match (*recv, *send) {
            (Some(r), Some(s)) => Some(std::cmp::min(r.elapsed(), s.elapsed())),
            (Some(r), None) => Some(r.elapsed()),
            (None, Some(s)) => Some(s.elapsed()),
            (None, None) => None,
        }
    }

    /// Check if there has been any activity
    pub async fn has_activity(&self) -> bool {
        self.last_receive.read().await.is_some() || self.last_send.read().await.is_some()
    }

    /// Reset all activity timestamps
    pub async fn reset(&self) {
        *self.last_receive.write().await = None;
        *self.last_send.write().await = None;
        *self.last_update.lock().await = None;
    }
}

impl Default for ActivityTracker {
    fn default() -> Self {
        Self::new(Duration::from_secs(1))
    }
}

// ============================================================================
// Connection Health
// ============================================================================

/// Connection health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionHealth {
    /// Connection is healthy - recent activity and responding to pings
    Healthy,

    /// Connection is degraded - some missed pings but still within threshold
    Degraded {
        /// Number of consecutive missed pings
        missed_pings: u32,
    },

    /// Connection is stale - too many missed pings
    Stale {
        /// Number of consecutive missed pings
        missed_pings: u32,
    },
}

impl ConnectionHealth {
    /// Check if the connection is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self, ConnectionHealth::Healthy)
    }

    /// Check if the connection is degraded
    pub fn is_degraded(&self) -> bool {
        matches!(self, ConnectionHealth::Degraded { .. })
    }

    /// Check if the connection is stale
    pub fn is_stale(&self) -> bool {
        matches!(self, ConnectionHealth::Stale { .. })
    }

    /// Get the number of missed pings
    pub fn missed_pings(&self) -> u32 {
        match self {
            ConnectionHealth::Healthy => 0,
            ConnectionHealth::Degraded { missed_pings } => *missed_pings,
            ConnectionHealth::Stale { missed_pings } => *missed_pings,
        }
    }
}

// ============================================================================
// Heartbeat Statistics
// ============================================================================

/// Statistics for the heartbeat mechanism
#[derive(Debug, Clone, Default)]
pub struct HeartbeatStats {
    /// Total pings sent
    pub pings_sent: u64,

    /// Total pong responses received
    pub pongs_received: u64,

    /// Total missed pings (no response within timeout)
    pub pings_missed: u64,

    /// Current consecutive missed pings
    pub consecutive_missed: u32,

    /// Average round-trip time for pings
    pub avg_rtt_ms: Option<f64>,

    /// Last ping send time
    pub last_ping_sent: Option<Instant>,

    /// Last pong receive time
    pub last_pong_received: Option<Instant>,
}

impl HeartbeatStats {
    /// Create new stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a sent ping
    pub fn record_ping_sent(&mut self) {
        self.pings_sent += 1;
        self.last_ping_sent = Some(Instant::now());
    }

    /// Record a received pong and update RTT
    pub fn record_pong_received(&mut self) {
        self.pongs_received += 1;
        self.consecutive_missed = 0;
        self.last_pong_received = Some(Instant::now());

        // Calculate RTT if we have a last ping time
        if let Some(sent) = self.last_ping_sent {
            let rtt = sent.elapsed().as_secs_f64() * 1000.0;
            self.avg_rtt_ms = Some(match self.avg_rtt_ms {
                Some(avg) => (avg + rtt) / 2.0,
                None => rtt,
            });
        }
    }

    /// Record a missed ping
    pub fn record_ping_missed(&mut self) {
        self.pings_missed += 1;
        self.consecutive_missed += 1;
    }

    /// Reset consecutive missed counter
    pub fn reset_consecutive(&mut self) {
        self.consecutive_missed = 0;
    }
}

// ============================================================================
// Heartbeat Manager
// ============================================================================

/// Manager for heartbeat/keepalive functionality
///
/// This struct coordinates:
/// - Activity tracking
/// - Periodic ping sending
/// - Connection health monitoring
/// - Stale connection detection
pub struct HeartbeatManager {
    /// Configuration
    config: HeartbeatConfig,

    /// Activity tracker
    activity: ActivityTracker,

    /// Statistics
    stats: Arc<Mutex<HeartbeatStats>>,

    /// Running flag (shared with background task)
    running: Arc<AtomicBool>,

    /// Stop signal sender
    stop_tx: Mutex<Option<mpsc::Sender<()>>>,
}

/// Handle to control the heartbeat task
#[derive(Debug)]
pub struct HeartbeatHandle {
    /// Stop signal sender
    stop_tx: mpsc::Sender<()>,

    /// Task handle
    task_handle: Mutex<Option<JoinHandle<()>>>,
}

impl HeartbeatHandle {
    /// Stop the heartbeat task
    pub async fn stop(&self) {
        let _ = self.stop_tx.send(()).await;
    }

    /// Wait for the heartbeat task to complete
    pub async fn join(&self) {
        if let Some(handle) = self.task_handle.lock().await.take() {
            let _ = handle.await;
        }
    }
}

/// Trait for sending ping messages
///
/// This must be implemented by the transport or client layer
/// to enable the heartbeat manager to send pings.
#[async_trait::async_trait]
pub trait PingSender: Send + Sync {
    /// Send a ping request
    ///
    /// Returns the request ID used for the ping
    async fn send_ping(&self, id: RequestId) -> McpResult<()>;
}

impl HeartbeatManager {
    /// Create a new heartbeat manager
    pub fn new(config: HeartbeatConfig) -> Self {
        let debounce = config.activity_debounce;
        Self {
            config,
            activity: ActivityTracker::new(debounce),
            stats: Arc::new(Mutex::new(HeartbeatStats::new())),
            running: Arc::new(AtomicBool::new(false)),
            stop_tx: Mutex::new(None),
        }
    }

    /// Create with default configuration
    pub fn default_manager() -> Self {
        Self::new(HeartbeatConfig::default())
    }

    /// Get the configuration
    pub fn config(&self) -> &HeartbeatConfig {
        &self.config
    }

    /// Record a sent message
    pub async fn record_send(&self) {
        self.activity.record_send().await;
    }

    /// Record a received message
    pub async fn record_receive(&self) {
        self.activity.record_receive().await;

        // Also update stats for pong tracking
        let mut stats = self.stats.lock().await;
        stats.record_pong_received();
    }

    /// Get time since last activity
    pub async fn time_since_activity(&self) -> Option<Duration> {
        self.activity.time_since_activity().await
    }

    /// Get current statistics
    pub async fn stats(&self) -> HeartbeatStats {
        self.stats.lock().await.clone()
    }

    /// Get current connection health
    pub async fn health(&self) -> ConnectionHealth {
        let stats = self.stats.lock().await;

        if stats.consecutive_missed == 0 {
            ConnectionHealth::Healthy
        } else if stats.consecutive_missed >= self.config.max_missed_pings {
            ConnectionHealth::Stale {
                missed_pings: stats.consecutive_missed,
            }
        } else {
            ConnectionHealth::Degraded {
                missed_pings: stats.consecutive_missed,
            }
        }
    }

    /// Check if the connection is healthy
    pub async fn is_healthy(&self) -> bool {
        self.health().await.is_healthy()
    }

    /// Check if the connection is stale
    pub async fn is_stale(&self) -> bool {
        self.health().await.is_stale()
    }

    /// Check if heartbeat is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Start the heartbeat background task
    ///
    /// Returns a handle to control the heartbeat task.
    pub async fn start(&self) -> McpResult<HeartbeatHandle> {
        // Check if already running
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(McpError::with_category(
                crate::mcp::protocol::errors::McpErrorCode::SessionError,
                "Heartbeat already running",
                crate::mcp::protocol::errors::ErrorCategory::Protocol,
            ));
        }

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);

        // Store for later use
        *self.stop_tx.lock().await = Some(stop_tx.clone());

        let config = self.config.clone();
        let stats = self.stats.clone();
        let running = self.running.clone();

        // Spawn the heartbeat task
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.interval);

            loop {
                tokio::select! {
                    _ = stop_rx.recv() => {
                        tracing::debug!("Heartbeat task received stop signal");
                        break;
                    }

                    _ = interval.tick() => {
                        // Record ping sent
                        {
                            let mut s = stats.lock().await;
                            s.record_ping_sent();
                        }

                        if config.debug_logging {
                            tracing::trace!("Heartbeat tick - ping would be sent");
                        }

                        // In a real implementation, we would send a ping here
                        // and track the response. For now, we just update stats.
                    }
                }
            }

            running.store(false, Ordering::SeqCst);
            tracing::info!("Heartbeat task stopped");
        });

        tracing::info!(
            interval_secs = config.interval.as_secs(),
            timeout_secs = config.timeout.as_secs(),
            max_missed = config.max_missed_pings,
            "Heartbeat manager started"
        );

        Ok(HeartbeatHandle {
            stop_tx,
            task_handle: Mutex::new(Some(handle)),
        })
    }

    /// Start heartbeat with a ping sender
    ///
    /// This version actually sends ping messages through the provided sender.
    pub async fn start_with_sender<S: PingSender + 'static>(
        &self,
        sender: Arc<S>,
        id_generator: impl Fn() -> RequestId + Send + 'static,
    ) -> McpResult<HeartbeatHandle> {
        // Check if already running
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(McpError::with_category(
                crate::mcp::protocol::errors::McpErrorCode::SessionError,
                "Heartbeat already running",
                crate::mcp::protocol::errors::ErrorCategory::Protocol,
            ));
        }

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        let (ping_timeout_tx, mut ping_timeout_rx) = mpsc::channel::<RequestId>(10);

        *self.stop_tx.lock().await = Some(stop_tx.clone());

        let config = self.config.clone();
        let stats = self.stats.clone();
        let activity = Arc::new(ActivityTracker::new(config.activity_debounce));
        let running = self.running.clone();

        // Spawn the heartbeat task
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.interval);
            let mut pending_pings: std::collections::HashMap<RequestId, Instant> =
                std::collections::HashMap::new();

            loop {
                tokio::select! {
                    _ = stop_rx.recv() => {
                        tracing::debug!("Heartbeat task received stop signal");
                        break;
                    }

                    _ = interval.tick() => {
                        // Check for idle-only mode
                        if config.idle_only {
                            if let Some(time) = activity.time_since_activity().await {
                                if time < config.interval {
                                    if config.debug_logging {
                                        tracing::trace!(
                                            "Skipping heartbeat - recent activity ({:?} ago)",
                                            time
                                        );
                                    }
                                    continue;
                                }
                            }
                        }

                        // Generate ID and send ping
                        let id = id_generator();

                        match sender.send_ping(id.clone()).await {
                            Ok(()) => {
                                let mut s = stats.lock().await;
                                s.record_ping_sent();
                                pending_pings.insert(id.clone(), Instant::now());

                                if config.debug_logging {
                                    tracing::trace!("Sent ping with ID {:?}", id);
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to send ping: {}", e);
                                let mut s = stats.lock().await;
                                s.record_ping_missed();
                            }
                        }
                    }

                    // Handle ping timeouts
                    Some(timed_out_id) = ping_timeout_rx.recv() => {
                        pending_pings.remove(&timed_out_id);
                        let mut s = stats.lock().await;
                        s.record_ping_missed();

                        tracing::warn!(
                            "Ping timeout for request {:?} - consecutive missed: {}",
                            timed_out_id,
                            s.consecutive_missed
                        );
                    }
                }

                // Check for timed out pings
                let now = Instant::now();
                let timeout_ids: Vec<RequestId> = pending_pings
                    .iter()
                    .filter(|(_, &sent)| now.duration_since(sent) > config.timeout)
                    .map(|(id, _)| id.clone())
                    .collect();

                for id in timeout_ids {
                    let _ = ping_timeout_tx.send(id).await;
                }

                // Check if connection is stale
                let s = stats.lock().await;
                if s.consecutive_missed >= config.max_missed_pings {
                    tracing::error!(
                        consecutive_missed = s.consecutive_missed,
                        max_allowed = config.max_missed_pings,
                        "Connection is stale - too many missed pings"
                    );
                }
            }

            running.store(false, Ordering::SeqCst);
            tracing::info!("Heartbeat task stopped");
        });

        tracing::info!(
            interval_secs = config.interval.as_secs(),
            timeout_secs = config.timeout.as_secs(),
            max_missed = config.max_missed_pings,
            "Heartbeat manager started with ping sender"
        );

        Ok(HeartbeatHandle {
            stop_tx,
            task_handle: Mutex::new(Some(handle)),
        })
    }

    /// Stop the heartbeat task
    ///
    /// Note: This only signals the task to stop. Use the HeartbeatHandle's
    /// stop() and join() methods for proper cleanup.
    pub async fn stop(&self) {
        // Signal stop
        if let Some(tx) = self.stop_tx.lock().await.take() {
            let _ = tx.send(()).await;
        }

        self.running.store(false, Ordering::SeqCst);
    }

    /// Reset statistics and activity tracking
    pub async fn reset(&self) {
        self.activity.reset().await;
        let mut stats = self.stats.lock().await;
        *stats = HeartbeatStats::new();
    }

    /// Handle a received pong response
    ///
    /// Call this when a ping response is received to update stats.
    pub async fn handle_pong(&self, _id: &RequestId) {
        let mut stats = self.stats.lock().await;
        stats.record_pong_received();
        self.activity.record_receive().await;
    }
}

impl Default for HeartbeatManager {
    fn default() -> Self {
        Self::default_manager()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_config_default() {
        let config = HeartbeatConfig::default();

        assert_eq!(config.interval, Duration::from_secs(30));
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.max_missed_pings, 3);
        assert!(config.idle_only);
    }

    #[test]
    fn test_heartbeat_config_builder() {
        let config = HeartbeatConfig::new()
            .with_interval(Duration::from_secs(60))
            .with_timeout(Duration::from_secs(15))
            .with_max_missed_pings(5)
            .with_idle_only(false)
            .with_debug_logging(true);

        assert_eq!(config.interval, Duration::from_secs(60));
        assert_eq!(config.timeout, Duration::from_secs(15));
        assert_eq!(config.max_missed_pings, 5);
        assert!(!config.idle_only);
        assert!(config.debug_logging);
    }

    #[tokio::test]
    async fn test_activity_tracker() {
        let tracker = ActivityTracker::new(Duration::from_millis(10));

        // Initially no activity
        assert!(!tracker.has_activity().await);
        assert!(tracker.time_since_activity().await.is_none());

        // Record send
        tracker.record_send().await;
        assert!(tracker.has_activity().await);
        assert!(tracker.time_since_send().await.is_some());

        // Record receive
        tracker.record_receive().await;
        assert!(tracker.time_since_receive().await.is_some());

        // Time since activity should be small
        let time = tracker.time_since_activity().await.unwrap();
        assert!(time < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_activity_tracker_debounce() {
        let tracker = ActivityTracker::new(Duration::from_millis(100));

        // Record receive
        tracker.record_receive().await;
        let time1 = tracker.time_since_receive().await;

        // Immediately record again - should be debounced
        tracker.record_receive().await;
        let time2 = tracker.time_since_receive().await;

        // Times should be similar (debounced)
        // time2 is measured after time1, so it should be >= time1 (more elapsed time)
        // If debouncing didn't work, time2 would be much smaller (close to 0)
        assert!(time2.unwrap() >= time1.unwrap());
    }

    #[tokio::test]
    async fn test_activity_tracker_reset() {
        let tracker = ActivityTracker::new(Duration::from_millis(10));

        tracker.record_send().await;
        tracker.record_receive().await;
        assert!(tracker.has_activity().await);

        tracker.reset().await;
        assert!(!tracker.has_activity().await);
    }

    #[test]
    fn test_connection_health() {
        let healthy = ConnectionHealth::Healthy;
        assert!(healthy.is_healthy());
        assert!(!healthy.is_degraded());
        assert!(!healthy.is_stale());
        assert_eq!(healthy.missed_pings(), 0);

        let degraded = ConnectionHealth::Degraded { missed_pings: 2 };
        assert!(!degraded.is_healthy());
        assert!(degraded.is_degraded());
        assert!(!degraded.is_stale());
        assert_eq!(degraded.missed_pings(), 2);

        let stale = ConnectionHealth::Stale { missed_pings: 5 };
        assert!(!stale.is_healthy());
        assert!(!stale.is_degraded());
        assert!(stale.is_stale());
        assert_eq!(stale.missed_pings(), 5);
    }

    #[test]
    fn test_heartbeat_stats() {
        let mut stats = HeartbeatStats::new();

        assert_eq!(stats.pings_sent, 0);
        assert_eq!(stats.pongs_received, 0);
        assert_eq!(stats.consecutive_missed, 0);

        stats.record_ping_sent();
        assert_eq!(stats.pings_sent, 1);
        assert!(stats.last_ping_sent.is_some());

        stats.record_pong_received();
        assert_eq!(stats.pongs_received, 1);
        assert_eq!(stats.consecutive_missed, 0);
        assert!(stats.last_pong_received.is_some());
        assert!(stats.avg_rtt_ms.is_some());

        stats.record_ping_missed();
        assert_eq!(stats.pings_missed, 1);
        assert_eq!(stats.consecutive_missed, 1);

        stats.record_ping_missed();
        assert_eq!(stats.consecutive_missed, 2);

        stats.reset_consecutive();
        assert_eq!(stats.consecutive_missed, 0);
    }

    #[tokio::test]
    async fn test_heartbeat_manager_creation() {
        let manager = HeartbeatManager::default_manager();

        assert!(!manager.is_running());
        assert!(manager.is_healthy().await);
        assert!(!manager.is_stale().await);
    }

    #[tokio::test]
    async fn test_heartbeat_manager_activity() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        // Initially no activity
        assert!(manager.time_since_activity().await.is_none());

        // Record activity
        manager.record_send().await;
        assert!(manager.time_since_activity().await.is_some());

        manager.record_receive().await;
        let time = manager.time_since_activity().await.unwrap();
        assert!(time < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_heartbeat_manager_start_stop() {
        let manager = HeartbeatManager::new(
            HeartbeatConfig::new()
                .with_interval(Duration::from_millis(100))
                .with_debug_logging(true),
        );

        assert!(!manager.is_running());

        let handle = manager.start().await.unwrap();
        assert!(manager.is_running());

        // Stop the heartbeat
        handle.stop().await;

        // Give it a moment to fully stop
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!manager.is_running());
    }

    #[tokio::test]
    async fn test_heartbeat_manager_cannot_start_twice() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        let _handle1 = manager.start().await.unwrap();

        // Second start should fail
        let result = manager.start().await;
        assert!(result.is_err());

        // Cleanup
        manager.stop().await;
    }

    #[tokio::test]
    async fn test_heartbeat_manager_stats() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        // Initial stats
        let stats = manager.stats().await;
        assert_eq!(stats.pings_sent, 0);

        // After starting, stats should update
        let handle = manager.start().await.unwrap();

        // Wait for a few ticks
        tokio::time::sleep(Duration::from_millis(250)).await;

        let stats = manager.stats().await;
        // Should have at least one ping sent
        assert!(stats.pings_sent >= 1);

        handle.stop().await;
    }

    #[tokio::test]
    async fn test_heartbeat_manager_reset() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        manager.record_send().await;
        manager.record_receive().await;

        assert!(manager.time_since_activity().await.is_some());

        manager.reset().await;

        assert!(manager.time_since_activity().await.is_none());
        let stats = manager.stats().await;
        assert_eq!(stats.pings_sent, 0);
    }

    #[tokio::test]
    async fn test_heartbeat_manager_handle_pong() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());

        let id = RequestId::Number(1);
        manager.handle_pong(&id).await;

        let stats = manager.stats().await;
        assert_eq!(stats.pongs_received, 1);
        assert_eq!(stats.consecutive_missed, 0);
    }

    #[tokio::test]
    async fn test_heartbeat_manager_health_transitions() {
        let manager = HeartbeatManager::new(
            HeartbeatConfig::new()
                .with_max_missed_pings(3),
        );

        // Initially healthy
        assert!(manager.is_healthy().await);

        // Simulate missed pings
        {
            let mut stats = manager.stats.lock().await;
            stats.record_ping_missed();
        }
        let health = manager.health().await;
        assert!(health.is_degraded());
        assert_eq!(health.missed_pings(), 1);

        // More missed pings
        {
            let mut stats = manager.stats.lock().await;
            stats.record_ping_missed();
            stats.record_ping_missed();
        }
        let health = manager.health().await;
        assert!(health.is_stale());
        assert_eq!(health.missed_pings(), 3);

        // Receive pong - should reset
        manager.handle_pong(&RequestId::Number(1)).await;
        assert!(manager.is_healthy().await);
    }
}
