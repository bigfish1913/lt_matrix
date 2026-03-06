# Commit Stage Implementation

## Overview

The commit stage handles version control for completed tasks using per-task branching with squash merge strategy.

## Implementation

**File:** `src/pipeline/commit.rs`

### Key Features

1. **Per-Task Branching Strategy**
   - Creates task branch: `task-{id}`
   - Stages all changes made during task execution
   - Commits with conventional commit message
   - Squash merges to base branch on success
   - Optionally deletes task branch after merge

2. **Direct Commit Strategy**
   - Commits directly to current branch
   - Used in fast mode for simplicity
   - No branch management overhead

3. **Conventional Commit Format**
   ```
   {type}: [{task-id}] {title}
   ```
   Example: `feat: [task-123] Add user authentication`

4. **Merge Conflict Handling**
   - Detects merge conflicts
   - Preserves task branch for manual resolution
   - Notifies user with instructions
   - Tracks conflicts in CommitSummary

5. **Git Repository Detection**
   - Automatically detects if not in a git repository
   - Configurable to skip or fail
   - Handles missing .git directory gracefully

### Configuration

```rust
pub struct CommitConfig {
    /// Enable/disable committing
    pub enabled: bool,

    /// Base branch for merging (default: current branch)
    pub base_branch: Option<String>,

    /// Skip if not in git repository
    pub skip_if_no_repo: bool,

    /// Delete task branches after merge
    pub delete_after_merge: bool,

    /// Use per-task branches (vs direct commits)
    pub use_task_branches: bool,

    /// Commit type for conventional commits
    pub commit_type: String,

    /// Working directory
    pub work_dir: PathBuf,
}
```

### Execution Modes

**Fast Mode:**
- Direct commits (no branching)
- No branch deletion (not needed)
- Faster execution
- Less isolation

**Expert Mode:**
- Full per-task branching
- Automatic branch cleanup
- Better isolation
- Easier rollback
- More comprehensive

### API

```rust
pub async fn commit_tasks(
    tasks: Vec<Task>,
    config: &CommitConfig,
) -> Result<(Vec<Task>, CommitSummary)>
```

**Returns:**
- Updated tasks (only completed ones)
- CommitSummary with statistics

**CommitSummary:**
```rust
pub struct CommitSummary {
    pub total_tasks: usize,
    pub committed_tasks: usize,
    pub failed_tasks: usize,
    pub skipped_tasks: usize,
    pub conflicts: usize,
    pub branches_created: usize,
    pub branches_deleted: usize,
    pub total_commits: usize,
    pub base_branch: Option<String>,
}
```

### Workflow

1. Check if committing is enabled
2. Open git repository (skip/fail if not found)
3. Get current branch as base branch
4. Filter for completed tasks only
5. For each completed task:
   - **With Branching:**
     - Create and checkout `task-{id}` branch
     - Stage all changes
     - Commit with conventional message
     - Return to base branch
     - Squash merge task branch
     - Delete task branch (if configured)
   - **Direct:**
     - Stage all changes
     - Commit with conventional message
6. Return updated tasks and summary

### Error Handling

- **No changes to commit:** Success with message in error field
- **Merge conflicts:** Preserves branch, returns conflict flag
- **Not a git repo:** Skip or fail based on config
- **Branch operations:** Full error propagation with context

### Testing

**Unit Tests (12):**
- Configuration modes (default, fast, expert)
- Commit message formatting
- Summary statistics
- Repository detection
- Branch creation/checkout
- Disabled committing

**Integration Tests (16):**
- Single/multiple task commits
- Branching vs direct strategies
- Task filtering by status
- Base branch handling
- File changes
- Config variations
- Error scenarios
- Serialization

**All tests passing:** ✅ 320 tests

### Integration Points

**Dependencies:**
- `crate::git` - All git operations
  - `create_branch`, `checkout`, `delete_branch`
  - `stage_all`, `commit_changes`
  - `merge_with_squash`
  - `get_current_branch`, `branch_exists`

**Used By:**
- Pipeline orchestration
- Task scheduler
- Main execution flow

### Examples

```rust
// Fast mode - direct commits
let config = CommitConfig::fast_mode();
let (tasks, summary) = commit_tasks(tasks, &config).await?;

// Expert mode - full branching
let config = CommitConfig::expert_mode();
let (tasks, summary) = commit_tasks(tasks, &config).await?;

// Custom configuration
let config = CommitConfig {
    enabled: true,
    use_task_branches: true,
    commit_type: "fix".to_string(),
    base_branch: Some("develop".to_string()),
    ..Default::default()
};
let (tasks, summary) = commit_tasks(tasks, &config).await?;
```

### Status

✅ **Complete** - Fully implemented and tested
- All functionality working
- Comprehensive test coverage
- Documentation complete
- Ready for integration