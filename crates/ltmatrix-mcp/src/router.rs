// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! MCP Request Routing and Response Parsing
//!
//! This module provides the core routing infrastructure to:
//! - Dispatch requests to handlers based on method name
//! - Parse responses into typed structures
//! - Handle correlation with request IDs
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      RequestRouter                               │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐       │
//! │  │  Handler      │  │  Response     │  │  Correlation  │       │
//! │  │  Registry     │  │  Parser       │  │  Integration  │       │
//! │  │               │  │               │  │               │       │
//! │  │  method -> fn │  │  JSON -> Type │  │  ID matching  │       │
//! │  └───────────────┘  └───────────────┘  └───────────────┘       │
//! │         │                   │                   │               │
//! │         └───────────────────┼───────────────────┘               │
//! │                             ▼                                   │
//! │              ┌──────────────────────┐                           │
//! │              │   TypedResponse<T>   │                           │
//! │              └──────────────────────┘                           │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::mcp::router::{RequestRouter, ResponseParser, TypedResponse};
//! use ltmatrix::mcp::protocol::wrappers::{Initialize, McpMethod};
//! use ltmatrix::mcp::correlation::RequestTracker;
//! use ltmatrix::mcp::{RequestId, JsonRpcResponse};
//! use std::time::Duration;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let router = RequestRouter::new();
//!     let tracker = RequestTracker::new(Duration::from_secs(30));
//!
//!     // Register a pending request
//!     let (id, handle) = tracker.register_request("initialize");
//!
//!     // Later, when response arrives, parse it
//!     // let typed = router.parse_typed_response::<Initialize>(response);
//!
//!     Ok(())
//! }
//! ```

use crate::correlation::{PendingRequestHandle, RequestTracker};
use crate::protocol::errors::{McpError, McpErrorCode, McpResult};
use crate::protocol::messages::{
    JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, RequestId,
};
use crate::protocol::wrappers::{McpMethod, McpMethodKind};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

// ============================================================================
// Response Parser
// ============================================================================

/// Parser for JSON-RPC responses into typed structures
///
/// This struct provides type-safe response parsing with automatic
/// error handling and type conversion.
#[derive(Debug, Default)]
pub struct ResponseParser;

impl ResponseParser {
    /// Create a new response parser
    pub fn new() -> Self {
        Self
    }

    /// Parse a raw JSON value into a typed response
    ///
    /// # Type Parameters
    ///
    /// * `T` - The target type to parse into
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The JSON is malformed
    /// - The type conversion fails
    pub fn parse<T: DeserializeOwned>(&self, json: &str) -> McpResult<T> {
        serde_json::from_str(json).map_err(McpError::from)
    }

    /// Parse a JSON value into a typed response
    pub fn parse_value<T: DeserializeOwned>(&self, value: Value) -> McpResult<T> {
        serde_json::from_value(value).map_err(McpError::from)
    }

    /// Parse a JsonRpcResponse and extract the result as a typed value
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The response contains an error
    /// - The result field is missing
    /// - The type conversion fails
    pub fn parse_response<T: DeserializeOwned>(&self, response: JsonRpcResponse) -> McpResult<T> {
        if let Some(error) = response.error {
            return Err(McpError::from_json_rpc(error));
        }

        response
            .result
            .ok_or_else(|| {
                McpError::protocol(
                    McpErrorCode::InternalError,
                    "Missing result field in response",
                )
            })
            .and_then(|value| self.parse_value(value))
    }

    /// Parse a response for a specific MCP method
    ///
    /// This uses the method's type definitions for type-safe parsing.
    pub fn parse_method<M: McpMethod>(&self, response: JsonRpcResponse) -> McpResult<M::Result> {
        M::parse_response(response)
    }

    /// Try to parse a response, returning the raw response on failure
    pub fn try_parse<T: DeserializeOwned>(
        &self,
        response: JsonRpcResponse,
    ) -> Result<T, (JsonRpcResponse, McpError)> {
        // Check for error first, cloning to avoid borrow issues
        if let Some(error) = response.error.clone() {
            return Err((response, McpError::from_json_rpc(error)));
        }

        match response.result {
            Some(value) => match self.parse_value(value) {
                Ok(typed) => Ok(typed),
                Err(e) => Err((
                    JsonRpcResponse::success(response.id.clone(), serde_json::json!({})),
                    e,
                )),
            },
            None => Err((
                response,
                McpError::protocol(McpErrorCode::InternalError, "Missing result field"),
            )),
        }
    }
}

// ============================================================================
// Typed Response
// ============================================================================

/// A type-safe wrapper around a JSON-RPC response
///
/// This struct provides convenient access to typed response data
/// while preserving the original response for debugging.
#[derive(Debug)]
pub struct TypedResponse<T> {
    /// The original JSON-RPC response
    pub raw: JsonRpcResponse,

    /// The parsed typed result
    pub result: T,

    /// The request ID (for correlation)
    pub request_id: RequestId,

    /// The method that was called
    pub method: String,

    /// Response time (from request to response)
    pub response_time_ms: Option<u64>,
}

impl<T> TypedResponse<T> {
    /// Create a new typed response
    pub fn new(raw: JsonRpcResponse, result: T, method: String) -> Self {
        let request_id = raw.id.clone();
        Self {
            raw,
            result,
            request_id,
            method,
            response_time_ms: None,
        }
    }

    /// Add response time information
    pub fn with_response_time(mut self, ms: u64) -> Self {
        self.response_time_ms = Some(ms);
        self
    }

    /// Check if this was a successful response
    pub fn is_success(&self) -> bool {
        self.raw.is_success()
    }

    /// Get a reference to the result
    pub fn result(&self) -> &T {
        &self.result
    }

    /// Consume and return the result
    pub fn into_result(self) -> T {
        self.result
    }
}

impl<T: Clone> Clone for TypedResponse<T> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            result: self.result.clone(),
            request_id: self.request_id.clone(),
            method: self.method.clone(),
            response_time_ms: self.response_time_ms,
        }
    }
}

// ============================================================================
// Request Handler Trait
// ============================================================================

/// Trait for handling incoming requests
///
/// Implement this trait to provide custom request handling logic.
#[async_trait::async_trait]
pub trait RequestHandler: Send + Sync {
    /// Handle an incoming request
    ///
    /// Returns the response result as a JSON value, or an error.
    async fn handle(&self, request: JsonRpcRequest) -> McpResult<Value>;
}

/// Type-safe request handler for a specific MCP method
#[async_trait::async_trait]
pub trait TypedRequestHandler<M: McpMethod>: Send + Sync {
    /// Handle the request with typed parameters
    async fn handle(&self, params: M::Params) -> McpResult<M::Result>;
}

/// Function type for async handlers
pub type AsyncHandlerFn = Arc<
    dyn Fn(JsonRpcRequest) -> Pin<Box<dyn Future<Output = McpResult<Value>> + Send>> + Send + Sync,
>;

// ============================================================================
// Notification Handler Trait
// ============================================================================

/// Trait for handling incoming notifications
#[async_trait::async_trait]
pub trait NotificationHandler: Send + Sync {
    /// Handle an incoming notification
    async fn handle(&self, notification: JsonRpcNotification);
}

/// Function type for notification handlers
pub type NotificationHandlerFn =
    Arc<dyn Fn(JsonRpcNotification) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

// ============================================================================
// Request Router
// ============================================================================

/// Router for dispatching requests to handlers
///
/// This struct manages:
/// - Request handler registration
/// - Notification handler registration
/// - Request dispatch
/// - Response parsing
pub struct RequestRouter {
    /// Request handlers by method name
    handlers: RwLock<HashMap<String, AsyncHandlerFn>>,

    /// Notification handlers by method name
    notification_handlers: RwLock<HashMap<String, NotificationHandlerFn>>,

    /// Response parser
    parser: ResponseParser,

    /// Router statistics
    stats: Mutex<RouterStats>,
}

/// Statistics for the request router
#[derive(Debug, Clone, Default)]
pub struct RouterStats {
    /// Total requests handled
    pub requests_handled: u64,

    /// Total notifications handled
    pub notifications_handled: u64,

    /// Total errors
    pub errors: u64,

    /// Requests by method
    pub requests_by_method: HashMap<String, u64>,
}

impl RequestRouter {
    /// Create a new request router
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            notification_handlers: RwLock::new(HashMap::new()),
            parser: ResponseParser::new(),
            stats: Mutex::new(RouterStats::default()),
        }
    }

    /// Register a handler for a method
    ///
    /// # Arguments
    ///
    /// * `method` - The method name to handle
    /// * `handler` - The handler function
    pub async fn register_handler<F, Fut>(&self, method: &str, handler: F)
    where
        F: Fn(JsonRpcRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = McpResult<Value>> + Send + 'static,
    {
        let handler_fn: AsyncHandlerFn = Arc::new(move |req| Box::pin(handler(req)));
        let mut handlers = self.handlers.write().await;
        handlers.insert(method.to_string(), handler_fn);
    }

    /// Register a handler for a notification
    ///
    /// # Arguments
    ///
    /// * `method` - The notification method name
    /// * `handler` - The handler function
    pub async fn register_notification_handler<F, Fut>(&self, method: &str, handler: F)
    where
        F: Fn(JsonRpcNotification) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler_fn: NotificationHandlerFn = Arc::new(move |notif| Box::pin(handler(notif)));
        let mut handlers = self.notification_handlers.write().await;
        handlers.insert(method.to_string(), handler_fn);
    }

    /// Dispatch a request to the appropriate handler
    ///
    /// # Returns
    ///
    /// The response JSON value, or an error if no handler is found.
    pub async fn dispatch(&self, request: JsonRpcRequest) -> McpResult<Value> {
        let method = request.method.clone();
        let request_id = request.id.clone();

        // Update stats
        {
            let mut stats = self.stats.lock().await;
            stats.requests_handled += 1;
            *stats.requests_by_method.entry(method.clone()).or_insert(0) += 1;
        }

        // Find handler
        let handlers = self.handlers.read().await;
        match handlers.get(&method) {
            Some(handler) => {
                let result = handler(request).await;
                if result.is_err() {
                    let mut stats = self.stats.lock().await;
                    stats.errors += 1;
                }
                result
            }
            None => Err(McpError::with_category(
                McpErrorCode::MethodNotFound,
                format!("No handler registered for method: {}", method),
                crate::protocol::errors::ErrorCategory::Protocol,
            )
            .with_data(serde_json::json!({ "request_id": request_id }))),
        }
    }

    /// Dispatch a notification to the appropriate handler
    pub async fn dispatch_notification(&self, notification: JsonRpcNotification) {
        let method = notification.method.clone();

        // Update stats
        {
            let mut stats = self.stats.lock().await;
            stats.notifications_handled += 1;
        }

        // Find handler
        let handlers = self.notification_handlers.read().await;
        if let Some(handler) = handlers.get(&method) {
            handler(notification).await;
        } else {
            tracing::debug!(
                method = %method,
                "No handler registered for notification"
            );
        }
    }

    /// Parse a response into a typed structure
    ///
    /// # Type Parameters
    ///
    /// * `M` - The MCP method type (implements McpMethod)
    pub fn parse_response<M: McpMethod>(
        &self,
        response: JsonRpcResponse,
    ) -> McpResult<TypedResponse<M::Result>> {
        let method = M::METHOD_NAME.to_string();
        let result = self.parser.parse_method::<M>(response.clone())?;
        Ok(TypedResponse::new(response, result, method))
    }

    /// Parse a raw response into a typed structure
    pub fn parse_raw<T: DeserializeOwned>(
        &self,
        response: JsonRpcResponse,
        method: &str,
    ) -> McpResult<TypedResponse<T>> {
        let result = self.parser.parse_response(response.clone())?;
        Ok(TypedResponse::new(response, result, method.to_string()))
    }

    /// Get router statistics
    pub async fn stats(&self) -> RouterStats {
        self.stats.lock().await.clone()
    }

    /// Check if a handler is registered for a method
    pub async fn has_handler(&self, method: &str) -> bool {
        let handlers = self.handlers.read().await;
        handlers.contains_key(method)
    }

    /// List all registered methods
    pub async fn registered_methods(&self) -> Vec<String> {
        let handlers = self.handlers.read().await;
        handlers.keys().cloned().collect()
    }
}

impl Default for RequestRouter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Response Correlator
// ============================================================================

/// Correlates responses with pending requests
///
/// This struct integrates the RequestTracker with ResponseParser
/// to provide a complete request/response handling solution.
pub struct ResponseCorrelator {
    /// Request tracker for pending requests
    tracker: Arc<RequestTracker>,

    /// Response parser
    parser: ResponseParser,
}

impl ResponseCorrelator {
    /// Create a new response correlator
    pub fn new(tracker: Arc<RequestTracker>) -> Self {
        Self {
            tracker,
            parser: ResponseParser::new(),
        }
    }

    /// Register a pending request
    ///
    /// # Returns
    ///
    /// A handle to wait for the response.
    pub fn register(&self, method: &str) -> (RequestId, PendingRequestHandle) {
        self.tracker.register_request(method)
    }

    /// Register a pending request with custom timeout
    pub fn register_with_timeout(
        &self,
        method: &str,
        timeout: std::time::Duration,
    ) -> (RequestId, PendingRequestHandle) {
        self.tracker.register_with_timeout(method, timeout)
    }

    /// Correlate a response with its pending request
    ///
    /// This method:
    /// 1. Finds the pending request by ID
    /// 2. Parses the response
    /// 3. Completes the pending request
    ///
    /// # Returns
    ///
    /// `true` if the response was correlated successfully.
    pub fn correlate(&self, response: JsonRpcResponse) -> bool {
        self.tracker.correlate(response)
    }

    /// Correlate a response asynchronously
    pub async fn correlate_async(&self, response: JsonRpcResponse) -> bool {
        self.tracker.correlate_async(response).await
    }

    /// Wait for a typed response
    ///
    /// # Type Parameters
    ///
    /// * `M` - The MCP method type
    pub async fn wait_for<M: McpMethod>(
        &self,
        handle: PendingRequestHandle,
    ) -> McpResult<TypedResponse<M::Result>> {
        let method = handle.method().to_string();
        let response = handle.wait().await?;
        self.parser.parse_method::<M>(response.clone())?;
        let result = self.parser.parse_method::<M>(response.clone())?;
        Ok(TypedResponse::new(response, result, method))
    }

    /// Get the underlying tracker
    pub fn tracker(&self) -> &RequestTracker {
        &self.tracker
    }

    /// Get the underlying parser
    pub fn parser(&self) -> &ResponseParser {
        &self.parser
    }
}

// ============================================================================
// Message Classifier
// ============================================================================

/// Classifies incoming JSON-RPC messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    /// Request (expects response)
    Request,
    /// Response (reply to request)
    Response,
    /// Notification (one-way)
    Notification,
}

/// Utility for classifying and routing messages
pub struct MessageClassifier;

impl MessageClassifier {
    /// Classify a raw JSON message
    pub fn classify(json: &str) -> McpResult<MessageKind> {
        let value: Value = serde_json::from_str(json)?;

        // Check if it has an "id" field
        if value.get("id").is_none() {
            // No ID = notification
            return Ok(MessageKind::Notification);
        }

        // Has ID - check for result or error
        if value.get("result").is_some() || value.get("error").is_some() {
            Ok(MessageKind::Response)
        } else {
            Ok(MessageKind::Request)
        }
    }

    /// Parse a raw JSON message into the appropriate type
    pub fn parse(json: &str) -> McpResult<JsonRpcMessage> {
        JsonRpcMessage::from_json(json).map_err(McpError::from)
    }

    /// Check if the message is a known MCP method
    pub fn is_mcp_method(method: &str) -> bool {
        McpMethodKind::from_method_name(method).is_some()
    }

    /// Get the method kind if it's a known MCP method
    pub fn get_method_kind(method: &str) -> Option<McpMethodKind> {
        McpMethodKind::from_method_name(method)
    }
}

// ============================================================================
// Request Builder
// ============================================================================

/// Builder for creating JSON-RPC requests
pub struct RequestBuilder {
    id: RequestId,
    method: String,
    params: Option<Value>,
}

impl RequestBuilder {
    /// Create a new request builder
    pub fn new(id: RequestId, method: impl Into<String>) -> Self {
        Self {
            id,
            method: method.into(),
            params: None,
        }
    }

    /// Add parameters to the request
    pub fn params(mut self, params: impl Serialize) -> McpResult<Self> {
        self.params = Some(serde_json::to_value(params).map_err(McpError::from)?);
        Ok(self)
    }

    /// Add raw JSON parameters
    pub fn params_raw(mut self, params: Value) -> Self {
        self.params = Some(params);
        self
    }

    /// Build the request
    pub fn build(self) -> JsonRpcRequest {
        match self.params {
            Some(params) => JsonRpcRequest::with_params(self.id, self.method, params),
            None => JsonRpcRequest::new(self.id, self.method),
        }
    }

    /// Build and serialize to JSON
    pub fn to_json(self) -> McpResult<String> {
        self.build().to_json().map_err(McpError::from)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::wrappers::{Initialize, Ping, ToolsList};
    use crate::protocol::{ImplementationInfo, InitializeParams, InitializeResult};
    use serde_json::json;
    use std::time::Duration;

    #[test]
    fn test_response_parser_parse_value() {
        let parser = ResponseParser::new();

        let value = json!({"name": "test", "version": "1.0"});
        let info: ImplementationInfo = parser.parse_value(value).unwrap();
        assert_eq!(info.name, "test");
        assert_eq!(info.version, "1.0");
    }

    #[test]
    fn test_response_parser_parse_response() {
        let parser = ResponseParser::new();

        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "serverInfo": {
                    "name": "test-server",
                    "version": "1.0.0"
                }
            }),
        );

        let result: InitializeResult = parser.parse_response(response).unwrap();
        assert_eq!(result.protocol_version, "2025-11-25");
        assert_eq!(result.server_info.name, "test-server");
    }

    #[test]
    fn test_response_parser_parse_error_response() {
        let parser = ResponseParser::new();

        let response = JsonRpcResponse::error(
            RequestId::Number(1),
            crate::protocol::errors::JsonRpcError::method_not_found("unknown"),
        );

        let result: McpResult<InitializeResult> = parser.parse_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_response_parser_parse_method() {
        let parser = ResponseParser::new();

        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "serverInfo": {
                    "name": "test-server",
                    "version": "1.0.0"
                }
            }),
        );

        let result = parser.parse_method::<Initialize>(response).unwrap();
        assert_eq!(result.protocol_version, "2025-11-25");
    }

    #[test]
    fn test_typed_response_creation() {
        let response = JsonRpcResponse::success(RequestId::Number(1), json!({}));

        let typed = TypedResponse::new(response.clone(), 42i32, "test".to_string());

        assert_eq!(typed.result, 42);
        assert_eq!(typed.method, "test");
        assert_eq!(typed.request_id, RequestId::Number(1));
        assert!(typed.response_time_ms.is_none());

        let typed_with_time = typed.with_response_time(100);
        assert_eq!(typed_with_time.response_time_ms, Some(100));
    }

    #[tokio::test]
    async fn test_request_router_creation() {
        let router = RequestRouter::new();
        assert!(!router.has_handler("initialize").await);

        let methods = router.registered_methods().await;
        assert!(methods.is_empty());
    }

    #[tokio::test]
    async fn test_request_router_register_handler() {
        let router = RequestRouter::new();

        router
            .register_handler(
                "test_method",
                |_req| async move { Ok(json!({"result": "ok"})) },
            )
            .await;

        assert!(router.has_handler("test_method").await);

        let methods = router.registered_methods().await;
        assert!(methods.contains(&"test_method".to_string()));
    }

    #[tokio::test]
    async fn test_request_router_dispatch() {
        let router = RequestRouter::new();

        router
            .register_handler(
                "echo",
                |req| async move { Ok(req.params.unwrap_or(json!({}))) },
            )
            .await;

        let request =
            JsonRpcRequest::with_params(RequestId::Number(1), "echo", json!({"message": "hello"}));

        let result = router.dispatch(request).await.unwrap();
        assert_eq!(result["message"], "hello");
    }

    #[tokio::test]
    async fn test_request_router_dispatch_unknown() {
        let router = RequestRouter::new();

        let request = JsonRpcRequest::new(RequestId::Number(1), "unknown_method");
        let result = router.dispatch(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_request_router_stats() {
        let router = RequestRouter::new();

        router
            .register_handler("test", |_req| async move { Ok(json!({})) })
            .await;

        let request = JsonRpcRequest::new(RequestId::Number(1), "test");
        let _ = router.dispatch(request).await;

        let stats = router.stats().await;
        assert_eq!(stats.requests_handled, 1);
        assert_eq!(stats.requests_by_method.get("test"), Some(&1));
    }

    #[test]
    fn test_message_classifier_classify() {
        let request_json = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
        let response_json = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
        let notification_json = r#"{"jsonrpc":"2.0","method":"test"}"#;

        assert_eq!(
            MessageClassifier::classify(request_json).unwrap(),
            MessageKind::Request
        );
        assert_eq!(
            MessageClassifier::classify(response_json).unwrap(),
            MessageKind::Response
        );
        assert_eq!(
            MessageClassifier::classify(notification_json).unwrap(),
            MessageKind::Notification
        );
    }

    #[test]
    fn test_message_classifier_is_mcp_method() {
        assert!(MessageClassifier::is_mcp_method("initialize"));
        assert!(MessageClassifier::is_mcp_method("tools/list"));
        assert!(MessageClassifier::is_mcp_method("ping"));
        assert!(!MessageClassifier::is_mcp_method("unknown_method"));
    }

    #[test]
    fn test_request_builder_basic() {
        let request = RequestBuilder::new(RequestId::Number(1), "ping").build();

        assert_eq!(request.method, "ping");
        assert!(request.params.is_none());
    }

    #[test]
    fn test_request_builder_with_params() {
        let request = RequestBuilder::new(RequestId::Number(1), "initialize")
            .params(InitializeParams::new("test", "1.0"))
            .unwrap()
            .build();

        assert_eq!(request.method, "initialize");
        assert!(request.params.is_some());
    }

    #[test]
    fn test_request_builder_to_json() {
        let json = RequestBuilder::new(RequestId::Number(1), "ping")
            .to_json()
            .unwrap();

        assert!(json.contains("\"method\":\"ping\""));
    }

    #[test]
    fn test_response_correlator() {
        let tracker = Arc::new(RequestTracker::new(Duration::from_secs(30)));
        let correlator = ResponseCorrelator::new(tracker);

        let (id, _handle) = correlator.register("test");

        // The ID should be valid
        match id {
            RequestId::Number(n) => assert!(n > 0),
            _ => panic!("Expected numeric ID"),
        }
    }

    #[test]
    fn test_parse_method_tools_list() {
        let parser = ResponseParser::new();

        let response = JsonRpcResponse::success(
            RequestId::Number(1),
            json!({
                "tools": [
                    {
                        "name": "test_tool",
                        "description": "A test tool",
                        "inputSchema": {"type": "object"}
                    }
                ]
            }),
        );

        let result = parser.parse_method::<ToolsList>(response).unwrap();
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "test_tool");
    }

    #[test]
    fn test_parse_method_ping() {
        let parser = ResponseParser::new();

        let response = JsonRpcResponse::success(RequestId::Number(1), json!({}));

        let result = parser.parse_method::<Ping>(response).unwrap();
        // PingResult is empty, just verify it parses
        let _ = result;
    }

    #[tokio::test]
    async fn test_router_notification_handler() {
        let router = RequestRouter::new();

        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let counter_clone = counter.clone();

        router
            .register_notification_handler("test_notification", move |_notif| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            })
            .await;

        let notification = JsonRpcNotification::new("test_notification");
        router.dispatch_notification(notification).await;

        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);

        let stats = router.stats().await;
        assert_eq!(stats.notifications_handled, 1);
    }

    #[test]
    fn test_typed_response_clone() {
        let response = JsonRpcResponse::success(RequestId::Number(1), json!({}));
        let typed = TypedResponse::new(response, "result".to_string(), "method".to_string());

        let cloned = typed.clone();
        assert_eq!(cloned.result, "result");
    }
}
