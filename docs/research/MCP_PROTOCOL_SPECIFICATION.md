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

MCP uses a client-server architecture:

- **Hosts**: LLM applications that initiate connections
- **Clients**: Connectors within host applications
- **Servers**: Services that provide context and functionality (e.g., Playwright, filesystem, browser)

### Transport Layer

MCP supports multiple transport mechanisms:
1. **WebSocket** - Primary transport for bidirectional communication
2. **Stdio** - Standard input/output for process-based communication
3. **Batch requests** - JSON-RPC batch request support

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
| -32000 to -32099 | MCP-specific errors |

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

**Sources:**
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [MCP Error Codes Reference](https://www.mcpevals.io/blog/mcp-error-codes)

## Lifecycle Management

### Connection Initialization

1. Client connects to server (WebSocket or stdio)
2. Client sends `initialize` request
3. Server responds with capabilities
4. Client sends `initialized` notification
5. Connection is ready

### Shutdown

1. Client sends `shutdown` request
2. Server responds with confirmation
3. Client closes connection

## Capability Negotiation

Clients and servers advertise their capabilities during initialization:

**Client Capabilities:**
```json
{
  "tools": {},
  "resources": {
    "subscribe": true,
    "list": true
  },
  "prompts": {}
}
```

**Server Capabilities:**
```json
{
  "tools": {
    "listChanged": true
  },
  "resources": {
    "subscribe": true,
    "list": true
  },
  "prompts": {
    "listChanged": true
  }
}
```

## 2025-11-25 Update Features

The latest specification update (November 25, 2025) includes:

1. **Async Tasks** - Support for long-running operations
2. **Enhanced OAuth** - Improved authentication capabilities
3. **Extension Improvements** - Better extensibility support

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
- WebSocket transport support
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
- [Transports Documentation](https://modelcontextprotocol.info/docs/concepts/transports/)

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

*Document Version: 1.0*
*Last Updated: 2025-03-07*
*Researcher: Claude Sonnet 4.6*
