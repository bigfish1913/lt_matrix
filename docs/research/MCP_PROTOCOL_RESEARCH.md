# MCP Protocol Specification Research

**Date**: 2025-03-07
**Version**: 2025-11-25 / 2025-06-18
**Status**: Research Complete

## Overview

The **Model Context Protocol (MCP)** is an open protocol developed by Anthropic that enables seamless integration between LLM applications and external data sources and tools. It uses **JSON-RPC 2.0** as its communication layer.

### Official Resources

- **Specification**: [modelcontextprotocol.io](https://modelcontextprotocol.io/specification/2025-11-25)
- **GitHub**: [modelcontextprotocol/modelcontextprotocol](https://github.com/modelcontextprotocol/modelcontextprotocol)
- **Introduction**: [Anthropic MCP Announcement](https://www.anthropic.com/news/model-context-protocol)

## Protocol Architecture

### Client-Host-Server Model

MCP follows a client-host-server architecture where:
- **Host**: Runs multiple client instances (similar to HTTP clients)
- **Client**: Connects to servers to access tools, resources, and prompts
- **Server**: Exposes functionality through standardized methods

### Transport Layer

The protocol is **transport-agnostic** and supports:
- **stdio** (standard input/output)
- **HTTP/HTTPS**
- **WebSocket**
- **SSE** (Server-Sent Events)
- **MQTT**

Messages are delimited by newlines and must NOT contain embedded newlines.

## Message Types

MCP defines **three types of messages** following JSON-RPC 2.0:

### 1. Requests

Messages that require a response. Each request has a unique ID.

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list",
  "params": {}
}
```

**Characteristics**:
- Must include `jsonrpc: "2.0"`
- Must include unique `id` (string or number)
- Must specify `method` name
- May include `params` object

### 2. Responses

Replies to requests that include the same ID.

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "tools": [
      {
        "name": "weather",
        "description": "Get weather information",
        "inputSchema": {
          "type": "object",
          "properties": {
            "location": {"type": "string"}
          },
          "required": ["location"]
        }
      }
    ]
  }
}
```

**Characteristics**:
- Must include the same `id` as the request
- Must include either `result` (success) or `error` (failure)
- Never includes both `result` and `error`

### 3. Notifications

One-way messages that do NOT expect a response.

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/initialized",
  "params": {}
}
```

**Common notifications**:
- `notifications/initialized` - Client signals it's ready
- `notifications/progress` - Server reports progress on long-running operations
- `notifications/cancelled` - Operation was cancelled

## Protocol Methods

### Lifecycle Methods

#### `initialize`

The **first required interaction** between client and server.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-11-25",
    "capabilities": {
      "tools": {},
      "resources": {},
      "prompts": {}
    },
    "clientInfo": {
      "name": "ltmatrix",
      "version": "0.1.0"
    }
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-11-25",
    "capabilities": {
      "tools": {
        "listChanged": true
      },
      "resources": {
        "subscribe": true,
        "listChanged": true
      }
    },
    "serverInfo": {
      "name": "playwright-mcp-server",
      "version": "1.0.0"
    }
  }
}
```

### Tool Methods

#### `tools/list`

Lists all available tools (functions) that the server exposes.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {
    "cursor": null
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "tools": [
      {
        "name": "browser_navigate",
        "description": "Navigate to a URL",
        "inputSchema": {
          "type": "object",
          "properties": {
            "url": {
              "type": "string",
              "description": "URL to navigate to"
            }
          },
          "required": ["url"]
        }
      }
    ],
    "nextCursor": null
  }
}
```

#### `tools/call`

Invokes a specific tool with provided arguments.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "browser_navigate",
    "arguments": {
      "url": "https://example.com"
    }
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Successfully navigated to https://example.com"
      },
      {
        "type": "image",
        "data": "base64-encoded-image-data",
        "mimeType": "image/png"
      }
    ],
    "isError": false
  }
}
```

### Resource Methods

#### `resources/list`

Lists available data resources (files, database schemas, application data).

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "resources/list",
  "params": {}
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "resources": [
      {
        "uri": "file:///project/package.json",
        "name": "package.json",
        "description": "Project package configuration",
        "mimeType": "application/json"
      }
    ]
  }
}
```

#### `resources/read`

Reads the contents of a specific resource.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "resources/read",
  "params": {
    "uri": "file:///project/package.json"
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "result": {
    "contents": [
      {
        "uri": "file:///project/package.json",
        "mimeType": "application/json",
        "text": "{\"name\": \"my-project\"}"
      }
    ]
  }
}
```

### Prompt Methods

#### `prompts/list`

Lists available prompt templates.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "prompts/list",
  "params": {}
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "result": {
    "prompts": [
      {
        "name": "code_review",
        "description": "Template for code review prompts",
        "arguments": [
          {
            "name": "language",
            "description": "Programming language",
            "required": true
          }
        ]
      }
    ]
  }
}
```

#### `prompts/get`

Retrieves a specific prompt template.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "prompts/get",
  "params": {
    "name": "code_review",
    "arguments": {
      "language": "rust"
    }
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "result": {
    "description": "Code review template for Rust",
    "messages": [
      {
        "role": "user",
        "content": {
          "type": "text",
          "text": "Review this Rust code for best practices, safety, and performance."
        }
      }
    ]
  }
}
```

## Error Handling

### Standard JSON-RPC 2.0 Error Codes

| Error Code | Name | Description |
|------------|------|-------------|
| **-32700** | Parse Error | Invalid JSON was received |
| **-32600** | Invalid Request | The JSON sent is not a valid Request object |
| **-32601** | Method Not Found | The method does not exist / is not available |
| **-32602** | Invalid Parameters | Invalid method parameters |
| **-32603** | Internal Error | Internal JSON-RPC error |
| **-32000 to -32099** | Server Error | Reserved for implementation-defined server errors |

### Error Response Structure

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "error": {
    "code": -32602,
    "message": "Invalid parameters",
    "data": {
      "field": "url",
      "reason": "must be a valid URL"
    }
  }
}
```

### MCP-Specific Error Scenarios

- **Unknown tools**: Tool name not found in `tools/list`
- **Malformed requests**: Requests that fail schema validation
- **Tool execution errors**: Runtime errors during tool execution
- **Resource not found**: Requested resource URI doesn't exist
- **Permission denied**: Client lacks access to requested resource

## Data Structures

### Tool Definition

```rust
struct Tool {
    name: String,
    description: String,
    input_schema: JsonSchema,  // JSON Schema draft 2020-12
}
```

### Tool Result Content

```rust
enum ToolContent {
    Text { text: String },
    Image {
        data: String,        // base64-encoded
        mime_type: String,
    },
    Resource {
        uri: String,
        mime_type: Option<String>,
    }
}
```

### Resource Definition

```rust
struct Resource {
    uri: String,
    name: String,
    description: Option<String>,
    mime_type: Option<String>,
}
```

### Resource Contents

```rust
struct ResourceContents {
    uri: String,
    mime_type: Option<String>,
    text: Option<String>,      // For text resources
    blob: Option<Vec<u8>>,     // For binary resources
}
```

## Communication Patterns

### Initialization Flow

1. **Client** → **Server**: `initialize` request with capabilities
2. **Server** → **Client**: `initialize` response with server capabilities
3. **Client** → **Server**: `notifications/initialized` (ready to proceed)

### Tool Execution Flow

1. **Client** → **Server**: `tools/list` to discover available tools
2. **Client** → **Server**: `tools/call` with tool name and arguments
3. **Server** → **Client**: `tools/call` response with results or error
4. **Server** → **Client**: `notifications/progress` (optional, for long-running operations)

### Resource Access Flow

1. **Client** → **Server**: `resources/list` to discover available resources
2. **Client** → **Server**: `resources/read` with specific URI
3. **Server** → **Client**: `resources/read` response with contents or error

## Protocol Capabilities

### Server Capabilities

Declared during initialization:

```json
{
  "tools": {
    "listChanged": true     // Server supports dynamic tool updates
  },
  "resources": {
    "subscribe": true,      // Server supports resource subscriptions
    "listChanged": true     // Server supports dynamic resource updates
  },
  "prompts": {
    "listChanged": true     // Server supports dynamic prompt updates
  }
}
```

### Client Capabilities

```json
{
  "sampling": {},           // Client supports sampling from models
  "roots": {
    "listChanged": true     // Client supports dynamic root updates
  }
}
```

## Transport Implementation Notes

### stdio Transport

- Messages delimited by newlines (`\n`)
- Each line is a complete JSON message
- No embedded newlines in messages
- Bidirectional communication over stdin/stdout

### WebSocket Transport

- Messages sent as WebSocket frames
- Each frame is a complete JSON message
- Supports server push via notifications
- Lower latency than stdio

## Key Design Principles

1. **Transport Agnostic**: Works over any bidirectional message channel
2. **Stateless**: Based on JSON-RPC 2.0
3. **Backward Compatible**: Protocol versioning in `initialize`
4. **Extensible**: Easy to add new methods and capabilities
5. **Type Safe**: JSON Schema for tool parameters and return values
6. **Progress Support**: Built-in progress notifications for long operations
7. **Resource Management**: Standardized resource access patterns
8. **Prompt Templates**: Reusable prompt patterns with parameters

## Use Cases for ltmatrix

Based on the project requirements:

1. **E2E Testing with Playwright**:
   - `tools/call` to invoke browser automation
   - `tools/list` to discover available Playwright commands
   - `notifications/progress` for test execution progress

2. **File System Operations**:
   - `resources/list` to discover project files
   - `resources/read` to read file contents
   - `tools/call` for file modifications

3. **Command Execution**:
   - `tools/call` to run build/test commands
   - Tool results capture stdout/stderr
   - Exit codes in tool results

## References

- [MCP Specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25)
- [MCP Specification 2025-06-18](https://modelcontextprotocol.io/specification/2025-06-18)
- [MCP Messages Documentation](https://modelcontextprotocol.io/specification/2024-11-05/basic/messages)
- [MCP Tools Documentation](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
- [MCP Resources Documentation](https://modelcontextprotocol.io/specification/2025-06-18/server/resources)
- [MCP Prompts Documentation](https://modelcontextprotocol.io/specification/2025-06-18/server/prompts)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [MCP Message Types Reference](https://portkey.ai/blog/mcp-message-types-complete-json-rpc-reference-guide)
