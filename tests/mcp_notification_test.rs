// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Integration tests for MCP Notification Handling
//!
//! These tests verify notification handling including:
//! - Event dispatching
//! - Callback registration
//! - Event subscription
//! - Statistics tracking
//! - Typed handlers
//! - Multi-subscriber broadcasting
//! - Integration with MCP router

use ltmatrix::mcp::protocol::wrappers::{
    LogMessageParams, McpNotification, NotificationsInitialized, NotificationsMessage,
    NotificationsProgress, NotificationsPromptsListChanged, NotificationsResourcesListChanged,
    NotificationsRootsListChanged, NotificationsToolsListChanged, ProgressParams,
};
use ltmatrix::mcp::{
    JsonRpcNotification, LogLevel, NotificationBuilder, NotificationDispatcher, NotificationEvent,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ============================================================================
// Notification Event Tests
// ============================================================================

mod notification_event_tests {
    use super::*;

    #[test]
    fn test_event_method_name_initialized() {
        let event = NotificationEvent::Initialized;
        assert_eq!(event.method_name(), "notifications/initialized");
    }

    #[test]
    fn test_event_method_name_tools_list_changed() {
        let event = NotificationEvent::ToolsListChanged;
        assert_eq!(event.method_name(), "notifications/tools/list_changed");
    }

    #[test]
    fn test_event_method_name_resources_list_changed() {
        let event = NotificationEvent::ResourcesListChanged;
        assert_eq!(event.method_name(), "notifications/resources/list_changed");
    }

    #[test]
    fn test_event_method_name_prompts_list_changed() {
        let event = NotificationEvent::PromptsListChanged;
        assert_eq!(event.method_name(), "notifications/prompts/list_changed");
    }

    #[test]
    fn test_event_method_name_roots_list_changed() {
        let event = NotificationEvent::RootsListChanged;
        assert_eq!(event.method_name(), "notifications/roots/list_changed");
    }

    #[test]
    fn test_event_method_name_progress() {
        let params = ProgressParams {
            progress_token: json!("token"),
            progress: 50.0,
            total: Some(100.0),
        };
        let event = NotificationEvent::Progress(params);
        assert_eq!(event.method_name(), "notifications/progress");
    }

    #[test]
    fn test_event_method_name_message() {
        let params = LogMessageParams {
            level: LogLevel::Info,
            logger: Some("test".to_string()),
            data: "message".to_string(),
        };
        let event = NotificationEvent::Message(params);
        assert_eq!(event.method_name(), "notifications/message");
    }

    #[test]
    fn test_event_method_name_unknown() {
        let event = NotificationEvent::Unknown {
            method: "custom/notification".to_string(),
            params: Some(json!({"key": "value"})),
        };
        assert_eq!(event.method_name(), "custom/notification");
    }

    #[test]
    fn test_event_is_list_changed() {
        assert!(NotificationEvent::ToolsListChanged.is_list_changed());
        assert!(NotificationEvent::ResourcesListChanged.is_list_changed());
        assert!(NotificationEvent::PromptsListChanged.is_list_changed());
        assert!(NotificationEvent::RootsListChanged.is_list_changed());
        assert!(!NotificationEvent::Initialized.is_list_changed());
        assert!(!NotificationEvent::Progress(ProgressParams {
            progress_token: json!(""),
            progress: 0.0,
            total: None
        })
        .is_list_changed());
        assert!(!NotificationEvent::Message(LogMessageParams {
            level: LogLevel::Info,
            logger: None,
            data: "".to_string(),
        })
        .is_list_changed());
    }
}

// ============================================================================
// Notification Parsing Tests
// ============================================================================

mod notification_parsing_tests {
    use super::*;

    #[test]
    fn test_parse_initialized_notification() {
        let notification = JsonRpcNotification::new(NotificationsInitialized::METHOD_NAME);
        let event = NotificationEvent::from_notification(&notification);
        assert!(matches!(event, NotificationEvent::Initialized));
    }

    #[test]
    fn test_parse_tools_list_changed_notification() {
        let notification = JsonRpcNotification::new(NotificationsToolsListChanged::METHOD_NAME);
        let event = NotificationEvent::from_notification(&notification);
        assert!(matches!(event, NotificationEvent::ToolsListChanged));
    }

    #[test]
    fn test_parse_resources_list_changed_notification() {
        let notification = JsonRpcNotification::new(NotificationsResourcesListChanged::METHOD_NAME);
        let event = NotificationEvent::from_notification(&notification);
        assert!(matches!(event, NotificationEvent::ResourcesListChanged));
    }

    #[test]
    fn test_parse_prompts_list_changed_notification() {
        let notification = JsonRpcNotification::new(NotificationsPromptsListChanged::METHOD_NAME);
        let event = NotificationEvent::from_notification(&notification);
        assert!(matches!(event, NotificationEvent::PromptsListChanged));
    }

    #[test]
    fn test_parse_roots_list_changed_notification() {
        let notification = JsonRpcNotification::new(NotificationsRootsListChanged::METHOD_NAME);
        let event = NotificationEvent::from_notification(&notification);
        assert!(matches!(event, NotificationEvent::RootsListChanged));
    }

    #[test]
    fn test_parse_progress_notification() {
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
            assert_eq!(params.progress_token, json!("token-123"));
            assert_eq!(params.progress, 50.0);
            assert_eq!(params.total, Some(100.0));
        } else {
            panic!("Expected Progress event");
        }
    }

    #[test]
    fn test_parse_progress_notification_without_total() {
        let notification = JsonRpcNotification::with_params(
            NotificationsProgress::METHOD_NAME,
            json!({
                "progressToken": "token-456",
                "progress": 25.0
            }),
        );
        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Progress(params) = event {
            assert_eq!(params.progress, 25.0);
            assert!(params.total.is_none());
        } else {
            panic!("Expected Progress event");
        }
    }

    #[test]
    fn test_parse_progress_notification_with_numeric_token() {
        let notification = JsonRpcNotification::with_params(
            NotificationsProgress::METHOD_NAME,
            json!({
                "progressToken": 42,
                "progress": 75.0
            }),
        );
        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Progress(params) = event {
            assert_eq!(params.progress_token, json!(42));
            assert_eq!(params.progress, 75.0);
        } else {
            panic!("Expected Progress event");
        }
    }

    #[test]
    fn test_parse_progress_notification_with_object_token() {
        let token = json!({"requestId": "req-123", "type": "upload"});
        let notification = JsonRpcNotification::with_params(
            NotificationsProgress::METHOD_NAME,
            json!({
                "progressToken": token,
                "progress": 33.33,
                "total": 100.0
            }),
        );
        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Progress(params) = event {
            assert_eq!(params.progress_token, token);
        } else {
            panic!("Expected Progress event");
        }
    }

    #[test]
    fn test_parse_message_notification() {
        let notification = JsonRpcNotification::with_params(
            NotificationsMessage::METHOD_NAME,
            json!({
                "level": "error",
                "logger": "test-logger",
                "data": "Test error message"
            }),
        );
        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Message(params) = event {
            assert_eq!(params.level, LogLevel::Error);
            assert_eq!(params.logger, Some("test-logger".to_string()));
            assert_eq!(params.data, "Test error message");
        } else {
            panic!("Expected Message event");
        }
    }

    #[test]
    fn test_parse_message_notification_minimal() {
        let notification = JsonRpcNotification::with_params(
            NotificationsMessage::METHOD_NAME,
            json!({
                "level": "info",
                "data": "Simple message"
            }),
        );
        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Message(params) = event {
            assert_eq!(params.level, LogLevel::Info);
            assert!(params.logger.is_none());
            assert_eq!(params.data, "Simple message");
        } else {
            panic!("Expected Message event");
        }
    }

    #[test]
    fn test_parse_unknown_notification() {
        let notification = JsonRpcNotification::with_params(
            "custom/notification",
            json!({"customField": "customValue"}),
        );
        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Unknown { method, params } = event {
            assert_eq!(method, "custom/notification");
            assert!(params.is_some());
        } else {
            panic!("Expected Unknown event");
        }
    }

    #[test]
    fn test_parse_unknown_notification_without_params() {
        let notification = JsonRpcNotification::new("custom/no-params");
        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Unknown { method, params } = event {
            assert_eq!(method, "custom/no-params");
            assert!(params.is_none());
        } else {
            panic!("Expected Unknown event");
        }
    }

    #[test]
    fn test_parse_progress_with_invalid_params() {
        // Missing required fields should fall back to Unknown
        let notification = JsonRpcNotification::with_params(
            NotificationsProgress::METHOD_NAME,
            json!({"invalidField": "value"}),
        );
        let event = NotificationEvent::from_notification(&notification);
        // Should fall back to Unknown because required fields are missing
        assert!(matches!(event, NotificationEvent::Unknown { .. }));
    }

    #[test]
    fn test_parse_message_with_invalid_level() {
        // Invalid log level should fall back to Unknown
        let notification = JsonRpcNotification::with_params(
            NotificationsMessage::METHOD_NAME,
            json!({
                "level": "invalid_level",
                "data": "Test"
            }),
        );
        let event = NotificationEvent::from_notification(&notification);
        assert!(matches!(event, NotificationEvent::Unknown { .. }));
    }
}

// ============================================================================
// Notification Builder Tests
// ============================================================================

mod notification_builder_tests {
    use super::*;

    #[test]
    fn test_builder_initialized() {
        let notification = NotificationBuilder::initialized().build();
        assert_eq!(notification.method, "notifications/initialized");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_builder_tools_list_changed() {
        let notification = NotificationBuilder::tools_list_changed().build();
        assert_eq!(notification.method, "notifications/tools/list_changed");
    }

    #[test]
    fn test_builder_resources_list_changed() {
        let notification = NotificationBuilder::resources_list_changed().build();
        assert_eq!(notification.method, "notifications/resources/list_changed");
    }

    #[test]
    fn test_builder_progress() {
        let notification = NotificationBuilder::progress("token-abc", 75.0).build();
        assert_eq!(notification.method, "notifications/progress");

        let params = notification.params.unwrap();
        assert_eq!(params["progressToken"], "token-abc");
        assert_eq!(params["progress"], 75.0);
    }

    #[test]
    fn test_builder_progress_with_json_token() {
        let token = json!({"type": "request", "id": 123});
        let notification = NotificationBuilder::progress(token.clone(), 50.0).build();

        let params = notification.params.unwrap();
        assert_eq!(params["progressToken"], token);
    }

    #[test]
    fn test_builder_progress_with_numeric_token() {
        let notification = NotificationBuilder::progress(42, 33.33).build();

        let params = notification.params.unwrap();
        assert_eq!(params["progressToken"], 42);
        assert_eq!(params["progress"], 33.33);
    }

    #[test]
    fn test_builder_message() {
        let notification =
            NotificationBuilder::message(LogLevel::Warning, "Warning message").build();
        assert_eq!(notification.method, "notifications/message");

        let params = notification.params.unwrap();
        assert_eq!(params["level"], "warning");
        assert_eq!(params["data"], "Warning message");
    }

    #[test]
    fn test_builder_message_all_levels() {
        let levels = [
            (LogLevel::Debug, "debug"),
            (LogLevel::Info, "info"),
            (LogLevel::Warning, "warning"),
            (LogLevel::Error, "error"),
        ];

        for (level, expected_str) in levels {
            let notification = NotificationBuilder::message(level.clone(), "test").build();
            let params = notification.params.unwrap();
            assert_eq!(params["level"], expected_str);
        }
    }

    #[test]
    fn test_builder_custom() {
        let notification = NotificationBuilder::new("custom/event")
            .params(json!({"key": "value"}))
            .build();

        assert_eq!(notification.method, "custom/event");
        assert_eq!(notification.params.unwrap()["key"], "value");
    }

    #[test]
    fn test_builder_to_json() {
        let json = NotificationBuilder::initialized().to_json().unwrap();
        assert!(json.contains("notifications/initialized"));
        assert!(json.contains("2.0"));
    }

    #[test]
    fn test_builder_custom_params() {
        #[derive(serde::Serialize)]
        struct CustomParams {
            name: String,
            count: i32,
        }

        let notification = NotificationBuilder::new("custom/notification")
            .params(CustomParams {
                name: "test".to_string(),
                count: 42,
            })
            .build();

        let params = notification.params.unwrap();
        assert_eq!(params["name"], "test");
        assert_eq!(params["count"], 42);
    }

    #[test]
    fn test_builder_jsonrpc_version() {
        let notification = NotificationBuilder::initialized().build();
        assert_eq!(notification.jsonrpc, "2.0");
    }
}

// ============================================================================
// Notification Dispatcher Tests
// ============================================================================

mod notification_dispatcher_tests {
    use super::*;

    #[tokio::test]
    async fn test_dispatcher_creation() {
        let dispatcher = NotificationDispatcher::new();
        let stats = dispatcher.stats().await;
        assert_eq!(stats.total_received, 0);
    }

    #[tokio::test]
    async fn test_dispatcher_with_capacity() {
        let dispatcher = NotificationDispatcher::with_capacity(512);
        let stats = dispatcher.stats().await;
        assert_eq!(stats.total_received, 0);
    }

    #[tokio::test]
    async fn test_dispatcher_default() {
        let dispatcher = NotificationDispatcher::default();
        let stats = dispatcher.stats().await;
        assert_eq!(stats.total_received, 0);
    }

    #[tokio::test]
    async fn test_dispatcher_on_initialized_handler() {
        let dispatcher = NotificationDispatcher::new();
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));

        {
            let called_clone = called.clone();
            dispatcher
                .on_initialized(move || {
                    let c = called_clone.clone();
                    async move {
                        c.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let notification = NotificationBuilder::initialized().build();
        dispatcher.dispatch(notification).await;

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_dispatcher_on_tools_list_changed_handler() {
        let dispatcher = NotificationDispatcher::new();
        let count = Arc::new(std::sync::atomic::AtomicU64::new(0));

        {
            let count_clone = count.clone();
            dispatcher
                .on_tools_list_changed(move || {
                    let c = count_clone.clone();
                    async move {
                        c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let n1 = NotificationBuilder::tools_list_changed().build();
        let n2 = NotificationBuilder::tools_list_changed().build();

        dispatcher.dispatch(n1).await;
        dispatcher.dispatch(n2).await;

        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_dispatcher_on_resources_list_changed_handler() {
        let dispatcher = NotificationDispatcher::new();
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));

        {
            let called_clone = called.clone();
            dispatcher
                .on_resources_list_changed(move || {
                    let c = called_clone.clone();
                    async move {
                        c.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let notification = NotificationBuilder::resources_list_changed().build();
        dispatcher.dispatch(notification).await;

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_dispatcher_on_prompts_list_changed_handler() {
        let dispatcher = NotificationDispatcher::new();
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));

        {
            let called_clone = called.clone();
            dispatcher
                .on_prompts_list_changed(move || {
                    let c = called_clone.clone();
                    async move {
                        c.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let notification = JsonRpcNotification::new(NotificationsPromptsListChanged::METHOD_NAME);
        dispatcher.dispatch(notification).await;

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_dispatcher_on_roots_list_changed_handler() {
        let dispatcher = NotificationDispatcher::new();
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));

        {
            let called_clone = called.clone();
            dispatcher
                .on_roots_list_changed(move || {
                    let c = called_clone.clone();
                    async move {
                        c.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let notification = JsonRpcNotification::new(NotificationsRootsListChanged::METHOD_NAME);
        dispatcher.dispatch(notification).await;

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_dispatcher_on_progress_handler() {
        let dispatcher = NotificationDispatcher::new();
        let progress_value = Arc::new(std::sync::atomic::AtomicU64::new(0));

        {
            let progress_clone = progress_value.clone();
            dispatcher
                .on_progress(move |params| {
                    let p = progress_clone.clone();
                    async move {
                        p.store(params.progress as u64, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let notification = NotificationBuilder::progress("token", 75.0).build();
        dispatcher.dispatch(notification).await;

        assert_eq!(progress_value.load(std::sync::atomic::Ordering::SeqCst), 75);
    }

    #[tokio::test]
    async fn test_dispatcher_on_progress_handler_with_total() {
        let dispatcher = NotificationDispatcher::new();
        let total_value = Arc::new(Mutex::new(None));

        {
            let total_clone = total_value.clone();
            dispatcher
                .on_progress(move |params| {
                    let t = total_clone.clone();
                    async move {
                        *t.lock().unwrap() = params.total;
                    }
                })
                .await;
        }

        let notification = JsonRpcNotification::with_params(
            NotificationsProgress::METHOD_NAME,
            json!({
                "progressToken": "token",
                "progress": 50.0,
                "total": 200.0
            }),
        );
        dispatcher.dispatch(notification).await;

        assert_eq!(*total_value.lock().unwrap(), Some(200.0));
    }

    #[tokio::test]
    async fn test_dispatcher_on_message_handler() {
        let dispatcher = NotificationDispatcher::new();
        let message_data = Arc::new(Mutex::new(String::new()));

        {
            let data_clone = message_data.clone();
            dispatcher
                .on_message(move |params| {
                    let d = data_clone.clone();
                    async move {
                        *d.lock().unwrap() = params.data;
                    }
                })
                .await;
        }

        let notification = NotificationBuilder::message(LogLevel::Error, "Test error").build();
        dispatcher.dispatch(notification).await;

        assert_eq!(*message_data.lock().unwrap(), "Test error");
    }

    #[tokio::test]
    async fn test_dispatcher_on_message_handler_with_level() {
        let dispatcher = NotificationDispatcher::new();
        let captured_level = Arc::new(Mutex::new(None));

        {
            let level_clone = captured_level.clone();
            dispatcher
                .on_message(move |params| {
                    let l = level_clone.clone();
                    async move {
                        *l.lock().unwrap() = Some(params.level);
                    }
                })
                .await;
        }

        let notification = NotificationBuilder::message(LogLevel::Debug, "debug msg").build();
        dispatcher.dispatch(notification).await;

        assert_eq!(*captured_level.lock().unwrap(), Some(LogLevel::Debug));
    }

    #[tokio::test]
    async fn test_dispatcher_on_any_event() {
        let dispatcher = NotificationDispatcher::new();
        let count = Arc::new(std::sync::atomic::AtomicU64::new(0));

        {
            let count_clone = count.clone();
            dispatcher
                .on_any_event(move |_| {
                    let c = count_clone.clone();
                    async move {
                        c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let n1 = NotificationBuilder::initialized().build();
        let n2 = NotificationBuilder::tools_list_changed().build();
        let n3 = NotificationBuilder::progress("t", 50.0).build();

        dispatcher.dispatch(n1).await;
        dispatcher.dispatch(n2).await;
        dispatcher.dispatch(n3).await;

        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_dispatcher_on_any_event_receives_event() {
        let dispatcher = NotificationDispatcher::new();
        let events = Arc::new(Mutex::new(Vec::new()));

        {
            let events_clone = events.clone();
            dispatcher
                .on_any_event(move |event| {
                    let e = events_clone.clone();
                    async move {
                        e.lock().unwrap().push(event.method_name().to_string());
                    }
                })
                .await;
        }

        let n1 = NotificationBuilder::initialized().build();
        let n2 = NotificationBuilder::progress("t", 50.0).build();

        dispatcher.dispatch(n1).await;
        dispatcher.dispatch(n2).await;

        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert!(captured.contains(&"notifications/initialized".to_string()));
        assert!(captured.contains(&"notifications/progress".to_string()));
    }

    #[tokio::test]
    async fn test_dispatcher_subscribe() {
        let dispatcher = NotificationDispatcher::new();
        let mut receiver = dispatcher.subscribe();

        let notification = NotificationBuilder::initialized().build();
        dispatcher.dispatch(notification).await;

        let event = receiver.try_recv().unwrap();
        assert!(matches!(event, NotificationEvent::Initialized));
    }

    #[tokio::test]
    async fn test_dispatcher_multiple_subscribers() {
        let dispatcher = NotificationDispatcher::new();
        let mut receiver1 = dispatcher.subscribe();
        let mut receiver2 = dispatcher.subscribe();
        let mut receiver3 = dispatcher.subscribe();

        let notification = NotificationBuilder::tools_list_changed().build();
        dispatcher.dispatch(notification).await;

        // All subscribers should receive the event
        let event1 = receiver1.try_recv().unwrap();
        let event2 = receiver2.try_recv().unwrap();
        let event3 = receiver3.try_recv().unwrap();

        assert!(matches!(event1, NotificationEvent::ToolsListChanged));
        assert!(matches!(event2, NotificationEvent::ToolsListChanged));
        assert!(matches!(event3, NotificationEvent::ToolsListChanged));
    }

    #[tokio::test]
    async fn test_dispatcher_subscribe_receives_progress_params() {
        let dispatcher = NotificationDispatcher::new();
        let mut receiver = dispatcher.subscribe();

        let notification = NotificationBuilder::progress("token-xyz", 66.6).build();
        dispatcher.dispatch(notification).await;

        let event = receiver.try_recv().unwrap();
        if let NotificationEvent::Progress(params) = event {
            assert_eq!(params.progress_token, json!("token-xyz"));
            assert!((params.progress - 66.6).abs() < 0.001);
        } else {
            panic!("Expected Progress event");
        }
    }

    #[tokio::test]
    async fn test_dispatcher_stats() {
        let dispatcher = NotificationDispatcher::new();

        dispatcher.on_initialized(|| async {}).await;

        let n1 = NotificationBuilder::initialized().build();
        let n2 = NotificationBuilder::tools_list_changed().build();

        dispatcher.dispatch(n1).await;
        dispatcher.dispatch(n2).await;

        let stats = dispatcher.stats().await;
        assert_eq!(stats.total_received, 2);
        // handlers_called includes: 1 for initialized handler + 1 for on_any_event (none registered)
        // Only initialized has a specific handler
        assert_eq!(stats.handlers_called, 1);
    }

    #[tokio::test]
    async fn test_dispatcher_stats_by_type() {
        let dispatcher = NotificationDispatcher::new();

        let n1 = NotificationBuilder::initialized().build();
        let n2 = NotificationBuilder::initialized().build();
        let n3 = NotificationBuilder::tools_list_changed().build();

        dispatcher.dispatch(n1).await;
        dispatcher.dispatch(n2).await;
        dispatcher.dispatch(n3).await;

        let stats = dispatcher.stats().await;
        assert_eq!(stats.by_type.get("notifications/initialized"), Some(&2));
        assert_eq!(
            stats.by_type.get("notifications/tools/list_changed"),
            Some(&1)
        );
    }

    #[tokio::test]
    async fn test_dispatcher_stats_includes_all_types() {
        let dispatcher = NotificationDispatcher::new();

        dispatcher
            .dispatch(NotificationBuilder::initialized().build())
            .await;
        dispatcher
            .dispatch(NotificationBuilder::tools_list_changed().build())
            .await;
        dispatcher
            .dispatch(NotificationBuilder::resources_list_changed().build())
            .await;
        dispatcher
            .dispatch(NotificationBuilder::progress("t", 50.0).build())
            .await;
        dispatcher
            .dispatch(NotificationBuilder::message(LogLevel::Info, "msg").build())
            .await;

        let stats = dispatcher.stats().await;
        assert_eq!(stats.by_type.len(), 5);
    }

    #[tokio::test]
    async fn test_dispatcher_dispatch_json() {
        let dispatcher = NotificationDispatcher::new();
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));

        {
            let called_clone = called.clone();
            dispatcher
                .on_initialized(move || {
                    let c = called_clone.clone();
                    async move {
                        c.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let json = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        dispatcher.dispatch_json(json).await.unwrap();

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_dispatcher_dispatch_json_with_params() {
        let dispatcher = NotificationDispatcher::new();
        let progress_value = Arc::new(std::sync::atomic::AtomicU64::new(0));

        {
            let progress_clone = progress_value.clone();
            dispatcher
                .on_progress(move |params| {
                    let p = progress_clone.clone();
                    async move {
                        p.store(params.progress as u64, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let json = r#"{"jsonrpc":"2.0","method":"notifications/progress","params":{"progressToken":"t","progress":42.0}}"#;
        dispatcher.dispatch_json(json).await.unwrap();

        assert_eq!(progress_value.load(std::sync::atomic::Ordering::SeqCst), 42);
    }

    #[tokio::test]
    async fn test_dispatcher_dispatch_invalid_json() {
        let dispatcher = NotificationDispatcher::new();

        let result = dispatcher.dispatch_json("not valid json").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dispatcher_dispatch_json_missing_method() {
        let dispatcher = NotificationDispatcher::new();

        let result = dispatcher.dispatch_json(r#"{"jsonrpc":"2.0"}"#).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dispatcher_has_handler() {
        let dispatcher = NotificationDispatcher::new();

        assert!(!dispatcher.has_handler("notifications/initialized").await);

        dispatcher.on_initialized(|| async {}).await;

        assert!(dispatcher.has_handler("notifications/initialized").await);
    }

    #[tokio::test]
    async fn test_dispatcher_registered_methods() {
        let dispatcher = NotificationDispatcher::new();

        dispatcher.on_initialized(|| async {}).await;
        dispatcher.on_tools_list_changed(|| async {}).await;

        let methods = dispatcher.registered_methods().await;
        assert_eq!(methods.len(), 2);
        assert!(methods.contains(&"notifications/initialized".to_string()));
        assert!(methods.contains(&"notifications/tools/list_changed".to_string()));
    }

    #[tokio::test]
    async fn test_dispatcher_clear_handlers() {
        let dispatcher = NotificationDispatcher::new();

        dispatcher.on_initialized(|| async {}).await;
        assert!(dispatcher.has_handler("notifications/initialized").await);

        dispatcher.clear_handlers().await;
        assert!(!dispatcher.has_handler("notifications/initialized").await);
    }

    #[tokio::test]
    async fn test_dispatcher_clear_handlers_removes_all() {
        let dispatcher = NotificationDispatcher::new();

        dispatcher.on_initialized(|| async {}).await;
        dispatcher.on_tools_list_changed(|| async {}).await;
        dispatcher.on_progress(|_| async {}).await;

        dispatcher.clear_handlers().await;

        assert!(!dispatcher.has_handler("notifications/initialized").await);
        assert!(
            !dispatcher
                .has_handler("notifications/tools/list_changed")
                .await
        );
        assert!(!dispatcher.has_handler("notifications/progress").await);
    }

    #[tokio::test]
    async fn test_dispatcher_reset_stats() {
        let dispatcher = NotificationDispatcher::new();

        let notification = NotificationBuilder::initialized().build();
        dispatcher.dispatch(notification).await;

        let stats = dispatcher.stats().await;
        assert_eq!(stats.total_received, 1);

        dispatcher.reset_stats().await;

        let stats = dispatcher.stats().await;
        assert_eq!(stats.total_received, 0);
        assert!(stats.by_type.is_empty());
    }

    #[tokio::test]
    async fn test_dispatcher_custom_handler() {
        let dispatcher = NotificationDispatcher::new();
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));

        {
            let called_clone = called.clone();
            dispatcher
                .on_custom("custom/event", move |_| {
                    let c = called_clone.clone();
                    async move {
                        c.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let notification = JsonRpcNotification::new("custom/event");
        dispatcher.dispatch(notification).await;

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_dispatcher_no_handler() {
        let dispatcher = NotificationDispatcher::new();

        // Dispatch without any handlers - should not panic
        let notification = NotificationBuilder::initialized().build();
        dispatcher.dispatch(notification).await;

        let stats = dispatcher.stats().await;
        assert_eq!(stats.total_received, 1);
        assert_eq!(stats.handlers_called, 0);
    }

    #[tokio::test]
    async fn test_dispatcher_handler_replacement() {
        let dispatcher = NotificationDispatcher::new();
        let value = Arc::new(std::sync::atomic::AtomicU64::new(0));

        // Register first handler
        {
            let value_clone = value.clone();
            dispatcher
                .on_initialized(move || {
                    let v = value_clone.clone();
                    async move {
                        v.store(1, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        // Replace with second handler
        {
            let value_clone = value.clone();
            dispatcher
                .on_initialized(move || {
                    let v = value_clone.clone();
                    async move {
                        v.store(2, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let notification = NotificationBuilder::initialized().build();
        dispatcher.dispatch(notification).await;

        // Only the second handler should have been called
        assert_eq!(value.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_dispatcher_typed_handler() {
        let dispatcher = NotificationDispatcher::new();

        #[derive(Debug, serde::Deserialize)]
        struct CustomParams {
            name: String,
            count: i32,
        }

        let captured_name = Arc::new(Mutex::new(String::new()));
        let captured_count = Arc::new(std::sync::atomic::AtomicI32::new(0));

        {
            let name_clone = captured_name.clone();
            let count_clone = captured_count.clone();
            dispatcher
                .on_typed::<CustomParams, _, _>("custom/typed", move |params| {
                    let n = name_clone.clone();
                    let c = count_clone.clone();
                    async move {
                        *n.lock().unwrap() = params.name;
                        c.store(params.count, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let notification = JsonRpcNotification::with_params(
            "custom/typed",
            json!({"name": "test-name", "count": 42}),
        );
        dispatcher.dispatch(notification).await;

        assert_eq!(*captured_name.lock().unwrap(), "test-name");
        assert_eq!(captured_count.load(std::sync::atomic::Ordering::SeqCst), 42);
    }
}

// ============================================================================
// Multiple Handlers Tests
// ============================================================================

mod multiple_handlers_tests {
    use super::*;

    #[tokio::test]
    async fn test_multiple_handlers_same_event() {
        let dispatcher = NotificationDispatcher::new();
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        // Register specific handler
        {
            let counter_clone = counter.clone();
            dispatcher
                .on_initialized(move || {
                    let c = counter_clone.clone();
                    async move {
                        c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        // Register generic handler
        {
            let counter_clone = counter.clone();
            dispatcher
                .on_any_event(move |_| {
                    let c = counter_clone.clone();
                    async move {
                        c.fetch_add(10, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let notification = NotificationBuilder::initialized().build();
        dispatcher.dispatch(notification).await;

        // Both handlers should be called: 1 + 10 = 11
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 11);
    }

    #[tokio::test]
    async fn test_multiple_any_event_handlers() {
        let dispatcher = NotificationDispatcher::new();
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        // Register multiple any_event handlers
        {
            let counter_clone = counter.clone();
            dispatcher
                .on_any_event(move |_| {
                    let c = counter_clone.clone();
                    async move {
                        c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        {
            let counter_clone = counter.clone();
            dispatcher
                .on_any_event(move |_| {
                    let c = counter_clone.clone();
                    async move {
                        c.fetch_add(2, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        {
            let counter_clone = counter.clone();
            dispatcher
                .on_any_event(move |_| {
                    let c = counter_clone.clone();
                    async move {
                        c.fetch_add(3, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let notification = NotificationBuilder::initialized().build();
        dispatcher.dispatch(notification).await;

        // All three handlers should be called: 1 + 2 + 3 = 6
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 6);
    }

    #[tokio::test]
    async fn test_concurrent_dispatch() {
        let dispatcher = Arc::new(NotificationDispatcher::new());
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        {
            let counter_clone = counter.clone();
            dispatcher
                .on_initialized(move || {
                    let c = counter_clone.clone();
                    async move {
                        c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let mut handles = vec![];

        for _ in 0..10 {
            let dispatcher_clone = Arc::clone(&dispatcher);
            let handle = tokio::spawn(async move {
                let notification = NotificationBuilder::initialized().build();
                dispatcher_clone.dispatch(notification).await;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn test_concurrent_dispatch_different_types() {
        let dispatcher = Arc::new(NotificationDispatcher::new());
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        {
            let counter_clone = counter.clone();
            dispatcher
                .on_any_event(move |_| {
                    let c = counter_clone.clone();
                    async move {
                        c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        let mut handles = vec![];

        for i in 0..20 {
            let dispatcher_clone = Arc::clone(&dispatcher);
            let handle = tokio::spawn(async move {
                let notification = if i % 4 == 0 {
                    NotificationBuilder::initialized().build()
                } else if i % 4 == 1 {
                    NotificationBuilder::tools_list_changed().build()
                } else if i % 4 == 2 {
                    NotificationBuilder::progress("t", 50.0).build()
                } else {
                    NotificationBuilder::message(LogLevel::Info, "msg").build()
                };
                dispatcher_clone.dispatch(notification).await;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 20);
    }
}

// ============================================================================
// McpNotification Trait Tests
// ============================================================================

mod mcp_notification_trait_tests {
    use super::*;

    #[test]
    fn test_notifications_initialized_build() {
        let notification = NotificationsInitialized::build_notification_empty();
        assert_eq!(notification.method, "notifications/initialized");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_notifications_tools_list_changed_build() {
        let notification = NotificationsToolsListChanged::build_notification_empty();
        assert_eq!(notification.method, "notifications/tools/list_changed");
    }

    #[test]
    fn test_notifications_resources_list_changed_build() {
        let notification = NotificationsResourcesListChanged::build_notification_empty();
        assert_eq!(notification.method, "notifications/resources/list_changed");
    }

    #[test]
    fn test_notifications_prompts_list_changed_build() {
        let notification = NotificationsPromptsListChanged::build_notification_empty();
        assert_eq!(notification.method, "notifications/prompts/list_changed");
    }

    #[test]
    fn test_notifications_roots_list_changed_build() {
        let notification = NotificationsRootsListChanged::build_notification_empty();
        assert_eq!(notification.method, "notifications/roots/list_changed");
    }

    #[test]
    fn test_notifications_progress_build() {
        let params = ProgressParams {
            progress_token: json!("test-token"),
            progress: 42.5,
            total: Some(100.0),
        };
        let notification = NotificationsProgress::build_notification(params);

        assert_eq!(notification.method, "notifications/progress");
        let params = notification.params.unwrap();
        assert_eq!(params["progressToken"], "test-token");
        assert_eq!(params["progress"], 42.5);
        assert_eq!(params["total"], 100.0);
    }

    #[test]
    fn test_notifications_message_build() {
        let params = LogMessageParams {
            level: LogLevel::Warning,
            logger: Some("my-logger".to_string()),
            data: "Warning message".to_string(),
        };
        let notification = NotificationsMessage::build_notification(params);

        assert_eq!(notification.method, "notifications/message");
        let params = notification.params.unwrap();
        assert_eq!(params["level"], "warning");
        assert_eq!(params["logger"], "my-logger");
        assert_eq!(params["data"], "Warning message");
    }
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

mod edge_cases_tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_method_name() {
        let notification = JsonRpcNotification::new("");
        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Unknown { method, .. } = event {
            assert_eq!(method, "");
        } else {
            panic!("Expected Unknown event");
        }
    }

    #[tokio::test]
    async fn test_very_long_progress_value() {
        let notification = NotificationBuilder::progress("token", 999999999999.999999).build();

        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Progress(params) = event {
            assert!(params.progress > 999999999999.0);
        } else {
            panic!("Expected Progress event");
        }
    }

    #[tokio::test]
    async fn test_negative_progress() {
        let notification = JsonRpcNotification::with_params(
            NotificationsProgress::METHOD_NAME,
            json!({
                "progressToken": "token",
                "progress": -50.0
            }),
        );

        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Progress(params) = event {
            assert_eq!(params.progress, -50.0);
        } else {
            panic!("Expected Progress event");
        }
    }

    #[tokio::test]
    async fn test_zero_progress() {
        let notification = NotificationBuilder::progress("token", 0.0).build();
        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Progress(params) = event {
            assert_eq!(params.progress, 0.0);
        } else {
            panic!("Expected Progress event");
        }
    }

    #[tokio::test]
    async fn test_all_log_levels() {
        let levels = [
            (LogLevel::Debug, "debug"),
            (LogLevel::Info, "info"),
            (LogLevel::Warning, "warning"),
            (LogLevel::Error, "error"),
        ];

        for (level, expected_str) in levels {
            let notification = NotificationBuilder::message(level.clone(), "test").build();
            let event = NotificationEvent::from_notification(&notification);

            if let NotificationEvent::Message(params) = event {
                assert_eq!(params.level, level);
            } else {
                panic!("Expected Message event for level {:?}", level);
            }
        }
    }

    #[tokio::test]
    async fn test_unicode_in_message_data() {
        let unicode_msg = "Hello 世界 🌍 Привет";
        let notification = NotificationBuilder::message(LogLevel::Info, unicode_msg).build();
        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Message(params) = event {
            assert_eq!(params.data, unicode_msg);
        } else {
            panic!("Expected Message event");
        }
    }

    #[tokio::test]
    async fn test_unicode_in_method_name() {
        let notification = JsonRpcNotification::new("custom/通知");
        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Unknown { method, .. } = event {
            assert_eq!(method, "custom/通知");
        } else {
            panic!("Expected Unknown event");
        }
    }

    #[tokio::test]
    async fn test_empty_message_data() {
        let notification = NotificationBuilder::message(LogLevel::Info, "").build();
        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Message(params) = event {
            assert_eq!(params.data, "");
        } else {
            panic!("Expected Message event");
        }
    }

    #[tokio::test]
    async fn test_large_nested_params() {
        let large_params = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "items": [1, 2, 3, 4, 5],
                        "nested": {"key": "value"}
                    }
                }
            }
        });

        let notification = JsonRpcNotification::with_params("custom/complex", large_params.clone());
        let event = NotificationEvent::from_notification(&notification);

        if let NotificationEvent::Unknown { params, .. } = event {
            assert_eq!(params, Some(large_params));
        } else {
            panic!("Expected Unknown event");
        }
    }

    #[tokio::test]
    async fn test_dispatcher_stats_accumulates() {
        let dispatcher = NotificationDispatcher::new();

        for i in 0..100 {
            let notification = NotificationBuilder::progress("token", i as f64).build();
            dispatcher.dispatch(notification).await;
        }

        let stats = dispatcher.stats().await;
        assert_eq!(stats.total_received, 100);
        assert_eq!(*stats.by_type.get("notifications/progress").unwrap(), 100);
    }

    #[tokio::test]
    async fn test_dispatcher_clear_preserves_stats() {
        let dispatcher = NotificationDispatcher::new();

        dispatcher.on_initialized(|| async {}).await;
        let notification = NotificationBuilder::initialized().build();
        dispatcher.dispatch(notification).await;

        let stats_before = dispatcher.stats().await;
        assert_eq!(stats_before.total_received, 1);

        dispatcher.clear_handlers().await;

        // Stats should remain after clearing handlers
        let stats_after = dispatcher.stats().await;
        assert_eq!(stats_after.total_received, 1);
    }
}

// ============================================================================
// Thread Safety Tests
// ============================================================================

mod thread_safety_tests {
    use super::*;

    #[tokio::test]
    async fn test_shared_dispatcher_across_tasks() {
        let dispatcher = Arc::new(NotificationDispatcher::new());
        let results = Arc::new(Mutex::new(Vec::new()));

        // Register handler
        {
            let results_clone = results.clone();
            dispatcher
                .on_progress(move |params| {
                    let r = results_clone.clone();
                    async move {
                        r.lock().unwrap().push(params.progress as i32);
                    }
                })
                .await;
        }

        // Spawn multiple tasks
        let mut handles = vec![];
        for i in 0..5 {
            let dispatcher_clone = dispatcher.clone();
            let handle = tokio::spawn(async move {
                let notification = NotificationBuilder::progress("token", i as f64 * 10.0).build();
                dispatcher_clone.dispatch(notification).await;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let results = results.lock().unwrap();
        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_dispatcher_arc_clone() {
        let dispatcher = Arc::new(NotificationDispatcher::new());
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        {
            let counter_clone = counter.clone();
            dispatcher
                .on_any_event(move |_| {
                    let c = counter_clone.clone();
                    async move {
                        c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .await;
        }

        // Clone and use from different "threads" (tasks)
        let d1 = Arc::clone(&dispatcher);
        let d2 = Arc::clone(&dispatcher);

        d1.dispatch(NotificationBuilder::initialized().build())
            .await;
        d2.dispatch(NotificationBuilder::tools_list_changed().build())
            .await;

        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
}
