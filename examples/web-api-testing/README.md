# Web API Testing Example

This example demonstrates building a web API with comprehensive testing, including E2E tests.

## Purpose

Learn how ltmatrix handles web projects with testing requirements, including API testing, integration tests, and E2E scenarios.

## Command

```bash
ltmatrix "Create a task management REST API with CRUD operations. Include unit tests, integration tests, and API documentation."
```

## Expected Behavior

### 1. Generate Stage
The agent generates tasks for a complete web API:

```json
{
  "tasks": [
    {
      "id": "task-001",
      "title": "Create API project structure",
      "description": "Set up Axum web framework project",
      "status": "pending"
    },
    {
      "id": "task-002",
      "title": "Implement task model and storage",
      "description": "Define Task model with in-memory storage",
      "status": "pending"
    },
    {
      "id": "task-003",
      "title": "Create CRUD endpoints",
      "description": "GET, POST, PUT, DELETE endpoints for tasks",
      "status": "pending"
    },
    {
      "id": "task-004",
      "title": "Write unit tests",
      "description": "Unit tests for model and business logic",
      "status": "pending"
    },
    {
      "id": "task-005",
      "title": "Write integration tests",
      "description": "API endpoint integration tests",
      "status": "pending"
    }
  ]
}
```

### 2. Test Stage (Standard Mode)
Comprehensive testing is performed:

```
[Test] Running tests...

[Unit Tests]
  ✓ test_create_task
  ✓ test_update_task
  ✓ test_delete_task
  ✓ test_task_validation
  ✓ test_task_not_found
  Passed: 5/5

[Integration Tests]
  ✓ test_get_all_tasks_empty
  ✓ test_create_and_get_task
  ✓ test_update_existing_task
  ✓ test_delete_task
  ✓ test_api_error_responses
  Passed: 5/5

[API Contract Tests]
  ✓ Response format matches OpenAPI spec
  ✓ Error codes are correct
  Passed: 2/2

Total: 12 tests passed
```

## Expected Output

```
$ ltmatrix "Create a task management REST API..."

Pipeline: Generate → Assess → Execute → Test → Verify → Commit → Memory
Mode: Standard

[Generate] Analyzing goal...
  ✓ Generated 5 tasks

[Assess] Evaluating task complexity...
  ✓ task-001: Simple
  ✓ task-002: Medium
  ✓ task-003: Medium
  ✓ task-004: Medium (testing focus)
  ✓ task-005: Complex (E2E testing)

[Execute] Implementing tasks...
  [task-001] Create API project structure...
    ✓ Added axum, tokio, serde dependencies
    ✓ Created src/main.rs with router setup

  [task-002] Implement task model and storage...
    ✓ Created src/models/task.rs
    ✓ Created src/store/memory.rs

  [task-003] Create CRUD endpoints...
    ✓ Created src/routes/tasks.rs
    ✓ Created src/handlers/

  [task-004] Write unit tests...
    ✓ Created src/models/task_test.rs
    ✓ Created tests/unit/

  [task-005] Write integration tests...
    ✓ Created tests/integration/api_test.rs
    ✓ Added test utilities

[Test] Running tests...
  ✓ Unit tests passed (5/5)
  ✓ Integration tests passed (5/5)
  ✓ All tests passed in 12 seconds

[Verify] Reviewing completion...
  ✓ API responds on localhost:3000
  ✓ OpenAPI spec generated
  ✓ Test coverage: 87%

[Commit] Creating git commit...
  ✓ Committed: feat: Add task management API with tests

[Memory] Recording decisions...
  ✓ Updated .claude/memory.md

Summary: 5 tasks completed in 5 minutes
```

## E2E Testing with Playwright

For web projects, ltmatrix can generate E2E tests:

```bash
ltmatrix --test-e2e "Create a task management web app with frontend"
```

Generated E2E test structure:

```javascript
// tests/e2e/task-management.spec.ts
import { test, expect } from '@playwright/test';

test('create and complete a task', async ({ page }) => {
  await page.goto('/');

  // Create task
  await page.fill('[data-testid="task-input"]', 'New task');
  await page.click('[data-testid="add-task-btn"]');

  // Verify task appears
  await expect(page.locator('.task-item')).toContainText('New task');

  // Complete task
  await page.click('[data-testid="complete-task-btn"]');
  await expect(page.locator('.task-item.completed')).toBeVisible();
});
```

## Test Configuration

```toml
# .ltmatrix/test.toml
[test]
unit_test_command = "cargo test --lib"
integration_test_command = "cargo test --test '*'"
e2e_test_command = "npx playwright test"
coverage_threshold = 80
```

## Files Generated

```
task-api/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── models/
│   │   └── task.rs
│   ├── routes/
│   │   └── tasks.rs
│   ├── handlers/
│   │   └── mod.rs
│   └── store/
│       └── memory.rs
├── tests/
│   ├── integration/
│   │   └── api_test.rs
│   └── common/
│       └── mod.rs
├── docs/
│   └── openapi.yaml
└── .claude/
    └── memory.md
```

## Expert Mode for Production APIs

```bash
ltmatrix --expert "Create a task management REST API..."
```

Expert mode adds:
- Authentication/authorization
- Rate limiting
- Request validation
- Error handling middleware
- Comprehensive error responses
- API versioning
- Database migrations
- 95%+ test coverage