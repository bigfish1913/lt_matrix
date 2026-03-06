# Retry Scenario Session Reuse Implementation

## Overview

This implementation adds retry tracking to the Task model and modifies the AgentPool to detect and reuse existing sessions on retry, ensuring session continuity across task retry attempts.

## Changes Made

### 1. Task Model Enhancements (`src/models/mod.rs`)

**Added field:**
- `session_id: Option<String>` - Tracks which session was used for task execution

**Added methods:**
- `has_session() -> bool` - Check if task has a session ID
- `get_session_id() -> Option<&str>` - Get the session ID
- `set_session_id(session_id: impl Into<String>)` - Set the session ID
- `clear_session_id()` - Clear the session ID (e.g., if session becomes stale)
- `prepare_retry()` - Increment retry count and prepare task for retry (preserves session_id)

### 2. SessionPool Enhancements (`src/agent/pool.rs`)

**Added methods:**

#### `get_for_retry(&mut self, session_id: &str) -> Option<&mut MemorySession>`
- Retrieves a session by ID for retry scenarios
- Marks the session as accessed (increments reuse_count, updates last_accessed)
- Returns `None` if session is not found or is stale

#### `get_or_create_for_task(&mut self, task: &mut Task) -> &MemorySession`
- Main API for task execution with session tracking
- If task has a session_id and the session exists and is not stale, reuse it
- Otherwise, create a new session and associate it with the task
- Automatically handles stale session detection and cleanup

### 3. Integration Tests (`tests/retry_session_reuse_test.rs`)

Comprehensive test suite covering:
- Session reuse on retry
- Multiple retries with session continuity
- Stale session handling
- Multiple tasks sharing sessions
- Full task lifecycle with session tracking
- Non-existent session ID handling
- Session helper methods

## Key Features

### 1. Automatic Session Reuse

When a task is retried, the system automatically reuses the same session:

```rust
let mut pool = SessionPool::new();
let mut task = Task::new("task-1", "Test", "Description");

// First execution
let session1 = pool.get_or_create_for_task(&mut task);
let session_id = task.get_session_id().unwrap();

// Task fails
task.status = TaskStatus::Failed;
task.prepare_retry();

// Retry - reuses same session
let session2 = pool.get_or_create_for_task(&mut task);
assert_eq!(session2.session_id(), session_id);
assert_eq!(session2.reuse_count(), 1); // Incremented
```

### 2. Stale Session Detection

Sessions older than 1 hour are considered stale and not reused:

```rust
// Stale sessions are automatically detected
// A new session is created if the old one is stale
let session = pool.get_or_create_for_task(&mut task);
```

### 3. Session Continuity Across Multiple Retries

The same session is reused across multiple retry attempts:

```rust
for retry_num in 1..=5 {
    task.prepare_retry();
    let session = pool.get_or_create_for_task(&mut task);
    assert_eq!(session.reuse_count(), retry_num);
}
```

### 4. Task-Session Association

Tasks maintain their session association through the retry cycle:

```rust
// Session ID is preserved across retries
task.prepare_retry();
assert_eq!(task.get_session_id(), Some(original_session_id));
```

## Design Decisions

1. **Session Pooling by (agent_name, model)**: Sessions are keyed by agent name and model, allowing multiple tasks to share the same session if they use the same agent.

2. **Automatic Staleness Handling**: Stale sessions are automatically detected and not reused, with a new session created instead.

3. **Minimal API Surface**: The main API is `get_or_create_for_task()` which handles all the complexity of session management.

4. **Preserve Session on Retry**: The `prepare_retry()` method preserves the session_id, ensuring continuity across retry attempts.

## Test Coverage

- **Unit Tests** (src/agent/pool.rs): 14 tests covering all pool methods and retry scenarios
- **Integration Tests** (tests/retry_session_reuse_test.rs): 10 tests covering end-to-end retry behavior

All tests pass successfully (585 total tests in the library).

## Usage Example

```rust
use ltmatrix::agent::pool::SessionPool;
use ltmatrix::models::Task;

let mut pool = SessionPool::new();
let mut task = Task::new("task-1", "Implement feature", "Description");

// Execute task
let session = pool.get_or_create_for_task(&mut task);

// Task execution fails
task.status = TaskStatus::Failed;
task.error = Some("Network error".to_string());

// Prepare for retry
task.prepare_retry();

// Retry with same session
let session = pool.get_or_create_for_task(&mut task);
// Session is reused, maintaining context
```

## Benefits

1. **Context Preservation**: Agent maintains conversation context across retries
2. **Performance**: Reuses existing sessions instead of creating new ones
3. **Automatic Cleanup**: Stale sessions are automatically detected and handled
4. **Transparent API**: Simple, intuitive API for task execution with retries
5. **Testable**: Comprehensive test coverage ensures reliability

## Future Enhancements

Potential areas for future improvement:
- Configurable stale timeout (currently hardcoded to 1 hour)
- Session cleanup based on last accessed time
- Session sharing across dependent tasks
- Metrics/observability for session reuse statistics
