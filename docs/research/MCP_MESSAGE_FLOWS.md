# MCP Protocol Message Flows for ltmatrix

**Date**: 2025-03-07
**Purpose**: Practical examples of MCP message exchanges for E2E testing

## Scenario 1: Initial Handshake with Playwright Server

### Context

ltmatrix starts a Playwright MCP server and performs the initialization handshake.

### Message Flow

**1. Client → Server: Initialize Request**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-11-25",
    "capabilities": {
      "tools": {},
      "resources": {}
    },
    "clientInfo": {
      "name": "ltmatrix",
      "version": "0.1.0"
    }
  }
}
```

**2. Server → Client: Initialize Response**

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
      "resources": {}
    },
    "serverInfo": {
      "name": "@playwright/mcp-server",
      "version": "1.0.0"
    }
  }
}
```

**3. Client → Server: Initialized Notification**

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/initialized"
}
```

**Status**: ✅ Handshake complete, server ready for tool calls

---

## Scenario 2: Discover Available Testing Tools

### Context

ltmatrix wants to discover what Playwright testing tools are available.

### Message Flow

**Client → Server: Tools List Request**

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {}
}
```

**Server → Client: Tools List Response**

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "tools": [
      {
        "name": "browser_launch",
        "description": "Launch a browser instance",
        "inputSchema": {
          "type": "object",
          "properties": {
            "headless": {
              "type": "boolean",
              "description": "Run browser in headless mode",
              "default": true
            },
            "browserType": {
              "type": "string",
              "enum": ["chromium", "firefox", "webkit"],
              "description": "Browser type to launch",
              "default": "chromium"
            }
          }
        }
      },
      {
        "name": "page_navigate",
        "description": "Navigate to a URL",
        "inputSchema": {
          "type": "object",
          "properties": {
            "url": {
              "type": "string",
              "format": "uri",
              "description": "URL to navigate to"
            }
          },
          "required": ["url"]
        }
      },
      {
        "name": "page_screenshot",
        "description": "Take a screenshot of the current page",
        "inputSchema": {
          "type": "object",
          "properties": {
            "path": {
              "type": "string",
              "description": "Path to save screenshot"
            },
            "fullPage": {
              "type": "boolean",
              "description": "Capture full scrollable page",
              "default": false
            }
          }
        }
      },
      {
        "name": "page_click",
        "description": "Click an element on the page",
        "inputSchema": {
          "type": "object",
          "properties": {
            "selector": {
              "type": "string",
              "description": "CSS selector for element"
            }
          },
          "required": ["selector"]
        }
      },
      {
        "name": "page_fill",
        "description": "Fill a form field",
        "inputSchema": {
          "type": "object",
          "properties": {
            "selector": {
              "type": "string",
              "description": "CSS selector for input"
            },
            "value": {
              "type": "string",
              "description": "Value to fill"
            }
          },
          "required": ["selector", "value"]
        }
      },
      {
        "name": "page_wait_for_selector",
        "description": "Wait for a selector to appear",
        "inputSchema": {
          "type": "object",
          "properties": {
            "selector": {
              "type": "string",
              "description": "CSS selector to wait for"
            },
            "timeout": {
              "type": "number",
              "description": "Maximum time to wait in ms",
              "default": 30000
            }
          },
          "required": ["selector"]
        }
      },
      {
        "name": "page_close",
        "description": "Close the current page",
        "inputSchema": {
          "type": "object",
          "properties": {}
        }
      },
      {
        "name": "browser_close",
        "description": "Close the browser instance",
        "inputSchema": {
          "type": "object",
          "properties": {}
        }
      }
    ]
  }
}
```

**Status**: ✅ 8 tools discovered, ready for testing

---

## Scenario 3: Execute E2E Test Flow

### Context

ltmatrix runs a simple E2E test: launch browser, navigate, take screenshot.

### Message Flow

**1. Client → Server: Launch Browser**

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "browser_launch",
    "arguments": {
      "headless": true,
      "browserType": "chromium"
    }
  }
}
```

**Server → Client: Launch Response**

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Launched Chromium browser (headless)"
      }
    ],
    "isError": false
  }
}
```

**2. Client → Server: Navigate to URL**

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "page_navigate",
    "arguments": {
      "url": "https://example.com"
    }
  }
}
```

**Server → Client: Navigate Response**

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Navigated to https://example.com"
      }
    ],
    "isError": false
  }
}
```

**3. Client → Server: Wait for Page Load**

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "tools/call",
  "params": {
    "name": "page_wait_for_selector",
    "arguments": {
      "selector": "body",
      "timeout": 5000
    }
  }
}
```

**Server → Client: Wait Response**

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Selector 'body' found after 234ms"
      }
    ],
    "isError": false
  }
}
```

**4. Client → Server: Take Screenshot**

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "tools/call",
  "params": {
    "name": "page_screenshot",
    "arguments": {
      "path": "screenshot.png",
      "fullPage": true
    }
  }
}
```

**Server → Client: Screenshot Response**

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Screenshot saved to screenshot.png"
      },
      {
        "type": "image",
        "data": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
        "mimeType": "image/png"
      }
    ],
    "isError": false
  }
}
```

**5. Client → Server: Close Browser**

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "tools/call",
  "params": {
    "name": "browser_close",
    "arguments": {}
  }
}
```

**Server → Client: Close Response**

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Browser closed"
      }
    ],
    "isError": false
  }
}
```

**Status**: ✅ Test completed successfully

---

## Scenario 4: Form Interaction Test

### Context

ltmatrix tests a login form by filling fields and clicking submit.

### Message Flow

**1. Client → Server: Navigate to Login Page**

```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "tools/call",
  "params": {
    "name": "page_navigate",
    "arguments": {
      "url": "https://example.com/login"
    }
  }
}
```

**2. Client → Server: Fill Username Field**

```json
{
  "jsonrpc": "2.0",
  "id": 9,
  "method": "tools/call",
  "params": {
    "name": "page_fill",
    "arguments": {
      "selector": "#username",
      "value": "testuser@example.com"
    }
  }
}
```

**3. Client → Server: Fill Password Field**

```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "method": "tools/call",
  "params": {
    "name": "page_fill",
    "arguments": {
      "selector": "#password",
      "value": "secretpassword123"
    }
  }
}
```

**4. Client → Server: Click Submit Button**

```json
{
  "jsonrpc": "2.0",
  "id": 11,
  "method": "tools/call",
  "params": {
    "name": "page_click",
    "arguments": {
      "selector": "button[type='submit']"
    }
  }
}
```

**5. Client → Server: Wait for Success Message**

```json
{
  "jsonrpc": "2.0",
  "id": 12,
  "method": "tools/call",
  "params": {
    "name": "page_wait_for_selector",
    "arguments": {
      "selector": ".success-message",
      "timeout": 5000
    }
  }
}
```

**6. Client → Server: Take Final Screenshot**

```json
{
  "jsonrpc": "2.0",
  "id": 13,
  "method": "tools/call",
  "params": {
    "name": "page_screenshot",
    "arguments": {
      "path": "login-success.png"
    }
  }
}
```

**Status**: ✅ Form interaction test completed

---

## Scenario 5: Error Handling - Invalid Tool Call

### Context

ltmatrix attempts to call a non-existent tool.

### Message Flow

**Client → Server: Invalid Tool Call**

```json
{
  "jsonrpc": "2.0",
  "id": 14,
  "method": "tools/call",
  "params": {
    "name": "nonexistent_tool",
    "arguments": {}
  }
}
```

**Server → Client: Error Response**

```json
{
  "jsonrpc": "2.0",
  "id": 14,
  "error": {
    "code": -32001,
    "message": "Unknown tool: nonexistent_tool",
    "data": {
      "availableTools": [
        "browser_launch",
        "page_navigate",
        "page_screenshot",
        "page_click",
        "page_fill",
        "page_wait_for_selector",
        "page_close",
        "browser_close"
      ]
    }
  }
}
```

**Status**: ❌ Error properly handled with helpful information

---

## Scenario 6: Error Handling - Invalid Parameters

### Context

ltmatrix calls a tool with invalid parameters.

### Message Flow

**Client → Server: Navigate Without URL**

```json
{
  "jsonrpc": "2.0",
  "id": 15,
  "method": "tools/call",
  "params": {
    "name": "page_navigate",
    "arguments": {}
  }
}
```

**Server → Client: Validation Error**

```json
{
  "jsonrpc": "2.0",
  "id": 15,
  "error": {
    "code": -32602,
    "message": "Invalid params",
    "data": {
      "field": "url",
      "reason": "missing required property"
    }
  }
}
```

**Status**: ❌ Validation error with specific field information

---

## Scenario 7: Progress Notification for Long-Running Operation

### Context

ltmatrix triggers a long-running operation and receives progress updates.

### Message Flow

**Client → Server: Start Long Operation**

```json
{
  "jsonrpc": "2.0",
  "id": 16,
  "method": "tools/call",
  "params": {
    "name": "browser_launch",
    "arguments": {
      "headless": false
    }
  }
}
```

**Server → Client: Progress Notification (before response)**

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/progress",
  "params": {
    "progressToken": 16,
    "progress": {
      "kind": "operation_in_progress",
      "message": "Downloading browser binaries...",
      "percentage": 45
    }
  }
}
```

**Server → Client: Another Progress Update**

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/progress",
  "params": {
    "progressToken": 16,
    "progress": {
      "kind": "operation_in_progress",
      "message": "Installing browser...",
      "percentage": 78
    }
  }
}
```

**Server → Client: Final Response**

```json
{
  "jsonrpc": "2.0",
  "id": 16,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Browser launched successfully"
      }
    ],
    "isError": false
  }
}
```

**Status**: ✅ Operation completed with progress tracking

---

## Scenario 8: Resource Access - Reading Test Files

### Context

ltmatrix reads project files through MCP resources (future enhancement).

### Message Flow

**Client → Server: List Available Resources**

```json
{
  "jsonrpc": "2.0",
  "id": 17,
  "method": "resources/list",
  "params": {}
}
```

**Server → Client: Resources List**

```json
{
  "jsonrpc": "2.0",
  "id": 17,
  "result": {
    "resources": [
      {
        "uri": "file:///project/package.json",
        "name": "package.json",
        "description": "Project configuration",
        "mimeType": "application/json"
      },
      {
        "uri": "file:///project/tests/example.test.ts",
        "name": "example.test.ts",
        "description": "Example test file",
        "mimeType": "text/typescript"
      },
      {
        "uri": "file:///project/README.md",
        "name": "README.md",
        "description": "Project documentation",
        "mimeType": "text/markdown"
      }
    ]
  }
}
```

**Client → Server: Read Test File**

```json
{
  "jsonrpc": "2.0",
  "id": 18,
  "method": "resources/read",
  "params": {
    "uri": "file:///project/tests/example.test.ts"
  }
}
```

**Server → Client: Resource Contents**

```json
{
  "jsonrpc": "2.0",
  "id": 18,
  "result": {
    "contents": [
      {
        "uri": "file:///project/tests/example.test.ts",
        "mimeType": "text/typescript",
        "text": "import { test, expect } from '@playwright/test';\n\ntest('example test', async ({ page }) => {\n  await page.goto('https://example.com');\n  await expect(page).toHaveTitle(/Example/);\n});"
      }
    ]
  }
}
```

**Status**: ✅ Resource accessed successfully

---

## Key Takeaways for Implementation

### Message Structure

1. **All messages** have `jsonrpc: "2.0"`
2. **Requests** always have an `id` (string or number)
3. **Notifications** never have an `id`
4. **Responses** always match the request `id`
5. **Responses** have either `result` OR `error`, never both

### Error Handling

1. Use standard JSON-RPC 2.0 error codes
2. Include MCP-specific error codes (-32000 to -32099)
3. Provide helpful error data with context
4. Validate parameters before tool execution

### Request IDs

1. Use sequential integers for simplicity
2. Track pending requests by ID
3. Match responses to requests
4. Handle notifications (no ID) separately

### Transport Considerations

1. Messages are newline-delimited
2. No embedded newlines in JSON
3. Read/write line by line over stdio
4. Handle partial reads gracefully

### Testing Flow

1. Initialize → Discover → Execute → Cleanup
2. Always close resources (browser, page)
3. Use progress notifications for long operations
4. Capture screenshots for debugging
5. Handle errors gracefully with context

## Integration Points for ltmatrix

### Test Execution Pipeline

```rust
// In src/pipeline/test.rs
async fn run_e2e_tests_with_mcp(config: &McpConfig) -> Result<TestResults> {
    let mut client = McpClient::from_server_config(&config.server)?;

    // Initialize
    client.initialize().await?;

    // Discover tools
    let tools = client.tools_list().await?;

    // Launch browser
    client.call_tool("browser_launch", json!({"headless": true})).await?;

    // Run test steps
    for step in test_steps {
        client.call_tool(&step.tool, step.args).await?;
    }

    // Cleanup
    client.call_tool("browser_close", json!({})).await?;

    Ok(results)
}
```

### Error Recovery

```rust
// Handle tool errors gracefully
match client.call_tool("page_navigate", args).await {
    Ok(response) => {
        if response.is_error {
            // Tool executed but returned error
            log::warn!("Tool returned error: {:?}", response.content);
        } else {
            // Success
        }
    }
    Err(e) => {
        // Communication or protocol error
        if let Some(mcp_error) = e.downcast_ref::<JsonRpcError>() {
            match mcp_error.code {
                -32001 => log::error!("Unknown tool"),
                -32602 => log::error!("Invalid parameters: {}", mcp_error.message),
                _ => log::error!("MCP error: {}", mcp_error.message),
            }
        }
    }
}
```

### Progress Tracking

```rust
// Subscribe to progress notifications
client.set_progress_handler(|progress| {
    log::info!("Tool progress: {}% - {}", progress.percentage, progress.message);
});
```

## References

- [MCP Protocol Research](./MCP_PROTOCOL_RESEARCH.md)
- [MCP Implementation Requirements](./MCP_IMPLEMENTATION_REQUIREMENTS.md)
- [MCP Specification](https://modelcontextprotocol.io/specification/2025-11-25)
