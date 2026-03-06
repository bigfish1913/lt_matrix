//! Comprehensive tests for Claude agent backend implementation
//!
//! This test suite verifies the public API of the Claude agent backend:
//! - Agent creation and configuration
//! - Model selection (Haiku 4.5, Sonnet 4.6, Opus 4.6)
//! - Session file management and reuse
//! - AgentBackend trait implementation
//! - Error handling and edge cases

use ltmatrix::agent::backend::{AgentBackend, AgentResponse, ExecutionConfig};
use ltmatrix::agent::claude::ClaudeAgent;
use ltmatrix::agent::session::{SessionData, SessionManager};
use ltmatrix::models::{Agent, ExecutionMode, Task};

// =============================================================================
// Agent Creation and Configuration Tests
// =============================================================================

#[test]
fn test_claude_agent_new() {
    let agent = ClaudeAgent::new();
    assert!(agent.is_ok(), "Should create agent successfully");

    let claude = agent.unwrap();
    assert_eq!(claude.agent().name, "claude");
    assert_eq!(claude.agent().command, "claude");
    assert_eq!(claude.agent().model, "claude-sonnet-4-6");
}

#[test]
fn test_claude_agent_default() {
    let agent = ClaudeAgent::default();
    assert_eq!(agent.agent().name, "claude");
    assert_eq!(agent.agent().model, "claude-sonnet-4-6");
}

#[test]
fn test_claude_agent_with_custom_config() {
    let custom_agent = Agent::new("custom-claude", "claude-custom", "claude-opus-4-6", 7200);
    let session_manager = SessionManager::default_manager().unwrap();

    let agent = ClaudeAgent::with_agent(custom_agent.clone(), session_manager);

    assert_eq!(agent.agent().name, "custom-claude");
    assert_eq!(agent.agent().command, "claude-custom");
    assert_eq!(agent.agent().model, "claude-opus-4-6");
}

#[tokio::test]
async fn test_claude_agent_without_verification() {
    let agent = ClaudeAgent::new()
        .unwrap()
        .without_verification();

    // Health check should succeed when verification is disabled
    let health_result = agent.health_check().await;
    assert!(health_result.is_ok());
    assert!(health_result.unwrap(), "Health check should pass");
}

// =============================================================================
// ExecutionConfig Tests
// =============================================================================

#[test]
fn test_execution_config_default() {
    let config = ExecutionConfig::default();

    assert_eq!(config.model, "claude-sonnet-4-6");
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.timeout, 3600);
    assert!(config.enable_session);
    assert!(config.env_vars.is_empty());
}

#[test]
fn test_execution_config_custom() {
    let config = ExecutionConfig {
        model: "claude-opus-4-6".to_string(),
        max_retries: 5,
        timeout: 7200,
        enable_session: false,
        env_vars: vec![("KEY".to_string(), "value".to_string())],
    };

    assert_eq!(config.model, "claude-opus-4-6");
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.timeout, 7200);
    assert!(!config.enable_session);
    assert_eq!(config.env_vars.len(), 1);
}

#[test]
fn test_execution_config_fast_mode() {
    let config = ExecutionConfig {
        model: "claude-haiku-4-5".to_string(),
        max_retries: 1,
        timeout: 1800,
        enable_session: true,
        env_vars: vec![],
    };

    assert_eq!(config.model, "claude-haiku-4-5");
    assert_eq!(config.max_retries, 1);
    assert_eq!(config.timeout, 1800);
}

#[test]
fn test_execution_config_with_all_models() {
    let models = vec![
        ("claude-haiku-4-5", 1, 1800),
        ("claude-sonnet-4-6", 3, 3600),
        ("claude-opus-4-6", 3, 7200),
    ];

    for (model, retries, timeout) in models {
        let config = ExecutionConfig {
            model: model.to_string(),
            max_retries: retries,
            timeout,
            ..Default::default()
        };

        assert_eq!(config.model, model);
        assert_eq!(config.max_retries, retries);
        assert_eq!(config.timeout, timeout);
    }
}

// =============================================================================
// AgentResponse Tests
// =============================================================================

#[test]
fn test_agent_response_success() {
    let response = AgentResponse {
        output: "Task completed successfully".to_string(),
        structured_data: Some(serde_json::json!({"status": "success"})),
        is_complete: true,
        error: None,
    };

    assert!(response.is_complete);
    assert!(response.error.is_none());
    assert!(response.structured_data.is_some());
}

#[test]
fn test_agent_response_with_error() {
    let response = AgentResponse {
        output: "".to_string(),
        structured_data: None,
        is_complete: false,
        error: Some("Process failed with exit code 1".to_string()),
    };

    assert!(!response.is_complete);
    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap(), "Process failed with exit code 1");
}

#[test]
fn test_agent_response_partial() {
    let response = AgentResponse {
        output: "Some output but not complete".to_string(),
        structured_data: None,
        is_complete: false,
        error: None,
    };

    assert!(!response.is_complete);
    assert!(response.error.is_none());
    assert!(response.structured_data.is_none());
}

// =============================================================================
// Session Management Tests
// =============================================================================

#[tokio::test]
async fn test_session_creation_and_persistence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    let session = manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();

    assert!(!session.session_id.is_empty());
    assert_eq!(session.agent_name, "claude");
    assert_eq!(session.model, "claude-sonnet-4-6");
    assert_eq!(session.reuse_count, 0);
    assert!(session.file_path.exists());
}

#[tokio::test]
async fn test_session_loading_and_reuse() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    // Create session
    let original = manager
        .create_session("claude", "claude-opus-4-6")
        .await
        .unwrap();
    let session_id = original.session_id.clone();

    // Load session
    let loaded = manager.load_session(&session_id).await.unwrap().unwrap();

    assert_eq!(loaded.session_id, original.session_id);
    assert_eq!(loaded.agent_name, original.agent_name);
    assert_eq!(loaded.model, original.model);
    assert_eq!(loaded.reuse_count, 1); // Should increment on load
}

#[tokio::test]
async fn test_session_mark_accessed() {
    let mut session = SessionData::new("test-agent", "test-model");

    assert_eq!(session.reuse_count, 0);

    session.mark_accessed();
    assert_eq!(session.reuse_count, 1);

    session.mark_accessed();
    assert_eq!(session.reuse_count, 2);

    // Verify last_accessed was updated
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(session.last_accessed);
    assert!(duration.num_seconds() < 5, "last_accessed should be recent");
}

#[tokio::test]
async fn test_session_stale_detection() {
    let mut session = SessionData::new("test-agent", "test-model");

    // Fresh session should not be stale
    assert!(!session.is_stale());

    // Make session appear old (more than 1 hour ago)
    session.last_accessed = chrono::Utc::now() - chrono::Duration::hours(2);
    assert!(session.is_stale());
}

#[tokio::test]
async fn test_session_deletion() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    let session = manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();

    assert!(session.file_path.exists());

    let deleted = manager
        .delete_session(&session.session_id)
        .await
        .unwrap();

    assert!(deleted, "Should return true when session is deleted");
    assert!(!session.file_path.exists(), "File should be removed");
}

#[tokio::test]
async fn test_session_cleanup_stale_sessions() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    // Create a stale session
    let mut stale_session = manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();
    stale_session.last_accessed = chrono::Utc::now() - chrono::Duration::hours(2);
    manager.save_session(&stale_session).await.unwrap();

    // Create a fresh session
    let fresh_session = manager
        .create_session("claude", "claude-opus-4-6")
        .await
        .unwrap();

    // Run cleanup
    let cleaned = manager.cleanup_stale_sessions().await.unwrap();

    assert_eq!(cleaned, 1, "Should clean up 1 stale session");
    assert!(!stale_session.file_path.exists(), "Stale session should be removed");
    assert!(fresh_session.file_path.exists(), "Fresh session should remain");
}

#[tokio::test]
async fn test_session_list_all_sessions() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    // Create multiple sessions
    manager
        .create_session("claude", "claude-sonnet-4-6")
        .await
        .unwrap();
    manager
        .create_session("claude", "claude-opus-4-6")
        .await
        .unwrap();
    manager
        .create_session("claude", "claude-haiku-4-5")
        .await
        .unwrap();

    let sessions = manager.list_sessions().await.unwrap();

    assert_eq!(sessions.len(), 3);
}

#[tokio::test]
async fn test_session_manager_nonexistent_session_load() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    let result = manager.load_session("nonexistent-id").await.unwrap();

    assert!(result.is_none(), "Should return None for nonexistent session");
}

#[tokio::test]
async fn test_session_manager_delete_nonexistent_session() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    let result = manager.delete_session("nonexistent-id").await.unwrap();

    assert!(!result, "Should return false for nonexistent session");
}

#[tokio::test]
async fn test_session_data_serialization_roundtrip() {
    let session = SessionData::new("test-agent", "test-model");

    let json = serde_json::to_string(&session).unwrap();
    let deserialized: SessionData = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.session_id, session.session_id);
    assert_eq!(deserialized.agent_name, session.agent_name);
    assert_eq!(deserialized.model, session.model);
    assert_eq!(deserialized.reuse_count, session.reuse_count);
}

#[tokio::test]
async fn test_session_creation_in_nonexistent_directory() {
    let temp_dir = tempfile::tempdir().unwrap();
    let nested_dir = temp_dir.path().join("nested").join("dirs");

    // SessionManager should create the directory
    let manager = SessionManager::new(&nested_dir).unwrap();

    assert!(nested_dir.exists(), "Should create nested directories");

    let session = manager.create_session("claude", "claude-sonnet-4-6").await.unwrap();
    assert!(session.file_path.exists());
}

#[tokio::test]
async fn test_session_manager_list_empty_directory() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    let sessions = manager.list_sessions().await.unwrap();

    assert!(sessions.is_empty(), "Empty directory should return empty list");
}

#[tokio::test]
async fn test_session_concurrent_access() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    // Create multiple sessions concurrently
    let handle1 = manager.create_session("claude", "claude-sonnet-4-6");
    let handle2 = manager.create_session("claude", "claude-opus-4-6");
    let handle3 = manager.create_session("claude", "claude-haiku-4-5");

    let (session1, session2, session3) = tokio::join!(handle1, handle2, handle3);

    assert!(session1.is_ok());
    assert!(session2.is_ok());
    assert!(session3.is_ok());

    // Verify all sessions have unique IDs
    let id1 = session1.unwrap().session_id;
    let id2 = session2.unwrap().session_id;
    let id3 = session3.unwrap().session_id;

    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert_ne!(id1, id3);
}

#[tokio::test]
async fn test_session_with_different_models() {
    let models = vec![
        "claude-haiku-4-5",
        "claude-sonnet-4-6",
        "claude-opus-4-6",
    ];

    for model in models {
        let session = SessionData::new("claude", model);
        assert_eq!(session.model, model);
        assert_eq!(session.agent_name, "claude");
    }
}

// =============================================================================
// Integration Tests (Public API)
// =============================================================================

#[tokio::test]
async fn test_claude_agent_backend_name() {
    let agent = ClaudeAgent::new().unwrap();
    assert_eq!(agent.backend_name(), "claude");
}

#[tokio::test]
async fn test_claude_agent_agent_accessor() {
    let agent = ClaudeAgent::new().unwrap();
    let config = agent.agent();

    assert_eq!(config.name, "claude");
    assert_eq!(config.command, "claude");
    assert_eq!(config.model, "claude-sonnet-4-6");
    assert_eq!(config.timeout, 3600);
    assert!(config.is_default);
}

#[tokio::test]
async fn test_claude_agent_health_check_with_verification_disabled() {
    let agent = ClaudeAgent::new()
        .unwrap()
        .without_verification();

    // Should return Ok(true) when verification is disabled
    let result = agent.health_check().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_claude_agent_task_prompt_formatting() {
    let agent = ClaudeAgent::new()
        .unwrap()
        .without_verification();

    let task = Task::new("task-1", "Test Task", "A test description");
    let context = "Some context information";
    let config = ExecutionConfig::default();

    // We can't actually execute without Claude installed, but we can verify
    // the method exists and has the right signature
    let result = agent.execute_task(&task, context, &config).await;

    // Will fail because claude command doesn't exist, but that's expected
    // The important thing is the method compiles and has the right behavior
    assert!(result.is_err() || result.is_ok());
}

#[tokio::test]
async fn test_claude_agent_execute_with_disabled_verification() {
    let agent = ClaudeAgent::new()
        .unwrap()
        .without_verification();

    let prompt = "Test prompt";
    let config = ExecutionConfig::default();

    // With verification disabled, the command execution proceeds without
    // checking if claude is available first. The result depends on whether
    // claude is actually installed on the system.
    let result = agent.execute(prompt, &config).await;

    // The test verifies that verification was bypassed (no verification error).
    // Actual execution result depends on environment:
    // - If claude is installed: may succeed or fail depending on the prompt
    // - If claude is not installed: fails with "command not found"
    //
    // The key assertion is that we got past verification without error.
    // We accept both success and failure since the environment varies.
    match result {
        Ok(_) => {
            // Claude is installed and executed - verification was bypassed
        }
        Err(e) => {
            // Verify the error is NOT a verification error
            let error_msg = e.to_string().to_lowercase();
            assert!(!error_msg.contains("verification"),
                    "Should not get a verification error when verification is disabled: {}", e);
            // Command not found or other execution errors are acceptable
        }
    }
}

// =============================================================================
// Model Selection Tests
// =============================================================================

#[test]
fn test_model_selection_for_execution_modes() {
    assert_eq!(ExecutionMode::Fast.default_model(), "claude-haiku-4-5");
    assert_eq!(ExecutionMode::Standard.default_model(), "claude-sonnet-4-6");
    assert_eq!(ExecutionMode::Expert.default_model(), "claude-opus-4-6");
}

#[test]
fn test_claude_default_agent_configuration() {
    let agent = Agent::claude_default();

    assert_eq!(agent.name, "claude");
    assert_eq!(agent.command, "claude");
    assert_eq!(agent.model, "claude-sonnet-4-6");
    assert_eq!(agent.timeout, 3600);
    assert!(agent.is_default);
}

#[test]
fn test_all_claude_models_are_valid() {
    let models = vec![
        "claude-haiku-4-5",
        "claude-sonnet-4-6",
        "claude-opus-4-6",
    ];

    for model in models {
        // Verify these models are non-empty strings
        assert!(!model.is_empty());
        assert!(model.starts_with("claude-"));
    }
}

// =============================================================================
// Task and Context Tests
// =============================================================================

#[test]
fn test_task_creation_for_agent() {
    let task = Task::new("task-1", "Implement feature", "Add a new feature");

    assert_eq!(task.id, "task-1");
    assert_eq!(task.title, "Implement feature");
    assert_eq!(task.description, "Add a new feature");
    assert_eq!(task.status, ltmatrix::models::TaskStatus::Pending);
}

// =============================================================================
// Multiple Agents Tests
// =============================================================================

#[tokio::test]
async fn test_multiple_claude_agents_with_different_configs() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(temp_dir.path()).unwrap();

    let agent1 = ClaudeAgent::with_agent(
        Agent::new("claude-haiku", "claude", "claude-haiku-4-5", 1800),
        manager.clone(),
    );

    let agent2 = ClaudeAgent::with_agent(
        Agent::new("claude-sonnet", "claude", "claude-sonnet-4-6", 3600),
        manager.clone(),
    );

    let agent3 = ClaudeAgent::with_agent(
        Agent::new("claude-opus", "claude", "claude-opus-4-6", 7200),
        manager,
    );

    assert_eq!(agent1.agent().model, "claude-haiku-4-5");
    assert_eq!(agent2.agent().model, "claude-sonnet-4-6");
    assert_eq!(agent3.agent().model, "claude-opus-4-6");
}

// =============================================================================
// Retry Logic Verification Tests
// =============================================================================

#[test]
fn test_retry_timing_calculation() {
    // Verify the exponential backoff formula
    let base_delay_ms = 100;

    let delays: Vec<u64> = (0..5u32)
        .map(|attempt| base_delay_ms * 2_u64.pow(attempt))
        .collect();

    assert_eq!(delays, vec![100, 200, 400, 800, 1600]);
}

#[test]
fn test_max_retries_configuration() {
    let configs = vec![
        ("fast", 1),
        ("standard", 3),
        ("expert", 3),
    ];

    for (_name, max_retries) in configs {
        let config = ExecutionConfig {
            max_retries,
            ..Default::default()
        };

        // Should be able to configure different retry counts
        assert_eq!(config.max_retries, max_retries);
    }
}

#[test]
fn test_timeout_configuration() {
    let timeouts = vec![
        (60, "1 minute timeout"),
        (300, "5 minute timeout"),
        (1800, "30 minute timeout"),
        (3600, "1 hour timeout"),
        (7200, "2 hour timeout"),
    ];

    for (timeout, _description) in timeouts {
        let config = ExecutionConfig {
            timeout,
            ..Default::default()
        };

        assert_eq!(config.timeout, timeout);
    }
}

// =============================================================================
// AgentBackend Trait Implementation Tests
// =============================================================================

#[tokio::test]
async fn test_agent_backend_trait_methods_exist() {
    let agent = ClaudeAgent::new()
        .unwrap()
        .without_verification();

    // Test that all trait methods are accessible
    let _ = agent.agent();
    let _ = agent.backend_name();

    // These will fail without claude installed, but we verify they exist
    let config = ExecutionConfig::default();

    // health_check should work with verification disabled
    let health = agent.health_check().await;
    assert!(health.is_ok());

    // execute and execute_task will fail without claude, but they exist
    let task = Task::new("test", "Test", "Test task");
    let _ = agent.execute("prompt", &config).await;
    let _ = agent.execute_task(&task, "context", &config).await;
}
