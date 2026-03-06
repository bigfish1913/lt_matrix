//! Acceptance tests for SessionPool core data structures
//!
//! These tests verify the complete acceptance criteria:
//! - AgentPool (SessionPool) struct with proper storage
//! - Session storage using HashMap
//! - Session state tracking (active/idle via staleness)
//! - SessionHandle (MemorySession) with required fields
//! - Parent-child relationship tracking (via agent/model keying)
//!
//! Tests are organized by acceptance criterion.

use ltmatrix::agent::backend::{AgentSession, MemorySession};
use ltmatrix::agent::pool::SessionPool;
use ltmatrix::agent::session::{SessionData, SessionManager};
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Acceptance Criterion 1: AgentPool struct exists with proper storage
// ============================================================================

#[test]
fn acceptance_1_1_session_pool_struct_exists() {
    // Verify SessionPool can be created
    let pool = SessionPool::new();
    assert_eq!(pool.len(), 0, "New pool should be empty");
}

#[test]
fn acceptance_1_2_session_pool_default_implemented() {
    // Verify Default trait is implemented
    let pool = SessionPool::default();
    assert!(pool.is_empty(), "Default pool should be empty");
}

#[test]
fn acceptance_1_3_session_pool_uses_hashmap_storage() {
    // Verify sessions are stored with HashMap semantics
    let mut pool = SessionPool::new();

    // Register multiple sessions
    let session1 = MemorySession::default();
    let id1 = session1.session_id.clone();
    pool.register(session1);

    let session2 = MemorySession::default();
    let id2 = session2.session_id.clone();
    pool.register(session2);

    // Verify both can be retrieved
    assert!(pool.get(&id1).is_some());
    assert!(pool.get(&id2).is_some());
    assert_eq!(pool.len(), 2);

    // Verify replacement behavior (HashMap semantics)
    let session3 = MemorySession {
        session_id: id1.clone(),
        agent_name: "different".to_string(),
        ..Default::default()
    };
    pool.register(session3);

    // Should still have 2 sessions (not 3)
    assert_eq!(pool.len(), 2);
    let retrieved = pool.get(&id1).unwrap();
    assert_eq!(retrieved.agent_name, "different");
}

// ============================================================================
// Acceptance Criterion 2: SessionHandle with required fields
// ============================================================================

#[test]
fn acceptance_2_1_memory_session_has_session_id() {
    let session = MemorySession::default();
    assert!(!session.session_id.is_empty(), "Session ID should not be empty");
    assert!(
        session.session_id.len() == 36,
        "Session ID should be UUID format (36 chars with dashes)"
    );
}

#[test]
fn acceptance_2_2_memory_session_has_agent_name() {
    let session = MemorySession::default();
    assert_eq!(session.agent_name, "claude");
}

#[test]
fn acceptance_2_3_memory_session_has_model() {
    let session = MemorySession::default();
    assert_eq!(session.model, "claude-sonnet-4-6");
}

#[test]
fn acceptance_2_4_memory_session_has_timestamps() {
    let session = MemorySession::default();
    let now = chrono::Utc::now();

    assert!(session.created_at <= now, "Creation time should be in the past");
    assert!(session.last_accessed <= now, "Last accessed should be in the past");
    assert!(
        session.last_accessed >= session.created_at,
        "Last accessed should be after or equal to creation"
    );
}

#[test]
fn acceptance_2_5_memory_session_has_reuse_count() {
    let session = MemorySession::default();
    assert_eq!(session.reuse_count, 0, "Initial reuse count should be 0");
}

#[test]
fn acceptance_2_6_memory_session_clone_debug() {
    let session = MemorySession::default();

    // Verify Clone is implemented
    let cloned = session.clone();
    assert_eq!(cloned.session_id, session.session_id);

    // Verify Debug is implemented
    let debug_str = format!("{:?}", session);
    assert!(debug_str.contains("MemorySession"));
}

// ============================================================================
// Acceptance Criterion 3: Session state tracking (active/idle via staleness)
// ============================================================================

#[test]
fn acceptance_3_1_fresh_session_is_not_stale() {
    let session = MemorySession::default();
    assert!(!session.is_stale(), "Fresh session should not be stale");
}

#[test]
fn acceptance_3_2_old_session_is_stale() {
    let mut session = MemorySession::default();

    // Make session appear old (more than 1 hour)
    let old_time = chrono::Utc::now() - chrono::Duration::seconds(3700);
    session.last_accessed = old_time;

    assert!(session.is_stale(), "Session older than 1 hour should be stale");
}

#[test]
fn acceptance_3_3_exactly_one_hour_is_stale() {
    let mut session = MemorySession::default();

    // Make session exactly 1 hour + 1 second old
    let old_time = chrono::Utc::now() - chrono::Duration::seconds(3601);
    session.last_accessed = old_time;

    assert!(session.is_stale(), "Session exactly 1 hour + 1 second old should be stale");
}

#[test]
fn acceptance_3_4_mark_accessed_updates_state() {
    let mut session = MemorySession::default();
    let initial_count = session.reuse_count;

    session.mark_accessed();

    assert_eq!(
        session.reuse_count,
        initial_count + 1,
        "Reuse count should increment"
    );
    assert!(
        session.last_accessed >= session.created_at,
        "Last accessed should be updated"
    );
}

#[test]
fn acceptance_3_5_mark_accessed_refreshes_stale_session() {
    let mut session = MemorySession::default();

    // Make session stale
    session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(3700);
    assert!(session.is_stale());

    // Mark as accessed
    session.mark_accessed();

    // Should no longer be stale
    assert!(!session.is_stale(), "Session should be fresh after being marked accessed");
}

// ============================================================================
// Acceptance Criterion 4: Parent-child relationship tracking (via agent/model)
// ============================================================================

#[test]
fn acceptance_4_1_get_or_create_reuses_same_agent_model() {
    let mut pool = SessionPool::new();

    let id1 = pool.get_or_create("claude", "claude-sonnet-4-6").session_id.clone();
    let id2 = pool.get_or_create("claude", "claude-sonnet-4-6").session_id.clone();

    assert_eq!(
        id1, id2,
        "Should reuse session for same (agent, model) pair"
    );
    assert_eq!(pool.len(), 1, "Pool should have only one session");
}

#[test]
fn acceptance_4_2_different_models_create_separate_sessions() {
    let mut pool = SessionPool::new();

    let id1 = pool.get_or_create("claude", "claude-sonnet-4-6").session_id.clone();
    let id2 = pool.get_or_create("claude", "claude-opus-4-6").session_id.clone();

    assert_ne!(
        id1, id2,
        "Different models should create separate sessions"
    );
    assert_eq!(pool.len(), 2);
}

#[test]
fn acceptance_4_3_different_agents_create_separate_sessions() {
    let mut pool = SessionPool::new();

    let id1 = pool.get_or_create("claude", "claude-sonnet-4-6").session_id.clone();
    let id2 = pool.get_or_create("opencode", "gpt-4").session_id.clone();

    assert_ne!(
        id1, id2,
        "Different agents should create separate sessions"
    );
    assert_eq!(pool.len(), 2);
}

#[test]
fn acceptance_4_4_stale_sessions_not_reused() {
    let mut pool = SessionPool::new();

    // Create first session
    let id1 = pool.get_or_create("claude", "claude-sonnet-4-6").session_id.clone();

    // Remove the session and create a stale version
    let stale_session = MemorySession {
        session_id: id1.clone(),
        agent_name: "claude".to_string(),
        model: "claude-sonnet-4-6".to_string(),
        created_at: chrono::Utc::now() - chrono::Duration::seconds(4000),
        last_accessed: chrono::Utc::now() - chrono::Duration::seconds(3700),
        reuse_count: 0,
    };
    pool.remove(&id1);
    pool.register(stale_session);

    // get_or_create should create a new session instead of reusing stale one
    let id2 = pool.get_or_create("claude", "claude-sonnet-4-6").session_id.clone();

    assert_ne!(id1, id2, "Should not reuse stale session");
}

#[test]
fn acceptance_4_5_list_by_agent_groups_sessions() {
    let mut pool = SessionPool::new();

    pool.get_or_create("claude", "claude-sonnet-4-6");
    pool.get_or_create("claude", "claude-opus-4-6");
    pool.get_or_create("opencode", "gpt-4");

    let claude_sessions = pool.list_by_agent("claude");
    assert_eq!(claude_sessions.len(), 2, "Should have 2 claude sessions");

    let opencode_sessions = pool.list_by_agent("opencode");
    assert_eq!(opencode_sessions.len(), 1, "Should have 1 opencode session");
}

// ============================================================================
// Acceptance Criterion 5: Pool operations (register, get, remove, cleanup)
// ============================================================================

#[test]
fn acceptance_5_1_register_and_get_roundtrip() {
    let mut pool = SessionPool::new();
    let session = MemorySession::default();
    let id = session.session_id.clone();

    pool.register(session.clone());
    let retrieved = pool.get(&id);

    assert!(retrieved.is_some(), "Should retrieve registered session");
    assert_eq!(retrieved.unwrap().session_id, id);
}

#[test]
fn acceptance_5_2_register_replaces_existing() {
    let mut pool = SessionPool::new();
    let session1 = MemorySession::default();
    let id = session1.session_id.clone();

    pool.register(session1);
    assert_eq!(pool.len(), 1);

    let session2 = MemorySession {
        session_id: id.clone(),
        agent_name: "updated".to_string(),
        ..Default::default()
    };
    pool.register(session2);

    assert_eq!(pool.len(), 1, "Should replace, not add");
    assert_eq!(pool.get(&id).unwrap().agent_name, "updated");
}

#[test]
fn acceptance_5_3_get_nonexistent_returns_none() {
    let pool = SessionPool::new();
    assert!(pool.get("nonexistent").is_none());
}

#[test]
fn acceptance_5_4_remove_existing_session() {
    let mut pool = SessionPool::new();
    let session = MemorySession::default();
    let id = session.session_id.clone();

    pool.register(session);
    assert_eq!(pool.len(), 1);

    let removed = pool.remove(&id);
    assert!(removed.is_some(), "Should return removed session");
    assert_eq!(pool.len(), 0, "Pool should be empty after removal");
}

#[test]
fn acceptance_5_5_remove_nonexistent_returns_none() {
    let mut pool = SessionPool::new();
    assert!(pool.remove("nonexistent").is_none());
}

#[test]
fn acceptance_5_6_cleanup_stale_removes_old_sessions() {
    let mut pool = SessionPool::new();

    // Add fresh session
    pool.register(MemorySession::default());

    // Add stale session
    let mut stale = MemorySession::default();
    stale.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(4000);
    pool.register(stale);

    assert_eq!(pool.len(), 2);

    let removed = pool.cleanup_stale();
    assert_eq!(removed, 1, "Should remove 1 stale session");
    assert_eq!(pool.len(), 1, "Pool should have 1 session remaining");
}

#[test]
fn acceptance_5_7_cleanup_all_stale() {
    let mut pool = SessionPool::new();

    // Add multiple stale sessions
    let mut stale1 = MemorySession::default();
    stale1.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(4000);
    pool.register(stale1);

    let mut stale2 = MemorySession::default();
    stale2.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(5000);
    pool.register(stale2);

    assert_eq!(pool.len(), 2);

    let removed = pool.cleanup_stale();
    assert_eq!(removed, 2, "Should remove all 2 stale sessions");
    assert!(pool.is_empty(), "Pool should be empty");
}

#[test]
fn acceptance_5_8_mark_accessed_updates_pool_session() {
    let mut pool = SessionPool::new();
    let session = MemorySession::default();
    let id = session.session_id.clone();

    pool.register(session);

    let initial_count = pool.get(&id).unwrap().reuse_count;
    let success = pool.mark_accessed(&id);

    assert!(success, "mark_accessed should return true");
    assert_eq!(
        pool.get(&id).unwrap().reuse_count,
        initial_count + 1,
        "Reuse count should increment"
    );
}

#[test]
fn acceptance_5_9_mark_accessed_nonexistent_returns_false() {
    let mut pool = SessionPool::new();
    assert!(!pool.mark_accessed("nonexistent"));
}

#[test]
fn acceptance_5_10_iter_returns_all_sessions() {
    let mut pool = SessionPool::new();

    pool.register(MemorySession::default());
    pool.register(MemorySession::default());
    pool.register(MemorySession::default());

    let count = pool.iter().count();
    assert_eq!(count, 3, "Iterator should return all 3 sessions");
}

// ============================================================================
// Acceptance Criterion 6: SessionData for file-based storage
// ============================================================================

#[test]
fn acceptance_6_1_session_data_has_required_fields() {
    let session = SessionData::new("claude", "claude-sonnet-4-6");

    assert!(!session.session_id.is_empty());
    assert_eq!(session.agent_name, "claude");
    assert_eq!(session.model, "claude-sonnet-4-6");
    assert!(session.created_at <= chrono::Utc::now());
    assert!(session.last_accessed <= chrono::Utc::now());
    assert_eq!(session.reuse_count, 0);
}

#[test]
fn acceptance_6_2_session_data_has_file_path() {
    let session = SessionData::new("claude", "claude-sonnet-4-6");
    assert_eq!(session.file_path, PathBuf::new(), "Initial file path should be empty");
}

#[test]
fn acceptance_6_3_session_data_mark_accessed() {
    let mut session = SessionData::new("claude", "claude-sonnet-4-6");
    session.mark_accessed();

    assert_eq!(session.reuse_count, 1);
    assert!(session.last_accessed >= session.created_at);
}

#[test]
fn acceptance_6_4_session_data_staleness() {
    let mut session = SessionData::new("claude", "claude-sonnet-4-6");

    assert!(!session.is_stale(), "Fresh session should not be stale");

    session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(3700);
    assert!(session.is_stale(), "Old session should be stale");
}

#[test]
fn acceptance_6_5_session_data_serialization() {
    let session = SessionData::new("claude", "claude-sonnet-4-6");

    // Verify Serialize/Deserialize are implemented
    let json = serde_json::to_string(&session).expect("Should serialize");
    let deserialized: SessionData =
        serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(deserialized.session_id, session.session_id);
    assert_eq!(deserialized.agent_name, session.agent_name);
    assert_eq!(deserialized.model, session.model);
}

// ============================================================================
// Edge cases and error scenarios
// ============================================================================

#[test]
fn edge_case_empty_pool_operations() {
    let mut pool = SessionPool::new();

    assert!(pool.is_empty());
    assert_eq!(pool.len(), 0);
    assert_eq!(pool.cleanup_stale(), 0);
    assert!(pool.get("anything").is_none());
    assert!(pool.remove("anything").is_none());
    assert!(!pool.mark_accessed("anything"));
    assert_eq!(pool.iter().count(), 0);
    assert_eq!(pool.list_by_agent("claude").len(), 0);
}

#[test]
fn edge_case_get_or_create_with_empty_strings() {
    let mut pool = SessionPool::new();

    // Should still work with empty strings (creates valid session)
    let session = pool.get_or_create("", "");
    assert!(!session.session_id.is_empty());
    assert_eq!(session.agent_name, "");
    assert_eq!(session.model, "");
}

#[test]
fn edge_case_multiple_cleanup_calls() {
    let mut pool = SessionPool::new();

    pool.register(MemorySession::default());

    // First cleanup - nothing stale
    assert_eq!(pool.cleanup_stale(), 0);

    // Make session stale by removing and registering a stale version
    let id = pool.iter().next().unwrap().session_id.clone();
    pool.remove(&id);
    let stale_session = MemorySession {
        session_id: id,
        created_at: chrono::Utc::now() - chrono::Duration::seconds(4000),
        last_accessed: chrono::Utc::now() - chrono::Duration::seconds(4000),
        ..Default::default()
    };
    pool.register(stale_session);

    // Second cleanup - removes stale
    assert_eq!(pool.cleanup_stale(), 1);

    // Third cleanup - nothing left to clean
    assert_eq!(pool.cleanup_stale(), 0);
}

#[test]
fn edge_case_session_id_collision() {
    let mut pool = SessionPool::new();

    // Create two sessions with same ID (via register)
    let session1 = MemorySession::default();
    let id = session1.session_id.clone();

    pool.register(session1);
    assert_eq!(pool.len(), 1);

    // Create new session with same ID
    let session2 = MemorySession {
        session_id: id.clone(),
        agent_name: "replacement".to_string(),
        ..Default::default()
    };
    pool.register(session2);

    // Should replace, not duplicate
    assert_eq!(pool.len(), 1);
    assert_eq!(pool.get(&id).unwrap().agent_name, "replacement");
}

#[test]
fn edge_case_unicode_in_agent_names() {
    let mut pool = SessionPool::new();

    // Unicode characters should work
    let session = pool.get_or_create("代理", "模型-4-6");
    assert_eq!(session.agent_name, "代理");
    assert_eq!(session.model, "模型-4-6");
}

#[test]
fn edge_case_very_long_session_ids() {
    let mut pool = SessionPool::new();

    let long_id = "a".repeat(1000);
    let session = MemorySession {
        session_id: long_id.clone(),
        ..Default::default()
    };

    pool.register(session);
    assert!(pool.get(&long_id).is_some());
}

// ============================================================================
// Integration scenarios
// ============================================================================

#[test]
fn integration_full_session_lifecycle() {
    let mut pool = SessionPool::new();

    // 1. Create session via get_or_create
    let session1 = pool.get_or_create("claude", "claude-sonnet-4-6");
    let id = session1.session_id.clone();
    assert_eq!(pool.len(), 1);

    // 2. Retrieve session
    let session2 = pool.get(&id);
    assert!(session2.is_some());
    assert_eq!(session2.unwrap().session_id, id);

    // 3. Mark as accessed
    assert!(pool.mark_accessed(&id));

    // 4. List by agent
    let sessions = pool.list_by_agent("claude");
    assert_eq!(sessions.len(), 1);

    // 5. Remove session
    assert!(pool.remove(&id).is_some());
    assert!(pool.is_empty());
}

#[test]
fn integration_session_reuse_across_tasks() {
    let mut pool = SessionPool::new();
    let mut session_id = String::new();

    // Simulate multiple tasks using the same agent/model
    for task_num in 1..=5 {
        {
            let session = pool.get_or_create("claude", "claude-sonnet-4-6");
            session_id = session.session_id.clone();

            if task_num > 1 {
                // All tasks should get the same session
                assert_eq!(session.reuse_count, task_num - 1);
            }
        }

        pool.mark_accessed(&session_id);
    }

    // Should still have only 1 session
    assert_eq!(pool.len(), 1);

    let session = pool.get_or_create("claude", "claude-sonnet-4-6");
    assert_eq!(session.reuse_count, 5, "Session should have been reused 5 times");
}

#[test]
fn integration_mixed_agent_scenarios() {
    let mut pool = SessionPool::new();

    // Use multiple agents and models
    {
        let _ = pool.get_or_create("claude", "claude-sonnet-4-6");
        let _ = pool.get_or_create("claude", "claude-opus-4-6");
        let _ = pool.get_or_create("opencode", "gpt-4");
        let _ = pool.get_or_create("kimicode", "moonshot-v1");
    }

    assert_eq!(pool.len(), 4);

    // Reuse some
    {
        let _ = pool.get_or_create("claude", "claude-sonnet-4-6");
        let _ = pool.get_or_create("opencode", "gpt-4");
    }

    assert_eq!(pool.len(), 4, "Reuse should not create new sessions");

    // Verify groupings
    assert_eq!(pool.list_by_agent("claude").len(), 2);
    assert_eq!(pool.list_by_agent("opencode").len(), 1);
}

// ============================================================================
// SessionManager integration tests
// ============================================================================

#[tokio::test]
async fn integration_session_manager_creation() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    assert!(manager.sessions_dir.exists());
    assert!(manager.sessions_dir.ends_with(".ltmatrix/sessions"));
}

#[tokio::test]
async fn integration_session_manager_create_and_load() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    // Create session
    let session = manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();

    assert!(!session.session_id.is_empty());
    assert!(session.file_path.exists());

    // Load session
    let loaded = manager
        .load_session(&session.session_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(loaded.session_id, session.session_id);
    assert_eq!(loaded.agent_name, "claude");
    assert_eq!(loaded.model, "claude-sonnet-4-6");
    // Loading marks as accessed
    assert_eq!(loaded.reuse_count, 1);
}

#[tokio::test]
async fn integration_session_manager_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    // Create fresh session
    manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();

    // Create stale session by manually writing old session
    let session_id = uuid::Uuid::new_v4().to_string();
    let stale_file = manager
        .sessions_dir
        .join(format!("claude-{}.json", session_id));

    let stale_session = SessionData {
        session_id: session_id.clone(),
        agent_name: "claude".to_string(),
        model: "claude-sonnet-4-6".to_string(),
        created_at: chrono::Utc::now() - chrono::Duration::seconds(4000),
        last_accessed: chrono::Utc::now() - chrono::Duration::seconds(4000),
        reuse_count: 0,
        file_path: stale_file.clone(),
    };

    let content = serde_json::to_string(&stale_session).unwrap();
    tokio::fs::write(&stale_file, content).await.unwrap();

    // Cleanup should remove stale session
    let cleaned = manager.cleanup_stale_sessions().await.unwrap();
    assert_eq!(cleaned, 1, "Should clean 1 stale session");
}

#[tokio::test]
async fn integration_session_manager_delete() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    let session = manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();

    assert!(session.file_path.exists());

    let deleted = manager.delete_session(&session.session_id).await.unwrap();
    assert!(deleted, "Should return true on successful delete");
    assert!(!session.file_path.exists(), "File should be deleted");
}

#[tokio::test]
async fn integration_session_manager_list() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    // Create multiple sessions
    manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();
    manager
        .create_session("opencode", "gpt-4")
        .await
        .unwrap();

    let sessions = manager.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 2);
}
