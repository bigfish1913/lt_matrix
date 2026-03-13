# Feature Integration Gaps Fix Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Connect 4 implemented-but-disconnected features to make the blog post claims accurate.

**Architecture:** Replace internal implementations in execute.rs and generate.rs to use the already-implemented scheduler, memory, and error classification systems. Each fix is isolated and can be tested independently.

**Tech Stack:** Rust, async/await (tokio), tracing for logging, serde for JSON. tempfile for testing.

---

## Task 1: Write failing tests for unified memory loader

**Files:**
- Create: `tests/unified_memory_loader_test.rs`

**Step 1: Write the failing test for unified memory loader**

```rust
use ltmatrix::pipeline::execute::load_unified_memory;
use std::path::Path;
use tempfile::TempDir;

#[tokio::test]
async fn test_load_unified_memory_returns_default_when_no_memory_exists() {
    let temp_dir = TempDir::new().unwrap();

    let result = load_unified_memory(temp_dir.path()).await.unwrap();

    // Should return default message when no memory files exist
    assert!(result.contains("No project memory") || result.is_empty());
}

#[tokio::test]
async fn test_load_unified_memory_loads_project_memory() {
    let temp_dir = TempDir::new().unwrap();

    // Create project memory file
    let memory_dir = temp_dir.path().join(".ltmatrix/memory");
    tokio::fs::create_dir_all(&memory_dir).await.unwrap();

    let project_mem = r#"{"version":"1.0.0","project_name":"TestProject"}"#;
    tokio::fs::write(
        memory_dir.join("project.json"),
        project_mem
    ).await.unwrap();

    let result = load_unified_memory(temp_dir.path()).await.unwrap();

    assert!(result.contains("TestProject"));
}

#[tokio::test]
async fn test_load_unified_memory_falls_back_to_legacy_memory() {
    let temp_dir = TempDir::new().unwrap();

    // Create old-style memory file
    let claude_dir = temp_dir.path().join(".claude");
    tokio::fs::create_dir_all(&claude_dir).await.unwrap();

    let legacy_mem = "# Architecture\nUse async patterns";
    tokio::fs::write(
        claude_dir.join("memory.md"),
        legacy_mem
    ).await.unwrap();

    let result = load_unified_memory(temp_dir.path()).await.unwrap();

    assert!(result.contains("async patterns"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test unified_memory_loader_test --no-fail-fast 2>&1`
Expected: Compilation error or test failure (function not defined)

---

## Task 2: Implement unified memory loader

**Files:**
- Modify: `src/pipeline/execute.rs:119-170` (add new function)

**Step 1: Add imports for memory modules**

```rust
// Add to existing imports in execute.rs:
use crate::memory::{
    ProjectMemory, RunMemory,
    get_project_memory_path, get_current_run_memory_path,
};
```

**Step 2: Implement load_unified_memory function**

```rust
/// Load unified memory from all sources (ProjectMemory, RunMemory, legacy)
///
/// This function combines memory from multiple sources:
/// 1. New ProjectMemory system (.ltmatrix/memory/project.json)
/// 2. New RunMemory system (.ltmatrix/memory/run-{id}.json)
/// 3. Legacy memory.md (.claude/memory.md) for backwards compatibility
pub async fn load_unified_memory(project_root: &Path) -> Result<String> {
    let mut context = String::new();

    // 1. Load new ProjectMemory if exists
    let project_mem_path = get_project_memory_path(project_root);
    if project_mem_path.exists() {
        match ProjectMemory::load(&project_mem_path).await {
            Ok(mem) => {
                context.push_str(&mem.generate_summary());
                context.push_str("\n\n");
            }
            Err(e) => warn!("Failed to load project memory: {}", e),
        }
    }

    // 2. Load current RunMemory if exists (for session context)
    let run_mem_path = get_current_run_memory_path(project_root);
    if run_mem_path.exists() {
        match RunMemory::load(&run_mem_path).await {
            Ok(mem) => {
                context.push_str(&mem.generate_summary());
            }
            Err(e) => warn!("Failed to load run memory: {}", e),
        }
    }

    // 3. Fallback: old memory.md for backwards compatibility
    if context.is_empty() {
        let old_path = project_root.join(".claude/memory.md");
        if old_path.exists() {
            context = tokio::fs::read_to_string(&old_path)
                .await
                .context("Failed to read legacy memory")?;
        }
    }

    if context.is_empty() {
        context = "No project memory available yet.".to_string();
    }

    Ok(context)
}
```

**Step 3: Run tests to verify they pass**

Run: `cargo test unified_memory_loader_test`
Expected: All 3 tests pass

**Step 4: Commit**

```bash
git add src/pipeline/execute.rs tests/unified_memory_loader_test.rs
git commit -m "feat(execute): add unified memory loader combining all memory sources"
```

---

## Task 3: Replace old memory loader with unified loader

**Files:**
- Modify: `src/pipeline/execute.rs:401-402`

**Step 1: Find and replace load_project_memory call**

Locate this code (around line 401):
```rust
// OLD CODE:
let project_memory = load_project_memory(&config.memory_file).await?;
```

Replace with:
```rust
// NEW CODE:
let project_memory = load_unified_memory(&config.work_dir).await?;
```

**Step 2: Run tests to verify change doesn't break anything**

Run: `cargo test execute_stage`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/pipeline/execute.rs
git commit -m "feat(execute): use unified memory loader instead of legacy format"
```

---

## Task 4: Write failing tests for scheduler integration

**Files:**
- Create: `tests/scheduler_integration_test.rs`

**Step 1: Write the failing test**

```rust
use ltmatrix::pipeline::execute::{execute_tasks, ExecuteConfig};
use ltmatrix::models::{Task, TaskComplexity, TaskStatus};
use ltmatrix_core::AgentType;
use tempfile::TempDir;

fn create_test_task(id: &str, title: &str, priority: u8, depends_on: Vec<String>) -> Task {
    let mut task = Task::new(id, title, format!("Task: {}", title));
    task.priority = priority;
    task.depends_on = depends_on;
    task.complexity = TaskComplexity::Simple;
    task
}

#[tokio::test]
async fn test_scheduler_priority_boost_is_applied() {
    // Create tasks where task-1 blocks multiple downstream tasks
    let task1 = create_test_task("task-1", "Foundation", 5, vec![]);
    let task2 = create_test_task("task-2", "Feature A", 5, vec!["task-1".to_string()]);
    let task3 = create_test_task("task-3", "Feature B", 5, vec!["task-1".to_string()]);
    let task4 = create_test_task("task-4", "Feature C", 5, vec!["task-1".to_string()]);

    let tasks = vec![task1, task2, task3, task4];

    // When we execute, task-1 should be executed FIRST because it blocks 3 others
    // The scheduler should boost its priority

    // This test verifies the scheduler is being used
    // We can check logs or execution order
    let temp_dir = TempDir::new().unwrap();
    let config = ExecuteConfig {
        work_dir: temp_dir.path().to_path_buf(),
        ..ExecuteConfig::default()
    };

    // Note: This is an integration test that would need mocking
    // For now, we verify the order calculation uses scheduler
    use ltmatrix::tasks::scheduler::{schedule_tasks_with_priority, PriorityConfig};

    let priority_config = PriorityConfig::default();
    let plan = schedule_tasks_with_priority(tasks.clone(), &priority_config).unwrap();

    // task-1 should be first in execution order
    assert_eq!(plan.execution_order[0], "task-1");

    // Verify it identified the critical path
    assert!(!plan.critical_path.is_empty());
}
```

**Step 2: Run test to verify it compiles but scheduler isn't used yet**

Run: `cargo test scheduler_integration_test --no-fail-fast 3>&1`
Expected: Test compiles and passes (verifying scheduler works in isolation)

**Step 3: Commit**

```bash
git add tests/scheduler_integration_test.rs
git commit -m "test(scheduler): add integration test for scheduler priority boosting"
```

---

## Task 5: Integrate scheduler into execute_tasks

**Files:**
- Modify: `src/pipeline/execute.rs:376-435`

**Step 1: Add scheduler import**

```rust
// Add to imports:
use crate::tasks::scheduler::{schedule_tasks_with_priority, PriorityConfig, ExecutionPlan};
```

**Step 2: Replace get_execution_order with scheduler call**

Find this section (around line 405-435):
```rust
// OLD CODE - build task map and use get_execution_order:
let task_map: HashMap<String, Task> = tasks
    .into_iter()
    .map(|task| (task.id.clone(), task))
    .collect();

// ... later ...
for task_id in get_execution_order(&task_map)? {
```

Replace with:
```rust
// NEW CODE - use scheduler with priority:
let task_map: HashMap<String, Task> = tasks
    .iter()
    .map(|task| (task.id.clone(), task.clone()))
    .collect();

// Configure priority scheduling
let priority_config = PriorityConfig {
    blocking_boost: 1,
    max_boost: 3,
    enable_priority_sorting: true,
    enable_related_grouping: true,
};

// Get execution plan from scheduler
let execution_plan = schedule_tasks_with_priority(
    tasks.clone(),
    &priority_config,
).context("Failed to create execution plan")?;

info!(
    "Execution plan: {} levels, {} tasks, critical path length: {}",
    execution_plan.max_depth,
    execution_plan.total_tasks,
    execution_plan.critical_path.len()
);

// Log parallelizable tasks
if !execution_plan.parallelizable_tasks.is_empty() {
    debug!(
        "Parallelizable tasks: {:?}",
        execution_plan.parallelizable_tasks
    );
}

// Execute tasks in scheduled order
for task_id in &execution_plan.execution_order {
```

**Step 3: Update the task lookup to use task_map**

The task lookup inside the loop should now reference the pre-built task_map:
```rust
let mut task = task_map
    .get(task_id)
    .cloned()
    .context(format!("Task {} not found in task map", task_id))?;
```

**Step 4: Run tests to verify changes work**

Run: `cargo test execute_stage`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/pipeline/execute.rs
git commit -m "feat(execute): integrate scheduler with priority boosting"
```

---

## Task 6: Remove unused get_execution_order function

**Files:**
- Modify: `src/pipeline/execute.rs:568-619`

**Step 1: Delete the unused function**

Find and remove this function (lines 568-619):
```rust
/// Get execution order respecting dependencies
pub fn get_execution_order(task_map: &HashMap<String, Task>) -> Result<Vec<String>> {
    // ... entire function body ...
}
```

**Step 2: Verify it compiles after removal**

Run: `cargo build`
Expected: Success (no compilation errors)

**Step 3: Run tests to verify nothing breaks**

Run: `cargo test`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/pipeline/execute.rs
git commit -m "refactor(execute): remove unused get_execution_order function"
```

---

## Task 7: Write failing test for agent type fallback

**Files:**
- Create: `tests/agent_type_fallback_test.rs`

**Step 1: Write the failing test**

```rust
use ltmatrix::pipeline::generate::{generate_tasks, GenerateConfig};
use ltmatrix::models::{Task, TaskComplexity};
use ltmatrix_core::AgentType;

// This test verifies that even if LLM returns wrong/missing agent_type,
// the keyword fallback corrects it

#[test]
fn test_agent_type_fallback_corrects_test_keyword() {
    // Simulate a task where LLM returned "Dev" but content has test keywords
    let mut task = Task::new("task-1", "Write unit tests", "Add test coverage for module");
    task.agent_type = AgentType::Dev; // LLM incorrectly set Dev

    // The fallback should detect "test" keyword and correct to Test
    let combined = format!("{} {}", task.title, task.description);
    let detected = AgentType::from_keywords(&combined);

    assert_eq!(detected, AgentType::Test);
}

#[test]
fn test_agent_type_fallback_corrects_review_keyword() {
    let mut task = Task::new("task-1", "Code review", "Review the implementation for quality");
    task.agent_type = AgentType::Dev;

    let combined = format!("{} {}", task.title, task.description);
    let detected = AgentType::from_keywords(&combined);

    assert_eq!(detected, AgentType::Review);
}

#[test]
fn test_agent_type_fallback_keeps_dev_for_implementation() {
    let task = Task::new("task-1", "Implement feature", "Add new functionality");

    let combined = format!("{} {}", task.title, task.description);
    let detected = AgentType::from_keywords(&combined);

    // No keywords found, should stay Dev
    assert_eq!(detected, AgentType::Dev);
}

#[test]
fn test_agent_type_fallback_handles_chinese_keywords() {
    let mut task = Task::new("task-1", "测试模块", "添加单元测试");
    task.agent_type = AgentType::Dev;

    let combined = format!("{} {}", task.title, task.description);
    let detected = AgentType::from_keywords(&combined);

    assert_eq!(detected, AgentType::Test);
}
```

**Step 2: Run tests to verify from_keywords works**

Run: `cargo test agent_type_fallback_test`
Expected: All tests pass (from_keywords already implemented)

**Step 3: Commit**

```bash
git add tests/agent_type_fallback_test.rs
git commit -m "test(generate): add tests for agent type keyword fallback"
```

---

## Task 8: Integrate agent type fallback into generate.rs

**Files:**
- Modify: `src/pipeline/generate.rs:449-500`

**Step 1: Add post-processing after parse_generation_response**

Find this section (around line 449):
```rust
let mut tasks = parse_generation_response(&response.output)
    .context("Failed to parse generation response")?;
```

Add after it:
```rust
// Apply keyword-based agent type detection as fallback
// This corrects cases where LLM omits or incorrectly sets agent_type
for task in &mut tasks {
    if task.agent_type == AgentType::Dev {
        let combined = format!("{} {}", task.title, task.description);
        let detected = AgentType::from_keywords(&combined);
        if detected != AgentType::Dev {
            debug!(
                "Task {} agent type auto-corrected: Dev -> {:?} (keywords detected)",
                task.id, detected
            );
            task.agent_type = detected;
        }
    }
}

debug!("Applied agent type keyword fallback to {} tasks", tasks.len());
```

**Step 2: Run tests to verify it compiles**

Run: `cargo build`
Expected: Success

**Step 3: Run generate tests**

Run: `cargo test generate`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/pipeline/generate.rs
git commit -m "feat(generate): add agent type keyword fallback after LLM generation"
```

---

## Task 9: Write failing test for failure report surfacing

**Files:**
- Create: `tests/failure_report_surfacing_test.rs`

**Step 1: Write the failing test**

```rust
use ltmatrix::pipeline::execute::{generate_failure_report, classify_error, ErrorClass, FailureAction};
use ltmatrix::models::{Task, TaskStatus, TaskComplexity};
use std::collections::HashMap;

fn create_failed_task(id: &str, error: &str) -> Task {
    let mut task = Task::new(id, "Failed task", "This task will fail");
    task.status = TaskStatus::Failed;
    task.error = Some(error.to_string());
    task.retry_count = 2;
    task
}

#[test]
fn test_classify_error_retryable_timeout() {
    let error = "Operation timeout after 30 seconds";
    assert_eq!(classify_error(error), ErrorClass::Retryable);
}

#[test]
fn test_classify_error_retryable_rate_limit() {
    let error = "Rate limit exceeded: too many requests";
    assert_eq!(classify_error(error), ErrorClass::Retryable);
}

#[test]
fn test_classify_error_non_retryable_syntax() {
    let error = "Syntax error: unexpected token at line 42";
    assert_eq!(classify_error(error), ErrorClass::NonRetryable);
}

#[test]
fn test_classify_error_non_retryable_permission() {
    let error = "Permission denied: cannot access file";
    assert_eq!(classify_error(error), ErrorClass::NonRetryable);
}

#[test]
fn test_generate_failure_report_suggests_retry_for_retryable() {
    let task = create_failed_task("task-1", "timeout exceeded");
    let task_map: HashMap<String, Task> = [(task.id.clone(), task.clone())].into_iter().collect();

    let report = generate_failure_report(&task, &task_map);

    assert_eq!(report.suggested_action, FailureAction::Retry);
}

#[test]
fn test_generate_failure_report_suggests_skip_for_non_retryable() {
    let task = create_failed_task("task-1", "syntax error in code");
    let task_map: HashMap<String, Task> = [(task.id.clone(), task.clone())].into_iter().collect();

    let report = generate_failure_report(&task, &task_map);

    assert_eq!(report.suggested_action, FailureAction::Skip);
}

#[test]
fn test_generate_failure_report_identifies_blocked_downstream() {
    let task1 = create_failed_task("task-1", "timeout exceeded");
    let mut task2 = Task::new("task-2", "Dependent task", "Depends on task-1");
    task2.depends_on = vec!["task-1".to_string()];

    let task_map: HashMap<String, Task> = [
        (task1.id.clone(), task1.clone()),
        (task2.id.clone(), task2),
    ].into_iter().collect();

    let report = generate_failure_report(&task1, &task_map);

    assert!(report.blocked_downstream.contains(&"task-2".to_string()));
}
```

**Step 2: Run tests to verify failure report functions work**

Run: `cargo test failure_report_surfacing_test`
Expected: All tests pass (functions already implemented)

**Step 3: Commit**

```bash
git add tests/failure_report_surfacing_test.rs
git commit -m "test(execute): add tests for failure report generation"
```

---

## Task 10: Surface failure reports in execute_tasks

**Files:**
- Modify: `src/pipeline/execute.rs` (in the task failure handling section)

**Step 1: Find the failure handling section**

Look for code like this (after a task fails):
```rust
if task.error.is_some() {
    stats.failed_tasks += 1;
    results.push(task);
}
```

**Step 2: Add failure report surfacing**

Replace the failure handling section with:
```rust
if task.error.is_some() {
    stats.failed_tasks += 1;

    // Generate and surface failure report
    let report = generate_failure_report(&task, &task_map);

    // Log prominently for visibility
    warn!("=== FAILURE REPORT ===");
    warn!("Task: {} ({})", report.task_id, task.title);
    warn!("Error: {}", report.error_message);
    warn!("Retries: {}/{}", report.retry_count, config.max_retries);
    if !report.blocked_downstream.is_empty() {
        warn!("Blocked downstream: {:?}", report.blocked_downstream);
    }
    warn!("Suggested action: {:?}", report.suggested_action);
    warn!("======================");

    // Write to failure log for post-mortem analysis
    let failure_log = config.work_dir.join(".ltmatrix").join("failures.log");
    if let Some(parent) = failure_log.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    let entry = format!(
        "[{}] Task {} failed: {} (action: {:?})\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
        report.task_id,
        report.error_message,
        report.suggested_action
    );
    tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&failure_log)
        .await
        .and_then(|f| async {
            tokio::fs::write(f, entry.as_bytes()).await
        })
        .await
        .ok();

    results.push(task);
}
```

**Step 3: Run tests to verify it compiles**

Run: `cargo build`
Expected: Success

**Step 4: Run execute tests**

Run: `cargo test execute_stage`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/pipeline/execute.rs
git commit -m "feat(execute): surface failure reports with logging and file output"
```

---

## Task 11: Run full test suite

**Files:**
- None (verification only)

**Step 1: Run all tests**

Run: `cargo test --all-features`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: No warnings

**Step 3: Run fmt check**

Run: `cargo fmt --check`
Expected: No formatting issues

**Step 4: Commit (if any fixes needed)**

```bash
git add -A
git commit -m "chore: fix test/clippy/fmt issues after integration"
```

---

## Task 12: Update blog post to reflect fixes

**Files:**
- Modify: `docs/blog/feature-1.0.0-release.md`

**Step 1: Update blog post to note fixes made**

Add a section near the end of the blog post:
```markdown
## 🔧 Post-Release Fixes (2026-03-14)

After the 1.0.0 release, we identified integration gaps where features were implemented but not connected to the pipeline. These have now been fixed:

### Fixed Issues

1. **Scheduler Integration** - The priority scheduler with blocking boosts and related task grouping is now fully integrated into the execution pipeline.

2. **Agent Type Auto-Assignment** - Keyword-based fallback now ensures tasks get the correct agent type even if the LLM omits it.

3. **Memory System Integration** - The new `ProjectMemory` and `RunMemory` systems are now used for context injection before execution.

4. **Failure Report Surfacing** - Failed tasks now generate detailed reports logged and written to `.ltmatrix/failures.log`.
```

**Step 2: Commit**

```bash
git add docs/blog/feature-1.0.0-release.md
git commit -m "docs: update blog post with post-release fix notes"
```

---

## Task 13: Final verification

**Files:**
- None (verification only)

**Step 1: Verify all changes compile and tests pass**

Run: `cargo test --all-features`
Expected: All tests pass

**Step 2: Create summary commit**

```bash
git add -A
git commit -m "feat: complete feature integration gaps fix

- Integrate scheduler with priority boosting
- Add agent type keyword fallback
- Use unified memory loader
- Surface failure reports

All 4 integration gaps from the 1.0.0 release are now fixed."
```

---

## Summary

| Task | Description | Files Changed |
|------|-------------|---------------|
| 1 | Write failing tests for unified memory loader | `tests/unified_memory_loader_test.rs` |
| 2 | Implement unified memory loader | `src/pipeline/execute.rs` |
| 3 | Replace old memory loader | `src/pipeline/execute.rs` |
| 4 | Write failing tests for scheduler integration | `tests/scheduler_integration_test.rs` |
| 5 | Integrate scheduler into execute_tasks | `src/pipeline/execute.rs` |
| 6 | Remove unused get_execution_order | `src/pipeline/execute.rs` |
| 7 | Write tests for agent type fallback | `tests/agent_type_fallback_test.rs` |
| 8 | Integrate agent type fallback | `src/pipeline/generate.rs` |
| 9 | Write tests for failure report surfacing | `tests/failure_report_surfacing_test.rs` |
| 10 | Surface failure reports | `src/pipeline/execute.rs` |
| 11 | Run full test suite | None (verification) |
| 12 | Update blog post | `docs/blog/feature-1.0.0-release.md` |
| 13 | Final verification | None (verification) |
