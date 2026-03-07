# Test Fixtures

This directory contains test fixtures for the ltmatrix test suite.

## Directory Structure

```
tests/fixtures/
├── tasks/                    # Task list fixtures
│   ├── simple_task_list.json
│   ├── complex_task_list.json
│   ├── failed_tasks_list.json
│   ├── circular_dependency_list.json
│   └── diamond_dependency_list.json
├── responses/                # Agent response fixtures
│   ├── generate_success.json
│   ├── execute_success.json
│   ├── error_timeout.json
│   ├── verify_success.json
│   └── verify_failure.json
├── projects/                 # Sample project structures
│   ├── rust_basic/
│   ├── node_basic/
│   └── python_basic/
├── git/                      # Git repository setup scripts
│   └── setup_test_repo.sh
├── mocks/                    # Mock agent implementations
│   └── mod.rs
└── README.md                 # This file
```

## Task Fixtures

### simple_task_list.json
A basic linear dependency chain with 3 tasks:
- task-001 (no dependencies)
- task-002 (depends on task-001)
- task-003 (depends on task-002)

Use for: Testing basic pipeline execution, dependency resolution.

### complex_task_list.json
A more complex task structure with:
- Completed, in-progress, blocked, and pending tasks
- Nested subtasks (up to 3 levels)
- Session IDs for retry reuse
- Parent-child session relationships

Use for: Testing state transitions, session inheritance, subtask handling.

### failed_tasks_list.json
Tasks in various failure states:
- Completed tasks
- Failed tasks with error messages
- Blocked tasks due to failed dependencies
- Pending tasks

Use for: Testing error handling, retry logic, state recovery.

### circular_dependency_list.json
A cycle of dependencies: A → B → C → A

Use for: Testing dependency validation, cycle detection.

### diamond_dependency_list.json
Diamond pattern for parallel execution:
```
    root
    /   \
 left  right
    \   /
   merge
```

Use for: Testing parallel execution, dependency resolution.

## Response Fixtures

### generate_success.json
Sample successful response from the generate stage with structured task data.

### execute_success.json
Sample successful response from the execute stage with file modification details.

### error_timeout.json
Sample error response when execution times out.

### verify_success.json
Sample successful verification response.

### verify_failure.json
Sample failed verification response with unmet criteria and suggestions.

## Project Fixtures

### rust_basic/
A minimal Rust project with:
- Cargo.toml with basic dependencies
- src/main.rs with a simple main function
- Basic test structure

Use for: Testing Rust project detection, Cargo command execution.

### node_basic/
A minimal Node.js project with:
- package.json with Jest for testing
- Basic index.js entry point

Use for: Testing Node.js project detection, npm command execution.

### python_basic/
A minimal Python project with:
- pyproject.toml configuration
- pytest configuration

Use for: Testing Python project detection, pytest command execution.

## Git Fixtures

### setup_test_repo.sh
Bash script that creates a test git repository with:
- Initial commit
- Feature branch with commits
- Merged changes
- Sample tags (v1.0.0)

Usage:
```bash
./tests/fixtures/git/setup_test_repo.sh /path/to/target
```

## Mock Agents

The `mocks/mod.rs` file provides mock implementations of the `AgentBackend` trait:

### MockAgent
Configurable mock that returns preset responses.

```rust
let mut mock = MockAgent::new();
mock.set_response("execute", MockResponse::success("Test output"));

let config = ExecutionConfig::default();
let response = mock.execute("test prompt", &config).await?;
```

### RecordingMockAgent
Records all calls for verification in tests.

```rust
let mock = RecordingMockAgent::new();
// ... execute operations ...
let calls = mock.get_calls();
assert_eq!(calls.len(), 2);
```

### FailingMockAgent
Always returns errors for testing error handling.

```rust
let mock = FailingMockAgent::execution_failed("Always fails");
```

### DelayedMockAgent
Simulates slow responses for timeout testing.

```rust
let mock = DelayedMockAgent::from_millis(100);
```

### MockResponse
Response configuration enum:

```rust
// Success response
MockResponse::success("Output text")

// Success with structured data
MockResponse::success_with_data("Output", json!({"key": "value"}))

// Failure response
MockResponse::failure("Error message")

// Timeout
MockResponse::timeout()

// Delayed response
MockResponse::delayed(MockResponse::success("Delayed"), 100)

// Sequence of responses
MockResponse::Sequence {
    responses: vec![
        MockResponse::success("First"),
        MockResponse::success("Second"),
    ]
}
```

### MockAgentBuilder
Builder pattern for creating mock agents:

```rust
let mock = MockAgentBuilder::new()
    .name("test-agent")
    .model("test-model")
    .execute_response(MockResponse::success("Built response"))
    .healthy(true)
    .build();
```

## Usage in Tests

### Loading Task Fixtures

```rust
use std::fs;

fn load_tasks_fixture(name: &str) -> Vec<Task> {
    let path = format!("tests/fixtures/tasks/{}.json", name);
    let content = fs::read_to_string(&path).expect("Failed to read fixture");
    let json: serde_json::Value = serde_json::from_str(&content).expect("Invalid JSON");
    // Parse tasks from JSON
    // ...
}
```

### Using Project Fixtures

```rust
use std::path::PathBuf;

fn get_rust_project_fixture() -> PathBuf {
    PathBuf::from("tests/fixtures/projects/rust_basic")
}

#[test]
fn test_rust_project_detection() {
    let project_path = get_rust_project_fixture();
    let detected = detect_project_type(&project_path);
    assert_eq!(detected, Some(ProjectType::Rust));
}
```

### Using Mock Agents

```rust
use ltmatrix::testing::mocks::{MockAgent, MockResponse};

#[tokio::test]
async fn test_with_mock() {
    let mut mock = MockAgent::new();
    mock.set_response("generate", MockResponse::success_fixture("tasks/simple_task_list"));

    let config = ExecutionConfig::default();
    let response = mock.execute("Generate tasks", &config).await?;

    assert!(response.is_complete);
}
```

## Adding New Fixtures

When adding new fixtures:

1. **Task fixtures**: Add JSON file to `tasks/` directory following the existing schema
2. **Response fixtures**: Add JSON file to `responses/` directory with `output`, `structured_data`, `is_complete`, and `error` fields
3. **Project fixtures**: Create a new directory in `projects/` with the minimal required files
4. **Mock implementations**: Add to `mocks/mod.rs` with appropriate documentation

## Fixture Naming Conventions

- Task fixtures: `<description>_task_list.json`
- Response fixtures: `<stage>_<status>.json`
- Project fixtures: `<language>_<complexity>/`
- Mock agents: `<description>_mock_agent.rs` (if split into separate files)
