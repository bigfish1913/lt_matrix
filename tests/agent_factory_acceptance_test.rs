//! AgentFactory and Session Management — Acceptance Tests
//!
//! Verifies all acceptance criteria for task-009-3:
//!   1. AgentFactory creates backends by name
//!   2. Session pool / registry enables agent reuse
//!   3. Per-backend configuration validation
//!   4. SessionManager persists and loads sessions on disk
//!   5. Factory handles unknown / invalid inputs gracefully
//!   6. AgentFactoryConfig sets the default backend

use ltmatrix::agent::backend::{AgentConfig, AgentSession, MemorySession};
use ltmatrix::agent::factory::{AgentFactory, AgentFactoryConfig};
use ltmatrix::agent::pool::SessionPool;
use ltmatrix::agent::session::{SessionData, SessionManager};

// ============================================================================
// AC-1: AgentFactory creates backends by name
// ============================================================================

#[test]
fn ac01_factory_create_claude_by_name_succeeds() {
    let factory = AgentFactory::new();
    let result = factory.create("claude");
    assert!(
        result.is_ok(),
        "factory.create('claude') should succeed; got: {:?}",
        result.err()
    );
}

#[test]
fn ac02_factory_created_agent_reports_claude_backend_name() {
    let factory = AgentFactory::new();
    let agent = factory.create("claude").unwrap();
    assert_eq!(
        agent.backend_name(),
        "claude",
        "backend_name should be 'claude'"
    );
}

#[test]
fn ac03_factory_create_default_produces_claude_backend() {
    let factory = AgentFactory::new();
    let agent = factory.create_default().unwrap();
    assert_eq!(agent.backend_name(), "claude");
}

#[test]
fn ac04_factory_create_unknown_backend_returns_error() {
    let factory = AgentFactory::new();
    let result = factory.create("does-not-exist");
    assert!(
        result.is_err(),
        "creating an unknown backend should return Err"
    );
}

#[test]
fn ac05_factory_error_message_mentions_unknown_backend_name() {
    let factory = AgentFactory::new();
    let err = factory.create("phantom-agent").err().unwrap();
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("phantom-agent") || msg.contains("unsupported") || msg.contains("unknown"),
        "error should reference the unknown name; got: {}",
        err
    );
}

#[test]
fn ac06_factory_supports_at_least_one_backend() {
    let factory = AgentFactory::new();
    let backends = factory.supported_backends();
    assert!(!backends.is_empty(), "supported_backends should not be empty");
}

#[test]
fn ac07_factory_supported_backends_includes_claude() {
    let factory = AgentFactory::new();
    assert!(
        factory.supported_backends().contains(&"claude"),
        "supported_backends should include 'claude'"
    );
}

#[test]
fn ac08_factory_is_supported_returns_true_for_claude() {
    let factory = AgentFactory::new();
    assert!(factory.is_supported("claude"));
}

#[test]
fn ac09_factory_is_supported_returns_false_for_unknown() {
    let factory = AgentFactory::new();
    assert!(!factory.is_supported("gpt-9000"));
}

// ============================================================================
// AC-2: create_with_config accepts a fully-specified AgentConfig
// ============================================================================

#[test]
fn ac10_factory_create_with_valid_config_succeeds() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("claude")
        .model("claude-opus-4-6")
        .command("claude")
        .timeout_secs(7200)
        .max_retries(5)
        .enable_session(true)
        .build();

    assert!(factory.create_with_config(config).is_ok());
}

#[test]
fn ac11_factory_create_with_empty_model_fails() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("claude")
        .model("")
        .command("claude")
        .timeout_secs(3600)
        .build();

    assert!(
        factory.create_with_config(config).is_err(),
        "empty model should fail validation"
    );
}

#[test]
fn ac12_factory_create_with_empty_command_fails() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("claude")
        .model("claude-sonnet-4-6")
        .command("")
        .timeout_secs(3600)
        .build();

    assert!(
        factory.create_with_config(config).is_err(),
        "empty command should fail validation"
    );
}

#[test]
fn ac13_factory_create_with_zero_timeout_fails() {
    let factory = AgentFactory::new();
    let config = AgentConfig {
        name: "claude".to_string(),
        model: "claude-sonnet-4-6".to_string(),
        command: "claude".to_string(),
        timeout_secs: 0,
        max_retries: 3,
        enable_session: true,
    };

    assert!(
        factory.create_with_config(config).is_err(),
        "zero timeout should fail validation"
    );
}

#[test]
fn ac14_factory_create_with_empty_name_fails() {
    let factory = AgentFactory::new();
    let config = AgentConfig {
        name: "".to_string(),
        model: "claude-sonnet-4-6".to_string(),
        command: "claude".to_string(),
        timeout_secs: 3600,
        max_retries: 3,
        enable_session: true,
    };

    assert!(
        factory.create_with_config(config).is_err(),
        "empty name should fail validation"
    );
}

// ============================================================================
// AC-3: Per-backend configuration validation
// ============================================================================

#[test]
fn ac15_validate_config_accepts_valid_claude_config() {
    let factory = AgentFactory::new();
    let config = AgentConfig::default(); // name == "claude"
    assert!(
        factory.validate_config("claude", &config).is_ok(),
        "default config should pass claude validation"
    );
}

#[test]
fn ac16_validate_config_rejects_wrong_name_for_claude_backend() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("opencode") // wrong name for the claude backend
        .model("claude-sonnet-4-6")
        .command("claude")
        .timeout_secs(3600)
        .build();

    assert!(
        factory.validate_config("claude", &config).is_err(),
        "claude backend requires name='claude'"
    );
}

#[test]
fn ac17_validate_config_rejects_empty_model_for_claude() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("claude")
        .model("")
        .command("claude")
        .timeout_secs(3600)
        .build();

    assert!(factory.validate_config("claude", &config).is_err());
}

#[test]
fn ac18_validate_config_rejects_unknown_backend_name() {
    let factory = AgentFactory::new();
    let config = AgentConfig::default();
    let result = factory.validate_config("nonexistent", &config);
    assert!(
        result.is_err(),
        "validating against an unknown backend name should fail"
    );
}

#[test]
fn ac19_validation_error_mentions_field_name() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("claude")
        .model("")
        .command("claude")
        .timeout_secs(3600)
        .build();

    let err = factory.validate_config("claude", &config).unwrap_err();
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("model") || msg.contains("empty"),
        "error should mention the invalid field; got: {}",
        err
    );
}

// ============================================================================
// AC-4: AgentFactoryConfig — configurable default backend
// ============================================================================

#[test]
fn ac20_factory_config_default_backend_is_claude() {
    let config = AgentFactoryConfig::default();
    assert_eq!(config.default_backend, "claude");
}

#[test]
fn ac21_factory_with_custom_factory_config_sets_default_backend() {
    let fc = AgentFactoryConfig {
        default_backend: "claude".to_string(),
    };
    let factory = AgentFactory::with_factory_config(fc);
    let result = factory.create_default();
    assert!(
        result.is_ok(),
        "create_default with custom factory config should succeed"
    );
}

#[test]
fn ac22_factory_default_impl_matches_new() {
    // AgentFactory should implement Default identically to AgentFactory::new()
    let f1 = AgentFactory::new();
    let f2 = AgentFactory::default();
    // Both should create claude successfully
    assert!(f1.create("claude").is_ok());
    assert!(f2.create("claude").is_ok());
}

// ============================================================================
// AC-5: SessionPool — in-memory registry for session reuse
// ============================================================================

#[test]
fn ac23_session_pool_starts_empty() {
    let pool = SessionPool::new();
    assert_eq!(pool.len(), 0);
    assert!(pool.is_empty());
}

#[test]
fn ac24_session_pool_register_increases_length() {
    let mut pool = SessionPool::new();
    pool.register(MemorySession::default());
    assert_eq!(pool.len(), 1);
}

#[test]
fn ac25_session_pool_get_returns_registered_session() {
    let mut pool = SessionPool::new();
    let session = MemorySession::default();
    let id = session.session_id().to_string();

    pool.register(session);
    let found = pool.get(&id);
    assert!(found.is_some());
    assert_eq!(found.unwrap().session_id(), id);
}

#[test]
fn ac26_session_pool_get_returns_none_for_missing_id() {
    let pool = SessionPool::new();
    assert!(pool.get("nonexistent").is_none());
}

#[test]
fn ac27_session_pool_remove_existing_session() {
    let mut pool = SessionPool::new();
    let session = MemorySession::default();
    let id = session.session_id().to_string();

    pool.register(session);
    let removed = pool.remove(&id);
    assert!(removed.is_some());
    assert_eq!(pool.len(), 0);
}

#[test]
fn ac28_session_pool_remove_nonexistent_returns_none() {
    let mut pool = SessionPool::new();
    assert!(pool.remove("ghost-id").is_none());
}

#[test]
fn ac29_session_pool_replace_on_duplicate_register() {
    let mut pool = SessionPool::new();
    let mut s = MemorySession::default();
    let id = s.session_id().to_string();

    pool.register(s.clone());
    assert_eq!(pool.len(), 1);

    // Mutate then re-register same ID — should replace
    s.mark_accessed();
    pool.register(s);
    assert_eq!(pool.len(), 1, "duplicate registration should replace, not add");

    // reuse_count should now be 1
    assert_eq!(pool.get(&id).unwrap().reuse_count(), 1);
}

#[test]
fn ac30_session_pool_get_or_create_creates_when_none_exist() {
    let mut pool = SessionPool::new();
    let session = pool.get_or_create("claude", "claude-sonnet-4-6");

    assert!(!session.session_id().is_empty());
    assert_eq!(session.agent_name(), "claude");
    assert_eq!(session.model(), "claude-sonnet-4-6");
    assert_eq!(pool.len(), 1);
}

#[test]
fn ac31_session_pool_get_or_create_reuses_non_stale_session() {
    let mut pool = SessionPool::new();

    let id1 = pool
        .get_or_create("claude", "claude-sonnet-4-6")
        .session_id()
        .to_string();

    let id2 = pool
        .get_or_create("claude", "claude-sonnet-4-6")
        .session_id()
        .to_string();

    assert_eq!(id1, id2, "second call should reuse the existing session");
    assert_eq!(pool.len(), 1);
}

#[test]
fn ac32_session_pool_get_or_create_different_agents_get_different_sessions() {
    let mut pool = SessionPool::new();

    let id1 = pool
        .get_or_create("claude", "claude-sonnet-4-6")
        .session_id()
        .to_string();
    let id2 = pool
        .get_or_create("opencode", "gpt-4o")
        .session_id()
        .to_string();

    assert_ne!(id1, id2);
    assert_eq!(pool.len(), 2);
}

#[test]
fn ac33_session_pool_get_or_create_different_models_get_different_sessions() {
    let mut pool = SessionPool::new();

    let id1 = pool
        .get_or_create("claude", "claude-sonnet-4-6")
        .session_id()
        .to_string();
    let id2 = pool
        .get_or_create("claude", "claude-opus-4-6")
        .session_id()
        .to_string();

    assert_ne!(
        id1, id2,
        "different models for the same agent should create separate sessions"
    );
    assert_eq!(pool.len(), 2);
}

#[test]
fn ac34_session_pool_cleanup_stale_removes_only_old_sessions() {
    let mut pool = SessionPool::new();

    // Fresh session
    pool.register(MemorySession::default());

    // Artificially stale session (last_accessed > 1 hour ago)
    let mut stale = MemorySession::default();
    stale.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(3700);
    pool.register(stale);

    let removed = pool.cleanup_stale();
    assert_eq!(removed, 1, "exactly one stale session should be removed");
    assert_eq!(pool.len(), 1, "the fresh session should survive");
}

#[test]
fn ac35_session_pool_cleanup_stale_noop_when_all_fresh() {
    let mut pool = SessionPool::new();
    pool.register(MemorySession::default());
    pool.register(MemorySession::default());

    assert_eq!(pool.cleanup_stale(), 0);
    assert_eq!(pool.len(), 2);
}

#[test]
fn ac36_session_pool_cleanup_stale_removes_all_when_all_stale() {
    let mut pool = SessionPool::new();

    for _ in 0..3 {
        let mut s = MemorySession::default();
        s.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(4000);
        pool.register(s);
    }

    assert_eq!(pool.cleanup_stale(), 3);
    assert!(pool.is_empty());
}

#[test]
fn ac37_session_pool_mark_accessed_increments_reuse_count() {
    let mut pool = SessionPool::new();
    let session = MemorySession::default();
    let id = session.session_id().to_string();

    pool.register(session);

    let ok = pool.mark_accessed(&id);
    assert!(ok, "mark_accessed should return true for a known session");
    assert_eq!(pool.get(&id).unwrap().reuse_count(), 1);

    pool.mark_accessed(&id);
    assert_eq!(pool.get(&id).unwrap().reuse_count(), 2);
}

#[test]
fn ac38_session_pool_mark_accessed_returns_false_for_missing_id() {
    let mut pool = SessionPool::new();
    assert!(!pool.mark_accessed("nonexistent-session"));
}

#[test]
fn ac39_session_pool_list_by_agent_returns_sessions_for_that_agent() {
    let mut pool = SessionPool::new();

    pool.get_or_create("claude", "claude-sonnet-4-6");
    pool.get_or_create("claude", "claude-opus-4-6");
    pool.get_or_create("opencode", "gpt-4o");

    let claude_sessions = pool.list_by_agent("claude");
    assert!(
        !claude_sessions.is_empty(),
        "should find at least one claude session"
    );
    for s in &claude_sessions {
        assert_eq!(s.agent_name(), "claude");
    }

    let opencode_sessions = pool.list_by_agent("opencode");
    assert!(!opencode_sessions.is_empty());
    for s in &opencode_sessions {
        assert_eq!(s.agent_name(), "opencode");
    }
}

#[test]
fn ac40_session_pool_list_by_agent_returns_empty_for_unknown_agent() {
    let mut pool = SessionPool::new();
    pool.register(MemorySession::default());

    let result = pool.list_by_agent("phantom");
    assert!(result.is_empty());
}

#[test]
fn ac41_session_pool_iter_covers_all_sessions() {
    let mut pool = SessionPool::new();
    pool.register(MemorySession::default());
    pool.register(MemorySession::default());
    pool.register(MemorySession::default());

    assert_eq!(pool.iter().count(), 3);
}

// ============================================================================
// AC-6: SessionData / SessionManager — disk-backed session persistence
// ============================================================================

#[test]
fn ac42_session_data_new_generates_unique_ids() {
    let s1 = SessionData::new("claude", "claude-sonnet-4-6");
    let s2 = SessionData::new("claude", "claude-sonnet-4-6");
    assert_ne!(s1.session_id, s2.session_id, "session IDs must be unique");
}

#[test]
fn ac43_session_data_mark_accessed_increments_reuse_count() {
    let mut session = SessionData::new("claude", "claude-sonnet-4-6");
    assert_eq!(session.reuse_count, 0);

    session.mark_accessed();
    assert_eq!(session.reuse_count, 1);

    session.mark_accessed();
    assert_eq!(session.reuse_count, 2);
}

#[test]
fn ac44_session_data_fresh_is_not_stale() {
    let session = SessionData::new("claude", "claude-sonnet-4-6");
    assert!(!session.is_stale());
}

#[test]
fn ac45_session_data_old_last_accessed_is_stale() {
    let mut session = SessionData::new("claude", "claude-sonnet-4-6");
    session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(3700);
    assert!(session.is_stale());
}

#[test]
fn ac46_session_manager_creates_sessions_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(tmp.path()).unwrap();

    assert!(
        manager.sessions_dir.exists(),
        "sessions directory should be created"
    );
}

#[tokio::test]
async fn ac47_session_manager_create_session_writes_file() {
    let tmp = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(tmp.path()).unwrap();

    let session = manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();

    assert!(!session.session_id.is_empty());
    assert_eq!(session.agent_name, "claude");
    assert_eq!(session.model, "claude-sonnet-4-6");
    assert!(
        session.file_path.exists(),
        "session file should exist on disk"
    );
}

#[tokio::test]
async fn ac48_session_manager_load_session_returns_correct_data() {
    let tmp = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(tmp.path()).unwrap();

    let created = manager
        .create_session("claude", "claude-opus-4-6")
        .await
        .unwrap();

    let loaded = manager
        .load_session(&created.session_id)
        .await
        .unwrap()
        .expect("session should be found");

    assert_eq!(loaded.session_id, created.session_id);
    assert_eq!(loaded.agent_name, "claude");
    assert_eq!(loaded.model, "claude-opus-4-6");
}

#[tokio::test]
async fn ac49_session_manager_load_increments_reuse_count() {
    let tmp = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(tmp.path()).unwrap();

    let session = manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();

    let loaded = manager
        .load_session(&session.session_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.reuse_count, 1, "load should increment reuse_count");
}

#[tokio::test]
async fn ac50_session_manager_load_nonexistent_returns_none() {
    let tmp = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(tmp.path()).unwrap();

    let result = manager.load_session("no-such-session-id").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn ac51_session_manager_delete_removes_file() {
    let tmp = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(tmp.path()).unwrap();

    let session = manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();
    let file_path = session.file_path.clone();

    let deleted = manager.delete_session(&session.session_id).await.unwrap();
    assert!(deleted, "delete should return true");
    assert!(!file_path.exists(), "file should be gone after deletion");
}

#[tokio::test]
async fn ac52_session_manager_delete_nonexistent_returns_false() {
    let tmp = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(tmp.path()).unwrap();

    let result = manager.delete_session("phantom-id").await.unwrap();
    assert!(!result, "deleting nonexistent session should return false");
}

#[tokio::test]
async fn ac53_session_manager_cleanup_removes_stale_sessions() {
    let tmp = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(tmp.path()).unwrap();

    // Create a fresh session
    manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();

    // Create a stale session and manually write an outdated timestamp to disk
    let mut stale = manager
        .create_session("claude", "claude-opus-4-6")
        .await
        .unwrap();

    // Patch the file on disk to appear stale
    stale.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(4000);
    manager.save_session(&stale).await.unwrap();

    let cleaned = manager.cleanup_stale_sessions().await.unwrap();
    assert_eq!(cleaned, 1, "one stale session should have been removed");
}

#[tokio::test]
async fn ac54_session_manager_list_sessions_returns_all() {
    let tmp = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(tmp.path()).unwrap();

    manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();
    manager
        .create_session("claude", "claude-opus-4-6")
        .await
        .unwrap();

    let sessions = manager.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 2, "should list both created sessions");
}

// ============================================================================
// AC-7: Integration — factory + pool work together
// ============================================================================

#[test]
fn ac55_factory_created_backend_has_correct_agent_name() {
    let factory = AgentFactory::new();
    let backend = factory.create("claude").unwrap();
    assert_eq!(backend.agent().name, "claude");
}

#[test]
fn ac56_factory_created_backend_has_non_empty_model() {
    let factory = AgentFactory::new();
    let backend = factory.create("claude").unwrap();
    assert!(!backend.agent().model.is_empty());
}

#[test]
fn ac57_factory_created_backend_has_non_empty_command() {
    let factory = AgentFactory::new();
    let backend = factory.create("claude").unwrap();
    assert!(!backend.agent().command.is_empty());
}

#[test]
fn ac58_pool_and_factory_can_be_combined_for_reuse() {
    // Simulate the pipeline pattern: factory creates the backend,
    // pool tracks a session for that backend.
    let factory = AgentFactory::new();
    let _backend = factory.create("claude").unwrap();

    let mut pool = SessionPool::new();
    let session = pool.get_or_create("claude", "claude-sonnet-4-6");
    let session_id = session.session_id().to_string();

    // Simulate reuse
    pool.mark_accessed(&session_id);

    let reused = pool.get(&session_id).unwrap();
    assert_eq!(reused.reuse_count(), 1);
}

#[test]
fn ac59_stale_session_not_returned_by_get_or_create() {
    let mut pool = SessionPool::new();

    // Insert a stale session manually
    let mut stale = MemorySession::default();
    stale.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(4000);
    let stale_id = stale.session_id().to_string();
    pool.register(stale);

    assert_eq!(pool.len(), 1);

    // get_or_create should bypass the stale session and make a new one
    let fresh = pool.get_or_create("claude", "claude-sonnet-4-6");
    let fresh_id = fresh.session_id().to_string();

    assert_ne!(fresh_id, stale_id, "stale session should not be reused");
    assert_eq!(pool.len(), 2); // stale stays + new one added
}

#[test]
fn ac60_multiple_backends_from_same_factory() {
    let factory = AgentFactory::new();
    let b1 = factory.create("claude").unwrap();
    let b2 = factory.create("claude").unwrap();
    // Two independent backend instances
    assert_eq!(b1.backend_name(), "claude");
    assert_eq!(b2.backend_name(), "claude");
}
