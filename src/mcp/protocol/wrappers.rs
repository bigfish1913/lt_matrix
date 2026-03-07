// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Type-safe protocol method wrappers for MCP
//!
//! This module provides compile-time type-safe wrappers for MCP protocol methods.
//! Using Rust's type system, we ensure that:
//!
//! - Each method has correctly typed request parameters
//! - Each method returns the correct response type
//! - Method names are validated at compile time
//! - Serialization/deserialization is type-safe
//!
//! # Example
//!
//! ```
//! use ltmatrix::mcp::protocol::wrappers::{McpMethod, Initialize};
//! use ltmatrix::mcp::protocol::{InitializeParams, InitializeResult, RequestId};
//!
//! // Create an initialize request
//! let params = InitializeParams::new("my-client", "1.0.0");
//! let request = Initialize::build_request(RequestId::Number(1), params);
//! assert_eq!(request.method, "initialize");
//! ```

use super::errors::{JsonRpcError, JsonRpcResult, McpError, McpResult};
use super::messages::{JsonRpcRequest, JsonRpcResponse, RequestId};
use super::methods::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// Core Trait Definitions
// ============================================================================

/// Trait for type-safe MCP method wrappers
///
/// This trait provides compile-time type safety for MCP protocol methods.
/// Each implementing type represents a specific MCP method with:
///
/// - A constant method name (validated at compile time)
/// - Associated request parameter type
/// - Associated response result type
/// - Type-safe request building and response parsing
pub trait McpMethod: Sized + Send + Sync + 'static {
    /// The JSON-RPC method name (e.g., "initialize", "tools/list")
    const METHOD_NAME: &'static str;

    /// The request parameter type
    type Params: Serialize + DeserializeOwned + Send + Sync + 'static;

    /// The response result type
    type Result: Serialize + DeserializeOwned + Send + Sync + 'static;

    /// Build a JSON-RPC request for this method
    fn build_request(id: RequestId, params: Self::Params) -> JsonRpcRequest {
        JsonRpcRequest::with_params(id, Self::METHOD_NAME, serde_json::to_value(params).unwrap())
    }

    /// Build a JSON-RPC request with optional params
    fn build_request_optional(id: RequestId, params: Option<Self::Params>) -> JsonRpcRequest {
        match params {
            Some(p) => Self::build_request(id, p),
            None => JsonRpcRequest::new(id, Self::METHOD_NAME),
        }
    }

    /// Parse a JSON-RPC response into the typed result
    fn parse_response(response: JsonRpcResponse) -> McpResult<Self::Result> {
        if let Some(error) = response.error {
            return Err(McpError::from_json_rpc(error));
        }

        response
            .result
            .ok_or_else(|| McpError::protocol(super::errors::McpErrorCode::InternalError, "Missing result field"))
            .and_then(|value| {
                serde_json::from_value(value).map_err(McpError::from)
            })
    }

    /// Parse raw JSON bytes into the typed result
    fn parse_response_json(json: &str) -> McpResult<Self::Result> {
        let response: JsonRpcResponse =
            serde_json::from_str(json).map_err(McpError::from)?;
        Self::parse_response(response)
    }

    /// Serialize params to JSON value
    fn params_to_value(params: Self::Params) -> JsonRpcResult<Value> {
        serde_json::to_value(params).map_err(|e| {
            JsonRpcError::invalid_params(&e.to_string())
        })
    }

    /// Deserialize result from JSON value
    fn result_from_value(value: Value) -> McpResult<Self::Result> {
        serde_json::from_value(value).map_err(McpError::from)
    }
}

/// Trait for MCP methods that support pagination
pub trait PaginatedMethod: McpMethod {
    /// Extract the next cursor from the result
    fn next_cursor(result: &Self::Result) -> Option<&str>;
}

/// Trait for MCP notification types (no response expected)
pub trait McpNotification: Send + Sync + 'static {
    /// The notification method name
    const METHOD_NAME: &'static str;

    /// The notification parameter type
    type Params: Serialize + DeserializeOwned + Send + Sync + 'static;

    /// Build a JSON-RPC notification
    fn build_notification(params: Self::Params) -> super::messages::JsonRpcNotification {
        super::messages::JsonRpcNotification::with_params(
            Self::METHOD_NAME,
            serde_json::to_value(params).unwrap(),
        )
    }

    /// Build notification without params
    fn build_notification_empty() -> super::messages::JsonRpcNotification {
        super::messages::JsonRpcNotification::new(Self::METHOD_NAME)
    }
}

// ============================================================================
// Lifecycle Methods
// ============================================================================

/// Initialize method - Required handshake to establish connection
///
/// This is the first method that must be called when establishing an MCP
/// connection. It exchanges protocol versions and capabilities.
pub struct Initialize;

impl McpMethod for Initialize {
    const METHOD_NAME: &'static str = "initialize";
    type Params = InitializeParams;
    type Result = InitializeResult;
}

impl Initialize {
    /// Create initialize params with default capabilities
    pub fn params(client_name: &str, client_version: &str) -> InitializeParams {
        InitializeParams::new(client_name, client_version)
    }

    /// Create initialize params with custom capabilities
    pub fn params_with_capabilities(
        client_name: &str,
        client_version: &str,
        capabilities: ClientCapabilities,
    ) -> InitializeParams {
        InitializeParams::new(client_name, client_version).with_capabilities(capabilities)
    }
}

/// Ping method - Connection health check
pub struct Ping;

/// Ping request parameters (empty)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PingParams {}

/// Ping response result (empty)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PingResult {}

impl McpMethod for Ping {
    const METHOD_NAME: &'static str = "ping";
    type Params = PingParams;
    type Result = PingResult;
}

// ============================================================================
// Tools Methods
// ============================================================================

/// Tools List method - Discover available tools
pub struct ToolsList;

impl McpMethod for ToolsList {
    const METHOD_NAME: &'static str = "tools/list";
    type Params = ToolsListParams;
    type Result = ToolsListResult;
}

impl PaginatedMethod for ToolsList {
    fn next_cursor(result: &Self::Result) -> Option<&str> {
        result.next_cursor.as_deref()
    }
}

impl ToolsList {
    /// Create a tools/list request with no cursor
    pub fn params() -> ToolsListParams {
        ToolsListParams::default()
    }

    /// Create a tools/list request with pagination cursor
    pub fn params_with_cursor(cursor: impl Into<String>) -> ToolsListParams {
        ToolsListParams {
            cursor: Some(cursor.into()),
        }
    }
}

/// Tools Call method - Execute a tool
pub struct ToolsCall;

impl McpMethod for ToolsCall {
    const METHOD_NAME: &'static str = "tools/call";
    type Params = ToolCallParams;
    type Result = ToolCallResult;
}

impl ToolsCall {
    /// Create a tool call request
    pub fn params(name: impl Into<String>) -> ToolCallParams {
        ToolCallParams::new(name)
    }

    /// Create a tool call request with arguments
    pub fn params_with_args(name: impl Into<String>, arguments: Value) -> ToolCallParams {
        ToolCallParams::new(name).with_arguments(arguments)
    }
}

// ============================================================================
// Resources Methods
// ============================================================================

/// Resources List method - Discover available resources
pub struct ResourcesList;

impl McpMethod for ResourcesList {
    const METHOD_NAME: &'static str = "resources/list";
    type Params = ResourcesListParams;
    type Result = ResourcesListResult;
}

impl PaginatedMethod for ResourcesList {
    fn next_cursor(result: &Self::Result) -> Option<&str> {
        result.next_cursor.as_deref()
    }
}

impl ResourcesList {
    /// Create a resources/list request
    pub fn params() -> ResourcesListParams {
        ResourcesListParams::default()
    }

    /// Create a resources/list request with pagination cursor
    pub fn params_with_cursor(cursor: impl Into<String>) -> ResourcesListParams {
        ResourcesListParams {
            cursor: Some(cursor.into()),
        }
    }
}

/// Resources Read method - Read resource contents
pub struct ResourcesRead;

impl McpMethod for ResourcesRead {
    const METHOD_NAME: &'static str = "resources/read";
    type Params = ResourceReadParams;
    type Result = ResourceReadResult;
}

impl ResourcesRead {
    /// Create a resources/read request
    pub fn params(uri: impl Into<String>) -> ResourceReadParams {
        ResourceReadParams::new(uri)
    }
}

/// Resources Subscribe method - Subscribe to resource updates
pub struct ResourcesSubscribe;

/// Resources subscribe request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesSubscribeParams {
    /// Resource URI to subscribe to
    pub uri: String,
}

/// Resources subscribe response (empty)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourcesSubscribeResult {}

impl McpMethod for ResourcesSubscribe {
    const METHOD_NAME: &'static str = "resources/subscribe";
    type Params = ResourcesSubscribeParams;
    type Result = ResourcesSubscribeResult;
}

/// Resources Unsubscribe method - Unsubscribe from resource updates
pub struct ResourcesUnsubscribe;

/// Resources unsubscribe request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesUnsubscribeParams {
    /// Resource URI to unsubscribe from
    pub uri: String,
}

/// Resources unsubscribe response (empty)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourcesUnsubscribeResult {}

impl McpMethod for ResourcesUnsubscribe {
    const METHOD_NAME: &'static str = "resources/unsubscribe";
    type Params = ResourcesUnsubscribeParams;
    type Result = ResourcesUnsubscribeResult;
}

// ============================================================================
// Prompts Methods
// ============================================================================

/// Prompts List method - Discover available prompts
pub struct PromptsList;

impl McpMethod for PromptsList {
    const METHOD_NAME: &'static str = "prompts/list";
    type Params = PromptsListParams;
    type Result = PromptsListResult;
}

impl PaginatedMethod for PromptsList {
    fn next_cursor(result: &Self::Result) -> Option<&str> {
        result.next_cursor.as_deref()
    }
}

impl PromptsList {
    /// Create a prompts/list request
    pub fn params() -> PromptsListParams {
        PromptsListParams::default()
    }

    /// Create a prompts/list request with pagination cursor
    pub fn params_with_cursor(cursor: impl Into<String>) -> PromptsListParams {
        PromptsListParams {
            cursor: Some(cursor.into()),
        }
    }
}

/// Prompts Get method - Retrieve a specific prompt
pub struct PromptsGet;

impl McpMethod for PromptsGet {
    const METHOD_NAME: &'static str = "prompts/get";
    type Params = PromptsGetParams;
    type Result = PromptsGetResult;
}

impl PromptsGet {
    /// Create a prompts/get request
    pub fn params(name: impl Into<String>) -> PromptsGetParams {
        PromptsGetParams::new(name)
    }

    /// Create a prompts/get request with arguments
    pub fn params_with_args(name: impl Into<String>, arguments: Value) -> PromptsGetParams {
        let mut params = PromptsGetParams::new(name);
        params.arguments = Some(arguments);
        params
    }
}

// ============================================================================
// Roots Methods
// ============================================================================

/// Roots List method - List filesystem roots
pub struct RootsList;

impl McpMethod for RootsList {
    const METHOD_NAME: &'static str = "roots/list";
    type Params = RootsListParams;
    type Result = RootsListResult;
}

// ============================================================================
// Logging Methods
// ============================================================================

/// Logging Set Level method - Set logging verbosity
pub struct LoggingSetLevel;

impl McpMethod for LoggingSetLevel {
    const METHOD_NAME: &'static str = "logging/setLevel";
    type Params = LoggingSetLevelParams;
    type Result = LoggingSetLevelResult;
}

/// Logging set level response (empty)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoggingSetLevelResult {}

impl LoggingSetLevel {
    /// Create a logging/setLevel request
    pub fn params(level: LogLevel) -> LoggingSetLevelParams {
        LoggingSetLevelParams { level }
    }
}

// ============================================================================
// Completion Method
// ============================================================================

/// Completion Complete method - Get argument completions
pub struct CompletionComplete;

/// Completion reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionReference {
    /// Reference type (e.g., "ref/prompt", "ref/resource")
    #[serde(rename = "type")]
    pub ref_type: String,
    /// Resource or prompt URI
    pub uri: Option<String>,
    /// Prompt name (for prompt references)
    pub name: Option<String>,
}

/// Completion complete request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionCompleteParams {
    /// Reference to complete
    pub r#ref: CompletionReference,
    /// Argument being completed
    pub argument: CompletionArgument,
}

/// Argument completion context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionArgument {
    /// Argument name
    pub name: String,
    /// Current value
    pub value: String,
}

/// Completion complete response result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionCompleteResult {
    /// Completion values
    pub completion: CompletionInfo,
}

/// Completion information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionInfo {
    /// Suggested values
    pub values: Vec<String>,
    /// Total number of available completions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
    /// Indicates more completions are available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

impl McpMethod for CompletionComplete {
    const METHOD_NAME: &'static str = "completion/complete";
    type Params = CompletionCompleteParams;
    type Result = CompletionCompleteResult;
}

// ============================================================================
// Sampling Method
// ============================================================================

/// Sampling Create Message method - Request LLM sampling
pub struct SamplingCreateMessage;

/// Sampling message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingMessage {
    /// Message role
    pub role: String,
    /// Message content
    pub content: SamplingContent,
}

/// Sampling content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SamplingContent {
    /// Text content
    #[serde(rename = "text")]
    Text { text: String },
    /// Image content
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
}

/// Sampling create message request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingCreateMessageParams {
    /// Messages to include
    pub messages: Vec<SamplingMessage>,
    /// Model preferences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,
    /// System prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Include context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_context: Option<String>,
    /// Temperature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Max tokens
    pub max_tokens: u32,
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Model preferences for sampling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreferences {
    /// Hints for model selection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<ModelHint>>,
    /// Cost priority (0-1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_priority: Option<f64>,
    /// Speed priority (0-1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_priority: Option<f64>,
    /// Intelligence priority (0-1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_priority: Option<f64>,
}

/// Model hint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHint {
    /// Preferred model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Sampling create message response result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingCreateMessageResult {
    /// Model used
    pub model: String,
    /// Stop reason
    #[serde(rename = "stopReason")]
    pub stop_reason: String,
    /// Generated content
    pub content: SamplingContent,
}

impl McpMethod for SamplingCreateMessage {
    const METHOD_NAME: &'static str = "sampling/createMessage";
    type Params = SamplingCreateMessageParams;
    type Result = SamplingCreateMessageResult;
}

// ============================================================================
// Notifications
// ============================================================================

/// Initialized notification - Sent after initialization completes
pub struct NotificationsInitialized;

impl McpNotification for NotificationsInitialized {
    const METHOD_NAME: &'static str = "notifications/initialized";
    type Params = ();
}

/// Tools list changed notification - Tools list has changed
pub struct NotificationsToolsListChanged;

impl McpNotification for NotificationsToolsListChanged {
    const METHOD_NAME: &'static str = "notifications/tools/list_changed";
    type Params = ();
}

/// Resources list changed notification - Resources list has changed
pub struct NotificationsResourcesListChanged;

impl McpNotification for NotificationsResourcesListChanged {
    const METHOD_NAME: &'static str = "notifications/resources/list_changed";
    type Params = ();
}

/// Prompts list changed notification - Prompts list has changed
pub struct NotificationsPromptsListChanged;

impl McpNotification for NotificationsPromptsListChanged {
    const METHOD_NAME: &'static str = "notifications/prompts/list_changed";
    type Params = ();
}

/// Roots list changed notification - Roots list has changed
pub struct NotificationsRootsListChanged;

impl McpNotification for NotificationsRootsListChanged {
    const METHOD_NAME: &'static str = "notifications/roots/list_changed";
    type Params = ();
}

/// Progress notification - Reports progress on long-running operations
pub struct NotificationsProgress;

/// Progress notification parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressParams {
    /// Progress token matching the request
    pub progress_token: Value,
    /// Current progress value
    pub progress: f64,
    /// Total expected progress (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}

impl McpNotification for NotificationsProgress {
    const METHOD_NAME: &'static str = "notifications/progress";
    type Params = ProgressParams;
}

/// Logging message notification - Server log message
pub struct NotificationsMessage;

/// Logging message notification parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMessageParams {
    /// Log level
    pub level: LogLevel,
    /// Logger name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
    /// Log message
    pub data: String,
}

impl McpNotification for NotificationsMessage {
    const METHOD_NAME: &'static str = "notifications/message";
    type Params = LogMessageParams;
}

// ============================================================================
// Method Registry
// ============================================================================

/// Enum representing all known MCP methods for dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum McpMethodKind {
    // Lifecycle
    Initialize,
    Ping,

    // Tools
    ToolsList,
    ToolsCall,

    // Resources
    ResourcesList,
    ResourcesRead,
    ResourcesSubscribe,
    ResourcesUnsubscribe,

    // Prompts
    PromptsList,
    PromptsGet,

    // Roots
    RootsList,

    // Logging
    LoggingSetLevel,

    // Completion
    CompletionComplete,

    // Sampling
    SamplingCreateMessage,
}

impl McpMethodKind {
    /// Get the method name string for this kind
    pub const fn method_name(&self) -> &'static str {
        match self {
            McpMethodKind::Initialize => Initialize::METHOD_NAME,
            McpMethodKind::Ping => Ping::METHOD_NAME,
            McpMethodKind::ToolsList => ToolsList::METHOD_NAME,
            McpMethodKind::ToolsCall => ToolsCall::METHOD_NAME,
            McpMethodKind::ResourcesList => ResourcesList::METHOD_NAME,
            McpMethodKind::ResourcesRead => ResourcesRead::METHOD_NAME,
            McpMethodKind::ResourcesSubscribe => ResourcesSubscribe::METHOD_NAME,
            McpMethodKind::ResourcesUnsubscribe => ResourcesUnsubscribe::METHOD_NAME,
            McpMethodKind::PromptsList => PromptsList::METHOD_NAME,
            McpMethodKind::PromptsGet => PromptsGet::METHOD_NAME,
            McpMethodKind::RootsList => RootsList::METHOD_NAME,
            McpMethodKind::LoggingSetLevel => LoggingSetLevel::METHOD_NAME,
            McpMethodKind::CompletionComplete => CompletionComplete::METHOD_NAME,
            McpMethodKind::SamplingCreateMessage => SamplingCreateMessage::METHOD_NAME,
        }
    }

    /// Parse method name string to kind
    pub fn from_method_name(name: &str) -> Option<Self> {
        match name {
            Initialize::METHOD_NAME => Some(McpMethodKind::Initialize),
            Ping::METHOD_NAME => Some(McpMethodKind::Ping),
            ToolsList::METHOD_NAME => Some(McpMethodKind::ToolsList),
            ToolsCall::METHOD_NAME => Some(McpMethodKind::ToolsCall),
            ResourcesList::METHOD_NAME => Some(McpMethodKind::ResourcesList),
            ResourcesRead::METHOD_NAME => Some(McpMethodKind::ResourcesRead),
            ResourcesSubscribe::METHOD_NAME => Some(McpMethodKind::ResourcesSubscribe),
            ResourcesUnsubscribe::METHOD_NAME => Some(McpMethodKind::ResourcesUnsubscribe),
            PromptsList::METHOD_NAME => Some(McpMethodKind::PromptsList),
            PromptsGet::METHOD_NAME => Some(McpMethodKind::PromptsGet),
            RootsList::METHOD_NAME => Some(McpMethodKind::RootsList),
            LoggingSetLevel::METHOD_NAME => Some(McpMethodKind::LoggingSetLevel),
            CompletionComplete::METHOD_NAME => Some(McpMethodKind::CompletionComplete),
            SamplingCreateMessage::METHOD_NAME => Some(McpMethodKind::SamplingCreateMessage),
            _ => None,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_initialize_wrapper() {
        let params = Initialize::params("test-client", "1.0.0");
        let request = Initialize::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "initialize");
        assert!(request.params.is_some());
    }

    #[test]
    fn test_tools_list_wrapper() {
        let params = ToolsList::params();
        let request = ToolsList::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "tools/list");
    }

    #[test]
    fn test_tools_list_with_cursor() {
        let params = ToolsList::params_with_cursor("next-page-token");
        let request = ToolsList::build_request(RequestId::Number(1), params);

        let params_value = request.params.unwrap();
        assert_eq!(params_value["cursor"], "next-page-token");
    }

    #[test]
    fn test_tools_call_wrapper() {
        let params = ToolsCall::params_with_args(
            "browser_navigate",
            json!({"url": "https://example.com"}),
        );
        let request = ToolsCall::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "tools/call");
        let params_value = request.params.unwrap();
        assert_eq!(params_value["name"], "browser_navigate");
        assert_eq!(params_value["arguments"]["url"], "https://example.com");
    }

    #[test]
    fn test_resources_read_wrapper() {
        let params = ResourcesRead::params("file:///project/package.json");
        let request = ResourcesRead::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "resources/read");
        let params_value = request.params.unwrap();
        assert_eq!(params_value["uri"], "file:///project/package.json");
    }

    #[test]
    fn test_prompts_get_wrapper() {
        let params = PromptsGet::params_with_args(
            "code_review",
            json!({"language": "rust"}),
        );
        let request = PromptsGet::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "prompts/get");
        let params_value = request.params.unwrap();
        assert_eq!(params_value["name"], "code_review");
        assert_eq!(params_value["arguments"]["language"], "rust");
    }

    #[test]
    fn test_parse_initialize_response() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2025-11-25",
                "capabilities": {
                    "tools": { "listChanged": true }
                },
                "serverInfo": {
                    "name": "test-server",
                    "version": "1.0.0"
                }
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = Initialize::parse_response(response).unwrap();

        assert_eq!(result.protocol_version, "2025-11-25");
        assert_eq!(result.server_info.name, "test-server");
    }

    #[test]
    fn test_parse_tools_list_response() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [
                    {
                        "name": "browser_navigate",
                        "description": "Navigate to URL",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "url": { "type": "string" }
                            }
                        }
                    }
                ]
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ToolsList::parse_response(response).unwrap();

        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "browser_navigate");
    }

    #[test]
    fn test_parse_tool_call_response() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": "Successfully navigated"
                    }
                ],
                "isError": false
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = ToolsCall::parse_response(response).unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn test_parse_error_response() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32601,
                "message": "Method not found"
            }
        });

        let response: JsonRpcResponse = serde_json::from_value(response_json).unwrap();
        let result = Initialize::parse_response(response);

        assert!(result.is_err());
    }

    #[test]
    fn test_method_kind_from_name() {
        assert_eq!(
            McpMethodKind::from_method_name("initialize"),
            Some(McpMethodKind::Initialize)
        );
        assert_eq!(
            McpMethodKind::from_method_name("tools/list"),
            Some(McpMethodKind::ToolsList)
        );
        assert_eq!(
            McpMethodKind::from_method_name("tools/call"),
            Some(McpMethodKind::ToolsCall)
        );
        assert_eq!(
            McpMethodKind::from_method_name("unknown"),
            None
        );
    }

    #[test]
    fn test_method_kind_method_name() {
        assert_eq!(McpMethodKind::Initialize.method_name(), "initialize");
        assert_eq!(McpMethodKind::ToolsList.method_name(), "tools/list");
        assert_eq!(McpMethodKind::ToolsCall.method_name(), "tools/call");
    }

    #[test]
    fn test_notification_initialized() {
        let notification = NotificationsInitialized::build_notification_empty();
        assert_eq!(notification.method, "notifications/initialized");
    }

    #[test]
    fn test_notification_progress() {
        let params = ProgressParams {
            progress_token: json!(1),
            progress: 50.0,
            total: Some(100.0),
        };
        let notification = NotificationsProgress::build_notification(params);
        assert_eq!(notification.method, "notifications/progress");
    }

    #[test]
    fn test_logging_set_level() {
        let params = LoggingSetLevel::params(LogLevel::Debug);
        let request = LoggingSetLevel::build_request(RequestId::Number(1), params);

        assert_eq!(request.method, "logging/setLevel");
        let params_value = request.params.unwrap();
        assert_eq!(params_value["level"], "debug");
    }

    #[test]
    fn test_pagination_cursor_extraction() {
        let result = ToolsListResult {
            tools: vec![],
            next_cursor: Some("next-token".to_string()),
        };

        assert_eq!(ToolsList::next_cursor(&result), Some("next-token"));

        let result_no_cursor = ToolsListResult {
            tools: vec![],
            next_cursor: None,
        };

        assert_eq!(ToolsList::next_cursor(&result_no_cursor), None);
    }
}
