# MCP Protocol Implementation Requirements for ltmatrix

**Date**: 2025-03-07
**Based on**: MCP Specification 2025-11-25 / 2025-06-18
**Purpose**: Define protocol structures for ltmatrix MCP integration

## Scope

This document defines the **minimum viable protocol implementation** for ltmatrix to communicate with MCP servers (e.g., Playwright, browser automation tools) for end-to-end testing purposes.

## Implementation Approach

**Choice**: **Focused e2e testing subset** with **extensible framework**

### Rationale

- ltmatrix needs MCP primarily for E2E testing (Playwright, browser automation)
- Full MCP client implementation is overkill for current requirements
- Core protocol structures allow future extensibility
- Implement critical methods initially, extend as needed

## Required Protocol Components

### 1. Core Message Types

Must implement all three JSON-RPC 2.0 message types:

#### Request Message
```rust
pub struct JsonRpcRequest {
    pub jsonrpc: String,              // Always "2.0"
    pub id: RequestId,                // String or Number
    pub method: String,               // Method name
    pub params: Option<Value>,        // Method parameters
}

pub enum RequestId {
    String(String),
    Number(i64),
    Null,
}
```

#### Response Message
```rust
pub struct JsonRpcResponse {
    pub jsonrpc: String,              // Always "2.0"
    pub id: RequestId,                // Must match request
    pub result: Option<Value>,        // Success result
    pub error: Option<JsonRpcError>,  // Error details
}

pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}
```

#### Notification Message
```rust
pub struct JsonRpcNotification {
    pub jsonrpc: String,              // Always "2.0"
    pub method: String,               // Method name
    pub params: Option<Value>,        // Method parameters
    // Note: No `id` field - notifications don't expect responses
}
```

### 2. Error Handling

#### Standard JSON-RPC 2.0 Error Codes
```rust
pub enum JsonRpcErrorCode {
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    ServerError(i32),                // -32000 to -32099
}

impl JsonRpcErrorCode {
    pub fn from_i32(code: i32) -> Self;
    pub fn as_i32(&self) -> i32;
}
```

#### MCP-Specific Error Codes
```rust
// Reserved range: -32000 to -32099
pub const MCP_ERROR_UNKNOWN_TOOL: i32 = -32001;
pub const MCP_ERROR_MALFORMED_REQUEST: i32 = -32002;
pub const MCP_ERROR_TOOL_EXECUTION: i32 = -32003;
pub const MCP_ERROR_RESOURCE_NOT_FOUND: i32 = -32004;
pub const MCP_ERROR_PERMISSION_DENIED: i32 = -32005;
```

### 3. Protocol Methods (Priority Implementation)

#### Phase 1: Essential Methods (MVP)

**`initialize`** - Required handshake
```rust
pub struct InitializeRequest {
    pub protocol_version: String,     // "2025-11-25"
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
}

pub struct ClientCapabilities {
    pub tools: Option<ToolCapabilities>,
    pub resources: Option<ResourceCapabilities>,
    pub prompts: Option<PromptCapabilities>,
}

pub struct ClientInfo {
    pub name: String,                 // "ltmatrix"
    pub version: String,              // "0.1.0"
}

pub struct InitializeResponse {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}
```

**`tools/list`** - Discover available tools
```rust
pub struct ToolsListRequest {
    pub cursor: Option<String>,       // For pagination
}

pub struct ToolsListResponse {
    pub tools: Vec<Tool>,
    pub next_cursor: Option<String>,  // More tools available
}

pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,          // JSON Schema
}
```

**`tools/call`** - Execute a tool
```rust
pub struct ToolCallRequest {
    pub name: String,
    pub arguments: Value,             // Tool input
}

pub struct ToolCallResponse {
    pub content: Vec<ToolContent>,
    pub is_error: bool,
}

pub enum ToolContent {
    Text { text: String },
    Image {
        data: String,                // base64
        mime_type: String,
    },
    Resource {
        uri: String,
        mime_type: Option<String>,
    },
}
```

#### Phase 2: Resource Methods (Future Enhancement)

**`resources/list`** - Discover available resources
```rust
pub struct ResourcesListResponse {
    pub resources: Vec<Resource>,
}

pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}
```

**`resources/read`** - Read resource contents
```rust
pub struct ResourceReadRequest {
    pub uri: String,
}

pub struct ResourceReadResponse {
    pub contents: Vec<ResourceContent>,
}

pub struct ResourceContent {
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub blob: Option<Vec<u8>>,
}
```

#### Phase 3: Prompt Methods (Optional Enhancement)

**`prompts/list`** - Discover prompt templates
**`prompts/get`** - Retrieve specific prompt

### 4. Transport Layer

#### stdio Transport (Primary Implementation)

```rust
pub struct StdioTransport {
    // Reading from stdin, writing to stdout
    // Messages delimited by newlines
}

impl StdioTransport {
    pub fn new() -> Self;
    pub fn send(&mut self, message: &str) -> Result<()>;
    pub fn receive(&mut self) -> Result<String>;
    pub fn send_message<T>(&mut self, msg: &T) -> Result<()>
    where
        T: Serialize;
    pub fn receive_message(&mut self) -> Result<JsonRpcMessage>;
}
```

**Message Format**:
- Each message is a complete JSON object on one line
- Messages delimited by `\n`
- No embedded newlines in JSON strings
- UTF-8 encoding

#### Message Envelope

```rust
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}

impl JsonRpcMessage {
    pub fn from_json(json: &str) -> Result<Self>;
    pub fn to_json(&self) -> Result<String>;
}
```

### 5. Client Implementation

#### MCP Client Structure

```rust
pub struct McpClient {
    transport: StdioTransport,
    server_capabilities: Option<ServerCapabilities>,
    request_id: i64,
}

impl McpClient {
    pub fn new(command: &str, args: &[String]) -> Result<Self>;
    pub fn initialize(&mut self) -> Result<InitializeResponse>;
    pub fn tools_list(&mut self) -> Result<Vec<Tool>>;
    pub fn tools_call(&mut self, name: &str, args: Value) -> Result<ToolCallResponse>;
    pub fn resources_list(&mut self) -> Result<Vec<Resource>>;
    pub fn resources_read(&mut self, uri: &str) -> Result<Vec<ResourceContent>>;
}
```

#### Request/Response Handling

```rust
impl McpClient {
    fn send_request(&mut self, method: &str, params: Option<Value>)
        -> Result<Value>;

    fn wait_for_response(&mut self, id: RequestId)
        -> Result<JsonRpcResponse>;

    fn handle_notification(&mut self, notification: JsonRpcNotification)
        -> Result<()>;
}
```

### 6. Server Process Management

#### Server Spawning

```rust
pub struct McpServer {
    name: String,
    config: McpServerConfig,          // From config/mcp.rs
    child: Option<Child>,             // tokio::process::Child
}

impl McpServer {
    pub fn from_config(name: String, config: McpServerConfig) -> Self;
    pub fn spawn(&mut self) -> Result<()>;
    pub fn is_running(&self) -> bool;
    pub fn terminate(&mut self) -> Result<()>;
}
```

### 7. Type-Safe Method Wrappers

#### Method Registry Pattern

```rust
pub trait McpMethod {
    const METHOD_NAME: &'static str;
    type Request: DeserializeOwned;
    type Response: Serialize;

    fn execute(client: &mut McpClient, request: Self::Request)
        -> Result<Self::Response>;
}

// Example: tools/call wrapper
pub struct ToolsCallMethod;

impl McpMethod for ToolsCallMethod {
    const METHOD_NAME: &'static str = "tools/call";
    type Request = ToolCallRequest;
    type Response = ToolCallResponse;

    fn execute(client: &mut McpClient, request: Self::Request)
        -> Result<Self::Response>
    {
        let params = to_value(&request)?;
        let result = client.send_request(Self::METHOD_NAME, Some(params))?;
        let response: Self::Response = from_value(result)?;
        Ok(response)
    }
}
```

#### Convenience Methods

```rust
impl McpClient {
    pub fn call_tool(&mut self, name: &str, args: Value)
        -> Result<ToolCallResponse>
    {
        ToolsCallMethod::execute(self, ToolCallRequest {
            name: name.to_string(),
            arguments: args,
        })
    }
}
```

## Data Validation

### JSON Schema Validation

Use `jsonschema` crate for tool parameter validation:

```rust
use jsonschema::{JSONSchema, ValidationError};

pub fn validate_tool_input(schema: &Value, input: &Value)
    -> Result<()>
{
    let compiled_schema = JSONSchema::compile(schema)?;
    if let Some(error) = compiled_schema.validate(input).next() {
        Err(anyhow::anyhow!("Invalid input: {}", error))
    } else {
        Ok(())
    }
}
```

## Testing Strategy

### Unit Tests

- Message serialization/deserialization
- Error code mapping
- JSON Schema validation
- Type-safe wrapper execution

### Integration Tests

- stdio transport send/receive
- Initialize handshake with mock server
- Tool discovery and execution
- Error handling scenarios

### E2E Tests

- Spawn actual MCP server (Playwright)
- Execute real tool calls
- Verify response handling
- Test timeout handling

## Configuration Integration

### Existing Config Integration

The MCP configuration is already implemented in `src/config/mcp.rs`:

```rust
// Already exists - no changes needed
pub struct McpServer {
    pub server_type: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
    pub timeout: u64,
    pub enabled: bool,
}
```

### New Client Creation

```rust
// In src/mcp/client.rs
use crate::config::mcp::McpServer;

impl McpClient {
    pub fn from_server_config(config: &McpServer) -> Result<Self> {
        let command = config.command.as_ref()
            .ok_or_else(|| anyhow!("Server command required"))?;
        Self::new(command, &config.args)
    }
}
```

## Dependencies

### Required Crates

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.40", features = ["process", "io-util"] }
anyhow = "1.0"
jsonschema = { version = "0.18", default-features = false, features = ["resolve-http"] }
```

### Optional (Future Enhancement)

```toml
uuid = { version = "1.0", features = ["v4", "serde"] }  # For request IDs
async-trait = "0.1"                                    # For transport trait
```

## Module Structure

```
src/mcp/
├── mod.rs              # Module exports
├── protocol/           # Protocol definitions
│   ├── mod.rs
│   ├── messages.rs     # Request/Response/Notification
│   ├── methods.rs      # Method-specific types
│   └── errors.rs       # Error codes and types
├── transport/          # Transport layer
│   ├── mod.rs
│   ├── stdio.rs        # stdio transport
│   └── base.rs         # Transport trait
└── client.rs           # MCP client implementation
```

## Implementation Phases

### Phase 1: Foundation (Current Task)
1. Protocol message types
2. Error handling
3. Basic serialization/deserialization
4. stdio transport implementation

### Phase 2: Core Methods (Next)
1. Initialize handshake
2. tools/list and tools/call
3. Basic client implementation
4. Integration tests

### Phase 3: Server Integration (Future)
1. Server process spawning
2. Timeout handling
3. Resource access methods
4. E2E testing with Playwright

## Success Criteria

- [ ] All message types serialize/deserialize correctly
- [ ] Initialize handshake completes successfully
- [ ] tools/list discovers available tools
- [ ] tools/call executes tools with arguments
- [ ] Errors are properly decoded and returned
- [ ] stdio transport handles newline delimiting
- [ ] Unit tests pass (100% coverage of protocol types)
- [ ] Integration tests pass with mock server
- [ ] Compatible with existing MCP config system

## References

- [MCP Protocol Research](./MCP_PROTOCOL_RESEARCH.md) - Detailed protocol research
- [MCP Config Implementation](../../src/config/mcp.rs) - Existing config system
- [MCP Specification](https://modelcontextprotocol.io/specification/2025-11-25)
