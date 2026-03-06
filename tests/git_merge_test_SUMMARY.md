# Git Merge Operations Test Suite

## Overview
Comprehensive integration tests for the merge operations implementation, specifically the `merge_with_squash()` function.

## Test Coverage

### 13 Integration Tests Created

1. **test_basic_squash_merge** - Verifies basic squash merge workflow with proper commit creation
2. **test_squash_merge_multiple_commits** - Tests squashing multiple commits into one
3. **test_merge_conflict_detection** - Validates conflict detection when same file modified on both branches
4. **test_merge_nonexistent_branch_fails** - Ensures proper error handling for non-existent branches
5. **test_merge_no_new_commits_fails** - Validates error when source branch has no new commits
6. **test_merge_with_staged_changes_fails** - Tests that staged changes block merge operations
7. **test_merge_staged_changes_error_suggests_actions** - Verifies error messages suggest corrective actions
8. **test_merge_commit_message_formatting** - Ensures commit message formatting is preserved
9. **test_merge_whitespace_message_fails** - Tests validation of whitespace-only messages
10. **test_merge_empty_message_validation** - Validates empty message rejection
11. **test_merge_parent_relationship** - Verifies squash merge creates correct parent-child relationships
12. **test_merge_with_file_additions** - Tests merging new file additions
13. **test_merge_with_directory_structure** - Validates handling of nested directory structures

## Test Results

```
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
```

## Key Test Scenarios

### Success Paths
- ✅ Basic branch merging
- ✅ Multiple commit squashing
- ✅ File additions
- ✅ Directory structures
- ✅ Commit message preservation

### Error Conditions
- ✅ Conflict detection with user-friendly messages
- ✅ Non-existent branch errors with available branches listed
- ✅ No new commits error
- ✅ Staged changes blocking merge
- ✅ Empty/whitespace message validation
- ✅ Detached HEAD detection

## Implementation Notes

The tests verify that the `merge_with_squash()` function properly:
- Validates preconditions (staged changes, detached HEAD, branch existence)
- Detects merge conflicts before creating commits
- Provides user-friendly error messages with actionable guidance
- Creates squashed commits with correct parent relationships
- Handles various file system operations (additions, directories, etc.)

## Test File Location
`tests/git_merge_test.rs`

## Dependencies Updated
- Added `merge_with_squash` to public API exports in `src/git/mod.rs`
