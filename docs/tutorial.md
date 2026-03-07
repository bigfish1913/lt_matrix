# ltmatrix Tutorial

A comprehensive walkthrough for using ltmatrix - the Long-Time Agent Orchestrator.

## Introduction

ltmatrix is a high-performance agent orchestrator that automates software development tasks using AI agents. It manages the entire lifecycle of development work: planning, execution, testing, verification, and memory persistence.

### What ltmatrix Does

- **Plans**: Breaks down complex goals into manageable tasks
- **Executes**: Runs AI agents to implement each task
- **Tests**: Verifies code with automated tests
- **Remembers**: Persists decisions and context for future sessions

### When to Use ltmatrix

- Starting a new project from a description
- Adding features to existing codebases
- Refactoring with AI assistance
- Cross-platform development
- Projects requiring comprehensive testing

## Installation

### From Source

```bash
git clone https://github.com/bigfish/ltmatrix.git
cd ltmatrix
cargo install --path .
```

### Verify Installation

```bash
ltmatrix --version
ltmatrix --help
```

## Quick Start

### Your First Task

```bash
ltmatrix "Create a simple hello world CLI tool in Rust"
```

This command will:
1. Generate a task for the CLI tool
2. Assess the complexity
3. Execute the implementation
4. Run tests
5. Create a git commit

### Understanding the Pipeline

ltmatrix runs tasks through a pipeline of stages:

```
Generate → Assess → Execute → Test → Verify → Commit → Memory
```

Each stage has a specific purpose:

| Stage | Purpose |
|-------|---------|
| Generate | Analyze goal and create task list |
| Assess | Evaluate task complexity |
| Execute | Run agent to implement task |
| Test | Run automated tests |
| Verify | Confirm task completion |
| Commit | Create git commits |
| Memory | Record decisions for future reference |

## Execution Modes

### Standard Mode (Default)

```bash
ltmatrix "Build a REST API"
```

Full pipeline with testing and verification.

### Fast Mode

```bash
ltmatrix --fast "Build a REST API"
```

Skips the Test stage for quicker execution. Use for prototyping.

### Expert Mode

```bash
ltmatrix --expert "Build a REST API"
```

Adds comprehensive error handling, full test coverage, code review, and documentation.

## Working with Tasks

### Viewing Tasks

```bash
ltmatrix status
```

Shows current workspace state:

```
Workspace: ./my-project
Tasks: 5 total, 3 completed, 2 pending

task-001 ✓ Set up project structure
task-002 ✓ Create database models
task-003 ✓ Implement API routes
task-004 ○ Write tests
task-005 ○ Add documentation
```

### Resuming Interrupted Work

If execution is interrupted:

```bash
ltmatrix --resume
```

Resumes from the last checkpoint, preserving all progress.

### Task Dependencies

Complex projects have task dependencies:

```
task-001: Set up project
    ↓
task-002: Create models
    ↓
task-003: Implement API     ← depends on task-002
task-004: Write tests       ← depends on task-003
```

ltmatrix automatically resolves and executes tasks in the correct order.

## Configuration

### Global Configuration

Create `~/.ltmatrix/config.toml`:

```toml
[default]
agent = "claude"
timeout = 3600

[agents.claude]
model = "claude-sonnet-4-6"

[agents.codex]
model = "codex-4"

[workflow]
max_parallel_tasks = 3
```

### Project Configuration

Create `.ltmatrix/config.toml` in your project:

```toml
[project]
name = "my-api"
test_command = "cargo test"

[execution]
timeout_per_task = 1800
```

## Agent Backends

### Claude (Default)

```bash
ltmatrix --agent claude "task description"
```

### Codex

```bash
ltmatrix --agent codex "task description"
```

### OpenCode

```bash
ltmatrix --agent opencode "task description"
```

### KimiCode

```bash
ltmatrix --agent kimi-code "task description"
```

## Examples Walkthrough

### Example 1: Simple CLI Tool

See `examples/simple-cli-tool/` for a complete walkthrough of building a single-task CLI tool.

Key concepts:
- Basic pipeline execution
- Fast mode for prototyping
- Resume functionality

### Example 2: Multi-Task Workflow

See `examples/multi-task-workflow/` for complex workflows with dependencies.

Key concepts:
- Task dependency graphs
- Parallel execution
- Progress tracking

### Example 3: Web API with Testing

See `examples/web-api-testing/` for building tested web applications.

Key concepts:
- Test-driven development
- Integration tests
- API documentation

### Example 4: Cross-Platform Library

See `examples/cross-platform-lib/` for multi-platform development.

Key concepts:
- Conditional compilation
- Platform-specific code
- Multi-platform CI/CD

## Memory System

ltmatrix maintains persistent memory across sessions:

### How Memory Works

```
.claude/
├── memory.md          # Project-specific memory
├── workspace.json     # Current state
└── tasks/
    ├── task-001.json
    └── task-002.json
```

### Memory Content

The memory file records:
- Architectural decisions
- Code patterns used
- Important file locations
- Lessons learned

### Viewing Memory

```bash
ltmatrix memory show
```

### Clearing Memory

```bash
ltmatrix memory clear
```

## Advanced Features

### Dry Run

Preview what ltmatrix would do without executing:

```bash
ltmatrix --dry-run "Create a web server"
```

### Verbose Output

See detailed execution logs:

```bash
ltmatrix --verbose "task description"
```

### Custom Timeout

Set execution timeout:

```bash
ltmatrix --timeout 7200 "complex task"
```

### Skipping Stages

```bash
# Skip tests
ltmatrix --skip-test "task description"

# Skip commit
ltmatrix --skip-commit "task description"
```

## Troubleshooting

### Agent Timeout

If tasks timeout:

```bash
ltmatrix --timeout 7200 --resume
```

### Stuck Tasks

Force retry a stuck task:

```bash
ltmatrix --retry task-003
```

### Clearing State

Start fresh:

```bash
ltmatrix --reset
```

## Best Practices

### 1. Be Specific in Goals

Good:
```bash
ltmatrix "Create a Rust CLI tool called 'todo' that manages tasks with add, list, and complete commands using clap"
```

Too vague:
```bash
ltmatrix "Create a todo app"
```

### 2. Use Appropriate Mode

- **Fast mode**: Prototyping, quick experiments
- **Standard mode**: Normal development
- **Expert mode**: Production code, critical features

### 3. Review Generated Code

Always review and test generated code before merging.

### 4. Commit Frequently

Let ltmatrix create commits after each task:

```bash
ltmatrix --commit-each "task description"
```

### 5. Use Memory Wisely

The memory system helps maintain context across sessions. Review memory periodically:

```bash
ltmatrix memory show
```

## Getting Help

```bash
ltmatrix --help
ltmatrix <command> --help
```

## Next Steps

1. Try the [simple-cli-tool](../examples/simple-cli-tool/) example
2. Explore [multi-task workflows](../examples/multi-task-workflow/)
3. Build a [tested web API](../examples/web-api-testing/)
4. Create a [cross-platform library](../examples/cross-platform-lib/)