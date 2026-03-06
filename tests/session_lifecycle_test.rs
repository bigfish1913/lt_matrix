//! Session lifecycle management tests
//!
//! These tests verify the complete session lifecycle including:
//! - Session creation, acquisition, and release methods
//! - Session cleanup on completion (drop handlers, explicit cleanup methods)
//! - Session health monitoring and timeout handling
//! - Thread-safe concurrent access

use ltmatrix::agent::backend::{AgentSession, MemorySession};
use ltmatrix::agent::pool::SessionPool;
use ltmatrix::agent::session::{SessionData, SessionManager};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

// ============================================================================
// Session Creation, Acquisition, and Release
// ============================================================================

#[test]
fn lifecycle_session_creation_via_get_or_create() {
    let mut pool = SessionPool::new();

    // Create a new session
    let session = pool.get_or_create("claude", "claude-sonnet-4-6");

    assert!(!session.session_id().is_empty(), "Session ID should be generated");
    assert_eq!(session.agent_name(), "claude");
    assert_eq!(session.model(), "claude-sonnet-4-6");
    assert_eq!(session.reuse_count(), 0, "New session should have reuse_count of 0");
    assert!(!session.is_stale(), "New session should not be stale");
}

#[test]
fn lifecycle_session_acquisition_reuses_existing() {
    let mut pool = SessionPool::new();

    // Create initial session
    let session1 = pool.get_or_create("claude", "claude-sonnet-4-6");
    let id1 = session1.session_id().to_string();

    // Acquire same session (should reuse)
    let session2 = pool.get_or_create("claude", "claude-sonnet-4-6");
    let id2 = session2.session_id().to_string();

    assert_eq!(id1, id2, "Should reuse existing session");
    assert_eq!(pool.len(), 1, "Pool should only have one session");
}

#[test]
fn lifecycle_session_release_via_remove() {
    let mut pool = SessionPool::new();

    // Create and then release a session
    let session = pool.get_or_create("claude", "claude-sonnet-4-6");
    let id = session.session_id().to_string();

    assert_eq!(pool.len(), 1);

    // Release the session
    let removed = pool.remove(&id);
    assert!(removed.is_some(), "Should successfully remove session");
    assert_eq!(pool.len(), 0, "Pool should be empty after removal");
    assert!(pool.get(&id).is_none(), "Removed session should not be accessible");
}

#[test]
fn lifecycle_session_registration() {
    let mut pool = SessionPool::new();

    // Register custom session
    let session = MemorySession {
        session_id: "custom-session-123".to_string(),
        agent_name: "custom-agent".to_string(),
        model: "custom-model".to_string(),
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
        reuse_count: 5,
    };

    pool.register(session.clone());

    // Verify registration
    let retrieved = pool.get("custom-session-123");
    assert!(retrieved.is_some(), "Should retrieve registered session");
    assert_eq!(retrieved.unwrap().agent_name(), "custom-agent");
    assert_eq!(retrieved.unwrap().reuse_count(), 5);
}

#[test]
fn lifecycle_session_replacement_on_reregistration() {
    let mut pool = SessionPool::new();

    // Register initial session
    let session1 = MemorySession {
        session_id: "same-id".to_string(),
        agent_name: "agent-v1".to_string(),
        ..Default::default()
    };
    pool.register(session1);

    assert_eq!(pool.get("same-id").unwrap().agent_name(), "agent-v1");

    // Register new session with same ID (should replace)
    let session2 = MemorySession {
        session_id: "same-id".to_string(),
        agent_name: "agent-v2".to_string(),
        reuse_count: 10,
        ..Default::default()
    };
    pool.register(session2);

    assert_eq!(pool.len(), 1, "Should replace, not add");
    assert_eq!(pool.get("same-id").unwrap().agent_name(), "agent-v2");
    assert_eq!(pool.get("same-id").unwrap().reuse_count(), 10);
}

// ============================================================================
// Session Cleanup on Completion
// ============================================================================

#[test]
fn lifecycle_cleanup_stale_sessions() {
    let mut pool = SessionPool::new();

    // Add fresh session
    pool.register(MemorySession::default());

    // Add stale session (> 1 hour old)
    let mut stale_session = MemorySession::default();
    stale_session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(4000);
    let stale_id = stale_session.session_id.clone();
    pool.register(stale_session);

    assert_eq!(pool.len(), 2, "Should have 2 sessions before cleanup");

    // Cleanup stale sessions
    let removed_count = pool.cleanup_stale();

    assert_eq!(removed_count, 1, "Should remove 1 stale session");
    assert_eq!(pool.len(), 1, "Should have 1 session after cleanup");
    assert!(pool.get(&stale_id).is_none(), "Stale session should be removed");
}

#[test]
fn lifecycle_cleanup_all_sessions_stale() {
    let mut pool = SessionPool::new();

    // Add multiple stale sessions
    for i in 0..5 {
        let mut session = MemorySession::default();
        session.session_id = format!("stale-{}", i);
        session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(4000 + i * 100);
        pool.register(session);
    }

    assert_eq!(pool.len(), 5);

    let removed = pool.cleanup_stale();
    assert_eq!(removed, 5, "Should remove all 5 stale sessions");
    assert!(pool.is_empty(), "Pool should be empty");
}

#[test]
fn lifecycle_cleanup_no_sessions_to_remove() {
    let mut pool = SessionPool::new();

    // Add only fresh sessions
    for _ in 0..3 {
        pool.register(MemorySession::default());
    }

    let removed = pool.cleanup_stale();
    assert_eq!(removed, 0, "Should remove 0 sessions when none are stale");
    assert_eq!(pool.len(), 3, "All fresh sessions should remain");
}

#[test]
fn lifecycle_explicit_cleanup_removes_specific_session() {
    let mut pool = SessionPool::new();

    let id1 = pool.get_or_create("agent1", "model1").session_id().to_string();
    let id2 = pool.get_or_create("agent2", "model2").session_id().to_string();

    assert_eq!(pool.len(), 2);

    // Explicitly remove session1
    pool.remove(&id1);

    assert_eq!(pool.len(), 1);
    assert!(pool.get(&id1).is_none());
    assert!(pool.get(&id2).is_some());
}

#[tokio::test]
async fn lifecycle_session_manager_file_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    // Create fresh session
    manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();

    // Create stale session manually
    let stale_session = SessionData {
        session_id: "stale-id".to_string(),
        agent_name: "claude".to_string(),
        model: "claude-sonnet-4-6".to_string(),
        created_at: chrono::Utc::now() - chrono::Duration::seconds(4000),
        last_accessed: chrono::Utc::now() - chrono::Duration::seconds(4000),
        reuse_count: 0,
        file_path: manager
            .sessions_dir
            .join("claude-stale-id.json"),
    };

    let content = serde_json::to_string(&stale_session).unwrap();
    tokio::fs::write(&stale_session.file_path, content)
        .await
        .unwrap();

    // Cleanup should remove stale session
    let cleaned = manager.cleanup_stale_sessions().await.unwrap();
    assert_eq!(cleaned, 1, "Should clean 1 stale session file");

    // Verify file was deleted
    assert!(!stale_session.file_path.exists());
}

// ============================================================================
// Session Health Monitoring and Timeout Handling
// ============================================================================

#[test]
fn lifecycle_health_check_fresh_session() {
    let session = MemorySession::default();

    assert!(!session.is_stale(), "Fresh session should not be stale");
    assert_eq!(session.reuse_count(), 0);
}

#[test]
fn lifecycle_health_check_stale_session_detection() {
    let mut session = MemorySession::default();

    // Make session appear old
    session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(3700);

    assert!(session.is_stale(), "Session > 1 hour old should be stale");
}

#[test]
fn lifecycle_health_check_boundary_conditions() {
    let mut session = MemorySession::default();

    // Exactly 1 hour old (3600 seconds) - should NOT be stale
    session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(3600);
    assert!(!session.is_stale(), "Session exactly 1 hour old should not be stale");

    // 1 hour + 1 second old - should be stale
    session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(3601);
    assert!(session.is_stale(), "Session 1 hour + 1 second old should be stale");
}

#[test]
fn lifecycle_health_check_mark_accessed_refreshes() {
    let mut session = MemorySession::default();

    // Make session stale
    session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(4000);
    assert!(session.is_stale());

    // Mark as accessed
    session.mark_accessed();

    assert!(!session.is_stale(), "Session should be fresh after mark_accessed");
    assert_eq!(session.reuse_count(), 1);
}

#[test]
fn lifecycle_health_check_reuse_count_increments() {
    let mut session = MemorySession::default();

    assert_eq!(session.reuse_count(), 0);

    for i in 1..=10 {
        session.mark_accessed();
        assert_eq!(session.reuse_count(), i);
    }
}

#[test]
fn lifecycle_health_check_pool_mark_accessed() {
    let mut pool = SessionPool::new();

    let session = pool.get_or_create("claude", "claude-sonnet-4-6");
    let id = session.session_id().to_string();

    let initial_count = pool.get(&id).unwrap().reuse_count();

    // Mark as accessed via pool
    let success = pool.mark_accessed(&id);

    assert!(success, "mark_accessed should return true");
    assert_eq!(
        pool.get(&id).unwrap().reuse_count(),
        initial_count + 1
    );
}

#[test]
fn lifecycle_health_check_nonexistent_session_mark_accessed() {
    let mut pool = SessionPool::new();

    let success = pool.mark_accessed("nonexistent-id");
    assert!(!success, "mark_accessed should return false for nonexistent session");
}

// ============================================================================
// Thread-Safe Concurrent Access
// ============================================================================

#[test]
fn lifecycle_concurrent_get_or_create_same_session() {
    let pool = Arc::new(Mutex::new(SessionPool::new()));
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let pool_clone = Arc::clone(&pool);
            thread::spawn(move || {
                let mut pool = pool_clone.lock().unwrap();
                let session = pool.get_or_create("claude", "claude-sonnet-4-6");
                session.session_id().to_string()
            })
        })
        .collect();

    // Wait for all threads
    let session_ids: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect();

    // All threads should get the same session ID
    let first_id = &session_ids[0];
    for id in &session_ids[1..] {
        assert_eq!(id, first_id, "All threads should get the same session");
    }

    // Pool should only have one session
    assert_eq!(pool.lock().unwrap().len(), 1);
}

#[test]
fn lifecycle_concurrent_different_agents_create_separate_sessions() {
    let pool = Arc::new(Mutex::new(SessionPool::new()));
    let agents = vec![
        ("claude", "claude-sonnet-4-6"),
        ("opencode", "gpt-4"),
        ("kimicode", "moonshot-v1"),
    ];

    let handles: Vec<_> = agents
        .iter()
        .map(|(agent, model)| {
            let pool_clone = Arc::clone(&pool);
            let agent = agent.to_string();
            let model = model.to_string();
            thread::spawn(move || {
                let mut pool = pool_clone.lock().unwrap();
                pool.get_or_create(&agent, &model).session_id().to_string()
            })
        })
        .collect();

    // Wait for all threads
    let session_ids: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect();

    // All session IDs should be different
    assert_eq!(session_ids.len(), 3);
    for i in 0..session_ids.len() {
        for j in (i + 1)..session_ids.len() {
            assert_ne!(
                session_ids[i], session_ids[j],
                "Different agents should have different sessions"
            );
        }
    }

    // Pool should have 3 sessions
    assert_eq!(pool.lock().unwrap().len(), 3);
}

#[test]
fn lifecycle_concurrent_mark_accessed_thread_safety() {
    let pool = Arc::new(Mutex::new(SessionPool::new()));

    // Create initial session and capture its ID
    let id = {
        let mut pool = pool.lock().unwrap();
        pool.get_or_create("claude", "claude-sonnet-4-6").session_id().to_string()
    };

    // Multiple threads marking as accessed
    let handles: Vec<_> = (0..20)
        .map(|_| {
            let pool_clone = Arc::clone(&pool);
            let id_clone = id.clone();
            thread::spawn(move || {
                let mut pool = pool_clone.lock().unwrap();
                pool.mark_accessed(&id_clone);
                // Small delay to increase chance of race conditions
                thread::sleep(Duration::from_millis(1));
            })
        })
        .collect();

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Reuse count should be 20 (all marks succeeded)
    let pool = pool.lock().unwrap();
    assert_eq!(pool.get(&id).unwrap().reuse_count(), 20);
}

#[test]
fn lifecycle_concurrent_register_and_get() {
    let pool = Arc::new(Mutex::new(SessionPool::new()));
    let num_threads = 10;

    let handles: Vec<_> = (0..num_threads)
        .map(|i| {
            let pool_clone = Arc::clone(&pool);
            thread::spawn(move || {
                let mut pool = pool_clone.lock().unwrap();

                // Each thread registers a unique session
                let session = MemorySession {
                    session_id: format!("session-{}", i),
                    agent_name: format!("agent-{}", i),
                    model: "model".to_string(),
                    ..Default::default()
                };
                pool.register(session);

                // Then tries to get it
                pool.get(&format!("session-{}", i)).is_some()
            })
        })
        .collect();

    // Wait for all threads and verify all succeeded
    for handle in handles {
        assert!(handle.join().unwrap(), "Each thread should successfully register and get its session");
    }

    // Pool should have all sessions
    assert_eq!(pool.lock().unwrap().len(), num_threads);
}

#[test]
fn lifecycle_concurrent_cleanup_and_access() {
    let pool = Arc::new(Mutex::new(SessionPool::new()));

    // Populate pool with sessions
    {
        let mut pool = pool.lock().unwrap();
        for i in 0..20 {
            pool.get_or_create(&format!("agent-{}", i % 3), "model");
        }
    }

    // Thread 1: Continuously cleanup
    let pool_clone1 = Arc::clone(&pool);
    let cleanup_handle = thread::spawn(move || {
        for _ in 0..10 {
            let mut pool = pool_clone1.lock().unwrap();
            pool.cleanup_stale();
            thread::sleep(Duration::from_millis(1));
        }
    });

    // Thread 2: Continuously access sessions
    let pool_clone2 = Arc::clone(&pool);
    let access_handle = thread::spawn(move || {
        for i in 0..20 {
            let mut pool = pool_clone2.lock().unwrap();
            pool.get_or_create(&format!("agent-{}", i % 3), "model");
            pool.mark_accessed(&format!("session-{}", i));
        }
    });

    cleanup_handle.join().unwrap();
    access_handle.join().unwrap();

    // Pool should still be consistent
    let pool = pool.lock().unwrap();
    assert!(pool.len() <= 20, "Pool should have at most 20 sessions");
}

#[test]
fn lifecycle_concurrent_remove_different_sessions() {
    let pool = Arc::new(Mutex::new(SessionPool::new()));

    // Create multiple sessions
    let session_ids: Vec<String> = {
        let mut pool = pool.lock().unwrap();
        (0..5)
            .map(|i| {
                let session = pool.get_or_create(&format!("agent-{}", i), "model");
                session.session_id().to_string()
            })
            .collect()
    };

    // Multiple threads removing different sessions
    let handles: Vec<_> = session_ids
        .iter()
        .map(|id| {
            let pool_clone = Arc::clone(&pool);
            let id_clone = id.clone();
            thread::spawn(move || {
                let mut pool = pool_clone.lock().unwrap();
                pool.remove(&id_clone).is_some()
            })
        })
        .collect();

    // All removals should succeed
    for handle in handles {
        assert!(handle.join().unwrap(), "Each session should be successfully removed");
    }

    // Pool should be empty
    assert!(pool.lock().unwrap().is_empty());
}

// ============================================================================
// Drop Handlers and Resource Cleanup
// ============================================================================

#[test]
fn lifecycle_session_pool_operations_consistency_after_removal() {
    let mut pool = SessionPool::new();

    let id = pool.get_or_create("claude", "claude-sonnet-4-6").session_id().to_string();

    // Verify session exists
    assert!(pool.get(&id).is_some());
    assert_eq!(pool.len(), 1);

    // Remove session
    pool.remove(&id);

    // Verify state after removal
    assert!(pool.get(&id).is_none());
    assert_eq!(pool.len(), 0);
    assert!(pool.is_empty());

    // Can create new session after removal
    let new_id = pool.get_or_create("claude", "claude-sonnet-4-6").session_id().to_string();
    assert_ne!(new_id, id, "New session should have different ID");
}

#[test]
fn lifecycle_session_data_fields_persistence() {
    let mut session = SessionData::new("claude", "claude-sonnet-4-6");

    let original_id = session.session_id.clone();
    let original_created = session.created_at;

    // Modify session
    session.mark_accessed();

    // Verify fields are consistent
    assert_eq!(session.session_id, original_id, "Session ID should not change");
    assert_eq!(session.created_at, original_created, "Creation time should not change");
    assert!(session.last_accessed >= session.created_at);
    assert_eq!(session.reuse_count, 1);
}

#[tokio::test]
async fn lifecycle_session_manager_persistence_across_operations() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    // Create session (reuse_count = 0)
    let session = manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();
    let id = session.session_id.clone();
    let file_path = session.file_path.clone();
    assert_eq!(session.reuse_count, 0, "New session should have reuse_count of 0");

    // Verify file exists
    assert!(file_path.exists());

    // Load session (load_session marks as accessed, so reuse_count = 1)
    let loaded = manager.load_session(&id).await.unwrap().unwrap();
    assert_eq!(loaded.session_id, id);
    assert_eq!(loaded.reuse_count, 1, "load_session should increment reuse_count");

    // Mark as accessed and save (reuse_count = 2)
    {
        let mut session = loaded;
        session.mark_accessed();
        assert_eq!(session.reuse_count, 2);
        manager.save_session(&session).await.unwrap();
    }

    // Load again and verify updated reuse_count (load_session marks as accessed again, so reuse_count = 3)
    let reloaded = manager.load_session(&id).await.unwrap().unwrap();
    assert_eq!(reloaded.reuse_count, 3, "Reuse count should increment with each load");

    // Delete session
    let deleted = manager.delete_session(&id).await.unwrap();
    assert!(deleted);
    assert!(!file_path.exists(), "File should be deleted");
}

// ============================================================================
// Integration Scenarios
// ============================================================================

#[test]
fn lifecycle_full_session_workflow() {
    let mut pool = SessionPool::new();

    // 1. Create session
    let id = pool.get_or_create("claude", "claude-sonnet-4-6").session_id().to_string();
    assert_eq!(pool.len(), 1);

    // 2. Access/reuse session
    let reused_id = pool.get_or_create("claude", "claude-sonnet-4-6").session_id().to_string();
    assert_eq!(reused_id, id);
    assert_eq!(pool.len(), 1);

    // 3. Mark as accessed
    pool.mark_accessed(&id);
    assert_eq!(pool.get(&id).unwrap().reuse_count(), 1);

    // 4. Check health
    assert!(!pool.get(&id).unwrap().is_stale());

    // 5. Cleanup (should not remove fresh session)
    let removed = pool.cleanup_stale();
    assert_eq!(removed, 0);
    assert_eq!(pool.len(), 1);

    // 6. Release session
    pool.remove(&id);
    assert!(pool.is_empty());
}

#[test]
fn lifecycle_multiple_agents_with_session_reuse() {
    let mut pool = SessionPool::new();

    // Create sessions for different agents
    let claude_id = pool
        .get_or_create("claude", "claude-sonnet-4-6")
        .session_id()
        .to_string();
    let opencode_id = pool
        .get_or_create("opencode", "gpt-4")
        .session_id()
        .to_string();

    assert_eq!(pool.len(), 2);

    // Reuse sessions
    let claude_reused_id = pool.get_or_create("claude", "claude-sonnet-4-6").session_id().to_string();
    assert_eq!(claude_reused_id, claude_id);

    let opencode_reused_id = pool.get_or_create("opencode", "gpt-4").session_id().to_string();
    assert_eq!(opencode_reused_id, opencode_id);

    // Still only 2 sessions
    assert_eq!(pool.len(), 2);

    // Cleanup specific session
    pool.remove(&claude_id);
    assert_eq!(pool.len(), 1);
    assert!(pool.get(&claude_id).is_none());
    assert!(pool.get(&opencode_id).is_some());
}

#[test]
fn lifecycle_session_staleness_affects_get_or_create() {
    let mut pool = SessionPool::new();

    // Create initial session
    let id = pool
        .get_or_create("claude", "claude-sonnet-4-6")
        .session_id()
        .to_string();

    // Make session stale
    {
        let session = pool.get(&id).unwrap();
        let stale_session = MemorySession {
            session_id: session.session_id().to_string(),
            agent_name: session.agent_name().to_string(),
            model: session.model().to_string(),
            created_at: session.created_at(),
            last_accessed: chrono::Utc::now() - chrono::Duration::seconds(4000),
            reuse_count: session.reuse_count(),
        };
        pool.remove(&id);
        pool.register(stale_session);
    }

    // get_or_create should create new session instead of reusing stale one
    let new_id = pool.get_or_create("claude", "claude-sonnet-4-6").session_id().to_string();
    assert_ne!(new_id, id, "Should create new session when old one is stale");
}
