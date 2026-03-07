// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Request/Response Correlation System
//!
//! This module provides request tracking and response correlation for MCP clients.
//! It handles:
//!
//! - Unique request ID generation
//! - Pending request tracking with metadata
//! - Timeout handling per request
//! - Response correlation to match responses to their requests
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     RequestTracker                               │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌──────────────────┐  ┌──────────────────┐                    │
//! │  │  ID Generator    │  │  Pending Map     │                    │
//! │  │  (AtomicU64)     │  │  HashMap<ID,     │                    │
//! │  │                  │  │    PendingReq>   │                    │
//! │  └──────────────────┘  └──────────────────┘                    │
//! │           │                     │                               │
//! │           │                     │                               │
//! │           ▼                     ▼                               │
//! │  ┌──────────────────────────────────────────┐                  │
//! │  │          oneshot::Sender<Response>       │                  │
//! │  │    (for async response waiting)          │                  │
//! │  └──────────────────────────────────────────┘                  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::mcp::correlation::RequestTracker;
//! use std::time::Duration;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let tracker = RequestTracker::new(Duration::from_secs(30));
//!
//!     // Register a pending request
//!     let (id, handle) = tracker.register_request("initialize");
//!
//!     // Wait for response (with timeout)
//!     // let response = handle.wait().await?;
//!
//!     // Or correlate a received response
//!     // tracker.correlate(response);
//!
//!     Ok(())
//! }
//! ```

use crate::mcp::protocol::errors::{McpError, McpErrorCode, McpResult};
use crate::mcp::protocol::messages::RequestId;
use crate::mcp::JsonRpcResponse;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{oneshot, Mutex, RwLock};

// ============================================================================
// Pending Request
// ============================================================================

/// A pending request awaiting a response
///
/// This struct holds metadata about an in-flight request and provides
/// a mechanism to receive the response asynchronously.
#[derive(Debug)]
pub struct PendingRequest {
    /// The request ID
    pub id: RequestId,

    /// The method name being called
    pub method: String,

    /// When the request was created
    pub created_at: Instant,

    /// Timeout duration for this request
    pub timeout: Duration,

    /// Sender for the response (oneshot channel)
    response_tx: oneshot::Sender<McpResult<JsonRpcResponse>>,

    /// Receiver for the response (oneshot channel)
    response_rx: Option<oneshot::Receiver<McpResult<JsonRpcResponse>>>,
}

impl PendingRequest {
    /// Create a new pending request
    ///
    /// # Arguments
    ///
    /// * `id` - The request ID
    /// * `method` - The method name
    /// * `timeout` - Timeout duration
    pub fn new(id: RequestId, method: String, timeout: Duration) -> Self {
        let (response_tx, response_rx) = oneshot::channel();

        Self {
            id,
            method,
            created_at: Instant::now(),
            timeout,
            response_tx,
            response_rx: Some(response_rx),
        }
    }

    /// Check if the request has timed out
    pub fn is_timed_out(&self) -> bool {
        self.created_at.elapsed() > self.timeout
    }

    /// Get the remaining time until timeout
    pub fn remaining(&self) -> Duration {
        self.timeout.saturating_sub(self.created_at.elapsed())
    }

    /// Get the elapsed time since request creation
    pub fn elapsed(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Take the response receiver
    ///
    /// This should only be called once. Returns `None` if already taken.
    pub fn take_receiver(&mut self) -> Option<oneshot::Receiver<McpResult<JsonRpcResponse>>> {
        self.response_rx.take()
    }

    /// Complete the request with a response
    ///
    /// This sends the response through the oneshot channel.
    /// Returns `false` if the receiver was already dropped.
    pub fn complete(self, response: McpResult<JsonRpcResponse>) -> bool {
        self.response_tx.send(response).is_ok()
    }

    /// Fail the request with an error
    ///
    /// Convenience method for completing with an error.
    pub fn fail(self, error: McpError) -> bool {
        self.complete(Err(error))
    }
}

/// A handle to wait for a pending request's response
#[derive(Debug)]
pub struct PendingRequestHandle {
    /// The request ID
    pub id: RequestId,

    /// The method name
    pub method: String,

    /// Receiver for the response
    receiver: oneshot::Receiver<McpResult<JsonRpcResponse>>,

    /// Timeout duration
    timeout: Duration,

    /// Creation timestamp
    created_at: Instant,
}

impl PendingRequestHandle {
    /// Wait for the response with the configured timeout
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The request times out
    /// - The response channel is closed
    /// - The response itself contains an error
    pub async fn wait(mut self) -> McpResult<JsonRpcResponse> {
        // Calculate remaining timeout
        let remaining = self.timeout.saturating_sub(self.created_at.elapsed());

        // Wait with timeout
        let result = tokio::time::timeout(remaining, &mut self.receiver)
            .await
            .map_err(|_| {
                McpError::timeout(&self.method, self.timeout)
                    .with_data(serde_json::json!({
                        "request_id": self.id.to_string()
                    }))
            })?;

        // Handle channel closure
        result.map_err(|_| {
            McpError::communication(format!(
                "Response channel closed for request {:?} ({})",
                self.id, self.method
            ))
        })?
    }

    /// Get the request ID
    pub fn id(&self) -> &RequestId {
        &self.id
    }

    /// Get the method name
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Check if the request has timed out
    pub fn is_timed_out(&self) -> bool {
        self.created_at.elapsed() > self.timeout
    }

    /// Get remaining time
    pub fn remaining(&self) -> Duration {
        self.timeout.saturating_sub(self.created_at.elapsed())
    }
}

// ============================================================================
// Request Tracker
// ============================================================================

/// Tracker for pending requests with timeout handling
///
/// This struct manages all pending requests and provides:
///
/// - Thread-safe request registration
/// - Automatic timeout detection
/// - Response correlation
/// - Request cancellation
///
/// # Thread Safety
///
/// All methods are thread-safe and can be called from multiple threads.
#[derive(Debug)]
pub struct RequestTracker {
    /// Pending requests map
    pending: Arc<Mutex<HashMap<RequestId, PendingRequest>>>,

    /// Request ID generator
    id_counter: AtomicU64,

    /// Default timeout for requests
    default_timeout: Duration,

    /// Statistics
    stats: Arc<RwLock<TrackerStats>>,
}

/// Statistics for the request tracker
#[derive(Debug, Clone, Default)]
pub struct TrackerStats {
    /// Total requests registered
    pub total_requests: u64,

    /// Total requests completed successfully
    pub successful: u64,

    /// Total requests that timed out
    pub timeouts: u64,

    /// Total requests that failed with error
    pub errors: u64,

    /// Total requests cancelled
    pub cancelled: u64,

    /// Current pending count
    pub current_pending: usize,
}

impl RequestTracker {
    /// Create a new request tracker
    ///
    /// # Arguments
    ///
    /// * `default_timeout` - Default timeout for requests
    pub fn new(default_timeout: Duration) -> Self {
        Self {
            pending: Arc::new(Mutex::new(HashMap::new())),
            id_counter: AtomicU64::new(1),
            default_timeout,
            stats: Arc::new(RwLock::new(TrackerStats::default())),
        }
    }

    /// Create a new request tracker with custom configuration
    pub fn with_timeout(timeout: Duration) -> Self {
        Self::new(timeout)
    }

    /// Generate the next request ID
    ///
    /// Uses an atomic counter for thread-safe ID generation.
    pub fn generate_id(&self) -> RequestId {
        let id = self.id_counter.fetch_add(1, Ordering::SeqCst);
        RequestId::Number(id as i64)
    }

    /// Register a new pending request
    ///
    /// # Arguments
    ///
    /// * `method` - The method name being called
    ///
    /// # Returns
    ///
    /// A tuple of (request_id, handle) where the handle can be used to wait for the response.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::mcp::correlation::RequestTracker;
    /// use std::time::Duration;
    ///
    /// let tracker = RequestTracker::new(Duration::from_secs(30));
    /// let (id, handle) = tracker.register_request("initialize");
    ///
    /// // Later, when response arrives:
    /// // tracker.complete_request(&id, response);
    /// ```
    pub fn register_request(&self, method: &str) -> (RequestId, PendingRequestHandle) {
        self.register_with_timeout(method, self.default_timeout)
    }

    /// Register a new pending request with custom timeout
    ///
    /// # Arguments
    ///
    /// * `method` - The method name being called
    /// * `timeout` - Timeout duration for this specific request
    pub fn register_with_timeout(
        &self,
        method: &str,
        timeout: Duration,
    ) -> (RequestId, PendingRequestHandle) {
        let id = self.generate_id();
        let mut pending = PendingRequest::new(id.clone(), method.to_string(), timeout);
        let receiver = pending.take_receiver().expect("Receiver should be available");

        // Store pending request
        {
            let mut map = self.pending.blocking_lock();
            map.insert(id.clone(), pending);
        }

        // Update stats
        {
            let mut stats = self.stats.blocking_write();
            stats.total_requests += 1;
            stats.current_pending += 1;
        }

        let handle = PendingRequestHandle {
            id: id.clone(),
            method: method.to_string(),
            receiver,
            timeout,
            created_at: Instant::now(),
        };

        tracing::trace!(
            request_id = ?id,
            method = method,
            timeout_ms = timeout.as_millis(),
            "Registered pending request"
        );

        (id, handle)
    }

    /// Register a pending request with a specific ID
    ///
    /// This is useful when you need to use a pre-determined request ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the ID is already in use.
    pub fn register_with_id(
        &self,
        id: RequestId,
        method: &str,
        timeout: Duration,
    ) -> McpResult<PendingRequestHandle> {
        let mut pending = PendingRequest::new(id.clone(), method.to_string(), timeout);
        let receiver = pending.take_receiver().expect("Receiver should be available");

        // Check for duplicate ID and insert
        {
            let mut map = self.pending.blocking_lock();
            if map.contains_key(&id) {
                return Err(McpError::with_category(
                    McpErrorCode::InvalidRequest,
                    format!("Request ID {:?} is already in use", id),
                    crate::mcp::protocol::errors::ErrorCategory::Protocol,
                ));
            }
            map.insert(id.clone(), pending);
        }

        // Update stats
        {
            let mut stats = self.stats.blocking_write();
            stats.total_requests += 1;
            stats.current_pending += 1;
        }

        let handle = PendingRequestHandle {
            id: id.clone(),
            method: method.to_string(),
            receiver,
            timeout,
            created_at: Instant::now(),
        };

        Ok(handle)
    }

    /// Complete a pending request with a response
    ///
    /// # Returns
    ///
    /// `true` if the request was found and completed, `false` otherwise.
    pub fn complete_request(&self, id: &RequestId, response: McpResult<JsonRpcResponse>) -> bool {
        let pending = {
            let mut map = self.pending.blocking_lock();
            map.remove(id)
        };

        let found = pending.is_some();

        if let Some(pending) = pending {
            let method = pending.method.clone();
            let is_success = response.is_ok();
            let completed = pending.complete(response);

            // Update stats
            {
                let mut stats = self.stats.blocking_write();
                stats.current_pending = stats.current_pending.saturating_sub(1);
                if is_success && completed {
                    stats.successful += 1;
                } else if !is_success {
                    stats.errors += 1;
                }
            }

            tracing::trace!(
                request_id = ?id,
                method = method,
                success = is_success,
                "Completed pending request"
            );
        } else {
            tracing::warn!(
                request_id = ?id,
                "Received response for unknown request"
            );
        }

        found
    }

    /// Cancel a pending request
    ///
    /// # Returns
    ///
    /// `true` if the request was found and cancelled, `false` otherwise.
    pub fn cancel_request(&self, id: &RequestId) -> bool {
        let pending = {
            let mut map = self.pending.blocking_lock();
            map.remove(id)
        };

        if let Some(pending) = pending {
            // Save method for logging before moving pending
            let method = pending.method.clone();
            let request_id = pending.id.clone();

            let error = McpError::with_category(
                McpErrorCode::RequestCancelled,
                format!("Request {:?} ({}) was cancelled", request_id, method),
                crate::mcp::protocol::errors::ErrorCategory::Protocol,
            );
            let _ = pending.fail(error);

            // Update stats
            {
                let mut stats = self.stats.blocking_write();
                stats.current_pending = stats.current_pending.saturating_sub(1);
                stats.cancelled += 1;
            }

            tracing::debug!(
                request_id = ?id,
                method = method,
                "Cancelled pending request"
            );

            true
        } else {
            false
        }
    }

    /// Correlate a response to its pending request
    ///
    /// This is the main method for handling incoming responses.
    /// It matches the response ID to a pending request and completes it.
    ///
    /// # Returns
    ///
    /// `true` if the response was correlated to a pending request.
    pub fn correlate(&self, response: JsonRpcResponse) -> bool {
        let id = response.id.clone();
        self.complete_request(&id, Ok(response))
    }

    /// Correlate an error response
    ///
    /// This handles error responses from the server.
    pub fn correlate_error(&self, id: &RequestId, error: McpError) -> bool {
        self.complete_request(id, Err(error))
    }

    /// Check for timed out requests and remove them
    ///
    /// This method should be called periodically to clean up stale requests.
    ///
    /// # Returns
    ///
    /// The number of requests that timed out.
    pub fn cleanup_timeouts(&self) -> usize {
        let timed_out: Vec<(RequestId, String, Duration)> = {
            let map = self.pending.blocking_lock();

            map.iter()
                .filter(|(_, pending)| pending.is_timed_out())
                .map(|(id, pending)| {
                    (id.clone(), pending.method.clone(), pending.elapsed())
                })
                .collect()
        };

        let count = timed_out.len();

        for (id, method, elapsed) in timed_out {
            let error = McpError::timeout(&method, elapsed)
                .with_data(serde_json::json!({
                    "request_id": id.to_string()
                }));
            self.complete_request(&id, Err(error));

            // Update timeout stats
            {
                let mut stats = self.stats.blocking_write();
                stats.timeouts += 1;
            }

            tracing::warn!(
                request_id = ?id,
                method = method,
                elapsed_ms = elapsed.as_millis(),
                "Request timed out"
            );
        }

        count
    }

    /// Cancel all pending requests
    ///
    /// This is useful during shutdown.
    pub fn cancel_all(&self) -> usize {
        let ids: Vec<RequestId> = {
            let map = self.pending.blocking_lock();
            map.keys().cloned().collect()
        };

        let count = ids.len();
        for id in ids {
            self.cancel_request(&id);
        }

        count
    }

    /// Get the number of pending requests
    pub fn pending_count(&self) -> usize {
        let map = self.pending.blocking_lock();
        map.len()
    }

    /// Check if there are any pending requests
    pub fn has_pending(&self) -> bool {
        self.pending_count() > 0
    }

    /// Check if a specific request ID is pending
    pub fn is_pending(&self, id: &RequestId) -> bool {
        let map = self.pending.blocking_lock();
        map.contains_key(id)
    }

    /// Get a snapshot of the current statistics
    pub fn stats(&self) -> TrackerStats {
        let mut stats = self.stats.blocking_read().clone();
        stats.current_pending = self.pending_count();
        stats
    }

    /// Get the default timeout
    pub fn default_timeout(&self) -> Duration {
        self.default_timeout
    }

    /// Set the default timeout
    pub fn set_default_timeout(&mut self, timeout: Duration) {
        self.default_timeout = timeout;
    }

    /// Get list of pending request IDs
    pub fn pending_ids(&self) -> Vec<RequestId> {
        let map = self.pending.blocking_lock();
        map.keys().cloned().collect()
    }

    /// Get information about a pending request
    pub fn get_pending_info(&self, id: &RequestId) -> Option<PendingRequestInfo> {
        let map = self.pending.blocking_lock();
        map.get(id).map(|pending| PendingRequestInfo {
            id: pending.id.clone(),
            method: pending.method.clone(),
            elapsed: pending.elapsed(),
            remaining: pending.remaining(),
            is_timed_out: pending.is_timed_out(),
        })
    }

    // ========================================================================
    // Async versions for use within async context
    // ========================================================================

    /// Async version of `register_request` for use in async context
    pub async fn register_request_async(&self, method: &str) -> (RequestId, PendingRequestHandle) {
        self.register_with_timeout_async(method, self.default_timeout).await
    }

    /// Async version of `register_with_timeout` for use in async context
    pub async fn register_with_timeout_async(
        &self,
        method: &str,
        timeout: Duration,
    ) -> (RequestId, PendingRequestHandle) {
        let id = self.generate_id();
        let mut pending = PendingRequest::new(id.clone(), method.to_string(), timeout);
        let receiver = pending.take_receiver().expect("Receiver should be available");

        // Store pending request using async lock
        {
            let mut map = self.pending.lock().await;
            map.insert(id.clone(), pending);
        }

        // Update stats using async lock
        {
            let mut stats = self.stats.write().await;
            stats.total_requests += 1;
            stats.current_pending += 1;
        }

        let handle = PendingRequestHandle {
            id: id.clone(),
            method: method.to_string(),
            receiver,
            timeout,
            created_at: Instant::now(),
        };

        tracing::trace!(
            request_id = ?id,
            method = method,
            timeout_ms = timeout.as_millis(),
            "Registered pending request (async)"
        );

        (id, handle)
    }

    /// Async version of `complete_request` for use in async context
    pub async fn complete_request_async(&self, id: &RequestId, response: McpResult<JsonRpcResponse>) -> bool {
        let pending = {
            let mut map = self.pending.lock().await;
            map.remove(id)
        };

        let found = pending.is_some();

        if let Some(pending) = pending {
            let method = pending.method.clone();
            let is_success = response.is_ok();
            let completed = pending.complete(response);

            // Update stats using async lock
            {
                let mut stats = self.stats.write().await;
                stats.current_pending = stats.current_pending.saturating_sub(1);
                if is_success && completed {
                    stats.successful += 1;
                } else if !is_success {
                    stats.errors += 1;
                }
            }

            tracing::trace!(
                request_id = ?id,
                method = method,
                success = is_success,
                "Completed pending request (async)"
            );
        } else {
            tracing::warn!(
                request_id = ?id,
                "Received response for unknown request (async)"
            );
        }

        found
    }

    /// Async version of `correlate` for use in async context
    pub async fn correlate_async(&self, response: JsonRpcResponse) -> bool {
        let id = response.id.clone();
        self.complete_request_async(&id, Ok(response)).await
    }

    /// Async version of `is_pending` for use in async context
    pub async fn is_pending_async(&self, id: &RequestId) -> bool {
        let map = self.pending.lock().await;
        map.contains_key(id)
    }

    /// Async version of `pending_count` for use in async context
    pub async fn pending_count_async(&self) -> usize {
        let map = self.pending.lock().await;
        map.len()
    }
}

/// Information about a pending request
#[derive(Debug, Clone)]
pub struct PendingRequestInfo {
    /// Request ID
    pub id: RequestId,

    /// Method name
    pub method: String,

    /// Time elapsed since request was created
    pub elapsed: Duration,

    /// Time remaining until timeout
    pub remaining: Duration,

    /// Whether the request has timed out
    pub is_timed_out: bool,
}

impl Default for RequestTracker {
    fn default() -> Self {
        Self::new(Duration::from_secs(60))
    }
}

// ============================================================================
// Correlation Error
// ============================================================================

/// Error type for correlation failures
#[derive(Debug, Clone)]
pub enum CorrelationError {
    /// No pending request found for the response ID
    NoPendingRequest {
        id: RequestId,
    },

    /// Request has already been completed
    AlreadyCompleted {
        id: RequestId,
    },

    /// Request timed out before response arrived
    Timeout {
        id: RequestId,
        method: String,
        elapsed: Duration,
    },
}

impl std::fmt::Display for CorrelationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CorrelationError::NoPendingRequest { id } => {
                write!(f, "No pending request found for ID {:?}", id)
            }
            CorrelationError::AlreadyCompleted { id } => {
                write!(f, "Request {:?} has already been completed", id)
            }
            CorrelationError::Timeout { id, method, elapsed } => {
                write!(
                    f,
                    "Request {:?} ({}) timed out after {:?}",
                    id, method, elapsed
                )
            }
        }
    }
}

impl std::error::Error for CorrelationError {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::protocol::messages::JsonRpcResponse;
    use serde_json::json;

    #[test]
    fn test_pending_request_creation() {
        let id = RequestId::Number(1);
        let pending = PendingRequest::new(id.clone(), "test".to_string(), Duration::from_secs(30));

        assert_eq!(pending.id, id);
        assert_eq!(pending.method, "test");
        assert!(!pending.is_timed_out());
        assert!(pending.remaining() <= Duration::from_secs(30));
    }

    #[test]
    fn test_pending_request_timeout() {
        let id = RequestId::Number(1);
        let pending = PendingRequest::new(id, "test".to_string(), Duration::from_millis(1));

        // Should not be timed out initially
        assert!(!pending.is_timed_out());

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(5));

        // Now should be timed out
        assert!(pending.is_timed_out());
        assert_eq!(pending.remaining(), Duration::ZERO);
    }

    #[test]
    fn test_request_tracker_creation() {
        let tracker = RequestTracker::new(Duration::from_secs(30));
        assert_eq!(tracker.default_timeout(), Duration::from_secs(30));
        assert_eq!(tracker.pending_count(), 0);
        assert!(!tracker.has_pending());
    }

    #[test]
    fn test_request_tracker_generate_id() {
        let tracker = RequestTracker::new(Duration::from_secs(30));

        let id1 = tracker.generate_id();
        let id2 = tracker.generate_id();
        let id3 = tracker.generate_id();

        // IDs should be sequential
        match (id1, id2, id3) {
            (RequestId::Number(n1), RequestId::Number(n2), RequestId::Number(n3)) => {
                assert_eq!(n2 - n1, 1);
                assert_eq!(n3 - n2, 1);
            }
            _ => panic!("Expected numeric request IDs"),
        }
    }

    #[test]
    fn test_request_tracker_register() {
        let tracker = RequestTracker::new(Duration::from_secs(30));

        let (id, _handle) = tracker.register_request("initialize");

        assert!(tracker.has_pending());
        assert!(tracker.is_pending(&id));
        assert_eq!(tracker.pending_count(), 1);

        let info = tracker.get_pending_info(&id).unwrap();
        assert_eq!(info.method, "initialize");
        assert!(!info.is_timed_out);
    }

    #[test]
    fn test_request_tracker_complete() {
        let tracker = RequestTracker::new(Duration::from_secs(30));

        let (id, _handle) = tracker.register_request("initialize");
        assert!(tracker.has_pending());

        let response = JsonRpcResponse::success(id.clone(), json!({"status": "ok"}));
        let found = tracker.correlate(response);

        assert!(found);
        assert!(!tracker.has_pending());
        assert!(!tracker.is_pending(&id));
    }

    #[test]
    fn test_request_tracker_complete_unknown() {
        let tracker = RequestTracker::new(Duration::from_secs(30));

        let response = JsonRpcResponse::success(RequestId::Number(999), json!({}));
        let found = tracker.correlate(response);

        assert!(!found); // Unknown request ID
    }

    #[test]
    fn test_request_tracker_cancel() {
        let tracker = RequestTracker::new(Duration::from_secs(30));

        let (id, _handle) = tracker.register_request("initialize");
        assert!(tracker.has_pending());

        let cancelled = tracker.cancel_request(&id);

        assert!(cancelled);
        assert!(!tracker.has_pending());
    }

    #[test]
    fn test_request_tracker_cancel_unknown() {
        let tracker = RequestTracker::new(Duration::from_secs(30));

        let cancelled = tracker.cancel_request(&RequestId::Number(999));
        assert!(!cancelled);
    }

    #[test]
    fn test_request_tracker_stats() {
        let tracker = RequestTracker::new(Duration::from_secs(30));

        let (id1, _handle1) = tracker.register_request("method1");
        let (id2, _handle2) = tracker.register_request("method2");

        let stats = tracker.stats();
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.current_pending, 2);

        // Complete one
        let response = JsonRpcResponse::success(id1, json!({}));
        tracker.correlate(response);

        let stats = tracker.stats();
        assert_eq!(stats.successful, 1);
        assert_eq!(stats.current_pending, 1);

        // Cancel the other
        tracker.cancel_request(&id2);

        let stats = tracker.stats();
        assert_eq!(stats.cancelled, 1);
        assert_eq!(stats.current_pending, 0);
    }

    #[test]
    fn test_request_tracker_register_with_id() {
        let tracker = RequestTracker::new(Duration::from_secs(30));

        let custom_id = RequestId::String("custom-id".to_string());
        let handle = tracker.register_with_id(custom_id.clone(), "test", Duration::from_secs(10));

        assert!(handle.is_ok());
        assert!(tracker.is_pending(&custom_id));

        // Duplicate ID should fail
        let result = tracker.register_with_id(custom_id.clone(), "test", Duration::from_secs(10));
        assert!(result.is_err());
    }

    #[test]
    fn test_request_tracker_cleanup_timeouts() {
        let tracker = RequestTracker::new(Duration::from_millis(1));

        let (id, _handle) = tracker.register_request("test");

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(5));

        // Cleanup should find and remove the timed out request
        let cleaned = tracker.cleanup_timeouts();

        assert_eq!(cleaned, 1);
        assert!(!tracker.has_pending());

        let stats = tracker.stats();
        assert_eq!(stats.timeouts, 1);
    }

    #[test]
    fn test_request_tracker_cancel_all() {
        let tracker = RequestTracker::new(Duration::from_secs(30));

        let _ = tracker.register_request("method1");
        let _ = tracker.register_request("method2");
        let _ = tracker.register_request("method3");

        assert_eq!(tracker.pending_count(), 3);

        let cancelled = tracker.cancel_all();

        assert_eq!(cancelled, 3);
        assert!(!tracker.has_pending());
    }

    #[test]
    fn test_pending_request_info() {
        let tracker = RequestTracker::new(Duration::from_secs(30));

        let (id, _handle) = tracker.register_request("test_method");

        let info = tracker.get_pending_info(&id).unwrap();
        assert_eq!(info.id, id);
        assert_eq!(info.method, "test_method");
        assert!(!info.is_timed_out);
    }

    #[test]
    fn test_pending_request_info_unknown() {
        let tracker = RequestTracker::new(Duration::from_secs(30));

        let info = tracker.get_pending_info(&RequestId::Number(999));
        assert!(info.is_none());
    }

    #[test]
    fn test_correlation_error_display() {
        let error = CorrelationError::NoPendingRequest {
            id: RequestId::Number(1),
        };
        assert!(error.to_string().contains("No pending request"));

        let error = CorrelationError::Timeout {
            id: RequestId::Number(1),
            method: "test".to_string(),
            elapsed: Duration::from_secs(10),
        };
        assert!(error.to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn test_pending_request_handle_wait() {
        let tracker = RequestTracker::new(Duration::from_secs(30));

        let (id, handle) = tracker.register_request_async("test").await;

        // Complete the request in another task
        let tracker_clone = tracker;
        let id_clone = id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let response = JsonRpcResponse::success(id_clone, json!({"result": 42}));
            tracker_clone.correlate_async(response).await;
        });

        // Wait for the response
        let response = handle.wait().await.unwrap();
        assert!(response.is_success());
    }

    #[tokio::test]
    async fn test_pending_request_handle_timeout() {
        let tracker = RequestTracker::new(Duration::from_millis(10));

        let (_, handle) = tracker.register_request_async("test").await;

        // Don't complete the request, let it timeout
        let result = handle.wait().await;
        assert!(result.is_err());
    }
}
