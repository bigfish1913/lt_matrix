// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! MCP Notification Handling
//!
//! This module provides comprehensive support for server-initiated notifications
//! in the MCP protocol. It includes:
//!
//! - `NotificationDispatcher`: Central dispatcher for handling notifications
//! - Typed notification events for each notification type
//! - Callback registration for specific notification types
//! - Event subscription and broadcasting
//!
//! # Supported Notifications
//!
//! | Notification | Description |
//! |-------------|-------------|
//! | `notifications/initialized` | Server initialization complete |
//! | `notifications/tools/list_changed` | Tools list has changed |
//! | `notifications/resources/list_changed` | Resources list has changed |
//! | `notifications/prompts/list_changed` | Prompts list has changed |
//! | `notifications/roots/list_changed` | Roots list has changed |
//! | `notifications/progress` | Progress update for long operations |
//! | `notifications/message` | Server log message |
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::mcp::notification::{NotificationDispatcher, NotificationEvent};
//! use ltmatrix::mcp::JsonRpcNotification;
//!
//! async fn example() {
//!     let dispatcher = NotificationDispatcher::new();
//!
//!     // Register a handler for tools list changes
//!     dispatcher.on_tools_list_changed(|| {
//!         println!("Tools list has changed!");
//!         async {}
//!     }).await;
//!
//!     // Register a handler for progress updates
//!     dispatcher.on_progress(|params| {
//!         println!("Progress: {}/{}", params.progress, params.total.unwrap_or(0.0));
//!         async {}
//!     }).await;
//!
//!     // Dispatch a notification
//!     let notification = JsonRpcNotification::new("notifications/tools/list_changed");
//!     dispatcher.dispatch(notification).await;
//! }
//! ```

use crate::mcp::protocol::errors::{McpError, McpResult};
use crate::mcp::protocol::messages::JsonRpcNotification;
use crate::mcp::protocol::wrappers::{
    LogMessageParams, McpNotification, NotificationsInitialized, NotificationsMessage,
    NotificationsProgress, NotificationsPromptsListChanged, NotificationsResourcesListChanged,
    NotificationsRootsListChanged, NotificationsToolsListChanged, ProgressParams,
};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

// ============================================================================
// Notification Event Types
// ============================================================================

/// Enum representing all possible notification events
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    /// Server initialization complete
    Initialized,

    /// Tools list has changed
    ToolsListChanged,

    /// Resources list has changed
    ResourcesListChanged,

    /// Prompts list has changed
    PromptsListChanged,

    /// Roots list has changed
    RootsListChanged,

    /// Progress update
    Progress(ProgressParams),

    /// Log message
    Message(LogMessageParams),

    /// Unknown notification
    Unknown {
        method: String,
        params: Option<serde_json::Value>,
    },
}

impl NotificationEvent {
    /// Get the method name for this event
    pub fn method_name(&self) -> &str {
        match self {
            NotificationEvent::Initialized => NotificationsInitialized::METHOD_NAME,
            NotificationEvent::ToolsListChanged => NotificationsToolsListChanged::METHOD_NAME,
            NotificationEvent::ResourcesListChanged => {
                NotificationsResourcesListChanged::METHOD_NAME
            }
            NotificationEvent::PromptsListChanged => NotificationsPromptsListChanged::METHOD_NAME,
            NotificationEvent::RootsListChanged => NotificationsRootsListChanged::METHOD_NAME,
            NotificationEvent::Progress(_) => NotificationsProgress::METHOD_NAME,
            NotificationEvent::Message(_) => NotificationsMessage::METHOD_NAME,
            NotificationEvent::Unknown { method, .. } => method,
        }
    }

    /// Check if this is a list changed notification
    pub fn is_list_changed(&self) -> bool {
        matches!(
            self,
            NotificationEvent::ToolsListChanged
                | NotificationEvent::ResourcesListChanged
                | NotificationEvent::PromptsListChanged
                | NotificationEvent::RootsListChanged
        )
    }

    /// Parse notification from JSON-RPC notification
    pub fn from_notification(notification: &JsonRpcNotification) -> Self {
        match notification.method.as_str() {
            NotificationsInitialized::METHOD_NAME => NotificationEvent::Initialized,
            NotificationsToolsListChanged::METHOD_NAME => NotificationEvent::ToolsListChanged,
            NotificationsResourcesListChanged::METHOD_NAME => {
                NotificationEvent::ResourcesListChanged
            }
            NotificationsPromptsListChanged::METHOD_NAME => NotificationEvent::PromptsListChanged,
            NotificationsRootsListChanged::METHOD_NAME => NotificationEvent::RootsListChanged,
            NotificationsProgress::METHOD_NAME => {
                if let Some(params) = &notification.params {
                    if let Ok(progress_params) =
                        serde_json::from_value::<ProgressParams>(params.clone())
                    {
                        return NotificationEvent::Progress(progress_params);
                    }
                }
                // Fallback to unknown if params can't be parsed
                NotificationEvent::Unknown {
                    method: notification.method.clone(),
                    params: notification.params.clone(),
                }
            }
            NotificationsMessage::METHOD_NAME => {
                if let Some(params) = &notification.params {
                    if let Ok(log_params) = serde_json::from_value::<LogMessageParams>(params.clone())
                    {
                        return NotificationEvent::Message(log_params);
                    }
                }
                NotificationEvent::Unknown {
                    method: notification.method.clone(),
                    params: notification.params.clone(),
                }
            }
            _ => NotificationEvent::Unknown {
                method: notification.method.clone(),
                params: notification.params.clone(),
            },
        }
    }
}

// ============================================================================
// Handler Types
// ============================================================================

/// Type alias for async notification handlers
type NotificationHandlerFn =
    Arc<dyn Fn(NotificationEvent) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Type alias for raw notification handlers
type RawNotificationHandlerFn =
    Arc<dyn Fn(JsonRpcNotification) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

// ============================================================================
// Notification Dispatcher
// ============================================================================

/// Central dispatcher for handling MCP notifications
///
/// The `NotificationDispatcher` provides:
/// - Type-safe event handling for all notification types
/// - Event broadcasting to multiple subscribers
/// - Callback registration for specific notification types
/// - Statistics tracking
///
/// # Thread Safety
///
/// The dispatcher is thread-safe and can be shared across threads using `Arc`.
pub struct NotificationDispatcher {
    /// Handlers by notification method name
    handlers: RwLock<HashMap<String, NotificationHandlerFn>>,

    /// Raw handlers that receive the full notification
    raw_handlers: RwLock<Vec<RawNotificationHandlerFn>>,

    /// Event broadcaster for pub/sub pattern
    event_sender: broadcast::Sender<NotificationEvent>,

    /// Statistics
    stats: RwLock<NotificationStats>,
}

/// Statistics for notification handling
#[derive(Debug, Clone, Default)]
pub struct NotificationStats {
    /// Total notifications received
    pub total_received: u64,

    /// Notifications by type
    pub by_type: HashMap<String, u64>,

    /// Handlers called
    pub handlers_called: u64,

    /// Parse errors
    pub parse_errors: u64,
}

impl NotificationDispatcher {
    /// Create a new notification dispatcher
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(256);
        Self {
            handlers: RwLock::new(HashMap::new()),
            raw_handlers: RwLock::new(Vec::new()),
            event_sender,
            stats: RwLock::new(NotificationStats::default()),
        }
    }

    /// Create a new dispatcher with custom broadcast capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let (event_sender, _) = broadcast::channel(capacity);
        Self {
            handlers: RwLock::new(HashMap::new()),
            raw_handlers: RwLock::new(Vec::new()),
            event_sender,
            stats: RwLock::new(NotificationStats::default()),
        }
    }

    /// Subscribe to all notification events
    ///
    /// Returns a receiver that will receive all notification events.
    pub fn subscribe(&self) -> broadcast::Receiver<NotificationEvent> {
        self.event_sender.subscribe()
    }

    /// Register a handler for the `notifications/initialized` event
    pub async fn on_initialized<F, Fut>(&self, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler_fn: NotificationHandlerFn =
            Arc::new(move |_event| Box::pin(handler()));
        let mut handlers = self.handlers.write().await;
        handlers.insert(
            NotificationsInitialized::METHOD_NAME.to_string(),
            handler_fn,
        );
    }

    /// Register a handler for the `notifications/tools/list_changed` event
    pub async fn on_tools_list_changed<F, Fut>(&self, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler_fn: NotificationHandlerFn = Arc::new(move |_| Box::pin(handler()));
        let mut handlers = self.handlers.write().await;
        handlers.insert(
            NotificationsToolsListChanged::METHOD_NAME.to_string(),
            handler_fn,
        );
    }

    /// Register a handler for the `notifications/resources/list_changed` event
    pub async fn on_resources_list_changed<F, Fut>(&self, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler_fn: NotificationHandlerFn = Arc::new(move |_| Box::pin(handler()));
        let mut handlers = self.handlers.write().await;
        handlers.insert(
            NotificationsResourcesListChanged::METHOD_NAME.to_string(),
            handler_fn,
        );
    }

    /// Register a handler for the `notifications/prompts/list_changed` event
    pub async fn on_prompts_list_changed<F, Fut>(&self, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler_fn: NotificationHandlerFn = Arc::new(move |_| Box::pin(handler()));
        let mut handlers = self.handlers.write().await;
        handlers.insert(
            NotificationsPromptsListChanged::METHOD_NAME.to_string(),
            handler_fn,
        );
    }

    /// Register a handler for the `notifications/roots/list_changed` event
    pub async fn on_roots_list_changed<F, Fut>(&self, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler_fn: NotificationHandlerFn = Arc::new(move |_| Box::pin(handler()));
        let mut handlers = self.handlers.write().await;
        handlers.insert(
            NotificationsRootsListChanged::METHOD_NAME.to_string(),
            handler_fn,
        );
    }

    /// Register a handler for the `notifications/progress` event
    pub async fn on_progress<F, Fut>(&self, handler: F)
    where
        F: Fn(ProgressParams) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler_fn: NotificationHandlerFn =
            Arc::new(move |event| {
                if let NotificationEvent::Progress(params) = event {
                    Box::pin(handler(params.clone()))
                } else {
                    Box::pin(async {})
                }
            });
        let mut handlers = self.handlers.write().await;
        handlers.insert(NotificationsProgress::METHOD_NAME.to_string(), handler_fn);
    }

    /// Register a handler for the `notifications/message` (log) event
    pub async fn on_message<F, Fut>(&self, handler: F)
    where
        F: Fn(LogMessageParams) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler_fn: NotificationHandlerFn =
            Arc::new(move |event| {
                if let NotificationEvent::Message(params) = event {
                    Box::pin(handler(params.clone()))
                } else {
                    Box::pin(async {})
                }
            });
        let mut handlers = self.handlers.write().await;
        handlers.insert(NotificationsMessage::METHOD_NAME.to_string(), handler_fn);
    }

    /// Register a handler for any notification event
    ///
    /// The handler receives the typed `NotificationEvent`.
    pub async fn on_any_event<F, Fut>(&self, handler: F)
    where
        F: Fn(NotificationEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler_fn: RawNotificationHandlerFn =
            Arc::new(move |notification| {
                let event = NotificationEvent::from_notification(&notification);
                Box::pin(handler(event))
            });
        let mut raw_handlers = self.raw_handlers.write().await;
        raw_handlers.push(handler_fn);
    }

    /// Register a handler for a custom notification type
    pub async fn on_custom<F, Fut>(&self, method: &str, handler: F)
    where
        F: Fn(NotificationEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler_fn: NotificationHandlerFn = Arc::new(move |event| Box::pin(handler(event)));
        let mut handlers = self.handlers.write().await;
        handlers.insert(method.to_string(), handler_fn);
    }

    /// Register a typed handler for a custom notification
    ///
    /// This parses the notification params into the specified type.
    pub async fn on_typed<T, F, Fut>(&self, method: &str, handler: F)
    where
        T: DeserializeOwned + Send + Sync + 'static,
        F: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler_fn: NotificationHandlerFn =
            Arc::new(move |event| {
                if let NotificationEvent::Unknown { params, .. } = &event {
                    if let Some(params_value) = params {
                        if let Ok(typed_params) = serde_json::from_value::<T>(params_value.clone())
                        {
                            return Box::pin(handler(typed_params));
                        }
                    }
                }
                Box::pin(async {})
            });
        let mut handlers = self.handlers.write().await;
        handlers.insert(method.to_string(), handler_fn);
    }

    /// Dispatch a notification to registered handlers
    ///
    /// This method:
    /// 1. Parses the notification into a typed event
    /// 2. Broadcasts the event to all subscribers
    /// 3. Calls any registered handlers for this notification type
    /// 4. Updates statistics
    pub async fn dispatch(&self, notification: JsonRpcNotification) {
        let event = NotificationEvent::from_notification(&notification);
        let method = notification.method.clone();

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_received += 1;
            *stats.by_type.entry(method.clone()).or_insert(0) += 1;
        }

        // Broadcast to subscribers (ignore if no receivers)
        let _ = self.event_sender.send(event.clone());

        // Call specific handler
        {
            let handlers = self.handlers.read().await;
            if let Some(handler) = handlers.get(&method) {
                handler(event.clone()).await;
                let mut stats = self.stats.write().await;
                stats.handlers_called += 1;
            }
        }

        // Call raw handlers
        {
            let raw_handlers = self.raw_handlers.read().await;
            for handler in raw_handlers.iter() {
                handler(notification.clone()).await;
                let mut stats = self.stats.write().await;
                stats.handlers_called += 1;
            }
        }
    }

    /// Dispatch a notification from raw JSON
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON cannot be parsed.
    pub async fn dispatch_json(&self, json: &str) -> McpResult<()> {
        let notification: JsonRpcNotification =
            serde_json::from_str(json).map_err(McpError::from)?;

        // Validate it's a notification (no id field)
        // This is already handled by JsonRpcNotification deserialization

        self.dispatch(notification).await;
        Ok(())
    }

    /// Get notification statistics
    pub async fn stats(&self) -> NotificationStats {
        self.stats.read().await.clone()
    }

    /// Check if there are any handlers registered for a method
    pub async fn has_handler(&self, method: &str) -> bool {
        let handlers = self.handlers.read().await;
        handlers.contains_key(method)
    }

    /// Get all registered handler methods
    pub async fn registered_methods(&self) -> Vec<String> {
        let handlers = self.handlers.read().await;
        handlers.keys().cloned().collect()
    }

    /// Clear all handlers
    pub async fn clear_handlers(&self) {
        let mut handlers = self.handlers.write().await;
        handlers.clear();
        let mut raw_handlers = self.raw_handlers.write().await;
        raw_handlers.clear();
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = NotificationStats::default();
    }
}

impl Default for NotificationDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Notification Builder
// ============================================================================

/// Builder for creating notifications
pub struct NotificationBuilder {
    method: String,
    params: Option<serde_json::Value>,
}

impl NotificationBuilder {
    /// Create a new notification builder
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            params: None,
        }
    }

    /// Create a builder for `notifications/initialized`
    pub fn initialized() -> Self {
        Self::new(NotificationsInitialized::METHOD_NAME)
    }

    /// Create a builder for `notifications/tools/list_changed`
    pub fn tools_list_changed() -> Self {
        Self::new(NotificationsToolsListChanged::METHOD_NAME)
    }

    /// Create a builder for `notifications/resources/list_changed`
    pub fn resources_list_changed() -> Self {
        Self::new(NotificationsResourcesListChanged::METHOD_NAME)
    }

    /// Create a builder for `notifications/progress`
    pub fn progress(token: impl Into<serde_json::Value>, progress: f64) -> Self {
        Self::new(NotificationsProgress::METHOD_NAME).params(ProgressParams {
            progress_token: token.into(),
            progress,
            total: None,
        })
    }

    /// Create a builder for `notifications/message`
    pub fn message(level: crate::mcp::LogLevel, data: impl Into<String>) -> Self {
        Self::new(NotificationsMessage::METHOD_NAME).params(LogMessageParams {
            level,
            logger: None,
            data: data.into(),
        })
    }

    /// Add parameters to the notification
    pub fn params<T: serde::Serialize>(mut self, params: T) -> Self {
        self.params = serde_json::to_value(params).ok();
        self
    }

    /// Build the notification
    pub fn build(self) -> JsonRpcNotification {
        JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: self.method,
            params: self.params,
        }
    }

    /// Build and serialize to JSON
    pub fn to_json(self) -> McpResult<String> {
        let notification = self.build();
        serde_json::to_string(&notification).map_err(McpError::from)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::LogLevel;
    use serde_json::json;

    #[test]
    fn test_notification_event_from_notification() {
        let notification = JsonRpcNotification::new(NotificationsInitialized::METHOD_NAME);
        let event = NotificationEvent::from_notification(&notification);
        assert!(matches!(event, NotificationEvent::Initialized));
    }

    #[test]
    fn test_notification_event_progress() {
        let notification = JsonRpcNotification::with_params(
            NotificationsProgress::METHOD_NAME,
            json!({
                "progressToken": "token-123",
                "progress": 50.0,
                "total": 100.0
            }),
        );
        let event = NotificationEvent::from_notification(&notification);
        if let NotificationEvent::Progress(params) = event {
            assert_eq!(params.progress, 50.0);
            assert_eq!(params.total, Some(100.0));
        } else {
            panic!("Expected Progress event");
        }
    }

    #[test]
    fn test_notification_event_unknown() {
        let notification =
            JsonRpcNotification::with_params("custom/notification", json!({"key": "value"}));
        let event = NotificationEvent::from_notification(&notification);
        if let NotificationEvent::Unknown { method, params } = event {
            assert_eq!(method, "custom/notification");
            assert!(params.is_some());
        } else {
            panic!("Expected Unknown event");
        }
    }

    #[test]
    fn test_notification_event_is_list_changed() {
        assert!(NotificationEvent::ToolsListChanged.is_list_changed());
        assert!(NotificationEvent::ResourcesListChanged.is_list_changed());
        assert!(!NotificationEvent::Initialized.is_list_changed());
        assert!(!NotificationEvent::Progress(ProgressParams {
            progress_token: json!(""),
            progress: 0.0,
            total: None
        })
        .is_list_changed());
    }

    #[tokio::test]
    async fn test_dispatcher_on_initialized() {
        let dispatcher = NotificationDispatcher::new();
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));

        {
            let called_clone = called.clone();
            dispatcher
                .on_initialized(move || {
                    let called = called_clone.clone();
                    async move {
                        called.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let notification = JsonRpcNotification::new(NotificationsInitialized::METHOD_NAME);
        dispatcher.dispatch(notification).await;

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_dispatcher_on_progress() {
        let dispatcher = NotificationDispatcher::new();
        let progress_value = Arc::new(std::sync::atomic::AtomicU64::new(0));

        {
            let progress_clone = progress_value.clone();
            dispatcher
                .on_progress(move |params| {
                    let progress = progress_clone.clone();
                    async move {
                        progress.store(params.progress as u64, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let notification = JsonRpcNotification::with_params(
            NotificationsProgress::METHOD_NAME,
            json!({
                "progressToken": "token",
                "progress": 75.0
            }),
        );
        dispatcher.dispatch(notification).await;

        assert_eq!(
            progress_value.load(std::sync::atomic::Ordering::SeqCst),
            75
        );
    }

    #[tokio::test]
    async fn test_dispatcher_stats() {
        let dispatcher = NotificationDispatcher::new();

        dispatcher
            .on_tools_list_changed(|| async {})
            .await;

        let notification1 = JsonRpcNotification::new(NotificationsToolsListChanged::METHOD_NAME);
        let notification2 = JsonRpcNotification::new(NotificationsInitialized::METHOD_NAME);

        dispatcher.dispatch(notification1).await;
        dispatcher.dispatch(notification2).await;

        let stats = dispatcher.stats().await;
        assert_eq!(stats.total_received, 2);
        assert_eq!(stats.handlers_called, 1); // Only tools/list_changed has a handler
    }

    #[tokio::test]
    async fn test_dispatcher_subscribe() {
        let dispatcher = NotificationDispatcher::new();
        let mut receiver = dispatcher.subscribe();

        let notification = JsonRpcNotification::new(NotificationsInitialized::METHOD_NAME);
        dispatcher.dispatch(notification).await;

        let event = receiver.try_recv().unwrap();
        assert!(matches!(event, NotificationEvent::Initialized));
    }

    #[tokio::test]
    async fn test_dispatcher_on_any_event() {
        let dispatcher = NotificationDispatcher::new();
        let count = Arc::new(std::sync::atomic::AtomicU64::new(0));

        {
            let count_clone = count.clone();
            dispatcher
                .on_any_event(move |_| {
                    let count = count_clone.clone();
                    async move {
                        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let n1 = JsonRpcNotification::new(NotificationsInitialized::METHOD_NAME);
        let n2 = JsonRpcNotification::new(NotificationsToolsListChanged::METHOD_NAME);

        dispatcher.dispatch(n1).await;
        dispatcher.dispatch(n2).await;

        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn test_notification_builder_initialized() {
        let notification = NotificationBuilder::initialized().build();
        assert_eq!(notification.method, NotificationsInitialized::METHOD_NAME);
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_notification_builder_progress() {
        let notification = NotificationBuilder::progress("token-123", 50.0).build();
        assert_eq!(notification.method, NotificationsProgress::METHOD_NAME);
        assert!(notification.params.is_some());
    }

    #[test]
    fn test_notification_builder_message() {
        let notification = NotificationBuilder::message(LogLevel::Info, "Test message").build();
        assert_eq!(notification.method, NotificationsMessage::METHOD_NAME);
        assert!(notification.params.is_some());
    }

    #[test]
    fn test_notification_builder_to_json() {
        let json = NotificationBuilder::tools_list_changed().to_json().unwrap();
        assert!(json.contains("notifications/tools/list_changed"));
    }

    #[tokio::test]
    async fn test_dispatcher_dispatch_json() {
        let dispatcher = NotificationDispatcher::new();
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));

        {
            let called_clone = called.clone();
            dispatcher
                .on_initialized(move || {
                    let called = called_clone.clone();
                    async move {
                        called.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let json = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        dispatcher.dispatch_json(json).await.unwrap();

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_dispatcher_has_handler() {
        let dispatcher = NotificationDispatcher::new();

        assert!(
            !dispatcher
                .has_handler(NotificationsInitialized::METHOD_NAME)
                .await
        );

        dispatcher.on_initialized(|| async {}).await;

        assert!(
            dispatcher
                .has_handler(NotificationsInitialized::METHOD_NAME)
                .await
        );
    }

    #[tokio::test]
    async fn test_dispatcher_registered_methods() {
        let dispatcher = NotificationDispatcher::new();

        dispatcher.on_initialized(|| async {}).await;
        dispatcher.on_tools_list_changed(|| async {}).await;

        let methods = dispatcher.registered_methods().await;
        assert_eq!(methods.len(), 2);
        assert!(methods.contains(&NotificationsInitialized::METHOD_NAME.to_string()));
        assert!(
            methods.contains(&NotificationsToolsListChanged::METHOD_NAME.to_string())
        );
    }

    #[tokio::test]
    async fn test_dispatcher_clear_handlers() {
        let dispatcher = NotificationDispatcher::new();

        dispatcher.on_initialized(|| async {}).await;
        assert!(
            dispatcher
                .has_handler(NotificationsInitialized::METHOD_NAME)
                .await
        );

        dispatcher.clear_handlers().await;
        assert!(
            !dispatcher
                .has_handler(NotificationsInitialized::METHOD_NAME)
                .await
        );
    }
}
