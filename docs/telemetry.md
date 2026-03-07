# Telemetry and Analytics Documentation

## Overview

ltmatrix includes optional anonymous usage telemetry to help improve the tool and understand how it's being used. **Telemetry is opt-in only and fully disabled by default.**

## Privacy Guarantees

We take user privacy seriously. Here's what we promise:

### What We Collect

- **Execution Mode**: Fast/Standard/Expert
- **Agent Backend**: Which agent was used (claude/opencode/etc)
- **Task Metrics**: Total tasks, completed tasks, failed tasks
- **Pipeline Duration**: How long the pipeline took to run
- **Error Categories**: Only the type of error (no messages or stack traces)
- **System Information**: OS, architecture, ltmatrix version
- **Anonymous Session ID**: Random UUID generated once and stored locally

### What We Don't Collect

- ❌ No IP addresses
- ❌ No user identifiers or personal information
- ❌ No project names or file paths
- ❌ No code content
- ❌ No full error messages or stack traces
- ❌ No environment variables or configuration values

## How to Enable Telemetry

### Command Line

```bash
ltmatrix --telemetry "build a REST API"
```

### Configuration File

Add to `~/.ltmatrix/config.toml` or `.ltmatrix/config.toml`:

```toml
[telemetry]
enabled = true
endpoint = "https://telemetry.ltmatrix.dev/events"
batch_size = 10
```

## Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `false` | Enable/disable telemetry collection |
| `endpoint` | string | `https://telemetry.ltmatrix.dev/events` | Analytics server endpoint |
| `batch_size` | integer | `10` | Number of events to batch before sending |
| `max_buffer_size` | integer | `100` | Maximum events to buffer in memory |
| `timeout_secs` | integer | `5` | HTTP timeout for sending events (seconds) |
| `max_retries` | integer | `3` | Number of retry attempts for failed sends |

## Example Configuration

```toml
# ~/.ltmatrix/config.toml

[telemetry]
# Enable telemetry (opt-in only)
enabled = true

# Custom analytics endpoint (optional)
endpoint = "https://your-analytics-server.com/events"

# Send in batches of 20 events
batch_size = 20

# Keep up to 200 events in memory
max_buffer_size = 200

# Timeout after 10 seconds
timeout_secs = 10

# Retry up to 5 times
max_retries = 5
```

## Events Collected

### Session Start Event

When you run ltmatrix with telemetry enabled, a session start event is collected:

```json
{
  "event_type": "session_start",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "version": "0.1.0",
  "os": "linux",
  "arch": "x86_64",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

### Pipeline Complete Event

When a pipeline finishes execution:

```json
{
  "event_type": "pipeline_complete",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "execution_mode": "standard",
  "agent_backend": "claude",
  "total_tasks": 5,
  "completed_tasks": 4,
  "failed_tasks": 1,
  "duration_secs": 342,
  "timestamp": "2025-01-15T10:35:42Z"
}
```

### Error Event

When an error occurs (categories only, no messages):

```json
{
  "event_type": "error",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "error_category": "test_failure",
  "timestamp": "2025-01-15T10:33:15Z"
}
```

## Error Categories

We only collect the error category, not the full error message:

- `agent_timeout` - Agent execution timed out
- `agent_execution_failed` - Agent execution failed
- `test_failure` - Test execution failed
- `verification_failed` - Task verification failed
- `git_operation_failed` - Git operation failed
- `configuration_error` - Configuration error
- `dependency_validation_failed` - Dependency validation failed
- `pipeline_execution_failed` - Pipeline execution failed
- `other` - Other uncategorized errors

## Session Management

- **Anonymous UUID**: A random UUID is generated on first run
- **Persistent**: Stored in `~/.ltmatrix/telemetry_session_id`
- **Private**: Not linked to any personal information
- **Per-Installation**: Each installation has a unique session ID

## Data Transmission

### How Data is Sent

1. Events are collected in memory during pipeline execution
2. Events are batched (default: 10 events per batch)
3. Batches are sent via HTTP POST to the configured endpoint
4. Fire-and-forget: Transmission failures don't block execution
5. Retry logic: 3 attempts with exponential backoff (2^n seconds)

### Privacy in Transmission

- **HTTPS Only**: All data is sent over encrypted HTTPS
- **No IP Logging**: The analytics endpoint should not log IPs
- **Fire-and-Forget**: If telemetry fails, ltmatrix continues normally
- **Local Buffer**: Events are only stored in memory, never on disk

## Disabling Telemetry

### Temporary

```bash
# Omit the --telemetry flag (default)
ltmatrix "build a REST API"
```

### Permanent

Add to `~/.ltmatrix/config.toml` or `.ltmatrix/config.toml`:

```toml
[telemetry]
enabled = false
```

## Viewing What's Sent

If you want to see exactly what data is being sent, you can:

1. **Enable debug logging**:
   ```bash
   ltmatrix --log-level debug --telemetry "your goal"
   ```

2. **Use a proxy** (like mitmproxy) to inspect HTTP traffic

3. **Check the source code**: All telemetry code is in `src/telemetry/`

## Why Telemetry?

Telemetry helps us:

- 📊 **Understand Usage**: Which features are used most
- 🐛 **Identify Issues**: Common error patterns and failure points
- ⚡ **Improve Performance**: Identify slow operations
- 🔧 **Guide Development**: Prioritize features based on real usage
- 📈 **Track Adoption**: How many users are using ltmatrix

## Open Source Commitment

- **Source Available**: All telemetry code is open source
- **Transparent**: This document explains exactly what's collected
- **Opt-In**: Telemetry is disabled by default
- **Easy to Disable**: One flag or config change to disable
- **No Lock-In**: Disabling telemetry has no impact on functionality

## Analytics Endpoint

### Default Endpoint

```
https://telemetry.ltmatrix.dev/events
```

### Custom Endpoint

You can set up your own analytics server:

```toml
[telemetry]
enabled = true
endpoint = "https://your-server.com/analytics"
```

### Event Format

Events are sent as JSON arrays:

```json
Content-Type: application/json
User-Agent: ltmatrix/0.1.0

[
  {
    "event_type": "session_start",
    ...
  },
  {
    "event_type": "pipeline_complete",
    ...
  }
]
```

## FAQ

**Q: Is my code sent to the server?**
A: No. We never collect code content, file paths, or project information.

**Q: Can you identify me from the data?**
A: No. Session IDs are random UUIDs with no link to your identity.

**Q: What if the telemetry server is down?**
A: ltmatrix continues normally. Telemetry failures are silently ignored.

**Q: How much bandwidth does telemetry use?**
A: Minimal. Each event is ~200 bytes, and we send ~10 events per pipeline run.

**Q: Can I see what data was sent?**
A: Yes. Enable debug logging with `--log-level debug` to see all telemetry data.

**Q: Is telemetry GDPR compliant?**
A: Yes. We collect no personal data, use anonymous UUIDs, and require opt-in consent.

## Reporting Issues

If you find any privacy concerns or issues with telemetry:

1. **GitHub Issues**: https://github.com/bigfish/ltmatrix/issues
2. **Security Email**: security@ltmatrix.dev (for security issues only)

## Future Improvements

We plan to add:

- [ ] Local telemetry file option (for debugging)
- [ ] User-configurable retention policies
- [ ] Per-project telemetry settings
- [ ] Telemetry status command (`ltmatrix telemetry status`)
- [ ] Data export/deletion tools

## Version History

- **v0.1.0** (2025-01-15): Initial telemetry implementation
  - Session tracking
  - Pipeline completion metrics
  - Error categorization
  - Opt-in only
  - Anonymous UUIDs
