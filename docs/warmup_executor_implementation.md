# Warmup Executor Implementation

## Overview

The warmup executor module provides pre-initialization of agent sessions before actual task execution, improving first-response latency and ensuring agents are ready when needed.

## Architecture

### Components

1. **WarmupExecutor**: Main executor that orchestrates warmup queries
2. **WarmupConfig**: Configuration structure controlling warmup behavior
3. **WarmupResult**: Result type indicating warmup success, failure, or skip
4. **SessionPool**: In-memory session registry for warmed sessions

### Key Design Decisions

1. **Opt-In by Default**: Warmup is disabled by default to avoid unnecessary API calls
2. **Configurable Queries**: Supports multiple warmup queries with configurable limits
3. **Graceful Degradation**: Warmup failures don't prevent task execution
4. **Session Reuse**: Warmed sessions are automatically reused across the pipeline
5. **Timeout Protection**: Each warmup query has a configurable timeout

## Implementation Details

### WarmupExecutor

Located in `src/agent/warmup.rs`, the `WarmupExecutor` provides:

#### Core Methods

```rust
// Create executor with configuration
pub fn new(config: WarmupConfig) -> Self

// Warm up a single agent backend
pub async fn warmup_agent<B>(&self, backend: &B, pool: &mut SessionPool) -> Result<WarmupResult>

// Warm up multiple agent backends
pub async fn warmup_agents<B>(&self, backends: &[&B], pool: &mut SessionPool) -> Vec<WarmupResult>

// Check if an agent is available (lightweight check)
pub async fn check_agent_available<B>(&self, backend: &B) -> Result<bool>
```

#### Warmup Strategy

1. **Skip if Disabled**: Returns `WarmupResult::Skipped` immediately if warmup is disabled
2. **Get/Create Session**: Retrieves or creates a session from the pool
3. **Execute Warmup Queries**: Sends configured number of warmup prompts
4. **Handle Failures**: Implements retry logic if configured
5. **Track Metrics**: Records query count and duration

### WarmupResult

Result type with three states:

```rust
pub enum WarmupResult {
    Success {
        queries_executed: u32,
        duration_ms: u64,
    },
    Skipped,
    Failed {
        error: String,
        queries_executed: u32,
    },
}
```

Helper methods:
- `is_success()`, `is_skipped()`, `is_failed()`
- `queries_executed()` - Returns number of queries attempted

### WarmupConfig

Configuration structure with defaults:

```rust
pub struct WarmupConfig {
    pub enabled: bool,              // Default: false
    pub max_queries: u32,            // Default: 3
    pub timeout_seconds: u64,        // Default: 30
    pub retry_on_failure: bool,      // Default: false
    pub prompt_template: Option<String>, // Default: None
}
```

Validation rules:
- `max_queries` must be > 0
- `timeout_seconds` must be > 0
- `prompt_template` cannot be empty if provided

## Configuration Examples

### Basic Configuration

```toml
# ~/.ltmatrix/config.toml
[warmup]
enabled = true
max_queries = 3
timeout_seconds = 30
```

### Custom Prompt

```toml
[warmup]
enabled = true
prompt_template = "Hello! Please respond with 'Ready' if you're working."
```

### With Retry Logic

```toml
[warmup]
enabled = true
max_queries = 2
timeout_seconds = 45
retry_on_failure = true
```

## Usage Examples

### Single Agent Warmup

```rust
use ltmatrix::agent::warmup::WarmupExecutor;
use ltmatrix::agent::pool::SessionPool;
use ltmatrix::agent::ClaudeAgent;

// Create executor with config
let executor = WarmupExecutor::new(config);
let mut pool = SessionPool::new();

// Warm up Claude agent
let agent = ClaudeAgent::new()?;
match executor.warmup_agent(&agent, &mut pool).await? {
    WarmupResult::Success { queries_executed, duration_ms } => {
        println!("Warmed up with {} queries in {}ms", queries_executed, duration_ms);
    }
    WarmupResult::Skipped => {
        println!("Warmup was skipped (disabled)");
    }
    WarmupResult::Failed { error, .. } => {
        eprintln!("Warmup failed: {}", error);
    }
}
```

### Multiple Agents

```rust
let agents: Vec<&dyn AgentBackend> = vec![&claude_agent, &opencode_agent];
let results = executor.warmup_agents(&agents, &mut pool).await;

for (i, result) in results.iter().enumerate() {
    println!("Agent {} warmup: {:?}", i, result);
}
```

### Availability Check

```rust
// Quick check without full warmup
if executor.check_agent_available(&agent).await? {
    println!("Agent is available");
} else {
    println!("Agent is not available");
}
```

## Error Handling

### Timeout Handling

```rust
// Each warmup query is wrapped in a timeout
let timeout_duration = Duration::from_secs(config.timeout_seconds as u64);
timeout(timeout_duration, async {
    backend.execute_with_session(prompt, &config, session).await
}).await
```

### Retry Logic

When `retry_on_failure` is enabled:
- Up to 2 retry attempts
- Exponential backoff (100ms, 200ms)
- Returns last error if all retries fail

### Graceful Degradation

Warmup failures don't prevent task execution:
- Failed warmup returns `WarmupResult::Failed`
- Task execution proceeds with fresh session
- Error is logged but doesn't halt the pipeline

## Logging

Structured logging at appropriate levels:

```rust
// INFO: Warmup start/completion
info!("Starting warmup for agent '{}' (model: {})", agent.name, agent.model);
info!("Warmup completed for agent '{}' in {}ms", agent.name, duration_ms);

// DEBUG: Query execution details
debug!("Executing warmup query {}/{} for agent '{}'", query_num + 1, max_queries, agent.name);

// WARN: Failures and retries
warn!("Warmup query failed: {}", error);
warn!("Retrying warmup due to retry_on_failure=true");
```

## Testing

### Unit Tests

Located in `src/agent/warmup.rs`:
- 11 unit tests covering all functionality
- Mock agent backend for testing
- Tests for success, failure, retry, and skip scenarios

### Integration Tests

Located in `tests/warmup_executor_integration_test.rs`:
- 18 integration tests for real-world scenarios
- Configuration validation
- Session pool integration
- Serialization/deserialization

### Running Tests

```bash
# Unit tests only
cargo test --lib warmup

# Integration tests
cargo test --test warmup_executor_integration_test

# All warmup tests
cargo test warmup
```

## Performance Considerations

### Benefits

1. **Reduced First-Response Latency**: Agent processes are initialized before critical work
2. **Session Reuse**: Warmed sessions are reused across retry and dependency chains
3. **Early Failure Detection**: Agent availability issues are caught early

### Costs

1. **API Calls**: Each warmup query consumes API quota
2. **Startup Time**: Adds delay before task execution (typically 100-500ms)
3. **Memory**: Sessions consume memory until stale (1-hour default)

### Recommendations

- Enable warmup for production workflows where latency matters
- Disable warmup for development/dry-run scenarios
- Use `max_queries = 1` for minimal overhead
- Set appropriate `timeout_seconds` based on network conditions

## Integration with Pipeline

The warmup executor integrates with the task pipeline:

1. **Generate Stage**: Agent warmed up before task generation
2. **Execute Stage**: Warmed sessions are reused for task execution
3. **Retry Scenarios**: Same session used across retry attempts
4. **Dependency Chains**: Sessions inherited by dependent tasks

## Future Enhancements

Potential improvements:

1. **Parallel Warmup**: Warm multiple agents concurrently
2. **Adaptive Queries**: Adjust warmup queries based on agent response
3. **Metrics Collection**: Track warmup success rates and durations
4. **Conditional Warmup**: Skip warmup if session was recently used
5. **Health Monitoring**: Periodic warmup to maintain session readiness

## Configuration Reference

### Full Configuration

```toml
[warmup]
enabled = true                       # Enable warmup
max_queries = 3                      # Number of warmup queries
timeout_seconds = 30                 # Timeout per query
retry_on_failure = false             # Retry on warmup failure
prompt_template = "Ready?"           # Custom prompt (optional)
```

### Default Values

```toml
[warmup]
enabled = false                      # Disabled by default
max_queries = 3                      # Up to 3 warmup queries
timeout_seconds = 30                 # 30 second timeout
retry_on_failure = false             # No retries by default
prompt_template = null               # Use default prompt
```

## Example Configurations

### Fast Mode (Minimal Warmup)

```toml
[modes.fast]
[warmup]
enabled = true
max_queries = 1
timeout_seconds = 15
```

### Expert Mode (Thorough Warmup)

```toml
[modes.expert]
[warmup]
enabled = true
max_queries = 5
timeout_seconds = 60
retry_on_failure = true
prompt_template = "Are you ready for complex code generation tasks?"
```

## Troubleshooting

### Warmup Timeouts

**Symptom**: Warmup fails with timeout errors

**Solutions**:
- Increase `timeout_seconds` in config
- Check network connectivity to agent API
- Verify agent command is available

### High API Costs

**Symptom**: Increased API usage from warmup

**Solutions**:
- Disable warmup with `enabled = false`
- Reduce `max_queries` to minimum (1)
- Use warmup only in production, not development

### Stale Sessions

**Symptom**: Sessions not being reused

**Solutions**:
- Check session staleness threshold (1 hour default)
- Verify SessionPool is not being recreated
- Review session cleanup logic

## Summary

The warmup executor provides:
- ✅ Pre-initialization of agent sessions
- ✅ Configurable warmup strategy
- ✅ Robust error handling and retry logic
- ✅ Comprehensive logging and monitoring
- ✅ Integration with session pool and task pipeline
- ✅ Full test coverage (29 tests passing)

The implementation is production-ready and follows ltmatrix best practices for async operations, error handling, and structured logging.
