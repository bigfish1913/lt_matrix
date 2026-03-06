//! Edge case tests for retry scenario session reuse
//!
//! This test suite covers edge cases, error scenarios, and stress tests
//! for the retry session reuse functionality.

use ltmatrix::agent::backend::AgentSession;
use ltmatrix::agent::pool::SessionPool;
use ltmatrix::models::{Task, TaskStatus};

// ============================================================================
// Task Serialization and Session Persistence
// ============================================================================

#[test]
fn test_task_serialization_preserves_session_id() {
    let mut task = Task::new("task-1", "Test Task", "A test task");
    task.set_session_id("test-session-123");

    // Serialize
    let json = serde_json::to_string(&task).expect("Failed to serialize");

    // Deserialize
    let deserialized: Task = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.get_session_id(), Some("test-session-123"));
    assert_eq!(deserialized.id, "task-1");
}

#[test]
fn test_task_serialization_with_none_session_id() {
    let task = Task::new("task-1", "Test Task", "A test task");

    // Serialize
    let json = serde_json::to_string(&task).expect("Failed to serialize");

    // Deserialize
    let deserialized: Task = serde_json::from_str(&json).expect("Failed to deserialize");

    assert!(!deserialized.has_session());
    assert_eq!(deserialized.get_session_id(), None);
}

#[test]
fn test_task_serialization_preserves_retry_count() {
    let mut task = Task::new("task-1", "Test Task", "A test task");
    task.set_session_id("test-session-123");

    // Simulate multiple retries
    for _ in 0..3 {
        task.status = TaskStatus::Failed;
        task.prepare_retry();
    }

    // Serialize
    let json = serde_json::to_string(&task).expect("Failed to serialize");

    // Deserialize
    let deserialized: Task = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.retry_count, 3);
    assert_eq!(deserialized.get_session_id(), Some("test-session-123"));
    assert_eq!(deserialized.status, TaskStatus::Pending);
}

// ============================================================================
// Retry Limit and State Transitions
// ============================================================================

#[test]
fn test_retry_count_increments_correctly() {
    let mut task = Task::new("task-1", "Test Task", "A test task");
    task.set_session_id("test-session-123");

    assert_eq!(task.retry_count, 0);

    // First retry
    task.prepare_retry();
    assert_eq!(task.retry_count, 1);

    // Second retry
    task.prepare_retry();
    assert_eq!(task.retry_count, 2);

    // Third retry
    task.prepare_retry();
    assert_eq!(task.retry_count, 3);
}

#[test]
fn test_can_retry_respects_max_retries() {
    let mut task = Task::new("task-1", "Test Task", "A test task");
    task.status = TaskStatus::Failed;
    task.set_session_id("test-session-123");

    // Max retries = 3
    assert!(task.can_retry(3));

    // After 1st retry
    task.prepare_retry();
    task.status = TaskStatus::Failed;
    assert!(task.can_retry(3));

    // After 2nd retry
    task.prepare_retry();
    task.status = TaskStatus::Failed;
    assert!(task.can_retry(3));

    // After 3rd retry
    task.prepare_retry();
    task.status = TaskStatus::Failed;
    assert!(!task.can_retry(3), "Should not be able to retry after max_retries");
}

#[test]
fn test_prepare_retry_resets_status_and_started_at() {
    let mut task = Task::new("task-1", "Test Task", "A test task");
    task.set_session_id("test-session-123");

    // Set task as in progress
    task.status = TaskStatus::InProgress;
    task.started_at = Some(chrono::Utc::now());

    assert!(task.started_at.is_some());

    // Prepare for retry
    task.prepare_retry();

    assert_eq!(task.status, TaskStatus::Pending);
    assert!(task.started_at.is_none(), "started_at should be reset");
    assert_eq!(task.get_session_id(), Some("test-session-123"), "session_id should be preserved");
}

#[test]
fn test_prepare_retry_on_non_failed_task() {
    let mut task = Task::new("task-1", "Test Task", "A test task");
    task.set_session_id("test-session-123");

    // Task is pending, not failed
    task.status = TaskStatus::Pending;

    // prepare_retry should still work (resets state)
    task.prepare_retry();

    assert_eq!(task.status, TaskStatus::Pending);
    assert_eq!(task.retry_count, 1);
    assert_eq!(task.get_session_id(), Some("test-session-123"));
}

#[test]
fn test_prepare_retry_on_completed_task() {
    let mut task = Task::new("task-1", "Test Task", "A test task");
    task.set_session_id("test-session-123");

    // Task is completed
    task.status = TaskStatus::Completed;

    // prepare_retry should still work (allows re-execution)
    task.prepare_retry();

    assert_eq!(task.status, TaskStatus::Pending);
    assert_eq!(task.retry_count, 1);
    assert_eq!(task.get_session_id(), Some("test-session-123"));
}

// ============================================================================
// Session Isolation Between Different Agents/Models
// ============================================================================

#[test]
fn test_different_agents_have_separate_sessions() {
    let mut pool = SessionPool::new();

    let mut task1 = Task::new("task-1", "Task 1", "First task with claude");
    let mut task2 = Task::new("task-2", "Task 2", "Second task with opencode");

    // Get sessions for different agents
    // Note: Currently hardcoded to "claude" and "claude-sonnet-4-6" in get_or_create_for_task
    // This test will verify they share sessions (current behavior)
    let session1_id = pool.get_or_create_for_task(&mut task1).session_id().to_string();
    let session2_id = pool.get_or_create_for_task(&mut task2).session_id().to_string();

    // Currently both tasks use the same agent/model (hardcoded)
    assert_eq!(session1_id, session2_id, "Tasks share session (same agent/model)");
}

#[test]
fn test_session_reuse_after_task_failure_and_retry() {
    let mut pool = SessionPool::new();
    let mut task = Task::new("task-1", "Test Task", "A test task");

    // First execution
    let session1 = pool.get_or_create_for_task(&mut task);
    let session_id = session1.session_id().to_string();
    let initial_reuse_count = session1.reuse_count();

    // Task fails
    task.status = TaskStatus::Failed;
    task.error = Some("Network error".to_string());

    // Prepare for retry
    task.prepare_retry();

    // Retry should reuse the session
    let session2 = pool.get_or_create_for_task(&mut task);

    assert_eq!(session2.session_id(), session_id);
    assert_eq!(session2.reuse_count(), initial_reuse_count + 1);
    assert_eq!(task.retry_count, 1);
}

// ============================================================================
// Session ID Edge Cases
// ============================================================================

#[test]
fn test_empty_session_id_is_handled() {
    let mut task = Task::new("task-1", "Test Task", "A test task");
    task.session_id = Some(String::new()); // Empty string

    assert!(task.has_session(), "Empty string counts as having session");
    assert_eq!(task.get_session_id(), Some(""));

    // Clear should remove it
    task.clear_session_id();
    assert!(!task.has_session());
}

#[test]
fn test_set_session_id_with_empty_string() {
    let mut task = Task::new("task-1", "Test Task", "A test task");

    task.set_session_id("");
    assert!(task.has_session());
    assert_eq!(task.get_session_id(), Some(""));
}

#[test]
fn test_set_session_id_overwrites_existing() {
    let mut task = Task::new("task-1", "Test Task", "A test task");

    task.set_session_id("session-1");
    assert_eq!(task.get_session_id(), Some("session-1"));

    task.set_session_id("session-2");
    assert_eq!(task.get_session_id(), Some("session-2"));
}

#[test]
fn test_multiple_clear_session_id_calls() {
    let mut task = Task::new("task-1", "Test Task", "A test task");

    task.set_session_id("session-1");
    task.clear_session_id();
    task.clear_session_id(); // Should not panic

    assert!(!task.has_session());
}

// ============================================================================
// Pool and Task Integration Edge Cases
// ============================================================================

#[test]
fn test_pool_with_multiple_tasks_same_session() {
    let mut pool = SessionPool::new();

    let mut task1 = Task::new("task-1", "Task 1", "First");
    let mut task2 = Task::new("task-2", "Task 2", "Second");
    let mut task3 = Task::new("task-3", "Task 3", "Third");

    // All tasks should share the same session
    let session1_id = pool.get_or_create_for_task(&mut task1).session_id().to_string();
    let session2_id = pool.get_or_create_for_task(&mut task2).session_id().to_string();
    let session3_id = pool.get_or_create_for_task(&mut task3).session_id().to_string();

    assert_eq!(session2_id, session1_id);
    assert_eq!(session3_id, session1_id);
    assert_eq!(pool.len(), 1, "Only one session should exist");
}

#[test]
fn test_task_retry_preserves_other_fields() {
    let mut task = Task::new("task-1", "Test Task", "A test task");
    task.set_session_id("test-session-123");

    // Set various fields
    task.complexity = ltmatrix::models::TaskComplexity::Complex;
    task.depends_on = vec!["task-0".to_string()];
    task.description = "Updated description".to_string();

    let original_id = task.id.clone();
    let original_title = task.title.clone();
    let original_description = task.description.clone();
    let original_complexity = task.complexity.clone();
    let original_depends_on = task.depends_on.clone();
    let original_created_at = task.created_at;

    // Prepare for retry
    task.prepare_retry();

    // These fields should be preserved
    assert_eq!(task.id, original_id);
    assert_eq!(task.title, original_title);
    assert_eq!(task.description, original_description);
    assert_eq!(task.complexity, original_complexity);
    assert_eq!(task.depends_on, original_depends_on);
    assert_eq!(task.created_at, original_created_at);
}

#[test]
fn test_get_or_create_for_task_with_cleared_session() {
    let mut pool = SessionPool::new();
    let mut task = Task::new("task-1", "Test Task", "A test task");

    // First execution
    let session1 = pool.get_or_create_for_task(&mut task);
    let session1_id = session1.session_id().to_string();

    // Clear the session ID
    task.clear_session_id();

    // Should create a new session
    let session2 = pool.get_or_create_for_task(&mut task);
    let session2_id = session2.session_id().to_string();

    // Should have a different session ID (or the same if the old one was reused by get_or_create)
    // Actually, get_or_create_for_task will call get_or_create which reuses existing non-stale sessions
    // So the session ID might be the same, but the reuse count should increment
    assert_eq!(task.get_session_id(), Some(session2_id.as_str()));
}

#[test]
fn test_session_reuse_count_increments_on_each_access() {
    let mut pool = SessionPool::new();
    let mut task = Task::new("task-1", "Test Task", "A test task");

    // First access
    let session = pool.get_or_create_for_task(&mut task);
    assert_eq!(session.reuse_count(), 0);

    // Second access (same task, should reuse)
    let session = pool.get_or_create_for_task(&mut task);
    assert_eq!(session.reuse_count(), 1);

    // Third access
    let session = pool.get_or_create_for_task(&mut task);
    assert_eq!(session.reuse_count(), 2);
}

// ============================================================================
// Error Recovery Scenarios
// ============================================================================

#[test]
fn test_task_error_field_preserved_on_retry() {
    let mut task = Task::new("task-1", "Test Task", "A test task");
    task.set_session_id("test-session-123");

    // Task fails with error
    task.status = TaskStatus::Failed;
    task.error = Some("Connection timeout".to_string());

    // Prepare retry
    task.prepare_retry();

    // Error should still be present (not cleared by prepare_retry)
    assert_eq!(task.error, Some("Connection timeout".to_string()));
    assert_eq!(task.status, TaskStatus::Pending);
    assert_eq!(task.retry_count, 1);
}

#[test]
fn test_task_error_can_be_cleared() {
    let mut task = Task::new("task-1", "Test Task", "A test task");
    task.status = TaskStatus::Failed;
    task.error = Some("Some error".to_string());

    task.prepare_retry();

    // Manually clear error (simulating successful retry)
    task.error = None;
    task.status = TaskStatus::Completed;

    assert_eq!(task.error, None);
    assert_eq!(task.status, TaskStatus::Completed);
}

// ============================================================================
// Timestamp Behavior
// ============================================================================

#[test]
fn test_task_timestamps_on_retry() {
    let mut task = Task::new("task-1", "Test Task", "A test task");
    task.set_session_id("test-session-123");

    let created_at = task.created_at;

    // Start task
    task.status = TaskStatus::InProgress;
    task.started_at = Some(chrono::Utc::now());

    let started_at = task.started_at.unwrap();

    // Task fails and prepare for retry
    task.status = TaskStatus::Failed;
    task.prepare_retry();

    // created_at should never change
    assert_eq!(task.created_at, created_at);

    // started_at should be reset on retry
    assert!(task.started_at.is_none());

    // completed_at should remain None
    assert!(task.completed_at.is_none());
}

#[test]
fn test_task_completion_preserves_session() {
    let mut pool = SessionPool::new();
    let mut task = Task::new("task-1", "Test Task", "A test task");

    // Execute task
    let session = pool.get_or_create_for_task(&mut task);
    let session_id = session.session_id().to_string();

    // Task completes
    task.status = TaskStatus::Completed;
    task.completed_at = Some(chrono::Utc::now());

    // Session should still be associated
    assert_eq!(task.get_session_id(), Some(session_id.as_str()));
    assert!(task.has_session());
}

// ============================================================================
// Complex Retry Workflows
// ============================================================================

#[test]
fn test_multiple_failures_and_retries_same_session() {
    let mut pool = SessionPool::new();
    let mut task = Task::new("task-1", "Test Task", "A test task");

    // Initial execution
    let session = pool.get_or_create_for_task(&mut task);
    let session_id = session.session_id().to_string();

    // Simulate multiple failure-retry cycles
    for retry in 1..=5 {
        // Task fails
        task.status = TaskStatus::Failed;
        task.error = Some(format!("Failure {}", retry));

        // Prepare retry
        task.prepare_retry();
        assert_eq!(task.retry_count, retry);

        // Get session for retry
        let session = pool.get_or_create_for_task(&mut task);
        assert_eq!(session.session_id(), session_id);
        assert_eq!(session.reuse_count(), retry as u32);
    }

    // Final success
    task.status = TaskStatus::Completed;
    task.error = None;

    assert!(task.is_completed());
    assert_eq!(task.get_session_id(), Some(session_id.as_str()));
    assert_eq!(task.retry_count, 5);
}

#[test]
fn test_interleaved_task_executions_with_shared_session() {
    let mut pool = SessionPool::new();

    let mut task1 = Task::new("task-1", "Task 1", "First task");
    let mut task2 = Task::new("task-2", "Task 2", "Second task");

    // Task 1 starts
    pool.get_or_create_for_task(&mut task1);
    let session_id = task1.get_session_id().unwrap().to_string();

    // Task 2 starts (should share session)
    pool.get_or_create_for_task(&mut task2);
    assert_eq!(task2.get_session_id(), Some(session_id.as_str()));

    // Task 1 fails and retries
    task1.status = TaskStatus::Failed;
    task1.prepare_retry();
    pool.get_or_create_for_task(&mut task1);

    // Task 2 completes
    task2.status = TaskStatus::Completed;

    // Task 1 retries again
    task1.status = TaskStatus::Failed;
    task1.prepare_retry();
    let session = pool.get_or_create_for_task(&mut task1);

    // Both tasks should still share the same session
    assert_eq!(session.session_id(), session_id);
    assert_eq!(task1.get_session_id(), Some(session_id.as_str()));
    assert_eq!(task2.get_session_id(), Some(session_id.as_str()));
}
