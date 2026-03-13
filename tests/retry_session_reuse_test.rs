//! Integration tests for retry scenario session reuse
//!
//! This test suite verifies that sessions are properly reused across
//! retry attempts, maintaining conversational context and continuity.

use ltmatrix::agent::backend::AgentSession;
use ltmatrix::agent::pool::SessionPool;
use ltmatrix::models::{Task, TaskStatus};

/// Test that a task's session is reused on retry
#[test]
fn test_task_session_reused_on_retry() {
    let mut pool = SessionPool::new();
    let mut task = Task::new("task-1", "Test Task", "A test task");

    // First execution - creates a new session
    let session1 = pool.get_or_create_for_task(&mut task);
    let session_id = task.get_session_id().unwrap().to_string();
    let reuse_count_after_first = session1.reuse_count();

    // Simulate task failure and prepare for retry
    task.status = TaskStatus::Failed;
    task.error = Some("Temporary error".to_string());
    task.prepare_retry();

    // Verify session_id is preserved after prepare_retry
    assert_eq!(task.get_session_id(), Some(session_id.as_str()));
    assert_eq!(task.retry_count, 1);
    assert_eq!(task.status, TaskStatus::Pending);

    // Second execution (retry) - should reuse the same session
    let session2 = pool.get_or_create_for_task(&mut task);

    // Verify the same session was reused
    assert_eq!(session2.session_id(), session_id);
    assert_eq!(session2.reuse_count(), reuse_count_after_first + 1);
}

/// Test that multiple tasks with the same agent/model share sessions
#[test]
fn test_multiple_tasks_share_same_session_for_same_agent() {
    let mut pool = SessionPool::new();

    let mut task1 = Task::new("task-1", "Task 1", "First task");
    let mut task2 = Task::new("task-2", "Task 2", "Second task");

    // Both tasks use the same agent/model, so they share a session
    let session1_id = pool
        .get_or_create_for_task(&mut task1)
        .session_id()
        .to_string();
    let session2_id = pool
        .get_or_create_for_task(&mut task2)
        .session_id()
        .to_string();

    // Sessions are shared for the same (agent_name, model) pair
    assert_eq!(session1_id, session2_id);
    assert_eq!(task1.get_session_id(), task2.get_session_id());
    assert_eq!(pool.len(), 1); // Only one session created
}

/// Test that retry properly increments reuse count
#[test]
fn test_retry_increments_session_reuse_count() {
    let mut pool = SessionPool::new();
    let mut task = Task::new("task-1", "Test Task", "A test task");

    // Initial execution
    let session = pool.get_or_create_for_task(&mut task);
    assert_eq!(session.reuse_count(), 0);

    // First retry
    task.status = TaskStatus::Failed;
    task.prepare_retry();
    let session = pool.get_or_create_for_task(&mut task);
    assert_eq!(session.reuse_count(), 1);

    // Second retry
    task.status = TaskStatus::Failed;
    task.prepare_retry();
    let session = pool.get_or_create_for_task(&mut task);
    assert_eq!(session.reuse_count(), 2);
}

/// Test that session tracking works across the full lifecycle
#[test]
fn test_full_task_lifecycle_with_session_tracking() {
    let mut pool = SessionPool::new();
    let mut task = Task::new("task-1", "Test Task", "A test task");

    // 1. Initial execution
    let session1 = pool.get_or_create_for_task(&mut task);
    let session_id = task.get_session_id().unwrap().to_string();
    assert!(task.has_session());
    assert_eq!(session1.reuse_count(), 0);

    // 2. Task fails
    task.status = TaskStatus::Failed;
    task.error = Some("Network error".to_string());

    // 3. Prepare for retry
    task.prepare_retry();
    assert_eq!(task.retry_count, 1);
    assert_eq!(task.get_session_id(), Some(session_id.as_str()));

    // 4. Retry execution
    let session2 = pool.get_or_create_for_task(&mut task);
    assert_eq!(session2.session_id(), session_id);
    assert_eq!(session2.reuse_count(), 1);

    // 5. Task succeeds
    task.status = TaskStatus::Completed;
    task.error = None;

    // Verify final state
    assert!(task.is_completed());
    assert_eq!(task.get_session_id(), Some(session_id.as_str()));
}

/// Test that non-existent session IDs are handled gracefully
#[test]
fn test_nonexistent_session_id_handled_gracefully() {
    let mut pool = SessionPool::new();
    let mut task = Task::new("task-1", "Test Task", "A test task");

    // Set a non-existent session ID
    task.set_session_id("nonexistent-session-id");

    // Should create a new session instead of failing
    let session = pool.get_or_create_for_task(&mut task);
    assert_ne!(session.session_id(), "nonexistent-session-id");
    assert_eq!(task.get_session_id(), Some(session.session_id()));
}

/// Test session continuity across multiple retry attempts
#[test]
fn test_session_continuity_across_multiple_retries() {
    let mut pool = SessionPool::new();
    let mut task = Task::new("task-1", "Test Task", "A test task");

    // Initial execution
    let session_initial = pool.get_or_create_for_task(&mut task);
    let original_session_id = session_initial.session_id().to_string();

    // Simulate multiple retries
    for retry_num in 1..=5 {
        task.status = TaskStatus::Failed;
        task.error = Some(format!("Attempt {} failed", retry_num));
        task.prepare_retry();

        let session = pool.get_or_create_for_task(&mut task);

        // Verify session continuity
        assert_eq!(session.session_id(), original_session_id);
        assert_eq!(task.get_session_id(), Some(original_session_id.as_str()));
        assert_eq!(task.retry_count, retry_num as u32);
        assert_eq!(session.reuse_count(), retry_num as u32);
    }

    // Final state verification
    assert_eq!(pool.len(), 1); // Only one session ever created
}

/// Test that task helper methods work correctly
#[test]
fn test_task_session_helper_methods() {
    let mut task = Task::new("task-1", "Test Task", "A test task");

    // Initially, task has no session
    assert!(!task.has_session());
    assert_eq!(task.get_session_id(), None);

    // Set a session ID
    task.set_session_id("test-session-123");
    assert!(task.has_session());
    assert_eq!(task.get_session_id(), Some("test-session-123"));

    // Clear the session ID
    task.clear_session_id();
    assert!(!task.has_session());
    assert_eq!(task.get_session_id(), None);
}

/// Test that task.prepare_retry() preserves session
#[test]
fn test_task_prepare_retry_preserves_session_and_increments_count() {
    let mut task = Task::new("task-1", "Test Task", "A test task");
    task.set_session_id("test-session-123");

    let session_id = task.get_session_id().unwrap().to_string();
    assert_eq!(task.retry_count, 0);
    assert_eq!(task.status, TaskStatus::Pending);

    task.prepare_retry();

    // Session should be preserved
    assert_eq!(task.get_session_id(), Some(session_id.as_str()));
    assert_eq!(task.retry_count, 1);
    assert_eq!(task.status, TaskStatus::Pending);
    assert!(task.started_at.is_none()); // Should be reset
}

/// Test that fresh sessions are not stale
#[test]
fn test_fresh_session_is_not_stale() {
    let mut pool = SessionPool::new();
    let mut task = Task::new("task-1", "Test Task", "A test task");

    let session = pool.get_or_create_for_task(&mut task);

    // Fresh sessions should not be stale
    assert!(!session.is_stale());
    assert_eq!(session.reuse_count(), 0);
}

/// Test pool cleanup behavior
#[test]
fn test_pool_cleanup_with_fresh_sessions() {
    let mut pool = SessionPool::new();

    // Create sessions for tasks
    let mut task1 = Task::new("task-1", "Task 1", "First task");
    let mut task2 = Task::new("task-2", "Task 2", "Second task");

    pool.get_or_create_for_task(&mut task1);
    pool.get_or_create_for_task(&mut task2);

    // Both tasks share the same session (same agent/model)
    assert_eq!(pool.len(), 1);

    // Cleanup fresh sessions (should not remove any)
    let removed = pool.cleanup_stale();
    assert_eq!(removed, 0);
    assert_eq!(pool.len(), 1);

    // Note: We cannot make sessions stale from integration tests
    // because the sessions field is private. The unit tests in
    // pool.rs cover actual staleness cleanup behavior.
}
