# Simple CLI Tool Example

This example demonstrates the most basic use case: building a single-task command-line tool.

## Goal

Build a simple CLI tool that greets users with customizable messages.

## Command

```bash
ltmatrix "Create a simple CLI tool called 'greet' that takes a name as an argument and prints a greeting. Support a --formal flag for formal greetings."
```

## Expected Behavior

### 1. Generate Stage
The agent will analyze the goal and generate a single task:
```json
{
  "tasks": [
    {
      "id": "task-001",
      "title": "Create greet CLI tool",
      "description": "Implement a CLI tool with clap that greets users",
      "status": "pending",
      "complexity": "simple"
    }
  ]
}
```

### 2. Assess Stage
The task is assessed as "Simple" complexity - no subtask decomposition needed.

### 3. Execute Stage
The agent creates:
- `src/main.rs` with clap-based argument parsing
- `Cargo.toml` with dependencies

### 4. Test Stage (Standard Mode)
Since this is a simple tool, tests are minimal:
- Unit test for greeting generation
- Integration test for CLI parsing

### 5. Verify Stage
The agent verifies:
- Binary compiles successfully
- CLI arguments work correctly
- Help text is generated

### 6. Commit Stage
Creates a commit with the new files.

### 7. Memory Stage
Records the architecture decision in `.claude/memory.md`:
```markdown
## [task-001] CLI Architecture
- Using clap 4.x for argument parsing
- Simple binary with no subcommands
```

## Expected Output

```
$ ltmatrix "Create a simple CLI tool called 'greet'..."

Pipeline: Generate → Assess → Execute → Test → Verify → Commit → Memory
Mode: Standard

[Generate] Analyzing goal...
  ✓ Generated 1 task

[Assess] Evaluating task complexity...
  ✓ task-001: Simple complexity

[Execute] Implementing task-001: Create greet CLI tool...
  ✓ Created src/main.rs
  ✓ Created Cargo.toml
  ✓ Binary compiles successfully

[Test] Running tests...
  ✓ Unit tests passed (2/2)
  ✓ Integration tests passed (1/1)

[Verify] Reviewing completion...
  ✓ CLI arguments work correctly
  ✓ Help text generated

[Commit] Creating git commit...
  ✓ Committed: feat: Add greet CLI tool

[Memory] Recording decisions...
  ✓ Updated .claude/memory.md

Summary: 1 task completed in 45 seconds
```

## Fast Mode Example

For quick prototyping, use fast mode:

```bash
ltmatrix --fast "Create a simple CLI tool called 'greet'..."
```

Fast mode skips the Test stage, completing in ~30 seconds.

## Expert Mode Example

For production-ready code:

```bash
ltmatrix --expert "Create a simple CLI tool called 'greet'..."
```

Expert mode adds:
- Comprehensive error handling
- Full test suite with edge cases
- Code review stage
- Documentation comments

## Files Generated

```
greet/
├── Cargo.toml
├── src/
│   └── main.rs
├── tests/
│   └── integration_test.rs
├── .gitignore
└── .claude/
    └── memory.md
```

## Resume Example

If the execution is interrupted:

```bash
# First run (interrupted during Execute)
$ ltmatrix "Create a simple CLI tool..."
[Execute] Implementing task-001...
^C Interrupted

# Resume from where it left off
$ ltmatrix --resume
[Resume] Loading workspace state...
  ✓ Found 1 task (0 completed, 1 pending)
[Execute] Implementing task-001...
  ✓ Resuming from checkpoint
```

## Custom Agent Backend

Use a different agent:

```bash
# Using OpenCode
ltmatrix --agent opencode "Create a simple CLI tool..."

# Using KimiCode
ltmatrix --agent kimi-code "Create a simple CLI tool..."
```

## Configuration

Example `~/.ltmatrix/config.toml`:

```toml
[default]
agent = "claude"

[agents.claude]
model = "claude-sonnet-4-6"
timeout = 3600
```