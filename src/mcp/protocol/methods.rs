// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! MCP-specific method types and payloads
//!
//! This module implements the MCP-specific request and response types
//! for protocol methods like initialize, tools, resources, and prompts.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current MCP protocol version
pub const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

// ============================================================================
// Client/Server Info
// ============================================================================

/// Client or server implementation information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImplementationInfo {
    /// Name of the implementation
    pub name: String,
    /// Version of the implementation
    pub version: String,
}

impl ImplementationInfo {
    /// Create new implementation info
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

// ============================================================================
// Capabilities
// ============================================================================

/// Client capabilities declared during initialization
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ClientCapabilities {
    /// Experimental features (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,

    /// Roots capability (filesystem roots)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,

    /// Sampling capability (LLM sampling requests)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<Value>,
}

/// Roots capability configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RootsCapability {
    /// Client supports roots list change notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

impl RootsCapability {
    /// Create roots capability with list_changed support
    pub fn with_list_changed(list_changed: bool) -> Self {
        Self {
            list_changed: Some(list_changed),
        }
    }
}

/// Server capabilities declared during initialization
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ServerCapabilities {
    /// Experimental features (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,

    /// Logging capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<Value>,

    /// Completions capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions: Option<Value>,

    /// Prompts capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,

    /// Resources capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,

    /// Tools capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}

/// Prompts capability configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PromptsCapability {
    /// Server supports prompts list change notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Resources capability configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesCapability {
    /// Server supports resource subscriptions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,

    /// Server supports resources list change notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Tools capability configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    /// Server supports tools list change notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

// ============================================================================
// Initialize Method
// ============================================================================

/// Initialize request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// Protocol version the client supports
    pub protocol_version: String,

    /// Client capabilities
    pub capabilities: ClientCapabilities,

    /// Client implementation info
    pub client_info: ImplementationInfo,
}

impl InitializeParams {
    /// Create new initialize params
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: ImplementationInfo::new(name, version),
        }
    }

    /// Add capabilities to the initialize params
    pub fn with_capabilities(mut self, capabilities: ClientCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }
}

/// Initialize response result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// Protocol version the server supports
    pub protocol_version: String,

    /// Server capabilities
    pub capabilities: ServerCapabilities,

    /// Server implementation info
    pub server_info: ImplementationInfo,

    /// Instructions for the client (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

// ============================================================================
// Tools
// ============================================================================

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// Tool name
    pub name: String,

    /// Tool description
    pub description: String,

    /// JSON Schema for tool input
    pub input_schema: Value,
}

impl Tool {
    /// Create a new tool definition
    pub fn new(name: impl Into<String>, description: impl Into<String>, input_schema: Value) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

/// Tools list request parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsListParams {
    /// Pagination cursor (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Tools list response result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsListResult {
    /// Available tools
    pub tools: Vec<Tool>,

    /// Pagination cursor for more results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Tool call request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParams {
    /// Name of the tool to call
    pub name: String,

    /// Tool arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

impl ToolCallParams {
    /// Create new tool call params
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: None,
        }
    }

    /// Add arguments to the tool call
    pub fn with_arguments(mut self, arguments: Value) -> Self {
        self.arguments = Some(arguments);
        self
    }
}

/// Tool content item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ToolContent {
    /// Text content
    #[serde(rename = "text")]
    Text {
        /// The text content
        text: String,
    },

    /// Image content
    #[serde(rename = "image")]
    Image {
        /// Base64-encoded image data
        data: String,

        /// MIME type of the image
        mime_type: String,
    },

    /// Resource reference
    #[serde(rename = "resource")]
    Resource {
        /// Resource URI
        uri: String,

        /// MIME type (optional)
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
}

impl ToolContent {
    /// Create text content
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create image content
    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }

    /// Create resource reference
    pub fn resource(uri: impl Into<String>, mime_type: Option<String>) -> Self {
        Self::Resource {
            uri: uri.into(),
            mime_type,
        }
    }
}

/// Tool call response result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    /// Tool output content
    pub content: Vec<ToolContent>,

    /// Whether the tool execution resulted in an error
    #[serde(default)]
    pub is_error: bool,
}

impl ToolCallResult {
    /// Create a successful tool result with text
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(text)],
            is_error: false,
        }
    }

    /// Create an error tool result
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(text)],
            is_error: true,
        }
    }
}

// ============================================================================
// Resources
// ============================================================================

/// Resource definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Resource {
    /// Resource URI
    pub uri: String,

    /// Human-readable name
    pub name: String,

    /// Resource description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

impl Resource {
    /// Create a new resource
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            description: None,
            mime_type: None,
        }
    }
}

/// Resources list request parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourcesListParams {
    /// Pagination cursor (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Resources list response result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesListResult {
    /// Available resources
    pub resources: Vec<Resource>,

    /// Pagination cursor for more results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Resource read request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReadParams {
    /// Resource URI to read
    pub uri: String,
}

impl ResourceReadParams {
    /// Create new resource read params
    pub fn new(uri: impl Into<String>) -> Self {
        Self { uri: uri.into() }
    }
}

/// Resource contents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceContents {
    /// Resource URI
    pub uri: String,

    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Text contents (for text resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Binary contents as base64 (for binary resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

impl ResourceContents {
    /// Create text resource contents
    pub fn text(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type: Some("text/plain".to_string()),
            text: Some(text.into()),
            blob: None,
        }
    }

    /// Create binary resource contents
    pub fn blob(uri: impl Into<String>, blob: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type: Some(mime_type.into()),
            text: None,
            blob: Some(blob.into()),
        }
    }
}

/// Resource read response result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReadResult {
    /// Resource contents
    pub contents: Vec<ResourceContents>,
}

// ============================================================================
// Prompts
// ============================================================================

/// Prompt argument definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptArgument {
    /// Argument name
    pub name: String,

    /// Argument description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether the argument is required
    #[serde(default)]
    pub required: bool,
}

impl PromptArgument {
    /// Create a new prompt argument
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            required: false,
        }
    }

    /// Mark as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

/// Prompt definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Prompt {
    /// Prompt name
    pub name: String,

    /// Prompt description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Prompt arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

impl Prompt {
    /// Create a new prompt
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            arguments: None,
        }
    }
}

/// Prompts list request parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromptsListParams {
    /// Pagination cursor (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Prompts list response result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptsListResult {
    /// Available prompts
    pub prompts: Vec<Prompt>,

    /// Pagination cursor for more results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Prompts get request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsGetParams {
    /// Prompt name
    pub name: String,

    /// Prompt arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

impl PromptsGetParams {
    /// Create new prompts get params
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: None,
        }
    }
}

/// Prompt message content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum PromptContent {
    /// Text content
    #[serde(rename = "text")]
    Text {
        /// The text content
        text: String,
    },

    /// Image content
    #[serde(rename = "image")]
    Image {
        /// Image data (URL or base64)
        data: String,

        /// MIME type
        mime_type: String,
    },

    /// Resource content
    #[serde(rename = "resource")]
    Resource {
        /// Resource reference
        resource: Resource,
    },
}

/// Prompt message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptMessage {
    /// Message role
    pub role: String,

    /// Message content
    pub content: PromptContent,
}

impl PromptMessage {
    /// Create a user message
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: PromptContent::Text { text: text.into() },
        }
    }

    /// Create an assistant message
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: PromptContent::Text { text: text.into() },
        }
    }
}

/// Prompts get response result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsGetResult {
    /// Prompt description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Prompt messages
    pub messages: Vec<PromptMessage>,
}

// ============================================================================
// Roots
// ============================================================================

/// Root definition (filesystem root)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Root {
    /// Root URI
    pub uri: String,

    /// Human-readable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Root {
    /// Create a new root
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: None,
        }
    }
}

/// Roots list request parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RootsListParams {}

/// Roots list response result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsListResult {
    /// Available roots
    pub roots: Vec<Root>,
}

// ============================================================================
// Logging
// ============================================================================

/// Log level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Debug and above
    Debug,
    /// Info and above
    Info,
    /// Warning and above
    Warning,
    /// Error only
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

/// Logging set level request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSetLevelParams {
    /// Log level to set
    pub level: LogLevel,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_implementation_info() {
        let info = ImplementationInfo::new("ltmatrix", "0.1.0");
        assert_eq!(info.name, "ltmatrix");
        assert_eq!(info.version, "0.1.0");

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"name\":\"ltmatrix\""));
    }

    #[test]
    fn test_client_capabilities() {
        let caps = ClientCapabilities {
            roots: Some(RootsCapability::with_list_changed(true)),
            sampling: Some(json!({})),
            ..Default::default()
        };

        let json = serde_json::to_string(&caps).unwrap();
        assert!(json.contains("\"roots\""));
        assert!(json.contains("\"sampling\""));
    }

    #[test]
    fn test_server_capabilities() {
        let caps = ServerCapabilities {
            tools: Some(ToolsCapability {
                list_changed: Some(true),
            }),
            resources: Some(ResourcesCapability {
                subscribe: Some(true),
                list_changed: Some(true),
            }),
            ..Default::default()
        };

        let json = serde_json::to_string(&caps).unwrap();
        assert!(json.contains("\"tools\""));
        assert!(json.contains("\"resources\""));
    }

    #[test]
    fn test_initialize_params() {
        let params = InitializeParams::new("ltmatrix", "0.1.0")
            .with_capabilities(ClientCapabilities {
                roots: Some(RootsCapability::with_list_changed(true)),
                sampling: Some(json!({})),
                ..Default::default()
            });

        assert_eq!(params.protocol_version, MCP_PROTOCOL_VERSION);
        assert_eq!(params.client_info.name, "ltmatrix");

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("\"protocolVersion\""));
        assert!(json.contains("\"clientInfo\""));
    }

    #[test]
    fn test_tool_definition() {
        let tool = Tool::new(
            "browser_navigate",
            "Navigate to a URL",
            json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string"}
                },
                "required": ["url"]
            }),
        );

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("\"name\":\"browser_navigate\""));
        assert!(json.contains("\"inputSchema\""));
    }

    #[test]
    fn test_tool_content() {
        let text = ToolContent::text("Hello, world!");
        let json = serde_json::to_string(&text).unwrap();
        assert!(json.contains("\"type\":\"text\""));

        let image = ToolContent::image("base64data", "image/png");
        let json = serde_json::to_string(&image).unwrap();
        assert!(json.contains("\"type\":\"image\""));
    }

    #[test]
    fn test_tool_call_params() {
        let params = ToolCallParams::new("browser_navigate")
            .with_arguments(json!({"url": "https://example.com"}));

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("\"name\":\"browser_navigate\""));
        assert!(json.contains("\"arguments\""));
    }

    #[test]
    fn test_tool_call_result() {
        let result = ToolCallResult::text("Success");
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);

        let error = ToolCallResult::error("Failed");
        assert!(error.is_error);
    }

    #[test]
    fn test_resource() {
        let resource = Resource::new("file:///project/package.json", "package.json");
        let json = serde_json::to_string(&resource).unwrap();
        assert!(json.contains("\"uri\":\"file:///project/package.json\""));
    }

    #[test]
    fn test_resource_contents() {
        let contents = ResourceContents::text("file:///test.txt", "Hello");
        let json = serde_json::to_string(&contents).unwrap();
        assert!(json.contains("\"text\":\"Hello\""));

        let binary = ResourceContents::blob("file:///binary.bin", "base64data", "application/octet-stream");
        let json = serde_json::to_string(&binary).unwrap();
        assert!(json.contains("\"blob\":\"base64data\""));
    }

    #[test]
    fn test_prompt_message() {
        let msg = PromptMessage::user("Hello");
        assert_eq!(msg.role, "user");

        let msg = PromptMessage::assistant("Hi there");
        assert_eq!(msg.role, "assistant");
    }

    #[test]
    fn test_log_level() {
        let level = LogLevel::Debug;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"debug\"");

        let parsed: LogLevel = serde_json::from_str("\"info\"").unwrap();
        assert_eq!(parsed, LogLevel::Info);
    }

    #[test]
    fn test_initialize_result_deserialization() {
        let json = json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {
                "tools": {
                    "listChanged": true
                }
            },
            "serverInfo": {
                "name": "playwright-mcp",
                "version": "1.0.0"
            }
        });

        let result: InitializeResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.protocol_version, "2025-11-25");
        assert_eq!(result.server_info.name, "playwright-mcp");
        assert!(result.capabilities.tools.is_some());
    }

    #[test]
    fn test_tools_list_result_deserialization() {
        let json = json!({
            "tools": [
                {
                    "name": "browser_navigate",
                    "description": "Navigate to a URL",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "url": {"type": "string"}
                        }
                    }
                }
            ]
        });

        let result: ToolsListResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "browser_navigate");
    }

    #[test]
    fn test_tool_call_result_deserialization() {
        let json = json!({
            "content": [
                {
                    "type": "text",
                    "text": "Navigated successfully"
                }
            ],
            "isError": false
        });

        let result: ToolCallResult = serde_json::from_value(json).unwrap();
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
    }
}
