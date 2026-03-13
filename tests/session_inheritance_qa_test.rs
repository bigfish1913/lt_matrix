//! QA tests for dependent task session inheritance
//!
//! These tests provide comprehensive coverage of edge cases, integration scenarios,
//! and boundary conditions for the session inheritance feature beyond the basic
//! functionality tests in session_inheritance_test.rs.

use ltmatrix::agent::backend::MemorySession;
use ltmatrix::agent::pool::SessionPool;
use ltmatrix::agent::AgentSession;
use ltmatrix::models::{Task, TaskComplexity, TaskStatus};

// ============================================================================
// Edge Cases - Boundary Conditions
// ============================================================================

#[test]
fn qa_inheritance_with_empty_parent_session_id() {
    let mut pool = SessionPool::new();
    let mut child_task = Task::new("child", "Child", "Child task");

    // Set parent_session_id to empty string
    child_task.set_parent_session_id("");

    // Should treat empty string as None and create new session
    let session = pool.get_or_create_for_task(&mut child_task);

    assert!(child_task.has_session());
    assert!(
        child_task.get_parent_session_id().is_none()
            || child_task.get_parent_session_id() == Some("")
    );
    assert!(!session.session_id().is_empty());
}

#[test]
fn qa_inheritance_with_whitespace_parent_session_id() {
    let mut pool = SessionPool::new();
    let mut child_task = Task::new("child", "Child", "Child task");

    // Set parent_session_id to whitespace
    child_task.set_parent_session_id("   ");

    // Should create new session for invalid parent session ID
    let session = pool.get_or_create_for_task(&mut child_task);

    assert!(child_task.has_session());
    assert_ne!(child_task.get_session_id(), Some("   "));
    assert!(!session.session_id().is_empty());
}

#[test]
fn qa_inheritance_with_very_long_parent_session_id() {
    let mut pool = SessionPool::new();
    let mut child_task = Task::new("child", "Child", "Child task");

    // Create a very long parent session ID (10,000 characters)
    let long_id = "x".repeat(10_000);
    child_task.set_parent_session_id(&long_id);

    // Should handle gracefully and create new session
    let session = pool.get_or_create_for_task(&mut child_task);

    assert!(child_task.has_session());
    assert_ne!(child_task.get_session_id().map(|s| s.len()), Some(10_000));
    assert!(!session.session_id().is_empty());
}

#[test]
fn qa_inheritance_with_special_characters_in_session_id() {
    let mut pool = SessionPool::new();

    // Create a custom session with special characters
    let special_session = MemorySession {
        session_id: "session-with-special-chars-!@#$%^&*()".to_string(),
        agent_name: "claude".to_string(),
        model: "claude-sonnet-4-6".to_string(),
        ..Default::default()
    };
    pool.register(special_session.clone());

    // Child task should inherit the special character session
    let mut child_task = Task::new("child", "Child", "Child task");
    child_task.set_parent_session_id(&special_session.session_id);

    let child_session = pool.get_or_create_for_task(&mut child_task);

    assert_eq!(child_session.session_id(), special_session.session_id);
    assert_eq!(
        child_task.get_session_id(),
        Some(special_session.session_id.as_str())
    );
}

// ============================================================================
// Edge Cases - Stale Session Handling
// ============================================================================

#[test]
fn qa_inheritance_with_stale_parent_session_creates_new() {
    let mut pool = SessionPool::new();

    // Create a stale parent session
    let mut stale_session = MemorySession::default();
    stale_session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(4000);
    let stale_session_id = stale_session.session_id.clone();
    pool.register(stale_session);

    // Child task with stale parent session should get new session
    let mut child_task = Task::new("child", "Child", "Child task");
    child_task.set_parent_session_id(&stale_session_id);

    let child_session = pool.get_or_create_for_task(&mut child_task);

    // Should create new session, not inherit stale one
    assert_ne!(child_session.session_id(), stale_session_id);
    assert!(
        child_task.get_parent_session_id().is_none(),
        "Parent session ID should be cleared when stale"
    );
    assert_eq!(
        child_task.get_session_id(),
        Some(child_session.session_id())
    );
}

#[test]
fn qa_inheritance_with_exactly_one_hour_old_session() {
    let mut pool = SessionPool::new();

    // Create a parent session and make it exactly 1 hour old (boundary condition)
    let mut parent_task = Task::new("parent", "Parent", "Parent task");
    let parent_session = pool.get_or_create_for_task(&mut parent_task);
    let parent_session_id = parent_session.session_id().to_string();

    // Make the parent session exactly 1 hour old (stale boundary)
    let mut stale_session = (*pool.get(&parent_session_id).unwrap()).clone();
    stale_session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(3600);
    pool.register(stale_session); // Replace with stale version

    // Child task with exactly 1-hour-old parent session
    let mut child_task = Task::new("child", "Child", "Child task");
    child_task.set_parent_session_id(&parent_session_id);

    pool.get_or_create_for_task(&mut child_task);

    // At exactly 1 hour (3600 seconds), the session is NOT considered stale
    // because the staleness check is > 3600, not >= 3600
    // Therefore, the child should inherit the parent session
    assert_eq!(
        child_task.get_session_id(),
        Some(parent_session_id.as_str()),
        "Exactly 1-hour-old session should be inherited (not stale yet)"
    );
}

#[test]
fn qa_inheritance_with_just_under_one_hour_old_session() {
    let mut pool = SessionPool::new();

    // Create a session just under 1 hour old (3599 seconds - boundary condition)
    let mut fresh_session = MemorySession::default();
    fresh_session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(3599);
    let fresh_session_id = fresh_session.session_id.clone();
    pool.register(fresh_session);

    // Child task should inherit session that's just under 1 hour old
    let mut child_task = Task::new("child", "Child", "Child task");
    child_task.set_parent_session_id(&fresh_session_id);

    let child_session = pool.get_or_create_for_task(&mut child_task);

    // Should inherit the session
    assert_eq!(child_session.session_id(), fresh_session_id);
    assert_eq!(child_task.get_session_id(), Some(fresh_session_id.as_str()));
    assert_eq!(child_session.reuse_count(), 1); // Should be marked as accessed
}

// ============================================================================
// Edge Cases - Multiple Dependencies
// ============================================================================

#[test]
fn qa_inheritance_with_three_level_dependency_chain() {
    let mut pool = SessionPool::new();

    // Create three-level dependency chain: A -> B -> C -> D
    let mut task_a = Task::new("task-a", "A", "Root task");
    pool.get_or_create_for_task(&mut task_a);
    let session_id_a = task_a.get_session_id().unwrap().to_string();

    // Task B depends on A
    let mut task_b = Task::new("task-b", "B", "Second level");
    task_b.depends_on = vec!["task-a".to_string()];
    task_b.set_parent_session_id(&session_id_a);
    pool.get_or_create_for_task(&mut task_b);

    // Task C depends on B
    let mut task_c = Task::new("task-c", "C", "Third level");
    task_c.depends_on = vec!["task-b".to_string()];
    task_c.set_parent_session_id(&session_id_a); // Inherits from root
    pool.get_or_create_for_task(&mut task_c);

    // Task D depends on C
    let mut task_d = Task::new("task-d", "D", "Fourth level");
    task_d.depends_on = vec!["task-c".to_string()];
    task_d.set_parent_session_id(&session_id_a); // Inherits from root
    pool.get_or_create_for_task(&mut task_d);

    // All tasks should use the same session
    assert_eq!(task_a.get_session_id(), Some(session_id_a.as_str()));
    assert_eq!(task_b.get_session_id(), Some(session_id_a.as_str()));
    assert_eq!(task_c.get_session_id(), Some(session_id_a.as_str()));
    assert_eq!(task_d.get_session_id(), Some(session_id_a.as_str()));

    // Verify reuse counts increased appropriately
    let session = pool.get(&session_id_a).unwrap();
    assert_eq!(session.reuse_count(), 3); // B, C, D accessed it
}

#[test]
fn qa_inheritance_with_diamond_dependency_structure() {
    let mut pool = SessionPool::new();

    // Create diamond dependency: A -> B, A -> C, B -> D, C -> D
    let mut task_a = Task::new("task-a", "A", "Root task");
    let session_a = pool.get_or_create_for_task(&mut task_a);
    let session_id_a = session_a.session_id().to_string();

    // Task B depends on A
    let mut task_b = Task::new("task-b", "B", "Branch 1");
    task_b.depends_on = vec!["task-a".to_string()];
    task_b.set_parent_session_id(&session_id_a);
    pool.get_or_create_for_task(&mut task_b);

    // Task C depends on A
    let mut task_c = Task::new("task-c", "C", "Branch 2");
    task_c.depends_on = vec!["task-a".to_string()];
    task_c.set_parent_session_id(&session_id_a);
    pool.get_or_create_for_task(&mut task_c);

    // Task D depends on both B and C (diamond join)
    let mut task_d = Task::new("task-d", "D", "Join task");
    task_d.depends_on = vec!["task-b".to_string(), "task-c".to_string()];
    task_d.set_parent_session_id(&session_id_a); // Inherits from root A
    let session_d = pool.get_or_create_for_task(&mut task_d);

    // D should inherit from A
    assert_eq!(task_d.get_session_id(), Some(session_id_a.as_str()));

    // Session should have been accessed multiple times
    let session = pool.get(&session_id_a).unwrap();
    assert!(session.reuse_count() >= 3); // B, C, D each accessed it once
}

#[test]
fn qa_inheritance_with_multiple_children_same_parent() {
    let mut pool = SessionPool::new();

    // Create parent task
    let mut parent = Task::new("parent", "Parent", "Parent task");
    let parent_session = pool.get_or_create_for_task(&mut parent);
    let parent_session_id = parent_session.session_id().to_string();

    // Create multiple children that all inherit from same parent
    let mut child1 = Task::new("child1", "Child 1", "First child");
    child1.set_parent_session_id(&parent_session_id);
    pool.get_or_create_for_task(&mut child1);

    let mut child2 = Task::new("child2", "Child 2", "Second child");
    child2.set_parent_session_id(&parent_session_id);
    pool.get_or_create_for_task(&mut child2);

    let mut child3 = Task::new("child3", "Child 3", "Third child");
    child3.set_parent_session_id(&parent_session_id);
    pool.get_or_create_for_task(&mut child3);

    // All children should use the same session as parent
    assert_eq!(child1.get_session_id(), Some(parent_session_id.as_str()));
    assert_eq!(child2.get_session_id(), Some(parent_session_id.as_str()));
    assert_eq!(child3.get_session_id(), Some(parent_session_id.as_str()));

    // Session should have been accessed multiple times
    let session = pool.get(&parent_session_id).unwrap();
    assert_eq!(session.reuse_count(), 3); // child1, child2, child3
}

// ============================================================================
// Edge Cases - Session State Transitions
// ============================================================================

#[test]
fn qa_inheritance_task_status_transitions_preserve_inheritance() {
    let mut pool = SessionPool::new();

    // Create parent task
    let mut parent = Task::new("parent", "Parent", "Parent task");
    let parent_session = pool.get_or_create_for_task(&mut parent);
    let parent_session_id = parent_session.session_id().to_string();

    // Create child task that inherits session
    let mut child = Task::new("child", "Child", "Child task");
    child.set_parent_session_id(&parent_session_id);
    pool.get_or_create_for_task(&mut child);

    // Simulate task status transitions
    child.status = TaskStatus::InProgress;
    let session1 = pool.get_or_create_for_task(&mut child);
    assert_eq!(session1.session_id(), parent_session_id);

    child.status = TaskStatus::Failed;
    let session2 = pool.get_or_create_for_task(&mut child);
    assert_eq!(session2.session_id(), parent_session_id);

    child.status = TaskStatus::Pending;
    child.prepare_retry();
    let session3 = pool.get_or_create_for_task(&mut child);
    assert_eq!(session3.session_id(), parent_session_id);

    child.status = TaskStatus::Completed;
    let session4 = pool.get_or_create_for_task(&mut child);
    assert_eq!(session4.session_id(), parent_session_id);
}

#[test]
fn qa_inheritance_with_session_cleanup_between_retries() {
    let mut pool = SessionPool::new();

    // Create parent task
    let mut parent = Task::new("parent", "Parent", "Parent task");
    let parent_session = pool.get_or_create_for_task(&mut parent);
    let parent_session_id = parent_session.session_id().to_string();

    // Create child task that inherits session
    let mut child = Task::new("child", "Child", "Child task");
    child.set_parent_session_id(&parent_session_id);
    pool.get_or_create_for_task(&mut child);

    // Simulate failure and retry
    child.status = TaskStatus::Failed;
    child.prepare_retry();

    // Cleanup stale sessions (should not remove our inherited session)
    let _cleaned_count = pool.cleanup_stale();

    // Retry should still work with inherited session
    let retry_session = pool.get_or_create_for_task(&mut child);
    assert_eq!(retry_session.session_id(), parent_session_id);
}

// ============================================================================
// Integration - Task Properties
// ============================================================================

#[test]
fn qa_inheritance_with_different_task_complexities() {
    let mut pool = SessionPool::new();

    // Test inheritance works regardless of task complexity
    for complexity in &[
        TaskComplexity::Simple,
        TaskComplexity::Moderate,
        TaskComplexity::Complex,
    ] {
        let mut parent = Task::new("parent", "Parent", "Parent task");
        parent.complexity = complexity.clone();
        let parent_session = pool.get_or_create_for_task(&mut parent);
        let parent_session_id = parent_session.session_id().to_string();

        let mut child = Task::new("child", "Child", "Child task");
        child.complexity = complexity.clone();
        child.set_parent_session_id(&parent_session_id);

        let child_session = pool.get_or_create_for_task(&mut child);

        assert_eq!(child_session.session_id(), parent_session_id);
    }
}

#[test]
fn qa_inheritance_with_task_serialization_roundtrip() {
    let mut pool = SessionPool::new();

    // Create parent task
    let mut parent = Task::new("parent", "Parent", "Parent task");
    let parent_session = pool.get_or_create_for_task(&mut parent);
    let parent_session_id = parent_session.session_id().to_string();

    // Create child task with inheritance
    let mut child = Task::new("child", "Child", "Child task");
    child.set_parent_session_id(&parent_session_id);

    // Serialize and deserialize
    let json = serde_json::to_string(&child).expect("Serialization should succeed");
    let mut deserialized: Task =
        serde_json::from_str(&json).expect("Deserialization should succeed");

    // Deserialized task should preserve parent_session_id
    assert_eq!(
        deserialized.get_parent_session_id(),
        Some(parent_session_id.as_str())
    );

    // Deserialized task should be able to inherit session
    let child_session = pool.get_or_create_for_task(&mut deserialized);
    assert_eq!(child_session.session_id(), parent_session_id);
}

#[test]
fn qa_inheritance_with_task_cloning() {
    let mut pool = SessionPool::new();

    // Create parent task
    let mut parent = Task::new("parent", "Parent", "Parent task");
    let parent_session = pool.get_or_create_for_task(&mut parent);
    let parent_session_id = parent_session.session_id().to_string();

    // Create child task and clone it
    let mut child1 = Task::new("child", "Child", "Child task");
    child1.set_parent_session_id(&parent_session_id);

    let mut child2 = child1.clone();

    // Both clones should have the same parent_session_id
    assert_eq!(
        child1.get_parent_session_id(),
        child2.get_parent_session_id()
    );

    // Both should be able to inherit the session
    pool.get_or_create_for_task(&mut child1);
    pool.get_or_create_for_task(&mut child2);

    assert_eq!(child1.get_session_id(), Some(parent_session_id.as_str()));
    assert_eq!(child2.get_session_id(), Some(parent_session_id.as_str()));
}

// ============================================================================
// Performance and Stress Tests
// ============================================================================

#[test]
fn qa_inheritance_performance_with_many_tasks() {
    let mut pool = SessionPool::new();

    // Create parent task
    let mut parent = Task::new("parent", "Parent", "Parent task");
    let parent_session = pool.get_or_create_for_task(&mut parent);
    let parent_session_id = parent_session.session_id().to_string();

    // Create many child tasks that inherit from parent
    let num_children = 100;
    for i in 0..num_children {
        let mut child = Task::new(
            &format!("child-{}", i),
            &format!("Child {}", i),
            "Child task",
        );
        child.set_parent_session_id(&parent_session_id);
        pool.get_or_create_for_task(&mut child);
    }

    // All children should use the parent's session
    let session = pool.get(&parent_session_id).unwrap();
    assert_eq!(session.reuse_count(), num_children);
    assert_eq!(pool.len(), 1); // Only one session in the pool
}

#[test]
fn qa_inheritance_with_rapid_succession_access() {
    let mut pool = SessionPool::new();

    // Create parent task
    let mut parent = Task::new("parent", "Parent", "Parent task");
    let parent_session = pool.get_or_create_for_task(&mut parent);
    let parent_session_id = parent_session.session_id().to_string();

    // Rapidly create child tasks in succession
    let mut session_ids = Vec::new();
    for i in 0..10 {
        let mut child = Task::new(
            &format!("child-{}", i),
            &format!("Child {}", i),
            "Child task",
        );
        child.set_parent_session_id(&parent_session_id);
        let session = pool.get_or_create_for_task(&mut child);
        session_ids.push(session.session_id().to_string());
    }

    // All should use the same session
    for session_id in session_ids {
        assert_eq!(session_id, parent_session_id);
    }
}

// ============================================================================
// Error Recovery
// ============================================================================

#[test]
fn qa_inheritance_recovery_after_nonexistent_parent() {
    let mut pool = SessionPool::new();

    // Try to inherit from non-existent parent
    let mut child1 = Task::new("child1", "Child 1", "Child task");
    child1.set_parent_session_id("nonexistent-session");
    pool.get_or_create_for_task(&mut child1);

    // parent_session_id should be cleared
    assert!(child1.get_parent_session_id().is_none());

    // Now create a real parent
    let mut parent = Task::new("parent", "Parent", "Parent task");
    pool.get_or_create_for_task(&mut parent);
    let parent_session_id = parent.get_session_id().unwrap().to_string();

    // Create another child that successfully inherits
    let mut child2 = Task::new("child2", "Child 2", "Child task");
    child2.set_parent_session_id(&parent_session_id);
    pool.get_or_create_for_task(&mut child2);

    assert_eq!(child2.get_session_id(), Some(parent_session_id.as_str()));
    // Note: child1 and child2 may have the same session if using same agent/model pair
    // because get_or_create reuses sessions for the same (agent, model) pair
}

#[test]
fn qa_inheritance_with_session_removal_during_execution() {
    let mut pool = SessionPool::new();

    // Create parent task
    let mut parent = Task::new("parent", "Parent", "Parent task");
    let parent_session = pool.get_or_create_for_task(&mut parent);
    let parent_session_id = parent_session.session_id().to_string();

    // Create child task that inherits session
    let mut child = Task::new("child", "Child", "Child task");
    child.set_parent_session_id(&parent_session_id);
    pool.get_or_create_for_task(&mut child);

    // Remove the parent session
    pool.remove(&parent_session_id);

    // Try to use child again - should create new session
    let new_session = pool.get_or_create_for_task(&mut child);

    assert_ne!(new_session.session_id(), parent_session_id);
    assert_eq!(child.get_session_id(), Some(new_session.session_id()));
}
