# Feature Integration Gaps Fix Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Connect 4 implemented-but-disconnected features to the pipeline: scheduler priority, agent type fallback, unified memory, and failure reports.

**Architecture:** Replace simple internal implementations with calls to the already-implemented scheduler and memory systems. Add post-processing for agent type detection. Surface failure reports through logging and file output.

**Tech Stack:** Rust, Tokio, tracing, serde_json

---

## Task 1: Write test for scheduler integration

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

    // Use the scheduler directly to verify priority boosting
    use ltmatrix::tasks::scheduler::{schedule_tasks_with_priority, PriorityConfig};

    let priority_config = PriorityConfig::default();
    let plan = schedule_tasks_with_priority(tasks.clone(), &priority_config).unwrap();

    // task-1 should be first in execution order (it blocks 3 others)
    assert_eq!(plan.execution_order[0], "task-1");

    // Verify critical path is identified
    assert!(!plan.critical_path.is_empty());
}
```

**Step 2: Run test to verify it compiles**

Run: `cargo test scheduler_integration_test --no-fail-fast 3>&1`
Expected: Test compiles and passes (scheduler already implemented)

**Step 3: Commit**

```bash
git add tests/scheduler_integration_test.rs
git commit -m "test(scheduler): add integration test for scheduler priority boosting"
```

---

## Task 2: Write test for unified memory loading

**Files:**
- Create: `tests/unified_memory_test.rs`

**Step 1: Write the failing test**

```rust
use ltmatrix::pipeline::execute::load_unified_memory;
use ltmatrix::memory::{ProjectMemory, RunMemory};
use tempfile::TempDir;
use std::path::Path;

#[tokio::test]
async fn test_load_unified_memory_returns_empty_string_when_no_memory() {
    let temp_dir = TempDir::new().unwrap();
    let result = load_unified_memory(temp_dir.path()).await.unwrap();
    assert!(result.contains("No project memory available"));
}

#[tokio::test]
async fn test_load_unified_memory_loads_project_memory() {
    let temp_dir = TempDir::new().unwrap();

    // Create project memory
    let memory_dir = temp_dir.path().join(".ltmatrix").join("memory");
    tokio::fs::create_dir_all(&memory_dir).await.unwrap();

    let mut project_mem = ProjectMemory::new("test-project");
    project_mem.tech_stack.language = Some("Rust".to_string());
    project_mem.save(&memory_dir.join("project.json")).await.unwrap();

    let result = load_unified_memory(temp_dir.path()).await.unwrap();
    assert!(result.contains("test-project"));
    assert!(result.contains("Rust"));
}

#[tokio::test]
async fn test_load_unified_memory_falls_back_to_legacy() {
    let temp_dir = TempDir::new().unwrap();

    // Create old-style memory.md
    let claude_dir = temp_dir.path().join(".claude");
    tokio::fs::create_dir_all(&claude_dir).await.unwrap();
    tokio::fs::write(
        claude_dir.join("memory.md"),
        b"# Legacy Memory\nThis is the old format"
    ).await.unwrap();

    let result = load_unified_memory(temp_dir.path()).await.unwrap();
    assert!(result.contains("Legacy Memory"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test unified_memory_test --no-fail-fast 3>&1`
Expected: FAIL with "cannot find value `load_unified_memory`" (function not yet implemented)

**Step 3: Commit**

```bash
git add tests/unified_memory_test.rs
git commit -m "test(execute): add failing tests for unified memory loading"
```

---

## Task 3: Implement load_unified_memory function

**Files:**
- Modify: `src/pipeline/execute.rs:1-50` (imports section)

**Step 1: Add memory imports to execute.rs**

Add to the imports section:
```rust
use crate::memory::{ProjectMemory, RunMemory, get_project_memory_path, get_current_run_memory_path};
```

**Step 2: Implement load_unified_memory function**

Add after the imports section (around line 30):
```rust
/// Load unified memory from all sources (ProjectMemory, RunMemory, legacy)
///
/// Combines memory from:
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

    if context.trim().is_empty() {
        context = "No project memory available yet.".to_string();
    }

    Ok(context)
}
```

**Step 3: Run tests to verify implementation**

Run: `cargo test unified_memory_test`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/pipeline/execute.rs
git commit -m "feat(execute): add load_unified_memory for combined memory loading"
```

---

## Task 4: Replace load_project_memory with load_unified_memory

**Files:**
- Modify: `src/pipeline/execute.rs` (around line 400 in execute_tasks)

**Step 1: Find and replace the memory loading call**

Find this line (around line 402):
```rust
let project_memory = load_project_memory(&config.memory_file).await?;
```

Replace with:
```rust
let project_memory = load_unified_memory(&config.work_dir).await?;
```

**Step 2: Run tests to verify change**

Run: `cargo test execute_stage`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/pipeline/execute.rs
git commit -m "refactor(execute): use load_unified_memory instead of old format"
```

---

## Task 5: Integrate scheduler into execute_tasks

**Files:**
- Modify: `src/pipeline/execute.rs:376-435`

**Step 1: Add scheduler import**

Add to imports:
```rust
use crate::tasks::scheduler::{schedule_tasks_with_priority, PriorityConfig, ExecutionPlan};
```

**Step 2: Replace task map building and ordering**

Find this section (around line 404-435):
```rust
// Build task map for dependency lookup
let task_map: HashMap<String, Task> = tasks
    .into_iter()
    .map(|task| (task.id.clone(), task))
    .collect();
```

Replace with:
```rust
// Build task map for dependency lookup
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
    tasks,
    &priority_config,
).context("Failed to create execution plan")?;

info!(
    "Execution plan: {} levels, {} tasks, critical path length: {}",
    execution_plan.max_depth,
    execution_plan.total_tasks,
    execution_plan.critical_path.len()
);

// Log parallelizable tasks for debugging
if !execution_plan.parallelizable_tasks.is_empty() {
    debug!(
        "Parallelizable tasks: {:?}",
        execution_plan.parallelizable_tasks
    );
}
```

**Step 3: Update the execution loop to use execution_plan**

Find this line:
```rust
for task_id in get_execution_order(&task_map)? {
```

Replace with:
```rust
for task_id in &execution_plan.execution_order {
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

Find and remove this entire function:
```rust
/// Get execution order respecting dependencies
pub fn get_execution_order(task_map: &HashMap<String, Task>) -> Result<Vec<String>> {
    let mut order = Vec::new();
    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();

    for task_id in task_map.keys() {
        if !visited.contains(task_id) {
            visit_task(task_id, task_map, &mut visited, &mut visiting, &mut order)?;
        }
    }

    Ok(order)
}

/// Visit task for topological sort with cycle detection
fn visit_task(
    task_id: &str,
    task_map: &HashMap<String, Task>,
    visited: &mut HashSet<String>,
    visiting: &mut HashSet<String>,
    order: &mut Vec<String>,
) -> Result<()> {
    // ... entire function body (approximately 40 lines)
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

## Task 7: Write test for agent type fallback

**Files:**
- Create: `tests/agent_type_fallback_test.rs`

**Step 1: Write the failing test**

```rust
use ltmatrix::models::{Task, TaskComplexity};
use ltmatrix_core::AgentType;

fn create_task_with_agent_type(id: &str, title: &str, desc: &str, agent_type: AgentType) -> Task {
    let mut task = Task::new(id, title, desc);
    task.agent_type = agent_type;
    task
}

#[test]
fn test_from_keywords_detects_test_type() {
    let task = create_task_with_agent_type("t1", "Write tests", "Add unit tests", AgentType::Dev);
    let combined = format!("{} {}", task.title, task.description);
    let detected = AgentType::from_keywords(&combined);
    assert_eq!(detected, AgentType::Test);
}

#[test]
fn test_from_keywords_detects_review_type() {
    let task = create_task_with_agent_type("t1", "Code review", "Review the code", AgentType::Dev);
    let combined = format!("{} {}", task.title, task.description);
    let detected = AgentType::from_keywords(&combined);
    assert_eq!(detected, AgentType::Review);
}

#[test]
fn test_from_keywords_detects_plan_type() {
    let task = create_task_with_agent_type("t1", "Analyze architecture", "Plan the system", AgentType::Dev);
    let combined = format!("{} {}", task.title, task.description);
    let detected = AgentType::from_keywords(&combined);
    assert_eq!(detected, AgentType::Plan);
}

#[test]
fn test_from_keywords_keeps_dev_for_implementation() {
    let task = create_task_with_agent_type("t1", "Implement feature", "Add new functionality", AgentType::Dev);
    let combined = format!("{} {}", task.title, task.description);
    let detected = AgentType::from_keywords(&combined);
    assert_eq!(detected, AgentType::Dev);
}

#[test]
fn test_from_keywords_handles_chinese_keywords() {
    // Test Chinese keyword detection
    let task = create_task_with_agent_type("t1", "测试模块", "添加单元测试", AgentType::Dev);
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
- Modify: `src/pipeline/generate.rs:449-460`

**Step 1: Add post-processing after parse_generation_response**

Find this section (around line 449):
```rust
let mut tasks = parse_generation_response(&response.output)
    .context("Failed to parse generation response")?;
```

Add after it (before the max_tasks check):
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

## Task 9: Write test for failure report surfacing

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
    task.retry_count = 1;
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

**Step 1: Find and enhance the failure handling section**

Find this code (around line 530):
```rust
if task.error.is_some() {
    stats.failed_tasks += 1;
    results.push(task);
}
```

Replace with:
```rust
if task.error.is_some() {
    stats.failed_tasks += 1;

    // Generate and surface failure report
    let report = generate_failure_report(&task, &task_map);

    // Log prominently with warn level
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
        .and_then(|f| tokio::fs::write(f, entry.as_bytes()).await)
        .ok();

    results.push(task);
}
```

**Step 2: Run tests to verify it compiles**

Run: `cargo build`
Expected: Success

**Step 3: Run execute tests**

Run: `cargo test execute_stage`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/pipeline/execute.rs
git commit -m "feat(execute): surface failure reports through logging and file output"
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

**Step 1: Add post-release fixes section**

Add near the end of the blog post:
```markdown
## 🔧 Post-Release Fixes (2026-03-14)

After the 1.0.0 release, integration gaps were identified and fixed. The following
 features are now fully connected to the pipeline:

### Fixed Issues

1. **Scheduler Integration** - The priority scheduler with blocking boosts
   and related task grouping is now fully integrated into the execution pipeline.
   Tasks that block multiple downstream tasks now get priority boosts automatically.

2. **Agent Type Auto-Assignment** - Keyword-based fallback now ensures tasks
   get the correct agent type even if the LLM omits it. The `from_keywords()`
   function is now called after task generation to correct any missing assignments.

3. **Unified Memory System** - The new `ProjectMemory` and `RunMemory` systems
   are now integrated for context injection. Agent prompts include memory from
   `.ltmatrix/memory/` with fallback to legacy `.claude/memory.md`.

4. **Failure Report Surfacing** - Failed tasks now generate detailed failure reports
   that are logged prominently and written to `.ltmatrix/failures.log` for
   post-mortem analysis.
```

**Step 2: Verify the blog post still looks correct**

Run: `cat docs/blog/feature-1.0.0-release.md`
Expected: File updated correctly

**Step 3: Commit**

```bash
git add docs/blog/feature-1.0.0-release.md
git commit -m "docs: update blog post with post-release integration fixes"
```

---

## Summary

| Task | Description | Files Changed |
|------|-------------|---------------|
| 1 | Test scheduler integration | Create test file |
| 2 | Test unified memory | Create test file |
| 3 | Implement load_unified_memory | `execute.rs` |
| 4 | Replace memory loading call | `execute.rs` |
| 5 | Integrate scheduler | `execute.rs` |
| 6 | Remove unused function | `execute.rs` |
| 7 | Test agent type fallback | Create test file |
| 8 | Integrate agent type fallback | `generate.rs` |
| 9 | Test failure reports | Create test file |
| 10 | Surface failure reports | `execute.rs` |
| 11 | Run full test suite | Verification |
| 12 | Update blog post | `docs/blog/` |
