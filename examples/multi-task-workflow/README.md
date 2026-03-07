# Multi-Task Workflow Example

This example demonstrates a complex workflow with multiple tasks that have dependencies.

## Purpose

Learn how ltmatrix handles task dependencies, parallel execution, and workflow orchestration.

## Command

```bash
ltmatrix "Build a REST API with user authentication. Include: database models, authentication middleware, user registration/login endpoints, and API documentation."
```

## Task Dependency Graph

The agent will analyze the goal and break it into tasks with dependencies:

```
task-001: Set up project structure
    ↓
task-002: Create database models
    ↓
task-003: Implement authentication middleware  ←── depends on task-001, task-002
    ↓
task-004: Create user registration endpoint    ←── depends on task-002, task-003
task-005: Create user login endpoint           ←── depends on task-002, task-003
    ↓
task-006: Generate API documentation           ←── depends on task-004, task-005
```

## Expected Behavior

### 1. Generate Stage
```json
{
  "tasks": [
    {
      "id": "task-001",
      "title": "Set up project structure",
      "description": "Initialize Rust project with dependencies",
      "status": "pending",
      "dependencies": []
    },
    {
      "id": "task-002",
      "title": "Create database models",
      "description": "Define User model and database schema",
      "status": "pending",
      "dependencies": ["task-001"]
    },
    {
      "id": "task-003",
      "title": "Implement authentication middleware",
      "description": "JWT-based auth middleware",
      "status": "pending",
      "dependencies": ["task-001", "task-002"]
    }
  ]
}
```

### 2. Assess Stage
Tasks are assessed for complexity:
- task-001: Simple (project scaffold)
- task-002: Medium (requires database knowledge)
- task-003: Complex (security considerations)

### 3. Execute Stage
Tasks execute in dependency order:
1. Project structure is created first
2. Database models are defined
3. Auth middleware and endpoints are implemented in parallel where possible

## Expected Output

```
$ ltmatrix "Build a REST API with user authentication..."

Pipeline: Generate → Assess → Execute → Test → Verify → Commit → Memory
Mode: Standard

[Generate] Analyzing goal...
  ✓ Generated 6 tasks with 8 dependency edges

[Assess] Evaluating task complexity...
  ✓ task-001: Simple
  ✓ task-002: Medium
  ✓ task-003: Complex
  ✓ task-004: Medium
  ✓ task-005: Medium
  ✓ task-006: Simple

[Execute] Processing task graph...
  [task-001] Set up project structure...
    ✓ Created Cargo.toml with dependencies
    ✓ Created src/lib.rs

  [task-002] Create database models...
    ✓ Created src/models/user.rs
    ✓ Created src/db/schema.rs

  [task-003] Implement authentication middleware...
    ✓ Created src/middleware/auth.rs
    ✓ Created src/utils/jwt.rs

  [task-004, task-005] Implementing endpoints (parallel)...
    ✓ Created src/routes/auth.rs
    ✓ Created src/routes/user.rs

  [task-006] Generate API documentation...
    ✓ Created docs/api.md

[Test] Running tests...
  ✓ Unit tests passed (12/12)
  ✓ Integration tests passed (4/4)

[Verify] Reviewing completion...
  ✓ All endpoints respond correctly
  ✓ Authentication flow works
  ✓ Documentation is complete

[Commit] Creating git commits...
  ✓ Committed 6 tasks in 3 commits

[Memory] Recording decisions...
  ✓ Updated .claude/memory.md

Summary: 6 tasks completed in 8 minutes
```

## Workflow Features Demonstrated

### Dependency Resolution
- Tasks execute only after their dependencies complete
- Independent tasks can run in parallel

### Progress Tracking
```
[Execute] Processing task graph...
  ████████░░░░░░░░ 3/6 tasks complete
  Running: task-003, task-004
  Pending: task-005, task-006
```

### Resume with Dependencies
```bash
# If interrupted
$ ltmatrix --resume
[Resume] Loading workspace state...
  ✓ Found 6 tasks (2 completed, 4 pending)
  ✓ Dependency graph restored
[Execute] Resuming from task-003...
```

## Files Generated

```
auth-api/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── models/
│   │   └── user.rs
│   ├── db/
│   │   └── schema.rs
│   ├── middleware/
│   │   └── auth.rs
│   ├── routes/
│   │   ├── auth.rs
│   │   └── user.rs
│   └── utils/
│       └── jwt.rs
├── tests/
│   └── integration_test.rs
└── docs/
    └── api.md
```

## Configuration for Complex Workflows

```toml
# ~/.ltmatrix/config.toml
[workflow]
max_parallel_tasks = 3
dependency_check_interval = 5

[execution]
timeout_per_task = 1800  # 30 minutes
retry_failed_tasks = true
```