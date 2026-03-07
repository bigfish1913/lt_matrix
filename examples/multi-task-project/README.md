# Multi-Task Project Example

An example demonstrating task dependencies and parallel execution with ltmatrix.

## Purpose

This example shows how to use ltmatrix to:
- Work with multiple interdependent tasks
- Understand task dependency resolution
- Execute tasks in topological order
- Handle parallel task execution
- Manage task state across the pipeline

## Project Structure

```
multi-task-project/
├── README.md
├── .ltmatrix/
│   └── tasks-manifest.json    # Task state tracking
├── src/
│   ├── lib.rs                 # Core library
│   ├── config.rs              # Configuration module
│   ├── utils.rs               # Utility functions
│   └── error.rs               # Error types
├── tests/
│   └── integration_test.rs    # Integration tests
└── Cargo.toml
```

## Task Dependency Graph

```
task-001: Setup project structure
    │
    ├──► task-002: Implement config module
    │         │
    │         └──► task-004: Implement utils
    │                   │
    │                   └──► task-006: Write tests
    │
    └──► task-003: Implement error types
              │
              └──► task-005: Implement lib module
                        │
                        └──► task-006: Write tests
```

**Parallel Execution Opportunities:**
- Tasks 002 and 003 can run in parallel (both depend only on 001)
- Tasks 004 and 005 can run in parallel (depend on 002 and 003 respectively)

## Usage

```bash
# Navigate to this example
cd examples/multi-task-project

# Run ltmatrix with a complex goal
ltmatrix "Build a Rust library with config, error handling, and utilities"
```

## Expected Output

### Console Output

```
ltmatrix v0.1.0 - Long-Time Agent Network

Goal: Build a Rust library with config, error handling, and utilities

Phase 1: Generate
  ✓ Generated 6 tasks with dependencies
  ✓ Dependency graph validated

Phase 2: Assess
  ✓ Task complexity analysis complete
    - 2 simple tasks
    - 3 moderate tasks
    - 1 complex task (with subtasks)

Phase 3: Execute (Parallel: 2 workers)
  ✓ [1/6] task-001: Setup project structure
  ✓ [2/6] task-002: Implement config module  (parallel with 003)
  ✓ [2/6] task-003: Implement error types    (parallel with 002)
  ✓ [3/6] task-004: Implement utils
  ✓ [4/6] task-005: Implement lib module
  ✓ [5/6] task-006: Write tests
  ✓ [6/6] Integration complete

Phase 4: Test
  ✓ Running cargo test...
  ✓ All 12 tests passed

Phase 5: Verify
  ✓ All tasks verified complete

Phase 6: Commit
  ✓ 6 commits created on feature branches
  ✓ Squash merged to main

Summary
  Tasks completed: 6/6
  Tests passed: 12
  Time elapsed: 4m 32s
  Parallel efficiency: 45% (2 tasks ran in parallel)
  Status: SUCCESS
```

### Generated Task Manifest

```json
{
  "project_root": "/path/to/multi-task-project",
  "tasks": [
    {
      "id": "task-001",
      "title": "Setup project structure",
      "status": "completed",
      "complexity": "simple",
      "depends_on": []
    },
    {
      "id": "task-002",
      "title": "Implement config module",
      "status": "completed",
      "complexity": "moderate",
      "depends_on": ["task-001"]
    },
    {
      "id": "task-003",
      "title": "Implement error types",
      "status": "completed",
      "complexity": "simple",
      "depends_on": ["task-001"]
    },
    {
      "id": "task-004",
      "title": "Implement utils",
      "status": "completed",
      "complexity": "moderate",
      "depends_on": ["task-002"]
    },
    {
      "id": "task-005",
      "title": "Implement lib module",
      "status": "completed",
      "complexity": "moderate",
      "depends_on": ["task-003"]
    },
    {
      "id": "task-006",
      "title": "Write tests",
      "status": "completed",
      "complexity": "complex",
      "depends_on": ["task-004", "task-005"]
    }
  ]
}
```

## Key Concepts

### Dependency Resolution

Tasks are executed in topological order:
1. Tasks with no dependencies run first
2. Tasks only run when all dependencies are complete
3. Independent tasks can run in parallel

### Parallel Execution

Control parallel execution with flags:

```bash
# Disable parallel execution
ltmatrix --max-parallel 1 "Build a Rust library..."

# Allow 4 parallel tasks
ltmatrix --max-parallel 4 "Build a Rust library..."
```

### Resume After Interruption

If execution is interrupted:

```bash
# Resume from last checkpoint
ltmatrix --resume

# Check current status
ltmatrix status
```

### Handling Blocked Tasks

When a dependency fails:

```bash
# Skip blocked tasks
ltmatrix --on-blocked skip "Goal..."

# Mark as blocked and continue
ltmatrix --on-blocked block "Goal..."

# Retry with different strategy
ltmatrix --on-blocked retry "Goal..."
```

## Execution Modes Comparison

| Mode | Time | Quality | Best For |
|------|------|---------|----------|
| `--fast` | ~2 min | Basic | Prototyping |
| (default) | ~4 min | Standard | Production |
| `--expert` | ~8 min | High | Critical systems |

## Next Steps

- [web-api-testing](../web-api-testing/) - Web project with API testing
- [cross-platform-cli](../cross-platform-cli/) - Cross-platform considerations
