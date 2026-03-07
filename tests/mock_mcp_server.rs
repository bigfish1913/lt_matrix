// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Mock MCP Server Implementation for Integration Testing
//!
//! This module provides a mock MCP server that can be used for testing
//! the MCP client without requiring a real server process.
//!
//! # Features
//!
//! - In-memory transport implementation
//! - Configurable responses for different MCP methods
//! - Support for error scenarios
//! - Request validation and recording
//!
//! # Example
//!
//! ```rust,ignore
//! use mock_mcp_server::{MockMcpServer, MockServerConfig, MockTransport};
//!
//! #[tokio::test]
//! async fn test_client_handshake() {
//!     let config = MockServerConfig::default();
//!     let server = Arc::new(MockMcpServer::new(config));
//!     let mut transport = MockTransport::new(server);
//!
//!     transport.start().await.unwrap();
//!
//!     // Use transport with McpClient...
//! }
//! ```

use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, RwLock};

// Import ltmatrix MCP types
use ltmatrix::mcp::{
    JsonRpcError, JsonRpcErrorCode, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
    RequestId, Transport, TransportMessage, TransportStats, OutgoingMessage,
};
use ltmatrix::mcp::protocol::methods::{
    ServerCapabilities, ToolsCapability, ResourcesCapability, PromptsCapability, Tool,
    ToolCallResult, Resource, ResourceContents, Prompt, PromptArgument, PromptMessage,
    MCP_PROTOCOL_VERSION,
};
use ltmatrix::mcp::protocol::errors::McpError;

// ============================================================================
// Mock Server Configuration
// ============================================================================

/// Configuration for the mock MCP server
#[derive(Clone)]
pub struct MockServerConfig {
    /// Server name
    pub server_name: String,

    /// Server version
    pub server_version: String,

    /// Protocol version to report
    pub protocol_version: String,

    /// Server capabilities
    pub capabilities: ServerCapabilities,

    /// Tools to expose
    pub tools: Vec<Tool>,

    /// Resources to expose
    pub resources: Vec<Resource>,

    /// Resource contents (uri -> contents)
    pub resource_contents: HashMap<String, ResourceContents>,

    /// Prompts to expose
    pub prompts: Vec<Prompt>,

    /// Prompt templates (name -> messages)
    pub prompt_templates: HashMap<String, Vec<PromptMessage>>,

    /// Simulated delay for responses
    pub response_delay: Duration,

    /// Whether to return errors for specific methods
    pub error_methods: Vec<String>,
}

impl Default for MockServerConfig {
    fn default() -> Self {
        let mut capabilities = ServerCapabilities::default();
        capabilities.tools = Some(ToolsCapability { list_changed: Some(false) });
        capabilities.resources = Some(ResourcesCapability {
            subscribe: Some(false),
            list_changed: Some(false),
        });
        capabilities.prompts = Some(PromptsCapability { list_changed: Some(false) });

        Self {
            server_name: "mock-mcp-server".to_string(),
            server_version: "1.0.0".to_string(),
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities,
            tools: vec![
                Tool::new(
                    "test_tool",
                    "A test tool for integration testing",
                    json!({
                        "type": "object",
                        "properties": {
                            "message": { "type": "string" }
                        }
                    }),
                ),
                Tool::new(
                    "echo",
                    "Echo the input back",
                    json!({
                        "type": "object",
                        "properties": {
                            "text": { "type": "string", "description": "Text to echo" }
                        },
                        "required": ["text"]
                    }),
                ),
            ],
            resources: vec![
                {
                    let mut r1 = Resource::new("file:///test.txt", "test.txt");
                    r1.description = Some("A test text file".to_string());
                    r1
                },
                {
                    let mut r2 = Resource::new("file:///config.json", "config.json");
                    r2.description = Some("Configuration file".to_string());
                    r2
                },
            ],
            resource_contents: {
                let mut map = HashMap::new();
                map.insert(
                    "file:///test.txt".to_string(),
                    ResourceContents::text("file:///test.txt", "Hello, World!"),
                );
                map.insert(
                    "file:///config.json".to_string(),
                    ResourceContents::text("file:///config.json", r#"{"version":"1.0"}"#),
                );
                map
            },
            prompts: vec![
                {
                    let mut prompt = Prompt::new("greeting");
                    prompt.description = Some("A simple greeting prompt".to_string());
                    prompt
                },
                {
                    let mut prompt = Prompt::new("code_review");
                    prompt.description = Some("Code review prompt".to_string());
                    prompt.arguments = Some(vec![
                        PromptArgument::new("language").required(),
                        PromptArgument::new("code"),
                    ]);
                    prompt
                },
            ],
            prompt_templates: {
                let mut map = HashMap::new();
                map.insert(
                    "greeting".to_string(),
                    vec![
                        PromptMessage::user("Hello!"),
                        PromptMessage::assistant("Hi there! How can I help you today?"),
                    ],
                );
                map.insert(
                    "code_review".to_string(),
                    vec![
                        PromptMessage::user("Please review this code."),
                        PromptMessage::assistant("I'll review the code for you."),
                    ],
                );
                map
            },
            response_delay: Duration::from_millis(0),
            error_methods: vec![],
        }
    }
}

impl MockServerConfig {
    /// Create a new config with custom server info
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            server_name: name.into(),
            server_version: version.into(),
            ..Default::default()
        }
    }

    /// Add a tool to the config
    pub fn with_tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }

    /// Add a resource to the config
    pub fn with_resource(mut self, resource: Resource, contents: ResourceContents) -> Self {
        let uri = resource.uri.clone();
        self.resources.push(resource);
        self.resource_contents.insert(uri, contents);
        self
    }

    /// Add a prompt to the config
    pub fn with_prompt(mut self, prompt: Prompt, messages: Vec<PromptMessage>) -> Self {
        let name = prompt.name.clone();
        self.prompts.push(prompt);
        self.prompt_templates.insert(name, messages);
        self
    }

    /// Set response delay
    pub fn with_response_delay(mut self, delay: Duration) -> Self {
        self.response_delay = delay;
        self
    }

    /// Add a method that should return errors
    pub fn with_error_method(mut self, method: impl Into<String>) -> Self {
        self.error_methods.push(method.into());
        self
    }
}

// ============================================================================
// Request Recording
// ============================================================================

/// Record of a received request
#[derive(Debug, Clone)]
pub struct RequestRecord {
    /// Request ID
    pub id: RequestId,

    /// Method name
    pub method: String,

    /// Request parameters
    pub params: Option<Value>,

    /// Timestamp
    pub timestamp: std::time::Instant,
}

/// Records of all requests received by the mock server
#[derive(Debug, Default)]
pub struct RequestLog {
    /// All received requests
    pub requests: Vec<RequestRecord>,
}

impl RequestLog {
    /// Create a new request log
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new request
    pub fn record(&mut self, id: RequestId, method: String, params: Option<Value>) {
        self.requests.push(RequestRecord {
            id,
            method,
            params,
            timestamp: std::time::Instant::now(),
        });
    }

    /// Get requests by method
    pub fn by_method(&self, method: &str) -> Vec<&RequestRecord> {
        self.requests.iter().filter(|r| r.method == method).collect()
    }

    /// Get the last request
    pub fn last(&self) -> Option<&RequestRecord> {
        self.requests.last()
    }

    /// Clear the log
    pub fn clear(&mut self) {
        self.requests.clear();
    }
}

// ============================================================================
// Mock MCP Server
// ============================================================================

/// Mock MCP Server implementation
pub struct MockMcpServer {
    /// Server configuration
    config: MockServerConfig,

    /// Request log
    request_log: Arc<Mutex<RequestLog>>,

    /// Whether the server is started
    started: Arc<RwLock<bool>>,

    /// Incoming message sender (for the transport)
    incoming_tx: mpsc::Sender<TransportMessage>,

    /// Incoming request receiver
    incoming_rx: Mutex<mpsc::Receiver<TransportMessage>>,

    /// Outgoing message sender
    outgoing_tx: mpsc::Sender<OutgoingMessage>,

    /// Outgoing message receiver
    outgoing_rx: Mutex<mpsc::Receiver<OutgoingMessage>>,

    /// Transport statistics
    stats: Arc<Mutex<TransportStats>>,
}

impl MockMcpServer {
    /// Create a new mock server with the given configuration
    pub fn new(config: MockServerConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(100);
        let (outgoing_tx, outgoing_rx) = mpsc::channel(100);

        Self {
            config,
            request_log: Arc::new(Mutex::new(RequestLog::new())),
            started: Arc::new(RwLock::new(false)),
            incoming_rx: Mutex::new(incoming_rx),
            incoming_tx,
            outgoing_tx,
            outgoing_rx: Mutex::new(outgoing_rx),
            stats: Arc::new(Mutex::new(TransportStats::new())),
        }
    }

    /// Create a mock server with default configuration
    pub fn default_server() -> Self {
        Self::new(MockServerConfig::default())
    }

    /// Get the request log
    pub fn request_log(&self) -> Arc<Mutex<RequestLog>> {
        self.request_log.clone()
    }

    /// Check if the server is started
    pub async fn is_started(&self) -> bool {
        *self.started.read().await
    }

    /// Handle an incoming request and return a response
    async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // Record the request
        {
            let mut log = self.request_log.lock().await;
            log.record(
                request.id.clone(),
                request.method.clone(),
                request.params.clone(),
            );
        }

        // Apply response delay
        if self.config.response_delay > Duration::ZERO {
            tokio::time::sleep(self.config.response_delay).await;
        }

        // Check if this method should return an error
        if self.config.error_methods.contains(&request.method) {
            return JsonRpcResponse::error(
                request.id,
                JsonRpcError::new(JsonRpcErrorCode::InternalError, "Simulated error".to_string()),
            );
        }

        // Handle standard MCP methods
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params),
            "ping" => self.handle_ping(request.params),
            "tools/list" => self.handle_tools_list(request.params),
            "tools/call" => self.handle_tools_call(request.params),
            "resources/list" => self.handle_resources_list(request.params),
            "resources/read" => self.handle_resources_read(request.params),
            "prompts/list" => self.handle_prompts_list(request.params),
            "prompts/get" => self.handle_prompts_get(request.params),
            _ => {
                return JsonRpcResponse::error(
                    request.id,
                    JsonRpcError::new(JsonRpcErrorCode::MethodNotFound, "Method not found".to_string()),
                )
            }
        };

        match result {
            Ok(value) => JsonRpcResponse::success(request.id, value),
            Err(error) => JsonRpcResponse::error(request.id, error),
        }
    }

    fn handle_initialize(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        // Validate params exist
        let _params = params.as_ref().ok_or_else(|| {
            JsonRpcError::invalid_params("Missing initialize params")
        })?;

        Ok(json!({
            "protocolVersion": self.config.protocol_version,
            "capabilities": self.config.capabilities,
            "serverInfo": {
                "name": self.config.server_name,
                "version": self.config.server_version
            }
        }))
    }

    fn handle_ping(&self, _params: Option<Value>) -> Result<Value, JsonRpcError> {
        Ok(json!({}))
    }

    fn handle_tools_list(&self, _params: Option<Value>) -> Result<Value, JsonRpcError> {
        Ok(json!({
            "tools": self.config.tools
        }))
    }

    fn handle_tools_call(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params.ok_or_else(|| {
            JsonRpcError::invalid_params("Missing tool call params")
        })?;

        let name = params
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing tool name"))?;

        let arguments = params.get("arguments");

        // Find and execute the tool
        match name {
            "test_tool" => {
                let message = arguments
                    .and_then(|a| a.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("default message");

                Ok(serde_json::to_value(ToolCallResult::text(format!(
                    "Test tool executed: {}",
                    message
                )))
                .unwrap())
            }
            "echo" => {
                let text = arguments
                    .and_then(|a| a.get("text"))
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| JsonRpcError::invalid_params("Missing 'text' parameter"))?;

                Ok(serde_json::to_value(ToolCallResult::text(text)).unwrap())
            }
            "error_tool" => {
                Ok(serde_json::to_value(ToolCallResult::error("Tool execution failed")).unwrap())
            }
            _ => {
                // Check if tool exists in config
                if self.config.tools.iter().any(|t| t.name == name) {
                    Ok(serde_json::to_value(ToolCallResult::text(format!(
                        "Tool '{}' executed",
                        name
                    )))
                    .unwrap())
                } else {
                    Err(JsonRpcError::method_not_found(name))
                }
            }
        }
    }

    fn handle_resources_list(&self, _params: Option<Value>) -> Result<Value, JsonRpcError> {
        Ok(json!({
            "resources": self.config.resources
        }))
    }

    fn handle_resources_read(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params.ok_or_else(|| {
            JsonRpcError::invalid_params("Missing resource read params")
        })?;

        let uri = params
            .get("uri")
            .and_then(|u| u.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing resource URI"))?;

        // Look up the resource contents
        match self.config.resource_contents.get(uri) {
            Some(contents) => Ok(json!({
                "contents": [contents]
            })),
            None => Err(JsonRpcError::invalid_params("Resource not found")),
        }
    }

    fn handle_prompts_list(&self, _params: Option<Value>) -> Result<Value, JsonRpcError> {
        Ok(json!({
            "prompts": self.config.prompts
        }))
    }

    fn handle_prompts_get(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params.ok_or_else(|| {
            JsonRpcError::invalid_params("Missing prompts get params")
        })?;

        let name = params
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing prompt name"))?;

        // Look up the prompt template
        match self.config.prompt_templates.get(name) {
            Some(messages) => {
                let prompt_info = self.config.prompts.iter().find(|p| p.name == name);

                Ok(json!({
                    "description": prompt_info.and_then(|p| p.description.as_ref()),
                    "messages": messages
                }))
            }
            None => Err(JsonRpcError::invalid_params("Prompt not found")),
        }
    }

    /// Process an outgoing message (client -> server)
    pub async fn process_outgoing(&self, message: OutgoingMessage) -> Option<TransportMessage> {
        match message {
            OutgoingMessage::Request(request) => {
                let response = self.handle_request(request).await;
                Some(TransportMessage::Response(response))
            }
            OutgoingMessage::Notification(_notification) => {
                // Notifications don't get responses
                None
            }
            OutgoingMessage::Shutdown => None,
        }
    }
}

// ============================================================================
// Mock Transport Implementation
// ============================================================================

/// Mock transport that uses the mock server for testing
pub struct MockTransport {
    /// Reference to the mock server
    server: Arc<MockMcpServer>,

    /// Whether the transport is connected
    connected: Arc<RwLock<bool>>,

    /// Transport statistics
    stats: Arc<Mutex<TransportStats>>,

    /// Channel buffer size
    buffer_size: usize,
}

impl MockTransport {
    /// Create a new mock transport with the given server
    pub fn new(server: Arc<MockMcpServer>) -> Self {
        Self {
            server,
            connected: Arc::new(RwLock::new(false)),
            stats: Arc::new(Mutex::new(TransportStats::new())),
            buffer_size: 100,
        }
    }

    /// Create a mock transport with default server
    pub fn default_transport() -> Self {
        Self::new(Arc::new(MockMcpServer::default_server()))
    }

    /// Get the server reference
    pub fn server(&self) -> Arc<MockMcpServer> {
        self.server.clone()
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn start(&mut self) -> ltmatrix::mcp::protocol::errors::McpResult<()> {
        let mut connected = self.connected.write().await;
        *connected = true;

        let mut stats = self.stats.lock().await;
        stats.mark_connected();

        // Mark server as started
        let mut started = self.server.started.write().await;
        *started = true;

        Ok(())
    }

    async fn close(&mut self) -> ltmatrix::mcp::protocol::errors::McpResult<()> {
        let mut connected = self.connected.write().await;
        *connected = false;

        let mut stats = self.stats.lock().await;
        stats.mark_disconnected();

        // Mark server as stopped
        let mut started = self.server.started.write().await;
        *started = false;

        Ok(())
    }

    fn is_connected(&self) -> bool {
        match self.connected.try_read() {
            Ok(connected) => *connected,
            Err(_) => false,
        }
    }

    async fn send_request(
        &self,
        request: JsonRpcRequest,
    ) -> ltmatrix::mcp::protocol::errors::McpResult<()> {
        if !self.is_connected() {
            return Err(McpError::communication("Transport not connected"));
        }

        // Process the request through the mock server
        let response = self.server.handle_request(request).await;

        // Send the response back through the incoming channel
        let _ = self.server.incoming_tx.send(TransportMessage::Response(response)).await;

        // Update stats
        let mut stats = self.stats.lock().await;
        stats.record_sent(100); // Approximate bytes

        Ok(())
    }

    async fn send_notification(
        &self,
        notification: JsonRpcNotification,
    ) -> ltmatrix::mcp::protocol::errors::McpResult<()> {
        if !self.is_connected() {
            return Err(McpError::communication("Transport not connected"));
        }

        // Process the notification through the mock server
        // (notifications don't get responses, but we record them)
        {
            let mut log = self.server.request_log.lock().await;
            log.record(
                RequestId::Null,
                notification.method.clone(),
                notification.params.clone(),
            );
        }

        // Update stats
        let mut stats = self.stats.lock().await;
        stats.record_sent(50); // Approximate bytes

        Ok(())
    }

    async fn receive(&self) -> ltmatrix::mcp::protocol::errors::McpResult<TransportMessage> {
        let mut rx = self.server.incoming_rx.lock().await;
        match rx.recv().await {
            Some(message) => {
                let mut stats = self.stats.lock().await;
                stats.record_received(100); // Approximate bytes
                Ok(message)
            }
            None => Err(McpError::communication("Channel closed")),
        }
    }

    fn sender(&self) -> mpsc::Sender<OutgoingMessage> {
        self.server.outgoing_tx.clone()
    }

    fn receiver(&self) -> mpsc::Receiver<TransportMessage> {
        let (_, rx) = mpsc::channel(self.buffer_size);
        rx
    }

    fn stats(&self) -> TransportStats {
        match self.stats.try_lock() {
            Ok(stats) => stats.clone(),
            Err(_) => TransportStats::default(),
        }
    }
}

// ============================================================================
// Helper Functions for Tests
// ============================================================================

/// Create a default mock server with common test configuration
pub fn create_test_server() -> Arc<MockMcpServer> {
    Arc::new(MockMcpServer::default_server())
}

/// Create a mock server with custom tools
pub fn create_server_with_tools(tools: Vec<Tool>) -> Arc<MockMcpServer> {
    let config = MockServerConfig::default();
    let mut config = config;
    config.tools = tools;
    Arc::new(MockMcpServer::new(config))
}

/// Create a mock server that returns errors for specific methods
pub fn create_error_server(error_methods: Vec<&str>) -> Arc<MockMcpServer> {
    let config = MockServerConfig {
        error_methods: error_methods.into_iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    };
    Arc::new(MockMcpServer::new(config))
}

/// Create a mock server with delayed responses
pub fn create_delayed_server(delay: Duration) -> Arc<MockMcpServer> {
    let config = MockServerConfig {
        response_delay: delay,
        ..Default::default()
    };
    Arc::new(MockMcpServer::new(config))
}

// ============================================================================
// Tests for Mock Server
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_server_initialization() {
        let server = MockMcpServer::default_server();
        assert!(!server.is_started().await);
    }

    #[tokio::test]
    async fn test_mock_transport_lifecycle() {
        let server = Arc::new(MockMcpServer::default_server());
        let mut transport = MockTransport::new(server);

        assert!(!transport.is_connected());

        transport.start().await.unwrap();
        assert!(transport.is_connected());

        transport.close().await.unwrap();
        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn test_initialize_request() {
        let server = Arc::new(MockMcpServer::default_server());
        let mut transport = MockTransport::new(server.clone());

        transport.start().await.unwrap();

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "initialize",
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "1.0" }
            }),
        );

        transport.send_request(request).await.unwrap();

        // Receive the response
        let message = transport.receive().await.unwrap();
        assert!(message.is_response());

        let response = message.as_response().unwrap();
        assert!(response.result.is_some());

        // Check request was logged
        let log = server.request_log.lock().await;
        assert_eq!(log.requests.len(), 1);
        assert_eq!(log.requests[0].method, "initialize");
    }

    #[tokio::test]
    async fn test_tools_list() {
        let server = Arc::new(MockMcpServer::default_server());
        let mut transport = MockTransport::new(server);

        transport.start().await.unwrap();

        let request = JsonRpcRequest::new(RequestId::Number(1), "tools/list");
        transport.send_request(request).await.unwrap();

        let message = transport.receive().await.unwrap();
        let response = message.as_response().unwrap();

        let result = response.result.as_ref().unwrap();
        assert!(result.get("tools").is_some());
    }

    #[tokio::test]
    async fn test_tool_call() {
        let server = Arc::new(MockMcpServer::default_server());
        let mut transport = MockTransport::new(server);

        transport.start().await.unwrap();

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "tools/call",
            json!({
                "name": "echo",
                "arguments": { "text": "Hello, World!" }
            }),
        );

        transport.send_request(request).await.unwrap();

        let message = transport.receive().await.unwrap();
        let response = message.as_response().unwrap();

        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_resources_list() {
        let server = Arc::new(MockMcpServer::default_server());
        let mut transport = MockTransport::new(server);

        transport.start().await.unwrap();

        let request = JsonRpcRequest::new(RequestId::Number(1), "resources/list");
        transport.send_request(request).await.unwrap();

        let message = transport.receive().await.unwrap();
        let response = message.as_response().unwrap();

        let result = response.result.as_ref().unwrap();
        assert!(result.get("resources").is_some());
    }

    #[tokio::test]
    async fn test_resource_read() {
        let server = Arc::new(MockMcpServer::default_server());
        let mut transport = MockTransport::new(server);

        transport.start().await.unwrap();

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "resources/read",
            json!({
                "uri": "file:///test.txt"
            }),
        );

        transport.send_request(request).await.unwrap();

        let message = transport.receive().await.unwrap();
        let response = message.as_response().unwrap();

        let result = response.result.as_ref().unwrap();
        assert!(result.get("contents").is_some());
    }

    #[tokio::test]
    async fn test_prompts_list() {
        let server = Arc::new(MockMcpServer::default_server());
        let mut transport = MockTransport::new(server);

        transport.start().await.unwrap();

        let request = JsonRpcRequest::new(RequestId::Number(1), "prompts/list");
        transport.send_request(request).await.unwrap();

        let message = transport.receive().await.unwrap();
        let response = message.as_response().unwrap();

        let result = response.result.as_ref().unwrap();
        assert!(result.get("prompts").is_some());
    }

    #[tokio::test]
    async fn test_prompt_get() {
        let server = Arc::new(MockMcpServer::default_server());
        let mut transport = MockTransport::new(server);

        transport.start().await.unwrap();

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "prompts/get",
            json!({
                "name": "greeting"
            }),
        );

        transport.send_request(request).await.unwrap();

        let message = transport.receive().await.unwrap();
        let response = message.as_response().unwrap();

        let result = response.result.as_ref().unwrap();
        assert!(result.get("messages").is_some());
    }

    #[tokio::test]
    async fn test_error_method() {
        let server = create_error_server(vec!["tools/list"]);
        let mut transport = MockTransport::new(server);

        transport.start().await.unwrap();

        let request = JsonRpcRequest::new(RequestId::Number(1), "tools/list");
        transport.send_request(request).await.unwrap();

        let message = transport.receive().await.unwrap();
        let response = message.as_response().unwrap();

        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let server = Arc::new(MockMcpServer::default_server());
        let mut transport = MockTransport::new(server);

        transport.start().await.unwrap();

        let request = JsonRpcRequest::new(RequestId::Number(1), "unknown/method");
        transport.send_request(request).await.unwrap();

        let message = transport.receive().await.unwrap();
        let response = message.as_response().unwrap();

        assert!(response.error.is_some());
        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, JsonRpcErrorCode::MethodNotFound.as_i32());
    }

    #[test]
    fn test_mock_server_config_builder() {
        let config = MockServerConfig::new("test-server", "2.0.0")
            .with_response_delay(Duration::from_millis(100))
            .with_error_method("test/error");

        assert_eq!(config.server_name, "test-server");
        assert_eq!(config.server_version, "2.0.0");
        assert_eq!(config.response_delay, Duration::from_millis(100));
        assert!(config.error_methods.contains(&"test/error".to_string()));
    }

    #[tokio::test]
    async fn test_request_log() {
        let server = Arc::new(MockMcpServer::default_server());
        let mut transport = MockTransport::new(server.clone());

        transport.start().await.unwrap();

        // Send multiple requests
        for i in 1..=3 {
            let request = JsonRpcRequest::new(RequestId::Number(i), "ping");
            transport.send_request(request).await.unwrap();
            let _ = transport.receive().await;
        }

        // Check log
        let log = server.request_log.lock().await;
        assert_eq!(log.requests.len(), 3);
        assert_eq!(log.by_method("ping").len(), 3);
    }
}
