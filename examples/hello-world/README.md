# Hello World Example

A minimal example demonstrating the basic workflow of ltmatrix with a single task.

## Purpose

This example shows how to use ltmatrix to:
- Generate a simple task from a goal
- Execute a single task
- Verify the result
- Commit changes to git

## Project Structure

```
hello-world/
├── README.md           # This file
├── .ltmatrix/          # Created by ltmatrix (runtime)
│   └── tasks-manifest.json
└── output/             # Created by the task
    └── hello.txt
```

## Usage

```bash
# Navigate to this example
cd examples/hello-world

# Run ltmatrix with a simple goal
ltmatrix "Create a hello.txt file containing 'Hello, World!'"
```

## Expected Output

### Console Output

```
ltmatrix v0.1.0 - Long-Time Agent Network

Goal: Create a hello.txt file containing 'Hello, World!'

Phase 1: Generate
  ✓ Generated 1 task

Phase 2: Assess
  ✓ Task complexity: Simple

Phase 3: Execute
  ✓ task-001: Create hello.txt file
     - Created output/hello.txt

Phase 4: Verify
  ✓ Verification passed

Phase 5: Commit
  ✓ Committed to branch: task-001

Summary
  Tasks completed: 1/1
  Time elapsed: 15s
  Status: SUCCESS
```

### Generated Files

**tasks-manifest.json**
```json
{
  "project_root": "/path/to/hello-world",
  "tasks": [
    {
      "id": "task-001",
      "title": "Create hello.txt file",
      "description": "Create a text file containing 'Hello, World!'",
      "status": "completed",
      "complexity": "simple"
    }
  ],
  "metadata": {
    "version": "1.0",
    "created_at": "2026-03-07T10:00:00Z",
    "modified_at": "2026-03-07T10:00:15Z"
  }
}
```

**output/hello.txt**
```
Hello, World!
```

## Key Concepts

### Single Task Flow

This example demonstrates the simplest possible workflow:
1. **Generate**: One task is created from the goal
2. **Assess**: Task is marked as "simple" complexity
3. **Execute**: Agent creates the file
4. **Verify**: Output is verified against requirements
5. **Commit**: Changes are committed to git

### Execution Modes

You can run this example in different modes:

```bash
# Fast mode - skip tests, use faster model
ltmatrix --fast "Create a hello.txt file containing 'Hello, World!'"

# Expert mode - thorough verification
ltmatrix --expert "Create a hello.txt file containing 'Hello, World!'"

# Dry run - only generate tasks, don't execute
ltmatrix --dry-run "Create a hello.txt file containing 'Hello, World!'"
```

## Next Steps

After completing this example, try:
- [multi-task-project](../multi-task-project/) - Tasks with dependencies
- [web-api-testing](../web-api-testing/) - Web project with tests
