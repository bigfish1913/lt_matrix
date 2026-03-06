# Dry-Run Mode Implementation

## Overview

The dry-run mode provides a preview of the task execution plan without making any changes to the codebase or running any AI agents. This allows users to review and validate the proposed approach before committing resources to execution.

## Features

### Core Functionality

1. **Task Generation**: Simulates the Generate stage to create a task breakdown
2. **Complexity Assessment**: Evaluates each task's complexity using Claude
3. **Execution Planning**: Creates an optimized execution plan with dependency resolution
4. **Statistics Calculation**: Provides comprehensive metrics about the task graph
5. **Multiple Output Formats**: Supports both human-readable text and JSON output

### What Dry-Run Does

- ✅ Executes Generate stage (task breakdown)
- ✅ Executes Assess stage (complexity evaluation)
- ✅ Creates execution plan with dependencies
- ✅ Calculates critical path and parallelization opportunities
- ✅ Displays comprehensive statistics and summary
- ✅ Validates task dependencies and detects cycles

### What Dry-Run Does NOT Do

- ❌ Execute any code changes
- ❌ Run any AI agents for implementation
- ❌ Modify files in the workspace
- ❌ Run tests or verification
- ❌ Make git commits
- ❌ Write to project memory

## Usage

### Command Line

```bash
# Basic dry-run
ltmatrix --dry-run "build a REST API"

# Dry-run with JSON output
ltmatrix --dry-run --output json "implement authentication"

# Dry-run with specific execution mode
ltmatrix --dry-run --expert "design microservices architecture"

# Dry-run with custom log level
ltmatrix --dry-run --log-level debug "refactor database layer"
```

### Programmatic Usage

```rust
use ltmatrix::dryrun::{run_dry_run, DryRunConfig};
use ltmatrix::models::ExecutionMode;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let goal = "build a web application";
    let config = DryRunConfig {
        execution_mode: ExecutionMode::Standard,
        json_output: false,
        ..Default::default()
    };

    let result = run_dry_run(goal, &config).await?;

    // Access results programmatically
    println!("Total tasks: {}", result.statistics.total_tasks);
    println!("Execution depth: {}", result.statistics.execution_depth);

    Ok(())
}
```

## Output Formats

### Text Output (Default)

```
╔═══════════════════════════════════════════════════════════════╗
║           LTMATRIX - DRY RUN MODE                            ║
╚═══════════════════════════════════════════════════════════════╝

Goal: build a REST API

Summary:
  Total Tasks: 5
  Execution Depth: 3 levels
  Critical Path Length: 3 tasks
  Parallelizable Tasks: 2

Complexity Breakdown:
  Simple: 2 (fast model)
  Moderate: 2 (standard model)
  Complex: 1 (smart model)

Execution Plan:
  Level 1 (1 tasks):
    ⚡ task-1 - Analyze requirements

  Level 2 (2 tasks):
    ⚙️ task-2 - Design solution
    ⚙️ task-3 - Plan database schema

  Level 3 (2 tasks):
    🔧 task-4 - Implement core functionality
    🔧 task-5 - Write tests

Critical Path:
  1. task-1 (Analyze requirements)
  2. task-2 (Design solution)
  3. task-4 (Implement core functionality)

Notice:
  This is a DRY RUN - no changes will be made
  Remove --dry-run flag to execute the plan
```

### JSON Output

```json
{
  "goal": "build a REST API",
  "summary": {
    "total_tasks": 5,
    "execution_depth": 3,
    "critical_path_length": 3,
    "parallelizable_tasks": 2
  },
  "complexity_breakdown": {
    "simple": 2,
    "moderate": 2,
    "complex": 1
  },
  "execution_plan": {
    "max_depth": 3,
    "execution_order": ["task-1", "task-2", "task-3", "task-4", "task-5"],
    "critical_path": ["task-1", "task-2", "task-4"],
    "parallelizable_tasks": ["task-3", "task-5"],
    "execution_levels": [
      [
        {
          "id": "task-1",
          "title": "Analyze requirements",
          "description": "...",
          "complexity": "Simple",
          "depends_on": [],
          "subtasks_count": 0
        }
      ],
      ...
    ]
  },
  "tasks": [...]
}
```

## Architecture

### Module Structure

```
src/dryrun/
├── mod.rs              # Main dry-run implementation
├── tests/              # Unit tests
examples/
├── dry_run_example.rs  # Usage example
tests/
├── dryrun_integration_test.rs  # Integration tests
```

### Key Components

1. **DryRunConfig**: Configuration for dry-run execution
   - `execution_mode`: Fast/Standard/Expert mode
   - `assess_config`: Assessment stage configuration
   - `json_output`: Whether to output JSON instead of text

2. **DryRunResult**: Results from dry-run execution
   - `tasks`: Generated and assessed tasks
   - `execution_plan`: Optimized execution plan
   - `statistics`: Comprehensive statistics

3. **DryRunStatistics**: Statistical summary
   - Task counts by complexity
   - Execution depth and critical path
   - Parallelization opportunities

### Algorithm

1. **Task Generation** (placeholder)
   - Currently simulates task generation
   - Will be replaced by actual Generate stage

2. **Complexity Assessment**
   - Calls `assess_tasks()` from pipeline module
   - Evaluates each task's complexity
   - Splits complex tasks into subtasks

3. **Execution Planning**
   - Calls `schedule_tasks()` from tasks module
   - Performs topological sorting
   - Detects circular dependencies
   - Calculates critical path
   - Identifies parallelizable tasks

4. **Statistics Calculation**
   - Analyzes complexity distribution
   - Computes execution metrics
   - Generates summary information

5. **Result Display**
   - Formats output (text or JSON)
   - Displays execution plan
   - Shows statistics and summary

## Testing

### Unit Tests

- Test configuration defaults
- Test task generation simulation
- Test statistics calculation
- Test output formatting

### Integration Tests

- Test complete dry-run functionality
- Test different execution modes
- Test JSON vs text output
- Test dependency resolution
- Test critical path identification
- Test parallel execution levels

### Running Tests

```bash
# Run all dry-run tests
cargo test --test dryrun_integration_test

# Run specific test
cargo test test_dry_run_basic_functionality

# Run with output
cargo test --test dryrun_integration_test -- --nocapture
```

## Future Enhancements

1. **Integration with Generate Stage**
   - Replace placeholder with actual Generate stage implementation
   - Use Claude to generate real task breakdowns

2. **Interactive Mode**
   - Allow users to modify the plan before execution
   - Support task reordering and dependency editing

3. **Plan Comparison**
   - Compare different execution modes
   - Show differences between plans

4. **Historical Plans**
   - Save and retrieve previous dry-run results
   - Track plan evolution over time

5. **Export Options**
   - Save plans to file
   - Generate Mermaid diagrams
   - Create visual representations

## Dependencies

- `anyhow`: Error handling
- `serde_json`: JSON serialization
- `tracing`: Structured logging
- `console`: Terminal styling
- `tokio`: Async runtime

## Integration Points

- **Pipeline Module**: Uses `assess_tasks()` for complexity evaluation
- **Tasks Module**: Uses `schedule_tasks()` for execution planning
- **Models Module**: Uses core data structures (Task, ExecutionMode, etc.)
- **CLI Module**: Responds to --dry-run flag from command-line arguments

## Error Handling

The dry-run mode handles errors gracefully:

- **Missing Dependencies**: Reports missing task dependencies with clear error messages
- **Circular Dependencies**: Detects and reports cycles in the dependency graph
- **Assessment Failures**: Falls back to default complexity on assessment errors
- **Planning Failures**: Provides detailed context for scheduling failures

All errors use `anyhow::Context` to provide actionable error messages.

## Performance Considerations

- Dry-run mode is lightweight compared to full execution
- Only runs Generate and Assess stages (no code execution)
- Complexity assessment may take time depending on task count
- Suitable for frequent use during planning phases

## Limitations

1. **Placeholder Generation**: Currently uses simulated task generation
2. **No Actual Execution**: Does not make any changes to the codebase
3. **Static Assessment**: Complexity assessment is based on prompts, not actual code
4. **Single Session**: Does not persist results across sessions

## See Also

- [Pipeline Module](../pipeline/README.md): Full pipeline implementation
- [Tasks Module](../tasks/README.md): Task scheduling and execution
- [CLI Module](../cli/README.md): Command-line interface
- [Models Module](../models/README.md): Core data structures
