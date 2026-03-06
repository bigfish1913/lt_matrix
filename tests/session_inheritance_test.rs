//! Session inheritance tests for dependent tasks
//!
//! These tests verify that child tasks can inherit sessions from their
//! parent dependencies, enabling efficient context sharing across the
//! dependency chain.

use ltmatrix::agent::pool::SessionPool;
use ltmatrix::agent::AgentSession;
use ltmatrix::models::{Task, TaskStatus};

// ============================================================================
// Task Model - Parent Session Reference
// ============================================================================

#[test]
fn task_model_has_parent_session_id_field() {
    // This test verifies the Task model has a parent_session_id field
    let task = Task::new("task-1", "Test", "Description");

    // Task should have a parent_session_id field that is Optional
    // and defaults to None
    let parent_session_id = task.get_parent_session_id();
    assert!(
        parent_session_id.is_none(),
        "New task should not have a parent session ID"
    );
}

#[test]
fn task_model_can_set_parent_session_id() {
    let mut task = Task::new("task-1", "Test", "Description");

    // Should be able to set a parent session ID
    task.set_parent_session_id("parent-session-123");

    assert_eq!(
        task.get_parent_session_id(),
        Some("parent-session-123"),
        "Parent session ID should be set"
    );
}

#[test]
fn task_model_can_clear_parent_session_id() {
    let mut task = Task::new("task-1", "Test", "Description");
    task.set_parent_session_id("parent-session-123");

    assert!(task.get_parent_session_id().is_some());

    // Should be able to clear parent session ID
    task.clear_parent_session_id();

    assert!(
        task.get_parent_session_id().is_none(),
        "Parent session ID should be cleared"
    );
}

#[test]
fn task_model_serialization_includes_parent_session_id() {
    let mut task = Task::new("task-1", "Test", "Description");
    task.set_parent_session_id("parent-session-456");

    // Serialize to JSON
    let json = serde_json::to_string(&task).expect("Serialization should succeed");

    // JSON should include parent_session_id field
    assert!(
        json.contains("parent_session_id"),
        "Serialized JSON should include parent_session_id field"
    );
    assert!(
        json.contains("parent-session-456"),
        "Serialized JSON should include parent session ID value"
    );

    // Deserialize and verify
    let deserialized: Task = serde_json::from_str(&json).expect("Deserialization should succeed");
    assert_eq!(
        deserialized.get_parent_session_id(),
        Some("parent-session-456"),
        "Deserialized task should preserve parent session ID"
    );
}

// ============================================================================
// SessionPool - Session Inheritance Support
// ============================================================================

#[test]
fn session_pool_get_or_create_for_task_with_parent_session() {
    let mut pool = SessionPool::new();

    // Create parent task with session
    let mut parent_task = Task::new("parent-task", "Parent", "Parent task");
    let parent_session = pool.get_or_create_for_task(&mut parent_task);
    let parent_session_id = parent_session.session_id().to_string();

    // Create child task that references parent's session
    let mut child_task = Task::new("child-task", "Child", "Child task");
    child_task.set_parent_session_id(&parent_session_id);

    // Child task should inherit parent's session
    let child_session = pool.get_or_create_for_task(&mut child_task);

    assert_eq!(
        child_session.session_id(),
        parent_session_id,
        "Child task should inherit parent's session ID"
    );

    // Both tasks should use the same session
    assert_eq!(
        child_task.get_session_id(),
        Some(parent_session_id.as_str()),
        "Child task's session ID should match parent's"
    );
}

#[test]
fn session_pool_creates_new_session_if_parent_not_found() {
    let mut pool = SessionPool::new();

    // Create child task with non-existent parent session
    let mut child_task = Task::new("child-task", "Child", "Child task");
    child_task.set_parent_session_id("nonexistent-parent-session");

    // Should create a new session when parent doesn't exist
    let child_session = pool.get_or_create_for_task(&mut child_task);

    assert!(
        child_session.session_id() != "nonexistent-parent-session",
        "Should create new session when parent session not found"
    );

    // Child task should have the new session ID
    assert_eq!(
        child_task.get_session_id(),
        Some(child_session.session_id()),
        "Child task should be assigned the new session ID"
    );
}

#[test]
fn session_pool_get_or_create_for_task_without_parent_session() {
    let mut pool = SessionPool::new();

    // Create task without parent session reference
    let mut task = Task::new("task-1", "Test", "Description");

    // Should create a new session
    let session = pool.get_or_create_for_task(&mut task);

    assert!(task.has_session());
    assert_eq!(
        task.get_session_id(),
        Some(session.session_id()),
        "Task should be assigned a new session ID"
    );
}

// ============================================================================
// Integration - Dependency Chain Session Inheritance
// ============================================================================

#[test]
fn dependency_chain_inherits_session_from_parent() {
    let mut pool = SessionPool::new();

    // Create a dependency chain: task-1 -> task-2 -> task-3
    let mut task1 = Task::new("task-1", "First", "First task");
    let session1 = pool.get_or_create_for_task(&mut task1);
    let session_id_1 = session1.session_id().to_string();

    // task-2 depends on task-1 and should inherit its session
    let mut task2 = Task::new("task-2", "Second", "Second task");
    task2.depends_on = vec!["task-1".to_string()];
    task2.set_parent_session_id(&session_id_1);

    let session2 = pool.get_or_create_for_task(&mut task2);

    assert_eq!(
        session2.session_id(),
        session_id_1,
        "task-2 should inherit session from task-1"
    );

    // task-3 depends on task-2 and should also inherit the same session
    let mut task3 = Task::new("task-3", "Third", "Third task");
    task3.depends_on = vec!["task-2".to_string()];
    task3.set_parent_session_id(&session_id_1); // Inherits from original parent

    let session3 = pool.get_or_create_for_task(&mut task3);

    assert_eq!(
        session3.session_id(),
        session_id_1,
        "task-3 should also inherit the same session from task-1"
    );

    // All tasks should use the same session
    assert_eq!(
        task1.get_session_id(),
        task2.get_session_id(),
        "task-1 and task-2 should use same session"
    );
    assert_eq!(
        task2.get_session_id(),
        task3.get_session_id(),
        "task-2 and task-3 should use same session"
    );
}

#[test]
fn dependency_chain_with_multiple_parents_uses_first_parent_session() {
    let mut pool = SessionPool::new();

    // Create two parent tasks with different sessions
    let mut parent1 = Task::new("parent-1", "Parent 1", "First parent");
    let session1 = pool.get_or_create_for_task(&mut parent1);
    let session_id_1 = session1.session_id().to_string();

    let mut parent2 = Task::new("parent-2", "Parent 2", "Second parent");
    let session2 = pool.get_or_create_for_task(&mut parent2);
    let session_id_2 = session2.session_id().to_string();

    // Child task depends on both parents
    let mut child = Task::new("child", "Child", "Child task");
    child.depends_on = vec!["parent-1".to_string(), "parent-2".to_string()];

    // Set parent_session_id to first parent
    child.set_parent_session_id(&session_id_1);

    let child_session = pool.get_or_create_for_task(&mut child);

    // Should use the first parent's session
    assert_eq!(
        child_session.session_id(),
        session_id_1,
        "Child should use first parent's session"
    );
}

#[test]
fn dependency_chain_maintains_session_across_retries() {
    let mut pool = SessionPool::new();

    // Create parent task with session
    let mut parent = Task::new("parent", "Parent", "Parent task");
    let parent_session = pool.get_or_create_for_task(&mut parent);
    let parent_session_id = parent_session.session_id().to_string();

    // Create child task that inherits parent's session
    let mut child = Task::new("child", "Child", "Child task");
    child.depends_on = vec!["parent".to_string()];
    child.set_parent_session_id(&parent_session_id);

    // First execution
    let child_session1 = pool.get_or_create_for_task(&mut child);
    assert_eq!(child_session1.session_id(), parent_session_id);

    // Simulate child task failure and retry
    child.status = TaskStatus::Failed;
    child.prepare_retry();

    // Retry should reuse the same session
    let child_session2 = pool.get_or_create_for_task(&mut child);

    assert_eq!(
        child_session2.session_id(),
        parent_session_id,
        "Retry should reuse the same inherited session"
    );

    // Verify reuse count increased
    assert_eq!(
        pool.get(&parent_session_id).unwrap().reuse_count(),
        2, // Initial + retry
        "Session reuse count should increment on retry"
    );
}
