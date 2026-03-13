# Feature Integration Gaps Fix Design

**Date**: 2026-03-14
**Status**: Approved
**Author**: Claude (with user collaboration)

## Problem Statement

The blog post `docs/blog/feature-1.0.0-release.md` claims 6 major features, but code analysis revealed 4 integration gaps where features are implemented but NOT connected to the main pipeline:

1. **Scheduler Not Connected**: `scheduler.rs` has priority boosting and related task grouping, but `execute.rs` uses its own simple topological sort
2. **Agent Type Auto-Assignment Not Used**: `AgentType::from_keywords()` exists but is never called in task generation
3. **New Memory System Not Injected**: `ProjectMemory`/`RunMemory` exist but `execute.rs` reads old `.claude/memory.md` format
4. **Failure Reports Not Surfaced**: `generate_failure_report()` exists but output is never shown to users

## Design Goals

1. Connect the scheduler to the execution pipeline with full priority support
2. Add keyword-based agent type detection as fallback after LLM generation
3. Integrate new memory system for context injection before execution
4. Surface failure reports through logging and file output

## Design Details

### 1. Scheduler Integration

**File**: `src/pipeline/execute.rs`

**Current State**:
```rust
// Uses simple topological sort
for task_id in get_execution_order(&task_map)? {
    // execute task...
}
```

**Target State**:
```rust
use crate::tasks::scheduler::{schedule_tasks_with_priority, PriorityConfig};

// In execute_tasks():
let priority_config = PriorityConfig {
    blocking_boost: 1,
    max_boost: 3,
    enable_priority_sorting: true,
    enable_related_grouping: true,
};

let execution_plan = schedule_tasks_with_priority(
    tasks.clone(),
    &priority_config
)?;

// Log plan details
info!("Execution plan: {} levels, critical path length: {}",
    execution_plan.max_depth,
    execution_plan.critical_path.len()
);

// Use execution_plan.execution_order for sequential execution
for task_id in execution_plan.execution_order {
    // execute task...
}
```

**Changes Required**:
- Import scheduler module
- Replace `get_execution_order()` call with `schedule_tasks_with_priority()`
- Log execution plan statistics
- Remove unused `get_execution_order()` function

### 2. Agent Type Auto-Assignment Fallback

**File**: `src/pipeline/generate.rs`

**Current State**:
```rust
let mut tasks = parse_generation_response(&response.output)?;
// Tasks get agent_type from LLM JSON output only
```

**Target State**:
```rust
let mut tasks = parse_generation_response(&response.output)?;

// Apply keyword-based agent type detection as fallback
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
```

**Changes Required**:
- Add post-processing loop after JSON parsing
- Import `AgentType` if not already imported
- Log auto-corrections for transparency

### 3. Unified Memory Integration

**File**: `src/pipeline/execute.rs`

**Current State**:
```rust
let project_memory = load_project_memory(&config.memory_file).await?;
// Only reads .claude/memory.md
```

**Target State**:
```rust
use crate::memory::{ProjectMemory, RunMemory, get_project_memory_path, get_current_run_memory_path};

async fn load_unified_memory(project_root: &Path) -> Result<String> {
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

    // 2. Load current RunMemory if exists
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

// In execute_tasks():
let project_memory = load_unified_memory(&config.work_dir).await?;
```

**Changes Required**:
- Add `load_unified_memory()` function
- Import memory module types
- Replace `load_project_memory()` call
- Update `ExecuteConfig` to use `work_dir` instead of `memory_file` (or keep both for compatibility)

### 4. Failure Report Surfacing

**File**: `src/pipeline/execute.rs`

**Current State**:
```rust
if task.error.is_some() {
    stats.failed_tasks += 1;
    results.push(task);
}
```

**Target State**:
```rust
if task.error.is_some() {
    stats.failed_tasks += 1;

    // Generate and surface failure report
    let report = generate_failure_report(&task, &task_map);

    // Log prominently
    warn!("=== FAILURE REPORT ===");
    warn!("Task: {} ({})", report.task_id, task.title);
    warn!("Error: {}", report.error_message);
    warn!("Retries: {}/{}", report.retry_count, config.max_retries);
    if !report.blocked_downstream.is_empty() {
        warn!("Blocked downstream: {:?}", report.blocked_downstream);
    }
    warn!("Suggested action: {:?}", report.suggested_action);
    warn!("======================");

    // Write to failure log for post-mortem
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

**Changes Required**:
- Generate failure report for each failed task
- Add prominent logging with `warn!` level
- Write to `.ltmatrix/failures.log` for post-mortem analysis

## Impact Analysis

### Files Changed
| File | Changes | Lines Changed (est.) |
|------|---------|---------------------|
| `src/pipeline/execute.rs` | Scheduler integration, memory integration, failure reports | ~80 lines |
| `src/pipeline/generate.rs` | Agent type fallback | ~15 lines |

### Backward Compatibility
- Memory: Falls back to `.claude/memory.md` if new system doesn't exist
- Scheduler: No API change, just internal implementation
- Failure logs: New file, doesn't affect existing behavior

### Testing Strategy
1. Unit tests for `load_unified_memory()`
2. Integration test: verify scheduler priority boosts are applied
3. Integration test: verify agent type fallback triggers correctly
4. Manual test: verify failure log is created and contains expected content

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Scheduler change affects execution order | Keep `get_execution_order()` as fallback option |
| Memory load fails on corrupted JSON | Wrap in try/catch, fallback to empty context |
| Failure log grows unbounded | Consider rotation in future (out of scope) |

## Success Criteria

1. ✅ Priority boosts visible in logs during execution
2. ✅ Tasks with test keywords get `AgentType::Test` even if LLM omits it
3. ✅ Project memory from `.ltmatrix/memory/project.json` included in agent context
4. ✅ Failed tasks write entries to `.ltmatrix/failures.log`
5. ✅ All existing tests pass

## References

- Blog post: `docs/blog/feature-1.0.0-release.md`
- Scheduler: `src/tasks/scheduler.rs`
- Memory: `src/memory/project.rs`, `src/memory/run_memory.rs`
- Execute: `src/pipeline/execute.rs`
- Generate: `src/pipeline/generate.rs`
