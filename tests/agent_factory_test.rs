//! Tests for AgentFactory and session pool
//!
//! Verifies factory creation by name, per-backend validation,
//! and session pool reuse behaviour.

use ltmatrix::agent::backend::{AgentConfig, AgentSession, MemorySession};
use ltmatrix::agent::factory::{AgentFactory, AgentFactoryConfig};
use ltmatrix::agent::pool::SessionPool;

// ── AgentFactory ────────────────────────────────────────────────────────────

#[test]
fn factory_create_claude_by_name() {
    let factory = AgentFactory::new();
    let agent = factory.create("claude");
    assert!(
        agent.is_ok(),
        "expected Ok for 'claude', got {:?}",
        agent.err()
    );
}

#[test]
fn factory_create_unknown_backend_returns_error() {
    let factory = AgentFactory::new();
    let result = factory.create("nonexistent-backend");
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(
        err.to_string()
            .to_lowercase()
            .contains("nonexistent-backend")
            || err.to_string().to_lowercase().contains("unknown")
            || err.to_string().to_lowercase().contains("unsupported"),
        "error message should mention the unknown backend name: {}",
        err
    );
}

#[test]
fn factory_create_with_custom_config() {
    let factory = AgentFactory::new();
    let config = AgentConfig::builder()
        .name("claude")
        .model("claude-opus-4-6")
        .command("claude")
        .timeout_secs(7200)
        .max_retries(5)
        .enable_session(true)
        .build();

    let agent = factory.create_with_config(config);
    assert!(agent.is_ok());
}

#[test]
fn factory_create_with_invalid_config_returns_error() {
    let factory = AgentFactory::new();
    let mut config = AgentConfig::default();
    config.model = "".to_string(); // invalid

    let result = factory.create_with_config(config);
    assert!(result.is_err());
}

#[test]
fn factory_lists_supported_backends() {
    let factory = AgentFactory::new();
    let backends = factory.supported_backends();

    assert!(
        backends.contains(&"claude"),
        "claude must be supported; got {:?}",
        backends
    );
    // The factory knows at least one backend
    assert!(!backends.is_empty());
}

#[test]
fn factory_is_backend_supported_claude() {
    let factory = AgentFactory::new();
    assert!(factory.is_supported("claude"));
}

#[test]
fn factory_is_backend_supported_unknown() {
    let factory = AgentFactory::new();
    assert!(!factory.is_supported("does-not-exist"));
}

#[test]
fn factory_default_backend_is_claude() {
    let factory = AgentFactory::new();
    let agent = factory.create_default();
    assert!(agent.is_ok());
}

#[test]
fn factory_with_config_sets_defaults() {
    let factory_config = AgentFactoryConfig {
        default_backend: "claude".to_string(),
    };
    let factory = AgentFactory::with_factory_config(factory_config);
    let agent = factory.create_default();
    assert!(agent.is_ok());
}

// Per-backend validation ─────────────────────────────────────────────────────

#[test]
fn factory_validates_claude_name_field() {
    let factory = AgentFactory::new();

    let bad = AgentConfig::builder()
        .name("opencode") // wrong name for the claude backend
        .model("claude-sonnet-4-6")
        .command("claude")
        .timeout_secs(3600)
        .build();

    // Creating a claude backend with a non-claude name should fail validation
    let result = factory.validate_config("claude", &bad);
    assert!(result.is_err(), "expected validation error for wrong name");
}

#[test]
fn factory_validates_empty_model() {
    let factory = AgentFactory::new();

    let bad = AgentConfig::builder()
        .name("claude")
        .model("")
        .command("claude")
        .timeout_secs(3600)
        .build();

    let result = factory.validate_config("claude", &bad);
    assert!(result.is_err());
}

#[test]
fn factory_validates_zero_timeout() {
    let factory = AgentFactory::new();

    let bad = AgentConfig {
        name: "claude".to_string(),
        model: "claude-sonnet-4-6".to_string(),
        command: "claude".to_string(),
        timeout_secs: 0,
        max_retries: 3,
        enable_session: true,
        api_key: None,
        base_url: None,
    };

    let result = factory.validate_config("claude", &bad);
    assert!(result.is_err());
}

#[test]
fn factory_validate_config_valid_claude() {
    let factory = AgentFactory::new();
    let config = AgentConfig::default();
    let result = factory.validate_config("claude", &config);
    assert!(result.is_ok(), "{:?}", result);
}

// ── SessionPool ─────────────────────────────────────────────────────────────

#[test]
fn session_pool_new_is_empty() {
    let pool = SessionPool::new();
    assert_eq!(pool.len(), 0);
}

#[test]
fn session_pool_register_and_get() {
    let mut pool = SessionPool::new();
    let session = MemorySession::default();
    let id = session.session_id().to_string();

    pool.register(session);
    assert_eq!(pool.len(), 1);

    let fetched = pool.get(&id);
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().session_id(), id);
}

#[test]
fn session_pool_get_missing_returns_none() {
    let pool = SessionPool::new();
    assert!(pool.get("nonexistent-id").is_none());
}

#[test]
fn session_pool_remove_existing() {
    let mut pool = SessionPool::new();
    let session = MemorySession::default();
    let id = session.session_id().to_string();

    pool.register(session);
    assert_eq!(pool.len(), 1);

    let removed = pool.remove(&id);
    assert!(removed.is_some());
    assert_eq!(pool.len(), 0);
}

#[test]
fn session_pool_remove_missing_returns_none() {
    let mut pool = SessionPool::new();
    let removed = pool.remove("ghost");
    assert!(removed.is_none());
}

#[test]
fn session_pool_cleanup_stale_removes_old_sessions() {
    let mut pool = SessionPool::new();

    // Add a fresh session
    let fresh = MemorySession::default();
    pool.register(fresh);

    // Add a stale session (last_accessed > 1 hour ago)
    let mut stale = MemorySession::default();
    stale.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(4000);
    pool.register(stale);

    assert_eq!(pool.len(), 2);

    let removed = pool.cleanup_stale();
    assert_eq!(removed, 1, "only the stale session should be removed");
    assert_eq!(pool.len(), 1);
}

#[test]
fn session_pool_cleanup_stale_no_stale_sessions() {
    let mut pool = SessionPool::new();
    pool.register(MemorySession::default());

    let removed = pool.cleanup_stale();
    assert_eq!(removed, 0);
    assert_eq!(pool.len(), 1);
}

#[test]
fn session_pool_get_or_create_creates_when_missing() {
    let mut pool = SessionPool::new();

    let session = pool.get_or_create("claude", "claude-sonnet-4-6");
    assert!(!session.session_id().is_empty());
    assert_eq!(session.agent_name(), "claude");
    assert_eq!(session.model(), "claude-sonnet-4-6");
    assert_eq!(pool.len(), 1);
}

#[test]
fn session_pool_get_or_create_reuses_existing() {
    let mut pool = SessionPool::new();

    // First call – creates
    let first_id = {
        let s = pool.get_or_create("claude", "claude-sonnet-4-6");
        s.session_id().to_string()
    };
    assert_eq!(pool.len(), 1);

    // Second call with same agent/model – reuses
    let second_id = {
        let s = pool.get_or_create("claude", "claude-sonnet-4-6");
        s.session_id().to_string()
    };
    assert_eq!(first_id, second_id, "should reuse the existing session");
    assert_eq!(pool.len(), 1);
}

#[test]
fn session_pool_get_or_create_different_agents_get_different_sessions() {
    let mut pool = SessionPool::new();

    let s1 = pool.get_or_create("claude", "claude-sonnet-4-6");
    let id1 = s1.session_id().to_string();

    let s2 = pool.get_or_create("opencode", "gpt-4");
    let id2 = s2.session_id().to_string();

    assert_ne!(id1, id2);
    assert_eq!(pool.len(), 2);
}

#[test]
fn session_pool_list_by_agent() {
    let mut pool = SessionPool::new();

    pool.get_or_create("claude", "claude-sonnet-4-6");
    pool.get_or_create("claude", "claude-opus-4-6");
    pool.get_or_create("opencode", "gpt-4");

    let claude_sessions = pool.list_by_agent("claude");
    // We use get_or_create so same agent reuses; but different models create new sessions
    // The exact count depends on implementation – at minimum 1 claude session
    assert!(!claude_sessions.is_empty());

    let opencode_sessions = pool.list_by_agent("opencode");
    assert!(!opencode_sessions.is_empty());
}

#[test]
fn session_pool_mark_accessed_increments_reuse_count() {
    let mut pool = SessionPool::new();
    let session = MemorySession::default();
    let id = session.session_id().to_string();

    pool.register(session);

    let result = pool.mark_accessed(&id);
    assert!(
        result,
        "mark_accessed should return true for existing session"
    );

    let fetched = pool.get(&id).unwrap();
    assert_eq!(fetched.reuse_count(), 1);
}
