# MCP Protocol Specification Research

## Overview

**Model Context Protocol (MCP)** is an open protocol that enables seamless integration between LLM applications and external data sources and tools.

- **Latest Version**: 2025-11-25 (November 25, 2025 - one-year anniversary update)
- **Protocol Base**: JSON-RPC 2.0
- **Encoding**: UTF-8
- **Purpose**: Enable AI agents to interact with external tools, resources, and data sources

**Sources:**
- [Specification - Model Context Protocol](https://modelcontextprotocol.io/specification/2025-11-25)
- [Model Context Protocol - GitHub](https://github.com/modelcontextprotocol/modelcontextprotocol)
- [MCP 的JSON-RPC 消息详解](https://jimmysong.io/zh/book/ai-handbook/mcp/json-rpc/)

## Protocol Architecture

### Communication Model

MCP uses a client-host-server architecture:

- **Hosts**: LLM applications (IDEs, chat interfaces) that manage multiple client connections
- **Clients**: Connectors within host applications that maintain 1:1 connections with servers
- **Servers**: Services that provide context and functionality (e.g., Playwright, filesystem, browser)

### Protocol Lifecycle

MCP connections progress through three distinct phases:

#### 1. Initialization Phase

1. **Client** sends `initialize` request with protocol version and capabilities
2. **Server** responds with its protocol version and capabilities
3. **Client** sends `notifications/initialized` notification
4. Connection enters **Operation Phase**

```json
// Step 1: Client initialize request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-11-25",
    "capabilities": { "tools": {} },
    "clientInfo": { "name": "ltmatrix", "version": "0.1.0" }
  }
}

// Step 2: Server response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-11-25",
    "capabilities": { "tools": { "listChanged": true } },
    "serverInfo": { "name": "playwright-mcp", "version": "1.0.0" }
  }
}

// Step 3: Client initialized notification
{
  "jsonrpc": "2.0",
  "method": "notifications/initialized"
}
```

#### 2. Operation Phase

Normal request/response communication:
- Client requests (`tools/list`, `tools/call`, `resources/read`, etc.)
- Server responses with results
- Server notifications (`notifications/message`, `notifications/progress`)

#### 3. Shutdown Phase

Graceful termination:
1. **Client** sends `shutdown` request (optional, recommended)
2. **Server** responds with confirmation
3. **Client** closes transport connection

### Transport Layer

MCP supports two transport mechanisms:

#### 1. stdio Transport (Recommended for local servers)

- Communication via standard input/output
- Each message is a JSON object on a single line
- Messages delimited by newline characters (`\n`)
- No embedded newlines in JSON content
- UTF-8 encoding throughout

```
Client stdout → Server stdin
Server stdout → Client stdin
```

#### 2. Streamable HTTP Transport (For remote servers)

**POST Requests** (Client → Server):
- Endpoint: `/mcp`
- Headers: `Content-Type: application/json`
- Optional: `Mcp-Session-Id` header for session management
- Body: Single JSON-RPC message

**GET Requests** (Server → Client streaming):
- Endpoint: `/mcp`
- Headers: `Accept: text/event-stream`
- Response: Server-Sent Events (SSE) stream
- Includes `Mcp-Session-Id` header in response

**SSE Event Format**:
```
event: message
data: {"jsonrpc":"2.0","method":"notifications/message","params":{...}}
```

**Session Management**:
- Server returns `Mcp-Session-Id` header on initialize response
- Client includes this header in subsequent requests
- Sessions are immutable - include all capabilities from initialization

## Message Types

MCP implements the standard JSON-RPC 2.0 message types:

### 1. Request

Client-initiated message calling a server method.

**Structure:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list",
  "params": {}
}
```

**Fields:**
- `jsonrpc`: Always "2.0"
- `id`: Request identifier (number or string)
- `method`: Method name to invoke
- `params`: Method parameters (object or array, optional)

### 2. Response

Server reply to a request.

**Success Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "tools": [...]
  }
}
```

**Error Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32601,
    "message": "Method not found",
    "data": null
  }
}
```

### 3. Notification

One-way message from server to client (no response expected).

**Structure:**
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/message",
  "params": {
    "message": "Server status update"
  }
}
```

**Note:** Notifications omit the `id` field.

## Core Protocol Methods

### Initialize

Establishes the connection and negotiates capabilities.

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
      "version": "1.0.0"
    }
  }
}
```

### Tools

Server capabilities for executable functions.

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {}
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "tools": [
      {
        "name": "playwright_navigate",
        "description": "Navigate to a URL",
        "inputSchema": {
          "type": "object",
          "properties": {
            "url": {"type": "string"}
          },
          "required": ["url"]
        }
      }
    ]
  }
}
```

**Call Tool:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "playwright_navigate",
    "arguments": {
      "url": "https://example.com"
    }
  }
}
```

### Resources

Data that can be read and sometimes written.

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "resources/list",
  "params": {}
}
```

**Read Resource:**
```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "resources/read",
  "params": {
    "uri": "file:///path/to/file.txt"
  }
}
```

### Prompts

Templates for generating content.

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "prompts/list",
  "params": {}
}
```

**Get Prompt:**
```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "prompts/get",
  "params": {
    "name": "summarize",
    "arguments": {
      "topic": "MCP protocol"
    }
  }
}
```

## Error Handling

### Standard JSON-RPC Error Codes

| Code | Name | Description |
|------|------|-------------|
| -32700 | Parse Error | Invalid JSON received or JSON syntax error |
| -32600 | Invalid Request | The JSON sent is not a valid Request object |
| -32601 | Method Not Found | The requested method does not exist |
| -32602 | Invalid Parameters | Invalid method parameters |
| -32603 | Internal Error | Internal server error |

### MCP-Specific Error Codes

| Code Range | Description |
|------------|-------------|
| -32000 to -32099 | MCP-specific server errors |

**Common MCP Error Codes:**

| Code | Name | Scenario |
|------|------|----------|
| -32001 | Unknown Tool | Tool name not found in server's tool registry |
| -32002 | Malformed Request | Request fails schema validation |
| -32003 | Tool Execution Error | Runtime error during tool execution |
| -32004 | Resource Not Found | Requested resource URI doesn't exist |
| -32005 | Permission Denied | Client lacks access to requested resource |

### Error Response Format

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32602,
    "message": "Invalid parameters",
    "data": {
      "field": "url",
      "reason": "Must be a valid URL"
    }
  }
}
```

### Tool Execution Errors

Tools can return errors in two ways:

1. **Protocol Error**: `error` field in response (communication/protocol issues)
2. **Tool Error**: `isError: true` in result (tool executed but failed)

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Error: Failed to navigate - URL invalid"
      }
    ],
    "isError": true
  }
}
```

**Sources:**
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [MCP Error Codes Reference](https://www.mcpevals.io/blog/mcp-error-codes)

## Lifecycle Management

### Connection Initialization

1. Client establishes transport connection (stdio or HTTP)
2. Client sends `initialize` request with protocol version and capabilities
3. Server responds with its protocol version and capabilities
4. Client sends `initialized` notification (no response expected)
5. Connection enters **Operation Phase** - ready for normal requests

### Shutdown

Graceful shutdown is recommended:

1. Client sends `shutdown` request (optional but recommended)
2. Server responds with empty result `{}`
3. Client closes the transport connection

**Note**: Servers should handle abrupt connection closes gracefully.

## Capability Negotiation

Capabilities are exchanged during the initialization handshake. Both client and server declare what features they support.

### Client Capabilities

```json
{
  "experimental": {},          // Optional experimental features
  "roots": {
    "listChanged": true        // Client can notify server of root changes
  },
  "sampling": {}               // Client supports LLM sampling requests
}
```

**Client Capability Details:**

| Capability | Sub-capability | Description |
|------------|----------------|-------------|
| `roots` | `listChanged` | Client supports `notifications/roots/list_changed` |
| `sampling` | - | Client can handle `sampling/createMessage` requests |

### Server Capabilities

```json
{
  "experimental": {},          // Optional experimental features
  "logging": {},               // Server supports `logging/setLevel`
  "completions": {},           // Server supports `completion/complete`
  "prompts": {
    "listChanged": true        // Server supports prompt list change notifications
  },
  "resources": {
    "subscribe": true,         // Server supports resource subscriptions
    "listChanged": true        // Server supports resource list change notifications
  },
  "tools": {
    "listChanged": true        // Server supports tool list change notifications
  }
}
```

**Server Capability Details:**

| Capability | Sub-capability | Description |
|------------|----------------|-------------|
| `tools` | `listChanged` | Server sends `notifications/tools/list_changed` |
| `resources` | `subscribe` | Server supports `resources/subscribe` and `resources/unsubscribe` |
| `resources` | `listChanged` | Server sends `notifications/resources/list_changed` |
| `prompts` | `listChanged` | Server sends `notifications/prompts/list_changed` |
| `logging` | - | Server supports `logging/setLevel` for log filtering |
| `completions` | - | Server supports argument completion for prompts |

### Roots (Filesystem Roots)

Clients can expose filesystem roots to servers:

```json
{
  "roots": [
    {
      "uri": "file:///home/user/project",
      "name": "Project Root"
    }
  ]
}
```

Servers use roots to understand the client's filesystem context.

## 2025-11-25 Update Features

The latest specification update (November 25, 2025) includes:

1. **Streamable HTTP Transport** - New HTTP-based transport with optional SSE for server-to-client streaming
2. **Enhanced OAuth** - Improved authentication capabilities for remote servers
3. **Extension Improvements** - Better extensibility support with experimental capabilities
4. **Session Management** - Mcp-Session-Id header for HTTP transport session tracking
5. **Async Task Support** - Progress notifications for long-running operations

## Implementation Requirements for ltmatrix

Based on the Python baseline and the reference document, ltmatrix needs:

### Minimal Implementation (Option B - Recommended)

**Server Launcher Only:**
- Spawn MCP server processes (Playwright, etc.)
- Pass configuration via command-line arguments
- Delegate protocol handling to Claude CLI
- Simple process management

**Pros:**
- Simpler implementation
- Less maintenance burden
- Leverages existing Claude CLI MCP support
- Fits Python baseline pattern

**Cons:**
- Depends on Claude CLI for protocol handling
- Less control over MCP interactions

### Full Implementation (Option A)

**Complete MCP Client:**
- Full JSON-RPC 2.0 implementation
- stdio transport support (primary)
- HTTP transport support (optional, for remote servers)
- Direct tool invocation
- Resource management
- Prompt templates

**Pros:**
- Complete control
- No external dependencies
- Can extend beyond Claude CLI capabilities

**Cons:**
- Complex implementation
- Higher maintenance burden
- May reinvent existing functionality

## Recommended Approach for ltmatrix

Based on the reference document's Python baseline and the existing `--mcp-config` implementation:

**Start with Option B (Server Launcher)**
- MCP servers are configured via `--mcp-config`
- ltmatrix spawns server processes
- Server configuration is passed to Claude CLI
- Claude CLI handles actual MCP protocol communication

**Rationale:**
1. Python baseline uses this pattern (passes `--mcp-config` to Claude CLI)
2. Configuration already implemented
3. Simpler and more maintainable
4. Aligns with "don't reinvent the wheel" principle

## Next Steps

For the brainstorming process:

1. **Confirm Approach**: Validate Option B (Server Launcher) vs Option A (Full Client)
2. **Design Data Structures**: If full client needed, design protocol message types
3. **Define Module Structure**: `src/mcp/` for protocol implementation
4. **Integration Points**: How MCP integrates with pipeline stages (Test, Verify)

## References

**Official Documentation:**
- [Model Context Protocol Official Spec](https://modelcontextprotocol.io/specification/2025-11-25)
- [MCP GitHub Repository](https://github.com/modelcontextprotocol/modelcontextprotocol)
- [Transports Documentation](https://modelcontextprotocol.io/docs/concepts/transports)

**Technical References:**
- [MCP 的JSON-RPC 消息详解](https://jimmysong.io/zh/book/ai-handbook/mcp/json-rpc/)
- [MCP 基础协议：2025-03-26 修订版本 - 知乎](https://zhuanlan.zhihu.com/p/)
- [MCP规范完整中译稿：2025-3-26版 - 腾讯云](https://cloud.tencent.com/developer/article/2541726)
- [MCP Message Types: Complete MCP JSON-RPC Reference Guide](https://app.daily.dev/posts/mcp-message-types-complete-mcp-json-rpc-reference-guide-nn7scunif)
- [Why Model Context Protocol uses JSON-RPC - Medium](https://medium.com/@dan.avila7/why-model-context-protocol-uses-json-rpc-64d466112338)
- [Model Context Protocol Comprehensive Guide for 2025](https://dysnix.com/blog/model-context-protocol)

**Standards:**
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)

---

*Document Version: 1.1*
*Last Updated: 2026-03-07*
*Researcher: Claude Sonnet 4.6*
